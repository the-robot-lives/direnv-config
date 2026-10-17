use anyhow::Result;
use rand::Rng;
use std::io::Write;

const POINTER_PREFIX: &str = "🔗";
const MAX_POINTER_DEPTH: usize = 8;

/// Resolve a config path to its *raw stored* scalar (which may be a `🔒:`
/// token or the `⛔` restricted sentinel — callers decide how to handle it).
pub(crate) fn dc_lookup(name: &str, path: &str) -> Option<String> {
    dc_lookup_raw(name, path, MAX_POINTER_DEPTH)
}

fn dc_lookup_raw(name: &str, path: &str, depth: usize) -> Option<String> {
    let store = crate::store::find_current_store().ok()?;
    let chain = crate::store::resolve::resolve_chain(&store);
    let config = crate::store::resolve::resolve_config(&chain, name).ok()?;
    let val = crate::yaml::path::get_path(&config, path)?;
    let scalar = match val {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        serde_yaml::Value::Null => None,
        _ => Some(serde_yaml::to_string(&val).ok()?.trim().to_string()),
    }?;
    resolve_pointer(&scalar, depth)
}

fn resolve_pointer(val: &str, depth: usize) -> Option<String> {
    let trimmed = val.trim();
    if !trimmed.starts_with(POINTER_PREFIX) {
        return Some(val.to_string());
    }
    if depth == 0 {
        eprintln!("dc: pointer chain exceeded max depth ({})", MAX_POINTER_DEPTH);
        return None;
    }
    let rest = trimmed.strip_prefix(POINTER_PREFIX)?.trim();
    let mut parts = rest.splitn(2, ' ');
    let target_name = parts.next()?.trim();
    let target_path = parts.next()?.trim();
    if target_name.is_empty() || target_path.is_empty() {
        eprintln!("dc: malformed pointer (expected \"🔗 group path.to.key\"): {}", val);
        return None;
    }
    dc_lookup_raw(target_name, target_path, depth - 1)
}

/// Resolve a config path to its stored node (mapping, sequence or scalar).
/// Pointers (`🔗`) are resolved for scalar results only — a subtree is
/// returned as-is so `--reveal` can walk it.
fn dc_lookup_node(name: &str, path: &str) -> Option<serde_yaml::Value> {
    let store = crate::store::find_current_store().ok()?;
    let chain = crate::store::resolve::resolve_chain(&store);
    let config = crate::store::resolve::resolve_config(&chain, name).ok()?;
    let val = crate::yaml::path::get_path(&config, path)?;
    match &val {
        serde_yaml::Value::Mapping(_) | serde_yaml::Value::Sequence(_) => Some(val),
        serde_yaml::Value::String(s) => {
            resolve_pointer(s, MAX_POINTER_DEPTH).map(serde_yaml::Value::String)
        }
        serde_yaml::Value::Number(n) => Some(serde_yaml::Value::String(n.to_string())),
        serde_yaml::Value::Bool(b) => Some(serde_yaml::Value::String(b.to_string())),
        serde_yaml::Value::Null => None,
        _ => None,
    }
}

/// Resolve a config path to its decrypted plaintext (internal API for
/// compare/push/infisical). Decrypts `🔒:` tokens; passes plain values
/// through. Does NOT write an audit record — callers audit their own reveals.
pub(crate) fn get_secret_plaintext(name: &str, path: &str) -> anyhow::Result<Option<String>> {
    match dc_lookup(name, path) {
        None => Ok(None),
        Some(v) if crate::crypto::is_encrypted(&v) => {
            let key = crate::settings::key()?;
            let (_tier, pt) = crate::crypto::decode_token(&v, &key)?;
            Ok(Some(pt))
        }
        Some(v) => Ok(Some(v)),
    }
}

#[cfg(target_os = "macos")]
fn copy_to_clipboard(s: &str) -> anyhow::Result<()> {
    use std::io::Write as _;
    use std::process::{Command, Stdio};
    let mut child = Command::new("pbcopy").stdin(Stdio::piped()).spawn()?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("failed to open pbcopy stdin"))?
        .write_all(s.as_bytes())?;
    child.wait()?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn copy_to_clipboard(_s: &str) -> anyhow::Result<()> {
    anyhow::bail!("--clippy is only supported on macOS (pbcopy)")
}

