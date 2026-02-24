//! Context retrieval API endpoints.
//!
//! Provides HTTP endpoints for searching and managing context data
//! using hybrid search (vector + keyword).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use zero_memory::{HybridSearchEngine, MemoryCategory, MemoryEntry};

/// State for context API endpoints.
#[derive(Clone)]
pub struct ContextState {
    engine: Arc<HybridSearchEngine>,
}

impl ContextState {
    /// Create a new context state with the given hybrid search engine.
    pub fn new(engine: Arc<HybridSearchEngine>) -> Self {
        Self { engine }
    }

    /// Get reference to the hybrid search engine.
    pub fn engine(&self) -> &HybridSearchEngine {
        &self.engine
    }
}

/// Query parameters for context search.
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// Search query string
    pub q: String,
    /// Maximum number of results (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Optional category filter
    #[serde(default)]
    pub category: Option<String>,
    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    10
}

/// Search response containing results and metadata.
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    /// Search results
    pub results: Vec<ContextEntry>,
    /// Total number of results (before pagination)
    pub total: usize,
    /// Query execution time in milliseconds
    pub query_time_ms: u64,
}

/// A single context entry returned from search.
#[derive(Debug, Serialize, Deserialize)]
pub struct ContextEntry {
    /// Unique identifier
    pub id: String,
    /// Content text
    pub content: String,
    /// Category of the entry
    pub category: String,
    /// Relevance score (0.0-1.0)
    pub score: f32,
    /// Creation timestamp (Unix milliseconds)
    pub created_at: i64,
}

impl From<MemoryEntry> for ContextEntry {
    fn from(entry: MemoryEntry) -> Self {
        Self {
            id: entry.key,
            content: entry.content,
            category: entry.category.to_string(),
            score: entry.score,
            created_at: entry.created_at,
        }
    }
}

/// Request to ingest new context.
#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    /// Content to store
    pub content: String,
    /// Category for the content
    #[serde(default = "default_category")]
    pub category: String,
    /// Optional source identifier
    #[serde(default)]
    pub source: Option<String>,
    /// Optional tags for the content
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_category() -> String {
    "scratch".to_string()
}

/// Response after successful ingestion.
#[derive(Debug, Serialize)]
pub struct IngestResponse {
    /// Generated ID for the stored content
    pub id: String,
    /// Success message
    pub message: String,
}

/// Error response for context API.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
    /// Error code
    pub code: String,
}

/// Categories list response.
#[derive(Debug, Serialize)]
pub struct CategoriesResponse {
    /// Available categories
    pub categories: Vec<CategoryInfo>,
}

/// Information about a category.
#[derive(Debug, Serialize)]
pub struct CategoryInfo {
    /// Category name
    pub name: String,
    /// Number of entries in this category
    pub count: usize,
}

/// Build context API routes.
///
/// # Routes
///
/// - `GET /api/v1/context/search?q=...&limit=...&category=...` - Search context
/// - `POST /api/v1/context/ingest` - Ingest new context
/// - `GET /api/v1/context/:id` - Get context by ID
/// - `GET /api/v1/context/categories` - List available categories
pub fn context_routes(state: ContextState) -> Router {
    Router::new()
        .route("/api/v1/context/search", get(search_context))
        .route("/api/v1/context/ingest", post(ingest_context))
        .route("/api/v1/context/categories", get(list_categories))
        .route("/api/v1/context/:id", get(get_context))
        .with_state(state)
}

/// Search context using hybrid search.
async fn search_context(
    State(state): State<ContextState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate query
    if query.q.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Query parameter 'q' is required".into(),
                code: "MISSING_QUERY".into(),
            }),
        ));
    }

    if query.limit == 0 || query.limit > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Limit must be between 1 and 100".into(),
                code: "INVALID_LIMIT".into(),
            }),
        ));
    }

    let start = Instant::now();

    // Parse category filter
    let category_filter = query.category.map(|c| MemoryCategory::from(c.as_str()));

    // Perform search - fetch extra for pagination
    let fetch_limit = query.offset + query.limit;
    let results = state
        .engine
        .search(&query.q, fetch_limit, category_filter)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Context search failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Search failed".into(),
                    code: "SEARCH_ERROR".into(),
                }),
            )
        })?;

    let total = results.len();

    // Apply offset pagination
    let paginated: Vec<ContextEntry> = results
        .into_iter()
        .skip(query.offset)
        .take(query.limit)
        .map(ContextEntry::from)
        .collect();

    let query_time_ms = start.elapsed().as_millis() as u64;

    Ok(Json(SearchResponse {
        results: paginated,
        total,
        query_time_ms,
    }))
}

