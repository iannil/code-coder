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

/// Generate a unified diff text from `old` and `new` content for `path`.
///
/// Returns the diff with `--- a/{path}` / `+++ b/{path}` file headers so the
/// markdown renderer detects it. Binary inputs (containing NUL bytes) yield
/// the literal sentinel `"[binary file changed]"`.
pub fn compute_unified_diff(_old: &str, _new: &str, _path: &str) -> String {
    String::new() // Task 2 fills this in
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
}
