//! `dc config set --plain` / `--numeric` / `--bool` — literal-mode writes:
//! `--plain` stores the value as a quoted literal (no 🔒 marker, no
//! encryption), `--numeric` as an unquoted YAML number and `--bool` as an
//! unquoted YAML boolean (both also unmarked); overwriting a previously
//! encrypted line drops the tier marker in every literal mode. Default
//! (no flag) still encrypts, and the literal flags conflict with
//! `--encrypted` (`--numeric`/`--bool` also conflict with each other).
//!
//! Everything runs against temp fixtures (own project dir, throwaway key
//! file, `XDG_STATE_HOME` sandbox) — never against a real store. Values are
//! fake. Literal values are shown unredacted in previews (the user chose an
//! exposed value), so literal-mode runs assert the value IS echoed on
//! stderr; the default encrypting mode still asserts it NEVER is.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const SECRET: &str = "example-plain-value-NOT-SECRET-42";
const ENC_SECRET: &str = "example-enc-value-NOT-SECRET-7";

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
    /// `store_yaml` seeds the store via `dc load` (so a store exists).
    fn new(tag: &str, store_yaml: &str) -> (Self, tempfile::TempDir) {
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
        std::fs::write(dir.join("team.yaml"), store_yaml).unwrap();
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

    fn read_envrc(&self) -> String {
        std::fs::read_to_string(self.dir.join(".envrc")).unwrap()
    }

    fn run(
        &self,
        tmp: &tempfile::TempDir,
        args: &[&str],
        stdin: Option<&str>,
    ) -> std::process::Output {
        let mut c = dc();
        c.current_dir(&self.dir)
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

    /// A real 🔒:v1 token for `value`, minted with the throwaway key via
    /// `dc encrypt --stdin` (the same crypto path `dc config set` uses).
    fn mint_token(&self, tmp: &tempfile::TempDir, value: &str) -> String {
        let out = self.run(tmp, &["encrypt", "--stdin"], Some(value));
        assert!(
            out.status.success(),
            "dc encrypt failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        // dc encrypt prints the bare token wrapped in display quotes.
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .trim_matches('"')
            .to_string()
    }
}

fn ok(out: &std::process::Output, what: &str) -> (String, String) {
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        out.status.success(),
        "dc {what} failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
    (stdout, stderr)
}

fn assert_no_echo(out: &std::process::Output, secret: &str) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stdout.contains(secret),
        "plaintext leaked on stdout:\n{stdout}"
    );
    assert!(
        !stderr.contains(secret),
        "plaintext leaked on stderr:\n{stderr}"
    );
}

fn heredoc(value_line: &str) -> String {
    format!("dc_yaml --no-bump app <<'YAML'\ndb:\n{value_line}\nYAML\n")
}

fn password_line(written: &str) -> String {
    written
        .lines()
        .find(|l| l.trim_start().starts_with("password:"))
        .unwrap_or_else(|| panic!("no password line in:\n{written}"))
        .to_string()
}

// ---- --plain: quoted literal, unmarked, shown unredacted in previews ----

#[test]
fn plain_stores_literal_without_marker_and_drops_existing_marker() {
    let (proj, tmp) = Proj::new("plain", "meta:\n  unused: 1\n");
    // Seed a genuinely encrypted-at-rest line (marker + token, throwaway key).
    let token = proj.mint_token(&tmp, ENC_SECRET);
    proj.envrc(&heredoc(&format!("  password: \"🔒 {token}\"")));

    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.password",
            "--value",
            SECRET,
            "--plain",
            "--yes",
        ],
        None,
    );
    ok(&out, "config set --plain");
    // Plain values are NOT redacted: the preview's new-value line shows the
    // literal that will land in the file.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&format!("password: \"{SECRET}\"")),
        "--plain preview must show the literal value:\n{stderr}"
    );

    // The line is the literal value: no 🔒 marker, no ❗ tier, old token gone.
    let written = proj.read_envrc();
    let line = password_line(&written);
    assert_eq!(line, "  password: \"".to_owned() + SECRET + "\"");
    assert!(
        !line.contains('🔒') && !line.contains('❗'),
        "marker dropped: {line}"
    );
    assert!(
        !written.contains(&token),
        "stale token still present:\n{written}"
    );

    // The refresh re-ingested the plain value — the store serves it verbatim.
    let get = proj.run(&tmp, &["get", "app", "db.password"], None);
    let (stdout, _) = ok(&get, "get after --plain set");
    assert!(
        stdout.contains(SECRET),
        "plain value must resolve from the refreshed store:\n{stdout}"
    );
}