fn print_out(s: &str, raw: bool) {
    if raw {
        print!("{}", s);
    } else {
        println!("{}", s);
    }
}

/// Render a resolved scalar to stdout, applying redaction/reveal/clippy for
/// encrypted secrets and the `⛔` restricted sentinel. Plain values pass through.
#[allow(clippy::too_many_arguments)]
fn emit_value(
    name: &str,
    path: &str,
    val: &str,
    raw: bool,
    reveal: bool,
    clippy: bool,
    reveal_restricted: bool,
) -> Result<()> {
    // Restricted (⛔ shadowed) value — stricter than a normal secret.
    if crate::secret::is_restricted_sentinel(val) {
        if reveal_restricted || clippy {
            match crate::shadow::resolve(name, path)? {
                Some(plain) => {
                    crate::audit::record(name, path, 3, "reveal-restricted");
                    if clippy {
                        copy_to_clipboard(&plain)?;
                        eprintln!("dc: ⛔ restricted secret copied to clipboard");
                        return Ok(());
                    }
                    print_out(&plain, raw);
                    return Ok(());
                }
                None => anyhow::bail!(
                    "restricted secret {name}.{path} has no entry in the shadow store"
                ),
            }
        }
        crate::audit::record(name, path, 3, "redact-restricted-attempt");
        print_out("⛔ **restricted**", raw);
        return Ok(());
    }

    // Encrypted secret token.
    if crate::crypto::is_encrypted(val) {
        let tier = crate::crypto::token_tier(val).unwrap_or(0);
        if reveal || clippy {
            let key = crate::settings::key()?;
            let (_t, plain) = crate::crypto::decode_token(val, &key)?;
            if clippy {
                copy_to_clipboard(&plain)?;
                crate::audit::record(name, path, tier, "clippy");
                eprintln!("dc: 🔒 secret copied to clipboard");
                return Ok(());
            }
            crate::audit::record(name, path, tier, "reveal");
            print_out(&plain, raw);
            return Ok(());
        }
        // Default: redact without ever touching the key.
        print_out(&crate::secret::redaction_for(tier), raw);
        return Ok(());
    }

    // Plain (non-secret) value.
    print_out(val, raw);
    Ok(())
}

/// Render one leaf of a subtree for `--reveal`: encrypted leaves decrypt
/// (audited like a scalar reveal), ⛔ leaves honour `reveal_restricted`,
/// pointers resolve, plain leaves pass through.
fn render_leaf(
    name: &str,
    leaf_path: &str,
    val: &str,
    reveal: bool,
    reveal_restricted: bool,
) -> Result<String> {
    // Restricted (⛔ shadowed) value — stricter than a normal secret.
    if crate::secret::is_restricted_sentinel(val) {
        if reveal_restricted {
            return match crate::shadow::resolve(name, leaf_path)? {
                Some(plain) => {
                    crate::audit::record(name, leaf_path, 3, "reveal-restricted");
                    Ok(plain)
                }
                None => anyhow::bail!(
                    "restricted secret {name}.{leaf_path} has no entry in the shadow store"
                ),
            };
        }
        crate::audit::record(name, leaf_path, 3, "redact-restricted-attempt");
        return Ok("⛔ **restricted**".to_string());
    }
    // Pointers resolve to their target scalar first (same as the scalar path).
    let val = resolve_pointer(val, MAX_POINTER_DEPTH).unwrap_or_else(|| val.to_string());
    if crate::crypto::is_encrypted(&val) {
        let tier = crate::crypto::token_tier(&val).unwrap_or(0);
        if reveal {
            let key = crate::settings::key()?;
            let (_t, plain) = crate::crypto::decode_token(&val, &key)
                .map_err(|e| anyhow::anyhow!("failed to decrypt {name}.{leaf_path}: {e}"))?;
            crate::audit::record(name, leaf_path, tier, "reveal");
            return Ok(plain);
        }
        // Without --reveal the tree is printed unchanged (tokens stay).
        return Ok(val);
    }
    Ok(val)
}

