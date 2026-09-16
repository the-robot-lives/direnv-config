//! `dc keys` — manage the external, lockable encryption-key file.
//!
//! `status`  — which mode a store setup is in (external key file vs legacy settings.yaml)
//! `migrate` — move the legacy `key:` out of settings.yaml into the key file,
//!             verifying every encrypted token in the target store(s) decrypts
//!             with the new file BEFORE stripping the key from settings.yaml
//! `lock`    — print (and optionally run) the root-ownership hardening commands
//! `unlock`  — reverse the hardening

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

// ⟦𓆐𓎢𓇣𓊿⟧ run_status :: Report key-source mode for the active configuration.
pub fn run_status() -> Result<()> {
    let kpath = crate::keys::keys_path();
    let spath = crate::settings::settings_path();

    if kpath.exists() {
        println!("Mode:        external key file");
        println!("Key file:    {}", kpath.display());
        #[cfg(unix)]
        {
            let mode = crate::keys::mode_of(&kpath) & 0o777;
            println!("Perms:       {:04o}{}", mode, if crate::keys::weak_perms(&kpath) { "  (WARNING: group/other access — run `dc keys lock`)" } else { "" });
        }
        if crate::keys::legacy_key()?.is_some() {
            println!("Legacy:      `key:` still present in {} (unused; remove via `dc keys migrate --strip-only` or by hand)", spath.display());
        }
        println!("Lock state:  {}", lock_state(&kpath));
    } else {
        println!("Mode:        legacy (key embedded in settings.yaml)");
        println!("Settings:    {}", spath.display());
        if crate::keys::legacy_key()?.is_some() {
            #[cfg(unix)]
            {
                let mode = crate::keys::mode_of(&spath) & 0o777;
                if crate::keys::weak_perms(&spath) {
                    println!("Perms:       {:04o}  (WARNING: world-readable key material)", mode);
                }
            }
            println!("Next:        `dc keys migrate [<store.yaml> ...]`");
        } else {
            println!("Next:        no key found anywhere — generate one (`openssl rand -base64 32`) into {}", kpath.display());
        }
    }
    Ok(())
}

// ⟦𓍁𓄜𓋰𓎗⟧ lock_state :: Describe lock hardening state of the key file (macOS/Linux).
fn lock_state(path: &Path) -> String {
    #[cfg(unix)]
    {
        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return "unknown (unreadable)".into(),
        };
        use std::os::unix::fs::MetadataExt;
        let root_owned = meta.uid() == 0;
        let immutable = immutable_flag(path);
        match (root_owned, immutable) {
            (true, Some(true)) => "LOCKED (root-owned, immutable)".into(),
            (true, _) => "partially locked (root-owned; not immutable)".into(),
            (false, Some(true)) => "partially locked (immutable; not root-owned)".into(),
            (false, _) => "unlocked (user-owned, writable)".into(),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        "unknown (non-unix)".into()
    }
}

#[cfg(unix)]
fn immutable_flag(path: &Path) -> Option<bool> {
    if cfg!(target_os = "macos") {
        // `stat -f %Sf` prints ONLY the flags field (e.g. "uchg" or "0").
        let out = std::process::Command::new("stat").arg("-f").arg("%Sf").arg(path).output().ok()?;
        Some(stat_flags_have_uchg(&String::from_utf8_lossy(&out.stdout)))
    } else {
        // lsattr prints `<flags> <path>` — only the flags COLUMN may be searched,
        // else a path containing 'i' false-positives LOCKED.
        let out = std::process::Command::new("lsattr").arg(path).output().ok()?;
        Some(lsattr_flags_have_i(&String::from_utf8_lossy(&out.stdout)))
    }
}

/// macOS stat(1) %Sf output: whitespace-separated flag names (or `0`).
#[cfg(unix)]
fn stat_flags_have_uchg(out: &str) -> bool {
    out.split_whitespace()
        .flat_map(|f| f.split(','))
        .any(|f| f == "uchg")
}

/// Linux lsattr output: first whitespace field is the flags column.
#[cfg(unix)]
fn lsattr_flags_have_i(out: &str) -> bool {
    out.lines()
        .next()
        .and_then(|l| l.split_whitespace().next())
        .map(|flags| flags.contains('i'))
        .unwrap_or(false)
}

