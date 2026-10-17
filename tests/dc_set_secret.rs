//! `dc set` secret handling (default) vs `--plain`, positional and `--stdin`.
//!
//! Default (no flag) and explicit `--secret` must encrypt the value into a
//! `🔒:v1` token at rest (round-tripping via `dc get --reveal`), `--plain`
//! must store the value untouched (scalar typing included), and `--stdin`
//! must work with both modes. A value that is already a `dc` token is stored
//! verbatim, and a secret re-write inherits the existing token's tier.

use std::io::Write;
use std::path::{Path, PathBuf};
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
        std::fs::write(
            tmp.path().join("keys"),
            "version: 1\nkey: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\n",
        )
        .unwrap();
        std::fs::write(dir.join("team.yaml"), "app:\n  name: noizu\n").unwrap();

        let mut c = dc();
        c.current_dir(&dir)
            .env("DC_KEYS_FILE", tmp.path().join("keys"))
            .env("XDG_STATE_HOME", tmp.path().join("state"))
            .args(["load", "team.yaml"]);
        let out = c.output().unwrap();
        assert!(
            out.status.success(),
            "dc load failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        (Proj { dir }, tmp)
    }

    fn run(&self, tmp: &tempfile::TempDir, args: &[&str], stdin: Option<&str>) -> std::process::Output {
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
            child.stdin.as_mut().unwrap().write_all(input.as_bytes()).unwrap();
        }
        child.wait_with_output().unwrap()
    }

    fn ok(&self, tmp: &tempfile::TempDir, args: &[&str], stdin: Option<&str>) -> String {
        let out = self.run(tmp, args, stdin);
        assert!(
            out.status.success(),
            "dc {:?} failed:\nstdout: {}\nstderr: {}",
            args,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        String::from_utf8_lossy(&out.stdout).to_string()
    }
}

/// The `team/local.yaml` layer file inside the store (state dir layout is
/// hashed, so locate it by walking).
fn local_layer(tmp: &tempfile::TempDir) -> PathBuf {
    fn walk(dir: &Path, depth: u8) -> Option<PathBuf> {
        if depth > 8 {
            return None;
        }
        for e in std::fs::read_dir(dir).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() {
                if let Some(f) = walk(&p, depth + 1) {
                    return Some(f);
                }
            } else if p.file_name().is_some_and(|n| n == "local.yaml")
                && p.parent().is_some_and(|d| d.file_name().is_some_and(|n| n == "team"))
            {
                return Some(p);
            }
        }
        None
    }
    walk(&tmp.path().join("state"), 0).expect("team/local.yaml not found under state dir")
}

fn layer_text(tmp: &tempfile::TempDir) -> String {
    std::fs::read_to_string(local_layer(tmp)).unwrap()
}

/// `dc encrypt --tier N <value>` prints a quoted token — unwrap it.
fn encrypt_token(proj: &Proj, tmp: &tempfile::TempDir, tier: u8, value: &str) -> String {
    let tier = tier.to_string();
    let out = proj.ok(tmp, &["encrypt", "--value", value, "--tier", &tier], None);
    out.trim().trim_matches('"').to_string()
}

#[test]
fn default_encrypts_positional_value() {
    let (proj, tmp) = Proj::new("set");
    proj.ok(&tmp, &["set", "team", "app.api_key", "hunter2"], None);
    assert!(
        layer_text(&tmp).contains("🔒:v1:t"),
        "layer file must hold a dc token at rest:\n{}",
        layer_text(&tmp)
    );
    let got = proj.ok(&tmp, &["get", "team", "app.api_key", "--reveal", "--raw"], None);
    assert_eq!(got, "hunter2");
}

#[test]
fn explicit_secret_flag_encrypts() {
    let (proj, tmp) = Proj::new("set");
    proj.ok(&tmp, &["set", "team", "app.api_key", "hunter2", "--secret"], None);
    assert!(layer_text(&tmp).contains("🔒:v1:t"));
    let got = proj.ok(&tmp, &["get", "team", "app.api_key", "--reveal", "--raw"], None);
    assert_eq!(got, "hunter2");
}

