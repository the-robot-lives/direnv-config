//! `dc config get/set/setall` — locate and edit secrets in source `.envrc*` files.
//!
//! These commands operate on the literal hand-authored heredoc files (not the
//! state store), encrypting secret values at rest inline as `🔒 🔒:…` and
//! preserving all surrounding formatting/comments via line splicing.

use anyhow::{bail, Context, Result};
use std::io::{IsTerminal, Read, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::envrc::locator::{self, KeyLoc};
use crate::secret::PADLOCK;

// ---------------------------------------------------------------------------
// Parse-based placement verification (hard safety gate)
// ---------------------------------------------------------------------------
//
// Line heuristics decide WHERE to edit; the YAML parse decides whether that
// edit is allowed to be written at all. Every writer below (plain insert,
// `-p` chain insert, update, setall) must pass the same three gates:
//
//   1. the subject's body parses as YAML and the parent path resolves in
//      the parsed tree (unresolvable ⇒ warn naming subject+path, exit
//      non-zero, write nothing);
//   2. simulating the edit and re-parsing proves `<subject>.<path>` lands
//      at exactly its path and NOTHING else in the tree changed;
//   3. after the write, the file is re-read from disk and gate 2 is
//      re-asserted; on any mismatch the original bytes are restored and
//      the command exits non-zero.

/// Line range holding `subject`'s YAML in `lines`, plus the external flag.
/// Heredoc block: the body alone is the subject's mapping (the subject is
/// the heredoc label). External file: the fragment from the subject key to
/// the next subject parses standalone with the subject as root key.
fn subject_body_range(
    lines: &[String],
    file: &Path,
    external: bool,
    subject: &str,
) -> Result<(Range<usize>, bool)> {
    if external {
        let span = locator::external_subject_spans(lines)
            .into_iter()
            .find(|s| s.name == subject)
            .ok_or_else(|| {
                anyhow::anyhow!("subject `{subject}` not found in {}", file.display())
            })?;
        Ok((span.key_line..span.body.end, true))
    } else {
        let block = locator::scan_blocks(lines)
            .into_iter()
            .find(|b| b.subject == subject)
            .ok_or_else(|| {
                anyhow::anyhow!("subject `{subject}` not found in {}", file.display())
            })?;
        Ok((block.body, false))
    }
}

/// Parse `lines[range]` and return the subject's own tree (external files
/// hop into the subject's root key first). An empty body normalizes to an
/// empty mapping so before/after comparisons stay symmetric.
fn parse_subject_tree(
    lines: &[String],
    range: Range<usize>,
    external: bool,
    subject: &str,
) -> Result<serde_yaml::Value> {
    let text = lines[range].join("\n");
    let root: serde_yaml::Value =
        serde_yaml::from_str(&text).map_err(|e| anyhow::anyhow!("YAML parse failed: {e}"))?;
    let tree = if external {
        root.get(subject).cloned().ok_or_else(|| {
            anyhow::anyhow!("subject `{subject}` did not resolve in the parsed YAML")
        })?
    } else {
        root
    };
    Ok(match tree {
        serde_yaml::Value::Null => serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        other => other,
    })
}

/// The hard invariant for a single-leaf edit: applying the edit to the
/// `before` tree (`before` + `path` = `expected_field`) must reproduce the
/// `after` tree exactly. This proves the leaf landed at its own path —
/// i.e. inside the intended parent section — and that NOTHING else changed
/// (missing intermediates created by `-p` chains included).
fn assert_only_leaf_changed(
    before: &serde_yaml::Value,
    after: &serde_yaml::Value,
    subject: &str,
    path: &str,
    expected_field: &str,
) -> Result<()> {
    let expected: serde_yaml::Value = serde_yaml::from_str(expected_field)
        .map_err(|e| anyhow::anyhow!("written value is not valid YAML: {e}"))?;
    let mut expected_tree = before.clone();
    crate::yaml::path::set_path(&mut expected_tree, path, expected)
        .map_err(|e| anyhow::anyhow!("cannot apply `{path}` to the parsed tree: {e}"))?;
    if expected_tree != *after {
        let detail = if crate::yaml::path::get_path(after, path).is_none() {
            format!(
                "`{subject} {path}` would NOT resolve at its own path — the computed insertion point falls outside the parent section"
            )
        } else {
            format!("writing `{subject} {path}` would change other content in the section")
        };
        bail!("placement check failed: {detail}; no changes written (subject: {subject}, path: {path})");
    }
    Ok(())
}

/// Multi-key variant for `setall` section inserts: applying every inserted
/// key to the `before` tree must reproduce `after` exactly.
fn assert_only_keys_added(
    before: &serde_yaml::Value,
    after: &serde_yaml::Value,
    parent: Option<&str>,
    keys: &[(String, serde_yaml::Value)],
    subject: &str,
) -> Result<()> {
    let full = |k: &str| match parent {
        Some(p) => format!("{p}.{k}"),
        None => k.to_string(),
    };
    let mut expected_tree = before.clone();
    for (k, v) in keys {
        crate::yaml::path::set_path(&mut expected_tree, &full(k), v.clone())
            .map_err(|e| anyhow::anyhow!("cannot apply `{}` to the parsed tree: {e}", full(k)))?;
    }
    if expected_tree != *after {
        bail!(
            "placement check failed: inserting the section under `{subject}` would not land its keys at `{}` exactly, or would change other content; no changes written",
            keys.iter().map(|(k, _)| full(k)).collect::<Vec<_>>().join(", ")
        );
    }
    Ok(())
}

/// Gate 3: re-read `f` from disk, re-parse the subject's tree, and re-run
/// [`assert_only_leaf_changed`]. Any failure restores the original bytes
/// and exits non-zero — the file is never left half-edited.
fn post_write_verify_or_restore(
    f: &Path,
    original_bytes: &str,
    before_tree: &serde_yaml::Value,
    external: bool,
    subject: &str,
    path: &str,
    expected_field: &str,
) -> Result<()> {
    let failure = (|| -> Result<()> {
        // Test-only hook: simulate a post-write mismatch to exercise the
        // restore path in integration tests (never set in real environments).
        if std::env::var_os("DC_TEST_POST_WRITE_CORRUPT").is_some() {
            bail!("simulated post-write mismatch (test hook)");
        }
        let reread = read_lines(f)?;
        let (range, ext) = subject_body_range(&reread, f, external, subject)?;
        let tree = parse_subject_tree(&reread, range, ext, subject)?;
        assert_only_leaf_changed(before_tree, &tree, subject, path, expected_field)
    })();
    match failure {
        Ok(()) => Ok(()),
        Err(e) => {
            let restore = std::fs::write(f, original_bytes);
            bail!(
                "post-write verification failed for `{subject} {path}`: {e}; {}",
                match restore {
                    Ok(()) => "original file content restored".to_string(),
                    Err(rerr) => format!("RESTORE FAILED — file may be inconsistent ({rerr})"),
                }
            );
        }
    }
}

/// Insert-preview that renders the context around the REAL write position:
/// the ✚ lines print exactly where they will land (after the ➡-marked
/// line), not appended after the window — the previous rendering showed
/// following sections as if they were the parent.
fn preview_insert(lines: &[String], idx: usize, new_lines: &[String], mode: ValueMode, generated: bool) {
    let start = idx.saturating_sub(2);
    let end = (idx + 3).min(lines.len());
    eprintln!("  ┄┄ context ┄┄");
    for (i, line) in lines.iter().enumerate().take(end).skip(start) {
        if i == idx {
            for l in new_lines {
                eprintln!("  ✚  {}", preview_line(l, mode, generated));
            }
        }
        if i + 1 == idx {
            eprintln!("  ➡️  {}", redact_for_display(line));
        } else {
            eprintln!("      {line}");
        }
    }
    if idx >= lines.len() {
        for l in new_lines {
            eprintln!("  ✚  {}", preview_line(l, mode, generated));
        }
    }
}

pub(crate) fn read_lines(path: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.lines().map(|s| s.to_string()).collect())
}

fn write_lines(path: &Path, lines: &[String]) -> Result<()> {
    let mut body = lines.join("\n");
    body.push('\n');
    std::fs::write(path, body)?;
    Ok(())
}

