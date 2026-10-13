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

#[test]
fn seal_refresh_reflects_every_touched_file() {
    // W2 regression: `app` is split across two files (.envrc heredoc + the
    // registered external team.yaml). Sealing one key per file in one run must
    // refresh from BOTH files — a refresh fed by only one of them would leave
    // the other file's keys serving their pre-seal (stale) store values.
    let (proj, tmp) = Proj::new("refreshsplit");

    // pre-state sanity: the store-backed external key resolves to its
    // plaintext (heredoc keys only enter the store at refresh time)
    let out = proj.run(&tmp, &["get", "app", "db_password", "--reveal"], None);
    proj.assert_success(&out, "pre-state get");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "dbsec");

    // one run, one key per file
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--encrypt", "app.api_key", "app.db_password", "--yes"],
        None,
    );
    proj.assert_success(&out, "seal across both files");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("app.api_key"), "{stderr}");
    assert!(stderr.contains("app.db_password"), "{stderr}");

    // both files now hold tokens on disk
    let envrc = std::fs::read_to_string(proj.envrc()).unwrap();
    let team = std::fs::read_to_string(proj.dir.join("team.yaml")).unwrap();
    assert!(!envrc.contains("presecret"), "{envrc}");
    assert!(!team.contains("dbsec"), "{team}");

    // no staleness: the store serves the NEW state for BOTH files — the stale
    // plaintext values are gone from resolution, and --reveal still round-trips
    for (subject, path, want) in [("app", "api_key", "presecret"), ("app", "db_password", "dbsec")] {
        let out = proj.run(&tmp, &["get", subject, path], None);
        proj.assert_success(&out, path);
        let served = String::from_utf8_lossy(&out.stdout);
        assert!(
            !served.contains(want),
            "store served the stale pre-seal value for {subject}.{path}: {served}"
        );
        let out = proj.run(&tmp, &["get", subject, path, "--reveal"], None);
        proj.assert_success(&out, path);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            want,
            "{subject}.{path} must still decrypt after the multi-file seal"
        );
    }

    // sealing a key in only ONE file must not disturb the other file's keys
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--decrypt", "app.api_key", "--yes"],
        None,
    );
    proj.assert_success(&out, "single-file decrypt");
    let out = proj.run(&tmp, &["get", "app", "db_password", "--reveal"], None);
    proj.assert_success(&out, "other file untouched");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "dbsec");
}

#[test]
fn seal_conflicting_action_flags_die_at_the_parser() {
    // N1: clap-level mutual exclusion — contradictory combos never reach
    // cmd::seal's runtime check.
    let (proj, tmp) = Proj::new("parserx");
    for args in [
        vec!["seal", "--encrypt", "app.api_key", "--decrypt", "app.api_key"],
        vec!["seal", "--encrypt", "app.api_key", "--decrypt-all"],
        vec!["seal", "--encrypt-all-plain", "--decrypt-all"],
        vec!["seal", "--encrypt", "app.api_key", "--encrypt-all-plain"],
        vec!["seal", "--decrypt", "app.api_key", "--decrypt-all"],
    ] {
        let out = proj.run(&tmp, &args, None);
        assert!(!out.status.success(), "parser must reject {args:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("cannot be used with") || stderr.contains("conflict"),
            "expected a clap usage error for {args:?}:\n{stderr}"
        );
    }
}

// ---------------------------------------------------------------------------
// --match / --except selectors (leaf key name, case-insensitive wildcards)
// ---------------------------------------------------------------------------

fn vault_line(text: &str) -> String {
    text.lines()
        .find(|l| l.contains("vault"))
        .unwrap_or_else(|| panic!("no vault line in:\n{text}"))
        .to_string()
}

#[test]
fn seal_match_selects_only_matching_leaf_names_case_insensitively() {
    let (proj, tmp) = Proj::new("matchonly");
    let envrc_before = std::fs::read_to_string(proj.envrc()).unwrap();
    let out = proj.run(
        &tmp,
        &[
            "seal",
            "--headless",
            "--encrypt-all-plain",
            "--match",
            "*PASSWORD*",
            "--yes",
        ],
        None,
    );
    proj.assert_success(&out, "seal --match *PASSWORD* (uppercase pattern)");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("app.db_password"), "{stderr}");
    assert!(
        !stderr.contains("app.api_key") && !stderr.contains("app.host"),
        "match must exclude non-matching keys:\n{stderr}"
    );

    // only the external team.yaml key sealed
    let team = std::fs::read_to_string(proj.dir.join("team.yaml")).unwrap();
    assert!(!team.contains("dbsec"), "matched key must seal:\n{team}");
    // everything else byte-identical (plain base preserved, vault untouched)
    assert_eq!(
        std::fs::read_to_string(proj.envrc()).unwrap(),
        envrc_before,
        "unmatched .envrc keys must be untouched"
    );
    let out = proj.run(&tmp, &["get", "app", "db_password", "--reveal"], None);
    proj.assert_success(&out, "get --reveal after selective seal");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "dbsec");
}