/// Recursively render a subtree for `--reveal`: every encrypted leaf is
/// decrypted (audited per leaf), plain leaves / structure / keys pass
/// through. A leaf that fails to decrypt aborts with a per-leaf error
/// before anything is printed.
fn reveal_subtree(
    name: &str,
    prefix: &str,
    val: &serde_yaml::Value,
    reveal: bool,
    reveal_restricted: bool,
) -> Result<serde_yaml::Value> {
    match val {
        serde_yaml::Value::Mapping(map) => {
            let mut out = serde_yaml::Mapping::new();
            for (k, v) in map {
                let child_prefix = match k.as_str() {
                    Some(key) if prefix.is_empty() => key.to_string(),
                    Some(key) => format!("{prefix}.{key}"),
                    // Non-string keys are passed through untouched.
                    None => String::new(),
                };
                let rendered = if child_prefix.is_empty() {
                    v.clone()
                } else {
                    reveal_subtree(name, &child_prefix, v, reveal, reveal_restricted)?
                };
                out.insert(k.clone(), rendered);
            }
            Ok(serde_yaml::Value::Mapping(out))
        }
        serde_yaml::Value::Sequence(seq) => {
            let mut out = Vec::with_capacity(seq.len());
            for (i, v) in seq.iter().enumerate() {
                out.push(reveal_subtree(
                    name,
                    &format!("{prefix}[{i}]"),
                    v,
                    reveal,
                    reveal_restricted,
                )?);
            }
            Ok(serde_yaml::Value::Sequence(out))
        }
        serde_yaml::Value::String(s) => Ok(serde_yaml::Value::String(render_leaf(
            name, prefix, s, reveal, reveal_restricted,
        )?)),
        other => Ok(other.clone()),
    }
}

/// Emit a resolved section (mapping/sequence): `--reveal`/`--clippy`
/// decrypt every encrypted leaf in the subtree; without them the caller
/// keeps the legacy raw dump. Output is the YAML of the (decrypted) subtree
/// — `--raw` is a no-op for sections (raw only affects scalar trailing
/// newline handling).
fn emit_subtree(
    name: &str,
    path: &str,
    val: &serde_yaml::Value,
    reveal: bool,
    clippy: bool,
    reveal_restricted: bool,
) -> Result<()> {
    let rendered = reveal_subtree(name, path, val, reveal || clippy, reveal_restricted)?;
    let body = serde_yaml::to_string(&rendered)?;
    if clippy {
        copy_to_clipboard(&body)?;
        crate::audit::record(name, path, 0, "clippy-section");
        eprintln!("dc: section {name}.{path} copied to clipboard (decrypted)");
        return Ok(());
    }
    print!("{body}");
    Ok(())
}

fn env_lookup(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|v| !v.is_empty())
}

// ---------------------------------------------------------------------------
// --keys (`--redact`): shape-only output
// ---------------------------------------------------------------------------

/// Markers replacing leaf VALUES in `--keys` mode. The shape (keys, nesting,
/// order) stays; no stored value or ciphertext token is ever printed and no
/// decryption happens — the key is never read.
const MARKER_ENCRYPTED: &str = "🔒";
const MARKER_SET: &str = "<set>";
const MARKER_EMPTY: &str = "<empty>";

/// Marker for one stored scalar in `--keys` mode. Pointers (`🔗`) resolve
/// first (keylessly — resolution never decrypts): an encrypted target →
/// 🔒, the `⛔` restricted sentinel and every other non-empty scalar →
/// `<set>`, "" (and null/typed-null leaves) → `<empty>`.
// <REMOVED UUID HERE> keys_marker :: Marker for one stored scalar in --keys mode.
fn keys_marker(raw: &str) -> String {
    let val = resolve_pointer(raw, MAX_POINTER_DEPTH).unwrap_or_else(|| raw.to_string());
    if crate::crypto::is_encrypted(&val) {
        MARKER_ENCRYPTED.to_string()
    } else if val.is_empty() {
        MARKER_EMPTY.to_string()
    } else {
        MARKER_SET.to_string()
    }
}