#[test]
fn plain_stores_value_untouched() {
    let (proj, tmp) = Proj::new("set");
    proj.ok(&tmp, &["set", "team", "app.status", "deploying", "--plain"], None);
    proj.ok(&tmp, &["set", "team", "app.port", "8080", "--plain"], None);
    let text = layer_text(&tmp);
    assert!(!text.contains("🔒"), "--plain must not encrypt:\n{text}");
    assert!(text.contains("8080"), "plain scalar typing must survive:\n{text}");
    let got = proj.ok(&tmp, &["get", "team", "app.status"], None);
    assert_eq!(got.trim(), "deploying");
    let got = proj.ok(&tmp, &["get", "team", "app.port"], None);
    assert_eq!(got.trim(), "8080");
}

#[test]
fn stdin_default_encrypts() {
    let (proj, tmp) = Proj::new("set");
    proj.ok(&tmp, &["set", "team", "app.api_key", "--stdin"], Some("hunter2\n"));
    assert!(layer_text(&tmp).contains("🔒:v1:t"));
    let got = proj.ok(&tmp, &["get", "team", "app.api_key", "--reveal", "--raw"], None);
    // The single trailing newline is consumed, as with `dc config set --stdin`.
    assert_eq!(got, "hunter2");
}

#[test]
fn stdin_plain_stores_untouched() {
    let (proj, tmp) = Proj::new("set");
    proj.ok(&tmp, &["set", "team", "app.port", "--stdin", "--plain"], Some("8080\n"));
    assert!(!layer_text(&tmp).contains("🔒"));
    let got = proj.ok(&tmp, &["get", "team", "app.port"], None);
    assert_eq!(got.trim(), "8080");
}

#[test]
fn token_input_stored_verbatim() {
    let (proj, tmp) = Proj::new("set");
    let token = encrypt_token(&proj, &tmp, 0, "outer");
    proj.ok(&tmp, &["set", "team", "app.api_key", "--stdin"], Some(&token));
    assert!(
        layer_text(&tmp).contains(&token),
        "an existing dc token must be stored verbatim, not re-encrypted:\n{}",
        layer_text(&tmp)
    );
    let got = proj.ok(&tmp, &["get", "team", "app.api_key", "--reveal", "--raw"], None);
    assert_eq!(got, "outer");
}

#[test]
fn secret_rewrite_inherits_existing_tier() {
    let (proj, tmp) = Proj::new("set");
    let token = encrypt_token(&proj, &tmp, 1, "v1");
    proj.ok(&tmp, &["set", "team", "app.api_key", &token], None);
    proj.ok(&tmp, &["set", "team", "app.api_key", "v2"], None);
    assert!(
        layer_text(&tmp).contains("🔒:v1:t1:"),
        "re-written secret must keep the existing tier-1 marker:\n{}",
        layer_text(&tmp)
    );
    let got = proj.ok(&tmp, &["get", "team", "app.api_key", "--reveal", "--raw"], None);
    assert_eq!(got, "v2");
}

#[test]
fn positional_value_and_stdin_conflict() {
    let (proj, tmp) = Proj::new("set");
    let out = proj.run(&tmp, &["set", "team", "app.k", "v", "--stdin"], Some("ignored"));
    assert!(!out.status.success(), "VALUE + --stdin must be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not both"), "stderr should name the conflict:\n{stderr}");
}

#[test]
fn missing_value_and_stdin_errors() {
    let (proj, tmp) = Proj::new("set");
    let out = proj.run(&tmp, &["set", "team", "app.k"], None);
    assert!(!out.status.success(), "no VALUE and no --stdin must error");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--stdin"), "error should point at --stdin:\n{stderr}");
}

#[test]
fn secret_and_plain_flags_conflict() {
    let (proj, tmp) = Proj::new("set");
    let out = proj.run(&tmp, &["set", "team", "app.k", "v", "--secret", "--plain"], None);
    assert!(!out.status.success(), "--secret + --plain must be rejected");
}
