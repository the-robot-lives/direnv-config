//! Provenance-routing verification for `dc config set` (W2): pins where an
//! insert/edits lands when a key's registered sources overlap.
//!
//! Pinned contracts:
//! - a key defined only in a registered external file is edited there, and the
//!   notice names that file;
//! - a key defined in BOTH an `.envrc*` heredoc and a registered external file
//!   is an explicit-ambiguity error (no silent precedence); `--file` pins the
//!   destination to either side;
//! - a brand-new key (errata insert) lands in the last-loaded registered
//!   external file, ahead of `.envrc*` heredocs;
//! - `.envrc.dc` (the `.envrc.*` glob) is scanned like `.envrc` itself.
//!
//! Driven through the real binary against a tempdir fixture project, modeled
//! on tests/seal.rs. No real secret material — fixture strings only.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn dc() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_dc"));
    c.env_remove("DC_ENV");
    c
}

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
        (Proj { dir }, tmp)
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

    fn load(&self, tmp: &tempfile::TempDir, files: &[&str]) {
        let mut args = vec!["load"];
        args.extend(files);
        let out = self.run(tmp, &args, None);
        assert!(
            out.status.success(),
            "dc load {files:?} failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.dir.join(name)).unwrap()
    }

    fn assert_success(&self, out: &std::process::Output, what: &str) {
        assert!(
            out.status.success(),
            "{what} failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }
}

/// `dc config set <subject> <path> --value <v> --yes` returns (status, stdout, stderr).
fn set(proj: &Proj, tmp: &tempfile::TempDir, subject: &str, path: &str, value: &str) -> std::process::Output {
    proj.run(
        tmp,
        &["config", "set", subject, path, "--value", value, "--yes"],
        None,
    )
}

/// A `.envrc` whose heredoc defines `app` but never `app.host` — so an
/// `app.host` hit can only come from a registered external file.
fn envrc_without_host(dir: &Path) {
    std::fs::write(
        dir.join(".envrc"),
        "dc_yaml --no-bump app <<'YAML'\nregion: us-east-1\nYAML\n",
    )
    .unwrap();
}

#[test]
fn config_set_writes_registered_external_and_names_it() {
    let (proj, tmp) = Proj::new("external-only");
    envrc_without_host(&proj.dir);
    std::fs::write(
        proj.path("team.yaml"),
        "app:\n  host: \"🔒 old-host-value\"\n",
    )
    .unwrap();
    proj.load(&tmp, &["team.yaml"]);
    let team_before = proj.read("team.yaml");

    let out = set(&proj, &tmp, "app", "host", "new-host-value");
    proj.assert_success(&out, "config set on external-only key");

    // the notice names the external file
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&format!("updated {}:", proj.path("team.yaml").display())),
        "stdout must name the external file:\n{stdout}"
    );
    assert!(
        stdout.contains(":2"),
        "notice must carry the line number:\n{stdout}"
    );

    // the external file was spliced in place; the .envrc heredoc untouched
    let team = proj.read("team.yaml");
    assert_eq!(team.lines().count(), team_before.lines().count(), "line count preserved");
    assert!(
        team.contains("🔒:v1:t0:"),
        "host now holds an encrypted token:\n{team}"
    );
    assert!(
        !team.contains("old-host-value"),
        "old plaintext gone:\n{team}"
    );
    assert!(
        !proj.read(".envrc").contains("🔒:v1:"),
        ".envrc heredoc must be untouched"
    );

    // and the store refresh picked the edit up
    let out = proj.run(&tmp, &["get", "app", "host", "--reveal"], None);
    proj.assert_success(&out, "get --reveal after external edit");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "new-host-value");
}

