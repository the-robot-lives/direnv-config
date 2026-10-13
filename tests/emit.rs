//! `dc emit` end-to-end (W2): the store → loadable-yaml generator.
//!
//! Pinned contracts:
//! - gold-standard round trip: emit from a fixture store → `dc load` into a
//!   second sandboxed store → per-key resolution + `.active` parity with the
//!   source (fixtures only, never live data);
//! - 🔒 tokens re-emit byte-identical (no re-encryption; emit needs no key);
//! - multi-layer emission: non-base layers land in `<out>.<layer>.yaml`
//!   variants carrying per-subject `_dc_layer`; plain values stored in a
//!   secrets layer land in the `secrets` variant;
//! - special subjects (`_dc`, `auto`) get their own files and their own
//!   printed `dc load` invocations (`--replace` / `--if-missing`);
//! - `--dry-run` writes nothing and never prints a value;
//! - the safety gate refuses git-tracked and not-gitignored targets unless
//!   `--force`.
//!
//! Driven through the real binary against tempdir fixture projects, modeled
//! on tests/seal.rs. No real secret material — fixture strings only.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn dc() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_dc"));
    c.env_remove("DC_ENV");
    c
}

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn new(tag: &str) -> (Self, PathBuf) {
        let root = tempfile::TempDir::new().unwrap();
        std::fs::write(
            root.path().join("keys"),
            "version: 1\nkey: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\n",
        )
        .unwrap();
        let dir = Self::project(root.path(), tag);
        (Fixture { root }, dir)
    }

    /// A fresh git project with the emitted-file pattern gitignored.
    fn project(root: &Path, tag: &str) -> PathBuf {
        let dir = std::fs::canonicalize({
            std::fs::create_dir_all(root.join(tag)).unwrap();
            root.join(tag)
        })
        .unwrap();
        // canonicalize: the store hash must match what the binary computes
        // from the OS-resolved cwd (macOS $TMPDIR is a /var -> /private/var
        // symlink).
        std::fs::write(dir.join(".gitignore"), "secrets.dc.*\n").unwrap();
        git(&dir, &["init", "-q"]);
        git(&dir, &["add", ".gitignore"]);
        dir
    }

    fn run_in(&self, dir: &Path, args: &[&str], stdin: Option<&str>) -> std::process::Output {
        let mut c = dc();
        c.current_dir(dir)
            .env("DC_KEYS_FILE", self.root.path().join("keys"))
            .env("XDG_STATE_HOME", self.root.path().join("state"))
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

    /// Build the source store in `dir`: app (base plain + token + local
    /// overlay + a plain value parked in the secrets layer) and infra (plain).
    fn seed_source_store(&self, dir: &Path) {
        let cases: &[(&[&str], &str)] = &[
            (&["yaml", "app"], "host: localhost\nport: 5432\ntoken: \"🔒 round-trip-secret\"\n"),
            (&["yaml", "app", "--layer", "local"], "port: 3306\n"),
            (&["yaml", "app", "--layer", "secrets"], "plain_in_secrets: parked-plain\n"),
            (&["yaml", "infra"], "region: us-east-1\n"),
        ];
        for (args, input) in cases {
            let out = self.run_in(dir, args, Some(input));
            assert!(
                out.status.success(),
                "seeding {args:?} failed:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr),
            );
        }
    }

    /// The store directory backing `dir` (path_to_hash scheme).
    fn store_of(&self, dir: &Path) -> PathBuf {
        let name = dir.to_string_lossy().strip_prefix('/').unwrap().replace('/', "-");
        self.root.path().join("state").join("direnv-config").join(name)
    }

    fn get_out(&self, dir: &Path, args: &[&str]) -> String {
        let out = self.run_in(dir, args, None);
        assert!(
            out.status.success(),
            "dc {args:?} failed:\nstderr: {}",
            String::from_utf8_lossy(&out.stderr),
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("git available");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn emit_round_trips_store_through_dc_load() {
    let (fx, a) = Fixture::new("emit-rt-a");
    fx.seed_source_store(&a);

    // emit in A
    let out = fx.run_in(&a, &["emit"], None);
    assert!(
        out.status.success(),
        "dc emit failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("secrets.dc.yaml"), "{stdout}");
    assert!(stdout.contains("secrets.dc.local.yaml"), "{stdout}");
    assert!(stdout.contains("secrets.dc.secrets.yaml"), "{stdout}");
    let main_yaml = std::fs::read_to_string(a.join("secrets.dc.yaml")).unwrap();
    let local_yaml = std::fs::read_to_string(a.join("secrets.dc.local.yaml")).unwrap();
    let secrets_yaml = std::fs::read_to_string(a.join("secrets.dc.secrets.yaml")).unwrap();

    // token byte-fidelity: the stored token appears verbatim in the main file
    let secrets_layer =
        std::fs::read_to_string(fx.store_of(&a).join("app").join("secrets.yaml")).unwrap();
    let token = secrets_layer
        .lines()
        .find(|l| l.contains("🔒:v1:"))
        .and_then(|l| {
            let idx = l.find("🔒:v1:")?;
            Some(l[idx..].trim_end().to_string())
        })
        .expect("stored token in source secrets.yaml");
    assert!(
        main_yaml.contains(&token),
        "emitted file must carry the token byte-identical ({token}):\n{main_yaml}"
    );

    // layer routing shape
    assert!(
        local_yaml.contains("_dc_layer: local") && local_yaml.contains("port: 3306"),
        "local variant carries _dc_layer routing:\n{local_yaml}"
    );
    assert!(
        !main_yaml.contains("3306"),
        "local-layer value must not leak into the main file:\n{main_yaml}"
    );
    assert!(
        secrets_yaml.contains("_dc_layer: secrets") && secrets_yaml.contains("plain_in_secrets"),
        "plain-in-secrets variant:\n{secrets_yaml}"
    );
    assert!(
        main_yaml.contains("host: localhost") && main_yaml.contains("region: us-east-1"),
        "main file holds base plain content for all subjects:\n{main_yaml}"
    );

    // re-load into a second sandboxed store, in the printed order
    let b = Fixture::project(fx.root.as_ref(), "emit-rt-b");
    let main = a.join("secrets.dc.yaml");
    let local = a.join("secrets.dc.local.yaml");
    let secs = a.join("secrets.dc.secrets.yaml");
    let out = fx.run_in(&b, &["load", main.to_str().unwrap(), local.to_str().unwrap(), secs.to_str().unwrap()], None);
    assert!(
        out.status.success(),
        "dc load into second store failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    // per-key resolution parity (A vs B), tokens decrypted via the shared key
    for args in [
        vec!["get", "app", "host"],
        vec!["get", "app", "port"],
        vec!["get", "app", "plain_in_secrets"],
        vec!["get", "app", "token", "--reveal"],
        vec!["get", "infra", "region"],
    ] {
        let va = fx.get_out(&a, &args);
        let vb = fx.get_out(&b, &args);
        assert_eq!(va, vb, "resolution parity for {args:?}");
    }
    assert_eq!(fx.get_out(&b, &["get", "app", "token", "--reveal"]), "round-trip-secret");
    assert_eq!(fx.get_out(&b, &["get", "app", "port"]), "3306", "local overlay still wins");

    // strong parity: the resolved .active snapshots are byte-identical
    for subject in ["app", "infra"] {
        let active_a = std::fs::read(fx.store_of(&a).join(subject).join(".active")).unwrap();
        let active_b = std::fs::read(fx.store_of(&b).join(subject).join(".active")).unwrap();
        assert_eq!(
            active_a, active_b,
            ".active parity for {subject} (no plaintext asserted, bytes only)"
        );
    }
}

#[test]
fn emit_special_subjects_get_own_files_and_invocations() {
    let (fx, a) = Fixture::new("emit-sp-a");
    // `_dc` body uses dot-free keys so `dc get <path>` can address it for the
    // parity check (a literal dotted flatten key is not a get path)
    fx.run_in(&a, &["yaml", "_dc"], Some("flatten:\n  app: APP_HOST\n"));
    fx.run_in(&a, &["yaml", "auto", "--if-missing"], Some("seed: auto-seed-value\n"));
    fx.run_in(&a, &["yaml", "infra"], Some("region: eu-west-1\n"));

    let out = fx.run_in(&a, &["emit"], None);
    assert!(
        out.status.success(),
        "dc emit failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    // the exact per-flag invocations are printed (paths as passed — the cwd
    // the emit ran in, so relative names stay relative)
    assert!(
        stdout.contains("dc load --replace secrets.dc._dc.yaml"),
        "stdout must print the --replace invocation:\n{stdout}"
    );
    assert!(
        stdout.contains("dc load --if-missing secrets.dc.auto.yaml"),
        "stdout must print the --if-missing invocation:\n{stdout}"
    );
    // and the specials are NOT in the merge batch
    assert!(
        !stdout
            .lines()
            .any(|l| l.trim_start().starts_with("dc load secrets.dc.yaml")
                && (l.contains("_dc") || l.contains("auto"))),
        "specials must not ride the plain merge load:\n{stdout}"
    );

    // re-load into B with exactly the printed invocations
    let b = Fixture::project(fx.root.path(), "emit-sp-b");
    let main = a.join("secrets.dc.yaml");
    let dc_file = a.join("secrets.dc._dc.yaml");
    let auto_file = a.join("secrets.dc.auto.yaml");
    let out = fx.run_in(&b, &["load", main.to_str().unwrap()], None);
    assert!(out.status.success(), "load main failed: {}", String::from_utf8_lossy(&out.stderr));
    let out = fx.run_in(
        &b,
        &["load", "--replace", dc_file.to_str().unwrap()],
        None,
    );
    assert!(out.status.success(), "load _dc failed: {}", String::from_utf8_lossy(&out.stderr));
    let out = fx.run_in(
        &b,
        &["load", "--if-missing", auto_file.to_str().unwrap()],
        None,
    );
    assert!(out.status.success(), "load auto failed: {}", String::from_utf8_lossy(&out.stderr));

    // parity across the flag-bearing subjects
    for args in [
        vec!["get", "_dc", "flatten.app"],
        vec!["get", "auto", "seed"],
        vec!["get", "infra", "region"],
    ] {
        assert_eq!(fx.get_out(&a, &args), fx.get_out(&b, &args), "parity for {args:?}");
    }
}

#[test]
fn emit_dry_run_writes_nothing_and_never_prints_values() {
    let (fx, a) = Fixture::new("emit-dry-a");
    fx.seed_source_store(&a);

    let before = std::fs::read_dir(&a).unwrap().count();
    let out = fx.run_in(&a, &["emit", "--dry-run"], None);
    assert!(
        out.status.success(),
        "dc emit --dry-run failed:\nstderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dry-run:"), "{stdout}");
    assert!(stdout.contains("nothing written"), "{stdout}");
    // subject summaries count entries without printing them
    assert!(stdout.contains("app:"), "{stdout}");
    assert!(stdout.contains("1 encrypted token(s)"), "{stdout}");

    // no values — plaintext or token — anywhere in the output
    for leaked in ["round-trip-secret", "parked-plain", "🔒:v1:", "localhost", "us-east-1"] {
        assert!(!stdout.contains(leaked), "dry-run leaked `{leaked}`:\n{stdout}");
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("🔒:v1:"), "stderr leaked a token:\n{stderr}");

    // nothing written (only .gitignore + the pre-existing entry count)
    assert_eq!(
        std::fs::read_dir(&a).unwrap().count(),
        before,
        "--dry-run must not write files"
    );
    assert!(!a.join("secrets.dc.yaml").exists());
}

#[test]
fn emit_accepts_absolute_gitignored_output_without_force() {
    // W1 regression: the safety gate must resolve an absolute --output to a
    // repo-relative pathspec before asking git — an absolute path used to
    // hard-error ("outside repository") and force --force on a valid target.
    let (fx, a) = Fixture::new("emit-abs-a");
    fx.seed_source_store(&a);

    let abs_out = a.join("secrets.dc.abs.yaml");
    let out = fx.run_in(&a, &["emit", "--output", abs_out.to_str().unwrap()], None);
    assert!(
        out.status.success(),
        "absolute gitignored output must pass the gate without --force:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(abs_out.exists(), "absolute output must be written");
    // tracked absolute target is still refused (the gate, not the path form, decides)
    git(&a, &["add", "-f", "secrets.dc.abs.yaml"]);
    let out = fx.run_in(&a, &["emit", "--output", abs_out.to_str().unwrap()], None);
    assert!(
        !out.status.success(),
        "tracked absolute output must be refused without --force"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("tracked"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn emit_dry_run_previews_unignored_paths_without_force() {
    // N4 regression: --dry-run writes nothing, so it must return before the
    // gitignore safety gate and never demand --force just to show the plan.
    let (fx, a) = Fixture::new("emit-dry-b");
    fx.seed_source_store(&a);

    let out = fx.run_in(&a, &["emit", "--output", "out.yaml", "--dry-run"], None);
    assert!(
        out.status.success(),
        "dry-run of an unignored path must not demand --force:\nstderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dry-run:"), "{stdout}");
    assert!(stdout.contains("out.yaml"), "{stdout}");
    assert!(!a.join("out.yaml").exists(), "dry-run must not write the target");
}

#[test]
fn emit_refuses_committable_targets_unless_forced() {
    let (fx, a) = Fixture::new("emit-safe-a");
    fx.seed_source_store(&a);

    // not covered by .gitignore -> refused
    let out = fx.run_in(&a, &["emit", "--output", "out.yaml"], None);
    assert!(
        !out.status.success(),
        "unignored output must be refused:\nstdout: {}",
        String::from_utf8_lossy(&out.stdout),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(".gitignore"), "stderr:\n{stderr}");
    assert!(!a.join("out.yaml").exists(), "refused write must not exist");

    // --force overrides
    let out = fx.run_in(&a, &["emit", "--output", "out.yaml", "--force"], None);
    assert!(out.status.success(), "forced emit failed: {}", String::from_utf8_lossy(&out.stderr));
    assert!(a.join("out.yaml").exists());

    // git-tracked -> refused even when the pattern is ignored
    git(&a, &["add", "-f", "out.yaml"]);
    let out = fx.run_in(&a, &["emit", "--output", "out.yaml", "--force"], None);
    assert!(out.status.success(), "--force overrides tracking too");
    let out = fx.run_in(&a, &["emit", "--output", "out.yaml"], None);
    assert!(
        !out.status.success(),
        "tracked output must be refused without --force"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("tracked"), "stderr:\n{stderr}");

    // the default name is refused identically until ignored
    let (fx2, a2) = Fixture::new("emit-safe-b");
    fx2.seed_source_store(&a2);
    std::fs::remove_file(a2.join(".gitignore")).unwrap();
    let out = fx2.run_in(&a2, &["emit"], None);
    assert!(!out.status.success(), "default target must pass the same gate");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(".gitignore"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn emit_subject_filter_and_layer_only_outputs() {
    let (fx, a) = Fixture::new("emit-filter-a");
    fx.seed_source_store(&a);

    let out = fx.run_in(&a, &["emit", "--subject", "infra"], None);
    assert!(
        out.status.success(),
        "filtered emit failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let main = std::fs::read_to_string(a.join("secrets.dc.yaml")).unwrap();
    assert!(main.contains("infra:"), "{main}");
    assert!(
        !main.contains("app:"),
        "--subject infra must not emit app:\n{main}"
    );
    assert!(!a.join("secrets.dc.local.yaml").exists(), "no local layer for infra");

    // unknown subject is a hard error naming what exists
    let out = fx.run_in(&a, &["emit", "--subject", "nope"], None);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("nope") && stderr.contains("app"), "{stderr}");
}