#[test]
fn seal_except_excludes_matching_keys_from_the_base_set() {
    let (proj, tmp) = Proj::new("exceptonly");
    let out = proj.run(
        &tmp,
        &[
            "seal",
            "--headless",
            "--encrypt-all-plain",
            "--except",
            "*_KEY",
            "--except",
            "host",
            "--yes",
        ],
        None,
    );
    proj.assert_success(&out, "seal --except");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("app.db_password"), "{stderr}");
    assert!(
        !stderr.contains("app.api_key"),
        "--except *_KEY must drop api_key:\n{stderr}"
    );
    assert!(
        !stderr.contains("app.host"),
        "--except host must drop host:\n{stderr}"
    );

    let envrc = std::fs::read_to_string(proj.envrc()).unwrap();
    assert!(envrc.contains("presecret"), "excepted key stays plain:\n{envrc}");
    assert!(
        envrc.contains("host: localhost"),
        "excepted bare key stays plain:\n{envrc}"
    );
    let team = std::fs::read_to_string(proj.dir.join("team.yaml")).unwrap();
    assert!(!team.contains("dbsec"), "{team}");
}

#[test]
fn seal_except_wins_over_match_and_preserves_presealed_keys() {
    let (proj, tmp) = Proj::new("bothsel");
    let envrc_before = std::fs::read_to_string(proj.envrc()).unwrap();
    let out = proj.run(
        &tmp,
        &[
            "seal",
            "--headless",
            "--encrypt-all-plain",
            "--match",
            "*",
            "--except",
            "*key*",
            "--yes",
        ],
        None,
    );
    proj.assert_success(&out, "seal --match * --except *key*");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("app.api_key"),
        "--except must win over --match:\n{stderr}"
    );
    assert!(stderr.contains("app.host"), "{stderr}");
    assert!(stderr.contains("app.db_password"), "{stderr}");

    let envrc = std::fs::read_to_string(proj.envrc()).unwrap();
    assert!(
        envrc.contains("presecret"),
        "excepted api_key must stay plain:\n{envrc}"
    );
    assert!(
        envrc.contains("host: \"🔒 🔒:v1:t0:"),
        "matched host must seal:\n{envrc}"
    );
    // pre-encrypted keys are never touched by --encrypt, even with --match *
    assert_eq!(
        vault_line(&envrc),
        vault_line(&envrc_before),
        "sealed vault line byte-identical"
    );
}

