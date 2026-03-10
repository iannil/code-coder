//! Prompt Hot-Loading Routes
//!
//! Handles hot-loading of prompt files from the TypeScript source.
//! Prompts are loaded from packages/ccode/src/agent/prompt/*.txt

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::state::UnifiedApiState;

// ══════════════════════════════════════════════════════════════════════════════
// Request/Response Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct PromptListResponse {
    pub success: bool,
    pub prompts: Vec<PromptInfo>,
    pub total: usize,
    pub prompts_dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PromptInfo {
    pub name: String,
    pub size_bytes: usize,
    pub modified_at: Option<String>,
    pub has_metadata: bool,
}

#[derive(Debug, Serialize)]
pub struct PromptDetailResponse {
    pub success: bool,
    pub name: String,
    pub content: String,
    pub size_bytes: usize,
    pub modified_at: Option<String>,
    pub metadata: PromptMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMetadata {
    pub description: Option<String>,
    pub mode: String,
    pub temperature: Option<f64>,
    pub color: Option<String>,
    pub hidden: bool,
}

#[derive(Debug, Serialize)]
pub struct ReloadPromptsResponse {
    pub success: bool,
    pub loaded_count: usize,
    pub failed_count: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// Route Handlers
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/prompts - List all prompt files
pub async fn list_prompts(State(state): State<Arc<UnifiedApiState>>) -> impl IntoResponse {
    let prompts_dir = &state.prompts_dir;

    if !prompts_dir.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Prompts directory not found: {}", prompts_dir.display()),
            }),
        )
            .into_response();
    }

    let mut prompts = vec![];

    match std::fs::read_dir(prompts_dir) {
        Ok(entries) => {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "txt") {
                    let name = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let metadata = entry.metadata().ok();
                    let size_bytes = metadata.as_ref().map(|m| m.len() as usize).unwrap_or(0);
                    let modified_at = metadata
                        .and_then(|m| m.modified().ok())
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());

                    // Quick check for metadata
                    let has_metadata = std::fs::read_to_string(&path)
                        .map(|c| c.starts_with("<!--"))
                        .unwrap_or(false);

                    prompts.push(PromptInfo {
                        name,
                        size_bytes,
                        modified_at,
                        has_metadata,
                    });
                }
            }
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error: format!("Failed to read prompts directory: {}", e),
                }),
            )
                .into_response();
        }
    }

    // Sort by name
    prompts.sort_by(|a, b| a.name.cmp(&b.name));
    let total = prompts.len();

    Json(PromptListResponse {
        success: true,
        prompts,
        total,
        prompts_dir: prompts_dir.display().to_string(),
    })
    .into_response()
}

/// GET /api/v1/prompts/:name - Get a specific prompt
pub async fn get_prompt(
    State(state): State<Arc<UnifiedApiState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let prompt_path = state.prompts_dir.join(format!("{}.txt", name));

    if !prompt_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Prompt not found: {}", name),
            }),
        )
            .into_response();
    }

    match std::fs::read_to_string(&prompt_path) {
        Ok(content) => {
            let metadata = std::fs::metadata(&prompt_path).ok();
            let size_bytes = metadata.as_ref().map(|m| m.len() as usize).unwrap_or(0);
            let modified_at = metadata
                .and_then(|m| m.modified().ok())
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());

            let prompt_metadata = parse_metadata(&content);

            Json(PromptDetailResponse {
                success: true,
                name,
                content,
                size_bytes,
                modified_at,
                metadata: prompt_metadata,
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("Failed to read prompt: {}", e),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/prompts/reload - Reload all prompts
pub async fn reload_prompts(State(state): State<Arc<UnifiedApiState>>) -> impl IntoResponse {
    let mut loaded_count = 0;
    let mut failed_count = 0;
    let mut errors = vec![];

    match state.load_agents().await {
        Ok(_) => {
            let agents = state.agents.read().await;
            loaded_count = agents.len();
        }
        Err(e) => {
            failed_count = 1;
            errors.push(e.to_string());
        }
    }

    Json(ReloadPromptsResponse {
        success: errors.is_empty(),
        loaded_count,
        failed_count,
        errors,
    })
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper Functions
// ══════════════════════════════════════════════════════════════════════════════

fn parse_metadata(content: &str) -> PromptMetadata {
    let mut description = None;
    let mut mode = "subagent".to_string();
    let mut temperature = None;
    let mut color = None;
    let mut hidden = false;

    for line in content.lines().take(20) {
        let line = line.trim();

        if line.starts_with("<!-- description:") {
            description = line
                .strip_prefix("<!-- description:")
                .and_then(|s| s.strip_suffix("-->"))
                .map(|s| s.trim().to_string());
        } else if line.starts_with("<!-- mode:") {
            if let Some(m) = line
                .strip_prefix("<!-- mode:")
                .and_then(|s| s.strip_suffix("-->"))
            {
                mode = m.trim().to_string();
            }
        } else if line.starts_with("<!-- temperature:") {
            temperature = line
                .strip_prefix("<!-- temperature:")
                .and_then(|s| s.strip_suffix("-->"))
                .and_then(|s| s.trim().parse().ok());
        } else if line.starts_with("<!-- color:") {
            color = line
                .strip_prefix("<!-- color:")
                .and_then(|s| s.strip_suffix("-->"))
                .map(|s| s.trim().to_string());
        } else if line.starts_with("<!-- hidden:") {
            hidden = line
                .strip_prefix("<!-- hidden:")
                .and_then(|s| s.strip_suffix("-->"))
                .map(|s| s.trim() == "true")
                .unwrap_or(false);
        }
    }

    PromptMetadata {
        description,
        mode,
        temperature,
        color,
        hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metadata_full() {
        let content = r#"<!-- description: A test agent -->
<!-- mode: primary -->
<!-- temperature: 0.7 -->
<!-- color: blue -->
<!-- hidden: true -->

You are a helpful assistant."#;

        let meta = parse_metadata(content);

        assert_eq!(meta.description, Some("A test agent".to_string()));
        assert_eq!(meta.mode, "primary");
        assert_eq!(meta.temperature, Some(0.7));
        assert_eq!(meta.color, Some("blue".to_string()));
        assert!(meta.hidden);
    }

    #[test]
    fn test_parse_metadata_minimal() {
        let content = "You are a helpful assistant.";
        let meta = parse_metadata(content);

        assert!(meta.description.is_none());
        assert_eq!(meta.mode, "subagent");
        assert!(meta.temperature.is_none());
        assert!(meta.color.is_none());
        assert!(!meta.hidden);
    }
}