/// Recursively replace every leaf of a subtree with its `--keys` marker.
/// Mapping keys, nesting and insertion order are preserved; comments are
/// dropped (the output is a fresh YAML render, like `--reveal`'s).
// <REMOVED UUID HERE> redact_subtree :: Recursively replace every leaf of a subtree with its --keys marker.
fn redact_subtree(val: &serde_yaml::Value) -> serde_yaml::Value {
    match val {
        serde_yaml::Value::Mapping(map) => {
            let mut out = serde_yaml::Mapping::new();
            for (k, v) in map {
                out.insert(k.clone(), redact_subtree(v));
            }
            serde_yaml::Value::Mapping(out)
        }
        serde_yaml::Value::Sequence(seq) => {
            serde_yaml::Value::Sequence(seq.iter().map(redact_subtree).collect())
        }
        serde_yaml::Value::String(s) => serde_yaml::Value::String(keys_marker(s)),
        serde_yaml::Value::Number(_) | serde_yaml::Value::Bool(_) => {
            serde_yaml::Value::String(MARKER_SET.to_string())
        }
        serde_yaml::Value::Null => serde_yaml::Value::String(MARKER_EMPTY.to_string()),
        // Tagged/other exotic nodes carry a non-empty value of some kind.
        _ => serde_yaml::Value::String(MARKER_SET.to_string()),
    }
}

/// Append one `path<TAB>marker` line per leaf of an ALREADY-REDACTED tree.
/// List items keep their position as `[i]` — the same syntax `dc get`'s
/// path parser accepts, so every printed path round-trips into a lookup.
// <REMOVED UUID HERE> flat_walk :: Append one `path<TAB>marker` line per leaf of a redacted tree.
fn flat_walk(prefix: &str, val: &serde_yaml::Value, out: &mut Vec<String>) {
    match val {
        serde_yaml::Value::Mapping(map) => {
            for (k, v) in map {
                let key = k.as_str().map(str::to_string).unwrap_or_else(|| {
                    serde_yaml::to_string(k)
                        .unwrap_or_default()
                        .trim()
                        .to_string()
                });
                let child = if prefix.is_empty() {
                    key
                } else {
                    format!("{prefix}.{key}")
                };
                flat_walk(&child, v, out);
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            for (i, v) in seq.iter().enumerate() {
                flat_walk(&format!("{prefix}[{i}]"), v, out);
            }
        }
        other => {
            let marker = other.as_str().unwrap_or(MARKER_SET);
            out.push(format!("{prefix}\t{marker}"));
        }
    }
}

fn emit_flat(prefix: &str, redacted: &serde_yaml::Value) {
    let mut lines = Vec::new();
    flat_walk(prefix, redacted, &mut lines);
    for line in lines {
        println!("{line}");
    }
}

/// `--keys` (`--redact`): print the shape of a subject/section/leaf with
/// every leaf value replaced by a marker. Never decrypts and never prints a
/// stored value or ciphertext token. A missing path exits 1 (same contract
/// as the scalar path). `--override`/`--fallback`/`--auto`/`--default` do
/// not apply — the stored config is the only source being shown.
// <REMOVED UUID HERE> run_keys :: --keys redacted structure output for get.
fn run_keys(name: &str, path: Option<&str>, flat: bool) -> Result<()> {
    let store = crate::store::find_current_store()?;
    let chain = crate::store::resolve::resolve_chain(&store);
    let config = crate::store::resolve::resolve_config(&chain, name)?;
    match path {
        None => {
            let redacted = redact_subtree(&config);
            if flat {
                emit_flat(name, &redacted);
            } else {
                print!("{}", serde_yaml::to_string(&redacted)?);
            }
        }
        Some(p) => {
            let Some(node) = crate::yaml::path::get_path(&config, p) else {
                std::process::exit(1);
            };
            let prefix = format!("{name}.{p}");
            match node {
                serde_yaml::Value::Mapping(_) | serde_yaml::Value::Sequence(_) => {
                    let redacted = redact_subtree(&node);
                    if flat {
                        emit_flat(&prefix, &redacted);
                    } else {
                        print!("{}", serde_yaml::to_string(&redacted)?);
                    }
                }
                // Scalar leaf: `path: <marker>` (or the flat one-liner).
                scalar => {
                    let marker = redact_subtree(&scalar)
                        .as_str()
                        .unwrap_or(MARKER_SET)
                        .to_string();
                    if flat {
                        println!("{prefix}\t{marker}");
                    } else {
                        println!("{prefix}: {marker}");
                    }
                }
            }
        }
    }
    Ok(())
}


fn auto_generate(gen_type: &str, length: usize) -> String {
    let mut rng = rand::thread_rng();
    match gen_type {
        "hex" => {
            let bytes: Vec<u8> = (0..length).map(|_| rng.gen()).collect();
            bytes.iter().map(|b| format!("{:02x}", b)).collect()
        }
        // "password" and any unrecognised format
        _ => {
            const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            (0..length)
                .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
                .collect()
        }
    }
}

fn persist_auto(env_var: &str, value: &str) {
    let store = match crate::store::find_current_store() {
        Ok(s) => s,
        Err(_) => return,
    };
    let meta = match crate::store::meta::read_meta(&store) {
        Ok(m) => m,
        Err(_) => return,
    };
    let auto_path = meta.source.join("secrets/.envrc.auto");
    if let Ok(contents) = std::fs::read_to_string(&auto_path) {
        if contents.contains(&format!("export {}=", env_var)) {
            return;
        }
    }
    if let Some(parent) = auto_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&auto_path) {
        let _ = writeln!(f, "export {}=\"{}\"", env_var, value);
    }
}

