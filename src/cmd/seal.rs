//! `dc seal` — encryption inventory + in-place (un)seal over project scope.
//!
//! Scope: the `.envrc*` `dc_yaml` heredoc blocks and the registered external
//! (`dc load`) YAML files. Store layer files are never rewritten by seal —
//! values there are always encrypted at rest already.
//!
//! Per-entry states:
//!   - `🔒 t<N>` — an encrypted token at rest (tier from the token)
//!   - `plain`   — plaintext (bare, or a `🔒`-marked plaintext secret)
//!   - `⛔ shadow` — the restricted sentinel; listed, NEVER modified
//!
//! Values are never printed anywhere in seal output — key names, locations
//! and states only. Encrypt collapses a value to one quoted line carrying
//! `🔒<bangs> <token>` (reusing the `dc config set` splice machinery, so
//! surrounding comments/formatting survive and untouched lines re-emit
//! byte-identical). Decrypt restores `🔒<bangs> <plaintext>` with the
//! original marker bangs preserved — byte-exact for anything seal itself
//! encrypted from a `🔒`-marked original; bare originals come back as
//! `🔒 <plaintext>` (the canonical plaintext-at-rest secret form). The
//! decrypt→re-encrypt tier is the original marker tier when present, else
//! the token's own tier, else the repo's default tier 0.

use anyhow::{bail, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use crate::cmd::config::{
    compute_set_line, current_value_text, read_lines, resolve_forced_file, same_path, splice_set,
    write_lines_atomic,
};
use crate::envrc::locator::{self, KeyLoc};

// ---------------------------------------------------------------------------
// Inventory
// ---------------------------------------------------------------------------

/// At-rest state of one source entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Encrypted `🔒:v1` token (or legacy `dcenc:v1`); payload = token tier.
    Sealed(u8),
    /// Plaintext — bare value or `🔒`-marked plaintext.
    Plain,
    /// `⛔` restricted sentinel.
    Shadow,
}

impl State {
    pub fn label(&self) -> String {
        match self {
            State::Sealed(t) => format!("🔒 t{t}"),
            State::Plain => "plain".to_string(),
            State::Shadow => "⛔ shadow".to_string(),
        }
    }
}

/// One located, classified key in a project-scope source file.
#[derive(Debug, Clone)]
pub struct Entry {
    pub subject: String,
    pub path: String,
    pub file: PathBuf,
    /// 0-indexed line of the `key:` (display +1).
    pub key_line: usize,
    pub state: State,
    /// Full location (kept for splicing).
    pub loc: KeyLoc,
}

impl Entry {
    /// Selection identity: `subject.path`.
    pub fn key(&self) -> String {
        format!("{}.{}", self.subject, self.path)
    }
}

/// One scanned in-scope file with its indexed entries (entry.file names the
/// file; the scan groups work per-file).
pub struct FileScan {
    pub entries: Vec<Entry>,
}

/// Classify a raw (unquoted, block-joined) value text.
pub fn classify_value(raw: &str) -> State {
    if crate::secret::is_restricted_sentinel(raw) {
        return State::Shadow;
    }
    let token_text = crate::secret::parse_marker(raw)
        .map(|ps| ps.body)
        .unwrap_or_else(|| raw.to_string());
    if crate::crypto::is_encrypted(&token_text) {
        State::Sealed(crate::crypto::token_tier(&token_text).unwrap_or(0))
    } else {
        State::Plain
    }
}

/// Scan one file: heredoc blocks (`.envrc*`) or plain YAML (registered
/// external). Line-based, comment/formatting preserving.
pub fn scan_file(file: &Path, external: bool) -> Result<FileScan> {
    let content = std::fs::read_to_string(file)?;
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let locs = if external {
        locator::index_external_lines(&lines, file)
    } else {
        let mut out = Vec::new();
        for b in locator::scan_blocks(&lines) {
            out.extend(locator::index_block(&b, &lines, file));
        }
        out
    };
    let entries = locs
        .into_iter()
        .map(|loc| {
            let state = classify_value(&current_value_text(&lines, &loc));
            let key_line = loc.key_line;
            Entry {
                subject: loc.subject.clone(),
                path: loc.path.clone(),
                file: file.to_path_buf(),
                key_line,
                state,
                loc,
            }
        })
        .collect();
    Ok(FileScan { entries })
}

