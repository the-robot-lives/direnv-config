//! `dc config set -p/--parents` — missing intermediate sections are created
//! as nested YAML mappings, with `--below/--above/--auto` placing the new
//! top-most missing section among its siblings.
//!
//! Everything runs against temp fixtures (own project dir, throwaway key
//! file, `XDG_STATE_HOME` sandbox) — never against a real store. The
//! plaintext value is asserted to NEVER echo: absent from stdout, stderr,
//! and every written file.

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
    /// `store_yaml` seeds the store via `dc load` (so a store exists); it
    /// carries an unrelated subject unless a test says otherwise.
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
        if !store_yaml.is_empty() {
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
        }
        (Proj { dir }, tmp)
    }

    fn envrc(&self, body: &str) {
        std::fs::write(self.dir.join(".envrc"), body).unwrap();
    }

    fn read_envrc(&self) -> String {
        std::fs::read_to_string(self.dir.join(".envrc")).unwrap()
    }

    fn envrc_path(&self) -> String {
        self.dir.join(".envrc").display().to_string()
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
}

/// `dc_yaml --no-bump cf` heredoc with sendgrid + stripe top-level sections.
const TWO_SIBLINGS: &str =
    "dc_yaml --no-bump cf <<'YAML'\nsendgrid:\n  api_key: \"plain s\"\nstripe:\n  key: k\nYAML\n";

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

#[test]
fn parents_below_creates_nested_chain_in_envrc() {
    let (proj, tmp) = Proj::new("pbelow", "meta:\n  unused: 1\n");
    proj.envrc(TWO_SIBLINGS);
    const SECRET: &str = "S3cr3t-Value-42";
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "cf",
            "inference.jev.api_key",
            "--value",
            SECRET,
            "--yes",
            "-p",
            "--below",
            "sendgrid",
        ],
        None,
    );
    let (stdout, stderr) = ok(&out, "config set -p --below");
    assert_no_echo(&out, SECRET);

    // The whole anchor block is skipped: chain lands between the sections.
    let written = proj.read_envrc();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines[3], "inference:");
    assert_eq!(lines[4], "  jev:");
    assert!(
        lines[5].starts_with("    api_key: \"🔒 🔒:v1:"),
        "{}",
        lines[5]
    );
    assert_eq!(lines[6], "stripe:");
    assert!(
        lines.iter().any(|l| *l == "  api_key: \"plain s\""),
        "anchor content clobbered:\n{written}"
    );

    // Insert notices name the destination file:line on stderr only.
    let envrc = proj.envrc_path();
    assert!(
        stderr.contains(&format!("inserted cf inference.jev.api_key at {envrc}:4")),
        "missing insert notice:\n{stderr}"
    );
    assert!(
        !stdout.contains(&envrc),
        "destination leaked on stdout:\n{stdout}"
    );

    // The refreshed store resolves the new key.
    let get = proj.run(
        &tmp,
        &["config", "get", "cf", "inference.jev.api_key"],
        None,
    );
    ok(&get, "config get after -p insert");
    assert_no_echo(&get, SECRET);
}

#[test]
fn parents_auto_positions_first_middle_and_last() {
    // first: aaa sorts before sendgrid
    let (proj, tmp) = Proj::new("pauto1", "meta:\n  unused: 1\n");
    proj.envrc(TWO_SIBLINGS);
    let out = proj.run(
        &tmp,
        &[
            "config", "set", "cf", "aaa.key", "--value", "v", "--yes", "-p",
        ],
        None,
    );
    ok(&out, "auto first");
    let written = proj.read_envrc();
    let aaa = written.find("aaa:").unwrap();
    let sendgrid = written.find("sendgrid:").unwrap();
    let stripe = written.find("stripe:").unwrap();
    assert!(
        aaa < sendgrid && sendgrid < stripe,
        "auto-first order wrong:\n{written}"
    );

    // middle: smoke sorts between sendgrid and stripe
    let (proj, tmp) = Proj::new("pauto2", "meta:\n  unused: 1\n");
    proj.envrc(TWO_SIBLINGS);
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "cf",
            "smoke.key",
            "--value",
            "v",
            "--yes",
            "-p",
        ],
        None,
    );
    ok(&out, "auto middle");
    let written = proj.read_envrc();
    let sendgrid = written.find("sendgrid:").unwrap();
    let smoke = written.find("smoke:").unwrap();
    let stripe = written.find("stripe:").unwrap();
    assert!(
        sendgrid < smoke && smoke < stripe,
        "auto-middle order wrong:\n{written}"
    );

    // last: zzz sorts after everything, still inside the heredoc
    let (proj, tmp) = Proj::new("pauto3", "meta:\n  unused: 1\n");
    proj.envrc(TWO_SIBLINGS);
    let out = proj.run(
        &tmp,
        &[
            "config", "set", "cf", "zzz.key", "--value", "v", "--yes", "-p",
        ],
        None,
    );
    ok(&out, "auto last");
    let written = proj.read_envrc();
    let stripe = written.find("stripe:").unwrap();
    let zzz = written.find("zzz:").unwrap();
    let terminator = written.rfind("YAML").unwrap();
    assert!(
        stripe < zzz && zzz < terminator,
        "auto-last order wrong:\n{written}"
    );
}