// ⟦𓋹𓆗𓂀𓊛⟧ run_migrate :: Move key from settings.yaml to the key file, verifying every token decrypts first.
pub fn run_migrate(stores: &[String], strip_only: bool) -> Result<()> {
    let kpath = crate::keys::keys_path();
    let spath = crate::settings::settings_path();

    if kpath.exists() && !strip_only {
        bail!("key file {} already exists — nothing to migrate (`dc keys status`)", kpath.display());
    }

    let key = match crate::keys::legacy_key()? {
        Some(k) => k,
        None if strip_only => bail!("no legacy `key:` in {} to strip", spath.display()),
        None => bail!("no legacy `key:` found in {} — nothing to migrate", spath.display()),
    };

    if strip_only {
        strip_legacy_key(&spath)?;
        println!("Stripped `key:` from {}", spath.display());
        return Ok(());
    }

    // 1. Write the key file (0600) — not yet authoritative until verification passes.
    crate::keys::write_key_file(&kpath, &key)?;
    println!("Wrote key file {} (0600)", kpath.display());

    // 2. Verify: every encrypted token in the target store(s) must decrypt
    //    via the NEW key file before we strip anything from settings.yaml.
    let store_paths: Vec<PathBuf> = if stores.is_empty() {
        vec![PathBuf::from(".envrc.dc")]
    } else {
        stores.iter().map(PathBuf::from).collect()
    };
    let mut total = 0usize;
    for sp in &store_paths {
        let n = verify_store(sp, &key)
            .map_err(|e| { let _ = std::fs::remove_file(&kpath); e })?;
        println!("Verified    {}: {} encrypted entries round-trip via key file", sp.display(), n);
        total += n;
    }
    if total == 0 {
        let _ = std::fs::remove_file(&kpath);
        bail!(
            "no encrypted tokens found in {} — refusing to migrate blindly; pass the store path(s) explicitly",
            store_paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
        );
    }

    // 3. Strip the legacy key, keeping a 0600 backup beside settings.yaml.
    strip_legacy_key(&spath)?;
    let backup = spath.with_extension("yaml.pre-keys");
    println!("Stripped `key:` from {} (backup: {})", spath.display(), backup.display());
    println!("Done — mode is now `external key file` (`dc keys status`); harden with `dc keys lock`.");
    Ok(())
}

/// Decode every `dc` token found in a store file with `key`. Returns the count.
// ⟦𓄞𓍗𓊚𓇬⟧ verify_store :: Decode every `dc` token found in a store file with `key`.
fn verify_store(path: &Path, key: &[u8; 32]) -> Result<usize> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading store {}", path.display()))?;
    let mut count = 0usize;
    for line in raw.lines() {
        for candidate in extract_tokens(line) {
            crate::crypto::decode_token(&candidate, key).with_context(|| {
                format!(
                    "verify failed in {} — ABORTING, legacy settings.yaml left intact",
                    path.display()
                )
            })?;
            count += 1;
        }
    }
    Ok(count)
}

/// Pull complete `🔒:v1:` / `dcenc:v1:` tokens (single-line scalars) out of a line.
// ⟦𓇮𓎼𓋞𓁜⟧ extract_tokens :: Pull complete dc tokens out of a line.
fn extract_tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for prefix in ["🔒:v1:", "dcenc:v1:"] {
        let mut rest = line;
        while let Some(pos) = rest.find(prefix) {
            let tail = &rest[pos..];
            let end = tail
                .char_indices()
                .find(|(i, c)| *i >= prefix.len() && !is_token_char(*c))
                .map(|(i, _)| i)
                .unwrap_or(tail.len());
            out.push(tail[..end].to_string());
            rest = &tail[end..];
        }
    }
    out
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':'
}

/// Remove `key:` from settings.yaml, keeping a 0600 `.pre-keys` backup.
/// The backup is written via temp+chmod-0600+rename so key material never
/// rests on disk at umask perms.
// ⟦𓊓𓆏𓍾𓄂⟧ strip_legacy_key :: Remove `key:` from settings.yaml, keeping a backup.
fn strip_legacy_key(spath: &Path) -> Result<()> {
    let raw = std::fs::read_to_string(spath)
        .with_context(|| format!("reading {}", spath.display()))?;
    let kept: Vec<&str> = raw
        .lines()
        .filter(|l| {
            let t = l.trim();
            !(t.starts_with("key:") || t.starts_with('#') && t.contains("base64 32-byte"))
        })
        .collect();
    let backup = spath.with_extension("yaml.pre-keys");
    let tmp = spath.with_extension("yaml.pre-keys.tmp");
    std::fs::write(&tmp, &raw)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, &backup)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(spath, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::write(spath, kept.join("\n") + "\n")?;
    Ok(())
}

// ⟦𓎺𓊠𓍛𓃠⟧ run_lock :: Print (and optionally run) root-ownership hardening for the key file.
pub fn run_lock(yes: bool) -> Result<()> {
    let kpath = crate::keys::keys_path();
    if !kpath.exists() {
        bail!("no key file at {} — run `dc keys migrate` first", kpath.display());
    }
    let mut cmds = harden_cmds(&kpath);
    // No world-readable key artifact may survive anywhere: while a legacy
    // `key:` still lives in settings.yaml, tighten that file too.
    let spath = crate::settings::settings_path();
    if crate::keys::legacy_key()?.is_some() {
        println!("note: legacy `key:` still present in {} — included in hardening", spath.display());
        cmds.push(format!("sudo chmod 0600 {}", spath.display()));
    }
    println!("dc needs only READ access to the key file for both encrypt and decrypt.");
    println!("Hardening (run as a user who can sudo; you keep read access):\n");
    for c in &cmds {
        println!("  {}", c);
    }
    if yes {
        for c in &cmds {
            let status = std::process::Command::new("sudo").args(c.split_whitespace().skip(1)).status()?;
            if !status.success() {
                bail!("hardening command failed: {}", c);
            }
        }
        println!("\nLocked. `dc keys status` to confirm.");
    } else {
        println!("\nRe-run with --yes to execute these now.");
    }
    Ok(())
}