/// Full project-scope scan: `.envrc*` heredocs + registered external files.
/// Returns the scans and the store meta (for `--file` resolution).
pub fn scan_scope() -> Result<(Vec<FileScan>, crate::store::meta::StoreMeta)> {
    let store = crate::store::find_current_store()?;
    let meta = crate::store::meta::read_meta(&store)?;
    let mut scans = Vec::new();
    for f in locator::envrc_files(&meta.source) {
        match scan_file(&f, false) {
            Ok(s) => scans.push(s),
            Err(e) => eprintln!("warning: skipping {}: {e}", f.display()),
        }
    }
    for ef in &meta.external {
        if !ef.path.exists() {
            eprintln!(
                "warning: registered external file {} has vanished — skipping (re-run `dc load`; deregistration is not yet supported)",
                ef.path.display()
            );
            continue;
        }
        match scan_file(&ef.path, true) {
            Ok(s) => scans.push(s),
            Err(e) => eprintln!("warning: skipping {}: {e}", ef.path.display()),
        }
    }
    Ok((scans, meta))
}

/// All entries across scans, sorted by (key, file, line).
pub fn sorted_entries(scans: &[FileScan]) -> Vec<&Entry> {
    let mut all: Vec<&Entry> = scans.iter().flat_map(|s| s.entries.iter()).collect();
    all.sort_by(|a, b| {
        a.key()
            .cmp(&b.key())
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.key_line.cmp(&b.key_line))
    });
    all
}

fn print_table(entries: &[&Entry]) {
    let width = entries
        .iter()
        .map(|e| e.key().chars().count())
        .max()
        .unwrap_or(3)
        .clamp(3, 60);
    println!("{:<width$}  file:line  state", "key", width = width);
    for e in entries {
        println!(
            "{:<width$}  {}:{}  {}",
            e.key(),
            e.file.display(),
            e.key_line + 1,
            e.state.label(),
            width = width
        );
    }
}

fn print_json(entries: &[&Entry]) -> Result<()> {
    let arr: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            let (state, tier) = match &e.state {
                State::Sealed(t) => ("sealed", Some(*t)),
                State::Plain => ("plain", None),
                State::Shadow => ("shadow", None),
            };
            serde_json::json!({
                "key": e.key(),
                "file": e.file.display().to_string(),
                "line": e.key_line + 1,
                "state": state,
                "tier": tier,
            })
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&arr)?);
    Ok(())
}

// ---------------------------------------------------------------------------
// Planning + application
// ---------------------------------------------------------------------------

/// One validated, pending splice.
#[derive(Debug)]
struct PlanOp {
    subject: String,
    key: String,
    file: PathBuf,
    line_no: usize,
    encrypting: bool,
    from: String,
    to: String,
    loc: KeyLoc,
    new_line: String,
}

