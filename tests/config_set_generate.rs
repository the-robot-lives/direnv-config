//! `dc config set --generate <KIND>` — minted secrets: kind/length table,
//! `--if-missing` skip-vs-generate, mutual exclusion with the value sources,
//! `--plain` masking, `-p` deep inserts, and the no-echo guarantee (the
//! minted value never reaches stdout/stderr, in any mode).
//!
//! Everything runs against temp fixtures (own project dir, throwaway key
//! file, `XDG_STATE_HOME` sandbox) — never against a real store. Values are
//! machine-generated in the test; they are read back via `dc decrypt` and
//! held in memory only.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use base64::Engine;

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
        std::fs::write(
            tmp.path().join("keys"),
            "version: 1\nkey: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\n",
        )
        .unwrap();
        std::fs::write(dir.join("seed.yaml"), "meta:\n  unused: 1\n").unwrap();
        let out = dc()
            .current_dir(&dir)
            .env("DC_KEYS_FILE", tmp.path().join("keys"))
            .env("XDG_STATE_HOME", tmp.path().join("state"))
            .args(["load", "seed.yaml"])
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

    fn heredoc(&self, body: &str) {
        self.envrc(&format!("dc_yaml --no-bump app <<'YAML'\n{body}\nYAML\n"));
    }

    /// The `key: "🔒 <token>"` line for `leaf`, with the token split out.
    fn token_for(&self, leaf: &str) -> String {
        let written = self.read_envrc();
        let line = written
            .lines()
            .find(|l| l.trim_start().starts_with(&format!("{leaf}:")))
            .unwrap_or_else(|| panic!("no {leaf} line in:\n{written}"));
        line.split("\"🔒 ")
            .nth(1)
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or_else(|| panic!("no token in line: {line}"))
            .to_string()
    }

    /// Decrypt a stored token with the throwaway key (value stays in memory).
    fn decrypt(&self, tmp: &tempfile::TempDir, token: &str) -> String {
        let out = self.run(tmp, &["decrypt", token], None);
        assert!(
            out.status.success(),
            "dc decrypt failed:\n{}",
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
        "generated value leaked on stdout:\n{stdout}"
    );
    assert!(
        !stderr.contains(secret),
        "generated value leaked on stderr:\n{stderr}"
    );
}

// ---- the core flow: generated, encrypted at rest, never echoed ----

#[test]
fn generate_password_encrypts_round_trips_and_never_echoes() {
    let (proj, tmp) = Proj::new("gen-password");
    proj.heredoc("db:\n  password: placeholder");

    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.password",
            "--generate",
            "password",
            "--yes",
        ],
        None,
    );
    let (stdout, stderr) = ok(&out, "config set --generate password");
    // The prescribed output line: kind + length + destination, no value.
    assert!(
        stdout.contains("generated password (32 chars) → app db.password"),
        "missing prescribed output line:\n{stdout}"
    );
    assert!(
        stderr.contains("**generated**"),
        "generated preview must stay masked (never the value):\n{stderr}"
    );

    // Encrypted at rest; the minted value only exists in the file's token.
    let token = proj.token_for("password");
    let value = proj.decrypt(&tmp, &token);
    assert_eq!(value.len(), 32, "password default length is 32: {value}");
    assert!(
        value.chars().all(|c| c.is_ascii_alphanumeric()),
        "password charset must be [A-Za-z0-9]"
    );
    // The value never reached stdout/stderr of the set run…
    assert_no_echo(&out, &value);
    // …nor landed as plaintext in the file.
    assert!(
        !proj.read_envrc().contains(&value),
        "plaintext leaked into the source file"
    );
    // A second generate mints a different value.
    let out2 = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.password",
            "--generate",
            "password",
            "--yes",
        ],
        None,
    );
    ok(&out2, "second generate");
    let value2 = proj.decrypt(&tmp, &proj.token_for("password"));
    assert_ne!(value, value2, "each generate must mint a fresh value");
}

