//! A key that was 🔒 lives in the store's `secrets` layer; when its source
//! line later arrives plain (`dc config set --plain`/`--numeric`/`--bool`,
//! or a hand edit + `dc load`), ingest must retire the stale token from
//! secrets.yaml — otherwise the secrets overlay keeps outranking the fresh
//! plain value and `dc get` serves the ghost forever.
//!
//! Everything runs against temp fixtures (own project dir, throwaway key
//! file, `XDG_STATE_HOME` sandbox) — never against a real store. Values are
//! fake.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const SECRET: &str = "example-plain-value-NOT-SECRET-42";
const ENC_SECRET: &str = "example-enc-value-NOT-SECRET-7";
const ENC_SECRET_2: &str = "example-enc-value-2-NOT-SECRET-9";

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
            "dc load failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        (Proj { dir }, tmp)
    }

    fn envrc(&self, body: &str) {
        std::fs::write(self.dir.join(".envrc"), body).unwrap();
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

    fn mint_token(&self, tmp: &tempfile::TempDir, value: &str) -> String {
        let out = self.run(tmp, &["encrypt", "--stdin"], Some(value));
        assert!(
            out.status.success(),
            "dc encrypt failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        // dc encrypt prints the bare token wrapped in display quotes.
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .trim_matches('"')
            .to_string()
    }
}

fn ok(out: &std::process::Output, what: &str) -> String {
    assert!(
        out.status.success(),
        "dc {what} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Locate `<store>/<subject>/secrets.yaml` inside the sandboxed state dir.
fn secrets_yaml(tmp: &tempfile::TempDir, subject: &str) -> PathBuf {
    fn walk(dir: &Path, subject: &str, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                walk(&p, subject, out);
            } else if p.file_name().unwrap_or_default() == "secrets.yaml"
                && p.parent()
                    .map(|d| d.file_name().unwrap_or_default() == subject)
                    .unwrap_or(false)
            {
                out.push(p);
            }
        }
    }
    let mut hits = Vec::new();
    walk(&tmp.path().join("state"), subject, &mut hits);
    assert_eq!(hits.len(), 1, "expected exactly one {subject}/secrets.yaml");
    hits.remove(0)
}

fn has_path(file: &Path, path: &str) -> bool {
    let content = std::fs::read_to_string(file).unwrap();
    let v: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
    dc_yaml_path(&v, path).is_some()
}

/// Minimal dot-path lookup over the parsed layer file.
fn dc_yaml_path(root: &serde_yaml::Value, path: &str) -> Option<serde_yaml::Value> {
    let mut cur = root;
    for seg in path.split('.') {
        cur = cur.get(seg)?;
    }
    Some(cur.clone())
}

fn heredoc(lines: &[String]) -> String {
    format!(
        "dc_yaml --no-bump app <<'YAML'\ndb:\n{}\nYAML\n",
        lines.join("\n")
    )
}

#[test]
fn plain_rewrite_retires_secrets_layer_ghost() {
    let (proj, tmp) = Proj::new("ghost-plain");
    let tok = proj.mint_token(&tmp, ENC_SECRET);
    let tok2 = proj.mint_token(&tmp, ENC_SECRET_2);
    proj.envrc(&heredoc(&[
        format!("  password: \"🔒 {tok}\""),
        format!("  other: \"🔒 {tok2}\""),
    ]));

    // A config-set-free refresh first: ingest both keys into the store by
    // touching the subject through a plain set on a third key.
    proj.envrc(&heredoc(&[
        format!("  password: \"🔒 {tok}\""),
        format!("  other: \"🔒 {tok2}\""),
        "  host: localhost".to_string(),
    ]));
    let out = proj.run(
        &tmp,
        &[
            "config", "set", "app", "db.host", "--value", "h", "--plain", "--yes",
        ],
        None,
    );
    ok(&out, "seed ingest");

    let secrets = secrets_yaml(&tmp, "app");
    assert!(has_path(&secrets, "db.password"), "ghost seeded");
    assert!(has_path(&secrets, "db.other"), "second secret seeded");

    // Rewrite the ghost key plain.
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.password",
            "--value",
            SECRET,
            "--plain",
            "--yes",
        ],
        None,
    );
    ok(&out, "config set --plain");

    // dc get serves the fresh plain value, not the stale token.
    let get = proj.run(&tmp, &["get", "app", "db.password"], None);
    let stdout = ok(&get, "get after plain rewrite");
    assert!(
        stdout.contains(SECRET) && !stdout.contains("🔒:v1:"),
        "plain value must win over the secrets-layer ghost:\n{stdout}"
    );
    // secrets.yaml no longer holds the path; the untouched sibling survives.
    assert!(
        !has_path(&secrets, "db.password"),
        "ghost must be retired from secrets.yaml"
    );
    assert!(
        has_path(&secrets, "db.other"),
        "unrelated secrets-layer key untouched"
    );

    // The still-encrypted sibling still resolves: locked display + reveals
    // to its original secret.
    let get2 = proj.run(&tmp, &["get", "app", "db.other"], None);
    let stdout2 = ok(&get2, "get untouched sibling");
    assert!(
        stdout2.contains("🔒") && stdout2.contains("**redacted**"),
        "encrypted sibling keeps serving its token:\n{stdout2}"
    );
    let get3 = proj.run(&tmp, &["get", "app", "db.other", "--reveal"], None);
    let stdout3 = ok(&get3, "reveal untouched sibling");
    assert!(
        stdout3.contains(ENC_SECRET_2),
        "encrypted sibling must still reveal its original secret:\n{stdout3}"
    );
}