#[allow(clippy::too_many_arguments)]
// <REMOVED UUID HERE> run :: auto-generated pointer for public function run
pub fn run(
    name: &str,
    path: Option<&str>,
    raw: bool,
    override_var: Option<&str>,
    fallback_var: Option<&str>,
    auto: &[String],
    default: Option<&str>,
    reveal: bool,
    clippy: bool,
    reveal_restricted: bool,
    keys: bool,
    flat: bool,
) -> Result<()> {
    // --keys: shape-only output — handled before any value resolution;
    // never decrypts, never prints a stored value or ciphertext token.
    if keys {
        return run_keys(name, path, flat);
    }

    let env_var_name = override_var.or(fallback_var);

    // No path → dump whole config (original behavior)
    let Some(p) = path else {
        let store = crate::store::find_current_store()?;
        let chain = crate::store::resolve::resolve_chain(&store);
        let config = crate::store::resolve::resolve_config(&chain, name)?;
        print!("{}", serde_yaml::to_string(&config)?);
        return Ok(());
    };

    // Sections (mappings/sequences) under --reveal/--clippy: decrypt every
    // encrypted leaf in the subtree. Without those flags the legacy raw dump
    // is kept byte-identical (tokens visible).
    if reveal || clippy {
        if let Some(node) = dc_lookup_node(name, p) {
            if node.is_mapping() || node.is_sequence() {
                return emit_subtree(name, p, &node, reveal, clippy, reveal_restricted);
            }
        }
    }

    // Resolution: --override (ENV → dc) or --fallback (dc → ENV) or dc-only
    let resolved = if let Some(var) = override_var {
        env_lookup(var).or_else(|| dc_lookup(name, p))
    } else if let Some(var) = fallback_var {
        dc_lookup(name, p).or_else(|| env_lookup(var))
    } else {
        dc_lookup(name, p)
    };

    if let Some(val) = resolved {
        return emit_value(name, p, &val, raw, reveal, clippy, reveal_restricted);
    }

    // --auto: generate + persist to .envrc.auto
    if !auto.is_empty() {
        let gen_type = auto.first().map(|s| s.as_str()).unwrap_or("password");
        let length: usize = auto.get(1).and_then(|s| s.parse().ok()).unwrap_or(32);
        let val = auto_generate(gen_type, length);

        if let Some(var) = env_var_name {
            persist_auto(var, &val);
        }

        if raw { print!("{}", val); } else { println!("{}", val); }
        return Ok(());
    }

    // --default: static fallback
    if let Some(val) = default {
        if raw { print!("{}", val); } else { println!("{}", val); }
        return Ok(());
    }

    std::process::exit(1);
}
