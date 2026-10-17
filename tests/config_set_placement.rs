//! `dc config set` insertion placement — the hard safety contract.
//!
//! Regression coverage for two real incidents (2026-10-17):
//!
//! 1. A new child of an existing mapping was written AFTER the parent's
//!    trailing blank lines and the next section's leading comment (between
//!    the comment and the next key) — visually inside the wrong section.
//!    It must land directly after the parent's last child line.
//! 2. With `-p` on an external multi-subject file, path segments resolved
//!    against OTHER subjects' keys (`defaults npm.*` matched
//!    `vnext_infra.npm` ~5000 lines away) — silently writing a secret into
//!    a different section. Path resolution is now strictly subject-scoped.
//!
//! Plus the parse-based safety gates: unresolvable parent ⇒ non-zero with
//! the file untouched; forced post-write mismatch ⇒ original bytes restored,
//! non-zero exit.
//!
//! Everything runs against temp fixtures (own project dir, throwaway key
//! file, `XDG_STATE_HOME` sandbox) — never against a real store. `--plain`
//! values are used for byte-exact file comparisons.

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

/// The user's incident shape: mapping followed by blank lines, then a
/// comment leading the next section.
const FLEET_FILE: &str = concat!(
    "dc_yaml --no-bump fleet <<'YAML'\n",
    "google:\n",
    "  email: x\n",
    "\n",
    "\n",
    "\n",
    "# Sites - our products like therobotlearns, therobotplans etc.\n",
    "vnext_sites:\n",
    "  agentonomicom:\n",
    "    name: y\n",
    "YAML\n",
);

fn ok(out: &std::process::Output, what: &str) -> (String, String) {
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        out.status.success(),
        "dc {what} failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
    (stdout, stderr)
}

// ---------------------------------------------------------------------------
// Incident 1: insertion point after the parent's last child line
// ---------------------------------------------------------------------------

#[test]
fn insert_lands_right_after_parent_last_child_before_blanks_and_comment() {
    // User's exact shape, WITH -p (as they ran it).
    let (proj, tmp) = Proj::new("place1", "meta:\n  unused: 1\n");
    proj.envrc(FLEET_FILE);
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "fleet",
            "google.password",
            "--value",
            "S",
            "--plain",
            "--yes",
            "-p",
        ],
        &[],
    );
    ok(&out, "config set -p placement");

    let written = proj.read_envrc();
    let lines: Vec<&str> = written.lines().collect();
    // `password` must sit IMMEDIATELY after `email: x` — before the blank
    // run and before the `# Sites` comment that leads the next section.
    assert_eq!(lines[0], "dc_yaml --no-bump fleet <<'YAML'");
    assert_eq!(lines[1], "google:");
    assert_eq!(lines[2], "  email: x");
    assert_eq!(lines[3], "  password: \"S\"", "wrong line: {}", lines[3]);
    assert_eq!(lines[4], "", "blank run must survive below the insert");
    assert_eq!(
        lines[7], "# Sites - our products like therobotlearns, therobotplans etc.",
        "next section's comment untouched"
    );
    assert_eq!(lines[8], "vnext_sites:");
    assert_eq!(lines[9], "  agentonomicom:");
    assert_eq!(lines[10], "    name: y");
    assert_eq!(lines[11], "YAML");

    // The refreshed store must resolve the new key at its true path only.
    let get = proj.run(&tmp, &["config", "get", "fleet", "google.password"], &[]);
    ok(&get, "config get after placement insert");
}

#[test]
fn insert_without_parents_flag_same_placement() {
    // Same shape, plain insert (no -p): parent exists, leaf missing.
    let (proj, tmp) = Proj::new("place2", "meta:\n  unused: 1\n");
    proj.envrc(FLEET_FILE);
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "fleet",
            "google.password",
            "--value",
            "S",
            "--plain",
            "--yes",
        ],
        &[],
    );
    ok(&out, "config set plain insert");
    let written = proj.read_envrc();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines[3], "  password: \"S\"");
    assert_eq!(
        lines[7],
        "# Sites - our products like therobotlearns, therobotplans etc."
    );
}

