//! `dc config get/set/setall` — locate and edit secrets in source `.envrc*` files.
//!
//! These commands operate on the literal hand-authored heredoc files (not the
//! state store), encrypting secret values at rest inline as `🔒 🔒:…` and
//! preserving all surrounding formatting/comments via line splicing.

use anyhow::{bail, Result};
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use crate::envrc::locator::{self, KeyLoc};
use crate::secret::PADLOCK;

fn read_lines(path: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.lines().map(|s| s.to_string()).collect())
}

fn write_lines(path: &Path, lines: &[String]) -> Result<()> {
    let mut body = lines.join("\n");
    body.push('\n');
    std::fs::write(path, body)?;
    Ok(())
}

fn marker(tier: u8) -> String {
    let bangs: String = std::iter::repeat_n('❗', tier.min(3) as usize).collect();
    format!("{PADLOCK}{bangs}")
}

fn leaf_key(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

/// Find all source locations for a subject/path across `.envrc*` files and
/// registered external (`dc load`) files.
fn find_locs(subject: &str, path: &str) -> Result<Vec<KeyLoc>> {
    let store = crate::store::find_current_store()?;
    let meta = crate::store::meta::read_meta(&store)?;
    Ok(find_locs_in(&meta.source, &meta.external, subject, path))
}

/// Pure lookup over a source dir plus a registry of external files. Vanished
/// external files warn (staleness) but do not fail the lookup.
fn find_locs_in(
    dir: &Path,
    external: &[crate::store::meta::ExternalFile],
    subject: &str,
    path: &str,
) -> Vec<KeyLoc> {
    let mut matches = Vec::new();
    for f in locator::envrc_files(dir) {
        if let Ok(locs) = locator::index_file(&f) {
            for l in locs {
                if l.subject == subject && l.path == path {
                    matches.push(l);
                }
            }
        }
    }
    for ef in external {
        if !ef.path.exists() {
            eprintln!(
                "warning: registered external file {} has vanished — skipping (re-run `dc load`; deregistration is not yet supported)",
                ef.path.display()
            );
            continue;
        }
        if let Ok(locs) = locator::index_external(&ef.path) {
            for l in locs {
                if l.subject == subject && l.path == path {
                    matches.push(l);
                }
            }
        }
    }
    matches
}

/// Compare two paths, following symlinks when both exist.
fn same_path(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

/// Resolve a `--file` override to an absolute path, validating it is a known
/// source: a registered external file or an `.envrc*` file in the source dir.
fn resolve_forced_file(file: &str) -> Result<PathBuf> {
    let store = crate::store::find_current_store()?;
    let meta = crate::store::meta::read_meta(&store)?;
    let target = std::fs::canonicalize(file)
        .map_err(|e| anyhow::anyhow!("--file {file}: {e}"))?;
    let known = meta.external.iter().any(|e| same_path(&e.path, &target))
        || locator::envrc_files(&meta.source).iter().any(|f| same_path(f, &target));
    if !known {
        let registered: Vec<String> =
            meta.external.iter().map(|e| e.path.display().to_string()).collect();
        bail!(
            "--file {} is not a registered/loaded source (registered external files: {})",
            file,
            if registered.is_empty() { "none".to_string() } else { registered.join(", ") }
        );
    }
    Ok(target)
}

/// Errata candidate files for a new-key insert: the forced file when given,
/// else registered externals in reverse load order (last-loaded first),
/// then `.envrc*` files (the pre-external behaviour, last).
fn errata_candidates(
    external: &[crate::store::meta::ExternalFile],
    dir: &Path,
    forced: Option<&Path>,
) -> Vec<PathBuf> {
    if let Some(f) = forced {
        return vec![f.to_path_buf()];
    }
    let mut out: Vec<PathBuf> = external.iter().map(|e| e.path.clone()).collect();
    out.reverse();
    out.extend(locator::envrc_files(dir));
    out
}

/// Current raw value text for a location (block scalars joined).
fn current_value_text(lines: &[String], loc: &KeyLoc) -> String {
    if loc.is_block_scalar {
        lines[loc.value_start..=loc.value_end.min(lines.len() - 1)].join("\n")
    } else {
        lines[loc.key_line].split_once(':').map(|x| x.1)
            .unwrap_or("")
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .to_string()
    }
}

fn existing_tier(lines: &[String], loc: &KeyLoc) -> u8 {
    crate::secret::parse_marker(&current_value_text(lines, loc))
        .map(|p| p.tier)
        .unwrap_or(0)
}

/// Compute the replacement line for a located key.
fn compute_set_line(loc: &KeyLoc, tier: u8, token: &str) -> String {
    let indent = " ".repeat(loc.indent);
    format!("{indent}{}: \"{} {}\"", leaf_key(&loc.path), marker(tier), token)
}

/// Splice a single replacement line over a location's key+value range.
fn splice_set(lines: &[String], loc: &KeyLoc, new_line: String) -> Vec<String> {
    let after = if loc.is_block_scalar {
        loc.value_end + 1
    } else {
        loc.key_line + 1
    };
    let mut out = lines[..loc.key_line].to_vec();
    out.push(new_line);
    out.extend_from_slice(&lines[after.min(lines.len())..]);
    out
}

/// Compute an errata insertion `(index, line)` for a missing leaf whose parent
/// (or block top-level) exists.
fn errata_compute(
    lines: &[String],
    subject: &str,
    path: &str,
    tier: u8,
    token: &str,
) -> Option<(usize, String)> {
    let leaf = leaf_key(path);
    let parent = path.rsplit_once('.').map(|(p, _)| p);

    for block in locator::scan_blocks(lines) {
        if block.subject != subject {
            continue;
        }
        let locs = locator::index_block(&block, lines, Path::new(""));
        match parent {
            Some(p) => {
                if let Some(ploc) = locs.iter().find(|l| l.path == p) {
                    let child_indent = ploc.indent + 2;
                    // Find end of the parent's subtree.
                    let mut idx = if ploc.is_block_scalar {
                        ploc.value_end + 1
                    } else {
                        ploc.key_line + 1
                    };
                    while idx < block.body.end {
                        let lt = lines[idx].trim_start();
                        if lt.is_empty() || lt.starts_with('#') {
                            idx += 1;
                            continue;
                        }
                        let ind = lines[idx].len() - lt.len();
                        if ind > ploc.indent {
                            idx += 1;
                        } else {
                            break;
                        }
                    }
                    let line = format!(
                        "{}{}: \"{} {}\"",
                        " ".repeat(child_indent),
                        leaf,
                        marker(tier),
                        token
                    );
                    return Some((idx, line));
                }
            }
            None => {
                // Top-level: insert just before the block terminator.
                let line = format!("{}: \"{} {}\"", leaf, marker(tier), token);
                return Some((block.body.end, line));
            }
        }
    }
    None
}

fn acquire_value(value: Option<&str>, from: Option<&str>, stdin: bool) -> Result<String> {
    let sources = [value.is_some(), from.is_some(), stdin]
        .iter()
        .filter(|b| **b)
        .count();
    if sources == 0 {
        bail!("provide a value with --value, --from <FILE>, or --stdin");
    }
    if sources > 1 {
        bail!("use only one of --value / --from / --stdin");
    }
    if let Some(v) = value {
        return Ok(v.to_string());
    }
    if let Some(f) = from {
        return Ok(std::fs::read_to_string(f)?.trim_end_matches('\n').to_string());
    }
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf.trim_end_matches('\n').to_string())
}

/// Compute an errata insertion `(index, line)` for a missing leaf in an
/// external plain-YAML file, mirroring [`errata_compute`] for the extra
/// subject-indentation level. `spans` covers subjects with empty bodies too.
fn external_errata_compute(
    lines: &[String],
    locs: &[KeyLoc],
    spans: &[locator::SubjectSpan],
    subject: &str,
    path: &str,
    tier: u8,
    token: &str,
) -> Option<(usize, String)> {
    let leaf = leaf_key(path);
    let parent = path.rsplit_once('.').map(|(p, _)| p);
    let subject_locs: Vec<&KeyLoc> = locs.iter().filter(|l| l.subject == subject).collect();

    match parent {
        Some(p) => {
            let span = spans.iter().find(|s| s.name == subject)?;
            let ploc = subject_locs.iter().find(|l| l.path == p)?;
            // Find the end of the parent's subtree (bounded by the subject's body).
            let mut idx = if ploc.is_block_scalar {
                ploc.value_end + 1
            } else {
                ploc.key_line + 1
            };
            while idx < span.body.end {
                let lt = lines[idx].trim_start();
                if lt.is_empty() || lt.starts_with('#') {
                    idx += 1;
                    continue;
                }
                let ind = lines[idx].len() - lt.len();
                if ind > ploc.indent {
                    idx += 1;
                } else {
                    break;
                }
            }
            let line = format!(
                "{}{}: \"{} {}\"",
                " ".repeat(ploc.indent + 2),
                leaf,
                marker(tier),
                token
            );
            Some((idx, line))
        }
        None => {
            // Top-level within the subject body: insert after its last key
            // (or right after the subject line for an empty body).
            let span = spans.iter().find(|s| s.name == subject)?;
            let body_indent = span.child_indent.unwrap_or(span.indent + 2);
            let idx = subject_locs
                .iter()
                .map(|l| if l.is_block_scalar { l.value_end } else { l.key_line })
                .max()
                .map(|last| last + 1)
                .unwrap_or(span.key_line + 1);
            let line = format!("{}{}: \"{} {}\"", " ".repeat(body_indent), leaf, marker(tier), token);
            Some((idx, line))
        }
    }
}

/// Ask a y/N question on stdin. Errors (instead of reading EOF as "no") when
/// stdin is not a terminal, so non-interactive callers must pass `--yes`.
pub(crate) fn confirm(prompt: &str) -> Result<bool> {
    let stdin = std::io::stdin();
    confirm_with(prompt, stdin.is_terminal(), &mut stdin.lock())
}

fn confirm_with(prompt: &str, is_tty: bool, input: &mut impl std::io::BufRead) -> Result<bool> {
    if !is_tty {
        bail!("cannot confirm: stdin is not a terminal — re-run with --yes to apply non-interactively");
    }
    eprint!("{prompt} [y/N] ");
    std::io::stderr().flush().ok();
    let mut buf = String::new();
    input.read_line(&mut buf)?;
    Ok(matches!(buf.trim().to_lowercase().as_str(), "y" | "yes"))
}

/// Print a context preview around a location, redacting secret values, and
/// marking the line(s) being changed.
fn preview(lines: &[String], key_line: usize, last_line: usize, new_display: Option<&str>) {
    let start = key_line.saturating_sub(2);
    let end = (last_line + 3).min(lines.len());
    eprintln!("  ┄┄ context ┄┄");
    for (i, line) in lines.iter().enumerate().take(end).skip(start) {
        if i >= key_line && i <= last_line {
            eprintln!("  ➡️  {}", redact_for_display(line));
        } else {
            eprintln!("      {line}");
        }
    }
    if let Some(nd) = new_display {
        eprintln!("  ✚  {nd}");
    }
}

fn redact_for_display(line: &str) -> String {
    if let Some(pos) = line.find("🔒:v1:") {
        let prefix = &line[..pos];
        return format!("{prefix}**redacted**\"");
    }
    if let Some(pos) = line.find("dcenc:v1:") {
        let prefix = &line[..pos];
        return format!("{prefix}**redacted**\"");
    }
    line.to_string()
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Replace a subject/path's value in its `.envrc*` source with the `⛔`
/// shadowed-secret sentinel. Returns true if a source line was rewritten.
pub(crate) fn write_sentinel(subject: &str, path: &str) -> Result<bool> {
    let locs = find_locs(subject, path)?;
    let Some(loc) = locs.first() else {
        return Ok(false);
    };
    let lines = read_lines(&loc.file)?;
    let new_line = format!("{}{}: \"⛔\"", " ".repeat(loc.indent), leaf_key(&loc.path));
    let out = splice_set(&lines, loc, new_line);
    write_lines(&loc.file, &out)?;
    Ok(true)
}

/// One refresh ingest unit: the source file (for edited-last ordering), the
/// parsed subject body, and the layer the source declared.
struct RefreshJob {
    file: PathBuf,
    body: serde_yaml::Value,
    layer: String,
}

/// Build the refresh job list for a subject from `.envrc*` heredoc sources
/// and registered external files (registry order = load order). Pure: all
/// file IO is done by the caller ([`refresh_store`]); here `envrc_sources`
/// are already-read `.envrc*` line arrays and `external_docs` the parsed
/// subjects per registered external file.
///
/// `--if-missing` sources are skipped on both sides — they seed the store
/// once and are store-owned afterwards (e.g. generated `auto` values);
/// re-ingesting would clobber the current value with the stale seed. Layer
/// comes from the source: the heredoc's `--layer` flag, or the layer
/// recorded for the external file at `dc load` time.
fn refresh_jobs(
    subject: &str,
    envrc_sources: &[(PathBuf, Vec<String>)],
    external: &[crate::store::meta::ExternalFile],
    external_docs: &[(PathBuf, Vec<(String, serde_yaml::Value)>)],
) -> Result<Vec<RefreshJob>> {
    let mut jobs: Vec<RefreshJob> = Vec::new();
    for (file, lines) in envrc_sources {
        let blocks = locator::scan_blocks(lines);
        for block in blocks
            .iter()
            .filter(|b| b.subject == subject && !lines[b.start_line].contains("--if-missing"))
        {
            let body = lines[block.body.clone()].join("\n");
            let input: serde_yaml::Value = serde_yaml::from_str(&body)?;
            jobs.push(RefreshJob {
                file: file.clone(),
                body: input,
                layer: block.layer.clone().unwrap_or_else(|| "base".to_string()),
            });
        }
    }
    for ef in external {
        if ef.if_missing {
            continue; // seed-once: store-owned after the first ingest
        }
        let Some((_, subjects)) = external_docs.iter().find(|(p, _)| p == &ef.path) else {
            continue;
        };
        for (subj, body) in subjects {
            if subj == subject {
                jobs.push(RefreshJob {
                    file: ef.path.clone(),
                    body: body.clone(),
                    layer: ef.layer.clone().unwrap_or_else(|| "base".to_string()),
                });
            }
        }
    }
    Ok(jobs)
}

/// Re-ingest every source for a subject into the store — `dc_yaml … <subject>`
/// heredocs across `.envrc*` files (same order as `envrc_files`) and the
/// subject's bodies from registered external files (registry order = load
/// order, honoring the layer/if-missing options recorded at load time) — the
/// way direnv re-sourcing the `.envrc` would. The file just edited (`edited`)
/// is ingested last so its keys win over stale duplicates in other sources.
/// `--replace`/`--replace-key` are never replayed here: a destructive
/// replace belongs to the explicit `dc load` invocation only. Without this,
/// `dc get` and `infisical-populate-secrets` keep serving the pre-edit value
/// until the shell re-sources the files.
fn refresh_store(subject: &str, edited: Option<&Path>) -> Result<()> {
    let store = crate::store::find_current_store()?;
    let _lock = crate::store::lock_store(&store)?;
    let meta = crate::store::meta::read_meta(&store)?;
    let dir = meta.source.clone();

    let mut envrc_sources: Vec<(PathBuf, Vec<String>)> = Vec::new();
    for file in locator::envrc_files(&dir) {
        envrc_sources.push((file.clone(), read_lines(&file)?));
    }
    let mut external_docs: Vec<(PathBuf, Vec<(String, serde_yaml::Value)>)> = Vec::new();
    for ef in &meta.external {
        if ef.if_missing {
            continue; // seed-once: never re-read, never re-ingested
        }
        if !ef.path.exists() {
            eprintln!(
                "warning: registered external file {} has vanished — skipping (re-run `dc load`; deregistration is not yet supported)",
                ef.path.display()
            );
            continue;
        }
        let content = std::fs::read_to_string(&ef.path)?;
        let doc: serde_yaml::Value = serde_yaml::from_str(&content)?;
        external_docs.push((ef.path.clone(), crate::cmd::load::parse_subjects(&ef.path, &doc)?));
    }

    let mut jobs = refresh_jobs(subject, &envrc_sources, &meta.external, &external_docs)?;
    // The edited source ingests last (stable sort keeps the rest in order).
    jobs.sort_by_key(|j| edited.is_some_and(|e| same_path(&j.file, e)));

    for job in &jobs {
        let opts =
            crate::cmd::load::IngestOpts { layer: Some(job.layer.as_str()), ..Default::default() };
        crate::cmd::load::ingest_subject(&store, subject, &job.body, &opts, &crate::settings::key)?;
    }

    crate::store::resolve::resolve_active(&store, subject)?;
    crate::store::meta::update_configs_list(&store)?;
    Ok(())
}

// <REMOVED UUID HERE> get :: auto-generated pointer for public function get
pub fn get(subject: &str, path: &str) -> Result<()> {
    let locs = find_locs(subject, path)?;
    if locs.is_empty() {
        println!("no source entry found for `{subject} {path}` in .envrc* files");
        return Ok(());
    }
    for l in &locs {
        let lines = read_lines(&l.file)?;
        println!(
            "{}:{}  ({} {})",
            l.file.display(),
            l.key_line + 1,
            subject,
            path
        );
        preview(&lines, l.key_line, if l.is_block_scalar { l.value_end } else { l.key_line }, None);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
// <REMOVED UUID HERE> set :: auto-generated pointer for public function set
pub fn set(
    subject: &str,
    path: &str,
    value: Option<&str>,
    from: Option<&str>,
    stdin_flag: bool,
    encrypted: bool,
    yes: bool,
    file: Option<&str>,
) -> Result<()> {
    let plaintext = acquire_value(value, from, stdin_flag)?;
    // Using stdin for the value means we cannot also prompt on stdin.
    let yes = yes || stdin_flag;

    let forced = match file {
        Some(f) => Some(resolve_forced_file(f)?),
        None => None,
    };

    let mut locs = find_locs(subject, path)?;
    if let Some(target) = &forced {
        locs.retain(|l| same_path(&l.file, target));
    }
    if locs.len() > 1 {
        eprintln!("ambiguous — `{subject} {path}` matches multiple locations:");
        for l in &locs {
            eprintln!("  {}:{}", l.file.display(), l.key_line + 1);
        }
        bail!("narrow the path, edit the file directly, or force one with --file <path>");
    }

    if locs.is_empty() {
        return set_errata(subject, path, &plaintext, encrypted, yes, forced.as_deref());
    }

    let loc = &locs[0];
    let mut lines = read_lines(&loc.file)?;
    let tier = existing_tier(&lines, loc);
    let token = if encrypted {
        plaintext.clone()
    } else {
        crate::crypto::encode_token(&plaintext, tier, &crate::settings::key()?)?
    };
    let new_line = compute_set_line(loc, tier, &token);
    let new_display = format!("{}: \"{} **redacted**\"", " ".repeat(loc.indent) + leaf_key(&loc.path), marker(tier));

    eprintln!("Update {} {} in {}:{}", subject, path, loc.file.display(), loc.key_line + 1);
    preview(&lines, loc.key_line, if loc.is_block_scalar { loc.value_end } else { loc.key_line }, Some(&new_display));
    if !yes && !confirm("Apply this change?")? {
        bail!("aborted — no changes written");
    }
    lines = splice_set(&lines, loc, new_line);
    write_lines(&loc.file, &lines)?;
    println!("updated {}:{}", loc.file.display(), loc.key_line + 1);
    refresh_store(subject, Some(&loc.file))?;
    Ok(())
}

/// Insert a new key for a subject that has no location for it yet. External
/// (registered) files are considered first in reverse load order — so new
/// keys land in the last-loaded file — then `.envrc*` heredocs. A `forced`
/// file (from `--file`) pins the destination.
#[allow(clippy::too_many_arguments)]
fn set_errata(
    subject: &str,
    path: &str,
    plaintext: &str,
    encrypted: bool,
    yes: bool,
    forced: Option<&Path>,
) -> Result<()> {
    let store = crate::store::find_current_store()?;
    let meta = crate::store::meta::read_meta(&store)?;
    let dir = meta.source.clone();

    let candidates = errata_candidates(&meta.external, &dir, forced);
    for f in &candidates {
        let is_external = meta.external.iter().any(|e| same_path(&e.path, f));
        let mut lines = if is_external {
            if !f.exists() {
                eprintln!(
                    "warning: registered external file {} has vanished — skipping (re-run `dc load`; deregistration is not yet supported)",
                    f.display()
                );
                continue;
            }
            // subject must exist in this external file
            let lines = read_lines(f)?;
            if !locator::external_subject_spans(&lines).iter().any(|s| s.name == subject) {
                continue;
            }
            lines
        } else {
            let lines = read_lines(f)?;
            if !locator::scan_blocks(&lines).iter().any(|b| b.subject == subject) {
                continue;
            }
            lines
        };
        let token = if encrypted {
            plaintext.to_string()
        } else {
            crate::crypto::encode_token(plaintext, 0, &crate::settings::key()?)?
        };
        let computed = if is_external {
            let spans = locator::external_subject_spans(&lines);
            let locs = locator::index_external_lines(&lines, f);
            external_errata_compute(&lines, &locs, &spans, subject, path, 0, &token)
        } else {
            errata_compute(&lines, subject, path, 0, &token)
        };
        let Some((idx, new_line)) = computed else {
            bail!(
                "subject `{subject}` exists in {} but parent of `{path}` was not found — use `dc config setall`",
                f.display()
            );
        };
        let display = redact_for_display(&new_line);
        eprintln!("Insert {subject} {path} into {}:{}", f.display(), idx + 1);
        preview(&lines, idx.saturating_sub(1), idx.saturating_sub(1), Some(&display));
        if !yes && !confirm("Insert this entry?")? {
            bail!("aborted — no changes written");
        }
        lines.insert(idx, new_line);
        write_lines(f, &lines)?;
        eprintln!("inserted {subject} {path} at {}:{}", f.display(), idx + 1);
        refresh_store(subject, Some(f))?;
        return Ok(());
    }
    if let Some(f) = forced {
        bail!("subject `{subject}` not found in {}", f.display());
    }
    bail!("no `dc_yaml … {subject}` block or registered external subject `{subject}` found — use `dc config setall` or `dc load`")
}

/// Initialize a new config entry with optional YAML and auto-generated secrets.
///
/// If `yaml_input` is empty, creates a minimal `dc_yaml` block. Otherwise,
/// parses the YAML and auto-generates values for keys marked with 🎲.
pub fn init(subject: &str, yaml_input: &str, auto_gen: bool, tier: u8, yes: bool) -> Result<()> {
    let dir = source_dir()?;

    // Parse input YAML if provided, otherwise use an empty map
    let mut config: serde_yaml::Value = if yaml_input.trim().is_empty() {
        serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
    } else {
        serde_yaml::from_str(yaml_input)?
    };

    // Auto-generate secrets for keys marked with 🎲
    if auto_gen {
        if let serde_yaml::Value::Mapping(ref mut map) = config {
            generate_secrets_in_place(map, tier)?;
        }
    }

    let yaml_str = serde_yaml::to_string(&config)?;

    // Find or create a dc_yaml block for this subject
    for f in locator::envrc_files(&dir) {
        let mut lines = read_lines(&f)?;
        let blocks = locator::scan_blocks(&lines);

        if blocks.iter().any(|b| b.subject == subject) {
            bail!("subject `{subject}` already exists in {}; use `dc config set` to modify", f.display());
        }

        // Check if there's a dc_yaml block for any subject in this file
        if let Some(block) = blocks.first() {
            // Insert at the end of the first dc_yaml block
            let insert_at = block.body.end;
            let indent = "dc_yaml".len() + 1; // minimal indentation for top-level

            let header = format!("dc_yaml {subject} <<'YAML'", );
            let footer = "YAML";

            eprintln!("Initialize {subject} in {}:{}", f.display(), insert_at + 1);
            preview(&lines, insert_at.saturating_sub(1), insert_at.saturating_sub(1), Some(&header));

            if !yes && !confirm("Initialize this config?")? {
                bail!("aborted — no changes written");
            }

            let mut new_lines = vec![header];
            new_lines.extend(yaml_str.lines().map(|l| l.to_string()));
            new_lines.push(footer.to_string());

            let mut out = lines[..insert_at].to_vec();
            out.extend(new_lines);
            out.extend_from_slice(&lines[insert_at..]);

            write_lines(&f, &out)?;
            println!("initialized {subject} at {}:{}", f.display(), insert_at + 1);
            refresh_store(subject)?;
            return Ok(());
        }
    }

    bail!("no `dc_yaml` block found in any .envrc* file — use `dc yaml` to create one first")
}

/// Recursively scan a YAML mapping and generate secrets for keys marked with 🎲.
fn generate_secrets_in_place(map: &mut serde_yaml::Mapping, tier: u8) -> Result<()> {
    let key_vec: Vec<serde_yaml::Value> = map.keys().cloned().collect();

    for key in key_vec {
        if let Some(value) = map.get_mut(&key) {
            match value {
                serde_yaml::Value::String(s) if s.starts_with('🎲') => {
                    // Generate a secret for this key
                    let plaintext = crate::cmd::gen::generate_secret(false, false, 32);
                    let token = crate::crypto::encode_token(&plaintext, tier, &crate::settings::key()?)?;
                    *value = serde_yaml::Value::String(token);
                }
                serde_yaml::Value::Mapping(ref mut nested_map) => {
                    generate_secrets_in_place(nested_map, tier)?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Insert a YAML section relative to an anchor entry.
// <REMOVED UUID HERE> setall :: Insert a YAML section relative to an anchor entry.
pub fn setall(
    layer_subject: &str,
    yaml_input: &str,
    anchor_subject: &str,
    anchor_path: &str,
    below: bool,
    yes: bool,
    file: Option<&str>,
) -> Result<()> {
    let store = crate::store::find_current_store()?;
    let meta = crate::store::meta::read_meta(&store)?;
    let dir = meta.source.clone();
    let forced = match file {
        Some(f) => Some(resolve_forced_file(f)?),
        None => None,
    };

    // Registered external files (reverse load order — last-loaded first), then
    // `.envrc*` files, unless --file pins the destination.
    for f in errata_candidates(&meta.external, &dir, forced.as_deref()) {
        let is_external = meta.external.iter().any(|e| same_path(&e.path, &f));
        let mut lines = if is_external {
            if !f.exists() {
                eprintln!(
                    "warning: registered external file {} has vanished — skipping (re-run `dc load`; deregistration is not yet supported)",
                    f.display()
                );
                continue;
            }
            let lines = read_lines(&f)?;
            if !locator::scan_blocks(&lines).iter().any(|b| b.subject == layer_subject)
                && !locator::index_external_lines(&lines, &f)
                    .iter()
                    .any(|l| l.subject == layer_subject)
            {
                continue;
            }
            lines
        } else {
            let lines = read_lines(&f)?;
            if !locator::scan_blocks(&lines).iter().any(|b| b.subject == layer_subject) {
                continue;
            }
            lines
        };
        let locs = if is_external {
            locator::index_external_lines(&lines, &f)
                .into_iter()
                .filter(|l| l.subject == layer_subject)
                .collect()
        } else {
            let blocks = locator::scan_blocks(&lines);
            let Some(block) = blocks.iter().find(|b| b.subject == layer_subject) else {
                continue;
            };
            locator::index_block(block, &lines, &f)
        };
        // The anchor must belong to the requested subject too (we edit one block).
        let _ = anchor_subject;
        let Some(anchor) = locs.iter().find(|l| l.path == anchor_path) else {
            continue;
        };
        let indent = anchor.indent;
        let insert_at = if below {
            if anchor.is_block_scalar { anchor.value_end + 1 } else { anchor.key_line + 1 }
        } else {
            anchor.key_line
        };
        // Re-indent the provided YAML to the anchor's indent level.
        let new_lines: Vec<String> = yaml_input
            .lines()
            .map(|l| format!("{}{}", " ".repeat(indent), l))
            .collect();

        eprintln!(
            "Insert {} line(s) {} {} {} in {}:{}",
            new_lines.len(),
            if below { "below" } else { "above" },
            layer_subject,
            anchor_path,
            f.display(),
            insert_at + 1
        );
        for l in &new_lines {
            eprintln!("  ✚  {l}");
        }
        if !yes && !confirm("Insert this section?")? {
            bail!("aborted — no changes written");
        }
        let mut out = lines[..insert_at].to_vec();
        out.extend(new_lines);
        out.extend_from_slice(&lines[insert_at..]);
        lines = out;
        write_lines(&f, &lines)?;
        eprintln!("inserted section at {}:{}", f.display(), insert_at + 1);
        refresh_store(layer_subject, Some(&f))?;
        return Ok(());
    }
    bail!(
        "anchor `{anchor_subject} {anchor_path}` not found in any registered external file or .envrc* file"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envrc::locator::{index_block, scan_blocks};

    const KEY: [u8; 32] = [5u8; 32];

    #[test]
    fn confirm_refuses_non_terminal_stdin() {
        // A non-interactive caller must get an error, not a silent "no" + exit 0.
        let err = confirm_with("Apply?", false, &mut "y\n".as_bytes()).unwrap_err().to_string();
        assert!(err.contains("--yes"), "{err}");
    }

    #[test]
    fn confirm_reads_answer_on_terminal() {
        assert!(confirm_with("Apply?", true, &mut "yes\n".as_bytes()).unwrap());
        assert!(!confirm_with("Apply?", true, &mut "\n".as_bytes()).unwrap());
    }

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(|l| l.to_string()).collect()
    }

    fn locate(src: &[String], subject: &str, path: &str) -> KeyLoc {
        for b in scan_blocks(src) {
            for l in index_block(&b, src, Path::new("/x/.envrc")) {
                if l.subject == subject && l.path == path {
                    return l;
                }
            }
        }
        panic!("not found");
    }

    #[test]
    fn set_single_line_preserves_siblings_and_comments() {
        let src = lines(
            "dc_yaml --no-bump cf <<'YAML'\n# keep me\naccount_id: 123\naccess:\n  client_secret: \"🔒❗ old\"\nzone: z\nYAML",
        );
        let loc = locate(&src, "cf", "access.client_secret");
        let tier = existing_tier(&src, &loc);
        assert_eq!(tier, 1);
        let token = crate::crypto::encode_token("newsecret", tier, &KEY).unwrap();
        let new_line = compute_set_line(&loc, tier, &token);
        let out = splice_set(&src, &loc, new_line);

        // comment + siblings intact
        assert!(out.iter().any(|l| l == "# keep me"));
        assert!(out.iter().any(|l| l == "account_id: 123"));
        assert!(out.iter().any(|l| l == "zone: z"));
        // the secret line now holds an encrypted token, still 🔒❗ tier-1, indented 2
        let line = out.iter().find(|l| l.contains("client_secret")).unwrap();
        assert!(line.starts_with("  client_secret: \"🔒❗ 🔒:v1:t1:"));
    }

    #[test]
    fn set_block_scalar_collapses_to_single_line() {
        let src = lines(
            "dc_yaml --no-bump cf <<'YAML'\naccount_id: |\n  🔒 line-one\n  line-two\nzone: z\nYAML",
        );
        let loc = locate(&src, "cf", "account_id");
        assert!(loc.is_block_scalar);
        let token = crate::crypto::encode_token("v", 0, &KEY).unwrap();
        let new_line = compute_set_line(&loc, 0, &token);
        let out = splice_set(&src, &loc, new_line);
        // The 4-line block is gone; zone survives directly after.
        assert!(out.iter().any(|l| l.starts_with("account_id: \"🔒 🔒:v1:")));
        assert!(out.iter().any(|l| l == "zone: z"));
        assert!(!out.iter().any(|l| l.contains("line-two")));
    }

    #[test]
    fn errata_inserts_under_existing_parent() {
        let src = lines(
            "dc_yaml --no-bump cf <<'YAML'\naccess:\n  client_id: pub\nzone: z\nYAML",
        );
        let token = crate::crypto::encode_token("BANANA", 0, &KEY).unwrap();
        let (idx, line) = errata_compute(&src, "cf", "access.foobar", 0, &token).unwrap();
        // Inserted after access.client_id (index 3), indented 2.
        assert_eq!(idx, 3);
        assert!(line.starts_with("  foobar: \"🔒 🔒:v1:"));
        let mut out = src.clone();
        out.insert(idx, line);
        assert_eq!(out[2], "  client_id: pub");
        assert!(out[3].starts_with("  foobar:"));
        assert_eq!(out[4], "zone: z");
    }

    #[test]
    fn errata_inserts_top_level_before_terminator() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\naccount_id: 1\nYAML");
        let token = crate::crypto::encode_token("t", 0, &KEY).unwrap();
        let (idx, line) = errata_compute(&src, "cf", "api_token", 0, &token).unwrap();
        assert_eq!(idx, 2); // the YAML terminator line index
        assert!(line.starts_with("api_token: \"🔒 🔒:v1:"));
    }

    // ---- external-file routing (dc load provenance) ----

    use crate::store::meta::ExternalFile;
    use tempfile::TempDir;

    fn ext(p: &std::path::Path) -> ExternalFile {
        ExternalFile { path: p.to_path_buf(), mtime: 0, size: 0, layer: None, if_missing: false }
    }

    #[test]
    fn find_locs_in_spans_envrc_and_registered_external() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        std::fs::write(dir.join(".envrc"), "dc_yaml --no-bump app <<'YAML'\nhost: local\nYAML\n").unwrap();
        let extfile = dir.join("team.yaml");
        std::fs::write(&extfile, "app:\n  # db config\n  db:\n    host: \"🔒❗ tok\"\n").unwrap();
        let external = vec![ext(&extfile)];

        let locs = find_locs_in(dir, &external, "app", "db.host");
        assert_eq!(locs.len(), 1, "external hit expected");
        assert_eq!(locs[0].file, extfile);
        assert_eq!(locs[0].key_line, 3); // 0-indexed: app=0, comment=1, db=2, host=3
        assert_eq!(locs[0].layer, None);

        // a vanished registered file warns + skips; .envrc* locs still found
        let gone = dir.join("gone.yaml");
        let external2 = vec![ext(&gone), ext(&extfile)];
        let locs2 = find_locs_in(dir, &external2, "app", "host");
        assert_eq!(locs2.len(), 1);
        assert!(locs2[0].file.ends_with(".envrc"), "{}", locs2[0].file.display());
        assert_eq!(locs2[0].key_line, 1);
    }

    #[test]
    fn find_locs_in_ambiguous_two_files_names_both_candidates() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let a = dir.join("a.yaml");
        let b = dir.join("b.yaml");
        std::fs::write(&a, "app:\n  host: one\n").unwrap();
        std::fs::write(&b, "app:\n  host: two\n").unwrap();
        let external = vec![ext(&a), ext(&b)];

        let locs = find_locs_in(dir, &external, "app", "host");
        // both candidates survive with distinct file:line (what the refusal lists)
        assert_eq!(locs.len(), 2, "expected ambiguous pair");
        assert_eq!(locs[0].file, a);
        assert_eq!(locs[0].key_line, 1);
        assert_eq!(locs[1].file, b);
        assert_eq!(locs[1].key_line, 1);
    }

    #[test]
    fn set_splices_external_file_preserving_comments_and_siblings() {
        let tmp = TempDir::new().unwrap();
        let extfile = tmp.path().join("t.yaml");
        std::fs::write(
            &extfile,
            "app:\n  # tuning\n  pool: 5\n  host: \"🔒❗ old-tok\"\n  # trailing note\n",
        )
        .unwrap();
        let lines = read_lines(&extfile).unwrap();
        let locs = locator::index_external_lines(&lines, &extfile);
        let loc = locs.iter().find(|l| l.path == "host").expect("host loc");
        let tier = existing_tier(&lines, loc);
        let token = crate::crypto::encode_token("NEWVAL", tier, &KEY).unwrap();
        let new_line = compute_set_line(loc, tier, &token);
        let out = splice_set(&lines, loc, new_line);

        assert_eq!(out.len(), 5, "line count unchanged");
        assert_eq!(out[0], "app:");
        assert_eq!(out[1], "  # tuning");
        assert_eq!(out[2], "  pool: 5");
        assert!(out[3].starts_with("  host: \""), "host line replaced in place");
        assert!(!out[3].contains("old-tok"), "stale token gone");
        assert_eq!(out[4], "  # trailing note");
    }

    #[test]
    fn errata_routes_to_last_loaded_file_and_respects_file_override() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let a = dir.join("a.yaml");
        let b = dir.join("b.yaml");
        std::fs::write(&a, "app:\n  host: one\n").unwrap();
        std::fs::write(&b, "app:\n  pool: 9\n").unwrap();
        let external = vec![ext(&a), ext(&b)];

        // Default: reverse load order — last-loaded (b) first, then .envrc*.
        let cands = errata_candidates(&external, dir, None);
        assert_eq!(cands, vec![b.clone(), a.clone()]);

        // --file override pins the destination.
        assert_eq!(errata_candidates(&external, dir, Some(&a)), vec![a.clone()]);

        // The computed insert lands inside the pinned file, top-level body key.
        let lines = read_lines(&a).unwrap();
        let spans = locator::external_subject_spans(&lines);
        let locs = locator::index_external_lines(&lines, &a);
        let token = crate::crypto::encode_token("v", 0, &KEY).unwrap();
        let (idx, line) =
            external_errata_compute(&lines, &locs, &spans, "app", "api_key", 0, &token).unwrap();
        assert_eq!(idx, 2, "after `host: one` (line 1)");
        assert!(line.starts_with("  api_key: \"🔒 🔒:v1:"));
    }

    #[test]
    fn external_errata_under_parent_and_for_empty_body() {
        let src = lines("app:\n  db:\n    host: h\n  # note\nother:\n  x: 1\n");
        let file = Path::new("/x/team.yaml");
        let spans = locator::external_subject_spans(&src);
        let locs = locator::index_external_lines(&src, file);
        let token = crate::crypto::encode_token("v", 0, &KEY).unwrap();

        // Parent path: inserted after db's subtree (comments skipped), indent parent+2.
        let (idx, line) =
            external_errata_compute(&src, &locs, &spans, "app", "db.port", 0, &token).unwrap();
        assert_eq!(idx, 4, "after the trailing comment, before `other:`");
        assert!(line.starts_with("    port: \"🔒 🔒:v1:"));

        // Subject with an empty body: insert directly under the subject line.
        let src2 = lines("app:\n  x: 1\nboot:\n");
        let spans2 = locator::external_subject_spans(&src2);
        let locs2 = locator::index_external_lines(&src2, file);
        let (idx2, line2) =
            external_errata_compute(&src2, &locs2, &spans2, "boot", "api_key", 0, &token).unwrap();
        assert_eq!(idx2, 3, "after `boot:` (line 2, the last line)");
        assert!(line2.starts_with("  api_key: \"🔒 🔒:v1:"));
    }

    // ---- refresh: load options recorded in the registry are honored ----

    #[test]
    fn refresh_jobs_skips_if_missing_sources_on_both_sides() {
        let envrc = vec![(
            PathBuf::from("/x/.envrc"),
            lines(
                "dc_yaml --no-bump app <<'YAML'\nplain: 1\nYAML\n\
                 dc_yaml --if-missing app <<'YAML'\nseeded: s\nYAML\n",
            ),
        )];
        let seed = PathBuf::from("/x/seed.yaml");
        let external = vec![ExternalFile {
            path: seed.clone(),
            mtime: 0,
            size: 0,
            layer: None,
            if_missing: true,
        }];
        let docs = vec![(
            seed,
            vec![(
                "app".to_string(),
                serde_yaml::from_str::<serde_yaml::Value>("seeded: s\n").unwrap(),
            )],
        )];

        let jobs = refresh_jobs("app", &envrc, &external, &docs).unwrap();
        assert_eq!(jobs.len(), 1, "only the plain (non-if-missing) heredoc survives");
        assert_eq!(jobs[0].file, PathBuf::from("/x/.envrc"));
        for j in &jobs {
            assert!(
                j.body.get("seeded").is_none(),
                "seed-once values must not be re-ingested (got body {:?})",
                j.body
            );
        }
    }

    #[test]
    fn refresh_jobs_uses_recorded_layer_not_base() {
        let f = PathBuf::from("/x/stg.yaml");
        let body = serde_yaml::from_str::<serde_yaml::Value>("region: eu\n").unwrap();
        let docs = vec![(f.clone(), vec![("app".to_string(), body)])];

        let external = vec![ExternalFile {
            path: f.clone(),
            mtime: 0,
            size: 0,
            layer: Some("staging".to_string()),
            if_missing: false,
        }];
        let jobs = refresh_jobs("app", &[], &external, &docs).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].layer, "staging", "recorded load layer must be replayed");

        // No layer recorded (incl. pre-options registry entries) -> base.
        let external2 = vec![ExternalFile { layer: None, if_missing: false, ..external[0].clone() }];
        let jobs2 = refresh_jobs("app", &[], &external2, &docs).unwrap();
        assert_eq!(jobs2[0].layer, "base");
    }

    /// Restore process-global state (CWD + XDG_STATE_HOME) when dropped, so a
    /// panicking test does not poison the rest of the suite.
    /// Serializes e2e tests that mutate process-wide state (CWD + XDG_STATE_HOME).
    static REFRESH_E2E_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
    struct EnvGuard {
        cwd: PathBuf,
        xdg: Option<String>,
    }

    impl EnvGuard {
        fn capture() -> Self {
            EnvGuard {
                cwd: std::env::current_dir().unwrap(),
                xdg: std::env::var("XDG_STATE_HOME").ok(),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.cwd);
            match &self.xdg {
                Some(v) => std::env::set_var("XDG_STATE_HOME", v),
                None => std::env::remove_var("XDG_STATE_HOME"),
            }
        }
    }

    #[test]
    fn refresh_store_honors_per_subject_dc_layer_routing() {
        let _env_lock = REFRESH_E2E_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::capture();

        let tmp = TempDir::new().unwrap();
        let proj = std::fs::canonicalize({
            std::fs::create_dir_all(tmp.path().join("proj")).unwrap();
            tmp.path().join("proj")
        })
        .unwrap();
        // canonicalize so the store hash from `ensure_store(&proj)` matches
        // the one `find_current_store` computes from the OS-resolved CWD
        // (macOS $TMPDIR is a /var -> /private/var symlink).
        std::fs::write(
            proj.join("team.yaml"),
            "app:\n  _dc_layer: staging\n  runner: linux\n",
        )
        .unwrap();

        let state = tmp.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::env::set_var("XDG_STATE_HOME", &state);
        std::env::set_current_dir(&proj).unwrap();
        let store = crate::store::layout::ensure_store(&proj).unwrap();

        let files = vec![proj.join("team.yaml")];
        crate::cmd::load::load_files(
            &store,
            &files,
            &crate::cmd::load::IngestOpts::default(),
            &|| Ok([0u8; 32]),
        )
        .unwrap();

        // A refresh triggered by editing any source must keep routing the
        // subject through the body's `_dc_layer`, not the file default.
        refresh_store("app", None).unwrap();

        let staging = crate::store::load_layer(&crate::store::layout::layer_path(&store, "app", "staging"))
            .unwrap();
        assert_eq!(staging.get("runner"), Some(&serde_yaml::Value::String("linux".into())));
        let base_path = crate::store::layout::layer_path(&store, "app", "base");
        assert!(
            !base_path.exists(),
            "`_dc_layer: staging` subject must not land in base.yaml on refresh"
        );
    }

    #[test]
    fn refresh_store_does_not_revert_store_owned_values_after_if_missing_load() {
        let _env_lock = REFRESH_E2E_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::capture();
        let tmp = TempDir::new().unwrap();
        let proj = std::fs::canonicalize({
            std::fs::create_dir_all(tmp.path().join("proj")).unwrap();
            tmp.path().join("proj")
        })
        .unwrap();
        // canonicalize so the store hash from `ensure_store(&proj)` matches
        // the one `find_current_store` computes from the OS-resolved CWD
        // (macOS $TMPDIR is a /var -> /private/var symlink).
        // A refreshable heredoc for the same subject (a different key).
        std::fs::write(
            proj.join(".envrc"),
            "dc_yaml --no-bump app <<'YAML'\nother: heredoc\nYAML\n",
        )
        .unwrap();
        // The seed file: loaded with --if-missing, so its values become
        // store-owned after the first ingest. Plain values only — no 🔒, so
        // the (real-machine) encryption key is never needed.
        let seed = tmp.path().join("seed.yaml");
        std::fs::write(&seed, "app:\n  token: seed-value\n").unwrap();

        let _guard = EnvGuard::capture();
        std::env::set_var("XDG_STATE_HOME", tmp.path().join("state"));
        std::env::set_current_dir(&proj).unwrap();

        let store = crate::store::layout::ensure_store(&proj).unwrap();
        let opts = crate::cmd::load::IngestOpts { if_missing: true, ..Default::default() };
        crate::cmd::load::load_files(
            &store,
            std::slice::from_ref(&seed),
            &opts,
            &|| anyhow::bail!("no key needed for plain bodies"),
        )
        .unwrap();

        // Store-side mutation of the seeded key (what --if-missing ownership means).
        let mutated: serde_yaml::Value = serde_yaml::from_str("token: rotated-by-store\n").unwrap();
        crate::cmd::yaml::apply(
            &store,
            "app",
            "base",
            &mutated,
            &[],
            None,
            false,
            None,
        )
        .unwrap();

        // A refresh triggered by editing any key of the subject must not revert it.
        refresh_store("app", None).unwrap();

        let base = crate::store::load_layer(&crate::store::layout::layer_path(&store, "app", "base"))
            .unwrap();
        assert_eq!(
            crate::yaml::path::get_path(&base, "token"),
            Some(serde_yaml::Value::String("rotated-by-store".into())),
            "--if-missing seed must not clobber the store-owned value on refresh"
        );
        assert_eq!(
            crate::yaml::path::get_path(&base, "other"),
            Some(serde_yaml::Value::String("heredoc".into())),
            "the refreshable heredoc key is still ingested"
        );
    }
}