#[test]
fn generate_length_applies_per_kind() {
    let (proj, tmp) = Proj::new("gen-length");
    proj.heredoc("host: localhost");

    // hex counts BYTES: --length 8 → 16 hex chars.
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "a.hex",
            "--generate",
            "hex",
            "--length",
            "8",
            "-p",
            "--yes",
        ],
        None,
    );
    let (stdout, _) = ok(&out, "generate hex --length 8");
    assert!(stdout.contains("generated hex (16 chars)"), "{stdout}");
    let hex = proj.decrypt(&tmp, &proj.token_for("hex"));
    assert_eq!(hex.len(), 16);
    assert!(hex
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));

    // password counts CHARACTERS: --length 64 → 64 chars.
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "a.long",
            "--generate",
            "password",
            "--length",
            "64",
            "-p",
            "--yes",
        ],
        None,
    );
    ok(&out, "generate password --length 64");
    assert_eq!(proj.decrypt(&tmp, &proj.token_for("long")).len(), 64);

    // elixir counts characters with a floor of 32 (phx.gen.secret guard).
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "a.phx",
            "--generate",
            "secret-key-base",
            "--length",
            "32",
            "-p",
            "--yes",
        ],
        None,
    );
    ok(&out, "generate secret-key-base --length 32");
    let phx = proj.decrypt(&tmp, &proj.token_for("phx"));
    assert_eq!(phx.len(), 32, "elixir truncates to exactly --length chars");
    assert!(
        phx.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/'),
        "elixir alphabet is standard base64 (no padding, like phx.gen.secret): {phx}"
    );
}

#[test]
fn generate_uuid_base64_base64url_and_rsa_kinds() {
    let (proj, tmp) = Proj::new("gen-kinds");
    proj.heredoc("host: localhost");

    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "k.id",
            "--generate",
            "uuid",
            "-p",
            "--yes",
        ],
        None,
    );
    ok(&out, "generate uuid");
    assert!(String::from_utf8_lossy(&out.stdout).contains("generated uuid (36 chars)"));
    let uuid = proj.decrypt(&tmp, &proj.token_for("id"));
    let parts: Vec<&str> = uuid.split('-').collect();
    assert_eq!(
        parts.iter().map(|p| p.len()).collect::<Vec<_>>(),
        vec![8, 4, 4, 4, 12]
    );
    assert_eq!(parts[2].chars().next().unwrap(), '4', "v4 version nibble");
    assert!(
        matches!(parts[3].chars().next().unwrap(), '8' | '9' | 'a' | 'b'),
        "RFC 4122 variant nibble"
    );

    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "k.b64",
            "--generate",
            "base64",
            "-p",
            "--yes",
        ],
        None,
    );
    ok(&out, "generate base64");
    assert!(String::from_utf8_lossy(&out.stdout).contains("generated base64 (44 chars)"));
    let b64 = proj.decrypt(&tmp, &proj.token_for("b64"));
    assert_eq!(b64.len(), 44, "32 bytes → 44 padded chars");
    assert!(b64.ends_with('='));
    base64::engine::general_purpose::STANDARD
        .decode(&b64)
        .expect("stored base64 must decode");

    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "k.b64u",
            "--generate",
            "base64url",
            "-p",
            "--yes",
        ],
        None,
    );
    ok(&out, "generate base64url");
    assert!(String::from_utf8_lossy(&out.stdout).contains("generated base64url (43 chars)"));
    let b64u = proj.decrypt(&tmp, &proj.token_for("b64u"));
    assert_eq!(b64u.len(), 43, "32 bytes → 43 unpadded chars");
    assert!(!b64u.contains('='));
    assert!(!b64u.contains('+') && !b64u.contains('/'));
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&b64u)
        .expect("stored base64url must decode");

    // rsa: 2048-bit PKCS#1 PEM, base64-encoded onto one line.
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "k.key",
            "--generate",
            "rsa",
            "-p",
            "--yes",
        ],
        None,
    );
    ok(&out, "generate rsa");
    assert!(String::from_utf8_lossy(&out.stdout).contains("generated rsa"));
    let stored = proj.decrypt(&tmp, &proj.token_for("key"));
    let pem = base64::engine::general_purpose::STANDARD
        .decode(&stored)
        .expect("rsa value is base64 of the PEM");
    let pem = String::from_utf8(pem).expect("PEM is text");
    assert!(
        pem.starts_with("-----BEGIN RSA PRIVATE KEY-----\n"),
        "{pem}"
    );
    assert!(pem.ends_with("-----END RSA PRIVATE KEY-----\n"), "{pem}");
    assert!(!pem.contains('\r'), "LF line endings");
}

// ---- --if-missing: skip vs generate ----