#[test]
fn insert_when_parent_is_last_section_before_terminator() {
    // Parent is the LAST section: trailing blanks then the heredoc
    // terminator; insert goes right after the last child, before blanks.
    let (proj, tmp) = Proj::new("place3", "meta:\n  unused: 1\n");
    proj.envrc(concat!(
        "dc_yaml --no-bump fleet <<'YAML'\n",
        "google:\n",
        "  email: x\n",
        "\n",
        "\n",
        "YAML\n",
    ));
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "fleet",
            "google.password",
            "--value",
            "S",
            "--plain",
            "--yes",
        ],
        &[],
    );
    ok(&out, "config set last-section insert");
    let written = proj.read_envrc();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines[2], "  email: x");
    assert_eq!(
        lines[3], "  password: \"S\"",
        "right after email, before blanks"
    );
    assert_eq!(lines[4], "", "first trailing blank kept");
    assert_eq!(lines[6], "YAML");
}

#[test]
fn insert_after_nested_children_and_deeper_comments() {
    // Parent's last child has deeper-indented grandchildren and a deeper
    // comment: the insert goes after that whole deeper block.
    let (proj, tmp) = Proj::new("place4", "meta:\n  unused: 1\n");
    proj.envrc(concat!(
        "dc_yaml --no-bump fleet <<'YAML'\n",
        "google:\n",
        "  email: x\n",
        "  oauth:\n",
        "    client_id: cid\n",
        "    # client secret lives in the vault\n",
        "    secret_ref: vault://g\n",
        "next_section:\n",
        "  k: 1\n",
        "YAML\n",
    ));
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "fleet",
            "google.password",
            "--value",
            "S",
            "--plain",
            "--yes",
        ],
        &[],
    );
    ok(&out, "config set nested insert");
    let written = proj.read_envrc();
    let lines: Vec<&str> = written.lines().collect();
    // After the deepest grandchild (secret_ref), before `next_section:`.
    assert_eq!(lines[6], "    secret_ref: vault://g");
    assert_eq!(lines[7], "  password: \"S\"");
    assert_eq!(lines[8], "next_section:");
    assert_eq!(lines[9], "  k: 1");
}

#[test]
fn insert_when_parent_followed_directly_by_next_key() {
    // No blank lines at all between the parent's last child and the next key.
    let (proj, tmp) = Proj::new("place5", "meta:\n  unused: 1\n");
    proj.envrc(concat!(
        "dc_yaml --no-bump fleet <<'YAML'\n",
        "google:\n",
        "  email: x\n",
        "vnext_sites:\n",
        "  k: 1\n",
        "YAML\n",
    ));
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "fleet",
            "google.password",
            "--value",
            "S",
            "--plain",
            "--yes",
        ],
        &[],
    );
    ok(&out, "config set no-blank insert");
    let written = proj.read_envrc();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines[2], "  email: x");
    assert_eq!(lines[3], "  password: \"S\"");
    assert_eq!(lines[4], "vnext_sites:");
}

// ---------------------------------------------------------------------------
// Preview renders the real write position and the true parent chain
// ---------------------------------------------------------------------------

#[test]
fn preview_shows_plus_line_at_real_insert_position() {
    let (proj, tmp) = Proj::new("place6", "meta:\n  unused: 1\n");
    proj.envrc(FLEET_FILE);
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "fleet",
            "google.password",
            "--value",
            "S",
            "--plain",
            "--yes",
        ],
        &[],
    );
    let (_stdout, stderr) = ok(&out, "config set preview");
    // The context block: the ➡-marked line is the parent's last child
    // (`email`), and the ✚ line prints immediately after it — at the real
    // write position. Any following-section lines that fall in the window
    // appear AFTER the ✚, never above it (the old bug rendered them as
    // if they were the parent).
    let ctx = stderr.find("┄┄ context ┄┄").expect("context header");
    let arrow = stderr[ctx..].find("email: x").expect("arrow line (email)");
    let plus = stderr[ctx..].find("✚").expect("plus line");
    assert!(
        arrow < plus,
        "✚ must print right after the ➡-marked parent line:\n{stderr}"
    );
    assert!(
        stderr[ctx..ctx + plus].contains("➡"),
        "the line above ✚ must be ➡-marked:\n{stderr}"
    );
    // The marked line must be the email line (the true parent's last
    // child), not the next section's comment/key.
    let marked = stderr[ctx..]
        .match_indices("➡")
        .next()
        .expect("arrow mark")
        .0;
    assert!(
        stderr[marked..].contains("email: x"),
        "➡ must mark the parent's last child, got:\n{}",
        &stderr[marked..marked + 120]
    );
    if let Some(comment) = stderr[ctx..].find("# Sites") {
        assert!(
            plus < comment,
            "next section's comment must appear after the ✚, not as the parent:\n{stderr}"
        );
    }
}

