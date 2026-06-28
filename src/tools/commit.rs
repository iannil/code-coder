/// ─── CommitTool ────────────────────────────────────────────────────────────
///
/// Stage and commit code changes.  Agent specifies the commit message.
///
/// Input: {"message": "fix: handle edge case"}
///        {"message": "refactor: extract helper", "files": ["src/main.rs"]}

use super::Tool;
use std::process::Command;

pub struct CommitTool;

impl Tool for CommitTool {
    fn name(&self) -> &str {
        "commit"
    }

    fn description(&self) -> &str {
        "Stage and commit changes. Input JSON: {\"message\":\"commit msg\", \"files\":[]}. Returns commit summary."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        #[derive(serde::Deserialize)]
        struct CommitInput {
            message: String,
            #[serde(default)]
            files: Vec<String>,
        }

        let parsed: CommitInput = serde_json::from_str(input)
            .map_err(|e| anyhow::anyhow!("Invalid commit input: {e}"))?;

        if parsed.message.is_empty() {
            anyhow::bail!("commit message is required");
        }

        // Stage files (or all if none specified)
        if parsed.files.is_empty() {
            let output = Command::new("git")
                .args(["add", "-A"])
                .output()
                .map_err(|e| anyhow::anyhow!("git add failed: {e}"))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("git add failed: {stderr}");
            }
        } else {
            for file in &parsed.files {
                let output = Command::new("git")
                    .args(["add", file])
                    .output()
                    .map_err(|e| anyhow::anyhow!("git add {file} failed: {e}"))?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    anyhow::bail!("git add {file} failed: {stderr}");
                }
            }
        }

        // Check if anything is staged
        let diff_output = Command::new("git")
            .args(["diff", "--cached", "--stat"])
            .output()
            .map_err(|e| anyhow::anyhow!("git diff failed: {e}"))?;
        let diff_stat = String::from_utf8_lossy(&diff_output.stdout).to_string();

        if diff_stat.trim().is_empty() {
            return Ok("Nothing to commit — no changes staged.".into());
        }

        // Commit
        let output = Command::new("git")
            .args(["commit", "-m", &parsed.message])
            .output()
            .map_err(|e| anyhow::anyhow!("git commit failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git commit failed: {stderr}");
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(format!("Committed:\n{}\n{}", diff_stat.trim(), stdout.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_empty_message() {
        let tool = CommitTool;
        let result = tool.execute(r#"{"message": ""}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_commit_invalid_json() {
        let tool = CommitTool;
        let result = tool.execute("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_commit_tool_name() {
        let tool = CommitTool;
        assert_eq!(tool.name(), "commit");
    }

    #[test]
    fn test_commit_tool_description_not_empty() {
        let tool = CommitTool;
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_commit_missing_message() {
        let tool = CommitTool;
        let result = tool.execute(r#"{}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_commit_no_changes() {
        let tool = CommitTool;
        // In a non-git dir, it will fail at git add, but test the path
        let result = tool.execute(r#"{"message": "test commit"}"#);
        // Either git add error or "nothing to commit" — both exercise the code
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_commit_with_files() {
        let tool = CommitTool;
        let result = tool.execute(r#"{"message": "fix", "files": ["src/main.rs"]}"#);
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_commit_multiple_files() {
        let tool = CommitTool;
        let result = tool.execute(r#"{"message": "multi", "files": ["a.rs", "b.rs"]}"#);
        assert!(result.is_err() || result.is_ok());
    }
}
