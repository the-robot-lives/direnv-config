//! `dc get --keys` (`--redact`) — shape-only output: every leaf value is
//! replaced by a marker (🔒 encrypted / `<set>` non-empty plaintext /
//! `<empty>` empty/null) without ever printing a stored value or ciphertext
//! token and without ever touching the key. Every test runs in a throwaway
//! tempdir with HOME/XDG_* redirected inside it; fixture plaintexts and the
//! `🔒:v1:` ciphertext prefix are asserted absent from stdout+stderr.

use std::path::PathBuf;
use std::process::{Command, Stdio};

fn dc() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_dc"));
    c.env_remove("DC_ENV");
    c
}

/// Fresh project dir + throwaway key file + every state/config dir redirected
/// INSIDE a tempdir; every `dc` invocation runs with cwd = project dir.
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
        std::fs::write(
            tmp.path().join("keys"),
            "version: 1\nkey: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\n",
        )
        .unwrap();
        std::fs::write(dir.join("team.yaml"), "meta:\n  unused: 1\n").unwrap();
        let out = dc()
            .current_dir(&dir)
            .env("DC_KEYS_FILE", tmp.path().join("keys"))
            .env("XDG_STATE_HOME", tmp.path().join("state"))
            .args(["load", "team.yaml"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "dc load failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        (Proj { dir }, tmp)
    }

    fn envrc(&self, body: &str) {
        std::fs::write(self.dir.join(".envrc"), body).unwrap();
    }

    fn run(&self, tmp: &tempfile::TempDir, args: &[&str]) -> std::process::Output {
        dc().current_dir(&self.dir)
            .env("DC_KEYS_FILE", tmp.path().join("keys"))
            .env("XDG_STATE_HOME", tmp.path().join("state"))
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap()
    }

    /// Like `run`, but with NO key available anywhere: DC_KEYS_FILE removed,
    /// XDG_CONFIG_HOME/HOME pointed at empty dirs inside the tempdir.
    fn run_no_key(&self, tmp: &tempfile::TempDir, args: &[&str]) -> std::process::Output {
        let empty = tmp.path().join("empty-home");
        std::fs::create_dir_all(&empty).unwrap();
        dc().current_dir(&self.dir)
            .env_remove("DC_KEYS_FILE")
            .env("HOME", &empty)
            .env("XDG_CONFIG_HOME", tmp.path().join("empty-xdg"))
            .env("XDG_STATE_HOME", tmp.path().join("state"))
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap()
    }
}

/// Fixture plaintext values — must never appear in any `--keys` output.
const PLAINTEXTS: &[&str] = &[
    "sekrit-value-1",
    "plain-hello-marker",
    "alpha-tag",
    "beta-tag",
    "uno-value",
    "verum-value",
    "plainer-key",
];