#[test]
fn seal_zero_match_exits_non_zero_and_writes_nothing() {
    let (proj, tmp) = Proj::new("zeromatch");
    let envrc_before = std::fs::read(proj.envrc()).unwrap();
    let team_before = std::fs::read(proj.dir.join("team.yaml")).unwrap();
    let out = proj.run(
        &tmp,
        &[
            "seal",
            "--headless",
            "--encrypt-all-plain",
            "--match",
            "*nonexistent*",
            "--yes",
        ],
        None,
    );
    assert!(
        !out.status.success(),
        "zero-match selection must fail loudly (false-green guard):\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("0 keys selected"), "{stderr}");
    assert_eq!(std::fs::read(proj.envrc()).unwrap(), envrc_before);
    assert_eq!(std::fs::read(proj.dir.join("team.yaml")).unwrap(), team_before);
}

#[test]
fn seal_patterns_test_the_leaf_name_not_the_full_path() {
    let (proj, tmp) = Proj::new("leaflen");
    let out = proj.run(
        &tmp,
        &[
            "seal",
            "--headless",
            "--encrypt-all-plain",
            "--match",
            "app.*",
            "--yes",
        ],
        None,
    );
    assert!(
        !out.status.success(),
        "`app.*` must select nothing — patterns test the leaf name only"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("0 keys selected"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn seal_selectors_compose_with_explicit_encrypt_lists() {
    let (proj, tmp) = Proj::new("explicit");
    let out = proj.run(
        &tmp,
        &[
            "seal",
            "--headless",
            "--encrypt",
            "app.api_key",
            "app.db_password",
            "--match",
            "*password*",
            "--yes",
        ],
        None,
    );
    proj.assert_success(&out, "explicit list narrowed by --match");
    let envrc = std::fs::read_to_string(proj.envrc()).unwrap();
    assert!(envrc.contains("presecret"), "filtered-out explicit key stays plain");
    let team = std::fs::read_to_string(proj.dir.join("team.yaml")).unwrap();
    assert!(!team.contains("dbsec"), "{team}");
}

#[test]
fn seal_dry_run_previews_exactly_the_selector_applied_set() {
    let (proj, tmp) = Proj::new("seldry");
    let envrc_before = std::fs::read(proj.envrc()).unwrap();
    let team_before = std::fs::read(proj.dir.join("team.yaml")).unwrap();
    let out = proj.run(
        &tmp,
        &[
            "seal",
            "--headless",
            "--encrypt-all-plain",
            "--match",
            "*password*",
            "--dry-run",
        ],
        None,
    );
    proj.assert_success(&out, "selector dry-run");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let team_path = proj.dir.join("team.yaml").display().to_string();
    assert!(
        stderr.lines().any(|l| l
            == format!(
                "would seal app.db_password at {team_path}:2 (plain → 🔒 t1)"
            )),
        "stderr:\n{stderr}"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("dry-run: 1 change(s) — nothing written"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(std::fs::read(proj.envrc()).unwrap(), envrc_before);
    assert_eq!(std::fs::read(proj.dir.join("team.yaml")).unwrap(), team_before);
}

#[test]
fn seal_json_action_summary_carries_selected_names_and_count() {
    let (proj, tmp) = Proj::new("seljson");
    let out = proj.run(
        &tmp,
        &[
            "seal",
            "--headless",
            "--encrypt-all-plain",
            "--match",
            "*password*",
            "--dry-run",
            "--json",
        ],
        None,
    );
    proj.assert_success(&out, "selector --json dry-run");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["action"], "encrypt");
    assert_eq!(json["selected"], 1);
    assert_eq!(
        json["selected_keys"],
        serde_json::json!(["app.db_password"]),
        "{json}"
    );
    assert_eq!(json["applied"], 1);
    assert_eq!(json["dry_run"], true);
    assert_no_values(&serde_json::to_string(&json).unwrap());
    assert_no_values(&String::from_utf8_lossy(&out.stderr));
}

#[test]
fn seal_selectors_narrow_the_inventory_list() {
    let (proj, tmp) = Proj::new("sellist");
    let out = proj.run(
        &tmp,
        &["seal", "--list", "--match", "*password*", "--json"],
        None,
    );
    proj.assert_success(&out, "seal --list --match --json");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1, "{json}");
    assert_eq!(arr[0]["key"], "app.db_password");

    // except filters the inventory too
    let out = proj.run(
        &tmp,
        &["seal", "--list", "--except", "*", "--json"],
        None,
    );
    proj.assert_success(&out, "seal --list --except * --json");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 0, "{json}");
}

#[test]
fn seal_selectors_work_on_the_decrypt_direction() {
    let (proj, tmp) = Proj::new("seldecrypt");
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--encrypt", "app.api_key", "--yes"],
        None,
    );
    proj.assert_success(&out, "pre-seal api_key");
    let out = proj.run(
        &tmp,
        &[
            "seal",
            "--headless",
            "--decrypt-all",
            "--match",
            "*KEY*",
            "--yes",
        ],
        None,
    );
    proj.assert_success(&out, "decrypt-all narrowed by --match");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("app.api_key"), "{stderr}");
    assert!(
        !stderr.contains("app.vault"),
        "vault is sealed but must not match *KEY*:\n{stderr}"
    );
    let envrc = std::fs::read_to_string(proj.envrc()).unwrap();
    assert!(envrc.contains("presecret"), "api_key restored:\n{envrc}");
    assert!(
        envrc.contains("🔒:v1:t1:"),
        "vault must stay sealed:\n{envrc}"
    );
}

#[test]
fn seal_json_action_mode_prints_a_result_summary() {
    // N2: --json is honored in action mode — stdout is pure JSON, with the
    // human progress lines moved to stderr.
    let (proj, tmp) = Proj::new("jsonact");

    // dry-run + json: preview summary, nothing written
    let before = std::fs::read(proj.envrc()).unwrap();
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--encrypt", "app.api_key", "--dry-run", "--json"],
        None,
    );
    proj.assert_success(&out, "seal --json dry-run");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["action"], "encrypt");
    assert_eq!(json["applied"], 1);
    assert_eq!(json["dry_run"], true);
    assert_eq!(std::fs::read(proj.envrc()).unwrap(), before, "dry-run must not write");

    // real apply + json
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--encrypt", "app.api_key", "--yes", "--json"],
        None,
    );
    proj.assert_success(&out, "seal --json apply");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["applied"], 1);
    assert_eq!(json["dry_run"], false);
    // no plaintext may leak around the JSON summary (either stream)
    for stream in [
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    ] {
        assert!(!stream.contains("presecret"), "plaintext leaked: {stream}");
    }

    // decrypt direction reports its own action
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--decrypt", "app.api_key", "--yes", "--json"],
        None,
    );
    proj.assert_success(&out, "seal --json decrypt");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["action"], "decrypt");
    assert_eq!(json["applied"], 1);

    // nothing eligible: the "nothing to do" line moves to stderr so stdout
    // stays parseable (applied: 0)
    let out = proj.run(
        &tmp,
        &["seal", "--headless", "--encrypt", "app.vault", "--yes", "--json"],
        None,
    );
    proj.assert_success(&out, "seal --json nothing-to-do");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["applied"], 0);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("nothing to do"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
