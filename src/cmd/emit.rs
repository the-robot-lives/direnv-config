//! `dc emit` — emit a loadable YAML secret file from the store's subjects.
//!
//! The inverse of `dc load`: every subject in the current project's store is
//! re-emitted as a per-subject top-level group (nesting verbatim, 🔒 tokens
//! byte-identical under the at-rest `🔒 <token>` marked form, plain values
//! plain) so `dc load <file>` reconstructs the same store.
//!
//! Layer expressiveness: `_dc_layer` is per-subject routing (one string,
//! stripped at ingest), so one file cannot place one subject in two named
//! layers. But 🔒 values need no routing at all — the ingest auto-splits them
//! into the subject's `secrets` layer and stores already-encrypted tokens
//! verbatim. Emission therefore uses:
//!
//! - one **main file** (`secrets.dc.yaml` by default): base-layer content with
//!   secrets-layer tokens inlined — no `_dc_layer` needed;
//! - one **variant file per other layer** (`<out>.<layer>.yaml`): that layer's
//!   content, each subject body carrying `_dc_layer: <layer>`;
//! - a **`<out>.secrets.yaml` variant** only when a subject's secrets layer
//!   holds plain (non-token) values, which the auto-split cannot route;
//! - **special subjects** `_dc` (canonically loaded `--replace`) and `auto`
//!   (canonically loaded `--if-missing`) to their own files, because load
//!   flags are invocation-level and cannot ride in the file — the exact
//!   `dc load` invocation is printed for each.
//!
//! Safety: values are never printed; `--dry-run` prints counts only; and the
//! output paths are refused (unless `--force`) when they are not covered by
//! `.gitignore` — plaintext secret values must never land in a committable
//! file silently. Tokens are re-emitted byte-identical, so emit never needs
//! the encryption key.

use anyhow::{bail, Context, Result};
use serde_yaml::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::yaml::path::set_path;

/// Arguments for `dc emit` (see `main.rs`).
pub(crate) struct EmitArgs<'a> {
    pub output: Option<&'a str>,
    pub subjects: &'a [String],
    pub dry_run: bool,
    pub force: bool,
}

/// Subjects whose canonical `dc load` invocation carries a flag the file
/// format cannot express; they are emitted to their own files.
const SPECIAL_SUBJECTS: &[(&str, &str)] = &[("_dc", "--replace"), ("auto", "--if-missing")];

const DEFAULT_OUTPUT: &str = "secrets.dc.yaml";

/// One planned output file: target path, YAML body, and — for special
/// subjects — the `dc load` flag their re-load invocation needs.
struct Planned {
    path: PathBuf,
    yaml: String,
    solo_flag: Option<&'static str>,
}

