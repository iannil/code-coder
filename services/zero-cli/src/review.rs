//! Pull Request review with AI analysis
//!
//! This module provides intelligent code review for pull requests and branches.
//! It analyzes changes and generates actionable feedback.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use zero_core::git::{CommitInfo, DiffResult, GitOpsHandle};
use zero_core::provider::{
    create_provider, ChatRequest, Message, MessageContent, MessageRole, ProviderConfig,
};

use crate::Config;

/// Review command options
#[derive(Debug, Clone)]
pub struct ReviewOptions {
    /// Target branch or commit to review (default: compare to main/master)
    pub target: Option<String>,
    /// Base branch to compare against
    pub base: Option<String>,
    /// Output format (text, json, markdown)
    pub format: OutputFormat,
    /// Focus areas for review
    pub focus: Vec<ReviewFocus>,
    /// Show full diff in output
    pub show_diff: bool,
}

impl Default for ReviewOptions {
    fn default() -> Self {
        Self {
            target: None,
            base: None,
            format: OutputFormat::Text,
            focus: vec![
                ReviewFocus::Security,
                ReviewFocus::Bugs,
                ReviewFocus::Performance,
                ReviewFocus::Style,
            ],
            show_diff: false,
        }
    }
}

/// Output format for review results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Markdown,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

/// Focus areas for code review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewFocus {
    Security,
    Bugs,
    Performance,
    Style,
    Testing,
    Documentation,
    Architecture,
}

/// Single review finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub severity: Severity,
    pub category: ReviewFocus,
    pub file: String,
    pub line: Option<u32>,
    pub message: String,
    pub suggestion: Option<String>,
}

/// Finding severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    fn emoji(&self) -> &'static str {
        match self {
            Severity::Critical => "🔴",
            Severity::High => "🟠",
            Severity::Medium => "🟡",
            Severity::Low => "🔵",
            Severity::Info => "ℹ️",
        }
    }
}

/// Complete review result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    pub summary: String,
    pub findings: Vec<ReviewFinding>,
    pub files_reviewed: usize,
    pub total_additions: u32,
    pub total_deletions: u32,
    pub recommendation: Recommendation,
}

/// Overall recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Recommendation {
    Approve,
    RequestChanges,
    Comment,
}

impl Recommendation {
    fn emoji(&self) -> &'static str {
        match self {
            Recommendation::Approve => "✅",
            Recommendation::RequestChanges => "🔄",
            Recommendation::Comment => "💬",
        }
    }
}

/// System prompt for code review
const REVIEW_SYSTEM_PROMPT: &str = r#"You are an expert code reviewer. Analyze the provided code changes and provide actionable feedback.

Your review should:
1. Identify bugs, security issues, and potential problems
2. Suggest improvements for performance and readability
3. Check for best practices and coding standards
4. Verify error handling and edge cases
5. Consider testing implications

Response format (JSON):
{
  "summary": "Brief overall assessment (1-2 sentences)",
  "recommendation": "approve" | "request_changes" | "comment",
  "findings": [
    {
      "severity": "critical" | "high" | "medium" | "low" | "info",
      "category": "security" | "bugs" | "performance" | "style" | "testing" | "documentation" | "architecture",
      "file": "path/to/file.rs",
      "line": 42,
      "message": "Description of the issue",
      "suggestion": "How to fix it (optional)"
    }
  ]
}

Be concise and actionable. Focus on significant issues, not nitpicks.
Only output valid JSON, no markdown or explanation.
"#;

