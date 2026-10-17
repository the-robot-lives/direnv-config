//! Scanner for `dc_yaml … <<'YAML' … YAML` heredoc blocks in `.envrc*` files.
//!
//! Produces a path→line index (indentation-aware, block-scalar aware) so callers
//! can locate and precisely rewrite the source line(s) for a given subject/path.

use std::ops::Range;
use std::path::{Path, PathBuf};

/// One `dc_yaml` heredoc block within a file.
#[derive(Debug, Clone)]
pub struct HeredocBlock {
    pub subject: String,
    pub layer: Option<String>,
    /// 0-indexed line of the `dc_yaml … <<'YAML'` opener.
    #[allow(dead_code)]
    pub start_line: usize,
    /// 0-indexed body range (exclusive end = the terminator line).
    pub body: Range<usize>,
}

/// A located key within a heredoc block.
#[derive(Debug, Clone)]
pub struct KeyLoc {
    pub subject: String,
    #[allow(dead_code)]
    pub layer: Option<String>,
    pub path: String,
    /// The key as written on the line (quoted keys keep their quotes), so
    /// rewrites of an existing line preserve its style.
    pub raw_key: String,
    pub file: PathBuf,
    /// 0-indexed line where `key:` appears.
    pub key_line: usize,
    /// 0-indexed inclusive range of line(s) holding the value.
    pub value_start: usize,
    pub value_end: usize,
    pub is_block_scalar: bool,
    pub indent: usize,
}

/// Parse the args of a `dc_yaml` opener into `(flags, subject, layer)`.
fn parse_opener_args(args: &str) -> (Vec<String>, Option<String>, Option<String>) {
    let toks: Vec<&str> = args.split_whitespace().collect();
    let mut flags = Vec::new();
    let mut subject = None;
    let mut layer = None;
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i];
        if t == "--layer" {
            flags.push(t.to_string());
            if let Some(v) = toks.get(i + 1) {
                layer = Some(v.to_string());
                flags.push(v.to_string());
                i += 2;
                continue;
            }
        } else if t == "--replace-key" {
            flags.push(t.to_string());
            if let Some(v) = toks.get(i + 1) {
                flags.push(v.to_string());
                i += 2;
                continue;
            }
        } else if t.starts_with("--") {
            flags.push(t.to_string());
        } else if subject.is_none() {
            subject = Some(t.to_string());
        }
        i += 1;
    }
    (flags, subject, layer)
}

/// Match a `dc_yaml … <<'YAML'` opener line, returning its raw arg string.
fn match_opener(line: &str) -> Option<String> {
    let t = line.trim_start();
    let rest = t.strip_prefix("dc_yaml")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    // Split on the heredoc operator `<<`.
    let idx = rest.find("<<")?;
    let args = rest[..idx].trim();
    // Validate the heredoc tail is a YAML terminator declaration.
    let tail = rest[idx + 2..].trim();
    let tail = tail.trim_start_matches('-').trim();
    let tail = tail.trim_matches(|c| c == '\'' || c == '"');
    if tail != "YAML" {
        return None;
    }
    Some(args.to_string())
}

fn is_terminator(line: &str) -> bool {
    line.trim() == "YAML"
}

