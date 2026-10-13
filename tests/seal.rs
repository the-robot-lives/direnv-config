//! `dc seal` end-to-end + the `dc secrets` in-scope extension, driven against
//! a tempdir fixture project (modeled on insert_destination_notice.rs).
//!
//! Pinned contracts:
//! - inventory lists all three states (🔒 t<N> / plain / ⛔ shadow) and never
//!   prints a value (no plaintext, no token);
//! - headless encrypt turns plain → `🔒:v1:` on disk and the project still
//!   resolves the secret (`dc get --reveal`);
//! - decrypt restores the original file bytes exactly;
//! - --dry-run writes nothing;
//! - ⛔ shadowed keys are refused, never modified;
//! - plain `dc seal` without a tty errors pointing at --headless;
//! - `dc secrets` merges .envrc* heredocs + registered externals + store
//!   layers, with `--sources` annotations, values never printed.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn dc() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_dc"));
    c.env_remove("DC_ENV");
    c
}

/// Fresh project dir + state dir + throwaway key file, all inside a tempdir;
/// every `dc` invocation runs with cwd = project dir.
struct Proj {
    dir: PathBuf,
}

impl Proj {
    fn new(tag: &str) -> (Self, tempfile::TempDir) {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = std::fs::canonicalize({
            std::fs::create_dir_all(tmp.path().join(tag)).unwrap();
            tmp.path().join(tag)
        })
        .unwrap();
        // canonicalize: the store hash must match what the binary computes
        // from the OS-resolved cwd (macOS $TMPDIR is a /var -> /private/var
        // symlink).
        std::fs::write(
            tmp.path().join("keys"),
            "version: 1\nkey: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\n",
        )
        .unwrap();

        // A pre-sealed (real) token for the vault key, so the inventory has a
        // genuine `🔒 t1` state without hand-crafting base64.
        let enc = Command::new(env!("CARGO_BIN_EXE_dc"))
            .current_dir(&dir)
            .env("DC_KEYS_FILE", tmp.path().join("keys"))
            .env("XDG_STATE_HOME", tmp.path().join("state"))
            .args(["encrypt", "--value", "vaulted", "--tier", "1"])
            .output()
            .unwrap();
        assert!(
            enc.status.success(),
            "dc encrypt failed: {}",
            String::from_utf8_lossy(&enc.stderr)
        );
        // dc encrypt prints the token shell-quoted ("🔒:v1:t1:…")
        let token = String::from_utf8_lossy(&enc.stdout)
            .trim()
            .trim_matches('"')
            .to_string();
        assert!(
            token.starts_with("🔒:v1:t1:"),
            "unexpected token shape: {token}"
        );

        std::fs::write(
            dir.join(".envrc"),
            format!(
                "dc_yaml --no-bump app <<'YAML'\nhost: localhost\napi_key: \"🔒 presecret\"\nvault: \"🔒❗❗ {token}\"\nlocked: \"⛔\"\nYAML\n"
            ),
        )
        .unwrap();
        std::fs::write(
            dir.join("team.yaml"),
            "app:\n  db_password: \"🔒❗ dbsec\"\n  locked: \"⛔\"\n",
        )
        .unwrap();

        let out = Self::run_in(&dir, &tmp, &["load", "team.yaml"], None);
        assert!(
            out.status.success(),
            "dc load failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        (Proj { dir }, tmp)
    }

    fn run_in(
        dir: &PathBuf,
        tmp: &tempfile::TempDir,
        args: &[&str],
        stdin: Option<&str>,
    ) -> std::process::Output {
        let mut c = dc();
        c.current_dir(dir)
            .env("DC_KEYS_FILE", tmp.path().join("keys"))
            .env("XDG_STATE_HOME", tmp.path().join("state"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = c.spawn().unwrap();
        if let Some(input) = stdin {
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
        }
        child.wait_with_output().unwrap()
    }

    fn run(
        &self,
        tmp: &tempfile::TempDir,
        args: &[&str],
        stdin: Option<&str>,
    ) -> std::process::Output {
        Self::run_in(&self.dir, tmp, args, stdin)
    }

    fn envrc(&self) -> PathBuf {
        self.dir.join(".envrc")
    }

    fn assert_success(&self, out: &std::process::Output, what: &str) {
        assert!(
            out.status.success(),
            "{what} failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }
}

/// No secret material may leak into seal/listing output.
fn assert_no_values(text: &str) {
    for leaked in ["presecret", "dbsec", "vaulted", "🔒:v1:", "dcenc:v1:"] {
        assert!(!text.contains(leaked), "output leaked `{leaked}`:\n{text}");
    }
}

fn envrc_path(proj: &Proj) -> String {
    proj.envrc().display().to_string()
}

#[test]
fn seal_list_shows_all_three_states_and_no_values() {
    let (proj, tmp) = Proj::new("list");
    let out = proj.run(&tmp, &["seal", "--list"], None);
    proj.assert_success(&out, "dc seal --list");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let envrc = envrc_path(&proj);

    let lines: Vec<&str> = stdout.lines().collect();
    let find = |key: &str| {
        lines
            .iter()
            .find(|l| l.starts_with(&format!("{key} ")))
            .unwrap_or_else(|| panic!("no `{key}` row in:\n{stdout}"))
    };

    // sealed: tier from the token
    let vault = find("app.vault");
    assert!(vault.contains("🔒 t1"), "vault row: {vault}");
    assert!(vault.contains(&format!("{envrc}:4")), "vault row: {vault}");
    // plain (marked plaintext)
    let api = find("app.api_key");
    assert!(api.contains(" plain"), "api_key row: {api}");
    // plain (bare)
    assert!(find("app.host").contains(" plain"));
    // shadow, from both the heredoc and the registered external
    let shadow_rows: Vec<&&str> = lines.iter().filter(|l| l.contains("⛔ shadow")).collect();
    assert_eq!(shadow_rows.len(), 2, "both ⛔ rows expected:\n{stdout}");

    assert_no_values(&stdout);
    assert_no_values(&String::from_utf8_lossy(&out.stderr));

    // --json variant: machine-readable, same facts
    let out = proj.run(&tmp, &["seal", "--list", "--json"], None);
    proj.assert_success(&out, "dc seal --list --json");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = json.as_array().unwrap();
    let state_of = |k: &str| {
        arr.iter()
            .find(|e| e["key"] == k)
            .unwrap_or_else(|| panic!("no {k} in json: {json}"))["state"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert_eq!(state_of("app.vault"), "sealed");
    assert_eq!(state_of("app.api_key"), "plain");
    assert_eq!(state_of("app.locked"), "shadow");
    let vault = arr.iter().find(|e| e["key"] == "app.vault").unwrap();
    assert_eq!(vault["tier"], 1);
    assert!(vault["file"].as_str().unwrap().ends_with(".envrc"));
    assert_eq!(vault["line"], 4);
    assert_no_values(&serde_json::to_string(&json).unwrap());
}

#[test]
fn seal_list_respects_file_filter() {
    let (proj, tmp) = Proj::new("filter");
    let team = proj.dir.join("team.yaml");
    let out = proj.run(
        &tmp,
        &["seal", "--list", "--file", team.to_str().unwrap()],
        None,
    );
    proj.assert_success(&out, "seal --list --file");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("app.db_password"), "{stdout}");
    assert!(stdout.contains("app.locked"), "{stdout}");
    assert!(
        !stdout.contains("app.host"),
        "--file filter leaked other files:\n{stdout}"
    );
    assert!(
        !stdout.contains("app.vault"),
        "--file filter leaked other files:\n{stdout}"
    );
}

#[test]
fn seal_dry_run_writes_nothing() {
    let (proj, tmp) = Proj::new("dryrun");
    let before = std::fs::read(proj.envrc()).unwrap();
    let out = proj.run(
        &tmp,
        &[
            "seal",
            "--headless",
            "--encrypt",
            "app.api_key",
            "--dry-run",
        ],
        None,
    );
    proj.assert_success(&out, "seal dry-run");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.lines().any(|l| l
            == format!(
                "would seal app.api_key at {}:3 (plain → 🔒 t0)",
                envrc_path(&proj)
            )),
        "stderr:\n{stderr}"
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("dry-run: 1 change(s) — nothing written"));
    assert_eq!(
        std::fs::read(proj.envrc()).unwrap(),
        before,
        "--dry-run must not write"
    );
}

#[test]
fn seal_encrypt_then_decrypt_round_trips_and_project_still_loads() {
    let (proj, tmp) = Proj::new("roundtrip");
    let original = std::fs::read_to_string(proj.envrc()).unwrap();

    // encrypt: plain (marked plaintext) → sealed token on disk
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--encrypt", "app.api_key", "--yes"],
        None,
    );
    proj.assert_success(&out, "seal encrypt");
    let sealed = std::fs::read_to_string(proj.envrc()).unwrap();
    let api_line = sealed.lines().find(|l| l.contains("api_key")).unwrap();
    assert!(api_line.contains("🔒:v1:t0:"), "sealed line: {api_line}");
    assert!(
        !sealed.contains("presecret"),
        "plaintext must be gone from disk:\n{sealed}"
    );
    // siblings + comments/structure intact
    assert!(sealed.contains("host: localhost"));
    assert!(sealed.contains("YAML"));

    // the project still loads: store refreshed, resolves + decrypts to plaintext
    let out = proj.run(&tmp, &["get", "app", "api_key", "--reveal"], None);
    proj.assert_success(&out, "get --reveal after seal");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "presecret");

    // decrypt: back to the original bytes
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--decrypt", "app.api_key", "--yes"],
        None,
    );
    proj.assert_success(&out, "seal decrypt");
    let restored = std::fs::read_to_string(proj.envrc()).unwrap();
    assert_eq!(
        restored, original,
        "decrypt must restore the original bytes"
    );
    let out = proj.run(&tmp, &["get", "app", "api_key", "--reveal"], None);
    proj.assert_success(&out, "get --reveal after unseal");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "presecret");
}

#[test]
fn seal_encrypt_all_plain_skips_sealed_and_shadow() {
    let (proj, tmp) = Proj::new("bulk");
    // bulk encrypt: api_key (marked plain) seals; host (bare plain) seals too;
    // vault (already sealed) + locked (⛔) are untouched.
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--encrypt-all-plain", "--yes"],
        None,
    );
    proj.assert_success(&out, "seal --encrypt-all-plain");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("app.api_key") && stderr.contains("app.host"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("app.locked"),
        "shadow must not be selected:\n{stderr}"
    );