/// Emit entry point.
pub fn run(args: EmitArgs) -> Result<()> {
    let store = crate::store::find_current_store()?;
    let subjects = discover_subjects(&store)?;
    if subjects.is_empty() {
        bail!("store has no subjects to emit");
    }
    let wanted = filter_subjects(&subjects, args.subjects)?;

    let output = args.output.unwrap_or(DEFAULT_OUTPUT);
    let main_path = PathBuf::from(output);

    // --- assemble per-subject content --------------------------------------
    let mut main_body: serde_yaml::Mapping = serde_yaml::Mapping::new();
    // layer -> (subject -> body); covers every layer except base and secrets
    let mut layer_bodies: BTreeMap<String, serde_yaml::Mapping> = BTreeMap::new();
    let mut summary: Vec<(String, String)> = Vec::new();

    for subject in &wanted {
        let layers = read_layers(&store, subject)?;

        let base = layers.get("base").cloned().unwrap_or_else(empty_mapping);
        let mut body = match base {
            Value::Mapping(m) => m,
            _other => bail!("subject `{subject}` base layer is not a mapping"),
        };

        let mut enc = 0usize;
        let mut plain_leaves = 0usize;

        // secrets layer: tokens ride inline in the main body (the ingest
        // auto-splits them into the secrets layer, verbatim); plain values
        // stored in secrets.yaml need the `_dc_layer: secrets` variant.
        if let Some(secrets) = layers.get("secrets") {
            let mut token_tree = empty_mapping();
            let mut plain_tree = empty_mapping();
            collect_secrets_leaves(secrets, "", &mut token_tree, &mut plain_tree, &mut enc, &mut plain_leaves);
            merge_into(&mut body, &token_tree);
            if let Value::Mapping(m) = plain_tree {
                if !m.is_empty() {
                    route_layer(&mut layer_bodies, "secrets", subject, Value::Mapping(m));
                }
            }
        }

        for (layer, val) in &layers {
            if layer == "base" || layer == "secrets" {
                continue;
            }
            if let Value::Mapping(m) = val {
                if m.is_empty() {
                    continue;
                }
                count_leaves(val, "", &mut plain_leaves);
                route_layer(&mut layer_bodies, layer, subject, val.clone());
            }
        }

        if body.is_empty() && enc == 0 && plain_leaves == 0 {
            continue; // nothing to emit for this subject
        }

        summary.push((
            subject.clone(),
            format!(
                "{} plain, {} encrypted token(s) across layer(s): {}",
                plain_leaves,
                enc,
                layers.keys().cloned().collect::<Vec<_>>().join(", ")
            ),
        ));
        main_body.insert(Value::String(subject.clone()), Value::Mapping(body));
    }

    // Special subjects move out of the main body into their own files.
    let mut special_files: Vec<(&str, serde_yaml::Mapping, &'static str)> = Vec::new();
    for (name, flag) in SPECIAL_SUBJECTS {
        if !wanted.iter().any(|s| s == name) {
            continue;
        }
        if let Some(Value::Mapping(m)) = main_body.remove(Value::String((*name).to_string())) {
            if !m.is_empty() {
                special_files.push((name, m, flag));
            }
        }
    }

    if main_body.is_empty() && layer_bodies.is_empty() && special_files.is_empty() {
        bail!("nothing to emit for the selected subject(s)");
    }

    // --- plan files ---------------------------------------------------------
    let mut plans: Vec<Planned> = Vec::new();
    if !main_body.is_empty() {
        plans.push(Planned {
            path: main_path.clone(),
            yaml: to_yaml(&Value::Mapping(main_body))?,
            solo_flag: None,
        });
    }
    for (layer, subjects_body) in &layer_bodies {
        plans.push(Planned {
            path: variant_path(&main_path, layer)?,
            yaml: to_yaml(&Value::Mapping(subjects_body.clone()))?,
            solo_flag: None,
        });
    }
    for (name, body, flag) in &special_files {
        // subject-keyed like any loadable file — the flag lives only on the
        // printed invocation
        let mut wrap = serde_yaml::Mapping::new();
        wrap.insert(
            Value::String((*name).to_string()),
            Value::Mapping(body.clone()),
        );
        plans.push(Planned {
            path: variant_path(&main_path, name)?,
            yaml: to_yaml(&Value::Mapping(wrap))?,
            solo_flag: Some(flag),
        });
    }

    // two plans must never share a target path
    let mut seen: Vec<PathBuf> = Vec::new();
    for p in &plans {
        if seen.contains(&p.path) {
            bail!(
                "output collision: {} would be written twice — pick a different --output",
                p.path.display()
            );
        }
        seen.push(p.path.clone());
    }

    // --- report -------------------------------------------------------------
    let cwd = std::env::current_dir()?;
    println!("emit: {} subject(s) from {}", wanted.len(), store.display());
    for (subject, line) in &summary {
        println!("  {subject}: {line}");
    }
    for p in &plans {
        println!("  → {}", abs(&cwd, &p.path).display());
    }

    // dry-run previews the plan and writes nothing, so it returns before the
    // safety gate: a preview of a not-yet-ignored path must not demand --force.
    if args.dry_run {
        println!("dry-run: {} file(s) — nothing written", plans.len());
        return Ok(());
    }

    // --- safety gate --------------------------------------------------------
    for p in &plans {
        ensure_safe_output(&p.path, args.force)?;
    }

    for p in &plans {
        // the gate may bless a gitignored target whose parents do not exist
        // yet — create them so the write lands instead of erroring
        if let Some(dir) = p.path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("failed to create parent dir for {}", p.path.display()))?;
        }
        std::fs::write(&p.path, &p.yaml)
            .with_context(|| format!("failed to write {}", p.path.display()))?;
    }

    // printed invocations: main + layer variants share one merge load (later
    // files win per layer; each variant only touches its own layer), specials
    // get the flag their store semantics need.
    let batch: Vec<String> = plans
        .iter()
        .filter(|p| p.solo_flag.is_none())
        .map(|p| p.path.display().to_string())
        .collect();
    if !batch.is_empty() {
        println!("\nre-load with:");
        println!("  dc load {}", batch.join(" "));
    }
    for p in plans.iter().filter(|p| p.solo_flag.is_some()) {
        println!("  dc load {} {}", p.solo_flag.unwrap(), p.path.display());
    }
    Ok(())
}

// --- helpers ---------------------------------------------------------------

fn empty_mapping() -> Value {
    Value::Mapping(serde_yaml::Mapping::new())
}

fn to_yaml(v: &Value) -> Result<String> {
    serde_yaml::to_string(v).context("failed to serialize emitted yaml")
}

fn abs(cwd: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}

