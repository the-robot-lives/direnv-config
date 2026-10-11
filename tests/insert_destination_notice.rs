//! New-key inserts (`dc config set` errata path, `dc config setall`) must name
//! their resolved destination as `file:line` on **stderr** — stdout stays
//! clean so pipelines are unaffected (user decision 2026-10-12).

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

    // destination is named as file:line on stderr (pre-insert notice + result)
    assert!(
        stderr.contains("Insert app api_key into"),
        "stderr lacks insert notice:\n{stderr}"
    );
    assert!(
        stderr.contains("inserted app api_key at"),
        "stderr lacks inserted-at notice:\n{stderr}"
    );
    assert!(
        stderr.contains("team.yaml:"),
        "stderr lacks file:line destination:\n{stderr}"
    );
    // stdout is unpolluted — no destination, no inserted chatter
    assert!(
        !stdout.contains("team.yaml") && !stdout.contains("inserted"),
        "stdout polluted:\n{stdout}"
    );
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

    assert!(
        stderr.contains("Insert 1 line(s) below app name in"),
        "stderr lacks insert notice:\n{stderr}"
    );
    assert!(
        stderr.contains("inserted section at") && stderr.contains("team.yaml:"),
        "stderr lacks inserted-at destination:\n{stderr}"
    );
    assert!(
        !stdout.contains("team.yaml") && !stdout.contains("inserted"),
        "stdout polluted:\n{stdout}"
    );
}