    let sealed = std::fs::read_to_string(proj.envrc()).unwrap();
    assert!(!sealed.contains("presecret"));
    assert!(
        sealed.contains("host: \"🔒 🔒:v1:t0:"),
        "bare plain sealed too:\n{sealed}"
    );
    assert!(sealed.contains("⛔"), "shadow sentinel untouched");

    // store still resolves everything through the sealed sources
    for (subject, path, want) in [
        ("app", "api_key", "presecret"),
        ("app", "host", "localhost"),
        ("app", "vault", "vaulted"),
    ] {
        let out = proj.run(&tmp, &["get", subject, path, "--reveal"], None);
        proj.assert_success(&out, path);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            want,
            "{subject}.{path}"
        );
    }

    // decrypt-all reverses every sealed key (shadow stays). Marked originals
    // round-trip byte-exact; bare originals come back as `🔒 <value>` —
    // still a secret, plaintext-at-rest (bare tokens can't exist in source:
    // ingest requires the 🔒 marker to route into secrets.yaml).
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--decrypt-all", "--yes"],
        None,
    );
    proj.assert_success(&out, "seal --decrypt-all");
    let restored = std::fs::read_to_string(proj.envrc()).unwrap();
    assert!(restored.contains("presecret"));
    assert!(
        restored.contains("host: \"🔒 localhost\""),
        "bare original restores as marked plaintext-at-rest:\n{restored}"
    );
    assert!(restored.contains("⛔"));
}