/// `secrets.dc.yaml` + `local` → `secrets.dc.local.yaml`; a stem without a
/// `.yaml` extension simply gains `.<suffix>.yaml`.
fn variant_path(output: &Path, suffix: &str) -> Result<PathBuf> {
    if suffix.is_empty() || suffix.contains('/') || suffix.contains('.') {
        // keep emitted names unambiguous: dotted layer names would collide
        // with the stem/extension filename grammar
        bail!("cannot derive an output variant for `{suffix}` — pick a different --output");
    }
    let s = output.to_string_lossy().into_owned();
    let stem = s.strip_suffix(".yaml").unwrap_or(&s);
    Ok(PathBuf::from(format!("{stem}.{suffix}.yaml")))
}

fn discover_subjects(store: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(store)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "history" {
            continue;
        }
        out.push(name.into_owned());
    }
    out.sort();
    Ok(out)
}

fn filter_subjects(available: &[String], wanted: &[String]) -> Result<Vec<String>> {
    if wanted.is_empty() {
        return Ok(available.to_vec());
    }
    for w in wanted {
        if !available.contains(w) {
            bail!(
                "subject `{w}` not found in store (available: {})",
                available.join(", ")
            );
        }
    }
    // keep store order, restricted to the requested set
    Ok(available.iter().filter(|s| wanted.contains(s)).cloned().collect())
}

/// Parse every `*.yaml` layer file of one subject. A layer that exists but
/// fails to parse is a hard error — emit must never produce a lossy export.
fn read_layers(store: &Path, subject: &str) -> Result<BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    let dir = store.join(subject);
    for entry in std::fs::read_dir(&dir)
        .with_context(|| format!("failed to read subject dir {}", dir.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".yaml") else {
            continue;
        };
        let path = dir.join(&name);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read layer {}", path.display()))?;
        let val: Value = serde_yaml::from_str(&content).with_context(|| {
            format!("failed to parse layer {} (refusing a lossy emit)", path.display())
        })?;
        out.insert(stem.to_string(), val);
    }
    Ok(out)
}

/// Route a subject body into `layer_bodies[layer]`, prepending the reserved
/// `_dc_layer` key so `dc load` re-routes the subject to this layer (it is
/// stripped at ingest — locator::LAYER_KEY, consumed by load::ingest_subject).
fn route_layer(
    layer_bodies: &mut BTreeMap<String, serde_yaml::Mapping>,
    layer: &str,
    subject: &str,
    body: Value,
) {
    let mut routed = serde_yaml::Mapping::new();
    routed.insert(
        Value::String(crate::envrc::locator::LAYER_KEY.to_string()),
        Value::String(layer.to_string()),
    );
    if let Value::Mapping(m) = body {
        for (k, v) in m {
            routed.insert(k, v);
        }
    }
    let entry = layer_bodies.entry(layer.to_string()).or_default();
    entry.insert(Value::String(subject.to_string()), Value::Mapping(routed));
}

/// Merge `src` over `dst` in place (deep merge, src wins).
fn merge_into(dst: &mut serde_yaml::Mapping, src: &Value) {
    if let Value::Mapping(sm) = src {
        for (k, v) in sm {
            match dst.get_mut(k) {
                Some(Value::Mapping(dm)) if v.is_mapping() => merge_into(dm, v),
                _ => {
                    dst.insert(k.clone(), v.clone());
                }
            }
        }
    }
}

/// Walk a tree, splitting leaves into token (`🔒:v1`/legacy) and plain
/// subtrees at their original paths, counting both.
fn collect_secrets_leaves(
    val: &Value,
    prefix: &str,
    tokens: &mut Value,
    plain: &mut Value,
    enc: &mut usize,
    plain_count: &mut usize,
) {
    match val {
        Value::Mapping(m) => {
            for (k, v) in m {
                let key = k.as_str().unwrap_or_default();
                let path =
                    if prefix.is_empty() { key.to_string() } else { format!("{prefix}.{key}") };
                collect_secrets_leaves(v, &path, tokens, plain, enc, plain_count);
            }
        }
        Value::String(s) if crate::crypto::is_encrypted(s.trim()) => {
            *enc += 1;
            // Marked form `🔒 <token>`: parse_marker strips the marker on
            // re-ingest, build_encrypted_tree sees an already-encrypted body
            // and stores it verbatim (a bare token would be mistaken for
            // marked plaintext and double-processed).
            set_path(
                tokens,
                prefix,
                Value::String(format!("{} {}", crate::secret::PADLOCK, s.trim())),
            )
            .ok();
        }
        other => {
            *plain_count += 1;
            set_path(plain, prefix, other.clone()).ok();
        }
    }
}

