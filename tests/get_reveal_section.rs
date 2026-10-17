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
        dc()
            .current_dir(&self.dir)
            .env("DC_KEYS_FILE", tmp.path().join("keys"))
            .env("XDG_STATE_HOME", tmp.path().join("state"))
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap()
    }
}

/// Seed a `defaults` subject with a mixed subtree: two encrypted leaves
/// (written via `dc config set`, default encryption), plain leaves, and a
/// list under the section.
fn seed_mixed_subject(proj: &Proj, tmp: &tempfile::TempDir) {
    proj.envrc(
        "dc_yaml --no-bump defaults <<'YAML'\ninference:\n  wafer:\n    apikey: placeholder\n    region_key: placeholder\n    plain_note: hello\n  tags:\n    - alpha\n    - beta\nYAML\n",
    );
    for (path, val) in [
        ("inference.wafer.apikey", "sekrit-value-1"),
        ("inference.wafer.region_key", "sekrit-value-2"),
    ] {
        let out = proj.run(
            tmp,
            &["config", "set", "defaults", path, "--value", val, "--yes"],
        );
        assert!(
            out.status.success(),
            "config set {path} failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
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

#[test]
fn reveal_decrypts_every_encrypted_leaf_in_a_section() {
    let (proj, tmp) = Proj::new("revsec");
    seed_mixed_subject(&proj, &tmp);
    let out = proj.run(&tmp, &["get", "defaults", "inference", "--reveal"]);
    let stdout = ok(&out, "get section --reveal");
    assert!(
        stdout.contains("sekrit-value-1") && stdout.contains("sekrit-value-2"),
        "decrypted leaves missing:\n{stdout}"
    );
    assert!(
        !stdout.contains("🔒:v1:"),
        "ciphertext still present after --reveal:\n{stdout}"
    );
    assert!(
        stdout.contains("plain_note: hello"),
        "plain leaves disturbed:\n{stdout}"
    );
    assert!(
        stdout.contains("apikey: sekrit-value-1"),
        "structure/keys not kept:\n{stdout}"
    );
}

#[test]
fn section_without_reveal_keeps_tokens_unchanged() {
    let (proj, tmp) = Proj::new("revbase");
    seed_mixed_subject(&proj, &tmp);
    let out = proj.run(&tmp, &["get", "defaults", "inference"]);
    let stdout = ok(&out, "get section (no flags)");
    assert!(
        stdout.contains("🔒:v1:"),
        "baseline changed — tokens no longer printed:\n{stdout}"
    );
    assert!(
        !stdout.contains("sekrit-value-1"),
        "plaintext leaked without --reveal:\n{stdout}"
    );
}

#[test]
fn reveal_walks_lists_under_a_section() {
    let (proj, tmp) = Proj::new("revlist");
    seed_mixed_subject(&proj, &tmp);
    let out = proj.run(&tmp, &["get", "defaults", "inference", "--reveal"]);
    let stdout = ok(&out, "get section with list --reveal");
    assert!(
        stdout.contains("- alpha") && stdout.contains("- beta"),
        "list items missing:\n{stdout}"
    );
}

#[test]
fn undecryptable_leaf_errors_per_leaf_without_printing_ciphertext() {
    let (proj, tmp) = Proj::new("revbad");
    proj.envrc(
        "dc_yaml --no-bump defaults <<'YAML'\ninference:\n  wafer:\n    apikey: placeholder\n    plain_note: hello\nYAML\n",
    );
    // A literal garbage token stored verbatim (--plain) — the store treats it
    // as an encrypted leaf, but no key can decode it.
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "defaults",
            "inference.wafer.apikey",
            "--plain",
            "--value",
            "🔒:v1:t0:garbage-not-a-token",
            "--yes",
        ],
    );
    ok(&out, "store garbage token");

    let out = proj.run(&tmp, &["get", "defaults", "inference", "--reveal"]);
    assert!(
        !out.status.success(),
        "undecryptable leaf must fail the reveal"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("failed to decrypt") && stderr.contains("inference.wafer.apikey"),
        "per-leaf error missing:\n{stderr}"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("garbage-not-a-token") && !stdout.contains("🔒:v1:"),
        "ciphertext leaked on stdout:\n{stdout}"
    );
}

#[test]
fn single_leaf_reveal_is_unchanged() {
    let (proj, tmp) = Proj::new("revleaf");
    seed_mixed_subject(&proj, &tmp);
    let out = proj.run(
        &tmp,
        &["get", "defaults", "inference.wafer.apikey", "--reveal"],
    );
    let stdout = ok(&out, "scalar --reveal");
    assert_eq!(stdout.trim_end(), "sekrit-value-1");
}

#[test]
fn raw_flag_on_a_section_is_a_yaml_noop() {
    let (proj, tmp) = Proj::new("revraw");
    seed_mixed_subject(&proj, &tmp);
    let plain = ok(
        &proj.run(&tmp, &["get", "defaults", "inference", "--reveal"]),
        "section reveal",
    );
    let raw = ok(
        &proj.run(&tmp, &["get", "defaults", "inference", "--reveal", "--raw"]),
        "section reveal --raw",
    );
    assert_eq!(plain, raw, "--raw must not change section output");
    assert!(raw.ends_with('\n'), "YAML dump must end with a newline");
}
