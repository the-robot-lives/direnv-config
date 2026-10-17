use anyhow::Result;
use serde_yaml::Value;
use std::io::Read;
use std::path::Path;

// <REMOVED UUID HERE> run :: auto-generated pointer for public function run
pub fn run(
    name: &str,
    layer: Option<&str>,
    replace: bool,
    replace_key: Option<&str>,
    if_missing: bool,
    no_bump: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let store = crate::store::ensure_store(&cwd)?;
    let _lock = crate::store::lock_store(&store)?;
    crate::store::ensure_config(&store, name)?;

    let layer_name = layer.unwrap_or("base");
    let layer_file = crate::store::layout::layer_path(&store, name, layer_name);

    if if_missing && layer_file.exists() {
        return Ok(());
    }

    let mut stdin_buf = String::new();
    std::io::stdin().read_to_string(&mut stdin_buf)?;
    let input: Value = serde_yaml::from_str(&stdin_buf)?;

    // Separate 🔒-marked secret scalars (at any depth) from plain settings.
    let (plain, secrets) = crate::secret::split_secrets(&input);

    // Secrets require the AEAD key from settings.yaml — load it only when needed
    // so plain heredocs never depend on a configured key.
    let key = if secrets.is_empty() {
        None
    } else {
        Some(crate::settings::key()?)
    };

    apply(
        &store,
        name,
        layer_name,
        &plain,
        &secrets,
        key.as_ref(),
        replace,
        replace_key,
    )?;

    // Resolve active for this config
    crate::store::resolve::resolve_active(&store, name)?;
    crate::store::meta::update_configs_list(&store)?;

    if !no_bump {
        crate::store::bump_version(&store)?;
    }

    Ok(())
}

/// Write a parsed heredoc into the store: the plain (non-secret) portion goes to
/// the requested `layer`, while 🔒 secrets are encrypted and always merged into
/// the `secrets` layer regardless of the requested layer.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply(
    store: &Path,
    name: &str,
    layer_name: &str,
    plain: &Value,
    secrets: &[(String, crate::secret::ParsedSecret)],
    key: Option<&[u8; 32]>,
    replace: bool,
    replace_key: Option<&str>,
) -> Result<()> {
    crate::store::ensure_config(store, name)?;

    let plain_is_empty = matches!(plain, Value::Mapping(m) if m.is_empty());

    // Write the plain portion to the requested layer. Skip the write only when
    // the heredoc was *entirely* secrets, to avoid creating an empty `{}` file.
    if !plain_is_empty || secrets.is_empty() {
        let layer_file = crate::store::layout::layer_path(store, name, layer_name);
        write_layer(&layer_file, plain, replace, replace_key)?;
    }

    // 🔒 secrets → encrypted → secrets.yaml (always), deep-merged in.
    if !secrets.is_empty() {
        let key = key.ok_or_else(|| {
            anyhow::anyhow!("internal: encryption key required to store 🔒 secrets")
        })?;
        let enc_tree = crate::secret::build_encrypted_tree(secrets, key)?;
        let secrets_file = crate::store::layout::layer_path(store, name, "secrets");
        let existing = crate::store::load_layer(&secrets_file)?;
        let merged = crate::yaml::merge::deep_merge(&existing, &enc_tree);
        std::fs::write(&secrets_file, serde_yaml::to_string(&merged)?)?;
    }

    // A plain arrival retires the stale ghost: if this path was 🔒 before and
    // now arrives plain, the old token left in the secrets layer would keep
    // outranking the fresh plain value on every resolve (the secrets overlay
    // wins over lower layers). Skipped when the caller explicitly targets the
    // secrets layer — there the plain keys BELONG in secrets.yaml.
    if !plain_is_empty && layer_name != "secrets" {
        clear_shadowed_secrets(store, name, plain)?;
    }

    Ok(())
}

/// Delete every leaf path of the incoming plain tree from the `secrets` layer
/// file (pruning now-empty parent maps). Paths the plain tree does not mention
/// are never touched, so unrelated secrets survive; covers `dc load`,
/// `dc yaml`, and every `dc config set` mode via the shared ingest pipeline.
fn clear_shadowed_secrets(store: &Path, name: &str, plain: &Value) -> Result<()> {
    let secrets_file = crate::store::layout::layer_path(store, name, "secrets");
    if !secrets_file.exists() {
        return Ok(());
    }
    let mut existing = crate::store::load_layer(&secrets_file)?;
    let mut changed = false;
    for path in plain_leaf_paths(plain) {
        if crate::yaml::path::delete_path(&mut existing, &path) {
            changed = true;
        }
    }
    if changed {
        prune_empty_mappings(&mut existing);
        std::fs::write(&secrets_file, serde_yaml::to_string(&existing)?)?;
    }
    Ok(())
}

