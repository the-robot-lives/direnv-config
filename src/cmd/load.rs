//! `dc load <file> [<file>...]` — ingest external plain-YAML config files.
//!
//! Each file is a single YAML document whose root mapping keys are subject
//! names (optionally wrapped under a `subjects:` key). Each subject body goes
//! through the exact same split-secrets + [`crate::cmd::yaml::apply`]
//! pipeline as `dc yaml`, with per-subject `_dc_layer` routing, so plain keys
//! land in the chosen layer file and `🔒` values are encrypted into
//! `secrets.yaml`. Encryption is unchanged from `dc yaml`.
//!
//! Load order is argument order and later files win on conflict: every
//! ingest deep-merges over what earlier ones wrote (per layer). Conflicts
//! between layers resolve by the existing layer weights — there is no
//! separate "inline beats external" rule; ingest order inside `.envrc`
//! decides, then layer weights.

use anyhow::{bail, Context, Result};
use serde_yaml::Value;
use std::path::{Path, PathBuf};

use crate::envrc::locator::LAYER_KEY;

/// Flags mirrored from `dc yaml`, applied per subject.
#[derive(Default)]
pub(crate) struct IngestOpts<'a> {
    /// Default layer for subjects lacking `_dc_layer` (default `base`).
    pub layer: Option<&'a str>,
    pub replace: bool,
    pub replace_key: Option<&'a str>,
    pub if_missing: bool,
}

/// Parse a loaded document into `(subject, body)` pairs.
///
/// The root mapping maps subject names to config bodies; a lone `subjects:`
/// key whose value is a mapping is an optional wrapper holding the same
/// shape one level deeper. `subjects:` mixed with other root keys is an
/// ambiguity we refuse rather than guess at.
pub(crate) fn parse_subjects(file: &Path, doc: &Value) -> Result<Vec<(String, Value)>> {
    let Value::Mapping(root) = doc else {
        bail!(
            "{}: root must be a YAML mapping (subject names to config bodies)",
            file.display()
        );
    };
    let entries = mapping_entries(file, root)?;
    if let Some((_, wrapper)) = entries.iter().find(|(k, _)| k == "subjects") {
        if entries.len() > 1 {
            let others: Vec<&str> =
                entries.iter().filter(|(k, _)| k != "subjects").map(|(k, _)| k.as_str()).collect();
            bail!(
                "{}: ambiguous — `subjects:` wrapper cannot be combined with other root keys ({})",
                file.display(),
                others.join(", ")
            );
        }
        let Value::Mapping(inner) = wrapper else {
            bail!(
                "{}: `subjects:` value must be a mapping of subject names to config bodies",
                file.display()
            );
        };
        return mapping_entries(file, inner);
    }
    Ok(entries)
}

fn mapping_entries(file: &Path, m: &serde_yaml::Mapping) -> Result<Vec<(String, Value)>> {
    let mut out = Vec::new();
    for (k, v) in m {
        let Some(name) = k.as_str() else {
            bail!(
                "{}: subject keys must be strings, got: {}",
                file.display(),
                serde_yaml::to_string(k)?.trim()
            );
        };
        if name.is_empty() {
            bail!("{}: subject name must not be empty", file.display());
        }
        out.push((name.to_string(), v.clone()));
    }
    Ok(out)
}

/// Ingest one subject body into the store via the `dc yaml` apply path.
///
/// Strips the reserved `_dc_layer` key (which selects the subject's layer,
/// overriding the CLI `--layer` default), splits `🔒` secrets, and encrypts
/// them into `secrets.yaml`. Returns `false` when `--if-missing` skipped the
/// subject because its target layer file already exists.
pub(crate) fn ingest_subject(
    store: &Path,
    subject: &str,
    body: &Value,
    opts: &IngestOpts,
    load_key: &dyn Fn() -> Result<[u8; 32]>,
) -> Result<bool> {
    let mut body = body.clone();
    let layer_override = if let Value::Mapping(m) = &mut body {
        match m.remove(Value::String(LAYER_KEY.to_string())) {
            None => None,
            Some(Value::String(s)) => Some(s),
            Some(other) => bail!(
                "`_dc_layer` must be a layer name string, got: {}",
                serde_yaml::to_string(&other)?.trim()
            ),
        }
    } else {
        bail!("subject `{subject}` body must be a mapping");
    };
    let layer_name = layer_override.unwrap_or_else(|| opts.layer.unwrap_or("base").to_string());

    let layer_file = crate::store::layout::layer_path(store, subject, &layer_name);
    if opts.if_missing && layer_file.exists() {
        return Ok(false);
    }

    let (plain, secrets) = crate::secret::split_secrets(&body);
    let key = if secrets.is_empty() { None } else { Some(load_key()?) };
    crate::cmd::yaml::apply(
        store,
        subject,
        &layer_name,
        &plain,
        &secrets,
        key.as_ref(),
        opts.replace,
        opts.replace_key,
    )?;
    Ok(true)
}

