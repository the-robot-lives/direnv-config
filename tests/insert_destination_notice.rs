//! New-key inserts (`dc config set` errata path, `dc config setall`) must name
//! their resolved destination as `file:line` on **stderr** — stdout stays
//! clean so pipelines are unaffected (user decision 2026-10-12).
//!
//! The destination is asserted **exactly**: the full
//! `inserted <subject> <key> at <file>:<line>` shape with the real
//! (canonicalized) file path and a concrete line number, pinned whole-line
//! against `stderr.lines()`. A loose substring such as `team.yaml:` can be
//! satisfied by unrelated stderr (e.g. the vanished-registry-file warning
//! also names files), letting a wrong, misspelled, or line-less notice pass
//! (the-robot-reviews on #19).

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
        std::fs::write(
            dir.join("team.yaml"),
            "app:\n  name: noizu\n",
        )
        .unwrap();

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
}

/// The destination file exactly as the binary prints it: an absolute,
/// canonicalized path (`f.display()` of the resolved candidate).
fn team_yaml(proj: &Proj) -> String {
    proj.dir.join("team.yaml").display().to_string()
}

/// The exact notice line must be present as a whole stderr line — not as a
/// substring some other line happens to contain.
fn assert_stderr_line(stderr: &str, expected: &str) {
    assert!(
        stderr.lines().any(|l| l == expected),
        "stderr lacks the exact notice line `{expected}`:\n{stderr}"
    );
}

/// stdout must carry no insert-notice text at all: neither verb, nor the
/// inserted key, nor the destination file.
fn assert_stdout_clean(stdout: &str, notice_tokens: &[&str]) {
    for token in notice_tokens {
        assert!(
            !stdout.contains(token),
            "stdout polluted with notice text `{token}`:\n{stdout}"
        );
    }
}

#[test]
fn config_set_new_key_names_destination_on_stderr_not_stdout() {
    let (proj, tmp) = Proj::new("set");
    let out = proj.run(&tmp, &["config", "set", "app", "api_key", "--value", "tok", "--yes"], None);
    assert!(
        out.status.success(),
        "dc config set failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Fixture geometry: `app:` (line 1) + `  name: noizu` (line 2) — a new
    // app child lands on line 3. Ground the pinned `:3` in the written file
    // so it cannot silently rot if insertion logic moves.
    let written = std::fs::read_to_string(proj.dir.join("team.yaml")).unwrap();
    assert!(
        written
            .lines()
            .nth(2)
            .is_some_and(|l| l.trim_start().starts_with("api_key:")),
        "fixture drift: expected api_key on line 3 of team.yaml:\n{written}"
    );

    // destination is named as file:line on stderr (pre-insert notice + result)
    let file = team_yaml(&proj);
    assert_stderr_line(&stderr, &format!("Insert app api_key into {file}:3"));
    assert_stderr_line(&stderr, &format!("inserted app api_key at {file}:3"));

    // stdout is unpolluted — no fragment of the notices may appear on it
    assert_stdout_clean(&stdout, &["Insert", "inserted", "api_key", "team.yaml"]);
}

#[test]
fn config_setall_insert_names_destination_on_stderr_not_stdout() {
    let (proj, tmp) = Proj::new("setall");
    let out = proj.run(
        &tmp,
        &["config", "setall", "app", "--below", "name", "--yes"],
        Some("extra: \"v\"\n"),
    );
    assert!(
        out.status.success(),
        "dc config setall failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Anchor `app name` sits on line 2, so `--below name` lands the section
    // on line 3. Ground the pinned `:3` in the written file.
    let written = std::fs::read_to_string(proj.dir.join("team.yaml")).unwrap();
    assert!(
        written
            .lines()
            .nth(2)
            .is_some_and(|l| l.trim_start() == "extra: \"v\""),
        "fixture drift: expected `extra: \"v\"` on line 3 of team.yaml:\n{written}"
    );

    let file = team_yaml(&proj);
    assert_stderr_line(&stderr, &format!("Insert 1 line(s) below app name in {file}:3"));
    assert_stderr_line(&stderr, &format!("inserted section at {file}:3"));

    assert_stdout_clean(&stdout, &["Insert", "inserted", "extra", "team.yaml"]);
}
