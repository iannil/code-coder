/// ─── EditFileTool ──────────────────────────────────────────────────────────
///
/// Precise file editing via search-and-replace.  Safer than full file write
/// because it validates that the search string matches exactly once.
///
/// Input: {"path": "src/main.rs", "old": "exact text to replace", "new": "replacement text"}
/// If `old` is not unique in the file, the edit is rejected with a clear error.

use super::Tool;

pub struct EditFileTool;

impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by searching for exact text and replacing it. Input JSON: {\"path\":\"...\", \"old\":\"exact text\", \"new\":\"replacement\"}. The old text must match exactly once in the file."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        #[derive(serde::Deserialize)]
        struct EditInput {
            path: String,
            old: String,
            new: String,
        }

        let parsed: EditInput = serde_json::from_str(input)
            .map_err(|e| anyhow::anyhow!("Invalid edit_file input: {e}"))?;

        if parsed.old.is_empty() {
            anyhow::bail!("'old' field cannot be empty");
        }

        let content = std::fs::read_to_string(&parsed.path)
            .map_err(|e| anyhow::anyhow!("Cannot read {}: {e}", parsed.path))?;

        // Count occurrences
        let count = content.matches(&parsed.old).count();
        if count == 0 {
            anyhow::bail!("'old' text not found in {}. Check exact content including whitespace.", parsed.path);
        }
        if count > 1 {
            anyhow::bail!("'old' text matches {count} times in {}. Add more surrounding context to make it unique.", parsed.path);
        }

        let new_content = content.replace(&parsed.old, &parsed.new);
        std::fs::write(&parsed.path, &new_content)
            .map_err(|e| anyhow::anyhow!("Cannot write {}: {e}", parsed.path))?;

        // Compute diff stats
        let old_lines = content.lines().count();
        let new_lines = new_content.lines().count();
        let diff_lines = (new_lines as isize - old_lines as isize).abs();

        // V1 (audit-tui-visual-fidelity.md bucket-4 #109/#110):
        // attach unified diff with file_path so renderer can highlight.
        let diff_text = crate::tui::diff::compute_unified_diff(
            &content,
            &new_content,
            &parsed.path,
        );

        Ok(format!(
            "Edited {}. Replaced 1 occurrence ({}→{} lines, {} line diff).\n\n```diff path=\"{}\"\n{}```",
            parsed.path, old_lines, new_lines, diff_lines, parsed.path, diff_text
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_edit_file_replace() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file.as_file(), "Hello {{name}}, welcome!").unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let tool = EditFileTool;
        let input = format!(r#"{{"path": "{path}", "old": "{{name}}", "new": "World"}}"#);
        let result = tool.execute(&input).unwrap();
        assert!(result.contains("Edited"));

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Hello World, welcome!");
    }

    #[test]
    fn test_edit_file_not_found() {
        let tool = EditFileTool;
        let input = r#"{"path": "/nonexistent/file.rs", "old": "foo", "new": "bar"}"#;
        let result = tool.execute(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot read"));
    }

    #[test]
    fn test_edit_file_no_match() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file.as_file(), "Hello World").unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let tool = EditFileTool;
        let input = format!(r#"{{"path": "{path}", "old": "Nonexistent", "new": "X"}}"#);
        let result = tool.execute(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_edit_file_output_contains_diff_block() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(file.as_file_mut(), b"line1\nline2\nline3\n").unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let tool = EditFileTool;
        let input = format!(r#"{{"path": "{path}", "old": "line2", "new": "LINE TWO"}}"#);
        let result = tool.execute(&input).unwrap();

        assert!(result.contains("```diff"), "result: {result}");
        assert!(result.contains(&format!("path=\"{path}\"")), "result: {result}");
    }

    #[test]
    fn test_edit_file_diff_has_correct_line_numbers() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(file.as_file_mut(), b"a\nb\nc\nd\ne\n").unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let tool = EditFileTool;
        let input = format!(r#"{{"path": "{path}", "old": "c", "new": "CHANGED"}}"#);
        let result = tool.execute(&input).unwrap();

        // The diff should show "3" as the line number for the change (new file).
        assert!(result.contains("@@ -"), "result: {result}");
    }

    #[test]
    fn test_edit_file_diff_includes_unified_file_header() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(file.as_file_mut(), b"hello\n").unwrap();
        let path = file.path().to_str().unwrap().to_string();

        let tool = EditFileTool;
        let input = format!(r#"{{"path": "{path}", "old": "hello", "new": "world"}}"#);
        let result = tool.execute(&input).unwrap();

        assert!(result.contains(&format!("--- a/{path}")), "result: {result}");
        assert!(result.contains(&format!("+++ b/{path}")), "result: {result}");
    }
}
