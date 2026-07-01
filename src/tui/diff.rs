/// ─── Diff Rendering Pipeline ──────────────────────────────────────────────
///
/// Implements V1 from docs/audit-tui-visual-fidelity.md: produce and render
/// unified diffs with gutter (line numbers) and syntect syntax highlighting.
/// Two public pure functions:
///   - compute_unified_diff: generate unified diff text from before/after
///   - render_diff: parse unified diff text and produce styled ratatui Lines

#[allow(unused_imports)]
use ratatui::style::{Color, Modifier, Style};
#[allow(unused_imports)]
use ratatui::text::{Line, Span};

/// Maximum lines in a single hunk before truncation kicks in.
const MAX_HUNK_LINES: usize = 500;
/// Maximum total diff lines across all hunks.
const MAX_TOTAL_LINES: usize = 2000;
/// Maximum diff body lines rendered inside a permission dialog preview.
pub const MAX_DIALOG_PREVIEW_LINES: usize = 20;

/// One parsed hunk of a unified diff.
struct Hunk {
    /// New-file line number where the hunk starts (extracted from
    /// `@@ -X,Y +A,B @@` — the A value).
    new_start: usize,
    /// Raw `@@ ...` header line (kept for rendering).
    header: String,
    /// Body lines (skipping the `@@` header itself).
    lines: Vec<ParsedLine>,
}

/// A single line inside a hunk.
#[allow(dead_code)] // FileHeader only fires on multi-file diffs
enum ParsedLine {
    Add(String),
    Del(String),
    Context(String),
    /// `--- a/path` / `+++ b/path` — pre-hunk metadata.
    FileHeader(String),
}

/// Generate a unified diff text from `old` and `new` content for `path`.
///
/// Returns the diff with `--- a/{path}` / `+++ b/{path}` file headers so the
/// markdown renderer detects it. Binary inputs (containing NUL bytes) yield
/// the literal sentinel `"[binary file changed]"`.
pub fn compute_unified_diff(old: &str, new: &str, path: &str) -> String {
    if is_binary(old) || is_binary(new) {
        return "[binary file changed]".to_string();
    }

    use similar::TextDiff;
    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();
    output.push_str(&format!("--- a/{path}\n+++ b/{path}\n"));

    // UnifiedDiff implements Display, so we can format it directly
    output.push_str(&format!("{}", diff.unified_diff().context_radius(3)));

    output
}

/// Detect binary content by NUL byte presence (same heuristic as git).
fn is_binary(s: &str) -> bool {
    s.contains('\0')
}

/// Parse a unified diff text into hunks. Returns empty Vec if `text`
/// does not look like a diff (no `@@` markers found).
fn parse_hunks(text: &str) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    let mut current: Option<Hunk> = None;
    let mut saw_file_header = false;

    for line in text.lines() {
        if line.starts_with("--- ") || line.starts_with("+++ ") {
            saw_file_header = true;
            if let Some(h) = current.as_mut() {
                h.lines.push(ParsedLine::FileHeader(line.to_string()));
            }
            continue;
        }
        if line.starts_with("@@") {
            if let Some(h) = current.take() {
                hunks.push(h);
            }
            let new_start = extract_new_start(line).unwrap_or(1);
            current = Some(Hunk {
                new_start,
                header: line.to_string(),
                lines: Vec::new(),
            });
            continue;
        }
        if let Some(h) = current.as_mut() {
            match line.chars().next() {
                Some('+') => h.lines.push(ParsedLine::Add(line[1..].to_string())),
                Some('-') => h.lines.push(ParsedLine::Del(line[1..].to_string())),
                Some(' ') => h.lines.push(ParsedLine::Context(line[1..].to_string())),
                _ => {} // skip unknown lines (e.g., "\ No newline at end of file")
            }
        }
    }
    if let Some(h) = current.take() {
        hunks.push(h);
    }
    if !saw_file_header && hunks.is_empty() {
        return Vec::new();
    }
    hunks
}