fn count_leaves(val: &Value, prefix: &str, n: &mut usize) {
    match val {
        Value::Mapping(m) => {
            for (k, v) in m {
                let key = k.as_str().unwrap_or_default();
                let path =
                    if prefix.is_empty() { key.to_string() } else { format!("{prefix}.{key}") };
                count_leaves(v, &path, n);
            }
        }
        _ => *n += 1,
    }
}

// --- safety gate -----------------------------------------------------------

fn git(cwd: &Path, args: &[&str]) -> Result<std::process::Output> {
    std::process::Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run git: {e}"))
}

/// Resolve an output path to `(repo toplevel, toplevel-relative pathspec)` so
/// `git ls-files` / `git check-ignore` see the same form for relative and
/// absolute outputs (and for `..`-bearing ones, via canonicalization).
/// `None` = not a git work tree; the caller's check-ignore step reports that.
fn repo_relative(cwd: &Path, abs_path: &Path) -> Option<(PathBuf, String)> {
    let out = git(cwd, &["rev-parse", "--show-toplevel"]).ok()?;
    if !out.status.success() {
        return None;
    }
    let top = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    // macOS $TMPDIR-style symlink prefixes (/var → /private/var): resolve both
    // sides before comparing, falling back to the raw forms.
    let top_res = std::fs::canonicalize(&top).unwrap_or_else(|_| top.clone());

    // The output (and its parent directory) may not exist yet, so the parent
    // cannot be canonicalized directly. Canonicalize the nearest EXISTING
    // ancestor instead and resolve the nonexistent remainder lexically: the
    // pathspec handed to git must always be toplevel-relative — the raw
    // absolute fallback makes git hard-error ("outside repository"),
    // re-introducing the W1 failure this gate replaced.
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut probe = abs_path.to_path_buf();
    let anc = loop {
        match std::fs::canonicalize(&probe) {
            Ok(p) => break p,
            Err(_) => match probe.components().next_back() {
                Some(std::path::Component::Normal(name)) => {
                    tail.push(name.to_owned());
                    probe.pop();
                }
                Some(std::path::Component::ParentDir) => {
                    tail.push(std::ffi::OsString::from(".."));
                    probe.pop();
                }
                _ => return None, // no resolvable anchor within the path
            },
        }
    };

    // Lexical `.`/`..` resolution over the tail (shallowest first); a `..`
    // past the tail pops the canonical ancestor itself, so `resolved` never
    // carries dot components. No symlink assumptions: the tail names do not
    // exist, so there is nothing to resolve on disk.
    let mut parts: Vec<std::ffi::OsString> = Vec::new();
    let mut resolved = anc;
    for name in tail.into_iter().rev() {
        if name.as_os_str() == ".." {
            if parts.pop().is_none() && !resolved.pop() {
                return None;
            }
            continue;
        }
        if name.as_os_str() == "." {
            continue;
        }
        parts.push(name);
    }
    for part in parts {
        resolved.push(part);
    }

    let rel = resolved
        .strip_prefix(&top_res)
        .or_else(|_| resolved.strip_prefix(&top))
        .ok()?;
    Some((top_res, rel.to_string_lossy().into_owned()))
}

/// Refuse (unless `--force`) any output path that could be committed: tracked
/// by git, or not covered by `.gitignore`. Plaintext secret values must never
/// land in a committable file silently.
fn ensure_safe_output(path: &Path, force: bool) -> Result<()> {
    if force {
        return Ok(());
    }
    let cwd = std::env::current_dir()?;
    let display = path.display().to_string();

    // git pathspecs are cwd-relative: hand git a toplevel-relative form (run
    // from the toplevel) so absolute outputs and `..` components hit the same
    // gate as repo-relative ones.
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let (git_cwd, target) = match repo_relative(&cwd, &abs_path) {
        Some((top, rel)) => (top, rel),
        None => (cwd.clone(), abs_path.display().to_string()),
    };

    // tracked files are committable even when gitignored
    let ls = git(&git_cwd, &["ls-files", "--", &target])?;
    if !ls.status.success() {
        bail!(
            "cannot verify git coverage for {display} (git ls-files failed: {})",
            String::from_utf8_lossy(&ls.stderr).trim()
        );
    }
    if !String::from_utf8_lossy(&ls.stdout).trim().is_empty() {
        bail!(
            "refusing to write {display}: it is tracked by git — emitted secret files \
             must never be committable; untrack it or pass --force"
        );
    }

    let check = git(&git_cwd, &["check-ignore", "-q", "--", &target])?;
    match check.status.code() {
        Some(0) => Ok(()), // ignored → safe
        Some(1) => bail!(
            "refusing to write {display}: it is not covered by .gitignore — add a \
             .gitignore rule for it or pass --force"
        ),
        code => bail!(
            "cannot verify .gitignore coverage for {display} (git check-ignore exit \
             {code:?}; is this a git repository?) — pass --force to write anyway"
        ),
    }
}
