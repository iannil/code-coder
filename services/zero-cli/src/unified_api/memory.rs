//! Memory System Routes
//!
//! Implements the dual-layer memory system:
//! - Daily notes (Flow layer): /memory/daily/{YYYY-MM-DD}.md
//! - Long-term memory (Sediment layer): /memory/MEMORY.md
//!
//! This is a Rust implementation of the TypeScript memory-markdown module.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use super::state::UnifiedApiState;

// ══════════════════════════════════════════════════════════════════════════════
// Constants
// ══════════════════════════════════════════════════════════════════════════════

const MEMORY_DIR: &str = "memory";
const DAILY_DIR: &str = "daily";
const LONG_TERM_FILE: &str = "MEMORY.md";

// ══════════════════════════════════════════════════════════════════════════════
// Request/Response Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct DailyDatesResponse {
    pub success: bool,
    pub dates: Vec<String>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct DailyNotesResponse {
    pub success: bool,
    pub date: String,
    pub content: String,
    pub entries: Vec<DailyEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyEntry {
    pub time: String,
    pub content: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AppendDailyRequest {
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional date override (default: today)
    pub date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AppendDailyResponse {
    pub success: bool,
    pub date: String,
    pub entry: DailyEntry,
}

#[derive(Debug, Serialize)]
pub struct LongTermResponse {
    pub success: bool,
    pub categories: HashMap<String, String>,
    pub raw: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateCategoryResponse {
    pub success: bool,
    pub category: String,
}

#[derive(Debug, Deserialize)]
pub struct MergeToCategoryRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct MergeToCategoryResponse {
    pub success: bool,
    pub category: String,
    pub merged: bool,
}

#[derive(Debug, Serialize)]
pub struct ConsolidateResponse {
    pub success: bool,
    pub consolidated_count: usize,
    pub updated_categories: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MemorySummaryResponse {
    pub success: bool,
    pub daily_entries_today: usize,
    pub daily_entries_total: usize,
    pub long_term_categories: Vec<String>,
    pub total_size_bytes: usize,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// Route Handlers
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/memory/daily - List available daily dates
pub async fn list_daily_dates(State(state): State<Arc<UnifiedApiState>>) -> impl IntoResponse {
    let daily_dir = get_daily_dir(&state.workspace_dir);

    let dates = match fs::read_dir(&daily_dir) {
        Ok(entries) => {
            let mut dates: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.ends_with(".md") {
                        Some(name.trim_end_matches(".md").to_string())
                    } else {
                        None
                    }
                })
                .collect();
            dates.sort();
            dates.reverse(); // Most recent first
            dates
        }
        Err(_) => vec![],
    };

    let total = dates.len();
    Json(DailyDatesResponse {
        success: true,
        dates,
        total,
    })
}

/// GET /api/v1/memory/daily/:date - Get daily notes for a specific date
pub async fn get_daily_notes(
    State(state): State<Arc<UnifiedApiState>>,
    Path(date): Path<String>,
) -> impl IntoResponse {
    // Validate date format
    if NaiveDate::parse_from_str(&date, "%Y-%m-%d").is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: format!("Invalid date format: {}. Expected YYYY-MM-DD", date),
            }),
        )
            .into_response();
    }

    let file_path = get_daily_file(&state.workspace_dir, &date);

    match fs::read_to_string(&file_path) {
        Ok(content) => {
            let entries = parse_daily_entries(&content);
            Json(DailyNotesResponse {
                success: true,
                date,
                content,
                entries,
            })
            .into_response()
        }
        Err(_) => {
            // Return empty response for non-existent dates
            Json(DailyNotesResponse {
                success: true,
                date,
                content: String::new(),
                entries: vec![],
            })
            .into_response()
        }
    }
}