/// Atomic variant of [`write_lines`]: writes a sibling temp file, preserves
/// the original's unix permissions, then renames over the target — a crash
/// mid-write can never leave a truncated source file behind. A failure part
/// way through (create/write/rename) removes the sibling temp instead of
/// leaking it next to the source.
pub(crate) fn write_lines_atomic(path: &Path, lines: &[String]) -> Result<()> {
    use std::io::Write as _;
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let fname = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = dir.join(format!(".{fname}.dc-tmp-{}", std::process::id()));
    let attempt = || -> Result<()> {
        let mut body = lines.join("\n");
        body.push('\n');
        {
            let mut f = std::fs::File::create(&tmp)
                .with_context(|| format!("create temp file {}", tmp.display()))?;
            f.write_all(body.as_bytes())?;
            f.sync_all().ok();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(path) {
                std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(meta.permissions().mode())).ok();
            }
        }
        std::fs::rename(&tmp, path)
            .with_context(|| format!("atomically replace {}", path.display()))?;
        Ok(())
    };
    let result = attempt();
    if result.is_err() {
        // best-effort: a failed write must not accumulate dotfile siblings
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

pub(crate) fn marker(tier: u8) -> String {
    let bangs: String = std::iter::repeat_n('❗', tier.min(3) as usize).collect();
    format!("{PADLOCK}{bangs}")
}

pub(crate) fn leaf_key(path: &str) -> &str {
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
pub(crate) fn same_path(a: &Path, b: &Path) -> bool {
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
pub(crate) fn resolve_forced_file(file: &str) -> Result<PathBuf> {
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
pub(crate) fn current_value_text(lines: &[String], loc: &KeyLoc) -> String {
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

/// Compute the replacement line for a located key. The key is written back
/// exactly as it was found (`loc.raw_key`, so quoted keys keep their quotes).
/// In `plain` mode the value is stored literally — no 🔒 marker, no encryption
/// (a previously encrypted value's tier marker is dropped); in
/// `numeric`/`bool` mode it is written as a bare YAML number/boolean (also
/// unmarked); otherwise the marker + token form.
pub(crate) fn compute_set_line(loc: &KeyLoc, tier: u8, token: &str, mode: ValueMode) -> String {
    let indent = " ".repeat(loc.indent);
    format!("{indent}{}: {}", loc.raw_key, value_field(tier, token, mode))
}

/// How `dc config set` renders the value it writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueMode {
    /// Encrypt and store `🔒<tier-bangs> 🔒:v1:…` (the default).
    Encrypt,
    /// Store the literal value as a quoted string, unmarked (`--plain`).
    Plain,
    /// Store the value as a bare YAML number — implies plaintext, no marker
    /// (`--numeric`; numbers are never encrypted).
    Numeric,
    /// Store the value as a bare YAML boolean — implies plaintext, no marker
    /// (`--bool`; booleans are never encrypted).
    Bool,
}

/// The complete YAML scalar for a written value, quotes included: a quoted
/// literal in `plain` mode, a bare number / boolean in `numeric`/`bool`
/// mode, else the quoted `🔒<bangs> token` marked form.
fn value_field(tier: u8, token: &str, mode: ValueMode) -> String {
    match mode {
        ValueMode::Plain => format!("\"{token}\""),
        ValueMode::Numeric | ValueMode::Bool => token.to_string(),
        ValueMode::Encrypt => format!("\"{} {}\"", marker(tier), token),
    }
}

/// Validate a `--numeric` value: an integer or finite float written as an
/// unquoted YAML scalar. Accepts `4`, `-3`, `2.5`, `1e3`, `+4`, `.5`; rejects
/// everything else — NaN/inf spellings included — with a clear error so an
/// invalid value can never reach the file. Returns the trimmed text to write.
fn validate_numeric(value: &str) -> Result<String> {
    let s = value.trim();
    let ok = s.parse::<f64>().map(|f| f.is_finite()).unwrap_or(false)
        && s.chars().all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E'));
    if !ok {
        bail!(
            "--numeric needs a number (integer or float, e.g. 4, -3, 2.5, 1e3); got `{value}`"
        );
    }
    Ok(s.to_string())
}

/// Validate a `--bool` value: `true`/`false` case-insensitively, normalized
/// to lowercase for writing. Anything else (`yes`/`no`/`1`/`0`/`on`/`off`,
/// empty, …) is rejected with a clear error before any file is touched.
fn validate_bool(value: &str) -> Result<String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Ok("true".to_string()),
        "false" => Ok("false".to_string()),
        _ => bail!("--bool needs `true` or `false` (case-insensitive); got `{value}`"),
    }
}

/// Splice a single replacement line over a location's key+value range.
pub(crate) fn splice_set(lines: &[String], loc: &KeyLoc, new_line: String) -> Vec<String> {
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
    mode: ValueMode,
) -> Result<Option<(usize, String)>> {
    let leaf = leaf_key(path);
    let parent = path.rsplit_once('.').map(|(p, _)| p);
    let value_field = value_field(tier, token, mode);

    for block in locator::scan_blocks(lines) {
        if block.subject != subject {
            continue;
        }
        let locs = locator::index_block(&block, lines, Path::new(""));
        match parent {
            Some(p) => {
                if let Some(ploc) = locs.iter().find(|l| l.path == p && l.subject == subject) {
                    ensure_extendable(lines, ploc, p)?;
                    let child_indent = ploc.indent + 2;
                    // End of the parent's subtree: directly after its last
                    // line, before trailing blanks / the next section's
                    // leading comment.
                    let idx = subtree_end(lines, ploc, block.body.end);
                    let line = format!("{}{}: {}", " ".repeat(child_indent), leaf, value_field);
                    return Ok(Some((idx, line)));
                }
            }
            None => {
                // Top-level: insert just before the block terminator.
                let line = format!("{}: {}", leaf, value_field);
                return Ok(Some((block.body.end, line)));
            }
        }
    }
    Ok(None)
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
#[allow(clippy::too_many_arguments)]
fn external_errata_compute(
    lines: &[String],
    locs: &[KeyLoc],
    spans: &[locator::SubjectSpan],
    subject: &str,
    path: &str,
    tier: u8,
    token: &str,
    mode: ValueMode,
) -> Result<Option<(usize, String)>> {
    let leaf = leaf_key(path);
    let parent = path.rsplit_once('.').map(|(p, _)| p);
    let subject_locs: Vec<&KeyLoc> = locs.iter().filter(|l| l.subject == subject).collect();
    let value_field = value_field(tier, token, mode);

    match parent {
        Some(p) => {
            let Some(span) = spans.iter().find(|s| s.name == subject) else {
                return Ok(None);
            };
            let Some(ploc) = subject_locs.iter().find(|l| l.path == p) else {
                return Ok(None);
            };
            ensure_extendable(lines, ploc, p)?;
            // End of the parent's subtree (bounded by the subject's body):
            // directly after its last line, before trailing blanks / the
            // next section's leading comment.
            let idx = subtree_end(lines, ploc, span.body.end);
            let line = format!("{}{}: {}", " ".repeat(ploc.indent + 2), leaf, value_field);
            Ok(Some((idx, line)))
        }
        None => {
            // Top-level within the subject body: insert after its last key
            // (or right after the subject line for an empty body).
            let Some(span) = spans.iter().find(|s| s.name == subject) else {
                return Ok(None);
            };
            let body_indent = span.child_indent.unwrap_or(span.indent + 2);
            let idx = subject_locs
                .iter()
                .map(|l| if l.is_block_scalar { l.value_end } else { l.key_line })
                .max()
                .map(|last| last + 1)
                .unwrap_or(span.key_line + 1);
            let line = format!("{}{}: {}", " ".repeat(body_indent), leaf, value_field);
            Ok(Some((idx, line)))
        }
    }
}


/// Placement of a `-p` parents-insert's new top-most missing section among
/// its siblings (mutually exclusive; the default with `-p` is [`Placement::Auto`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// Alphabetical among the existing siblings at the deepest existing
    /// ancestor; appended at the end when there are no siblings.
    Auto,
    /// Insert the new section above the anchor sibling path.
    Above(String),
    /// Insert the new section below the anchor sibling path — after its whole
    /// block (the anchor's subtree is skipped, so the section lands as a true
    /// sibling, not a child).
    Below(String),
}

/// Longest dotted-prefix of `path` already present in `locs` (`None` = even
/// the first component is missing — the new section is top-level). Only keys
/// belonging to `subject` count: paths are subject-relative, so another
/// subject's identically-named key in the same external file is never a
/// match (matching it silently wrote new keys into the wrong section).
fn deepest_prefix(locs: &[KeyLoc], subject: &str, path: &str) -> Option<String> {
    let comps: Vec<&str> = path.split('.').collect();
    for n in (1..comps.len()).rev() {
        let p = comps[..n].join(".");
        if locs.iter().any(|l| l.path == p && l.subject == subject) {
            return Some(p);
        }
    }
    None
}

/// Direct children of `prefix` (top-level keys when `prefix` is `None`), in
/// file order — strictly within `subject` (see [`deepest_prefix`]).
/// Direct children of `prefix` (top-level keys when `prefix` is `None`), in
/// file order — strictly within `subject` (see [`deepest_prefix`]). Returns
/// owned clones (call sites only read position/indent/path).
fn children_of(locs: &[KeyLoc], subject: &str, prefix: Option<&str>) -> Vec<KeyLoc> {
    locs.iter()
        .filter(|l| {
            if l.subject != subject {
                return false;
            }
            match prefix {
                Some(p) => l
                    .path
                    .strip_prefix(p)
                    .and_then(|rest| rest.strip_prefix('.'))
                    .is_some_and(|rest| !rest.contains('.')),
                None => !l.path.contains('.'),
            }
        })
        .cloned()
        .collect()
}

/// One-past-the-end index of the subtree rooted at `loc`, bounded by
/// `body_end` (the heredoc terminator / external subject span end).
///
/// The walk extends the subtree over deeper-indented real lines and
/// deeper-indented comments (a child's own nested keys and comments belong
/// to it), across interleaved blank lines — and stops at the first real
/// line or comment at `loc.indent` or shallower (the next section and its
/// leading comment). Blank lines never extend the subtree, so the returned
/// index sits directly after the subtree's LAST line: BEFORE any trailing
/// blank lines and before any following comment/section. Inserting at this
/// index therefore lands inside the block, never between the block and the
/// next section's comment.
fn subtree_end(lines: &[String], loc: &KeyLoc, body_end: usize) -> usize {
    let mut end = if loc.is_block_scalar {
        loc.value_end + 1
    } else {
        loc.key_line + 1
    };
    let mut idx = end;
    while idx < body_end {
        let line = &lines[idx];
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            idx += 1;
            continue;
        }
        let ind = line.len() - trimmed.len();
        if ind > loc.indent {
            end = idx + 1;
            idx += 1;
        } else {
            break;
        }
    }
    end
}

/// Index at which a line inserted BEFORE the anchor lands outside the
/// anchor's lead-in: walking back from `anchor_key_line`, blank lines and
/// comments belong to the gap between sections (often the NEXT section's
/// leading comment), so the insert goes right after the last real line
/// above the anchor — or at `body_start` when the anchor opens the body.
/// Mirrors [`subtree_end`]'s "never inside a trailing blank/comment run"
/// rule on the insertion-before side.
fn insert_before(lines: &[String], anchor_key_line: usize, body_start: usize) -> usize {
    let mut idx = anchor_key_line;
    while idx > body_start {
        let t = lines[idx - 1].trim_start();
        if t.is_empty() || t.starts_with('#') {
            idx -= 1;
        } else {
            break;
        }
    }
    idx
}

/// A `-p`/errata chain may only nest under an existing section that is open
/// for children: a mapping (a key with nested keys) or a null scalar (empty
/// inline value, optionally with a trailing comment). A key that already
/// carries a scalar / flow value (`key: "v"`, `key: {}`, `key: |`) cannot
/// gain nested children — refuse before anything is written.
fn ensure_extendable(lines: &[String], loc: &KeyLoc, path: &str) -> Result<()> {
    if loc.is_block_scalar {
        bail!(
            "`{path}` already holds a block-scalar value — it cannot gain nested children; edit the file directly"
        );
    }
    let line = &lines[loc.key_line];
    let rest = line.split_once(':').map(|x| x.1.trim()).unwrap_or("");
    if rest.is_empty() || rest.starts_with('#') {
        return Ok(()); // mapping or null — open for children
    }
    bail!(
        "`{path}` already holds a scalar value (`{rest}`) — it cannot gain nested children; edit the file directly"
    );
}

/// Compute a `-p` insertion for a path whose parent chain is (partly)
/// missing: returns `(insert index, chain lines)`. The missing components
/// nest under the deepest existing ancestor (or the subject's top level);
/// the leaf carries the 🔒 marker + token exactly like [`errata_compute`]
/// (or, in `plain`/`numeric` mode, the quoted literal / bare number — no
/// marker). `body_end` bounds the subject body; `top_indent` is a new
/// top-level section's indent.
#[allow(clippy::too_many_arguments)]
fn parents_compute(
    lines: &[String],
    locs: &[KeyLoc],
    subject: &str,
    path: &str,
    tier: u8,
    token: &str,
    mode: ValueMode,
    body_start: usize,
    body_end: usize,
    top_indent: usize,
    placement: &Placement,
) -> Result<(usize, Vec<String>)> {
    let comps: Vec<&str> = path.split('.').collect();
    let prefix = deepest_prefix(locs, subject, path);
    let base = prefix.as_ref().map(|p| p.split('.').count()).unwrap_or(0);
    let missing: Vec<&str> = comps[base..].to_vec();
    let head = missing[0];
    let value_field = value_field(tier, token, mode);

    // The chain must be able to nest under the deepest existing ancestor:
    // refuse (before any write) when that key already holds a scalar value.
    if let Some(p) = &prefix {
        let ploc = locs
            .iter()
            .find(|l| &l.path == p && l.subject == subject)
            .unwrap();
        ensure_extendable(lines, ploc, p)?;
    }

    // Anchor + nesting level. When a prefix exists the level is forced (the
    // chain must nest under it); otherwise the anchor's own level defines
    // where the new section nests. The anchor must belong to the subject
    // too — another section's identically-named key is never a valid anchor.
    let anchor = match placement {
        Placement::Above(a) | Placement::Below(a) => {
            let anchor = locs
                .iter()
                .find(|l| l.path == *a && l.subject == subject)
                .ok_or_else(|| {
                    anyhow::anyhow!("anchor `{a}` not found under subject `{subject}`")
                })?;
            if let Some(p) = &prefix {
                let want = locs
                    .iter()
                    .find(|l| &l.path == p && l.subject == subject)
                    .unwrap()
                    .indent
                    + 2;
                if anchor.indent != want {
                    bail!(
                        "anchor `{a}` is not a sibling of the new section `{head}` (expected indent {want}, found {})",
                        anchor.indent
                    );
                }
            }
            Some(anchor)
        }
        Placement::Auto => None,
    };
    let indent = match (&anchor, &prefix) {
        (Some(a), _) => a.indent,
        (None, Some(p)) => {
            locs.iter()
                .find(|l| &l.path == p && l.subject == subject)
                .unwrap()
                .indent
                + 2
        }
        (None, None) => top_indent,
    };

    // The chain: nested mappings; the leaf carries the marker + token.
    let mut chain: Vec<String> = Vec::with_capacity(missing.len());
    for (i, c) in missing.iter().enumerate() {
        let ind = " ".repeat(indent + 2 * i);
        if i + 1 < missing.len() {
            chain.push(format!("{ind}{c}:"));
        } else {
            chain.push(format!("{ind}{c}: {value_field}"));
        }
    }

    let idx = match placement {
        Placement::Above(_) => insert_before(lines, anchor.as_ref().unwrap().key_line, body_start),
        Placement::Below(_) => {
            let a = anchor.as_ref().unwrap();
            subtree_end(lines, a, body_end)
        }
        Placement::Auto => {
            let sibs = children_of(locs, subject, prefix.as_deref());
            // First existing sibling sorting after the new section name…
            let spot = {
                let mut names: Vec<&str> = sibs.iter().map(|s| leaf_key(&s.path)).collect();
                names.sort();
                names
                    .into_iter()
                    .find(|n| *n > head)
                    .and_then(|n| sibs.iter().find(|s| leaf_key(&s.path) == n))
            };
            match spot {
                Some(s) => insert_before(lines, s.key_line, body_start),
                // …else append after the last sibling (file order)…
                None => match sibs.last() {
                    Some(last) => subtree_end(lines, last, body_end),
                    // …else after the deepest existing ancestor's subtree…
                    None => match prefix.as_deref() {
                        Some(p) => subtree_end(
                            lines,
                            locs.iter()
                                .find(|l| l.path == p && l.subject == subject)
                                .unwrap(),
                            body_end,
                        ),
                        // …else at the very end of the subject body (before
                        // the heredoc terminator / external span end).
                        None => body_end,
                    },
                },
            }
        }
    };
    Ok((idx, chain))
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
            // The replacement lands exactly where the marked block sits, so
            // show the new value there — trailing it past the context window
            // reads as an append after the next section.
            if i == last_line {
                if let Some(nd) = new_display {
                    eprintln!("  ✚  {nd}");
                }
            }
        } else {
            eprintln!("      {line}");
        }
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

/// A line being shown in a preview: literal-mode lines (`--plain` /
/// `--numeric` / `--bool`) display their value as-is — the user explicitly
/// chose an exposed value; encrypted lines keep token redaction (structure-
/// only chain lines, e.g. `  parent:`, pass through unchanged either way).
/// A freshly *generated* value is never echoed: in literal (`--plain`) mode
/// the value is masked behind a `**generated**` placeholder.
fn display_line(line: &str, mode: ValueMode) -> String {
    match mode {
        ValueMode::Encrypt => redact_for_display(line),
        ValueMode::Plain | ValueMode::Numeric | ValueMode::Bool => line.to_string(),
    }
}

fn preview_line(line: &str, mode: ValueMode, generated: bool) -> String {
    if generated && matches!(mode, ValueMode::Plain) {
        mask_line_value(line)
    } else {
        display_line(line, mode)
    }
}

/// Replace a preview line's value with a `**generated**` placeholder — used
/// for freshly generated values, which must never reach stdout/stderr even
/// when stored as `--plain`. Structure-only lines (no `key: value` colon)
/// pass through unchanged.
fn mask_line_value(line: &str) -> String {
    match line.split_once(':') {
        Some((prefix, _)) => format!("{prefix}: \"{} **generated**\"", marker(0)),
        None => line.to_string(),
    }
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
    let new_line = format!("{}{}: \"⛔\"", " ".repeat(loc.indent), loc.raw_key);
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
pub(crate) fn refresh_store(subject: &str, edited: Option<&Path>) -> Result<()> {
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
    plain: bool,
    numeric: bool,
    boolean: bool,
    yes: bool,
    file: Option<&str>,
    parents: bool,
    placement: &Placement,
    generate_kind: Option<&str>,
    length: Option<usize>,
    if_missing: bool,
) -> Result<()> {
    // --generate mints the value here — it can never ride alongside a value
    // source (clap enforces too; this guards direct callers) or with the
    // typed/existing-token modes that only describe a provided value.
    if generate_kind.is_some() && (value.is_some() || from.is_some() || stdin_flag) {
        bail!("--generate is mutually exclusive with --value / --from / --stdin");
    }
    if generate_kind.is_some() && (encrypted || numeric || boolean) {
        bail!("--generate cannot be combined with --encrypted/--numeric/--bool");
    }
    if generate_kind.is_none() {
        if length.is_some() {
            bail!("--length requires --generate <KIND>");
        }
        if if_missing {
            bail!("--if-missing requires --generate <KIND>");
        }
    }
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

    // --if-missing decides generate-vs-skip BEFORE anything is minted: a leaf
    // that already holds a non-empty value (⚠ including the ⛔ shadow
    // sentinel — never clobbered) wins; only an empty value (`key: ""`) or a
    // missing leaf is generated.
    if if_missing {
        let already_set = match locs.first() {
            Some(loc) => {
                let lines = read_lines(&loc.file)?;
                !current_value_text(&lines, loc).trim().is_empty()
            }
            None => false,
        };
        if already_set {
            println!("skipped {subject} {path} — already set (--if-missing)");
            return Ok(());
        }
    }

    let mut generated_kind: Option<&'static str> = None;
    let plaintext = match generate_kind {
        Some(kind) => {
            let (v, canonical) = crate::generate::generate(kind, length)?;
            generated_kind = Some(canonical);
            v
        }
        None => acquire_value(value, from, stdin_flag)?,
    };
    // Validated (and trimmed/normalized) before anything is read/written, so
    // an invalid typed value can never reach a file.
    let literal_text = if numeric {
        Some(validate_numeric(&plaintext)?)
    } else if boolean {
        Some(validate_bool(&plaintext)?)
    } else {
        None
    };
    let mode = if numeric {
        ValueMode::Numeric
    } else if boolean {
        ValueMode::Bool
    } else if plain {
        ValueMode::Plain
    } else {
        ValueMode::Encrypt
    };

    if locs.is_empty() {
        return set_errata(
            subject,
            path,
            &plaintext,
            encrypted,
            mode,
            yes,
            forced.as_deref(),
            parents,
            placement,
            generated_kind,
        );
    }

    let loc = &locs[0];
    let mut lines = read_lines(&loc.file)?;
    let tier = existing_tier(&lines, loc);
    let token = if let Some(t) = &literal_text {
        t.clone()
    } else if encrypted || plain {
        plaintext.clone()
    } else {
        crate::crypto::encode_token(&plaintext, tier, &crate::settings::key()?)?
    };
    let new_line = compute_set_line(loc, tier, &token, mode);
    let new_display = if generated_kind.is_some() {
        // A minted value is never echoed — even in --plain mode the preview
        // shows a masked placeholder (the file itself gets the real value).
        format!(
            "{}: \"{} **generated**\"",
            " ".repeat(loc.indent) + &loc.raw_key,
            marker(0)
        )
    } else {
        match mode {
            ValueMode::Encrypt => format!(
                "{}: \"{} **redacted**\"",
                " ".repeat(loc.indent) + &loc.raw_key,
                marker(tier)
            ),
            // Plain/numeric/bool values are shown unredacted — the user chose to
            // store them in the open, so the preview must match the file.
            ValueMode::Plain | ValueMode::Numeric | ValueMode::Bool => format!(
                "{}: {}",
                " ".repeat(loc.indent) + &loc.raw_key,
                value_field(tier, &token, mode)
            ),
        }
    };

    eprintln!(
        "Update {} {} in {}:{}",
        subject,
        path,
        loc.file.display(),
        loc.key_line + 1
    );
    preview(
        &lines,
        loc.key_line,
        if loc.is_block_scalar {
            loc.value_end
        } else {
            loc.key_line
        },
        Some(&new_display),
    );
    // Hard safety gate (pre-write), same contract as the insert path: the
    // file must parse and the splice must change ONLY `<subject>.<path>`.
    let original_bytes = std::fs::read_to_string(&loc.file)?;
    let meta = crate::store::meta::read_meta(&crate::store::find_current_store()?)?;
    let is_external = meta.external.iter().any(|e| same_path(&e.path, &loc.file));
    let expected_field = value_field(tier, &token, mode);
    let (range, ext) = subject_body_range(&lines, &loc.file, is_external, subject)
        .map_err(|e| anyhow::anyhow!("{e} — no changes written"))?;
    let before_tree = parse_subject_tree(&lines, range, ext, subject).map_err(|e| {
        anyhow::anyhow!(
            "cannot verify update of `{subject} {path}` in {}: {e} — no changes written",
            loc.file.display()
        )
    })?;
    let after_lines = splice_set(&lines, loc, new_line.clone());
    let after_tree = match subject_body_range(&after_lines, &loc.file, is_external, subject)
        .map_err(|e| anyhow::anyhow!("{e} — no changes written"))
        .and_then(|(r, e2)| parse_subject_tree(&after_lines, r, e2, subject))
    {
        Ok(t) => t,
        Err(e) => bail!(
            "cannot verify update of `{subject} {path}` in {}: {e} — no changes written",
            loc.file.display()
        ),
    };
    if let Err(e) =
        assert_only_leaf_changed(&before_tree, &after_tree, subject, path, &expected_field)
    {
        bail!("{e} (in {})", loc.file.display());
    }
    if !yes && !confirm("Apply this change?")? {
        bail!("aborted — no changes written");
    }
    lines = after_lines;
    write_lines(&loc.file, &lines)?;
    post_write_verify_or_restore(
        &loc.file,
        &original_bytes,
        &before_tree,
        is_external,
        subject,
        path,
        &expected_field,
    )?;
    println!("updated {}:{}", loc.file.display(), loc.key_line + 1);
    if let Some(kind) = generated_kind {
        println!("generated {kind} ({} chars) → {subject} {path}", plaintext.chars().count());
    }
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
    mode: ValueMode,
    yes: bool,
    forced: Option<&Path>,
    parents: bool,
    placement: &Placement,
    generated_kind: Option<&'static str>,
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
            if !locator::external_subject_spans(&lines)
                .iter()
                .any(|s| s.name == subject)
            {
                continue;
            }
            lines
        } else {
            let lines = read_lines(f)?;
            if !locator::scan_blocks(&lines)
                .iter()
                .any(|b| b.subject == subject)
            {
                continue;
            }
            lines
        };
        let token = if encrypted || !matches!(mode, ValueMode::Encrypt) {
            plaintext.to_string()
        } else {
            crate::crypto::encode_token(plaintext, 0, &crate::settings::key()?)?
        };
        let computed: Result<(usize, Vec<String>)> = if parents {
            // -p routes every insert through the parents machinery — the
            // chain may be a single missing leaf, in which case placement
            // still positions it among its siblings.
            if is_external {
                let spans = locator::external_subject_spans(&lines);
                let Some(span) = spans.iter().find(|s| s.name == subject) else {
                    continue;
                };
                let locs = locator::index_external_lines(&lines, f);
                let top = span.child_indent.unwrap_or(span.indent + 2);
                parents_compute(
                    &lines,
                    &locs,
                    subject,
                    path,
                    0,
                    &token,
                    mode,
                    span.body.start,
                    span.body.end,
                    top,
                    placement,
                )
            } else {
                let blocks = locator::scan_blocks(&lines);
                let Some(block) = blocks.iter().find(|b| b.subject == subject) else {
                    continue;
                };
                let locs = locator::index_block(block, &lines, Path::new(""));
                parents_compute(
                    &lines,
                    &locs,
                    subject,
                    path,
                    0,
                    &token,
                    mode,
                    block.body.start,
                    block.body.end,
                    0,
                    placement,
                )
            }
        } else {
            let computed = if is_external {
                let spans = locator::external_subject_spans(&lines);
                let locs = locator::index_external_lines(&lines, f);
                external_errata_compute(&lines, &locs, &spans, subject, path, 0, &token, mode)
            } else {
                errata_compute(&lines, subject, path, 0, &token, mode)
            };
            match computed? {
                Some((idx, new_line)) => Ok((idx, vec![new_line])),
                None => bail!(
                    "subject `{subject}` exists in {} but parent of `{path}` was not found — pass -p/--parents (with --below <PATH>/--above <PATH>/--auto) or use `dc config setall`",
                    f.display()
                ),
            }
        };
        let (idx, new_lines) = computed.map_err(|e| anyhow::anyhow!("{e} (in {})", f.display()))?;
        eprintln!("Insert {subject} {path} into {}:{}", f.display(), idx + 1);
        // Hard safety gate (pre-write): the file must parse and the simulated
        // insert must land `<subject>.<path>` exactly at its path with
        // nothing else changed. Unresolvable parent / mis-placed insertion
        // ⇒ warn naming subject+path and exit non-zero WITHOUT writing.
        let original_bytes = std::fs::read_to_string(f)?;
        let expected_field = value_field(0, &token, mode);
        let (range, ext) = subject_body_range(&lines, f, is_external, subject)
            .map_err(|e| anyhow::anyhow!("{e} — no changes written"))?;
        let before_tree = parse_subject_tree(&lines, range, ext, subject).map_err(|e| {
            anyhow::anyhow!(
                "cannot verify placement of `{subject} {path}` in {}: {e} — no changes written",
                f.display()
            )
        })?;
        let mut after_lines = lines.clone();
        for (off, l) in new_lines.iter().enumerate() {
            after_lines.insert(idx + off, l.clone());
        }
        let after_tree = match subject_body_range(&after_lines, f, is_external, subject)
            .map_err(|e| anyhow::anyhow!("{e} — no changes written"))
            .and_then(|(r, e2)| parse_subject_tree(&after_lines, r, e2, subject))
        {
            Ok(t) => t,
            Err(e) => bail!(
                "cannot verify placement of `{subject} {path}` in {}: {e} — no changes written",
                f.display()
            ),
        };
        if let Err(e) =
            assert_only_leaf_changed(&before_tree, &after_tree, subject, path, &expected_field)
        {
            bail!("{e} (in {})", f.display());
        }
        preview_insert(&lines, idx, &new_lines, mode, generated_kind.is_some());
        if !yes && !confirm("Insert this entry?")? {
            bail!("aborted — no changes written");
        }
        for (off, l) in new_lines.iter().enumerate() {
            lines.insert(idx + off, l.clone());
        }
        write_lines(f, &lines)?;
        post_write_verify_or_restore(
            f,
            &original_bytes,
            &before_tree,
            is_external,
            subject,
            path,
            &expected_field,
        )?;
        eprintln!("inserted {subject} {path} at {}:{}", f.display(), idx + 1);
        if let Some(kind) = generated_kind {
            println!("generated {kind} ({} chars) → {subject} {path}", plaintext.chars().count());
        }
        refresh_store(subject, Some(f))?;
        return Ok(());
    }
    if let Some(f) = forced {
        bail!("subject `{subject}` not found in {}", f.display());
    }
    bail!("no `dc_yaml … {subject}` block or registered external subject `{subject}` found — use `dc config setall` or `dc load`")
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

    // The insert payload must be a valid YAML mapping — parse it ONCE up
    // front so a malformed payload can never reach a file.
    let parsed_input: serde_yaml::Value = serde_yaml::from_str(yaml_input)
        .map_err(|e| anyhow::anyhow!("setall input is not valid YAML: {e}"))?;
    let input_map = match parsed_input {
        serde_yaml::Value::Mapping(m) if !m.is_empty() => m,
        serde_yaml::Value::Null => bail!("setall input is empty"),
        _ => bail!("setall input must be a YAML mapping of `key: value` entries"),
    };
    let mut keys_with_values: Vec<(String, serde_yaml::Value)> = Vec::new();
    for (k, v) in input_map.iter() {
        let key = match k.as_str() {
            Some(s) => s.to_string(),
            None => bail!("setall input has a non-string key: {k:?}"),
        };
        keys_with_values.push((key, v.clone()));
    }

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
            if !locator::scan_blocks(&lines)
                .iter()
                .any(|b| b.subject == layer_subject)
                && !locator::index_external_lines(&lines, &f)
                    .iter()
                    .any(|l| l.subject == layer_subject)
            {
                continue;
            }
            lines
        } else {
            let lines = read_lines(&f)?;
            if !locator::scan_blocks(&lines)
                .iter()
                .any(|b| b.subject == layer_subject)
            {
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
        // Bound of the subject's body, so a `--below` insert skips the
        // anchor's whole subtree and never lands after trailing blanks or
        // the next section's leading comment.
        let body_end = if is_external {
            locator::external_subject_spans(&lines)
                .into_iter()
                .find(|s| s.name == layer_subject)
                .map(|s| s.body.end)
        } else {
            locator::scan_blocks(&lines)
                .into_iter()
                .find(|b| b.subject == layer_subject)
                .map(|b| b.body.end)
        };
        let indent = anchor.indent;
        let insert_at = if below {
            match body_end {
                Some(end) => subtree_end(&lines, anchor, end),
                None => continue,
            }
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
        // Hard safety gate (pre-write), same contract as config set: the
        // simulated insert must land every key at its path and change
        // nothing else in the subject's tree.
        let original_bytes = std::fs::read_to_string(&f)?;
        let (range, ext) = subject_body_range(&lines, &f, is_external, layer_subject)
            .map_err(|e| anyhow::anyhow!("{e} — no changes written"))?;
        let before_tree = parse_subject_tree(&lines, range, ext, layer_subject).map_err(|e| {
            anyhow::anyhow!(
                "cannot verify placement in {}: {e} — no changes written",
                f.display()
            )
        })?;
        let mut after_lines = lines.clone();
        for (off, l) in new_lines.iter().enumerate() {
            after_lines.insert(insert_at + off, l.clone());
        }
        let after_tree = match subject_body_range(&after_lines, &f, is_external, layer_subject)
            .map_err(|e| anyhow::anyhow!("{e} — no changes written"))
            .and_then(|(r, e2)| parse_subject_tree(&after_lines, r, e2, layer_subject))
        {
            Ok(t) => t,
            Err(e) => bail!(
                "cannot verify placement in {}: {e} — no changes written",
                f.display()
            ),
        };
        let parent = anchor.path.rsplit_once('.').map(|(p, _)| p);
        if let Err(e) = assert_only_keys_added(
            &before_tree,
            &after_tree,
            parent,
            &keys_with_values,
            layer_subject,
        ) {
            bail!("{e} (in {})", f.display());
        }
        preview_insert(&lines, insert_at, &new_lines, ValueMode::Plain, false);
        if !yes && !confirm("Insert this section?")? {
            bail!("aborted — no changes written");
        }
        let mut out = lines[..insert_at].to_vec();
        out.extend(new_lines);
        out.extend_from_slice(&lines[insert_at..]);
        lines = out;
        write_lines(&f, &lines)?;
        // Post-write gate: re-read from disk and re-assert; restore on mismatch.
        let verify = (|| -> Result<()> {
            let reread = read_lines(&f)?;
            let (r, e2) = subject_body_range(&reread, &f, is_external, layer_subject)?;
            let tree = parse_subject_tree(&reread, r, e2, layer_subject)?;
            assert_only_keys_added(
                &before_tree,
                &tree,
                parent,
                &keys_with_values,
                layer_subject,
            )
        })();
        if let Err(e) = verify {
            let restore = std::fs::write(&f, &original_bytes);
            bail!(
                "post-write verification failed: {e}; {}",
                match restore {
                    Ok(()) => "original file content restored".to_string(),
                    Err(rerr) => format!("RESTORE FAILED — file may be inconsistent ({rerr})"),
                }
            );
        }
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
        let err = confirm_with("Apply?", false, &mut "y\n".as_bytes())
            .unwrap_err()
            .to_string();
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
        let new_line = compute_set_line(&loc, tier, &token, ValueMode::Encrypt);
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
        let new_line = compute_set_line(&loc, 0, &token, ValueMode::Encrypt);
        let out = splice_set(&src, &loc, new_line);
        // The 4-line block is gone; zone survives directly after.
        assert!(out.iter().any(|l| l.starts_with("account_id: \"🔒 🔒:v1:")));
        assert!(out.iter().any(|l| l == "zone: z"));
        assert!(!out.iter().any(|l| l.contains("line-two")));
    }

    #[test]
    fn errata_inserts_under_existing_parent() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\naccess:\n  client_id: pub\nzone: z\nYAML");
        let token = crate::crypto::encode_token("BANANA", 0, &KEY).unwrap();
        let (idx, line) =
            errata_compute(&src, "cf", "access.foobar", 0, &token, ValueMode::Encrypt)
                .unwrap()
                .unwrap();
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
        let (idx, line) = errata_compute(&src, "cf", "api_token", 0, &token, ValueMode::Encrypt)
            .unwrap()
            .unwrap();
        assert_eq!(idx, 2); // the YAML terminator line index
        assert!(line.starts_with("api_token: \"🔒 🔒:v1:"));
    }

    // ---- --plain (literal, unmarked) writes ----

    #[test]
    fn set_plain_replaces_encrypted_line_without_marker() {
        let src = lines(
            "dc_yaml --no-bump cf <<'YAML'\naccess:\n  client_secret: \"🔒❗ old\"\nzone: z\nYAML",
        );
        let loc = locate(&src, "cf", "access.client_secret");
        let tier = existing_tier(&src, &loc);
        assert_eq!(tier, 1);
        let new_line = compute_set_line(&loc, tier, "literal-secret", ValueMode::Plain);
        let out = splice_set(&src, &loc, new_line);
        let line = out.iter().find(|l| l.contains("client_secret")).unwrap();
        assert_eq!(line, "  client_secret: \"literal-secret\"");
        assert!(
            !line.contains(PADLOCK) && !line.contains('❗'),
            "no marker on a --plain line"
        );
        assert!(!out.iter().any(|l| l.contains("old")), "stale token gone");
        assert!(out.iter().any(|l| l == "zone: z"), "siblings intact");
    }

    #[test]
    fn errata_plain_insert_has_no_marker() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\naccount_id: 1\nYAML");
        let (idx, line) = errata_compute(&src, "cf", "api_token", 0, "BANANA", ValueMode::Plain)
            .unwrap()
            .unwrap();
        assert_eq!(idx, 2);
        assert_eq!(line, "api_token: \"BANANA\"");
    }

    #[test]
    fn display_line_shows_literal_values_redacts_tokens() {
        // --plain / --numeric lines show their value verbatim — the user
        // chose an exposed value, and the preview must match the file.
        assert_eq!(
            display_line("  api_key: \"hunter2\"", ValueMode::Plain),
            "  api_key: \"hunter2\""
        );
        assert_eq!(display_line("  bip: 4", ValueMode::Numeric), "  bip: 4");
        // structure-only chain lines pass through in every mode
        assert_eq!(display_line("  jev:", ValueMode::Plain), "  jev:");
        assert_eq!(display_line("  jev:", ValueMode::Encrypt), "  jev:");
        // encrypted path unchanged (token-bearing lines redacted as before)
        let tok = crate::crypto::encode_token("shh", 1, &KEY).unwrap();
        let marked = format!("  k: \"{} {}\"", marker(1), tok);
        let shown = display_line(&marked, ValueMode::Encrypt);
        assert!(shown.contains("**redacted**"));
        assert!(!shown.contains("shh"));
    }

    // ---- --numeric validation ----

    #[test]
    fn validate_numeric_accepts_yaml_number_spellings() {
        for ok in [
            "4", "-3", "2.5", "1e3", "0", "+7", ".5", "4.", "-0.25", "6.02e23", " 8 ",
        ] {
            assert_eq!(
                validate_numeric(ok).unwrap(),
                ok.trim(),
                "`{ok}` must be accepted"
            );
        }
    }

    #[test]
    fn validate_numeric_rejects_non_numbers_and_non_finite() {
        for bad in [
            "abc",
            "",
            "NaN",
            "nan",
            "inf",
            "-Infinity",
            "1e",
            "e3",
            "4-5",
            "1 2",
            "0x10",
            "true",
        ] {
            let err = validate_numeric(bad).unwrap_err().to_string();
            assert!(err.contains("--numeric"), "`{bad}` must be rejected: {err}");
        }
    }

    #[test]
    fn numeric_leaf_writes_unquoted_scalar() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\nfoo:\n  bar:\n    bop: 2\nYAML\n");
        let (idx, line) = errata_compute(&src, "cf", "foo.bar.bip", 0, "4", ValueMode::Numeric)
            .unwrap()
            .unwrap();
        assert_eq!(idx, 4, "after bop, before the terminator");
        assert_eq!(
            line, "    bip: 4",
            "numeric leaves are bare scalars — no quotes, no marker"
        );
        // overwrite path: an existing encrypted line becomes a bare number
        let loc = locate(&src, "cf", "foo.bar.bop");
        let new_line = compute_set_line(&loc, 0, "-2.5", ValueMode::Numeric);
        assert_eq!(new_line, "    bop: -2.5");
    }

    // ---- --bool validation ----

    #[test]
    fn validate_bool_accepts_case_insensitive_true_false() {
        assert_eq!(validate_bool("true").unwrap(), "true");
        assert_eq!(validate_bool("TRUE").unwrap(), "true");
        assert_eq!(validate_bool("False").unwrap(), "false");
        assert_eq!(validate_bool(" false ").unwrap(), "false");
    }

    #[test]
    fn validate_bool_rejects_everything_else() {
        for bad in [
            "yes", "no", "1", "0", "on", "off", "", "truee", "Maybe", "tru",
        ] {
            let err = validate_bool(bad).unwrap_err().to_string();
            assert!(err.contains("--bool"), "`{bad}` must be rejected: {err}");
        }
    }

    #[test]
    fn bool_leaf_writes_unquoted_normalized_scalar() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\nfoo:\n  bar:\n    bip: \"4\"\nYAML\n");
        // set() normalizes via validate_bool before these helpers run.
        let (idx, line) = errata_compute(&src, "cf", "foo.bar.flag", 0, "true", ValueMode::Bool)
            .unwrap()
            .unwrap();
        assert_eq!(idx, 4);
        assert_eq!(
            line, "    flag: true",
            "bool leaves are lowercase bare scalars"
        );
        // overwrite path: an existing quoted line becomes a bare boolean
        let loc = locate(&src, "cf", "foo.bar.bip");
        let new_line = compute_set_line(&loc, 0, "false", ValueMode::Bool);
        assert_eq!(new_line, "    bip: false");
    }

    // ---- atomic write temp hygiene ----

    fn dc_tmp_leftovers(dir: &std::path::Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".dc-tmp-"))
            .collect()
    }

    #[test]
    fn write_lines_atomic_cleans_up_temp_on_failure() {
        let tmp = TempDir::new().unwrap();
        // renaming over a directory fails, exercising the mid-write error path
        let target = tmp.path().join("victim.envrc");
        std::fs::create_dir(&target).unwrap();
        assert!(
            write_lines_atomic(&target, &lines("a: 1\n")).is_err(),
            "rename over a directory must fail"
        );
        assert!(
            dc_tmp_leftovers(tmp.path()).is_empty(),
            "failed write leaked a temp sibling"
        );
    }

    #[test]
    fn write_lines_atomic_success_writes_and_leaves_no_temp() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("f.envrc");
        std::fs::write(&target, "old\n").unwrap();
        write_lines_atomic(&target, &lines("new: 1\n")).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new: 1\n");
        assert!(
            dc_tmp_leftovers(tmp.path()).is_empty(),
            "successful write leaked a temp sibling"
        );
    }

    // ---- external-file routing (dc load provenance) ----

    use crate::store::meta::ExternalFile;
    use tempfile::TempDir;

    fn ext(p: &std::path::Path) -> ExternalFile {
        ExternalFile {
            path: p.to_path_buf(),
            mtime: 0,
            size: 0,
            layer: None,
            if_missing: false,
        }
    }

    #[test]
    fn find_locs_in_spans_envrc_and_registered_external() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        std::fs::write(
            dir.join(".envrc"),
            "dc_yaml --no-bump app <<'YAML'\nhost: local\nYAML\n",
        )
        .unwrap();
        let extfile = dir.join("team.yaml");
        std::fs::write(
            &extfile,
            "app:\n  # db config\n  db:\n    host: \"🔒❗ tok\"\n",
        )
        .unwrap();
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
        assert!(
            locs2[0].file.ends_with(".envrc"),
            "{}",
            locs2[0].file.display()
        );
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
        let new_line = compute_set_line(loc, tier, &token, ValueMode::Encrypt);
        let out = splice_set(&lines, loc, new_line);

        assert_eq!(out.len(), 5, "line count unchanged");
        assert_eq!(out[0], "app:");
        assert_eq!(out[1], "  # tuning");
        assert_eq!(out[2], "  pool: 5");
        assert!(
            out[3].starts_with("  host: \""),
            "host line replaced in place"
        );
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
        let (idx, line) = external_errata_compute(
            &lines,
            &locs,
            &spans,
            "app",
            "api_key",
            0,
            &token,
            ValueMode::Encrypt,
        )
        .unwrap()
        .unwrap();
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

        // Parent path: inserted directly after db's last child line — before
        // the trailing comment that leads the next section (indent <= the
        // parent's stops the subtree walk).
        let (idx, line) = external_errata_compute(
            &src,
            &locs,
            &spans,
            "app",
            "db.port",
            0,
            &token,
            ValueMode::Encrypt,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            idx, 3,
            "right after `host: h`, before the trailing `# note` comment"
        );
        assert!(line.starts_with("    port: \"🔒 🔒:v1:"));

        // Subject with an empty body: insert directly under the subject line.
        let src2 = lines("app:\n  x: 1\nboot:\n");
        let spans2 = locator::external_subject_spans(&src2);
        let locs2 = locator::index_external_lines(&src2, file);
        let (idx2, line2) = external_errata_compute(
            &src2,
            &locs2,
            &spans2,
            "boot",
            "api_key",
            0,
            &token,
            ValueMode::Encrypt,
        )
        .unwrap()
        .unwrap();
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
        assert_eq!(
            jobs.len(),
            1,
            "only the plain (non-if-missing) heredoc survives"
        );
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
        assert_eq!(
            jobs[0].layer, "staging",
            "recorded load layer must be replayed"
        );

        // No layer recorded (incl. pre-options registry entries) -> base.
        let external2 = vec![ExternalFile {
            layer: None,
            if_missing: false,
            ..external[0].clone()
        }];
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

        let staging =
            crate::store::load_layer(&crate::store::layout::layer_path(&store, "app", "staging"))
                .unwrap();
        assert_eq!(
            staging.get("runner"),
            Some(&serde_yaml::Value::String("linux".into()))
        );
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
        let opts = crate::cmd::load::IngestOpts {
            if_missing: true,
            ..Default::default()
        };
        crate::cmd::load::load_files(&store, std::slice::from_ref(&seed), &opts, &|| {
            anyhow::bail!("no key needed for plain bodies")
        })
        .unwrap();

        // Store-side mutation of the seeded key (what --if-missing ownership means).
        let mutated: serde_yaml::Value = serde_yaml::from_str("token: rotated-by-store\n").unwrap();
        crate::cmd::yaml::apply(&store, "app", "base", &mutated, &[], None, false, None).unwrap();

        // A refresh triggered by editing any key of the subject must not revert it.
        refresh_store("app", None).unwrap();

        let base =
            crate::store::load_layer(&crate::store::layout::layer_path(&store, "app", "base"))
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

