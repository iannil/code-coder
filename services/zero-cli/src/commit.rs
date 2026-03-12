//! Git commit with AI-generated messages
//!
//! This module provides intelligent commit message generation using LLM providers.
//! It analyzes staged changes and generates conventional commit messages.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use zero_core::git::{DiffResult, GitOpsHandle, GitStatus};
use zero_core::provider::{
    create_provider, ChatRequest, Message, MessageContent, MessageRole, ProviderConfig,
};

use crate::Config;

/// Commit command options
#[derive(Debug, Clone)]
pub struct CommitOptions {
    /// User-provided message (skip AI generation)
    pub message: Option<String>,
    /// Dry run - show what would be committed
    pub dry_run: bool,
    /// Add all changes before committing
    pub add_all: bool,
    /// Allow empty commits
    pub allow_empty: bool,
    /// Use conventional commit format
    pub conventional: bool,
}

impl Default for CommitOptions {
    fn default() -> Self {
        Self {
            message: None,
            dry_run: false,
            add_all: false,
            allow_empty: false,
            conventional: true,
        }
    }
}

/// Commit result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    pub success: bool,
    pub commit_hash: Option<String>,
    pub message: String,
    pub files_changed: usize,
    pub insertions: u32,
    pub deletions: u32,
}

/// System prompt for commit message generation
const COMMIT_SYSTEM_PROMPT: &str = r#"You are a Git commit message generator. Generate a clear, concise commit message following conventional commits format.

Rules:
1. Use format: <type>(<scope>): <description>
2. Types: feat, fix, refactor, docs, test, chore, perf, ci, style, build
3. Scope is optional but recommended (e.g., auth, api, ui)
4. Description should be imperative mood ("add" not "added")
5. Keep first line under 72 characters
6. Focus on WHAT changed and WHY, not HOW
7. If multiple changes, focus on the primary one
8. Do NOT include any explanation, just the commit message

Examples:
- feat(auth): add OAuth2 login support
- fix(api): handle null response from user endpoint
- refactor(core): extract validation into separate module
- docs: update README with installation instructions
"#;

/// Generate AI commit message from status and diff
async fn generate_commit_message(
    config: &Config,
    status: &GitStatus,
    diff: &DiffResult,
) -> Result<String> {
    let provider_id = config.default_provider.as_deref().unwrap_or("openrouter");
    let api_key = config.api_key.as_deref().unwrap_or("");

    if api_key.is_empty() {
        bail!("No API key configured. Run 'zero-cli onboard' to set up.");
    }

    let provider_config = ProviderConfig::new(provider_id, api_key);
    let provider = create_provider(provider_id, provider_config)
        .context("Failed to create provider")?;

    // Build change summary from status
    let mut change_summary = String::new();
    change_summary.push_str(&format!("Branch: {}\n\n", status.branch));

    // Summarize staged changes
    let staged_files: Vec<_> = status.files.iter().filter(|f| f.staged).collect();
    change_summary.push_str(&format!("Staged files: {}\n", staged_files.len()));

    for file in &staged_files {
        change_summary.push_str(&format!("  {:?}: {}\n", file.status, file.path));
    }

    // Add diff information if available
    if !diff.files.is_empty() {
        change_summary.push_str(&format!(
            "\nDiff stats: +{} -{} in {} files\n",
            diff.insertions, diff.deletions, diff.files_changed
        ));

        // Add patches (truncated if too long)
        change_summary.push_str("\nPatches:\n");
        for file in &diff.files {
            if let Some(patch) = &file.patch {
                let truncated = if patch.len() > 1500 {
                    format!("{}...[truncated]", &patch[..1500])
                } else {
                    patch.clone()
                };
                change_summary.push_str(&format!("\n--- {} ---\n{}\n", file.path, truncated));
            }
        }
    }

    let user_content = format!(
        "Generate a commit message for these changes:\n\n{}",
        change_summary
    );

    let request = ChatRequest {
        model: config
            .default_model
            .clone()
            .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".to_string()),
        messages: vec![
            Message {
                role: MessageRole::System,
                content: MessageContent::text(COMMIT_SYSTEM_PROMPT),
                name: None,
            },
            Message {
                role: MessageRole::User,
                content: MessageContent::text(&user_content),
                name: None,
            },
        ],
        max_tokens: Some(200),
        temperature: Some(0.3),
        ..Default::default()
    };

    let response = provider.chat(request).await?;
    let message = response.text().trim().to_string();

    // Clean up the message (remove quotes if present)
    let message = message.trim_matches('"').trim_matches('`').trim();

    Ok(message.to_string())
}

/// Execute the commit command
pub async fn run(config: Config, options: CommitOptions) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let cwd_str = cwd.to_string_lossy();
    let git = GitOpsHandle::open(&cwd_str)?;

    // Get current status
    let status = git.status()?;

    if options.dry_run {
        println!("🔍 Dry run mode - no changes will be committed\n");
    }

    // Check if there are changes
    let has_changes = !status.modified.is_empty()
        || !status.added.is_empty()
        || !status.deleted.is_empty()
        || !status.untracked.is_empty();

    if !has_changes && !options.allow_empty {
        println!("✅ Working tree clean - nothing to commit");
        return Ok(());
    }

    // Check for staged changes
    let staged_files: Vec<_> = status.files.iter().filter(|f| f.staged).collect();

    if staged_files.is_empty() && !options.add_all && !options.allow_empty {
        println!("⚠️  No staged changes. Use -a/--add-all to stage all changes.");
        println!("\nUnstaged files:");
        for f in &status.files {
            if !f.staged {
                println!("  {:?}: {}", f.status, f.path);
            }
        }
        return Ok(());
    }

    // Get diff (HEAD to working tree) for AI analysis
    let diff = git.diff(Some("HEAD"), None).unwrap_or_default();

    // Generate or use provided message
    let commit_message = if let Some(msg) = options.message {
        msg
    } else {
        println!("🤖 Generating commit message...");
        generate_commit_message(&config, &status, &diff).await?
    };

    println!("\n📝 Commit message:");
    println!("   {}\n", commit_message);

    // Show summary
    println!("📊 Summary:");
    if options.add_all {
        println!("   Files to stage: {}", status.files.len());
    } else {
        println!("   Staged files: {}", staged_files.len());
    }
    println!("   Insertions:   +{}", diff.insertions);
    println!("   Deletions:    -{}", diff.deletions);

    let files_to_show = if options.add_all {
        &status.files
    } else {
        &staged_files.iter().cloned().cloned().collect::<Vec<_>>()
    };

    if !files_to_show.is_empty() {
        println!("\n   Files:");
        for file in files_to_show {
            let staged_marker = if file.staged { "✓" } else { " " };
            println!("     {} {:?}: {}", staged_marker, file.status, file.path);
        }
    }

    if options.dry_run {
        println!("\n🔍 Dry run complete - no commit created");
        return Ok(());
    }

    // Create the commit
    println!("\n🚀 Creating commit...");
    let result = git.commit(&commit_message, options.add_all, options.allow_empty)?;

    if result.success {
        let hash = result.commit_hash.as_deref().unwrap_or("unknown");
        let short_hash = &hash[..7.min(hash.len())];
        println!("✅ Committed: {} ({})", short_hash, commit_message);
    } else {
        let error = result.error.as_deref().unwrap_or("Unknown error");
        bail!("Commit failed: {}", error);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_options_default() {
        let opts = CommitOptions::default();
        assert!(opts.message.is_none());
        assert!(!opts.dry_run);
        assert!(!opts.add_all);
        assert!(opts.conventional);
    }
}
