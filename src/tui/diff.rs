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
#[derive(Clone)]
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

/// Compute the gutter width needed for `hunks`. Format is
/// `marker(1) + space(1) + line_no_digits + space(1)` = digits + 3.
fn compute_gutter_width(hunks: &[Hunk]) -> usize {
    let max_line = hunks.iter()
        .flat_map(|h| {
            let mut n = h.new_start;
            h.lines.iter().filter_map(move |l| {
                let val = match l {
                    ParsedLine::Add(_) | ParsedLine::Context(_) => {
                        let v = n;
                        n += 1;
                        Some(v)
                    }
                    _ => None,
                };
                val
            })
        })
        .max()
        .unwrap_or(1);
    max_line.to_string().len() + 3
}

/// Detect syntect language from file extension, falling back to shebang.
fn detect_language(path: &str, first_line: &str) -> Option<&'static syntect::parsing::SyntaxReference> {
    let ss = crate::tui::markdown::get_syntax_set();
    if let Some(ext) = std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
        if let Some(syntax) = ss.find_syntax_by_extension(ext) {
            return Some(syntax);
        }
    }
    if !first_line.is_empty() {
        if let Some(syntax) = ss.find_syntax_by_first_line(first_line) {
            return Some(syntax);
        }
    }
    None
}

/// Precompute syntect highlight spans for every line in `content`.
///
/// `HighlightLines` carries state across lines (multi-line strings, block
/// comments), so we must tokenize the whole file once and then index into
/// the result by line number during rendering.
fn precompute_line_highlights(
    content: &str,
    lang: &syntect::parsing::SyntaxReference,
) -> Vec<Vec<Span<'static>>> {
    let ss = crate::tui::markdown::get_syntax_set();
    let theme = crate::tui::markdown::get_theme();
    let mut h = syntect::easy::HighlightLines::new(lang, theme);
    content.lines().map(|line| {
        match h.highlight_line(line, ss) {
            Ok(ranges) => ranges
                .into_iter()
                .map(|(style, s)| {
                    let color = ratatui::style::Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                    let mut modifs = ratatui::style::Modifier::empty();
                    if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
                        modifs |= ratatui::style::Modifier::BOLD;
                    }
                    if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
                        modifs |= ratatui::style::Modifier::ITALIC;
                    }
                    if style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
                        modifs |= ratatui::style::Modifier::UNDERLINED;
                    }
                    Span::styled(s.to_string(), ratatui::style::Style::default().fg(color).add_modifier(modifs))
                })
                .collect(),
            Err(_) => vec![Span::raw(line.to_string())],
        }
    }).collect()
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

    #[test]
    fn test_gutter_single_digit() {
        // max line = 9, width = 1 + 3 = 4
        let hunks = vec![Hunk {
            new_start: 1,
            header: "@@ -1,3 +1,9 @@".to_string(),
            lines: vec![ParsedLine::Context("x".to_string()); 9],
        }];
        assert_eq!(compute_gutter_width(&hunks), 4);
    }

    #[test]
    fn test_gutter_triple_digit() {
        // max line = 100, width = 3 + 3 = 6
        let hunks = vec![Hunk {
            new_start: 100,
            header: "@@ -1,1 +100,1 @@".to_string(),
            lines: vec![ParsedLine::Context("x".to_string())],
        }];
        assert_eq!(compute_gutter_width(&hunks), 6);
    }

    #[test]
    fn test_gutter_empty() {
        assert_eq!(compute_gutter_width(&[]), 4);
    }

    #[test]
    fn test_detect_by_extension_rs() {
        let syntax = detect_language("foo.rs", "");
        assert!(syntax.is_some());
        assert_eq!(syntax.unwrap().name, "Rust");
    }

    #[test]
    fn test_detect_by_extension_py() {
        let syntax = detect_language("script.py", "");
        assert!(syntax.is_some());
        assert_eq!(syntax.unwrap().name, "Python");
    }

    #[test]
    fn test_detect_unknown_falls_back_to_none() {
        assert!(detect_language("file.unknownext", "").is_none());
    }

    #[test]
    fn test_highlight_no_language_returns_empty() {
        // Helper not yet wired; verify behavior via render_diff in Task 6.
        // For now, just sanity-check that calling precompute with a known
        // language produces non-empty output for non-empty content.
        let ss = crate::tui::markdown::get_syntax_set();
        let rust = ss.find_syntax_by_extension("rs").unwrap();
        let highlights = precompute_line_highlights("fn main() {}", rust);
        assert_eq!(highlights.len(), 1); // one line
        assert!(!highlights[0].is_empty()); // at least one span
    }

    #[test]
    fn test_highlight_multiline_string_spanning_lines() {
        // A multi-line string in Rust: highlighter state must carry across.
        let content = "fn x() {\n    let s = \"a\nb\nc\";\n}\n";
        let ss = crate::tui::markdown::get_syntax_set();
        let rust = ss.find_syntax_by_extension("rs").unwrap();
        let highlights = precompute_line_highlights(content, rust);
        assert_eq!(highlights.len(), 5);
        // Line 2 (index 1) and 3 (index 2) are inside the string literal.
        // We don't assert exact colors (theme-dependent), just that they
        // have spans (i.e., were tokenized, not skipped).
        assert!(!highlights[1].is_empty());
        assert!(!highlights[2].is_empty());
    }
}