#[test]
fn if_missing_skips_a_set_leaf_and_keeps_the_old_value() {
    let (proj, tmp) = Proj::new("if-missing-skip");
    proj.heredoc("host: localhost");

    // Seed a real value first (old flow).
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.password",
            "--value",
            "seed-value-NOT-SECRET",
            "-p",
            "--yes",
        ],
        None,
    );
    ok(&out, "seed value");
    let before = proj.read_envrc();

    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.password",
            "--generate",
            "uuid",
            "--if-missing",
            "--yes",
        ],
        None,
    );
    let (stdout, _) = ok(&out, "if-missing on set leaf");
    assert!(
        stdout.contains("skipped app db.password"),
        "output must say skipped:\n{stdout}"
    );
    assert!(
        !stdout.contains("generated"),
        "a skip must not say generated:\n{stdout}"
    );
    assert_eq!(
        proj.read_envrc(),
        before,
        "a skipped run must not touch the file"
    );
    // The old value survives.
    assert_eq!(
        proj.decrypt(&tmp, &proj.token_for("password")),
        "seed-value-NOT-SECRET"
    );
}

#[test]
fn if_missing_generates_for_empty_and_missing_leaves() {
    let (proj, tmp) = Proj::new("if-missing-gen");
    proj.heredoc("host: localhost\nblank: \"\"");

    // Empty value → generated (overwrite).
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "blank",
            "--generate",
            "hex",
            "--if-missing",
            "--yes",
        ],
        None,
    );
    let (stdout, _) = ok(&out, "if-missing on empty leaf");
    assert!(
        stdout.contains("generated hex (64 chars) → app blank"),
        "{stdout}"
    );

    // Missing leaf → generated + inserted.
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.token",
            "--generate",
            "uuid",
            "--if-missing",
            "-p",
            "--yes",
        ],
        None,
    );
    let (stdout, _) = ok(&out, "if-missing on missing leaf");
    assert!(
        stdout.contains("generated uuid (36 chars) → app db.token"),
        "{stdout}"
    );
    assert_eq!(proj.decrypt(&tmp, &proj.token_for("token")).len(), 36);
}

// ---- mutual exclusion / flag validation ----

#[test]
fn generate_conflicts_with_value_from_and_stdin() {
    for other in ["--value", "--from"] {
        let (proj, tmp) = Proj::new(&format!("conflict-{other}"));
        proj.heredoc("host: localhost");
        let out = proj.run(
            &tmp,
            &[
                "config",
                "set",
                "app",
                "host",
                "--generate",
                "uuid",
                other,
                "x",
                "--yes",
            ],
            None,
        );
        assert!(
            !out.status.success(),
            "--generate + {other} must be rejected"
        );
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("cannot be used with"),
            "expected a clap conflict error:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            proj.read_envrc().contains("host: localhost"),
            "conflicted invocation must not touch the file"
        );
    }

    // --stdin is a boolean — the conflict surfaces the same way.
    let (proj, tmp) = Proj::new("conflict-stdin");
    proj.heredoc("host: localhost");
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "host",
            "--generate",
            "uuid",
            "--stdin",
            "--yes",
        ],
        Some("x"),
    );
    assert!(
        !out.status.success(),
        "--generate + --stdin must be rejected"
    );
    assert!(
        proj.read_envrc().contains("host: localhost"),
        "conflicted invocation must not touch the file"
    );
}

#[test]
fn length_and_if_missing_require_generate() {
    let (proj, tmp) = Proj::new("requires-generate");
    proj.heredoc("host: localhost");
    let out = proj.run(
        &tmp,
        &[
            "config", "set", "app", "host", "--value", "x", "--length", "8", "--yes",
        ],
        None,
    );
    assert!(
        !out.status.success(),
        "--length without --generate must be rejected"
    );
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "host",
            "--value",
            "x",
            "--if-missing",
            "--yes",
        ],
        None,
    );
    assert!(
        !out.status.success(),
        "--if-missing without --generate must be rejected"
    );
    assert!(
        proj.read_envrc().contains("host: localhost"),
        "rejected invocations must not touch the file"
    );
}

#[test]
fn unknown_kind_is_rejected_with_the_kind_list_and_no_write() {
    let (proj, tmp) = Proj::new("unknown-kind");
    proj.heredoc("host: localhost");
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "host",
            "--generate",
            "passphrase",
            "--yes",
        ],
        None,
    );
    assert!(!out.status.success(), "unknown kind must be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    for kind in [
        "password",
        "hex",
        "base64",
        "base64url",
        "elixir",
        "uuid",
        "rsa",
    ] {
        assert!(stderr.contains(kind), "error must list {kind}:\n{stderr}");
    }
    assert!(
        proj.read_envrc().contains("host: localhost"),
        "rejected kind must not touch the file"
    );
}