// ---------------------------------------------------------------------------
// Incident 2: path segments resolve strictly within the subject's own block
// ---------------------------------------------------------------------------

/// External file where `npm` exists ONLY under another subject.
const CROSS_SUBJECT_FILE: &str = "\
defaults:
  inference:
    x: 1
  smtp:

vnext_infra:
  npm:
    name: a
";

#[test]
fn parents_creates_missing_section_in_own_subject_not_namesake_elsewhere() {
    // The incident: `dc config set defaults npm.self-hosted.token -p` used
    // to match vnext_infra.npm and write inside it. It must create
    // defaults.npm.self-hosted.token instead; vnext_infra stays untouched.
    let (proj, tmp) = Proj::new("place7", CROSS_SUBJECT_FILE);
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "defaults",
            "npm.self-hosted.token",
            "--value",
            "S",
            "--plain",
            "--yes",
            "-p",
        ],
        &[],
    );
    ok(&out, "config set -p cross-subject");

    // Byte-exact expectation: chain lands alphabetically inside defaults
    // (before smtp), vnext_infra byte-identical.
    let expected = "\
defaults:
  inference:
    x: 1
  npm:
    self-hosted:
      token: \"S\"
  smtp:

vnext_infra:
  npm:
    name: a
";
    assert_eq!(proj.read_team(), expected, "file diverged from expectation");

    // And the value resolves at the requested path, not the namesake's.
    let get = proj.run(
        &tmp,
        &["config", "get", "defaults", "npm.self-hosted.token"],
        &[],
    );
    ok(&get, "config get after cross-subject insert");
}

#[test]
fn missing_parent_without_parents_errors_and_leaves_file_byte_identical() {
    let (proj, tmp) = Proj::new("place8", CROSS_SUBJECT_FILE);
    let before = proj.read_team();
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "defaults",
            "npm.self-hosted.token",
            "--value",
            "S",
            "--plain",
            "--yes",
        ],
        &[],
    );
    assert!(
        !out.status.success(),
        "missing parent without -p must fail (not write into the namesake section):\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("pass -p/--parents"),
        "hint missing:\n{stderr}"
    );
    assert_eq!(
        proj.read_team(),
        before,
        "file must be byte-identical after a refused write"
    );
}

// ---------------------------------------------------------------------------
// Hard safety gates
// ---------------------------------------------------------------------------

#[test]
fn unresolvable_parent_in_heredoc_halts_without_writing() {
    // No `google` section at all → plain insert must refuse, byte-identical.
    let (proj, tmp) = Proj::new("place9", "meta:\n  unused: 1\n");
    proj.envrc(concat!(
        "dc_yaml --no-bump fleet <<'YAML'\n",
        "other:\n",
        "  k: 1\n",
        "YAML\n",
    ));
    let before = proj.read_envrc();
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "fleet",
            "google.password",
            "--value",
            "S",
            "--plain",
            "--yes",
        ],
        &[],
    );
    assert!(
        !out.status.success(),
        "unresolvable parent must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("fleet") && stderr.contains("google.password"),
        "warning must name subject + path:\n{stderr}"
    );
    assert_eq!(proj.read_envrc(), before, "file must be untouched");
}

#[test]
fn forced_post_write_mismatch_restores_original_bytes_and_fails() {
    // Test hook DC_TEST_POST_WRITE_CORRUPT simulates a post-write check
    // failure: dc must restore the original bytes and exit non-zero.
    let (proj, tmp) = Proj::new("place10", "meta:\n  unused: 1\n");
    proj.envrc(FLEET_FILE);
    let before = proj.read_envrc();
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "fleet",
            "google.password",
            "--value",
            "S",
            "--plain",
            "--yes",
        ],
        &[("DC_TEST_POST_WRITE_CORRUPT", "1")],
    );
    assert!(
        !out.status.success(),
        "forced post-write mismatch must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("post-write verification failed") && stderr.contains("restored"),
        "must report the verification failure and the restore:\n{stderr}"
    );
    assert_eq!(
        proj.read_envrc(),
        before,
        "original bytes must be restored after a failed post-write check"
    );
}