/// Validate and compute the splice for every target. All-or-nothing: any
/// failure aborts before a single file is written.
///
/// `explicit` selections (headless `--encrypt KEY` / `--decrypt KEY`) refuse
/// ⛔ shadow keys outright; bulk selections never contain them, but skip
/// defensively. Entries in the wrong state for the action warn + skip.
fn build_plan(
    encrypting: bool,
    explicit: bool,
    targets: &[&Entry],
    key: &[u8; 32],
) -> Result<Vec<PlanOp>> {
    let mut ops = Vec::new();
    for e in targets {
        if e.state == State::Shadow {
            if explicit {
                bail!(
                    "`{}` is a ⛔ shadowed key — seal never modifies shadowed entries",
                    e.key()
                );
            }
            continue;
        }
        let eligible = matches!(
            (&e.state, encrypting),
            (State::Plain, true) | (State::Sealed(_), false)
        );
        if !eligible {
            eprintln!(
                "warning: skipping {} — {}",
                e.key(),
                if encrypting {
                    "already sealed"
                } else {
                    "already plain"
                }
            );
            continue;
        }
        let lines = read_lines(&e.file)?;
        let vt = current_value_text(&lines, &e.loc);
        let (new_line, from, to) = if encrypting {
            let ps = crate::secret::parse_marker(&vt);
            let tier = ps.as_ref().map(|p| p.tier).unwrap_or(0);
            let body = ps.map(|p| p.body).unwrap_or_else(|| vt.clone());
            let token = crate::crypto::encode_token(&body, tier, key)?;
            let to = format!("🔒 t{tier}");
            (
                compute_set_line(&e.loc, tier, &token),
                "plain".to_string(),
                to,
            )
        } else {
            let ps = crate::secret::parse_marker(&vt);
            let (marker_tier, token) = match ps {
                Some(p) => (Some(p.tier), p.body),
                None => (None, vt.trim().to_string()),
            };
            let (tok_tier, plaintext) = crate::crypto::decode_token(&token, key)?;
            // Restore tier: original marker bangs when present, else the
            // token's own tier, else the repo's default (0).
            let restore_tier = marker_tier.unwrap_or(tok_tier);
            let new_line = compute_set_line(&e.loc, restore_tier, &plaintext);
            (new_line, format!("🔒 t{tok_tier}"), "plain".to_string())
        };
        ops.push(PlanOp {
            subject: e.subject.clone(),
            key: e.key(),
            file: e.file.clone(),
            line_no: e.key_line + 1,
            encrypting,
            from,
            to,
            loc: e.loc.clone(),
            new_line,
        });
    }
    Ok(ops)
}

/// Print the planned changes (key names + locations + state transitions).
fn print_plan(ops: &[PlanOp], dry_run: bool) {
    for op in ops {
        let verb = if op.encrypting { "seal" } else { "unseal" };
        let prefix = if dry_run { "would " } else { "" };
        eprintln!(
            "{prefix}{verb} {} at {}:{} ({} → {})",
            op.key,
            op.file.display(),
            op.line_no,
            op.from,
            op.to
        );
    }
}

/// Summary of an applied (or previewed) plan, for the `--json` action mode.
struct ApplyOutcome {
    applied: usize,
    dry_run: bool,
}

/// Apply a validated plan: atomic per-file writes (all-or-nothing per run —
/// [`build_plan`] has already validated every op), then a store refresh per
/// touched subject (the `dc config set` precedent, so `dc get` serves the
/// sealed/unsealed value immediately).
///
/// With `json`, the human progress lines move to stderr so stdout carries the
/// JSON summary only.
fn apply_plan(ops: &[PlanOp], dry_run: bool, yes: bool, json: bool) -> Result<ApplyOutcome> {
    let human = |text: &str| {
        if json {
            eprintln!("{text}");
        } else {
            println!("{text}");
        }
    };
    if ops.is_empty() {
        human("nothing to do — no eligible entries in scope");
        return Ok(ApplyOutcome { applied: 0, dry_run });
    }
    print_plan(ops, dry_run);
    if dry_run {
        human(&format!("dry-run: {} change(s) — nothing written", ops.len()));
        return Ok(ApplyOutcome { applied: ops.len(), dry_run: true });
    }
    if !yes {
        bail!("nothing written — re-run with --yes to apply (or --dry-run to preview)");
    }

    let mut by_file: BTreeMap<PathBuf, Vec<&PlanOp>> = BTreeMap::new();
    for op in ops {
        by_file.entry(op.file.clone()).or_default().push(op);
    }
    for (file, file_ops) in &by_file {
        let mut lines = read_lines(file)?;
        // Descending key_line: splices never invalidate not-yet-processed indices.
        let mut ordered: Vec<&&PlanOp> = file_ops.iter().collect();
        ordered.sort_by_key(|op| std::cmp::Reverse(op.loc.key_line));
        for op in ordered {
            lines = splice_set(&lines, &op.loc, op.new_line.clone());
        }
        write_lines_atomic(file, &lines)?;
        eprintln!("wrote {}", file.display());
    }

    // Refresh affected subjects so the store picks up the new source values.
    // Every touched file is refreshed — one refresh per (subject, file) pair,
    // in op order — so a subject spliced across several files re-ingests from
    // each of them and the last-touched file really does ingest last (its keys
    // win). A single file per subject would let the other files' keys serve
    // stale store values until a manual reload.
    let mut touched: Vec<(String, PathBuf)> = Vec::new();
    // Membership via BTreeSet keeps the dedup O(n log n) (Vec::contains was
    // O(n^2)); the Vec side preserves the op-order iteration the refresh
    // contract below depends on.
    let mut seen: BTreeSet<(String, PathBuf)> = BTreeSet::new();
    for op in ops {
        let pair = (op.subject.clone(), op.file.clone());
        if seen.insert(pair.clone()) {
            touched.push(pair);
        }
    }
    for (subject, file) in &touched {
        crate::cmd::config::refresh_store(subject, Some(file))?;
    }
    Ok(ApplyOutcome { applied: ops.len(), dry_run: false })
}