#[cfg(test)]
mod parents_tests {
    use super::*;
    use crate::envrc::locator::{index_block, scan_blocks};

    const KEY: [u8; 32] = [7u8; 32];

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(|l| l.to_string()).collect()
    }

    fn token_of(plain: &str) -> String {
        crate::crypto::encode_token(plain, 0, &KEY).unwrap()
    }

    fn locs_of(src: &[String]) -> Vec<KeyLoc> {
        let block = &scan_blocks(src)[0];
        index_block(block, src, Path::new("/x/.envrc"))
    }

    fn render(src: &[String], idx: usize, chain: &[String]) -> Vec<String> {
        let mut out = src[..idx].to_vec();
        out.extend_from_slice(chain);
        out.extend_from_slice(&src[idx..]);
        out
    }

    const FIXTURE: &str = "dc_yaml --no-bump cf <<'YAML'\nsendgrid:\n  api_key: \"plain s\"\nstripe:\n  key: k\nYAML\n";

    #[test]
    fn deep_missing_below_anchor_lands_after_whole_anchor_block() {
        let src = lines(FIXTURE);
        let locs = locs_of(&src);
        let tok = token_of("S3cr3t");
        let (idx, chain) = parents_compute(
            &src,
            &locs,
            "cf",
            "inference.jev.api_key",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            5,
            0,
            &Placement::Below("sendgrid".into()),
        )
        .unwrap();
        assert_eq!(idx, 3, "after sendgrid's subtree (stripe: comes next)");
        assert_eq!(chain[0], "inference:");
        assert_eq!(chain[1], "  jev:");
        assert!(chain[2].starts_with("    api_key: \"🔒 🔒:v1:"));
        let out = render(&src, idx, &chain);
        assert_eq!(out[3], "inference:");
        assert_eq!(out[6], "stripe:");
        assert_eq!(out.last().unwrap(), "YAML");
    }

    #[test]
    fn deep_missing_above_anchor_lands_before_anchor_line() {
        let src = lines(FIXTURE);
        let locs = locs_of(&src);
        let tok = token_of("v");
        let (idx, _) = parents_compute(
            &src,
            &locs,
            "cf",
            "a.b.c",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            5,
            0,
            &Placement::Above("stripe".into()),
        )
        .unwrap();
        assert_eq!(idx, 3, "directly before `stripe:`");
    }

    #[test]
    fn auto_alphabetical_first_middle_and_last() {
        let src = lines(FIXTURE);
        let locs = locs_of(&src);
        let tok = token_of("v");
        // before sendgrid (first sibling)
        let (idx, _) = parents_compute(
            &src,
            &locs,
            "cf",
            "aaa.key",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            5,
            0,
            &Placement::Auto,
        )
        .unwrap();
        assert_eq!(idx, 1);
        // between sendgrid and stripe (smoke sorts between them)
        let (idx, _) = parents_compute(
            &src,
            &locs,
            "cf",
            "smoke.key",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            5,
            0,
            &Placement::Auto,
        )
        .unwrap();
        assert_eq!(idx, 3);
        // appended at the end, before the terminator
        let (idx, _) = parents_compute(
            &src,
            &locs,
            "cf",
            "zzz.key",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            5,
            0,
            &Placement::Auto,
        )
        .unwrap();
        assert_eq!(idx, 5);
    }

    #[test]
    fn auto_with_no_siblings_appends_before_terminator() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\ntop: 1\nYAML\n");
        let locs = locs_of(&src);
        let tok = token_of("v");
        let (idx, chain) = parents_compute(
            &src,
            &locs,
            "cf",
            "x.a.b",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            3,
            0,
            &Placement::Auto,
        )
        .unwrap();
        assert_eq!(idx, 2, "before the YAML terminator");
        assert_eq!(chain[0], "x:");
        assert_eq!(chain[1], "  a:");
        assert!(chain[2].starts_with("    b: \"🔒 🔒:v1:"));
    }

    #[test]
    fn partial_parents_nest_under_deepest_existing_ancestor() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\ninference:\n  kv: k\nYAML\n");
        let locs = locs_of(&src);
        let tok = token_of("v");
        // jev < kv alphabetically → inserted before kv, nested under inference
        let (idx, chain) = parents_compute(
            &src,
            &locs,
            "cf",
            "inference.jev.api_key",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            3,
            0,
            &Placement::Auto,
        )
        .unwrap();
        assert_eq!(idx, 2);
        assert_eq!(chain[0], "  jev:");
        assert!(chain[1].starts_with("    api_key: \"🔒 🔒:v1:"));
        // anchor at the wrong level is rejected when a prefix exists
        let err = parents_compute(
            &src,
            &locs,
            "cf",
            "inference.jev.api_key",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            3,
            0,
            &Placement::Below("inference".into()),
        )
        .unwrap_err();
        assert!(err.to_string().contains("not a sibling"), "{err}");
        // sibling-level anchor under the prefix is honoured
        let (idx, chain) = parents_compute(
            &src,
            &locs,
            "cf",
            "inference.alpha.api_key",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            3,
            0,
            &Placement::Below("inference.kv".into()),
        )
        .unwrap();
        assert_eq!(idx, 3, "after kv's subtree");
        assert_eq!(chain[0], "  alpha:");
    }

    #[test]
    fn existing_parent_auto_positions_leaf_among_siblings() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\ngrp:\n  zed: z\nYAML\n");
        let locs = locs_of(&src);
        let tok = token_of("v");
        let (idx, chain) = parents_compute(
            &src,
            &locs,
            "cf",
            "grp.aaa",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            3,
            0,
            &Placement::Auto,
        )
        .unwrap();
        assert_eq!(idx, 2, "aaa < zed → before zed");
        assert_eq!(chain.len(), 1);
        assert!(chain[0].starts_with("  aaa: \"🔒 🔒:v1:"));
    }

    #[test]
    fn plain_parents_leaf_has_no_marker() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\ngrp:\n  zed: z\nYAML\n");
        let locs = locs_of(&src);
        let (idx, chain) = parents_compute(
            &src,
            &locs,
            "cf",
            "grp.aaa",
            0,
            "S3cr3t",
            ValueMode::Plain,
            1,
            3,
            0,
            &Placement::Auto,
        )
        .unwrap();
        assert_eq!(idx, 2);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0], "  aaa: \"S3cr3t\"");
    }

    #[test]
    fn numeric_parents_leaf_is_a_bare_scalar() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\ngrp:\n  zed: z\nYAML\n");
        let locs = locs_of(&src);
        let (idx, chain) = parents_compute(
            &src,
            &locs,
            "cf",
            "grp.aaa",
            0,
            "42",
            ValueMode::Numeric,
            1,
            3,
            0,
            &Placement::Auto,
        )
        .unwrap();
        assert_eq!(idx, 2);
        assert_eq!(chain.len(), 1);
        assert_eq!(
            chain[0], "  aaa: 42",
            "numeric leaves are unquoted YAML numbers"
        );
    }

    #[test]
    fn anchor_missing_in_subject_is_a_clear_error() {
        let src = lines(FIXTURE);
        let locs = locs_of(&src);
        let tok = token_of("v");
        let err = parents_compute(
            &src,
            &locs,
            "cf",
            "x.y.z",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            5,
            0,
            &Placement::Below("mailgun".into()),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("anchor `mailgun` not found"),
            "{err}"
        );
    }

    #[test]
    fn external_file_deep_chain_below_anchor() {
        use crate::envrc::locator::{external_subject_spans, index_external_lines};
        let src = lines("app:\n  name: noizu\n");
        let spans = external_subject_spans(&src);
        let locs = index_external_lines(&src, Path::new("/x/team.yaml"));
        let tok = token_of("v");
        let span = spans.iter().find(|s| s.name == "app").unwrap();
        let top = span.child_indent.unwrap_or(span.indent + 2);
        let (idx, chain) = parents_compute(
            &src,
            &locs,
            "app",
            "db.conn.host",
            0,
            &tok,
            ValueMode::Encrypt,
            span.body.start,
            span.body.end,
            top,
            &Placement::Below("name".into()),
        )
        .unwrap();
        assert_eq!(idx, 2, "after name's subtree (span end)");
        assert_eq!(chain[0], "  db:");
        assert_eq!(chain[1], "    conn:");
        assert!(chain[2].starts_with("      host: \"🔒 🔒:v1:"));
    }

    // -- full-path details: quoted keys, non-mapping intermediates ----------

    #[test]
    fn quoted_keys_are_indexed_unquoted_with_raw_text_preserved() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\n\"anthropic\":\n  \"trr\": old\nYAML\n");
        let locs = locs_of(&src);
        let section = locs
            .iter()
            .find(|l| l.path == "anthropic")
            .expect("quoted section indexed");
        assert_eq!(section.raw_key, "\"anthropic\"");
        let leaf = locs
            .iter()
            .find(|l| l.path == "anthropic.trr")
            .expect("quoted leaf indexed");
        assert_eq!(leaf.raw_key, "\"trr\"");
        // Rewrites keep the quotes.
        let new_line = compute_set_line(leaf, 0, "tok", ValueMode::Plain);
        assert_eq!(new_line, "  \"trr\": \"tok\"");
    }

    #[test]
    fn chain_nests_inside_quoted_intermediate_section() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\n\"anthropic\":\n  model: m\nYAML\n");
        let locs = locs_of(&src);
        let tok = token_of("v");
        // keys < model alphabetically → chain inserted before the model child
        let (idx, chain) = parents_compute(
            &src,
            &locs,
            "cf",
            "anthropic.keys.trr",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            3,
            0,
            &Placement::Auto,
        )
        .unwrap();
        assert_eq!(idx, 2);
        assert_eq!(chain[0], "  keys:");
        assert!(chain[1].starts_with("    trr: \"🔒 🔒:v1:"));
    }

    #[test]
    fn scalar_intermediate_chain_is_refused() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\ngrp:\n  keys: \"v\"\nYAML\n");
        let locs = locs_of(&src);
        let tok = token_of("v");
        let err = parents_compute(
            &src,
            &locs,
            "cf",
            "grp.keys.trr",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            3,
            0,
            &Placement::Auto,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("already holds a scalar value"),
            "{err}"
        );
    }

    #[test]
    fn block_scalar_intermediate_chain_is_refused() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\ngrp:\n  keys: |\n    text\nYAML\n");
        let locs = locs_of(&src);
        let tok = token_of("v");
        let err = parents_compute(
            &src,
            &locs,
            "cf",
            "grp.keys.trr",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            4,
            0,
            &Placement::Auto,
        )
        .unwrap_err();
        assert!(err.to_string().contains("block-scalar value"), "{err}");
    }

    #[test]
    fn null_intermediate_chain_extends_into_mapping() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\ngrp:\n  keys:\nYAML\n");
        let locs = locs_of(&src);
        let tok = token_of("v");
        let (idx, chain) = parents_compute(
            &src,
            &locs,
            "cf",
            "grp.keys.trr",
            0,
            &tok,
            ValueMode::Encrypt,
            1,
            3,
            0,
            &Placement::Auto,
        )
        .unwrap();
        assert_eq!(idx, 3, "right after the null key");
        assert_eq!(chain.len(), 1);
        assert!(chain[0].starts_with("    trr: \"🔒 🔒:v1:"));
    }

    #[test]
    fn errata_under_scalar_parent_is_refused() {
        let src = lines("dc_yaml --no-bump cf <<'YAML'\ngrp:\n  keys: \"v\"\nYAML\n");
        let err =
            errata_compute(&src, "cf", "grp.keys.trr", 0, "tok", ValueMode::Plain).unwrap_err();
        assert!(
            err.to_string().contains("already holds a scalar value"),
            "{err}"
        );
    }
}