#[test]
fn seal_refuses_shadow_keys_explicitly() {
    let (proj, tmp) = Proj::new("shadow");
    let before = std::fs::read(proj.envrc()).unwrap();
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--encrypt", "app.locked", "--yes"],
        None,
    );
    assert!(
        !out.status.success(),
        "encrypting a ⛔ shadow key must fail:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("⛔") || stderr.contains("shadow"),
        "stderr:\n{stderr}"
    );
    assert_eq!(
        std::fs::read(proj.envrc()).unwrap(),
        before,
        "shadow key must not be modified"
    );

    // unknown keys are a hard error too
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--encrypt", "nope.missing", "--yes"],
        None,
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("nope.missing"));
}

#[test]
fn seal_without_tty_points_at_headless() {
    let (proj, tmp) = Proj::new("nontty");
    let out = proj.run(&tmp, &["seal"], None);
    assert!(
        !out.status.success(),
        "bare `dc seal` without a tty must fail, not hang or no-op"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--headless"),
        "error must point at --headless:\n{stderr}"
    );
}

#[test]
fn seal_headless_requires_yes_to_write() {
    let (proj, tmp) = Proj::new("gate");
    let before = std::fs::read(proj.envrc()).unwrap();
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--encrypt", "app.api_key"],
        None,
    );
    assert!(
        !out.status.success(),
        "headless write without --yes must fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--yes"), "stderr:\n{stderr}");
    assert_eq!(std::fs::read(proj.envrc()).unwrap(), before);
}

