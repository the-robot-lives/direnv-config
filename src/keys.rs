//! External, lockable key store for `dc` secret encryption.
//!
//! Keys live in a dedicated file (`$DC_KEYS_FILE` →
//! `$XDG_CONFIG_HOME/direnv-config/keys` → `~/.config/direnv-config/keys`),
//! created 0600, so it can be root-owned and write/delete-protected
//! (`dc keys lock`) independently of the settings and store files. Ciphertext
//! in the stores never carries key material; this file is the single holder.
//!
//! Resolution order:
//! 1. key file exists → use it exclusively (fail loud if unreadable/invalid;
//!    never silently fall back to legacy)
//! 2. key file absent → legacy: `key:` inside settings.yaml (transparent read)
//!
//! `dc keys migrate` performs the legacy → external transition with a
//! full decrypt round-trip verification before stripping the key from
//! settings.yaml.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Key-file schema version.
pub const KEY_FILE_VERSION: u64 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct KeyFile {
    version: u64,
    /// base64 (standard) encoded 32-byte XChaCha20-Poly1305 key.
    key: String,
}

/// Where the active key came from.
#[derive(Debug)]
pub enum KeySource {
    /// Dedicated lockable key file.
    External { path: PathBuf, key: [u8; 32] },
    /// Legacy `key:` embedded in settings.yaml.
    Legacy { path: PathBuf, key: [u8; 32] },
}

impl KeySource {
    pub fn key(&self) -> &[u8; 32] {
        match self {
            KeySource::External { key, .. } | KeySource::Legacy { key, .. } => key,
        }
    }
}

/// Resolve the key-file path: `$DC_KEYS_FILE` → `$XDG_CONFIG_HOME/direnv-config/keys` → `~/.config/direnv-config/keys`.
// ⟦𓇑𓎕𓊽𓆑⟧ keys_path :: Resolve the key-file path.
pub fn keys_path() -> PathBuf {
    if let Ok(p) = std::env::var("DC_KEYS_FILE") {
        return PathBuf::from(p);
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("direnv-config").join("keys");
    }
    dirs::home_dir()
        .map(|h| h.join(".config").join("direnv-config").join("keys"))
        .unwrap_or_else(|| PathBuf::from(".config/direnv-config/keys"))
}

/// Parse + validate a key file's contents.
// ⟦𓃓𓆣𓍝𓄿⟧ parse_key_file :: Parse + validate a key file's contents.
fn parse_key_file(raw: &str, path: &Path) -> Result<[u8; 32]> {
    let parsed: KeyFile = serde_yaml::from_str(raw)
        .with_context(|| format!("parsing key file {}", path.display()))?;
    if parsed.version != KEY_FILE_VERSION {
        return Err(anyhow!(
            "key file {} has unsupported version {} (expected {})",
            path.display(),
            parsed.version,
            KEY_FILE_VERSION
        ));
    }
    decode_key_b64(&parsed.key).with_context(|| format!("key file {}", path.display()))
}