/// Extract the new-file start line number from `@@ -X,Y +A,B @@` text.
fn extract_new_start(header: &str) -> Option<usize> {
    // Find "+A" pattern; A is the new-file start.
    let plus = header.find('+')?;
    let rest = &header[plus + 1..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Render unified diff `text` into styled Lines with gutter and (if
/// `file_path`/`file_content` are available) syntect syntax highlighting.
///
/// Returns empty Vec if `text` is not a recognized diff.
pub fn render_diff(_text: &str, _file_path: &str, _file_content: &str) -> Vec<Line<'static>> {
    Vec::new() // Task 6 fills this in
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_basic_replace() {
        let old = "line1\nline2\nline3";
        let new = "line1\nchanged\nline3";
        let diff = compute_unified_diff(old, new, "foo.txt");
        assert!(diff.contains("--- a/foo.txt"));
        assert!(diff.contains("+++ b/foo.txt"));
        assert!(diff.contains("-line2"));
        assert!(diff.contains("+changed"));
        assert!(diff.contains("@@"));
    }

    #[test]
    fn test_compute_new_file() {
        let new = "alpha\nbeta\n";
        let diff = compute_unified_diff("", new, "new.txt");
        assert!(diff.contains("+alpha"));
        assert!(diff.contains("+beta"));
    }

    #[test]
    fn test_compute_delete_file() {
        let old = "alpha\nbeta\n";
        let diff = compute_unified_diff(old, "", "gone.txt");
        assert!(diff.contains("-alpha"));
        assert!(diff.contains("-beta"));
    }

    #[test]
    fn test_compute_no_change() {
        let same = "same\ncontent\n";
        let diff = compute_unified_diff(same, same, "same.txt");
        // No changes — output is just the file headers (no @@ hunks).
        assert!(diff.contains("--- a/same.txt"));
        assert!(diff.contains("+++ b/same.txt"));
        assert!(!diff.contains("@@"));
    }

    #[test]
    fn test_compute_includes_file_header() {
        let diff = compute_unified_diff("a\n", "b\n", "path/to/x.rs");
        assert!(diff.starts_with("--- a/path/to/x.rs\n"));
        assert!(diff.contains("+++ b/path/to/x.rs\n"));
    }

    #[test]
    fn test_compute_binary_returns_sentinel() {
        let old = "normal\x00binary";
        let new = "different\x00binary";
        let result = compute_unified_diff(old, new, "blob.bin");
        assert_eq!(result, "[binary file changed]");
    }

    #[test]
    fn test_parse_extracts_new_start() {
        let diff = "--- a/f\n+++ b/f\n@@ -1,3 +5,7 @@\n ctx\n+new\n";
        let hunks = parse_hunks(diff);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].new_start, 5);
    }

    #[test]
    fn test_parse_classifies_lines() {
        let diff = "--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@\n ctx\n-add\n+add\n";
        let hunks = parse_hunks(diff);
        assert_eq!(hunks.len(), 1);
        let lines = &hunks[0].lines;
        assert!(matches!(lines[0], ParsedLine::Context(_)));
        assert!(matches!(lines[1], ParsedLine::Del(_)));
        assert!(matches!(lines[2], ParsedLine::Add(_)));
    }

    #[test]
    fn test_parse_multiple_hunks() {
        let diff = "--- a/f\n+++ b/f\n@@ -1,1 +1,1 @@\n+a\n@@ -10,1 +11,1 @@\n+b\n";
        let hunks = parse_hunks(diff);
        assert_eq!(hunks.len(), 2);
        assert_eq!(hunks[0].new_start, 1);
        assert_eq!(hunks[1].new_start, 11);
    }

    #[test]
    fn test_parse_non_diff_returns_empty() {
        assert!(parse_hunks("just some\ntext").is_empty());
        assert!(parse_hunks("").is_empty());
    }
}