#[test]
fn secrets_lists_merged_scope_with_sources_and_no_values() {
    let (proj, tmp) = Proj::new("scope");
    let out = proj.run(&tmp, &["secrets", "--sources"], None);
    proj.assert_success(&out, "dc secrets --sources");
    let stdout = String::from_utf8_lossy(&out.stdout);

    // merged total: heredoc (host, api_key, vault, locked) + external/store
    // (db_password, locked-dedup) = 5 unique keys, 1 config
    assert!(stdout.contains("5 key(s) across 1 config(s)"), "{stdout}");

    // heredoc source, no store layer yet (no refresh happened)
    let host = stdout
        .lines()
        .find(|l| l.trim_start().starts_with("host"))
        .unwrap();
    assert!(host.contains(&format!("{}:2", envrc_path(&proj))), "{host}");
    assert!(
        !host.contains("store:app/base"),
        "heredoc not yet ingested: {host}"
    );

    // external + store layer merge for the same key
    let dbp = stdout
        .lines()
        .find(|l| l.trim_start().starts_with("db_password"))
        .unwrap();
    assert!(dbp.contains("team.yaml:2"), "{dbp}");
    assert!(dbp.contains("store:app/secrets"), "{dbp}");

    // plain key landed in the store base layer at load time
    let locked = stdout
        .lines()
        .find(|l| l.trim_start().starts_with("locked"))
        .unwrap();
    assert!(locked.contains("store:app/base"), "{locked}");

    assert_no_values(&stdout);

    // legacy JSON shape still works: { subject: [keys] }
    let out = proj.run(&tmp, &["secrets", "--json"], None);
    proj.assert_success(&out, "dc secrets --json");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let keys = json["app"].as_array().unwrap();
    assert_eq!(keys.len(), 5, "{json}");

    // --sources --json: annotated entries
    let out = proj.run(&tmp, &["secrets", "--sources", "--json"], None);
    proj.assert_success(&out, "dc secrets --sources --json");
    let arr: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = arr.as_array().unwrap();
    assert_eq!(arr.len(), 5);
    let dbp = arr.iter().find(|e| e["key"] == "db_password").unwrap();
    let sources: Vec<&str> = dbp["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert!(
        sources.iter().any(|s| s.ends_with("team.yaml:2")),
        "{sources:?}"
    );
    assert!(sources.contains(&"store:app/secrets"), "{sources:?}");
    assert_no_values(&serde_json::to_string(&arr).unwrap());
}