// ---------------------------------------------------------------------------
// Selection (headless + interactive share it)
// ---------------------------------------------------------------------------

/// Resolve explicit `subject.path` keys to inventory entries, erroring on
/// unknown keys (scripts must learn, not silently no-op).
fn resolve_targets<'a>(entries: &[&'a Entry], keys: &[String]) -> Result<Vec<&'a Entry>> {
    let mut out = Vec::new();
    let mut missing = Vec::new();
    for k in keys {
        let hits: Vec<&Entry> = entries.iter().copied().filter(|e| e.key() == *k).collect();
        if hits.is_empty() {
            missing.push(k);
        } else {
            out.extend(hits);
        }
    }
    if !missing.is_empty() {
        let missing: Vec<String> = missing.iter().map(|s| s.to_string()).collect();
        bail!(
            "unknown key(s): {} — run `dc seal --list` for the inventory",
            missing.join(", ")
        );
    }
    Ok(out)
}

/// Parse an interactive selection spec (`1,3-5`, `all`, `none`) into 0-based
/// indices. Pure; ranges are inclusive and order-agnostic.
pub fn parse_selection(spec: &str, len: usize) -> Result<BTreeSet<usize>> {
    let spec = spec.trim();
    if spec.is_empty() {
        bail!("empty selection");
    }
    let lower = spec.to_lowercase();
    if lower == "all" {
        return Ok((0..len).collect());
    }
    if lower == "none" {
        return Ok(BTreeSet::new());
    }
    let mut out = BTreeSet::new();
    for part in lower
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|p| !p.is_empty())
    {
        if let Some((a, b)) = part.split_once('-') {
            let a: usize = a.trim().parse().map_err(|_| {
                anyhow::anyhow!("bad selection token `{part}` (expected e.g. 1,3-5)")
            })?;
            let b: usize = b.trim().parse().map_err(|_| {
                anyhow::anyhow!("bad selection token `{part}` (expected e.g. 1,3-5)")
            })?;
            if a == 0 || b == 0 || a > len || b > len {
                bail!("selection out of range 1-{len}: `{part}`");
            }
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            out.extend(lo - 1..hi);
        } else {
            let n: usize = part.parse().map_err(|_| {
                anyhow::anyhow!("bad selection token `{part}` (expected e.g. 1,3-5)")
            })?;
            if n == 0 || n > len {
                bail!("selection out of range 1-{len}: `{part}`");
            }
            out.insert(n - 1);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Interactive (TUI) mode — dependency-free, reads /dev/tty
// ---------------------------------------------------------------------------

struct Tty {
    r: std::io::BufReader<std::fs::File>,
    w: std::io::BufWriter<std::fs::File>,
}

impl Tty {
    fn open() -> Result<Self> {
        let tty = std::fs::OpenOptions::new().read(true).write(true).open("/dev/tty")
            .map_err(|e| anyhow::anyhow!("cannot open /dev/tty for the interactive selector ({e}) — use --headless --encrypt/--decrypt <KEY>…, or --list"))?;
        let r = std::io::BufReader::new(tty.try_clone()?);
        let w = std::io::BufWriter::new(tty);
        Ok(Tty { r, w })
    }

    fn prompt(&mut self, text: &str) -> Result<String> {
        write!(self.w, "{text}")?;
        self.w.flush()?;
        let mut line = String::new();
        let n = self.r.read_line(&mut line)?;
        if n == 0 {
            bail!("aborted (eof on /dev/tty)");
        }
        Ok(line.trim().to_string())
    }
}

fn interactive(entries: &[&Entry]) -> Result<()> {
    if !std::io::stdin().is_terminal() {
        bail!(
            "seal's interactive selector needs a terminal — re-run with --headless --encrypt/--decrypt <KEY>…, or --list for the inventory"
        );
    }
    let mut tty = Tty::open()?;

    writeln!(tty.w, "dc seal — in-scope keys ({} entries)", entries.len())?;
    writeln!(tty.w)?;
    for (i, e) in entries.iter().enumerate() {
        writeln!(
            tty.w,
            "  {:>3}) {}  {}:{}  {}",
            i + 1,
            e.key(),
            e.file.display(),
            e.key_line + 1,
            e.state.label()
        )?;
    }
    writeln!(tty.w)?;

    let sel = loop {
        let line = tty.prompt("select keys (e.g. 1,3-5 | all | none): ")?;
        match parse_selection(&line, entries.len()) {
            Ok(s) => break s,
            Err(e) => writeln!(tty.w, "  {e}")?,
        }
    };
    if sel.is_empty() {
        writeln!(tty.w, "nothing selected — no changes written")?;
        return Ok(());
    }

    let encrypting = loop {
        let line = tty.prompt("action — (e)ncrypt or (d)ecrypt? ")?;
        match line.to_lowercase().as_str() {
            "e" | "encrypt" => break true,
            "d" | "decrypt" => break false,
            _ => writeln!(tty.w, "  answer e or d")?,
        }
    };

    // Preview: key NAMES only — never values.
    let names: Vec<String> = sel.iter().map(|&i| entries[i].key()).collect();
    writeln!(
        tty.w,
        "{} {} key(s): {}",
        if encrypting { "encrypt" } else { "decrypt" },
        sel.len(),
        names.join(", ")
    )?;

    let line = tty.prompt("apply? [y/N] ")?;
    if !matches!(line.to_lowercase().as_str(), "y" | "yes") {
        writeln!(tty.w, "aborted — no changes written")?;
        return Ok(());
    }

    let targets: Vec<&Entry> = sel.iter().map(|&i| entries[i]).collect();
    let key = crate::settings::key()?;
    let ops = build_plan(encrypting, false, &targets, &key)?;
    apply_plan(&ops, false, true, false)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Headless/interactive arguments (mirrors the clap definition in main.rs).
pub struct SealArgs<'a> {
    pub list: bool,
    pub json: bool,
    pub headless: bool,
    pub encrypt: Vec<String>,
    pub encrypt_all_plain: bool,
    pub decrypt: Vec<String>,
    pub decrypt_all: bool,
    pub file: Option<&'a str>,
    pub dry_run: bool,
    pub yes: bool,
}

// <REMOVED UUID HERE> run :: auto-generated pointer for public function run
pub fn run(args: SealArgs) -> Result<()> {
    let (scans, _meta) = scan_scope()?;
    let forced = match args.file {
        Some(f) => Some(resolve_forced_file(f)?),
        None => None,
    };
    let entries: Vec<&Entry> = sorted_entries(&scans)
        .into_iter()
        .filter(|e| forced.as_ref().is_none_or(|t| same_path(&e.file, t)))
        .collect();

    // Inventory/list mode.
    if args.list {
        if args.encrypt_all_plain
            || args.decrypt_all
            || !args.encrypt.is_empty()
            || !args.decrypt.is_empty()
        {
            bail!(
                "choose either --list (inventory only) or an --encrypt/--decrypt action, not both"
            );
        }
        if args.json {
            print_json(&entries)?;
        } else if entries.is_empty() {
            println!("no sealable keys in scope");
        } else {
            print_table(&entries);
        }
        return Ok(());
    }

    // Exactly-one action validation (shared by headless and explicit-TUI skip).
    let actions = [
        (!args.encrypt.is_empty(), "--encrypt"),
        (args.encrypt_all_plain, "--encrypt-all-plain"),
        (!args.decrypt.is_empty(), "--decrypt"),
        (args.decrypt_all, "--decrypt-all"),
    ];
    let chosen: Vec<&str> = actions
        .iter()
        .filter(|(b, _)| *b)
        .map(|(_, n)| *n)
        .collect();
    if chosen.len() > 1 {
        bail!("choose only one of --encrypt / --encrypt-all-plain / --decrypt / --decrypt-all (got: {})", chosen.join(", "));
    }

    if !args.headless {
        if !chosen.is_empty() {
            bail!(
                "{} require --headless (interactive mode is the default)",
                chosen.join(", ")
            );
        }
        return interactive(&entries);
    }

    // Headless.
    let encrypting = if !args.encrypt.is_empty() || args.encrypt_all_plain {
        true
    } else if !args.decrypt.is_empty() || args.decrypt_all {
        false
    } else {
        bail!("--headless needs an action: --encrypt <KEY>… | --encrypt-all-plain | --decrypt <KEY>… | --decrypt-all");
    };
    let explicit = !args.encrypt.is_empty() || !args.decrypt.is_empty();
    let targets: Vec<&Entry> = if explicit {
        let keys = if encrypting {
            &args.encrypt
        } else {
            &args.decrypt
        };
        resolve_targets(&entries, keys)?
    } else {
        // Bulk: only the action's own state is eligible; the other states are
        // definitionally out of scope for the bulk pass.
        entries
            .iter()
            .copied()
            .filter(|e| {
                matches!(
                    (&e.state, encrypting),
                    (State::Plain, true) | (State::Sealed(_), false)
                )
            })
            .collect()
    };

    let key = crate::settings::key()?;
    let ops = build_plan(encrypting, explicit, &targets, &key)?;
    let outcome = apply_plan(&ops, args.dry_run, args.yes, args.json)?;
    if args.json {
        // action-mode --json: a result summary (the --list inventory already
        // printed above returns its own JSON shape)
        println!(
            "{}",
            serde_json::json!({
                "action": if encrypting { "encrypt" } else { "decrypt" },
                "applied": outcome.applied,
                "dry_run": outcome.dry_run,
            })
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_tmp(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let p = dir.path().join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    const KEY: [u8; 32] = [5u8; 32];

    // ---- classification ----

    #[test]
    fn classify_distinguishes_all_three_states() {
        let tok = crate::crypto::encode_token("v", 2, &KEY).unwrap();
        assert_eq!(classify_value(&format!("🔒❗❗ {tok}")), State::Sealed(2));
        // legacy prefix still classifies as sealed
        assert_eq!(classify_value("dcenc:v1:t0:abc"), State::Sealed(0));
        assert_eq!(classify_value("🔒 plain-at-rest"), State::Plain);
        assert_eq!(classify_value("bare-value"), State::Plain);
        assert_eq!(classify_value("⛔"), State::Shadow);
    }

    #[test]
    fn scan_file_inventories_heredoc_states() {
        let tmp = TempDir::new().unwrap();
        let tok = crate::crypto::encode_token("vaulted", 1, &KEY).unwrap();
        let f = write_tmp(
            &tmp,
            ".envrc",
            &format!(
                "dc_yaml --no-bump app <<'YAML'\nhost: localhost\napi_key: \"🔒 presecret\"\nvault: \"🔒❗ {tok}\"\nlocked: \"⛔\"\nYAML\n"
            ),
        );
        let scan = scan_file(&f, false).unwrap();
        let by_path = |p: &str| {
            scan.entries
                .iter()
                .find(|e| e.path == p)
                .unwrap()
                .state
                .clone()
        };
        assert_eq!(by_path("host"), State::Plain);
        assert_eq!(by_path("api_key"), State::Plain);
        assert_eq!(by_path("vault"), State::Sealed(1));
        assert_eq!(by_path("locked"), State::Shadow);
        assert_eq!(scan.entries[0].subject, "app");
        assert_eq!(scan.entries[0].key(), "app.host");
    }

    // ---- splice round-trips ----

    fn entry_for<'a>(scan: &'a FileScan, path: &str) -> &'a Entry {
        scan.entries.iter().find(|e| e.path == path).unwrap()
    }

    #[test]
    fn encrypt_then_decrypt_restores_the_original_line() {
        let tmp = TempDir::new().unwrap();
        let f = write_tmp(
            &tmp,
            ".envrc",
            "dc_yaml --no-bump app <<'YAML'\n# keep\n  api_key: \"🔒 presecret\"\nzone: z\nYAML\n",
        );
        let scan = scan_file(&f, false).unwrap();
        let original = std::fs::read_to_string(&f).unwrap();

        // encrypt
        let targets = vec![entry_for(&scan, "api_key")];
        let ops = build_plan(true, true, &targets, &KEY).unwrap();
        assert_eq!(ops.len(), 1);
        assert!(ops[0].to.starts_with("🔒 t"));
        let sealed_lines = splice_set(
            &read_lines(&f).unwrap(),
            &ops[0].loc,
            ops[0].new_line.clone(),
        );
        std::fs::write(&f, sealed_lines.join("\n") + "\n").unwrap();

        // the sealed line carries a token; plaintext is gone from disk
        let sealed_src = std::fs::read_to_string(&f).unwrap();
        assert!(!sealed_src.contains("presecret"));
        assert!(sealed_src.contains("🔒:v1:t0:"));
        // comments and siblings untouched
        assert!(sealed_src.contains("# keep"));
        assert!(sealed_src.contains("zone: z"));

        // decrypt round-trip
        let scan2 = scan_file(&f, false).unwrap();
        let targets2 = vec![entry_for(&scan2, "api_key")];
        let ops2 = build_plan(false, true, &targets2, &KEY).unwrap();
        assert_eq!(ops2.len(), 1);
        let restored_lines = splice_set(
            &read_lines(&f).unwrap(),
            &ops2[0].loc,
            ops2[0].new_line.clone(),
        );
        std::fs::write(&f, restored_lines.join("\n") + "\n").unwrap();
        assert_eq!(
            std::fs::read_to_string(&f).unwrap(),
            original,
            "decrypt must restore the original bytes"
        );
    }

    #[test]
    fn decrypt_preserves_marker_tier_for_reseal() {
        let tmp = TempDir::new().unwrap();
        let tok = crate::crypto::encode_token("body", 2, &KEY).unwrap();
        let f = write_tmp(
            &tmp,
            ".envrc",
            &format!("dc_yaml --no-bump app <<'YAML'\nsecret: \"🔒❗❗ {tok}\"\nYAML\n"),
        );
        let scan = scan_file(&f, false).unwrap();
        assert_eq!(entry_for(&scan, "secret").state, State::Sealed(2));
        let targets = vec![entry_for(&scan, "secret")];
        let ops = build_plan(false, true, &targets, &KEY).unwrap();
        // restored line keeps the ❗❗ marker (byte-exact round-trip form)
        assert!(
            ops[0].new_line.contains("🔒❗❗ body"),
            "{}",
            ops[0].new_line
        );

        // Re-sealing the restored line must reuse the original tier 2.
        let restored = format!(
            "dc_yaml --no-bump app <<'YAML'\n{}\nYAML\n",
            ops[0].new_line.trim()
        );
        let rf = write_tmp(&tmp, ".envrc2", &restored);
        let rs = scan_file(&rf, false).unwrap();
        assert_eq!(
            entry_for(&rs, "secret").state,
            State::Plain,
            "restored line is plaintext-at-rest"
        );
        let reseal = vec![entry_for(&rs, "secret")];
        let ops2 = build_plan(true, true, &reseal, &KEY).unwrap();
        assert_eq!(
            ops2[0].to, "🔒 t2",
            "re-encrypt must preserve the original tier"
        );
    }

    #[test]
    fn build_plan_skips_wrong_state_and_refuses_shadow() {
        let tmp = TempDir::new().unwrap();
        let tok = crate::crypto::encode_token("v", 0, &KEY).unwrap();
        let f = write_tmp(
            &tmp,
            ".envrc",
            &format!(
                "dc_yaml --no-bump app <<'YAML'\nsealed: \"🔒 {tok}\"\nlocked: \"⛔\"\nYAML\n"
            ),
        );
        let scan = scan_file(&f, false).unwrap();
        // explicit encrypt of an already-sealed key warns + skips (no hard error —
        // multi-location keys can legitimately hold mixed states)
        let sealed = vec![entry_for(&scan, "sealed")];
        let ops = build_plan(true, true, &sealed, &KEY).unwrap();
        assert!(ops.is_empty());
        // explicit selection of a ⛔ shadowed key is a hard refusal — never modified
        let shadow = vec![entry_for(&scan, "locked")];
        let err = build_plan(true, true, &shadow, &KEY)
            .unwrap_err()
            .to_string();
        assert!(err.contains("⛔"), "{err}");
        let err = build_plan(false, true, &shadow, &KEY)
            .unwrap_err()
            .to_string();
        assert!(err.contains("⛔"), "{err}");
    }

    // ---- selection parsing ----

    #[test]
    fn parse_selection_singles_ranges_all_none() {
        assert_eq!(parse_selection("3", 5).unwrap(), BTreeSet::from([2]));
        assert_eq!(
            parse_selection("1,3-5", 5).unwrap(),
            BTreeSet::from([0, 2, 3, 4])
        );
        assert_eq!(
            parse_selection("5-3", 5).unwrap(),
            BTreeSet::from([2, 3, 4])
        );
        assert_eq!(
            parse_selection("all", 3).unwrap(),
            BTreeSet::from([0, 1, 2])
        );
        assert_eq!(parse_selection("none", 3).unwrap(), BTreeSet::new());
        assert_eq!(parse_selection(" 2 4 ", 5).unwrap(), BTreeSet::from([1, 3]));
        assert!(parse_selection("", 3).is_err());
        assert!(parse_selection("0", 3).is_err());
        assert!(parse_selection("4", 3).is_err());
        assert!(parse_selection("1,x", 3).is_err());
        assert!(parse_selection("1-", 3).is_err());
    }

    // ---- target resolution ----

    #[test]
    fn resolve_targets_errors_on_unknown_keys() {
        let tmp = TempDir::new().unwrap();
        let f = write_tmp(
            &tmp,
            ".envrc",
            "dc_yaml --no-bump app <<'YAML'\nhost: h\nYAML\n",
        );
        let scan = scan_file(&f, false).unwrap();
        let all = sorted_entries(std::slice::from_ref(&scan));
        let got = resolve_targets(&all, &["app.host".to_string()]).unwrap();
        assert_eq!(got.len(), 1);
        let err = resolve_targets(&all, &["nope.missing".to_string()])
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("nope.missing") && err.contains("--list"),
            "{err}"
        );
    }
}