/// Seed a `defaults` subject: a nested section holding one leaf that becomes
/// encrypted (`config set`, default encryption), plain strings, a number, a
/// bool, null, "", a list, and quoted keys under a second section.
fn seed_subject(proj: &Proj, tmp: &tempfile::TempDir) {
    proj.envrc(
        "dc_yaml --no-bump defaults <<'YAML'\ninference:\n  wafer:\n    apikey: placeholder\n    plain_note: plain-hello-marker\n    count: 4\n    enabled: true\n    nothing: ~\n    blank: \"\"\n  tags:\n    - alpha-tag\n    - beta-tag\nquoted:\n  \"01\": uno-value\n  \"true\": verum-value\n  plain: plainer-key\nYAML\n",
    );
    let out = proj.run(
        tmp,
        &[
            "config",
            "set",
            "defaults",
            "inference.wafer.apikey",
            "--value",
            "sekrit-value-1",
            "--yes",
        ],
    );
    assert!(
        out.status.success(),
        "config set failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn ok(out: &std::process::Output, what: &str) -> String {
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "dc {what} failed:\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    stdout
}

fn assert_no_leak(out: &std::process::Output, what: &str) {
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    for secret in PLAINTEXTS {
        assert!(
            !combined.contains(secret),
            "dc {what} leaked plaintext {secret:?}:\n{combined}"
        );
    }
    assert!(
        !combined.contains("🔒:v1:"),
        "dc {what} leaked a ciphertext token:\n{combined}"
    );
}

#[test]
fn keys_whole_subject_redacts_every_leaf() {
    let (proj, tmp) = Proj::new("kwhole");
    seed_subject(&proj, &tmp);
    let out = proj.run(&tmp, &["get", "defaults", "--keys"]);
    let stdout = ok(&out, "get --keys (whole subject)");
    assert_no_leak(&out, "get --keys (whole subject)");
    assert!(
        stdout.contains("apikey: 🔒"),
        "encrypted marker missing:\n{stdout}"
    );
    assert!(
        stdout.contains("plain_note: <set>"),
        "plaintext marker missing:\n{stdout}"
    );
    assert!(
        stdout.contains("count: <set>"),
        "number leaf marker missing:\n{stdout}"
    );
    assert!(
        stdout.contains("enabled: <set>"),
        "bool leaf marker missing:\n{stdout}"
    );
    assert!(
        stdout.contains("nothing: <empty>"),
        "null marker missing:\n{stdout}"
    );
    assert!(
        stdout.contains("blank: <empty>"),
        "empty-string marker missing:\n{stdout}"
    );
    assert!(
        stdout.contains("- <set>"),
        "list items not kept/marked:\n{stdout}"
    );
    // Key order preserved (seeded order: inference before quoted).
    let inf = stdout.find("inference:").unwrap();
    let quoted = stdout.find("quoted:").unwrap();
    assert!(inf < quoted, "mapping order not preserved:\n{stdout}");
}

#[test]
fn keys_section_scopes_to_the_subtree() {
    let (proj, tmp) = Proj::new("ksec");
    seed_subject(&proj, &tmp);
    let out = proj.run(&tmp, &["get", "defaults", "inference", "--keys"]);
    let stdout = ok(&out, "get section --keys");
    assert_no_leak(&out, "get section --keys");
    assert!(stdout.contains("wafer:"), "subtree root missing:\n{stdout}");
    assert!(
        stdout.contains("apikey: 🔒"),
        "encrypted marker missing:\n{stdout}"
    );
    assert!(
        !stdout.contains("quoted:"),
        "output escaped the section scope:\n{stdout}"
    );
}

#[test]
fn keys_list_items_marked_individually() {
    let (proj, tmp) = Proj::new("klist");
    seed_subject(&proj, &tmp);
    let stdout = ok(
        &proj.run(&tmp, &["get", "defaults", "inference", "--keys"]),
        "get section with list --keys",
    );
    assert!(
        stdout.matches("- <set>").count() == 2,
        "expected two marked list items:\n{stdout}"
    );
}

#[test]
fn keys_single_leaf_prints_path_and_marker() {
    let (proj, tmp) = Proj::new("kleaf");
    seed_subject(&proj, &tmp);
    let out = proj.run(
        &tmp,
        &["get", "defaults", "inference.wafer.apikey", "--keys"],
    );
    let stdout = ok(&out, "encrypted leaf --keys");
    assert_no_leak(&out, "encrypted leaf --keys");
    assert_eq!(
        stdout.trim_end(),
        "defaults.inference.wafer.apikey: 🔒",
        "leaf path/marker wrong:\n{stdout}"
    );

    let out = proj.run(&tmp, &["get", "defaults", "quoted.plain", "--keys"]);
    let stdout = ok(&out, "plain leaf --keys");
    assert_no_leak(&out, "plain leaf --keys");
    assert_eq!(
        stdout.trim_end(),
        "defaults.quoted.plain: <set>",
        "plain leaf marker wrong:\n{stdout}"
    );

    let out = proj.run(&tmp, &["get", "defaults", "missing.leaf", "--keys"]);
    assert!(!out.status.success(), "missing leaf must exit non-zero");
    assert_no_leak(&out, "missing leaf --keys");
}

#[test]
fn keys_preserves_quoted_keys() {
    let (proj, tmp) = Proj::new("kquote");
    seed_subject(&proj, &tmp);
    let out = proj.run(&tmp, &["get", "defaults", "quoted", "--keys"]);
    let stdout = ok(&out, "quoted keys --keys");
    assert_no_leak(&out, "quoted keys --keys");
    assert!(
        stdout.contains("'01': <set>") || stdout.contains("\"01\": <set>"),
        "quoted numeric key lost its quoting:\n{stdout}"
    );
    assert!(
        stdout.contains("'true': <set>") || stdout.contains("\"true\": <set>"),
        "quoted bool-like key lost its quoting:\n{stdout}"
    );
    assert!(
        stdout.contains("\nplain: <set>") && !stdout.contains("'plain'"),
        "plain key must stay unquoted:\n{stdout}"
    );
}

#[test]
fn keys_flat_lists_one_path_per_leaf() {
    let (proj, tmp) = Proj::new("kflat");
    seed_subject(&proj, &tmp);
    let out = proj.run(&tmp, &["get", "defaults", "--keys", "--flat"]);
    let stdout = ok(&out, "get --keys --flat");
    assert_no_leak(&out, "get --keys --flat");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 11, "one line per leaf expected:\n{stdout:?}");
    assert!(
        lines.contains(&"defaults.inference.wafer.apikey\t🔒"),
        "encrypted flat path missing:\n{stdout}"
    );
    assert!(
        lines.contains(&"defaults.inference.wafer.plain_note\t<set>"),
        "plaintext flat path missing:\n{stdout}"
    );
    assert!(
        lines.contains(&"defaults.inference.wafer.nothing\t<empty>"),
        "null flat path missing:\n{stdout}"
    );
    assert!(
        lines.contains(&"defaults.inference.tags[0]\t<set>")
            && lines.contains(&"defaults.inference.tags[1]\t<set>"),
        "list indices not rendered as [i]:\n{stdout}"
    );
    assert!(
        lines.contains(&"defaults.quoted.true\t<set>"),
        "quoted-key flat path missing:\n{stdout}"
    );

    // --flat on a single leaf: same one-liner as structured, TAB-separated.
    let out = proj.run(
        &tmp,
        &[
            "get",
            "defaults",
            "inference.wafer.apikey",
            "--keys",
            "--flat",
        ],
    );
    let stdout = ok(&out, "flat leaf --keys");
    assert_no_leak(&out, "flat leaf --keys");
    assert_eq!(
        stdout.trim_end(),
        "defaults.inference.wafer.apikey\t🔒",
        "flat leaf line wrong:\n{stdout}"
    );
}