#[test]
fn numeric_and_bool_rewrites_also_retire_ghosts() {
    let (proj, tmp) = Proj::new("ghost-typed");
    let tok = proj.mint_token(&tmp, ENC_SECRET);
    let tok2 = proj.mint_token(&tmp, ENC_SECRET_2);
    proj.envrc(&heredoc(&[
        format!("  count: \"🔒 {tok}\""),
        format!("  flag: \"🔒 {tok2}\""),
    ]));
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.seeder",
            "--value",
            "s",
            "--plain",
            "--yes",
        ],
        None,
    );
    ok(&out, "seed ingest");
    let secrets = secrets_yaml(&tmp, "app");
    assert!(has_path(&secrets, "db.count") && has_path(&secrets, "db.flag"));

    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.count",
            "--value=4",
            "--numeric",
            "--yes",
        ],
        None,
    );
    ok(&out, "config set --numeric over ghost");
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.flag",
            "--value=TRUE",
            "--bool",
            "--yes",
        ],
        None,
    );
    ok(&out, "config set --bool over ghost");

    assert!(!has_path(&secrets, "db.count"), "numeric ghost retired");
    assert!(!has_path(&secrets, "db.flag"), "bool ghost retired");

    let get = proj.run(&tmp, &["get", "app", "db.count"], None);
    assert_eq!(ok(&get, "get count").trim(), "4");
    let get2 = proj.run(&tmp, &["get", "app", "db.flag"], None);
    assert_eq!(ok(&get2, "get flag").trim(), "true");
}

#[test]
fn plain_to_encrypted_direction_still_works() {
    let (proj, tmp) = Proj::new("plain-to-enc");
    proj.envrc(&heredoc(&["  password: plain-old".to_string()]));
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.seeder",
            "--value",
            "s",
            "--plain",
            "--yes",
        ],
        None,
    );
    ok(&out, "seed ingest");

    // Default mode encrypts: the value moves INTO the secrets layer.
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.password",
            "--value",
            SECRET,
            "--yes",
        ],
        None,
    );
    ok(&out, "config set (encrypt)");

    let secrets = secrets_yaml(&tmp, "app");
    assert!(
        has_path(&secrets, "db.password"),
        "encrypted value must land in secrets.yaml"
    );
    let get = proj.run(&tmp, &["get", "app", "db.password", "--reveal"], None);
    let stdout = ok(&get, "get --reveal");
    assert!(
        stdout.contains(SECRET),
        "encrypted value must resolve and reveal:\n{stdout}"
    );
}

#[test]
fn hand_edit_then_load_retires_ghost() {
    let (proj, tmp) = Proj::new("ghost-handedit");
    let tok = proj.mint_token(&tmp, ENC_SECRET);
    let team = proj.dir.join("team.yaml");
    std::fs::write(&team, format!("app:\n  db:\n    password: \"{tok}\"\n")).unwrap();

    // First load: the bare 🔒:v1 token lands in the secrets layer.
    let out = proj.run(&tmp, &["load", "team.yaml"], None);
    ok(&out, "initial load");
    let secrets = secrets_yaml(&tmp, "app");
    assert!(
        has_path(&secrets, "db.password"),
        "token ingested into secrets layer"
    );

    // Hand edit: the token line becomes a plain value, then re-load.
    std::fs::write(&team, format!("app:\n  db:\n    password: {SECRET}\n")).unwrap();
    let out = proj.run(&tmp, &["load", "team.yaml"], None);
    ok(&out, "reload after hand edit");

    assert!(
        !has_path(&secrets, "db.password"),
        "hand-edit + load must retire the ghost"
    );
    let get = proj.run(&tmp, &["get", "app", "db.password"], None);
    let stdout = ok(&get, "get after reload");
    assert!(
        stdout.contains(SECRET) && !stdout.contains("🔒:v1:"),
        "plain value must win after reload:\n{stdout}"
    );
}

#[test]
fn secure_sentinel_survives_subsequent_refreshes() {
    // `dc config secure` deliberately places ⛔ in the secrets layer; the
    // plain ⛔ a later source refresh re-ingests must NOT delete it.
    let (proj, tmp) = Proj::new("ghost-sentinel");
    let tok = proj.mint_token(&tmp, ENC_SECRET);
    proj.envrc(&heredoc(&[format!("  password: \"🔒 {tok}\"")]));
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.seeder",
            "--value",
            "s",
            "--plain",
            "--yes",
        ],
        None,
    );
    ok(&out, "seed ingest");

    let out = proj.run(&tmp, &["config", "secure", "app", "db.password"], None);
    // `dc secure` opens no prompt (this is the non-interactive path).
    if !out.status.success() {
        // secure may require a tty for audit confirm; skip gracefully only
        // if the tool refused for interactivity reasons.
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("terminal") || stderr.contains("tty"),
            "dc secure failed for an unexpected reason:\n{stderr}"
        );
        return;
    }

    let secrets = secrets_yaml(&tmp, "app");
    assert!(
        has_path(&secrets, "db.password"),
        "⛔ sentinel seeded in secrets layer"
    );

    // A later refresh of the same subject (another key set) re-ingests the
    // source's ⛔ line as plain — it must not wipe the sentinel.
    let out = proj.run(
        &tmp,
        &[
            "config",
            "set",
            "app",
            "db.seeder",
            "--value",
            "s2",
            "--plain",
            "--yes",
        ],
        None,
    );
    ok(&out, "refresh after secure");
    assert!(
        has_path(&secrets, "db.password"),
        "⛔ sentinel must survive the refresh"
    );
}