#[test]
fn config_set_dual_definition_is_ambiguous_and_file_flag_pins_destination() {
    let (proj, tmp) = Proj::new("dual");
    std::fs::write(
        proj.path(".envrc"),
        "dc_yaml --no-bump app <<'YAML'\nhost: \"🔒 env-side-value\"\nYAML\n",
    )
    .unwrap();
    std::fs::write(
        proj.path("team.yaml"),
        "app:\n  host: \"🔒 ext-side-value\"\n",
    )
    .unwrap();
    proj.load(&tmp, &["team.yaml"]);
    let envrc_before = proj.read(".envrc");
    let team_before = proj.read("team.yaml");

    // both live definitions -> refusal, both candidate files named
    let out = set(&proj, &tmp, "app", "host", "replacement");
    assert!(
        !out.status.success(),
        "a key defined in two live sources must not be silently routed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("ambiguous"), "stderr:\n{stderr}");
    assert!(
        stderr.contains(&proj.path(".envrc").display().to_string()),
        "both candidates named:\n{stderr}"
    );
    assert!(
        stderr.contains(&proj.path("team.yaml").display().to_string()),
        "both candidates named:\n{stderr}"
    );
    assert!(
        stderr.contains("--file"),
        "refusal points at the --file escape hatch:\n{stderr}"
    );
    assert_eq!(
        proj.read(".envrc"),
        envrc_before,
        "ambiguous refusal must not write"
    );
    assert_eq!(
        proj.read("team.yaml"),
        team_before,
        "ambiguous refusal must not write"
    );

    // --file pins the external side
    let out = proj.run(
        &tmp,
        &["config", "set", "app", "host", "--value", "pinned-ext", "--yes", "--file", "team.yaml"],
        None,
    );
    proj.assert_success(&out, "--file team.yaml");
    let team = proj.read("team.yaml");
    assert!(team.contains("🔒:v1:"), "external side updated:\n{team}");
    assert!(!team.contains("ext-side-value"));
    assert_eq!(proj.read(".envrc"), envrc_before, "heredoc side untouched");

    // --file pins the heredoc side too
    let out = proj.run(
        &tmp,
        &["config", "set", "app", "host", "--value", "pinned-env", "--yes", "--file", ".envrc"],
        None,
    );
    proj.assert_success(&out, "--file .envrc");
    let envrc = proj.read(".envrc");
    assert!(envrc.contains("🔒:v1:"), "heredoc side updated:\n{envrc}");
    assert_eq!(proj.read("team.yaml"), team, "external side untouched");
}

#[test]
fn config_set_errata_new_key_lands_in_last_loaded_external() {
    let (proj, tmp) = Proj::new("errata");
    envrc_without_host(&proj.dir);
    std::fs::write(proj.path("a.yaml"), "app:\n  host: one\n").unwrap();
    std::fs::write(proj.path("b.yaml"), "app:\n  pool: 9\n").unwrap();
    proj.load(&tmp, &["a.yaml", "b.yaml"]);
    let a_before = proj.read("a.yaml");
    let envrc_before = proj.read(".envrc");

    // `api_key` exists nowhere: candidates are b (last-loaded), a, then .envrc.
    let out = set(&proj, &tmp, "app", "api_key", "brand-new-value");
    proj.assert_success(&out, "errata insert");

    let b = proj.read("b.yaml");
    assert!(
        b.contains("api_key: \"🔒 🔒:v1:"),
        "new key lands in the last-loaded external:\n{b}"
    );
    assert_eq!(proj.read("a.yaml"), a_before, "earlier external untouched");
    assert_eq!(proj.read(".envrc"), envrc_before, "heredoc untouched");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&format!("inserted app api_key at {}:", proj.path("b.yaml").display())),
        "insert notice names the chosen file:\nstdout: {stdout}\nstderr: {stderr}"
    );

    // refresh routed through the edited external
    let out = proj.run(&tmp, &["get", "app", "api_key", "--reveal"], None);
    proj.assert_success(&out, "get --reveal after errata insert");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "brand-new-value");
}

#[test]
fn config_set_scans_envrc_dc_dotfile_like_envrc() {
    let (proj, tmp) = Proj::new("dotfile");
    // `.envrc` holds only an unrelated subject; the key lives in `.envrc.dc`.
    std::fs::write(
        proj.path(".envrc"),
        "dc_yaml --no-bump other <<'YAML'\nnoise: 1\nYAML\n",
    )
    .unwrap();
    std::fs::write(
        proj.path(".envrc.dc"),
        "dc_yaml --no-bump app <<'YAML'\nhost: \"🔒 dotfile-value\"\nYAML\n",
    )
    .unwrap();

    // `dc config set` refreshes through a store — scaffold one for this
    // heredoc-only fixture (the other tests get theirs from `dc load`)
    let out = proj.run(&tmp, &["init"], None);
    proj.assert_success(&out, "dc init");

    let before = proj.read(".envrc.dc");
    let out = set(&proj, &tmp, "app", "host", "dotfile-new");
    proj.assert_success(&out, "config set against .envrc.dc");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&format!("updated {}:", proj.path(".envrc.dc").display())),
        "notice must name .envrc.dc:\n{stdout}"
    );
    let dotfile = proj.read(".envrc.dc");
    assert!(dotfile.contains("🔒:v1:"), "edited in place:\n{dotfile}");
    assert_eq!(dotfile.lines().count(), before.lines().count());
    assert!(!proj.read(".envrc").contains("🔒:v1:"), ".envrc untouched");

    let out = proj.run(&tmp, &["get", "app", "host", "--reveal"], None);
    proj.assert_success(&out, "get --reveal after .envrc.dc edit");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "dotfile-new");

    // the merged in-scope listing attributes the key to .envrc.dc as well
    let out = proj.run(&tmp, &["secrets", "--sources"], None);
    proj.assert_success(&out, "dc secrets --sources");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let host = stdout
        .lines()
        .find(|l| l.trim_start().starts_with("host"))
        .unwrap_or_else(|| panic!("no host row:\n{stdout}"));
    assert!(
        host.contains(&format!("{}:2", proj.path(".envrc.dc").display())),
        "host attributed to .envrc.dc:2:\n{host}"
    );
}