fn decode_key_b64(b64: &str) -> Result<[u8; 32]> {
    let bytes = crate::crypto::b64_standard_decode(b64.trim())
        .with_context(|| "decoding base64 key material")?;
    if bytes.len() != 32 {
        return Err(anyhow!("key must be 32 bytes after base64-decode (got {})", bytes.len()));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

/// Load a key file from an explicit path (0600 expected; mode is advisory here, enforced by `dc keys lock`).
// ⟦𓎛𓍼𓋴𓁷⟧ load_key_file :: Load a key file from an explicit path.
pub fn load_key_file(path: &Path) -> Result<[u8; 32]> {
    let raw = std::fs::read_to_string(path).with_context(|| {
        format!(
            "reading key file {} — it exists but is unreadable; check ownership/permissions (`dc keys status`)",
            path.display()
        )
    })?;
    parse_key_file(&raw, path)
}

/// Write a key file with 0600 permissions (atomic temp+rename).
// ⟦𓍻𓊪𓇋𓎏⟧ write_key_file :: Write a key file with 0600 permissions (atomic temp+rename).
pub fn write_key_file(path: &Path, key: &[u8; 32]) -> Result<()> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(key);
    let body = format!(
        "# dc encryption keys — separate lockable file; harden with `dc keys lock`\nversion: {}\nkey: {}\n",
        KEY_FILE_VERSION, b64
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, body).with_context(|| format!("writing {}", tmp.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, path).with_context(|| format!("renaming into {}", path.display()))?;
    Ok(())
}

/// Read the legacy key from settings.yaml, if present.
// ⟦𓊨𓄗𓇢𓆰⟧ legacy_key :: Read the legacy key from settings.yaml, if present.
pub fn legacy_key() -> Result<Option<[u8; 32]>> {
    let path = crate::settings::settings_path();
    let raw = match std::fs::read_to_string(&path) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    let parsed: crate::settings::SettingsFile = serde_yaml::from_str(&raw)
        .with_context(|| format!("parsing settings file {}", path.display()))?;
    match parsed.key {
        Some(b64) => Ok(Some(decode_key_b64(&b64)?)),
        None => Ok(None),
    }
}

/// Resolve the active key: external key file wins exclusively; otherwise legacy settings.yaml.
// ⟦𓁽𓍺𓎞𓋹⟧ resolve :: Resolve the active key.
pub fn resolve() -> Result<KeySource> {
    let kpath = keys_path();
    if kpath.exists() {
        let key = load_key_file(&kpath)?;
        return Ok(KeySource::External { path: kpath, key });
    }
    let spath = crate::settings::settings_path();
    match legacy_key()? {
        Some(key) => Ok(KeySource::Legacy { path: spath, key }),
        None => Err(anyhow!(
            "no encryption key found — expected key file {} (create via `dc keys migrate`) or `key:` in {}",
            kpath.display(),
            spath.display()
        )),
    }
}

/// Unix permission bits of a path (0 if unavailable).
#[cfg(unix)]
pub fn mode_of(path: &Path) -> u32 {
    std::fs::metadata(path)
        .map(|m| {
            use std::os::unix::fs::PermissionsExt;
            m.permissions().mode()
        })
        .unwrap_or(0)
}

/// True if the file is readable by groups/others (weak permissions for key material).
#[cfg(unix)]
pub fn weak_perms(path: &Path) -> bool {
    mode_of(path) & 0o077 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("dc-keys-test-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn key_file_round_trip() {
        let path = tmp_dir().join("keys-rt");
        let key = [9u8; 32];
        write_key_file(&path, &key).unwrap();
        #[cfg(unix)]
        assert_eq!(mode_of(&path) & 0o777, 0o600, "key file must be 0600");
        let loaded = load_key_file(&path).unwrap();
        assert_eq!(loaded, key);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn bad_version_rejected() {
        let raw = "version: 99\nkey: {}\n";
        assert!(parse_key_file(raw, Path::new("x")).is_err());
    }

    #[test]
    fn bad_length_rejected() {
        use base64::Engine;
        let short = base64::engine::general_purpose::STANDARD.encode([1u8; 16]);
        let raw = format!("version: {KEY_FILE_VERSION}\nkey: {short}\n");
        assert!(parse_key_file(&raw, Path::new("x")).is_err());
    }

    #[test]
    fn corrupt_key_file_fails_loud_not_silent() {
        let path = tmp_dir().join("keys-corrupt");
        std::fs::write(&path, "version: 1\nkey: not-base64!!\n").unwrap();
        assert!(load_key_file(&path).is_err());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn missing_everything_errors_with_paths() {
        // No key file, no settings — resolve must fail naming both.
        let dir = tmp_dir().join("empty");
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("DC_KEYS_FILE", dir.join("absent-keys"));
        std::env::set_var("DC_SETTINGS", dir.join("absent-settings.yaml"));
        let err = resolve().err().unwrap().to_string();
        assert!(err.contains("dc keys migrate"), "error should hint: {err}");
        std::env::remove_var("DC_KEYS_FILE");
        std::env::remove_var("DC_SETTINGS");
    }

    #[test]
    fn external_wins_over_legacy() {
        let dir = tmp_dir().join("ext-wins");
        std::fs::create_dir_all(&dir).unwrap();
        let kpath = dir.join("keys");
        let key = [42u8; 32];
        write_key_file(&kpath, &key).unwrap();
        // legacy settings with a DIFFERENT key
        std::fs::write(
            dir.join("settings.yaml"),
            "key: a2VwAAAAAAD/////////////////////////////\n",
        )
        .unwrap();
        std::env::set_var("DC_KEYS_FILE", &kpath);
        std::env::set_var("DC_SETTINGS", dir.join("settings.yaml"));
        match resolve().unwrap() {
            KeySource::External { path, key: k } => {
                assert_eq!(path, kpath);
                assert_eq!(k, key);
            }
            other => panic!("expected external, got {other:?}"),
        }
        std::env::remove_var("DC_KEYS_FILE");
        std::env::remove_var("DC_SETTINGS");
        std::fs::remove_file(&kpath).unwrap();
    }
}