/// Generate AI review from diff
async fn generate_review(
    config: &Config,
    commits: &[CommitInfo],
    diff: &DiffResult,
    focus: &[ReviewFocus],
) -> Result<ReviewResult> {
    let provider_id = config.default_provider.as_deref().unwrap_or("openrouter");
    let api_key = config.api_key.as_deref().unwrap_or("");

    if api_key.is_empty() {
        bail!("No API key configured. Run 'zero-cli onboard' to set up.");
    }

    let provider_config = ProviderConfig::new(provider_id, api_key);
    let provider = create_provider(provider_id, provider_config)
        .context("Failed to create provider")?;

    // Build review context
    let mut review_context = String::new();

    // Add commit history
    if !commits.is_empty() {
        review_context.push_str("## Commits\n\n");
        for commit in commits {
            review_context.push_str(&format!("- {}: {}\n", &commit.hash[..7], commit.message.lines().next().unwrap_or("")));
        }
        review_context.push_str("\n");
    }

    // Add focus areas
    let focus_str: Vec<_> = focus.iter().map(|f| format!("{:?}", f).to_lowercase()).collect();
    review_context.push_str(&format!("## Focus Areas\n{}\n\n", focus_str.join(", ")));

    // Add diff stats
    review_context.push_str(&format!(
        "## Changes\n{} files changed, +{} -{}\n\n",
        diff.files_changed, diff.insertions, diff.deletions
    ));

    // Add file changes with patches
    review_context.push_str("## File Changes\n\n");
    for file in &diff.files {
        review_context.push_str(&format!(
            "### {} ({:?}, +{} -{})\n",
            file.path, file.status, file.additions, file.deletions
        ));
        if let Some(patch) = &file.patch {
            // Truncate very long patches
            let truncated = if patch.len() > 3000 {
                format!("{}...[truncated]", &patch[..3000])
            } else {
                patch.clone()
            };
            review_context.push_str(&format!("```diff\n{}\n```\n\n", truncated));
        }
    }

    let request = ChatRequest {
        model: config
            .default_model
            .clone()
            .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".to_string()),
        messages: vec![
            Message {
                role: MessageRole::System,
                content: MessageContent::text(REVIEW_SYSTEM_PROMPT),
                name: None,
            },
            Message {
                role: MessageRole::User,
                content: MessageContent::text(&format!("Review these changes:\n\n{}", review_context)),
                name: None,
            },
        ],
        max_tokens: Some(2000),
        temperature: Some(0.2),
        ..Default::default()
    };

    let response = provider.chat(request).await?;
    let response_text = response.text();
    let response_text = response_text.trim();

    // Parse JSON response
    let parsed: serde_json::Value = serde_json::from_str(response_text)
        .context("Failed to parse AI response as JSON")?;

    let summary = parsed["summary"].as_str().unwrap_or("Review complete").to_string();

    let recommendation = match parsed["recommendation"].as_str() {
        Some("approve") => Recommendation::Approve,
        Some("request_changes") => Recommendation::RequestChanges,
        _ => Recommendation::Comment,
    };

    let mut findings = Vec::new();
    if let Some(findings_arr) = parsed["findings"].as_array() {
        for f in findings_arr {
            let severity = match f["severity"].as_str() {
                Some("critical") => Severity::Critical,
                Some("high") => Severity::High,
                Some("medium") => Severity::Medium,
                Some("low") => Severity::Low,
                _ => Severity::Info,
            };

            let category = match f["category"].as_str() {
                Some("security") => ReviewFocus::Security,
                Some("bugs") => ReviewFocus::Bugs,
                Some("performance") => ReviewFocus::Performance,
                Some("style") => ReviewFocus::Style,
                Some("testing") => ReviewFocus::Testing,
                Some("documentation") => ReviewFocus::Documentation,
                Some("architecture") => ReviewFocus::Architecture,
                _ => ReviewFocus::Style,
            };

            findings.push(ReviewFinding {
                severity,
                category,
                file: f["file"].as_str().unwrap_or("unknown").to_string(),
                line: f["line"].as_u64().map(|n| n as u32),
                message: f["message"].as_str().unwrap_or("").to_string(),
                suggestion: f["suggestion"].as_str().map(String::from),
            });
        }
    }

    Ok(ReviewResult {
        summary,
        findings,
        files_reviewed: diff.files.len(),
        total_additions: diff.insertions,
        total_deletions: diff.deletions,
        recommendation,
    })
}