#[test]
fn keys_rejects_reveal_raw_clippy_and_reveal_restricted() {
    let (proj, tmp) = Proj::new("kconflict");
    seed_subject(&proj, &tmp);
    for flag in ["--reveal", "--raw", "--clippy", "--reveal-restricted"] {
        let out = proj.run(&tmp, &["get", "defaults", "inference", "--keys", flag]);
        assert!(!out.status.success(), "--keys {flag} must be rejected");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("--keys") && stderr.contains(flag),
            "unclear conflict error for {flag}:\n{stderr}"
        );
        assert_no_leak(&out, &format!("--keys {flag}"));
    }
}

#[test]
fn keys_flat_requires_keys() {
    let (proj, tmp) = Proj::new("kflatreq");
    seed_subject(&proj, &tmp);
    let out = proj.run(&tmp, &["get", "defaults", "--flat"]);
    assert!(!out.status.success(), "bare --flat must be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--keys"),
        "error must name the missing --keys requirement:\n{stderr}"
    );
    assert_no_leak(&out, "bare --flat");
}

#[test]
fn keys_works_with_no_key_available() {
    let (proj, tmp) = Proj::new("knokey");
    seed_subject(&proj, &tmp);

    // Baseline: with no key anywhere, decryption is impossible — --reveal
    // must fail, proving this environment truly has no key.
    let out = proj.run_no_key(
        &tmp,
        &["get", "defaults", "inference.wafer.apikey", "--reveal"],
    );
    assert!(
        !out.status.success(),
        "--reveal succeeded without a key — the no-key environment is broken"
    );

    // --keys must not need the key at all: whole subject, section, leaf, flat.
    let out = proj.run_no_key(&tmp, &["get", "defaults", "--keys"]);
    let stdout = ok(&out, "no-key whole subject --keys");
    assert_no_leak(&out, "no-key whole subject --keys");
    assert!(stdout.contains("apikey: 🔒"), "marker missing:\n{stdout}");

    let out = proj.run_no_key(
        &tmp,
        &["get", "defaults", "inference.wafer.apikey", "--keys"],
    );
    let stdout = ok(&out, "no-key leaf --keys");
    assert_no_leak(&out, "no-key leaf --keys");
    assert_eq!(stdout.trim_end(), "defaults.inference.wafer.apikey: 🔒");

    let out = proj.run_no_key(&tmp, &["get", "defaults", "--keys", "--flat"]);
    let stdout = ok(&out, "no-key flat --keys");
    assert_no_leak(&out, "no-key flat --keys");
    assert!(
        stdout.contains("defaults.inference.wafer.apikey\t🔒"),
        "flat marker missing:\n{stdout}"
    );
}

#[test]
fn keys_output_leaks_no_values_anywhere() {
    let (proj, tmp) = Proj::new("kleak");
    seed_subject(&proj, &tmp);
    let invocations: &[&[&str]] = &[
        &["get", "defaults", "--keys"],
        &["get", "defaults", "--keys", "--flat"],
        &["get", "defaults", "inference", "--keys"],
        &["get", "defaults", "inference", "--keys", "--flat"],
        &["get", "defaults", "inference.wafer.apikey", "--keys"],
        &["get", "defaults", "quoted", "--keys"],
        &["get", "defaults", "quoted", "--keys", "--flat"],
    ];
    for args in invocations {
        let out = proj.run(&tmp, args);
        let what = args.join(" ");
        assert!(
            out.status.success(),
            "dc {what} failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_no_leak(&out, &what);
    }
}