#[test]
fn update_of_existing_key_stays_in_place_and_verifies() {
    // The gate applies to updates too: value replaced, structure untouched.
    let (proj, tmp) = Proj::new("place11", "meta:\n  unused: 1\n");
    proj.envrc(FLEET_FILE);
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "fleet",
            "google.email",
            "--value",
            "newmail",
            "--plain",
            "--yes",
        ],
        &[],
    );
    ok(&out, "config set update");
    let written = proj.read_envrc();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines[2], "  email: \"newmail\"");
    // An update adds no line: the next section's comment stays at index 6.
    assert_eq!(
        lines[6],
        "# Sites - our products like therobotlearns, therobotplans etc."
    );
    assert_eq!(lines[7], "vnext_sites:");
}

#[test]
fn encrypted_insert_position_and_no_echo() {
    // Encrypted (default) mode: same placement; token never echoes.
    let (proj, tmp) = Proj::new("place12", "meta:\n  unused: 1\n");
    proj.envrc(FLEET_FILE);
    const SECRET: &str = "Super-Secret-42";
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "fleet",
            "google.password",
            "--value",
            SECRET,
            "--yes",
            "-p",
        ],
        &[],
    );
    let (_stdout, stderr) = ok(&out, "config set encrypted placement");
    let written = proj.read_envrc();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines[2], "  email: x");
    assert!(
        lines[3].starts_with("  password: \"🔒 🔒:v1:"),
        "position + marker expected: {}",
        lines[3]
    );
    assert!(
        !stderr.contains(SECRET) && !written.contains(SECRET),
        "plaintext leaked"
    );
}

/// Fourth variant of the placement bug (found by post-fix smoke): an insert
/// placed BEFORE the next sibling (`--above z`, or `--auto` sorting ahead of
/// a later sibling) used the sibling's own key line directly — which sits
/// after the previous sibling's trailing blank run and the next section's
/// lead-in comment. The insert must walk back over that lead-in and land
/// right after the last real line above the anchor.
const SIBLINGS_FILE: &str = concat!(
    "dc_yaml --no-bump aaa <<'YAML'\n",
    "b: 1\n",
    "\n",
    "# leads k\n",
    "k: 2\n",
    "\n",
    "# leads z\n",
    "z: 3\n",
    "YAML\n",
);

fn assert_between_k_and_z(written: &str) {
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(
        lines[4],
        "k: 2",
        "k must stay put: {:?}",
        &lines[..lines.len().min(10)]
    );
    assert_eq!(
        lines[5], "m:",
        "insert must follow k: 2 immediately: {}",
        lines[5]
    );
    assert_eq!(lines[6], "  key: \"v\"");
    assert_eq!(lines[7], "", "blank run must survive below the insert");
    assert_eq!(lines[8], "# leads z", "lead-in comment must stay with z");
    assert_eq!(lines[9], "z: 3");
}

#[test]
fn auto_insert_before_sort_next_sibling_skips_its_lead_in() {
    // `m` sorts after `k` and before `z`, so auto anchors on `z` — whose
    // key line is preceded by a blank run and the `# leads z` comment.
    let (proj, tmp) = Proj::new("place13", "meta:\n  unused: 1\n");
    proj.envrc(SIBLINGS_FILE);
    let out = proj.run(
        &tmp,
        &[
            "config", "set", "aaa", "m.key", "--value", "v", "--plain", "--yes", "-p",
        ],
        &[],
    );
    ok(&out, "config set auto before-sibling");
    assert_between_k_and_z(&proj.read_envrc());
}

#[test]
fn above_insert_before_anchor_skips_its_lead_in() {
    // `--above z` resolves to the same slot: directly after `k: 2`, never
    // between the `# leads z` comment and `z: 3`.
    let (proj, tmp) = Proj::new("place14", "meta:\n  unused: 1\n");
    proj.envrc(SIBLINGS_FILE);
    let out = proj.run(
        &tmp,
        &[
            "config", "set", "aaa", "m.key", "--above", "z", "--value", "v", "--plain", "--yes",
            "-p",
        ],
        &[],
    );
    ok(&out, "config set --above before-anchor");
    assert_between_k_and_z(&proj.read_envrc());
}