// ⟦𓍞𓇤𓆡𓎸⟧ run_unlock :: Reverse key-file hardening.
pub fn run_unlock(yes: bool) -> Result<()> {
    let kpath = crate::keys::keys_path();
    if !kpath.exists() {
        bail!("no key file at {}", kpath.display());
    }
    let platform_flag = if cfg!(target_os = "macos") { "sudo chflags nouchg" } else { "sudo chattr -i" };
    let cmds = vec![
        format!("{} {}", platform_flag, kpath.display()),
        format!("sudo chown {} {}", whoami(), kpath.display()),
        format!("sudo chmod 0600 {}", kpath.display()),
    ];
    for c in &cmds {
        println!("  {}", c);
    }
    if yes {
        for c in &cmds {
            let status = std::process::Command::new("sudo").args(c.split_whitespace().skip(1)).status()?;
            if !status.success() {
                bail!("unlock command failed: {}", c);
            }
        }
        println!("\nUnlocked (0600, user-owned).");
    } else {
        println!("\nRe-run with --yes to execute these now.");
    }
    Ok(())
}

fn harden_cmds(kpath: &Path) -> Vec<String> {
    let group = current_group();
    let mut cmds = vec![
        format!("sudo chown root:{} {}", group, kpath.display()),
        format!("sudo chmod 0440 {}", kpath.display()),
    ];
    if cfg!(target_os = "macos") {
        cmds.push(format!("sudo chflags uchg {}", kpath.display()));
    } else {
        cmds.push(format!("sudo chattr +i {}", kpath.display()));
    }
    cmds
}

/// Group name that keeps the invoking user read access after root-ownership.
fn current_group() -> String {
    std::process::Command::new("id")
        .arg("-gn")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "staff".into())
}

fn whoami() -> String {
    std::env::var("USER").unwrap_or_else(|_| "root".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_tokens_from_line() {
        let line = "  foo: \"🔒:v1:t1:AbC-dEf_123\" # comment";
        let toks = extract_tokens(line);
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0], "🔒:v1:t1:AbC-dEf_123");
    }

    #[test]
    fn extracts_legacy_tokens() {
        let toks = extract_tokens("k: dcenc:v1:t0:xyz");
        assert_eq!(toks, vec!["dcenc:v1:t0:xyz"]);
    }

    #[test]
    fn no_tokens_no_crash() {
        assert!(extract_tokens("plain: value").is_empty());
    }

    #[test]
    fn verify_store_round_trip_and_fail() {
        let dir = std::env::temp_dir().join(format!("dc-verify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("store.envrc.dc");
        let key = [3u8; 32];
        let tok = crate::crypto::encode_token("shh", 1, &key).unwrap();
        std::fs::write(&path, format!("sec: \"{}\"\n", tok)).unwrap();
        assert_eq!(verify_store(&path, &key).unwrap(), 1);
        let mut bad = key;
        bad[0] ^= 0xff;
        assert!(verify_store(&path, &bad).is_err(), "wrong key must abort");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn lsattr_flags_column_only() {
        // flags column has NO 'i'; the path contains 'i' — must not false-positive.
        assert!(!lsattr_flags_have_i(
            "--------------e--- /Users/keith/.immutable.config/dc/keys\n"
        ));
        assert!(lsattr_flags_have_i(
            "-------i------e--- /Users/keith/.config/direnv-config/keys\n"
        ));
        assert!(!lsattr_flags_have_i(""));
    }

    #[test]
    fn stat_flags_field_only() {
        assert!(stat_flags_have_uchg("uchg"));
        assert!(!stat_flags_have_uchg("0"));
        assert!(!stat_flags_have_uchg(""));
        assert!(stat_flags_have_uchg("uchg,sappnd"));
    }

    #[cfg(unix)]
    #[test]
    fn strip_legacy_key_backup_is_0600() {
        let dir = std::env::temp_dir().join(format!("dc-strip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.yaml");
        std::fs::write(&path, "# comment\neye: 1\nkey: abc\nother: 2\n").unwrap();
        strip_legacy_key(&path).unwrap();
        let backup = path.with_extension("yaml.pre-keys");
        assert_eq!(crate::keys::mode_of(&backup) & 0o777, 0o600);
        let kept = std::fs::read_to_string(&path).unwrap();
        assert!(!kept.contains("key: abc"));
        assert!(kept.contains("eye: 1") && kept.contains("other: 2"));
    }
}
