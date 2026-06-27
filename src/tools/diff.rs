/// ─── DiffTool ──────────────────────────────────────────────────────────────
///
/// Show git diff for the current project.  Wraps `git diff` with structured
/// output.
///
/// Input (JSON):
///   {"path": "."}                  — unstaged changes
///   {"path": ".", "cached": true}  — staged changes
///   {"path": ".", "since": "HEAD~1"} — diff since a ref

use super::Tool;
use std::process::Command;

pub struct DiffTool;

impl Tool for DiffTool {
    fn name(&self) -> &str {
        "diff"
    }

    fn description(&self) -> &str {
        "Show git diff. Input JSON: {\"path\":\".\", \"cached\":false, \"since\":\"\"}. Returns diff output."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        #[derive(serde::Deserialize)]
        struct DiffInput {
            #[serde(default = "default_path")]
            path: String,
            #[serde(default)]
            cached: bool,
            #[serde(default)]
            since: String,
        }

        fn default_path() -> String { ".".into() }

        let parsed: DiffInput = serde_json::from_str(input)
            .map_err(|e| anyhow::anyhow!("Invalid diff input: {e}"))?;

        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(&parsed.path);
        cmd.arg("diff");

        if parsed.cached {
            cmd.arg("--cached");
        }

        if !parsed.since.is_empty() {
            cmd.arg(&parsed.since);
        }

        cmd.arg("--no-color"); // raw output for agent

        let output = cmd.output()
            .map_err(|e| anyhow::anyhow!("git diff failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git diff error: {stderr}");
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if stdout.is_empty() {
            Ok("(no changes)".into())
        } else {
            // Return structured: file count + diff content
            let file_count = count_diff_files(&stdout);
            let stats = diff_stats(&stdout);
            Ok(format!(
                "── Diff ({} files, {} additions, {} deletions) ──\n{}",
                file_count, stats.0, stats.1, stdout
            ))
        }
    }
}

/// Count the number of files changed in a diff.
fn count_diff_files(diff: &str) -> usize {
    diff.lines()
        .filter(|l| l.starts_with("diff --git"))
        .count()
}

/// Count additions and deletions.
fn diff_stats(diff: &str) -> (usize, usize) {
    let mut added = 0;
    let mut removed = 0;
    for line in diff.lines() {
        if let Some(count) = line.strip_prefix('+').and_then(|s| s.parse::<usize>().ok()) {
            added += count;
        }
        if let Some(count) = line.strip_prefix('-').and_then(|s| s.parse::<usize>().ok()) {
            removed += count;
        }
    }
    (added, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_diff_files() {
        let diff = "diff --git a/a.rs b/b.rs\ndiff --git a/c.rs b/d.rs";
        assert_eq!(count_diff_files(diff), 2);
    }

    #[test]
    fn test_count_diff_files_empty() {
        assert_eq!(count_diff_files("no diff"), 0);
    }

    #[test]
    fn test_diff_tool_invalid_json() {
        let tool = DiffTool;
        let result = tool.execute("not json");
        assert!(result.is_err());
    }
}