#[test]
fn default_still_encrypts_and_token_round_trips() {
    let (proj, tmp) = Proj::new("enc", "meta:\n  unused: 1\n");
    proj.envrc(&heredoc("  password: plain-old"));

    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.password",
            "--value",
            SECRET,
            "--yes",
        ],
        None,
    );
    ok(&out, "config set (default)");
    // The encrypting preview still redacts: masked marker line, no plaintext…
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("**redacted**"),
        "encrypted preview must stay masked:\n{stderr}"
    );
    // …and the value never echoes anywhere.
    assert_no_echo(&out, SECRET);

    // Encrypted at rest: 🔒 marker + token, plaintext never written.
    let written = proj.read_envrc();
    let line = password_line(&written);
    assert!(
        line.starts_with("  password: \"🔒 🔒:v1:"),
        "default must encrypt: {line}"
    );
    assert!(
        !written.contains(SECRET),
        "plaintext leaked into file:\n{written}"
    );

    // Round-trip: the stored token decrypts back to the value with the key.
    let token = line
        .split("\"🔒 ")
        .nth(1)
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or_else(|| panic!("no token in line: {line}"));
    let dec = proj.run(&tmp, &["decrypt", token], None);
    let (stdout, _) = ok(&dec, "decrypt");
    assert_eq!(stdout.trim(), SECRET, "token must round-trip to the value");
}

#[test]
fn plain_and_encrypted_flags_conflict() {
    let (proj, tmp) = Proj::new("conflict", "meta:\n  unused: 1\n");
    proj.envrc(&heredoc("  password: plain-old"));
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.password",
            "--value",
            "v",
            "--plain",
            "--encrypted",
            "--yes",
        ],
        None,
    );
    assert!(
        !out.status.success(),
        "--plain + --encrypted must be rejected"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "expected a clap conflict error:\n{stderr}"
    );
    // Rejected before any write.
    assert!(
        proj.read_envrc().contains("plain-old"),
        "conflicted invocation must not touch the file"
    );
}

#[test]
fn plain_fresh_insert_has_no_marker_and_shows_value() {
    let (proj, tmp) = Proj::new("plain-insert", "meta:\n  unused: 1\n");
    proj.envrc(&heredoc("  host: localhost"));
    let out = proj.run(
        &tmp,
        &[
            "config", "set", "app", "db.user", "--value", SECRET, "--plain", "--yes",
        ],
        None,
    );
    ok(&out, "config set --plain (fresh insert)");
    // Insert preview shows the literal value.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&format!("user: \"{SECRET}\"")),
        "insert preview must show the literal:\n{stderr}"
    );
    let written = proj.read_envrc();
    assert!(
        written.contains(&format!("  user: \"{SECRET}\"")),
        "literal insert expected:\n{written}"
    );
    assert!(
        !written.contains('🔒'),
        "no marker anywhere in a --plain-only file:\n{written}"
    );
}

// ---- --numeric: unquoted YAML numbers ----

#[test]
fn numeric_writes_unquoted_int_and_float_and_shows_them() {
    let (proj, tmp) = Proj::new("numeric", "meta:\n  unused: 1\n");
    proj.envrc(&heredoc("  password: old"));

    for (input, want) in [("4", "  password: 4"), ("-3", "  password: -3"), ("2.5", "  password: 2.5"), ("1e3", "  password: 1e3")] {
        let out = proj.run(
            &tmp,
            &[
                "config",
                "set",
                "app",
                "db.password",
                // `--value -3` reads as a flag — the `=` form is required for
                // negative numbers (standard clap behaviour).
                &format!("--value={input}"),
                "--numeric",
                "--yes",
            ],
            None,
        );
        let (_, stderr) = ok(&out, "config set --numeric");
        assert!(
            stderr.contains(&format!("password: {}", input)),
            "numeric preview must show the bare number ({input}):\n{stderr}"
        );
        let written = proj.read_envrc();
        let line = password_line(&written);
        assert_eq!(line, want, "--numeric {input} must write an unquoted scalar");
        assert!(!line.contains('"'), "numeric scalar must be unquoted: {line}");
        assert!(!line.contains('🔒'), "numeric scalar carries no marker: {line}");
    }
}

#[test]
fn numeric_over_encrypted_line_drops_marker() {
    let (proj, tmp) = Proj::new("numeric-over-enc", "meta:\n  unused: 1\n");
    let token = proj.mint_token(&tmp, ENC_SECRET);
    proj.envrc(&heredoc(&format!("  password: \"🔒 {token}\"")));

    let out = proj.run(
        &tmp,
        &["config", "set", "app", "db.password", "--value", "7", "--numeric", "--yes"],
        None,
    );
    ok(&out, "config set --numeric over encrypted");
    let written = proj.read_envrc();
    let line = password_line(&written);
    assert_eq!(line, "  password: 7");
    assert!(!written.contains(&token), "stale token gone:\n{written}");
    assert!(!line.contains('🔒') && !line.contains('❗'), "marker dropped: {line}");
}

#[test]
fn numeric_rejects_non_numbers_with_no_write() {
    for bad in ["abc", "NaN", "", "inf", "4 2"] {
        let (proj, tmp) = Proj::new("numeric-bad", "meta:\n  unused: 1\n");
        proj.envrc(&heredoc("  password: keep-me"));
        let out = proj.run(
            &tmp,
            &["config", "set", "app", "db.password", "--value", bad, "--numeric", "--yes"],
            None,
        );
        assert!(
            !out.status.success(),
            "`--numeric {bad}` must be rejected:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("--numeric"),
            "rejection must name the flag:\n{stderr}"
        );
        assert!(
            proj.read_envrc().contains("keep-me"),
            "rejected value must not touch the file ({bad})"
        );
    }
}