#[test]
fn parents_stdin_value_with_placement() {
    let (proj, tmp) = Proj::new("pstdin", "meta:\n  unused: 1\n");
    proj.envrc(TWO_SIBLINGS);
    const SECRET: &str = "piped-secret-99";
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "cf",
            "inference.jev.api_key",
            "--stdin",
            "--yes",
            "-p",
            "--below",
            "sendgrid",
        ],
        Some(&format!("{SECRET}\n")),
    );
    ok(&out, "config set -p --stdin");
    assert_no_echo(&out, SECRET);
    let written = proj.read_envrc();
    assert!(
        written.contains("inference:\n  jev:"),
        "chain missing:\n{written}"
    );
    assert!(
        !written.contains(SECRET),
        "plaintext written to file:\n{written}"
    );
}

#[test]
fn missing_parent_without_parents_hints_at_dash_p() {
    let (proj, tmp) = Proj::new("pnop", "meta:\n  unused: 1\n");
    proj.envrc(TWO_SIBLINGS);
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "cf",
            "inference.jev.api_key",
            "--value",
            "v",
            "--yes",
        ],
        None,
    );
    assert!(
        !out.status.success(),
        "set without -p must fail on a missing parent:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("pass -p/--parents"),
        "hint missing:\n{stderr}"
    );
    assert!(
        stderr.contains("dc config setall"),
        "setall hint missing:\n{stderr}"
    );
}

#[test]
fn placement_flag_rules() {
    let (proj, tmp) = Proj::new("pflag", "meta:\n  unused: 1\n");
    proj.envrc(TWO_SIBLINGS);

    // Conflicting anchors (--below + --above).
    let out = proj.run(
        &tmp,
        &[
            "config", "set", "cf", "x.y.z", "--value", "v", "--yes", "-p", "--below", "sendgrid",
            "--above", "stripe",
        ],
        None,
    );
    assert!(!out.status.success(), "conflicting anchors must fail");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("at most one"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // --auto together with an anchor.
    let out = proj.run(
        &tmp,
        &[
            "config", "set", "cf", "x.y.z", "--value", "v", "--yes", "-p", "--auto", "--below",
            "sendgrid",
        ],
        None,
    );
    assert!(!out.status.success(), "--auto + anchor must fail");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("at most one"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // An anchor without -p.
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "cf",
            "inference.jev.api_key",
            "--value",
            "v",
            "--yes",
            "--below",
            "sendgrid",
        ],
        None,
    );
    assert!(!out.status.success(), "anchor without -p must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("require -p/--parents"),
        "hint missing:\n{stderr}"
    );

    // -p with an anchor missing from the subject: clear hard error.
    let out = proj.run(
        &tmp,
        &[
            "config", "set", "cf", "x.y.z", "--value", "v", "--yes", "-p", "--below", "mailgun",
        ],
        None,
    );
    assert!(!out.status.success(), "missing anchor must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("anchor `mailgun` not found"),
        "anchor error missing:\n{stderr}"
    );
}

#[test]
fn parents_insert_into_loaded_external_file() {
    let (proj, tmp) = Proj::new("pext", "app:\n  name: noizu\n");
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.conn.host",
            "--value",
            "h",
            "--yes",
            "-p",
            "--below",
            "name",
        ],
        None,
    );
    ok(&out, "external -p insert");
    let written = std::fs::read_to_string(proj.dir.join("team.yaml")).unwrap();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines[0], "app:");
    assert_eq!(lines[1], "  name: noizu");
    assert_eq!(lines[2], "  db:");
    assert_eq!(lines[3], "    conn:");
    assert!(
        lines[4].starts_with("      host: \"🔒 🔒:v1:"),
        "{}",
        lines[4]
    );
}

#[test]
fn parents_already_exist_positions_leaf_among_siblings() {
    let (proj, tmp) = Proj::new("pleaf", "meta:\n  unused: 1\n");
    proj.envrc("dc_yaml --no-bump cf <<'YAML'\ngrp:\n  zed: z\nYAML\n");
    let out = proj.run(
        &tmp,
        &[
            "config", "set", "cf", "grp.aaa", "--value", "v", "--yes", "-p",
        ],
        None,
    );
    ok(&out, "leaf auto placement");
    let written = proj.read_envrc();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines[1], "grp:");
    assert!(lines[2].starts_with("  aaa: \"🔒 🔒:v1:"), "{}", lines[2]);
    assert_eq!(lines[3], "  zed: z");
}