/// The plain (unquoted) key charset: alphanumeric, `_`, `-`, `.`.
fn is_plain_key(k: &str) -> bool {
    !k.is_empty()
        && k.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// Strip matching surrounding quotes from a key when the inner text is a
/// plain key (`"foo"` → `foo`). Returns the key unchanged when it is already
/// plain, and `None` for anything that is neither.
fn unquote_key(raw: &str) -> Option<String> {
    if is_plain_key(raw) {
        return Some(raw.to_string());
    }
    let ch: Vec<char> = raw.chars().collect();
    if ch.len() >= 2 && (ch[0] == '"' || ch[0] == '\'') && ch[ch.len() - 1] == ch[0] {
        let inner: String = ch[1..ch.len() - 1].iter().collect();
        if is_plain_key(&inner) {
            return Some(inner);
        }
    }
    None
}

/// Parse a `key: value` line, returning `(key, raw_key, rest_after_colon)`.
/// `key` is the addressable (unquoted) name; quoted keys (`"foo":`,
/// `'foo':`) are unquoted for path addressing while `raw_key` keeps the
/// as-written text so rewrites preserve the quoting.
fn parse_key_line(trimmed: &str) -> Option<(String, String, String)> {
    let pos = trimmed.find(':')?;
    let raw = &trimmed[..pos];
    let key = unquote_key(raw)?;
    Some((key, raw.to_string(), trimmed[pos + 1..].to_string()))
}

/// Scan all `dc_yaml` heredoc blocks in `lines`.
// <REMOVED UUID HERE> scan_blocks :: Scan all `dc_yaml` heredoc blocks in `lines`.
pub fn scan_blocks(lines: &[String]) -> Vec<HeredocBlock> {
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if let Some(args) = match_opener(&lines[i]) {
            let (_flags, subject, layer) = parse_opener_args(&args);
            // Find the terminator.
            let mut j = i + 1;
            while j < lines.len() && !is_terminator(&lines[j]) {
                j += 1;
            }
            if let Some(subject) = subject {
                blocks.push(HeredocBlock {
                    subject,
                    layer,
                    start_line: i,
                    body: (i + 1)..j,
                });
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    blocks
}

/// A scanned `key:` line within a YAML-ish region: indentation, block-scalar
/// span, and line numbers — everything needed to build a [`KeyLoc`] later.
#[derive(Debug, Clone)]
struct ScannedKey {
    indent: usize,
    key: String,
    /// Key text as written (quoted keys keep their quotes).
    raw: String,
    key_line: usize,
    value_start: usize,
    value_end: usize,
    is_block_scalar: bool,
}

/// Scan `lines[range]` for `key:` lines, skipping comments/blanks and jumping
/// over block-scalar (`|`/`>`) continuation lines. This is the line-level
/// grammar shared by heredoc blocks and external plain-YAML files.
fn scan_keys(lines: &[String], range: Range<usize>) -> Vec<ScannedKey> {
    let mut keys = Vec::new();
    let mut i = range.start;
    while i < range.end {
        let line = &lines[i];
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }
        let indent = line.len() - trimmed.len();
        let Some((key, raw, rest)) = parse_key_line(trimmed) else {
            i += 1;
            continue;
        };

        let rest_trim = rest.trim();
        if rest_trim.starts_with('|') || rest_trim.starts_with('>') {
            // Block scalar: value spans following more-indented lines.
            let mut j = i + 1;
            let mut last_nonblank = i;
            while j < range.end {
                let l = &lines[j];
                let lt = l.trim_start();
                if lt.is_empty() {
                    j += 1;
                    continue;
                }
                let ind2 = l.len() - lt.len();
                if ind2 > indent {
                    last_nonblank = j;
                    j += 1;
                } else {
                    break;
                }
            }
            keys.push(ScannedKey {
                indent,
                key,
                raw,
                key_line: i,
                value_start: i + 1,
                value_end: last_nonblank,
                is_block_scalar: true,
            });
            i = j;
        } else {
            keys.push(ScannedKey {
                indent,
                key,
                raw,
                key_line: i,
                value_start: i,
                value_end: i,
                is_block_scalar: false,
            });
            i += 1;
        }
    }
    keys
}

/// Index every key within a block to its line location.
// <REMOVED UUID HERE> index_block :: Index every key within a block to its line location.
pub fn index_block(block: &HeredocBlock, lines: &[String], file: &Path) -> Vec<KeyLoc> {
    let mut locs = Vec::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    for k in scan_keys(lines, block.body.clone()) {
        // Pop deeper-or-equal entries off the stack.
        while let Some((ind, _)) = stack.last() {
            if *ind >= k.indent {
                stack.pop();
            } else {
                break;
            }
        }
        stack.push((k.indent, k.key.clone()));
        locs.push(KeyLoc {
            subject: block.subject.clone(),
            layer: block.layer.clone(),
            path: stack
                .iter()
                .map(|(_, k)| k.as_str())
                .collect::<Vec<_>>()
                .join("."),
            raw_key: k.raw.clone(),
            file: file.to_path_buf(),
            key_line: k.key_line,
            value_start: k.value_start,
            value_end: k.value_end,
            is_block_scalar: k.is_block_scalar,
            indent: k.indent,
        });
    }
    locs
}

/// Read a file and return all located keys across its heredoc blocks.
// <REMOVED UUID HERE> index_file :: Read a file and return all located keys across its heredoc blocks.
pub fn index_file(path: &Path) -> std::io::Result<Vec<KeyLoc>> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut out = Vec::new();
    for block in scan_blocks(&lines) {
        out.extend(index_block(&block, &lines, path));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// External plain-YAML files (dc load)
// ---------------------------------------------------------------------------

/// Reserved per-subject routing key stripped during ingest and not indexed.
pub const LAYER_KEY: &str = "_dc_layer";

/// One subject entry (top-level key, or key under a `subjects:` wrapper) in an
/// external plain-YAML file, with its body's line span.
#[derive(Debug, Clone)]
pub struct SubjectSpan {
    pub name: String,
    /// 0-indexed line of the `subject:` key.
    pub key_line: usize,
    /// Indent of the subject key (wrapper files: one level deeper).
    pub indent: usize,
    /// Line span of the subject's body (after the key, up to the next subject).
    pub body: Range<usize>,
    /// Minimum indent of the body's keys (the subject-body indentation).
    pub child_indent: Option<usize>,
}

/// Split scanned keys into `(subject key, child-index range)` pairs.
///
/// Handles the optional `subjects:` wrapper: when the file's single root key is
/// `subjects`, subject names live one indent level deeper.
fn split_subjects(keys: &[ScannedKey]) -> Vec<(&ScannedKey, Range<usize>)> {
    if keys.is_empty() {
        return Vec::new();
    }
    let root_indent = keys.iter().map(|k| k.indent).min().unwrap();
    let root_count = keys.iter().filter(|k| k.indent == root_indent).count();
    let (subject_indent, first_subject_idx) = if root_count == 1
        && keys[0].indent == root_indent
        && keys[0].key == "subjects"
    {
        // Wrapper form: subjects sit at the next indent level.
        match keys.iter().position(|k| k.indent > root_indent) {
            Some(p) => (keys[p].indent, p),
            None => return Vec::new(),
        }
    } else {
        (root_indent, 0)
    };

    let mut out = Vec::new();
    let mut current: Option<(&ScannedKey, usize)> = None;
    for (idx, k) in keys.iter().enumerate() {
        if k.indent == subject_indent && idx >= first_subject_idx {
            if let Some((subject, start)) = current.take() {
                out.push((subject, start..idx));
            }
            current = Some((k, idx + 1));
        }
    }
    if let Some((subject, start)) = current {
        out.push((subject, start..keys.len()));
    }
    out
}

/// Index every key of every subject in an external plain-YAML file's lines,
/// producing the same [`KeyLoc`] shape as [`index_block`] — paths relative to
/// the subject body, indents/lines absolute within the file, so splices stay
/// comment-preserving. The reserved `_dc_layer` routing key is not indexed.
pub fn index_external_lines(lines: &[String], file: &Path) -> Vec<KeyLoc> {
    let keys = scan_keys(lines, 0..lines.len());
    let mut locs = Vec::new();
    for (subject, children) in split_subjects(&keys) {
        let mut stack: Vec<(usize, String)> = Vec::new();
        for k in &keys[children] {
            while let Some((ind, _)) = stack.last() {
                if *ind >= k.indent {
                    stack.pop();
                } else {
                    break;
                }
            }
            stack.push((k.indent, k.key.clone()));
            // `_dc_layer` at the top of a subject body is routing metadata.
            if stack.len() == 1 && k.key == LAYER_KEY {
                continue;
            }
            locs.push(KeyLoc {
                subject: subject.key.clone(),
                layer: None,
                path: stack
                    .iter()
                    .map(|(_, k)| k.as_str())
                    .collect::<Vec<_>>()
                    .join("."),
                raw_key: k.raw.clone(),
                file: file.to_path_buf(),
                key_line: k.key_line,
                value_start: k.value_start,
                value_end: k.value_end,
                is_block_scalar: k.is_block_scalar,
                indent: k.indent,
            });
        }
    }
    locs
}

/// Subject spans of an external plain-YAML file (including subjects whose
/// bodies hold no keys), used for errata inserts and subject discovery.
pub fn external_subject_spans(lines: &[String]) -> Vec<SubjectSpan> {
    let keys = scan_keys(lines, 0..lines.len());
    let pairs = split_subjects(&keys);
    let mut spans = Vec::new();
    for (i, (subject, children)) in pairs.iter().enumerate() {
        let body_end = pairs
            .get(i + 1)
            .map(|(next, _)| next.key_line)
            .unwrap_or(lines.len());
        spans.push(SubjectSpan {
            name: subject.key.clone(),
            key_line: subject.key_line,
            indent: subject.indent,
            body: (subject.key_line + 1)..body_end,
            child_indent: keys
                .get(children.clone())
                .and_then(|ks| ks.iter().map(|k| k.indent).min()),
        });
    }
    spans
}

/// Read an external plain-YAML file and return all located keys.
pub fn index_external(path: &Path) -> std::io::Result<Vec<KeyLoc>> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    Ok(index_external_lines(&lines, path))
}

/// Discover `.envrc*` files in `dir` (non-recursive).
// <REMOVED UUID HERE> envrc_files :: Discover `.envrc*` files in `dir` (non-recursive).
pub fn envrc_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name == ".envrc" || name.starts_with(".envrc.") {
                files.push(e.path());
            }
        }
    }
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(|l| l.to_string()).collect()
    }

    #[test]
    fn parses_opener_variants() {
        assert_eq!(match_opener("dc_yaml --no-bump cf <<'YAML'").as_deref(), Some("--no-bump cf"));
        assert_eq!(match_opener("  dc_yaml cf --layer secrets <<'YAML'").as_deref(), Some("cf --layer secrets"));
        assert_eq!(match_opener("dc_yaml cf <<-\"YAML\"").as_deref(), Some("cf"));
        assert!(match_opener("echo dc_yaml cf").is_none());
        assert!(match_opener("dc_yamlx cf <<'YAML'").is_none());
    }

    #[test]
    fn parses_subject_and_layer() {
        let (_f, s, l) = parse_opener_args("--no-bump cf --layer secrets");
        assert_eq!(s.as_deref(), Some("cf"));
        assert_eq!(l.as_deref(), Some("secrets"));
        let (_f, s, l) = parse_opener_args("--no-bump infra");
        assert_eq!(s.as_deref(), Some("infra"));
        assert_eq!(l, None);
    }

    #[test]
    fn indexes_nested_single_line_keys() {
        let src = lines(
            "dc_yaml --no-bump cf <<'YAML'\naccount_id: 123\naccess:\n  client_id: pub\n  client_secret: \"🔒 shh\"\nYAML\n",
        );
        let blocks = scan_blocks(&src);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].subject, "cf");
        let locs = index_block(&blocks[0], &src, Path::new("/x/.envrc"));
        let by_path = |p: &str| locs.iter().find(|l| l.path == p).unwrap();
        assert_eq!(by_path("account_id").key_line, 1);
        assert_eq!(by_path("access.client_id").key_line, 3);
        let cs = by_path("access.client_secret");
        assert_eq!(cs.key_line, 4);
        assert!(!cs.is_block_scalar);
        assert_eq!(cs.indent, 2);
    }

    #[test]
    fn indexes_block_scalar_range() {
        let src = lines(
            "dc_yaml --no-bump cf <<'YAML'\naccount_id: |\n  🔒 line-one\n  line-two\n\n  line-three\nzone: z\nYAML\n",
        );
        let blocks = scan_blocks(&src);
        let locs = index_block(&blocks[0], &src, Path::new("/x/.envrc"));
        let acct = locs.iter().find(|l| l.path == "account_id").unwrap();
        assert!(acct.is_block_scalar);
        assert_eq!(acct.key_line, 1);
        assert_eq!(acct.value_start, 2);
        // value_end is the last non-blank indented line (line index 5: "  line-three")
        assert_eq!(acct.value_end, 5);
        // the next sibling key is found after the block
        let zone = locs.iter().find(|l| l.path == "zone").unwrap();
        assert_eq!(zone.key_line, 6);
        assert!(!zone.is_block_scalar);
    }

    #[test]
    fn multiple_blocks_in_one_file() {
        let src = lines(
            "dc_yaml --no-bump infra <<'YAML'\nroot: /x\nYAML\n\ndc_yaml --no-bump cf --layer secrets <<'YAML'\napi_token: \"🔒 t\"\nYAML\n",
        );
        let blocks = scan_blocks(&src);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[1].subject, "cf");
        assert_eq!(blocks[1].layer.as_deref(), Some("secrets"));
        let locs = index_block(&blocks[1], &src, Path::new("/x/.envrc"));
        assert_eq!(locs.iter().find(|l| l.path == "api_token").unwrap().subject, "cf");
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let src = lines(
            "dc_yaml --no-bump cf <<'YAML'\n# a comment\naccount_id: 123\n\n# another\nzone: z\nYAML\n",
        );
        let blocks = scan_blocks(&src);
        let locs = index_block(&blocks[0], &src, Path::new("/x/.envrc"));
        assert_eq!(locs.len(), 2);
        assert_eq!(locs.iter().find(|l| l.path == "zone").unwrap().key_line, 5);
    }

    // --- external plain-YAML files ---

    #[test]
    fn indexes_external_subjects_and_nested_keys() {
        let src = lines(
            "# header comment\ncluster:\n  name: noizu\n  access:\n    client_id: pub\ninfra:\n  region: us-east-1\n",
        );
        let locs = index_external_lines(&src, Path::new("/x/config/dc.yaml"));
        let by = |s: &str, p: &str| locs.iter().find(|l| l.subject == s && l.path == p).unwrap();
        assert_eq!(by("cluster", "name").key_line, 2);
        assert_eq!(by("cluster", "name").indent, 2);
        assert_eq!(by("cluster", "access.client_id").key_line, 4);
        assert_eq!(by("cluster", "access.client_id").indent, 4);
        assert_eq!(by("infra", "region").key_line, 6);
        // paths are relative to the subject body, never include the subject
        let infra_paths: Vec<&str> =
            locs.iter().filter(|l| l.subject == "infra").map(|l| l.path.as_str()).collect();
        assert_eq!(infra_paths, vec!["region"]);
    }

    #[test]
    fn indexes_external_subjects_wrapper() {
        let src = lines(
            "subjects:\n  cluster:\n    name: noizu\n  infra:\n    region: us-east-1\n",
        );
        let locs = index_external_lines(&src, Path::new("/x/config/dc.yaml"));
        assert_eq!(
            locs.iter()
                .find(|l| l.subject == "cluster" && l.path == "name")
                .unwrap()
                .key_line,
            2
        );
        assert_eq!(locs.len(), 2);
    }

    #[test]
    fn external_index_handles_block_scalars() {
        let src = lines(
            "cluster:\n  token: |\n    🔒 line-one\n    line-two\n  name: noizu\n",
        );
        let locs = index_external_lines(&src, Path::new("/x/config/dc.yaml"));
        let tok = locs.iter().find(|l| l.path == "token").unwrap();
        assert!(tok.is_block_scalar);
        assert_eq!(tok.value_start, 2);
        assert_eq!(tok.value_end, 3);
        // the sibling after the block is indexed at the right line
        assert_eq!(locs.iter().find(|l| l.path == "name").unwrap().key_line, 4);
    }

    #[test]
    fn external_index_skips_dc_layer() {
        let src = lines("cluster:\n  _dc_layer: local\n  name: noizu\n");
        let locs = index_external_lines(&src, Path::new("/x/config/dc.yaml"));
        assert!(locs.iter().all(|l| l.path != "_dc_layer"));
        // nested keys that merely start with the prefix are still indexed
        let src = lines("cluster:\n  dc_layerish: v\n");
        let locs = index_external_lines(&src, Path::new("/x/config/dc.yaml"));
        assert!(locs.iter().any(|l| l.path == "dc_layerish"));
    }

    #[test]
    fn external_spans_cover_empty_bodied_subjects() {
        let src = lines("cluster: {}\ninfra:\n  region: us-east-1\n");
        let spans = external_subject_spans(&src);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].name, "cluster");
        assert_eq!(spans[0].child_indent, None);
        assert_eq!(spans[1].name, "infra");
        assert_eq!(spans[1].child_indent, Some(2));
        assert_eq!(spans[1].body, 2..3);
    }
}
