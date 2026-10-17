//! `dc config unset|delete` — remove a leaf/branch from SOURCE files.
//! Temp fixtures only (own project dir, throwaway key, XDG_STATE_HOME sandbox).

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

    /// Read `team.yaml` (the external file seeded by `Proj::new`).
    fn read_team(&self) -> String {
        std::fs::read_to_string(self.dir.join("team.yaml")).unwrap()
    }

    fn run(
        &self,
        tmp: &tempfile::TempDir,
        args: &[&str],
        extra_env: &[(&str, &str)],
    ) -> std::process::Output {
        let mut c = dc();
        c.current_dir(&self.dir)
            .env("DC_KEYS_FILE", tmp.path().join("keys"))
            .env("XDG_STATE_HOME", tmp.path().join("state"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in extra_env {
            c.env(k, v);
        }
        let mut child = c.spawn().unwrap();
        child.stdin.as_mut().unwrap().write_all(b"").unwrap();
        child.wait_with_output().unwrap()
    }
}


fn ok(out: &std::process::Output, what: &str) -> (String, String) {
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "dc {what} failed:\nstdout: {stdout}\nstderr: {stderr}");
    (stdout, stderr)
}

const TEAM: &str = concat!(
    "# top comment\n",
    "vnext_sites:\n",
    "  # lead for keep\n",
    "  keep:\n",
    "    host: keephost-VALMARK1\n",
    "  stray_branch:\n",
    "    host: strayhost-VALMARK2\n",
    "    nested:\n",
    "      token: straytok-VALMARK3\n",
    "  # trailing comment for other\n",
    "  other:\n",
    "    host: otherhost-VALMARK4\n",
    "bystander:\n",
    "  x: 1 # inline keep\n",
);

fn team_path(proj: &Proj) -> String {
    proj.dir.join("team.yaml").to_string_lossy().into_owned()
}

fn assert_no_values(out: &std::process::Output) {
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!all.contains("VALMARK"), "output leaked a value: {all}");
}

fn team_proj(tag: &str) -> (Proj, tempfile::TempDir) {
    Proj::new(tag, TEAM)
}

#[test]
fn unset_branch_removes_only_that_branch_and_keeps_comments() {
    let (proj, tmp) = team_proj("br");
    let f = team_path(&proj);
    let out = proj.run(&tmp, &["config", "unset", "vnext_sites", "stray_branch", "--file", &f, "--yes"], &[]);
    let (stdout, _) = ok(&out, "config unset");
    assert_no_values(&out);
    assert!(stdout.contains("stray_branch"), "{stdout}");
    assert!(stdout.contains("host") && stdout.contains("nested"), "child key names listed: {stdout}");
    let expected = TEAM.replace(
        concat!(
            "  stray_branch:\n",
            "    host: strayhost-VALMARK2\n",
            "    nested:\n",
            "      token: straytok-VALMARK3\n",
        ),
        "",
    );
    assert_eq!(proj.read_team(), expected, "comments/neighbours byte-preserved");
    // store layer no longer holds it, neighbour still does
    let gone = proj.run(&tmp, &["get", "vnext_sites", "stray_branch.host"], &[]);
    assert!(!String::from_utf8_lossy(&gone.stdout).contains("VALMARK2"));
    let kept = proj.run(&tmp, &["get", "vnext_sites", "keep.host"], &[]);
    assert!(String::from_utf8_lossy(&kept.stdout).contains("VALMARK1"));
}

#[test]
fn delete_alias_removes_a_leaf() {
    let (proj, tmp) = team_proj("leaf");
    let f = team_path(&proj);
    let out = proj.run(&tmp, &["config", "delete", "vnext_sites", "keep.host", "--file", &f, "--yes"], &[]);
    ok(&out, "config delete");
    assert_no_values(&out);
    assert_eq!(proj.read_team(), TEAM.replace("    host: keephost-VALMARK1\n", ""));
    // empty parent left in place by default
    assert!(proj.read_team().contains("  keep:\n"));
}

#[test]
fn prune_empty_removes_emptied_parent() {
    let (proj, tmp) = team_proj("prune");
    let f = team_path(&proj);
    let out = proj.run(&tmp, &["config", "unset", "vnext_sites", "keep.host", "--file", &f, "--yes", "--prune-empty"], &[]);
    ok(&out, "unset --prune-empty");
    assert_no_values(&out);
    assert_eq!(proj.read_team(), TEAM.replace("  keep:\n    host: keephost-VALMARK1\n", ""));
}

#[test]
fn not_found_exits_nonzero_and_leaves_file() {
    let (proj, tmp) = team_proj("nf");
    let f = team_path(&proj);
    let out = proj.run(&tmp, &["config", "unset", "vnext_sites", "nope.host", "--file", &f, "--yes"], &[]);
    assert!(!out.status.success());
    assert_no_values(&out);
    assert_eq!(proj.read_team(), TEAM);
}

#[test]
fn dry_run_writes_nothing_and_prints_no_values() {
    let (proj, tmp) = team_proj("dry");
    let f = team_path(&proj);
    let out = proj.run(&tmp, &["config", "unset", "vnext_sites", "stray_branch", "--file", &f, "--dry-run"], &[]);
    let (stdout, _) = ok(&out, "unset --dry-run");
    assert_no_values(&out);
    assert!(stdout.contains("Would remove") && stdout.contains("stray_branch"));
    assert_eq!(proj.read_team(), TEAM);
    let still = proj.run(&tmp, &["get", "vnext_sites", "stray_branch.host"], &[]);
    assert!(String::from_utf8_lossy(&still.stdout).contains("VALMARK2"), "store untouched");
}

#[test]
fn without_yes_and_no_tty_refuses_and_changes_nothing() {
    let (proj, tmp) = team_proj("notty");
    let f = team_path(&proj);
    let out = proj.run(&tmp, &["config", "unset", "vnext_sites", "stray_branch", "--file", &f], &[]);
    assert!(!out.status.success());
    assert_no_values(&out);
    assert_eq!(proj.read_team(), TEAM);
}

#[test]
fn post_write_mismatch_restores_original() {
    let (proj, tmp) = team_proj("restore");
    let f = team_path(&proj);
    let out = proj.run(
        &tmp,
        &["config", "unset", "vnext_sites", "stray_branch", "--file", &f, "--yes"],
        &[("DC_TEST_POST_WRITE_CORRUPT", "1")],
    );
    assert!(!out.status.success());
    assert_eq!(proj.read_team(), TEAM);
}

#[test]
fn heredoc_envrc_leaf_is_removed() {
    let (proj, tmp) = Proj::new("hd", "meta:\n  unused: 1\n");
    proj.envrc(FLEET_FILE);
    let out = proj.run(&tmp, &["config", "unset", "fleet", "google.email", "--yes"], &[]);
    ok(&out, "unset heredoc");
    assert_eq!(
        proj.read_envrc(),
        FLEET_FILE.replace("  email: x\n", "")
    );
}

const FLEET_FILE: &str = concat!(
    "dc_yaml --no-bump fleet <<'YAML'\n",
    "google:\n",
    "  email: x\n",
    "# Sites\n",
    "vnext_sites:\n",
    "  agentonomicom:\n",
    "    name: y\n",
    "YAML\n",
);