/// Dot-paths of every non-mapping node in `plain` — the values that arrived
/// plain from this ingest. `⛔` sentinels are excluded: `dc config secure`
/// deliberately places that sentinel in the secrets layer, and the plain `⛔`
/// a later refresh re-ingests from the source must not delete it.
fn plain_leaf_paths(value: &Value) -> Vec<String> {
    fn walk(v: &Value, prefix: &str, out: &mut Vec<String>) {
        match v {
            Value::Mapping(m) => {
                for (k, child) in m {
                    let key = k.as_str().unwrap_or_default();
                    let path = if prefix.is_empty() {
                        key.to_string()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    walk(child, &path, out);
                }
            }
            Value::String(s) if crate::secret::is_restricted_sentinel(s) => {}
            _ => out.push(prefix.to_string()),
        }
    }
    let mut out = Vec::new();
    walk(value, "", &mut out);
    out
}

/// Recursively drop mappings that became empty after leaf deletion (bottom-up,
/// so a branch whose whole subtree was cleared disappears too).
fn prune_empty_mappings(value: &mut Value) {
    if let Value::Mapping(m) = value {
        for (_, child) in m.iter_mut() {
            prune_empty_mappings(child);
        }
        m.retain(|_, child| !matches!(child, Value::Mapping(cm) if cm.is_empty()));
    }
}

fn write_layer(layer_file: &Path, input: &Value, replace: bool, replace_key: Option<&str>) -> Result<()> {
    if replace {
        std::fs::write(layer_file, serde_yaml::to_string(input)?)?;
    } else if let Some(rk) = replace_key {
        // Replace just one branch: load existing, replace that key, write back.
        let mut existing = crate::store::load_layer(layer_file)?;
        if let (Value::Mapping(ref mut emap), Value::Mapping(ref imap)) = (&mut existing, input) {
            let key_val = Value::String(rk.to_string());
            if let Some(new_val) = imap.get(&key_val) {
                emap.insert(key_val, new_val.clone());
            }
        }
        std::fs::write(layer_file, serde_yaml::to_string(&existing)?)?;
    } else {
        // Deep merge.
        let existing = crate::store::load_layer(layer_file)?;
        let merged = crate::yaml::merge::deep_merge(&existing, input);
        std::fs::write(layer_file, serde_yaml::to_string(&merged)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn plain_leaf_paths_enumerates_leaves_skips_sentinels() {
        let v: Value =
            serde_yaml::from_str("db:\n  host: h\n  port: 5432\n  password: \"⛔\"\nflag: true\n")
                .unwrap();
        let mut paths = plain_leaf_paths(&v);
        paths.sort();
        assert_eq!(
            paths,
            vec!["db.host", "db.port", "flag"],
            "⛔ leaf excluded"
        );
    }

    #[test]
    fn clear_shadowed_secrets_prunes_empty_parents_and_keeps_unrelated() {
        let tmp = TempDir::new().unwrap();
        let store = tmp.path();
        let secrets: Value = serde_yaml::from_str(
            "db:\n  password: \"🔒:v1:ghost\"\n  host: \"🔒:v1:keep\"\nother: \"🔒:v1:stay\"\n",
        )
        .unwrap();
        let secrets_file = crate::store::layout::layer_path(store, "cf", "secrets");
        std::fs::create_dir_all(secrets_file.parent().unwrap()).unwrap();
        std::fs::write(&secrets_file, serde_yaml::to_string(&secrets).unwrap()).unwrap();

        // Plain arrival for db.password only.
        let plain: Value = serde_yaml::from_str("db:\n  password: new-plain\n").unwrap();
        clear_shadowed_secrets(store, "cf", &plain).unwrap();

        let after = read(store, "cf", "secrets");
        assert!(
            crate::yaml::path::get_path(&after, "db.password").is_none(),
            "ghost cleared: {}",
            serde_yaml::to_string(&after).unwrap()
        );
        assert!(
            crate::yaml::path::get_path(&after, "db.host").is_some(),
            "unrelated sibling untouched: {}",
            serde_yaml::to_string(&after).unwrap()
        );
        assert!(
            crate::yaml::path::get_path(&after, "other").is_some(),
            "unmentioned key untouched: {}",
            serde_yaml::to_string(&after).unwrap()
        );

        // Clearing the last leaf of a branch removes the empty parent map too.
        let plain2: Value = serde_yaml::from_str("db:\n  host: plain-h\n").unwrap();
        clear_shadowed_secrets(store, "cf", &plain2).unwrap();
        let after2 = read(store, "cf", "secrets");
        assert_eq!(
            serde_yaml::to_string(&after2).unwrap(),
            "other: 🔒:v1:stay\n",
            "empty db: parent pruned: {}",
            serde_yaml::to_string(&after2).unwrap()
        );
    }

    const KEY: [u8; 32] = [9u8; 32];

    fn ingest(store: &Path, name: &str, layer: &str, yaml: &str) {
        let input: Value = serde_yaml::from_str(yaml).unwrap();
        let (plain, secrets) = crate::secret::split_secrets(&input);
        apply(store, name, layer, &plain, &secrets, Some(&KEY), false, None).unwrap();
    }

    fn read(store: &Path, name: &str, layer: &str) -> Value {
        crate::store::load_layer(&crate::store::layout::layer_path(store, name, layer)).unwrap()
    }

    #[test]
    fn mixed_heredoc_fans_out_base_and_secrets() {
        let tmp = TempDir::new().unwrap();
        let store = tmp.path();
        ingest(
            store,
            "cf",
            "base",
            "account_id: 123\naccess:\n  client_id: pub\n  client_secret: \"🔒 shh\"",
        );

        // Plain keys land in base.yaml; the secret leaf does not.
        let base = read(store, "cf", "base");
        assert_eq!(
            crate::yaml::path::get_path(&base, "account_id"),
            Some(Value::Number(123.into()))
        );
        assert_eq!(
            crate::yaml::path::get_path(&base, "access.client_id"),
            Some(Value::String("pub".into()))
        );
        assert!(crate::yaml::path::get_path(&base, "access.client_secret").is_none());

        // Secret lands encrypted in secrets.yaml.
        let secrets = read(store, "cf", "secrets");
        let tok = crate::yaml::path::get_path(&secrets, "access.client_secret").unwrap();
        let tok = tok.as_str().unwrap();
        assert!(crate::crypto::is_encrypted(tok));
        assert_eq!(crate::crypto::decode_token(tok, &KEY).unwrap().1, "shh");
    }

    #[test]
    fn all_secret_heredoc_creates_no_empty_base() {
        let tmp = TempDir::new().unwrap();
        let store = tmp.path();
        ingest(store, "cf", "base", "api_token: \"🔒 t\"");
        // base.yaml should not be created for an all-secret heredoc.
        assert!(!crate::store::layout::layer_path(store, "cf", "base").exists());
        assert!(crate::store::layout::layer_path(store, "cf", "secrets").exists());
    }

    #[test]
    fn replace_layer_does_not_wipe_unrelated_secrets() {
        let tmp = TempDir::new().unwrap();
        let store = tmp.path();
        // First seed a secret.
        ingest(store, "cf", "base", "api_token: \"🔒 first\"");
        // Now a --replace write of plain settings (with a new secret) must keep
        // merging secrets, not clobber the existing secret tree.
        let input: Value =
            serde_yaml::from_str("zone_id: z1\nother_secret: \"🔒 second\"").unwrap();
        let (plain, secrets) = crate::secret::split_secrets(&input);
        apply(store, "cf", "base", &plain, &secrets, Some(&KEY), true, None).unwrap();

        let secrets_layer = read(store, "cf", "secrets");
        // Both the original and the new secret survive.
        assert!(crate::yaml::path::get_path(&secrets_layer, "api_token").is_some());
        assert!(crate::yaml::path::get_path(&secrets_layer, "other_secret").is_some());
        // The base layer was replaced with just the plain settings.
        let base = read(store, "cf", "base");
        assert_eq!(
            crate::yaml::path::get_path(&base, "zone_id"),
            Some(Value::String("z1".into()))
        );
    }

    #[test]
    fn bare_seal_token_loads_verbatim_without_double_wrap() {
        let tmp = TempDir::new().unwrap();
        let store = tmp.path();
        let bare = crate::crypto::encode_token("shh-bare", 1, &KEY).unwrap();
        let marked = crate::crypto::encode_token("shh-marked", 2, &KEY).unwrap();
        ingest(
            store,
            "cf",
            "base",
            &format!("bare_token: \"{bare}\"\nmarked_token: \"🔒 {marked}\""),
        );
        let secrets = read(store, "cf", "secrets");
        // The bare `🔒:v1:...` token (no space after the padlock) must be
        // recognized as already-encrypted and stored verbatim — never
        // re-encrypted into an unresolvable double wrap.
        assert_eq!(
            crate::yaml::path::get_path(&secrets, "bare_token").unwrap(),
            Value::String(bare)
        );
        // The safe marked form `🔒 <token>` keeps storing the token verbatim.
        assert_eq!(
            crate::yaml::path::get_path(&secrets, "marked_token").unwrap(),
            Value::String(marked)
        );
    }

    #[test]
    fn layer_secrets_with_padlock_both_target_secrets_file() {
        let tmp = TempDir::new().unwrap();
        let store = tmp.path();
        // Caller explicitly targets the secrets layer with a mix of plain + 🔒.
        ingest(store, "cf", "secrets", "plain_in_secrets: v\nenc: \"🔒 e\"");
        let secrets = read(store, "cf", "secrets");
        // Plain key written to secrets.yaml as-is (caller's explicit layer choice).
        assert_eq!(
            crate::yaml::path::get_path(&secrets, "plain_in_secrets"),
            Some(Value::String("v".into()))
        );
        // 🔒 value encrypted in the same file.
        let tok = crate::yaml::path::get_path(&secrets, "enc").unwrap();
        assert!(crate::crypto::is_encrypted(tok.as_str().unwrap()));
    }
}