/// Load `files` (in argument order — later files win) into `store`, then
/// register them in the store meta so `dc config *` can find their keys.
/// Returns the subjects touched, in first-touch order.
pub(crate) fn load_files(
    store: &Path,
    files: &[PathBuf],
    opts: &IngestOpts,
    load_key: &dyn Fn() -> Result<[u8; 32]>,
) -> Result<Vec<String>> {
    let mut touched: Vec<String> = Vec::new();
    for f in files {
        let content =
            std::fs::read_to_string(f).with_context(|| format!("failed to read {}", f.display()))?;
        let doc: Value = serde_yaml::from_str(&content).with_context(|| {
            format!("failed to parse {} as a single YAML document", f.display())
        })?;
        for (subject, body) in parse_subjects(f, &doc)? {
            if !matches!(body, Value::Mapping(_)) {
                bail!("{}: subject `{subject}` body must be a mapping", f.display());
            }
            ingest_subject(store, &subject, &body, opts, load_key)?;
            if !touched.contains(&subject) {
                touched.push(subject);
            }
        }
    }
    crate::store::meta::register_external_files(store, files, opts.layer, opts.if_missing)?;
    Ok(touched)
}

/// `dc load` entry point — resolve the store from CWD, ingest every file,
/// re-resolve touched subjects, and bump the version once at the end.
pub fn run(
    files: &[String],
    layer: Option<&str>,
    replace: bool,
    replace_key: Option<&str>,
    if_missing: bool,
    no_bump: bool,
) -> Result<()> {
    if files.is_empty() {
        bail!("provide at least one file: dc load <file> [<file>...]");
    }
    let cwd = std::env::current_dir()?;
    let store = crate::store::ensure_store(&cwd)?;
    let _lock = crate::store::lock_store(&store)?;

    let paths: Vec<PathBuf> = files
        .iter()
        .map(|f| {
            std::fs::canonicalize(f)
                .with_context(|| format!("cannot load {f}: file not found"))
        })
        .collect::<Result<_>>()?;

    let opts = IngestOpts { layer, replace, replace_key, if_missing };
    let subjects = load_files(&store, &paths, &opts, &crate::settings::key)?;

    for subject in &subjects {
        crate::store::resolve::resolve_active(&store, subject)?;
    }
    crate::store::meta::update_configs_list(&store)?;

    if !no_bump {
        crate::store::bump_version(&store)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const KEY: [u8; 32] = [11u8; 32];

    fn key() -> Result<[u8; 32]> {
        Ok(KEY)
    }

    fn opts(layer: Option<&'static str>) -> IngestOpts<'static> {
        IngestOpts { layer, replace: false, replace_key: None, if_missing: false }
    }

    /// Create a store dir with a minimal .meta so registration works.
    fn init_store() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let store = tmp.path().join("store");
        std::fs::create_dir_all(&store).unwrap();
        crate::store::meta::write_meta(
            &store,
            &crate::store::meta::StoreMeta {
                source: tmp.path().to_path_buf(),
                created: "2026-01-01T00:00:00+00:00".to_string(),
                parent: None,
                configs: Vec::new(),
                external: Vec::new(),
            },
        )
        .unwrap();
        (tmp, store)
    }

    fn write(tmp: &TempDir, name: &str, content: &str) -> PathBuf {
        let p = tmp.path().join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    fn read_layer(store: &Path, subject: &str, layer: &str) -> Value {
        crate::store::load_layer(&crate::store::layout::layer_path(store, subject, layer)).unwrap()
    }

    #[test]
    fn multi_subject_load_from_one_file() {
        let (tmp, store) = init_store();
        let f = write(&tmp, "dc.yaml", "cluster:\n  name: noizu\ninfra:\n  region: us-east-1\n");
        let subjects = load_files(&store, std::slice::from_ref(&f), &opts(None), &key).unwrap();
        assert_eq!(subjects, vec!["cluster".to_string(), "infra".to_string()]);

        let cluster = read_layer(&store, "cluster", "base");
        assert_eq!(
            crate::yaml::path::get_path(&cluster, "name"),
            Some(Value::String("noizu".into()))
        );
        let infra = read_layer(&store, "infra", "base");
        assert_eq!(
            crate::yaml::path::get_path(&infra, "region"),
            Some(Value::String("us-east-1".into()))
        );

        // the file is registered in the store meta, in load order
        let meta = crate::store::meta::read_meta(&store).unwrap();
        assert_eq!(meta.external.len(), 1);
        assert_eq!(meta.external[0].path, f);
    }

    #[test]
    fn subjects_wrapper_parses_and_mixed_root_errors() {
        let doc: Value =
            serde_yaml::from_str("subjects:\n  cluster:\n    name: noizu\n").unwrap();
        let s = parse_subjects(Path::new("/x/dc.yaml"), &doc).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].0, "cluster");

        // wrapper mixed with other root keys → ambiguity error
        let doc: Value = serde_yaml::from_str(
            "subjects:\n  cluster:\n    name: noizu\ninfra:\n  region: x\n",
        )
        .unwrap();
        let err = parse_subjects(Path::new("/x/dc.yaml"), &doc).unwrap_err().to_string();
        assert!(err.contains("cannot be combined"), "{err}");
        assert!(err.contains("infra"), "{err}");

        // non-mapping root
        let doc: Value = serde_yaml::from_str("- a\n- b\n").unwrap();
        let err = parse_subjects(Path::new("/x/dc.yaml"), &doc).unwrap_err().to_string();
        assert!(err.contains("root must be a YAML mapping"), "{err}");
    }

    #[test]
    fn dc_layer_routes_and_cli_layer_is_the_default() {
        let (tmp, store) = init_store();
        let f = write(&tmp, "dc.yaml", "a:\n  _dc_layer: local\n  k: v\nb:\n  k: w\n");
        load_files(&store, &[f], &opts(None), &key).unwrap();
        // `a` routes to local via _dc_layer; `b` falls back to base
        assert_eq!(
            crate::yaml::path::get_path(&read_layer(&store, "a", "local"), "k"),
            Some(Value::String("v".into()))
        );
        assert!(!crate::store::layout::layer_path(&store, "a", "base").exists());
        assert_eq!(
            crate::yaml::path::get_path(&read_layer(&store, "b", "base"), "k"),
            Some(Value::String("w".into()))
        );

        // CLI --layer applies to subjects lacking _dc_layer; _dc_layer still wins
        let (tmp2, store2) = init_store();
        let f2 = write(&tmp2, "dc.yaml", "a:\n  _dc_layer: local\n  k: v\nb:\n  k: w\n");
        load_files(&store2, &[f2], &opts(Some("dev")), &key).unwrap();
        assert_eq!(
            crate::yaml::path::get_path(&read_layer(&store2, "b", "dev"), "k"),
            Some(Value::String("w".into()))
        );
        assert_eq!(
            crate::yaml::path::get_path(&read_layer(&store2, "a", "local"), "k"),
            Some(Value::String("v".into()))
        );
    }

    #[test]
    fn padlock_value_lands_encrypted_in_secrets_layer() {
        let (tmp, store) = init_store();
        let f = write(&tmp, "dc.yaml", "cluster:\n  name: noizu\n  token: \"🔒❗ secret-body\"\n");
        load_files(&store, &[f], &opts(None), &key).unwrap();

        // plain key lands in base; the secret leaf does not
        let base = read_layer(&store, "cluster", "base");
        assert_eq!(
            crate::yaml::path::get_path(&base, "name"),
            Some(Value::String("noizu".into()))
        );
        assert!(crate::yaml::path::get_path(&base, "token").is_none());

        // the secret lands encrypted in secrets.yaml — assert token shape only
        let secrets = read_layer(&store, "cluster", "secrets");
        let tok = crate::yaml::path::get_path(&secrets, "token").unwrap();
        let tok = tok.as_str().unwrap();
        assert!(crate::crypto::is_encrypted(tok), "not an encrypted token: {tok}");
        let (tier, body) = crate::crypto::decode_token(tok, &KEY).unwrap();
        assert_eq!(tier, 1);
        assert_eq!(body, "secret-body");
    }

    #[test]
    fn later_files_win_on_conflict() {
        let (tmp, store) = init_store();
        let f1 = write(&tmp, "a.yaml", "cluster:\n  name: first\n  only_a: x\n");
        let f2 = write(&tmp, "b.yaml", "cluster:\n  name: second\n");
        load_files(&store, &[f1.clone(), f2.clone()], &opts(None), &key).unwrap();

        let base = read_layer(&store, "cluster", "base");
        assert_eq!(
            crate::yaml::path::get_path(&base, "name"),
            Some(Value::String("second".into()))
        );
        // non-conflicting keys from the earlier file survive the deep merge
        assert_eq!(
            crate::yaml::path::get_path(&base, "only_a"),
            Some(Value::String("x".into()))
        );

        // registry reflects argument order
        let meta = crate::store::meta::read_meta(&store).unwrap();
        assert_eq!(meta.external.iter().map(|e| e.path.clone()).collect::<Vec<_>>(), vec![f1, f2]);
    }

    #[test]
    fn if_missing_skips_subjects_whose_layer_exists() {
        let (tmp, store) = init_store();
        let f = write(&tmp, "dc.yaml", "cluster:\n  name: noizu\n");
        let if_missing_opts =
            IngestOpts { layer: None, replace: false, replace_key: None, if_missing: true };

        load_files(&store, std::slice::from_ref(&f), &if_missing_opts, &key).unwrap();
        assert_eq!(
            crate::yaml::path::get_path(&read_layer(&store, "cluster", "base"), "name"),
            Some(Value::String("noizu".into()))
        );

        // a conflicting second file is skipped wholesale on re-load
        let f2 = write(&tmp, "b.yaml", "cluster:\n  name: clobbered\n");
        load_files(&store, &[f, f2], &if_missing_opts, &key).unwrap();        assert_eq!(
            crate::yaml::path::get_path(&read_layer(&store, "cluster", "base"), "name"),
            Some(Value::String("noizu".into()))
        );
    }
}