/// Ingest new context into the hybrid search engine.
async fn ingest_context(
    State(state): State<ContextState>,
    Json(request): Json<IngestRequest>,
) -> Result<(StatusCode, Json<IngestResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Validate content
    if request.content.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Content cannot be empty".into(),
                code: "EMPTY_CONTENT".into(),
            }),
        ));
    }

    if request.content.len() > 1_000_000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Content exceeds maximum size (1MB)".into(),
                code: "CONTENT_TOO_LARGE".into(),
            }),
        ));
    }

    // Generate unique ID
    let id = generate_context_id(&request.source, &request.tags);

    // Parse category
    let category = MemoryCategory::from(request.category.as_str());

    // Store in hybrid search engine
    state
        .engine
        .store(&id, &request.content, category)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to ingest context");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to store context".into(),
                    code: "STORE_ERROR".into(),
                }),
            )
        })?;

    tracing::info!(id = %id, category = %request.category, "Context ingested");

    Ok((
        StatusCode::CREATED,
        Json(IngestResponse {
            id,
            message: "Context stored successfully".into(),
        }),
    ))
}

/// Get a specific context entry by ID.
async fn get_context(
    State(state): State<ContextState>,
    Path(id): Path<String>,
) -> Result<Json<ContextEntry>, (StatusCode, Json<ErrorResponse>)> {
    let entry = state.engine.get(&id).await.map_err(|e| {
        tracing::error!(error = %e, id = %id, "Failed to get context");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to retrieve context".into(),
                code: "GET_ERROR".into(),
            }),
        )
    })?;

    match entry {
        Some(e) => Ok(Json(ContextEntry::from(e))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Context with id '{}' not found", id),
                code: "NOT_FOUND".into(),
            }),
        )),
    }
}

/// List available categories with counts.
async fn list_categories(
    State(state): State<ContextState>,
) -> Result<Json<CategoriesResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get counts for each standard category
    let categories = vec![
        MemoryCategory::Core,
        MemoryCategory::Project,
        MemoryCategory::Conversation,
        MemoryCategory::Daily,
        MemoryCategory::Scratch,
    ];

    let mut category_infos = Vec::with_capacity(categories.len());

    for cat in categories {
        let count = state.engine.count(Some(&cat)).await.unwrap_or(0);
        category_infos.push(CategoryInfo {
            name: cat.to_string(),
            count,
        });
    }

    Ok(Json(CategoriesResponse {
        categories: category_infos,
    }))
}

/// Generate a unique ID for context entry.
fn generate_context_id(source: &Option<String>, tags: &[String]) -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let random = uuid::Uuid::new_v4().to_string()[..8].to_string();

    match source {
        Some(s) => format!("ctx-{}-{}-{}", s, timestamp, random),
        None if !tags.is_empty() => format!("ctx-{}-{}-{}", tags[0], timestamp, random),
        None => format!("ctx-{}-{}", timestamp, random),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_entry_from_memory_entry() {
        let memory = MemoryEntry::new("test-key", "test content", MemoryCategory::Core)
            .with_score(0.85);

        let context: ContextEntry = memory.into();

        assert_eq!(context.id, "test-key");
        assert_eq!(context.content, "test content");
        assert_eq!(context.category, "core");
        assert!((context.score - 0.85).abs() < f32::EPSILON);
        assert!(context.created_at > 0);
    }

    #[test]
    fn search_query_defaults() {
        let json = r#"{"q": "test query"}"#;
        let query: SearchQuery = serde_json::from_str(json).unwrap();

        assert_eq!(query.q, "test query");
        assert_eq!(query.limit, 10); // default
        assert!(query.category.is_none());
        assert_eq!(query.offset, 0); // default
    }

    #[test]
    fn search_query_with_all_fields() {
        let json = r#"{"q": "test", "limit": 20, "category": "project", "offset": 5}"#;
        let query: SearchQuery = serde_json::from_str(json).unwrap();

        assert_eq!(query.q, "test");
        assert_eq!(query.limit, 20);
        assert_eq!(query.category, Some("project".to_string()));
        assert_eq!(query.offset, 5);
    }

    #[test]
    fn ingest_request_defaults() {
        let json = r#"{"content": "some content"}"#;
        let request: IngestRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.content, "some content");
        assert_eq!(request.category, "scratch"); // default
        assert!(request.source.is_none());
        assert!(request.tags.is_empty());
    }

    #[test]
    fn generate_context_id_with_source() {
        let id = generate_context_id(&Some("webhook".to_string()), &[]);
        assert!(id.starts_with("ctx-webhook-"));
    }

    #[test]
    fn generate_context_id_with_tags() {
        let id = generate_context_id(&None, &["important".to_string()]);
        assert!(id.starts_with("ctx-important-"));
    }

    #[test]
    fn generate_context_id_minimal() {
        let id = generate_context_id(&None, &[]);
        assert!(id.starts_with("ctx-"));
        // Should have timestamp and random suffix
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3); // ctx, timestamp, random
    }
}
