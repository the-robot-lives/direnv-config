//! Global `dc` settings — `~/.config/direnv-config/settings.yaml`.
//!
//! Holds the symmetric AEAD key used to encrypt secret values at rest and an
//! optional override for the audit-log path. This file is distinct from the
//! per-directory state store under `~/.local/state/direnv-config/`.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct SettingsFile {
    /// base64 (standard) encoded 32-byte XChaCha20-Poly1305 key.
    /// Legacy location — new deployments hold the key in the external key
    /// file (see [`crate::keys`]); `dc keys migrate` performs the move.
    pub key: Option<String>,
    /// Optional override for the audit log path.
    pub audit_log: Option<String>,
}

pub struct Settings {
    pub key: [u8; 32],
    pub audit_log: Option<PathBuf>,
}

/// Resolve the settings file path: `$DC_SETTINGS` → `$XDG_CONFIG_HOME/direnv-config/settings.yaml` → `~/.config/direnv-config/settings.yaml`.
// ⟦𓌂𓄬𓋿𓎱⟧ settings_path :: Resolve the settings file path: `$DC_SETTINGS` → `$XDG_CONFIG_HOME/direnv-config/settings.yaml` → `~
pub fn settings_path() -> PathBuf {
    if let Ok(p) = std::env::var("DC_SETTINGS") {
        return PathBuf::from(p);
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("direnv-config").join("settings.yaml");
    }
    dirs::home_dir()
        .map(|h| h.join(".config").join("direnv-config").join("settings.yaml"))
        .unwrap_or_else(|| PathBuf::from(".config/direnv-config/settings.yaml"))
}

/// Load and validate the settings file. The encryption key itself is resolved
/// via [`crate::keys::resolve`] (external key file → legacy `key:` here);
/// this file only contributes non-key settings such as `audit_log`.
// ⟦𓊿𓊵𓊇𓎸⟧ load :: Load and validate the settings file.
pub fn load() -> Result<Settings> {
    let path = settings_path();
    let parsed: SettingsFile = match std::fs::read_to_string(&path) {
        Ok(raw) => serde_yaml::from_str(&raw)
            .with_context(|| format!("parsing settings file {}", path.display()))?,
        // A settings file is optional once the key lives in the external key file.
        Err(_) if crate::keys::keys_path().exists() => SettingsFile { key: None, audit_log: None },
        Err(e) => {
            return Err(anyhow!(e)).with_context(|| {
                format!(
                    "reading settings file {} — create it with `key: <base64 32-byte key>` (e.g. `openssl rand -base64 32`)",
                    path.display()
                )
            })
        }
    };

    Ok(Settings {
        key: crate::keys::resolve()?.key().to_owned(),
        audit_log: parsed.audit_log.map(PathBuf::from),
    })
}

/// Convenience: load just the AEAD key (external key file wins over legacy `key:`).
// ⟦𓏚𓋮𓏋𓐆⟧ key :: Convenience: load just the AEAD key.
pub fn key() -> Result<[u8; 32]> {
    Ok(crate::keys::resolve()?.key().to_owned())
}