/// Print review in text format
fn print_text_review(result: &ReviewResult) {
    println!("\n{} Code Review Results", result.recommendation.emoji());
    println!("═══════════════════════════════════════════════════════\n");

    println!("📋 Summary: {}\n", result.summary);

    println!("📊 Statistics:");
    println!("   Files reviewed: {}", result.files_reviewed);
    println!("   Changes: +{} -{}", result.total_additions, result.total_deletions);
    println!("   Findings: {}", result.findings.len());
    println!();

    if result.findings.is_empty() {
        println!("✨ No issues found!\n");
    } else {
        // Group by severity
        let mut by_severity: std::collections::BTreeMap<u8, Vec<&ReviewFinding>> = std::collections::BTreeMap::new();
        for finding in &result.findings {
            let key = match finding.severity {
                Severity::Critical => 0,
                Severity::High => 1,
                Severity::Medium => 2,
                Severity::Low => 3,
                Severity::Info => 4,
            };
            by_severity.entry(key).or_default().push(finding);
        }

        for (_severity_key, findings) in by_severity {
            for finding in findings {
                let location = if let Some(line) = finding.line {
                    format!("{}:{}", finding.file, line)
                } else {
                    finding.file.clone()
                };

                println!("{} [{:?}] {}", finding.severity.emoji(), finding.category, location);
                println!("   {}", finding.message);
                if let Some(suggestion) = &finding.suggestion {
                    println!("   💡 {}", suggestion);
                }
                println!();
            }
        }
    }

    println!("Recommendation: {} {:?}", result.recommendation.emoji(), result.recommendation);
}

/// Execute the review command
pub async fn run(config: Config, options: ReviewOptions) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let cwd_str = cwd.to_string_lossy();
    let git = GitOpsHandle::open(&cwd_str)?;

    // Helper to check if branch exists
    let branch_exists = |name: &str| -> bool {
        git.branches(false)
            .map(|branches| branches.iter().any(|b| b == name))
            .unwrap_or(false)
    };

    // Determine base and target
    let base = options.base.as_deref().unwrap_or("main");

    // Check if base branch exists, fallback to master
    let base = if branch_exists(base) {
        base.to_string()
    } else if branch_exists("master") {
        "master".to_string()
    } else {
        // Try to use the provided base anyway (might be a commit hash)
        base.to_string()
    };

    let target = options.target.as_deref().unwrap_or("HEAD");

    println!("🔍 Reviewing changes: {} → {}\n", base, target);

    // Get commits in range
    let commits = git.commits(20)?; // Get recent commits for context

    // Get diff between base and target
    let diff = git.diff(Some(&base), Some(target))?;

    if diff.files.is_empty() {
        println!("✅ No changes to review between {} and {}", base, target);
        return Ok(());
    }

    println!("📁 Files to review: {}", diff.files.len());
    println!("📈 Changes: +{} -{}\n", diff.insertions, diff.deletions);

    if options.show_diff {
        println!("Changed files:");
        for file in &diff.files {
            println!("  {:?}: {} (+{} -{})", file.status, file.path, file.additions, file.deletions);
        }
        println!();
    }

    println!("🤖 Analyzing with AI...\n");

    let result = generate_review(&config, &commits, &diff, &options.focus).await?;

    match options.format {
        OutputFormat::Text => print_text_review(&result),
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&result)?;
            println!("{}", json);
        }
        OutputFormat::Markdown => {
            println!("# Code Review\n");
            println!("**Summary:** {}\n", result.summary);
            println!("**Recommendation:** {:?}\n", result.recommendation);
            println!("## Findings\n");
            for finding in &result.findings {
                let location = if let Some(line) = finding.line {
                    format!("`{}:{}`", finding.file, line)
                } else {
                    format!("`{}`", finding.file)
                };
                println!("### {} [{:?}] {}\n", finding.severity.emoji(), finding.category, location);
                println!("{}\n", finding.message);
                if let Some(suggestion) = &finding.suggestion {
                    println!("> 💡 {}\n", suggestion);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_options_default() {
        let opts = ReviewOptions::default();
        assert!(opts.target.is_none());
        assert!(opts.base.is_none());
        assert_eq!(opts.format, OutputFormat::Text);
        assert!(!opts.focus.is_empty());
    }

    #[test]
    fn test_output_format_parse() {
        assert_eq!("text".parse::<OutputFormat>().unwrap(), OutputFormat::Text);
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("markdown".parse::<OutputFormat>().unwrap(), OutputFormat::Markdown);
        assert_eq!("md".parse::<OutputFormat>().unwrap(), OutputFormat::Markdown);
    }
}