/// POST /api/v1/memory/daily - Append a note to daily log
pub async fn append_daily_note(
    State(state): State<Arc<UnifiedApiState>>,
    Json(request): Json<AppendDailyRequest>,
) -> impl IntoResponse {
    let date = request.date.unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    let time = Utc::now().format("%H:%M:%S").to_string();

    // Ensure directory exists
    let daily_dir = get_daily_dir(&state.workspace_dir);
    if let Err(e) = fs::create_dir_all(&daily_dir) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("Failed to create directory: {}", e),
            }),
        )
            .into_response();
    }

    let file_path = get_daily_file(&state.workspace_dir, &date);

    // Format entry
    let tags_str = if request.tags.is_empty() {
        String::new()
    } else {
        format!(
            " [{}]",
            request.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" ")
        )
    };
    let entry_line = format!("\n## {time}{tags_str}\n\n{}\n", request.content);

    // Append to file
    match append_to_file(&file_path, &entry_line) {
        Ok(_) => {
            let entry = DailyEntry {
                time,
                content: request.content,
                tags: request.tags,
            };
            Json(AppendDailyResponse {
                success: true,
                date,
                entry,
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("Failed to append: {}", e),
            }),
        )
            .into_response(),
    }
}

/// GET /api/v1/memory/long-term - Get long-term memory
pub async fn get_long_term(State(state): State<Arc<UnifiedApiState>>) -> impl IntoResponse {
    let file_path = get_long_term_file(&state.workspace_dir);

    match fs::read_to_string(&file_path) {
        Ok(content) => {
            let categories = parse_long_term_categories(&content);
            Json(LongTermResponse {
                success: true,
                categories,
                raw: content,
            })
            .into_response()
        }
        Err(_) => {
            Json(LongTermResponse {
                success: true,
                categories: HashMap::new(),
                raw: String::new(),
            })
            .into_response()
        }
    }
}

