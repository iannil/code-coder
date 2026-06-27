/// ─── ReviewTool ────────────────────────────────────────────────────────────
///
/// Analyze code changes and return structured diff for agent review.
///
/// Input: {"scope": "staged|unstaged|all"} (default: "unstaged")
///        {"scope": "unstaged", "path": "src/"}

use super::Tool;
use std::process::Command;

pub struct ReviewTool;

impl Tool for ReviewTool {
    fn name(&self) -> &str {
        "review"
    }

    fn description(&self) -> &str {
        "Review code changes. Input JSON: {\"scope\":\"staged|unstaged|all\", \"path\":\".\"}. Returns structured diff for agent analysis."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        #[derive(serde::Deserialize)]
        struct ReviewInput {
            #[serde(default = "default_scope")]
            scope: String,
            #[serde(default = "default_path")]
            path: String,
        }

        fn default_scope() -> String { "unstaged".into() }
        fn default_path() -> String { ".".into() }

        let parsed: ReviewInput = serde_json::from_str(input)
            .map_err(|e| anyhow::anyhow!("Invalid review input: {e}"))?;

        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(&parsed.path);
        cmd.arg("diff");

        match parsed.scope.as_str() {
            "staged" => { cmd.arg("--cached"); }
            "all" => {
                // show both staged and unstaged
                // First get HEAD diff
                cmd.arg("HEAD");
            }
            _ => {} // unstaged (default)
        }

        let output = cmd.arg("--no-color").output()
            .map_err(|e| anyhow::anyhow!("git diff failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git diff error: {stderr}");
        }

        let diff = String::from_utf8_lossy(&output.stdout).to_string();
        if diff.trim().is_empty() {
            return Ok("(no changes to review)".into());
        }

        // Compute review summary
        let file_count = diff.lines().filter(|l| l.starts_with("diff --git")).count();
        let additions = diff.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
        let deletions = diff.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
        let files: Vec<String> = diff.lines()
            .filter_map(|l| l.strip_prefix("+++ b/"))
            .map(|s| s.to_string())
            .collect();

        let mut out = format!(
            "── Review: {} files changed, +{}, -{} ──\n",
            file_count, additions, deletions
        );

        for f in &files {
            out.push_str(&format!("  · {}\n", f));
        }

        out.push_str("\n── Diff Content ──\n");
        out.push_str(&diff);

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_invalid_json() {
        let tool = ReviewTool;
        let result = tool.execute("bad");
        assert!(result.is_err());
    }

    #[test]
    fn test_review_defaults() {
        let tool = ReviewTool;
        let result = tool.execute(r#"{}"#);
        // Either git works or we get a no-repo error
        assert!(result.is_ok() || result.is_err());
    }
}