#[test]
fn numeric_conflicts_with_encrypted() {
    let (proj, tmp) = Proj::new("numeric-conflict", "meta:\n  unused: 1\n");
    proj.envrc(&heredoc("  password: plain-old"));
    let out = proj.run(
        &tmp,
        &["config", "set", "app", "db.password", "--value", "4", "--numeric", "--encrypted", "--yes"],
        None,
    );
    assert!(!out.status.success(), "--numeric + --encrypted must be rejected");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("cannot be used with"),
        "expected a clap conflict error:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        proj.read_envrc().contains("plain-old"),
        "conflicted invocation must not touch the file"
    );
}

#[test]
fn numeric_round_trips_through_the_store_read() {
    let (proj, tmp) = Proj::new("numeric-roundtrip", "meta:\n  unused: 1\n");
    proj.envrc(&heredoc("  password: old"));
    let out = proj.run(
        &tmp,
        &["config", "set", "app", "db.password", "--value", "4", "--numeric", "--yes"],
        None,
    );
    ok(&out, "config set --numeric (round-trip)");
    let get = proj.run(&tmp, &["get", "app", "db.password"], None);
    let (stdout, _) = ok(&get, "get after --numeric set");
    assert!(
        stdout.trim().ends_with("4"),
        "numeric scalar must read back as the number:\n{stdout}"
    );
}

// ---- --bool: unquoted YAML booleans ----

#[test]
fn bool_writes_unquoted_normalized_true_false_and_shows_them() {
    let (proj, tmp) = Proj::new("bool", "meta:\n  unused: 1\n");
    proj.envrc(&heredoc("  password: old"));

    for (input, want) in [("TRUE", "  password: true"), ("false", "  password: false"), ("False", "  password: false")] {
        let out = proj.run(
            &tmp,
            &[
                "config",
                "set",
                "app",
                "db.password",
                "--value",
                input,
                "--bool",
                "--yes",
            ],
            None,
        );
        let (_, stderr) = ok(&out, "config set --bool");
        assert!(
            stderr.contains(&format!("password: {}", want.rsplit_once(' ').unwrap().1)),
            "bool preview must show the bare boolean ({input}):\n{stderr}"
        );
        let written = proj.read_envrc();
        let line = password_line(&written);
        assert_eq!(line, want, "--bool {input} must write a normalized unquoted scalar");
        assert!(!line.contains('"'), "bool scalar must be unquoted: {line}");
        assert!(!line.contains('🔒'), "bool scalar carries no marker: {line}");
    }
}

#[test]
fn bool_rejects_yes_and_other_spellings_with_no_write() {
    for bad in ["yes", "no", "1", "0", "on", "off", ""] {
        let (proj, tmp) = Proj::new("bool-bad", "meta:\n  unused: 1\n");
        proj.envrc(&heredoc("  password: keep-me"));
        let out = proj.run(
            &tmp,
            &["config", "set", "app", "db.password", "--value", bad, "--bool", "--yes"],
            None,
        );
        assert!(
            !out.status.success(),
            "`--bool {bad}` must be rejected:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("--bool"),
            "rejection must name the flag:\n{stderr}"
        );
        assert!(
            proj.read_envrc().contains("keep-me"),
            "rejected value must not touch the file ({bad})"
        );
    }
}

#[test]
fn bool_conflicts_with_numeric_and_encrypted() {
    let (proj, tmp) = Proj::new("bool-conflict", "meta:\n  unused: 1\n");
    proj.envrc(&heredoc("  password: plain-old"));

    for other in ["--numeric", "--encrypted"] {
        let out = proj.run(
            &tmp,
            &["config", "set", "app", "db.password", "--value", "true", "--bool", other, "--yes"],
            None,
        );
        assert!(
            !out.status.success(),
            "--bool + {other} must be rejected"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("cannot be used with"),
            "expected a clap conflict error:\n{stderr}"
        );
    }
    assert!(
        proj.read_envrc().contains("plain-old"),
        "conflicted invocations must not touch the file"
    );
}

#[test]
fn bool_over_encrypted_line_drops_marker_and_round_trips() {
    let (proj, tmp) = Proj::new("bool-over-enc", "meta:\n  unused: 1\n");
    let token = proj.mint_token(&tmp, ENC_SECRET);
    proj.envrc(&heredoc(&format!("  password: \"🔒 {token}\"")));

    let out = proj.run(
        &tmp,
        &["config", "set", "app", "db.password", "--value", "TRUE", "--bool", "--yes"],
        None,
    );
    ok(&out, "config set --bool over encrypted");
    let written = proj.read_envrc();
    let line = password_line(&written);
    assert_eq!(line, "  password: true", "marker dropped, normalized lowercase");
    assert!(!written.contains(&token), "stale token gone:\n{written}");

    let get = proj.run(&tmp, &["get", "app", "db.password"], None);
    let (stdout, _) = ok(&get, "get after --bool set");
    assert!(
        stdout.trim().ends_with("true"),
        "bool scalar must read back as the boolean:\n{stdout}"
    );
}