/// PUT /api/v1/memory/category/:category - Update a category in long-term memory
pub async fn update_category(
    State(state): State<Arc<UnifiedApiState>>,
    Path(category): Path<String>,
    Json(request): Json<UpdateCategoryRequest>,
) -> impl IntoResponse {
    let file_path = get_long_term_file(&state.workspace_dir);

    // Read existing content
    let existing = fs::read_to_string(&file_path).unwrap_or_default();
    let mut categories = parse_long_term_categories(&existing);

    // Update category
    categories.insert(category.clone(), request.content);

    // Write back
    let new_content = format_long_term_categories(&categories);
    match fs::write(&file_path, new_content) {
        Ok(_) => Json(UpdateCategoryResponse {
            success: true,
            category,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("Failed to write: {}", e),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/memory/category/:category/merge - Merge content into a category
pub async fn merge_to_category(
    State(state): State<Arc<UnifiedApiState>>,
    Path(category): Path<String>,
    Json(request): Json<MergeToCategoryRequest>,
) -> impl IntoResponse {
    let file_path = get_long_term_file(&state.workspace_dir);

    // Read existing content
    let existing = fs::read_to_string(&file_path).unwrap_or_default();
    let mut categories = parse_long_term_categories(&existing);

    // Merge: append if exists, create if not
    let merged = if let Some(existing_content) = categories.get_mut(&category) {
        existing_content.push_str("\n\n");
        existing_content.push_str(&request.content);
        true
    } else {
        categories.insert(category.clone(), request.content);
        false
    };

    // Write back
    let new_content = format_long_term_categories(&categories);
    match fs::write(&file_path, new_content) {
        Ok(_) => Json(MergeToCategoryResponse {
            success: true,
            category,
            merged,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("Failed to write: {}", e),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/memory/consolidate - Consolidate daily notes to long-term memory
///
/// This function extracts tagged entries from recent daily notes and merges them
/// into the corresponding categories in long-term memory.
///
/// Strategy:
/// 1. Read daily notes from the past N days (default: 7)
/// 2. Extract entries with tags
/// 3. Group by tag and merge into corresponding categories
/// 4. For entries without tags, add to "Uncategorized" category
pub async fn consolidate(State(state): State<Arc<UnifiedApiState>>) -> impl IntoResponse {
    let _daily_dir = get_daily_dir(&state.workspace_dir);
    let long_term_file = get_long_term_file(&state.workspace_dir);

    // Ensure memory directory exists
    if let Err(e) = fs::create_dir_all(get_memory_dir(&state.workspace_dir)) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("Failed to create memory directory: {}", e),
            }),
        )
            .into_response();
    }

    // Read recent daily notes (last 7 days)
    let today = Utc::now().date_naive();
    let mut consolidated_count = 0;
    let mut entries_by_category: HashMap<String, Vec<String>> = HashMap::new();

    for days_ago in 0..7 {
        let date = today - chrono::Duration::days(days_ago);
        let date_str = date.format("%Y-%m-%d").to_string();
        let file_path = get_daily_file(&state.workspace_dir, &date_str);

        if let Ok(content) = fs::read_to_string(&file_path) {
            let entries = parse_daily_entries(&content);

            for entry in entries {
                if entry.content.trim().is_empty() {
                    continue;
                }

                // Determine category from tags
                let categories: Vec<String> = if entry.tags.is_empty() {
                    // Skip entries without tags (they stay in daily notes)
                    continue;
                } else {
                    // Use tags as category names
                    entry.tags.iter().map(|t| capitalize_first(t)).collect()
                };

                for category in categories {
                    let entry_text = format!(
                        "- [{}] {}: {}",
                        date_str,
                        entry.time,
                        entry.content.lines().collect::<Vec<_>>().join(" ")
                    );
                    entries_by_category
                        .entry(category)
                        .or_default()
                        .push(entry_text);
                    consolidated_count += 1;
                }
            }
        }
    }

    if entries_by_category.is_empty() {
        return Json(ConsolidateResponse {
            success: true,
            consolidated_count: 0,
            updated_categories: vec![],
        })
        .into_response();
    }

    // Read existing long-term memory
    let existing = fs::read_to_string(&long_term_file).unwrap_or_default();
    let mut categories = parse_long_term_categories(&existing);
    let mut updated_categories = vec![];

    // Merge new entries into categories
    for (category, entries) in entries_by_category {
        let new_content = entries.join("\n");

        if let Some(existing_content) = categories.get_mut(&category) {
            // Avoid duplicates by checking if content already exists
            let to_add: Vec<&str> = entries
                .iter()
                .filter(|e| !existing_content.contains(e.as_str()))
                .map(|s| s.as_str())
                .collect();

            if !to_add.is_empty() {
                existing_content.push_str("\n");
                existing_content.push_str(&to_add.join("\n"));
                updated_categories.push(category.clone());
            }
        } else {
            categories.insert(category.clone(), new_content);
            updated_categories.push(category.clone());
        }
    }

    // Write back long-term memory
    let new_content = format_long_term_categories(&categories);
    if let Err(e) = fs::write(&long_term_file, new_content) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: format!("Failed to write long-term memory: {}", e),
            }),
        )
            .into_response();
    }

    Json(ConsolidateResponse {
        success: true,
        consolidated_count,
        updated_categories,
    })
    .into_response()
}