#[test]
fn length_is_rejected_for_uuid_and_rsa() {
    let (proj, tmp) = Proj::new("length-invalid-kind");
    proj.heredoc("host: localhost");
    for kind in ["uuid", "rsa"] {
        let out = proj.run(
            &tmp,
            &[
                "config",
                "set",
                "app",
                "host",
                "--generate",
                kind,
                "--length",
                "8",
                "--yes",
            ],
            None,
        );
        assert!(!out.status.success(), "--length + {kind} must be rejected");
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("--length"),
            "rejection must name --length:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            proj.read_envrc().contains("host: localhost"),
            "rejected invocation must not touch the file ({kind})"
        );
    }
}

// ---- --plain with --generate: stored literal, preview still masked ----

#[test]
fn plain_generate_stores_literal_but_never_echoes_it() {
    let (proj, tmp) = Proj::new("gen-plain");
    proj.heredoc("host: localhost");

    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.token",
            "--generate",
            "uuid",
            "--plain",
            "-p",
            "--yes",
        ],
        None,
    );
    let (stdout, stderr) = ok(&out, "config set --generate uuid --plain");
    assert!(
        stderr.contains("**generated**"),
        "generated plain preview must be masked:\n{stderr}"
    );
    assert!(stdout.contains("generated uuid (36 chars)"), "{stdout}");

    // The file holds the literal (no marker), but the value never echoed.
    let written = proj.read_envrc();
    let line = written
        .lines()
        .find(|l| l.trim_start().starts_with("token:"))
        .unwrap_or_else(|| panic!("no token line:\n{written}"))
        .to_string();
    assert!(
        !line.contains('🔒'),
        "plain value carries no marker: {line}"
    );
    let value = line
        .split_once(": \"")
        .and_then(|(_, rest)| rest.strip_suffix('"'))
        .expect("quoted literal")
        .to_string();
    assert_eq!(value.len(), 36);
    assert_no_echo(&out, &value);
}

// ---- -p deep paths and the ⛔ shadow sentinel ----

#[test]
fn generate_through_deep_parents_chain_inserts_nested_sections() {
    let (proj, tmp) = Proj::new("gen-deep");
    proj.heredoc("host: localhost");

    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "a.b.c.secret",
            "--generate",
            "password",
            "--length",
            "16",
            "-p",
            "--yes",
        ],
        None,
    );
    let (stdout, stderr) = ok(&out, "generate through -p chain");
    assert!(
        stdout.contains("generated password (16 chars) → app a.b.c.secret"),
        "{stdout}"
    );
    assert_no_echo(&out, &proj.decrypt(&tmp, &proj.token_for("secret")));

    let written = proj.read_envrc();
    for want in ["a:", "b:", "c:"] {
        assert!(written.contains(want), "missing {want} section:\n{written}");
    }
    let _ = stderr;
}

#[test]
fn if_missing_never_clobbers_the_shadow_sentinel() {
    let (proj, tmp) = Proj::new("if-missing-shadow");
    proj.heredoc("host: localhost\nblocked: \"⛔\"");

    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "blocked",
            "--generate",
            "uuid",
            "--if-missing",
            "--yes",
        ],
        None,
    );
    let (stdout, _) = ok(&out, "if-missing over ⛔ sentinel");
    assert!(
        stdout.contains("skipped app blocked"),
        "⛔ counts as set — must skip:\n{stdout}"
    );
    assert!(
        proj.read_envrc().contains("\"⛔\""),
        "sentinel must survive:\n{}",
        proj.read_envrc()
    );
}

// ---- regression: the shared generator still serves `dc get --auto` ----

#[test]
fn get_auto_still_generates_with_the_shared_generator() {
    let (proj, tmp) = Proj::new("get-auto");
    proj.heredoc("host: localhost");

    // NOTE: `dc get --auto` prints the generated value by design (its
    // documented contract); here it is captured in memory, asserted, never
    // logged.
    let out = proj.run(
        &tmp,
        &["get", "app", "db.autosecret", "--auto", "password", "32"],
        None,
    );
    let (stdout, _) = ok(&out, "get --auto password");
    let v = stdout.trim().to_string();
    assert_eq!(v.len(), 32);
    assert!(v.chars().all(|c| c.is_ascii_alphanumeric()));

    let out = proj.run(
        &tmp,
        &["get", "app", "db.autosecret", "--auto", "hex", "8"],
        None,
    );
    let (stdout, _) = ok(&out, "get --auto hex");
    let v = stdout.trim().to_string();
    assert_eq!(v.len(), 16, "--auto hex counts bytes");
    assert!(v.chars().all(|c| c.is_ascii_hexdigit()));
}
