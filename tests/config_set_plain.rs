//! `dc config set --plain` — store the value as literal plaintext (no 🔒
//! marker, no encryption), including overwriting a previously encrypted line
//! (the tier marker is dropped). Default (no flag) still encrypts, and
//! `--plain` conflicts with `--encrypted`.
//!
//! Everything runs against temp fixtures (own project dir, throwaway key
//! file, `XDG_STATE_HOME` sandbox) — never against a real store. Values are
//! fake; the plaintext value is asserted to NEVER echo: absent from stdout
//! and stderr in every mode.

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
        String::from_utf8_lossy(&out.stdout).trim().to_string()
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
    assert_no_echo(&out, SECRET);

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
fn plain_fresh_insert_has_no_marker_and_never_echoes() {
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
    assert_no_echo(&out, SECRET);
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