/// Capitalize the first letter of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// GET /api/v1/memory/summary - Get memory system summary
pub async fn get_summary(State(state): State<Arc<UnifiedApiState>>) -> impl IntoResponse {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let daily_dir = get_daily_dir(&state.workspace_dir);
    let long_term_file = get_long_term_file(&state.workspace_dir);

    // Count today's entries
    let daily_entries_today = get_daily_file(&state.workspace_dir, &today)
        .exists()
        .then(|| {
            fs::read_to_string(get_daily_file(&state.workspace_dir, &today))
                .map(|c| parse_daily_entries(&c).len())
                .unwrap_or(0)
        })
        .unwrap_or(0);

    // Count total daily entries
    let daily_entries_total = fs::read_dir(&daily_dir)
        .map(|entries| entries.filter_map(|e| e.ok()).count())
        .unwrap_or(0);

    // Get categories
    let long_term_categories = fs::read_to_string(&long_term_file)
        .map(|c| parse_long_term_categories(&c).keys().cloned().collect())
        .unwrap_or_else(|_| vec![]);

    // Calculate total size
    let total_size_bytes = calculate_dir_size(&get_memory_dir(&state.workspace_dir));

    Json(MemorySummaryResponse {
        success: true,
        daily_entries_today,
        daily_entries_total,
        long_term_categories,
        total_size_bytes,
    })
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper Functions
// ══════════════════════════════════════════════════════════════════════════════

fn get_memory_dir(workspace: &std::path::Path) -> PathBuf {
    workspace.join(MEMORY_DIR)
}

fn get_daily_dir(workspace: &std::path::Path) -> PathBuf {
    get_memory_dir(workspace).join(DAILY_DIR)
}

fn get_daily_file(workspace: &std::path::Path, date: &str) -> PathBuf {
    get_daily_dir(workspace).join(format!("{}.md", date))
}

fn get_long_term_file(workspace: &std::path::Path) -> PathBuf {
    get_memory_dir(workspace).join(LONG_TERM_FILE)
}

fn append_to_file(path: &PathBuf, content: &str) -> std::io::Result<()> {
    use std::io::Write;

    // Create file if doesn't exist with header
    if !path.exists() {
        let date = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown");
        let header = format!("# Daily Notes - {date}\n");
        fs::write(path, header)?;
    }

    let mut file = fs::OpenOptions::new().append(true).open(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

fn parse_daily_entries(content: &str) -> Vec<DailyEntry> {
    let mut entries = vec![];
    let mut current_time = String::new();
    let mut current_tags = vec![];
    let mut current_content = String::new();

    for line in content.lines() {
        if line.starts_with("## ") {
            // Save previous entry
            if !current_time.is_empty() && !current_content.trim().is_empty() {
                entries.push(DailyEntry {
                    time: current_time.clone(),
                    content: current_content.trim().to_string(),
                    tags: current_tags.clone(),
                });
            }

            // Parse new entry header
            let header = line.trim_start_matches("## ").trim();

            // Extract time and tags
            if let Some((time_part, tags_part)) = header.split_once(" [") {
                current_time = time_part.to_string();
                current_tags = tags_part
                    .trim_end_matches(']')
                    .split_whitespace()
                    .filter_map(|t| t.strip_prefix('#').map(|s| s.to_string()))
                    .collect();
            } else {
                current_time = header.to_string();
                current_tags = vec![];
            }
            current_content = String::new();
        } else if !line.starts_with("# ") {
            // Append to content (skip title lines)
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Save last entry
    if !current_time.is_empty() && !current_content.trim().is_empty() {
        entries.push(DailyEntry {
            time: current_time,
            content: current_content.trim().to_string(),
            tags: current_tags,
        });
    }

    entries
}

fn parse_long_term_categories(content: &str) -> HashMap<String, String> {
    let mut categories = HashMap::new();
    let mut current_category = String::new();
    let mut current_content = String::new();

    for line in content.lines() {
        if line.starts_with("## ") {
            // Save previous category
            if !current_category.is_empty() {
                categories.insert(current_category.clone(), current_content.trim().to_string());
            }

            current_category = line.trim_start_matches("## ").trim().to_string();
            current_content = String::new();
        } else if !line.starts_with("# ") && !current_category.is_empty() {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Save last category
    if !current_category.is_empty() {
        categories.insert(current_category, current_content.trim().to_string());
    }

    categories
}

fn format_long_term_categories(categories: &HashMap<String, String>) -> String {
    let mut content = String::from("# Long-term Memory\n\n");

    // Sort categories for consistent output
    let mut sorted: Vec<_> = categories.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);

    for (category, text) in sorted {
        content.push_str(&format!("## {category}\n\n{text}\n\n"));
    }

    content
}

fn calculate_dir_size(dir: &std::path::Path) -> usize {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len() as usize)
        .sum()
}
