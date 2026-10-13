use anyhow::{bail, Result};
use std::io::Read;

// <REMOVED UUID HERE> run :: auto-generated pointer for public function run
#[allow(clippy::too_many_arguments)]
pub fn run(
    name: &str,
    key: &str,
    value: Option<&str>,
    stdin_flag: bool,
    secret: bool,
    plain: bool,
    layer: Option<&str>,
    no_bump: bool,
) -> Result<()> {
    if secret && plain {
        bail!("use only one of --secret / --plain");
    }
    let raw = acquire_value(value, stdin_flag)?;
    let as_secret = !plain;

    let cwd = std::env::current_dir()?;
    let store = crate::store::ensure_store(&cwd)?;
    let _lock = crate::store::lock_store(&store)?;
    crate::store::ensure_config(&store, name)?;

    let layer_name = layer.unwrap_or("local");
    let layer_file = crate::store::layout::layer_path(&store, name, layer_name);

    let mut doc = crate::store::load_layer(&layer_file)?;

    // Capture the current value (if any) before overwriting: a secret write
    // inherits the sensitivity tier of an existing token at this key.
    let current = crate::yaml::path::get_path(&doc, key);

    let stored = if as_secret {
        prepare_secret(&raw, current.as_ref(), &crate::settings::key()?)?
    } else {
        parse_plain(&raw)
    };

    crate::yaml::path::set_path(&mut doc, key, stored)?;

    let yaml_str = serde_yaml::to_string(&doc)?;
    std::fs::write(&layer_file, yaml_str)?;

    crate::store::resolve::resolve_active(&store, name)?;

    if !no_bump {
        crate::store::bump_version(&store)?;
    }

    Ok(())
}

/// Resolve the input source: the positional VALUE or `--stdin`, never both.
/// Mirrors `dc config set --stdin` trailing-newline handling.
fn acquire_value(value: Option<&str>, stdin_flag: bool) -> Result<String> {
    if stdin_flag && value.is_some() {
        bail!("use either the positional VALUE or --stdin, not both");
    }
    if !stdin_flag {
        return value.map(str::to_string).ok_or_else(|| {
            anyhow::anyhow!("provide a VALUE, or pass --stdin to read it from stdin")
        });
    }
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf.trim_end_matches('\n').to_string())
}

/// Secret handling (the default, or explicit `--secret`):
///
/// - input already a `dc` encrypted token → validated for shape and stored
///   verbatim (the store key is global, so decrypt-and-re-encrypt would be a
///   no-op that only churns ciphertext);
/// - otherwise → encrypted into a `🔒:v1` token, inheriting the sensitivity
///   tier of an existing token at the key (tier 0 for fresh keys).
///
/// The stored scalar is always a plain YAML string, so merge/flatten and the
/// SDKs pass it through unchanged; `dc get` decrypts/redacts downstream.
pub(crate) fn prepare_secret(
    raw: &str,
    current: Option<&serde_yaml::Value>,
    key: &[u8; 32],
) -> Result<serde_yaml::Value> {
    if crate::crypto::is_encrypted(raw) {
        if crate::crypto::token_tier(raw).is_none() {
            bail!("value looks like a dc encrypted token but is malformed (missing or bad tier marker)");
        }
        return Ok(serde_yaml::Value::String(raw.to_string()));
    }
    let tier = match current {
        Some(serde_yaml::Value::String(s)) if crate::crypto::is_encrypted(s) => {
            crate::crypto::token_tier(s).unwrap_or(0)
        }
        _ => 0,
    };
    let token = crate::crypto::encode_token(raw, tier, key)?;
    Ok(serde_yaml::Value::String(token))
}

/// Plain handling (`--plain`): store the value as given, preserving the
/// historical YAML scalar typing (numbers, bools).
pub(crate) fn parse_plain(raw: &str) -> serde_yaml::Value {
    serde_yaml::from_str(raw).unwrap_or(serde_yaml::Value::String(raw.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [7u8; 32];

    fn as_str(v: &serde_yaml::Value) -> &str {
        v.as_str().unwrap()
    }

    #[test]
    fn plain_preserves_yaml_typing() {
        assert_eq!(parse_plain("8080"), serde_yaml::Value::Number(8080.into()));
        assert_eq!(parse_plain("true"), serde_yaml::Value::Bool(true));
        assert_eq!(
            parse_plain("deploying"),
            serde_yaml::Value::String("deploying".into())
        );
    }

    #[test]
    fn secret_encrypts_and_roundtrips() {
        let v = prepare_secret("hunter2", None, &KEY).unwrap();
        let tok = as_str(&v);
        assert!(crate::crypto::is_encrypted(tok));
        let (tier, pt) = crate::crypto::decode_token(tok, &KEY).unwrap();
        assert_eq!((tier, pt.as_str()), (0, "hunter2"));
    }

    #[test]
    fn secret_inherits_existing_token_tier() {
        let existing = crate::crypto::encode_token("old", 2, &KEY).unwrap();
        let cur = serde_yaml::Value::String(existing);
        let v = prepare_secret("new", Some(&cur), &KEY).unwrap();
        let (tier, pt) = crate::crypto::decode_token(as_str(&v), &KEY).unwrap();
        assert_eq!((tier, pt.as_str()), (2, "new"));
    }

    #[test]
    fn secret_stores_existing_token_verbatim() {
        let tok = crate::crypto::encode_token("tok", 1, &KEY).unwrap();
        let v = prepare_secret(&tok, None, &KEY).unwrap();
        assert_eq!(as_str(&v), tok);
    }

    #[test]
    fn secret_rejects_malformed_token() {
        let err = prepare_secret("🔒:v1:zzz", None, &KEY).unwrap_err();
        assert!(err.to_string().contains("malformed"), "{err}");
    }

    #[test]
    fn plain_does_not_encrypt_token_shaped_input() {
        // --plain stores exactly what it is given, token or not.
        let v = parse_plain("🔒:v1:t0:abc");
        assert_eq!(as_str(&v), "🔒:v1:t0:abc");
    }
}
