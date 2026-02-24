//! Qdrant-backed vector memory for semantic search.
//!
//! Uses Qdrant vector database for high-performance similarity search.
//! Requires a running Qdrant instance.

use crate::embeddings::EmbeddingProvider;
use crate::traits::{Memory, MemoryCategory, MemoryEntry};
use async_trait::async_trait;
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter, GetPointsBuilder,
    PointId, PointStruct, PointsIdsList, ScrollPointsBuilder, SearchPointsBuilder,
    UpsertPointsBuilder, VectorParamsBuilder,
};
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Metadata stored with each Qdrant point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantMetadata {
    /// Unique key for this memory
    pub key: String,
    /// The stored content
    pub content: String,
    /// Category for organization
    pub category: String,
    /// Creation timestamp (Unix millis)
    pub created_at: i64,
    /// Last update timestamp (Unix millis)
    pub updated_at: i64,
}

impl From<&QdrantMetadata> for MemoryEntry {
    fn from(meta: &QdrantMetadata) -> Self {
        Self {
            key: meta.key.clone(),
            content: meta.content.clone(),
            category: MemoryCategory::from(meta.category.as_str()),
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            score: 0.0,
        }
    }
}

/// Qdrant-backed memory with vector similarity search.
pub struct QdrantMemory {
    client: Qdrant,
    collection: String,
    embedding: Arc<dyn EmbeddingProvider>,
    dimension: usize,
}

impl QdrantMemory {
    /// Connect to a Qdrant instance and create a memory backend.
    ///
    /// # Arguments
    /// * `url` - Qdrant server URL (e.g., "http://localhost:6334")
    /// * `collection` - Name of the collection to use
    /// * `embedding` - Embedding provider for vectorization
    ///
    /// # Example
    /// ```ignore
    /// let embedding = Arc::new(NoopEmbedding);
    /// let memory = QdrantMemory::connect("http://localhost:6334", "memories", embedding).await?;
    /// ```
    pub async fn connect(
        url: &str,
        collection: &str,
        embedding: Arc<dyn EmbeddingProvider>,
    ) -> anyhow::Result<Self> {
        let dimension = embedding.dimensions();
        if dimension == 0 {
            anyhow::bail!("Embedding provider must have non-zero dimensions for Qdrant");
        }

        let client = Qdrant::from_url(url).build()?;

        let memory = Self {
            client,
            collection: collection.to_string(),
            embedding,
            dimension,
        };

        memory.ensure_collection().await?;

        Ok(memory)
    }

    /// Ensure the collection exists with correct configuration.
    pub async fn ensure_collection(&self) -> anyhow::Result<()> {
        let collections = self.client.list_collections().await?;
        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.collection);

        if !exists {
            tracing::info!(collection = %self.collection, dimension = self.dimension, "Creating Qdrant collection");

            let vector_params = VectorParamsBuilder::new(self.dimension as u64, Distance::Cosine);

            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.collection)
                        .vectors_config(vector_params),
                )
                .await?;
        }

        Ok(())
    }

    /// Convert a string key to a deterministic point ID.
    fn key_to_id(key: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Create a point struct from key, content, and embedding.
    fn create_point(
        key: &str,
        content: &str,
        category: &MemoryCategory,
        embedding: Vec<f32>,
    ) -> PointStruct {
        let now = chrono::Utc::now().timestamp_millis();
        let id = Self::key_to_id(key);

        let metadata = QdrantMetadata {
            key: key.to_string(),
            content: content.to_string(),
            category: category.to_string(),
            created_at: now,
            updated_at: now,
        };

        let payload = serde_json::to_value(&metadata)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();

        PointStruct::new(
            PointId::from(id),
            embedding,
            payload
                .into_iter()
                .map(|(k, v)| (k, qdrant_value_from_json(v)))
                .collect::<std::collections::HashMap<_, _>>(),
        )
    }

    /// Extract metadata from a Qdrant point payload.
    fn extract_metadata(
        payload: &std::collections::HashMap<String, qdrant_client::qdrant::Value>,
    ) -> Option<QdrantMetadata> {
        let key = payload.get("key")?.as_str()?.to_string();
        let content = payload.get("content")?.as_str()?.to_string();
        let category = payload.get("category")?.as_str()?.to_string();
        let created_at = payload.get("created_at")?.as_integer()?;
        let updated_at = payload.get("updated_at")?.as_integer()?;

        Some(QdrantMetadata {
            key,
            content,
            category,
            created_at,
            updated_at,
        })
    }
}

/// Convert JSON value to Qdrant value.
fn qdrant_value_from_json(json: serde_json::Value) -> qdrant_client::qdrant::Value {
    use qdrant_client::qdrant::value::Kind;
    use qdrant_client::qdrant::Value;

    match json {
        serde_json::Value::Null => Value { kind: Some(Kind::NullValue(0)) },
        serde_json::Value::Bool(b) => Value { kind: Some(Kind::BoolValue(b)) },
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value { kind: Some(Kind::IntegerValue(i)) }
            } else if let Some(f) = n.as_f64() {
                Value { kind: Some(Kind::DoubleValue(f)) }
            } else {
                Value { kind: Some(Kind::StringValue(n.to_string())) }
            }
        }
        serde_json::Value::String(s) => Value { kind: Some(Kind::StringValue(s)) },
        serde_json::Value::Array(arr) => {
            use qdrant_client::qdrant::ListValue;
            let values: Vec<Value> = arr.into_iter().map(qdrant_value_from_json).collect();
            Value {
                kind: Some(Kind::ListValue(ListValue { values })),
            }
        }
        serde_json::Value::Object(obj) => {
            use qdrant_client::qdrant::Struct;
            let fields: std::collections::HashMap<String, Value> = obj
                .into_iter()
                .map(|(k, v)| (k, qdrant_value_from_json(v)))
                .collect();
            Value {
                kind: Some(Kind::StructValue(Struct { fields })),
            }
        }
    }
}

#[async_trait]
impl Memory for QdrantMemory {
    fn name(&self) -> &str {
        "qdrant"
    }

    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> anyhow::Result<()> {
        let embedding = self.embedding.embed_one(content).await?;
        let point = Self::create_point(key, content, &category, embedding);

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection, vec![point]).wait(true))
            .await?;

        tracing::debug!(key = key, collection = %self.collection, "Stored memory in Qdrant");
        Ok(())
    }

    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
        let query_embedding = self.embedding.embed_one(query).await?;

        let results = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.collection, query_embedding, limit as u64)
                    .with_payload(true),
            )
            .await?;

        let entries: Vec<MemoryEntry> = results
            .result
            .iter()
            .filter_map(|point| {
                let metadata = Self::extract_metadata(&point.payload)?;
                let mut entry = MemoryEntry::from(&metadata);
                entry.score = point.score;
                Some(entry)
            })
            .collect();

        Ok(entries)
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        let id = Self::key_to_id(key);

        // Use scroll with filter to find by key
        let filter = Filter::must([Condition::matches("key", key.to_string())]);

        let results = self
            .client
            .scroll(
                ScrollPointsBuilder::new(&self.collection)
                    .filter(filter)
                    .limit(1)
                    .with_payload(true),
            )
            .await?;

        let entry = results.result.first().and_then(|point| {
            Self::extract_metadata(&point.payload).map(|m| MemoryEntry::from(&m))
        });

        // Fallback: try direct point lookup by ID
        if entry.is_none() {
            let points = self
                .client
                .get_points(
                    GetPointsBuilder::new(&self.collection, vec![PointId::from(id)])
                        .with_payload(true),
                )
                .await?;

            return Ok(points.result.first().and_then(|point| {
                Self::extract_metadata(&point.payload).map(|m| MemoryEntry::from(&m))
            }));
        }

        Ok(entry)
    }

    async fn list(&self, category: Option<&MemoryCategory>) -> anyhow::Result<Vec<MemoryEntry>> {
        let mut scroll_builder = ScrollPointsBuilder::new(&self.collection)
            .limit(1000)
            .with_payload(true);

        if let Some(cat) = category {
            let filter = Filter::must([Condition::matches("category", cat.to_string())]);
            scroll_builder = scroll_builder.filter(filter);
        }

        let results = self.client.scroll(scroll_builder).await?;

        let entries: Vec<MemoryEntry> = results
            .result
            .iter()
            .filter_map(|point| {
                Self::extract_metadata(&point.payload).map(|m| MemoryEntry::from(&m))
            })
            .collect();

        Ok(entries)
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        let id = Self::key_to_id(key);

        // Check if point exists first
        let exists = self.get(key).await?.is_some();

        if exists {
            self.client
                .delete_points(
                    DeletePointsBuilder::new(&self.collection)
                        .points(PointsIdsList {
                            ids: vec![PointId::from(id)],
                        }),
                )
                .await?;
            tracing::debug!(key = key, "Deleted memory from Qdrant");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn count(&self, category: Option<&MemoryCategory>) -> anyhow::Result<usize> {
        let entries = self.list(category).await?;
        Ok(entries.len())
    }

    async fn health_check(&self) -> bool {
        self.client.health_check().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::NoopEmbedding;

    /// Mock embedding provider that returns fixed-dimension vectors for testing.
    struct MockEmbedding {
        dims: usize,
    }

    impl MockEmbedding {
        fn new(dims: usize) -> Self {
            Self { dims }
        }
    }

    #[async_trait]
    impl EmbeddingProvider for MockEmbedding {
        fn name(&self) -> &str {
            "mock"
        }

        fn dimensions(&self) -> usize {
            self.dims
        }

        async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
            Ok(texts
                .iter()
                .enumerate()
                .map(|(i, text)| {
                    // Create a simple embedding based on text length and index
                    let mut vec = vec![0.0f32; self.dims];
                    for (j, c) in text.chars().enumerate() {
                        vec[j % self.dims] += (c as u32 as f32) / 1000.0 + (i as f32) * 0.01;
                    }
                    // Normalize
                    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
                    if norm > 0.0 {
                        vec.iter_mut().for_each(|x| *x /= norm);
                    }
                    vec
                })
                .collect())
        }
    }

    #[test]
    fn key_to_id_deterministic() {
        let id1 = QdrantMemory::key_to_id("test_key");
        let id2 = QdrantMemory::key_to_id("test_key");
        assert_eq!(id1, id2);
    }

    #[test]
    fn key_to_id_different_for_different_keys() {
        let id1 = QdrantMemory::key_to_id("key1");
        let id2 = QdrantMemory::key_to_id("key2");
        assert_ne!(id1, id2);
    }

    #[test]
    fn metadata_to_entry_conversion() {
        let meta = QdrantMetadata {
            key: "test".to_string(),
            content: "content".to_string(),
            category: "core".to_string(),
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        let entry = MemoryEntry::from(&meta);
        assert_eq!(entry.key, "test");
        assert_eq!(entry.content, "content");
        assert_eq!(entry.category, MemoryCategory::Core);
        assert_eq!(entry.created_at, 1234567890);
    }

    #[tokio::test]
    async fn noop_embedding_fails_connect() {
        // NoopEmbedding has 0 dimensions, should fail
        let embedding = Arc::new(NoopEmbedding);
        let result = QdrantMemory::connect("http://localhost:6334", "test", embedding).await;
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("non-zero dimensions"));
    }

    #[tokio::test]
    #[ignore = "requires Qdrant"]
    async fn qdrant_store_and_recall() {
        let embedding = Arc::new(MockEmbedding::new(128));
        let memory = QdrantMemory::connect("http://localhost:6334", "test_memories", embedding)
            .await
            .expect("Failed to connect to Qdrant");

        // Store
        memory
            .store("rust_guide", "Rust is a systems programming language", MemoryCategory::Core)
            .await
            .expect("Failed to store");

        // Recall
        let results = memory.recall("systems programming", 5).await.expect("Failed to recall");
        assert!(!results.is_empty());
        assert_eq!(results[0].key, "rust_guide");

        // Cleanup
        memory.forget("rust_guide").await.ok();
    }

    #[tokio::test]
    #[ignore = "requires Qdrant"]
    async fn qdrant_get_by_key() {
        let embedding = Arc::new(MockEmbedding::new(128));
        let memory = QdrantMemory::connect("http://localhost:6334", "test_memories", embedding)
            .await
            .expect("Failed to connect to Qdrant");

        // Store
        memory
            .store("unique_key_123", "test content", MemoryCategory::Project)
            .await
            .expect("Failed to store");

        // Get
        let entry = memory
            .get("unique_key_123")
            .await
            .expect("Failed to get")
            .expect("Entry not found");

        assert_eq!(entry.key, "unique_key_123");
        assert_eq!(entry.content, "test content");
        assert_eq!(entry.category, MemoryCategory::Project);

        // Cleanup
        memory.forget("unique_key_123").await.ok();
    }

    #[tokio::test]
    #[ignore = "requires Qdrant"]
    async fn qdrant_forget() {
        let embedding = Arc::new(MockEmbedding::new(128));
        let memory = QdrantMemory::connect("http://localhost:6334", "test_memories", embedding)
            .await
            .expect("Failed to connect to Qdrant");

        // Store
        memory
            .store("to_delete", "will be deleted", MemoryCategory::Scratch)
            .await
            .expect("Failed to store");

        // Verify exists
        let exists = memory.get("to_delete").await.expect("Failed to get");
        assert!(exists.is_some());

        // Forget
        let deleted = memory.forget("to_delete").await.expect("Failed to forget");
        assert!(deleted);

        // Verify gone
        let gone = memory.get("to_delete").await.expect("Failed to get");
        assert!(gone.is_none());

        // Forget again should return false
        let deleted_again = memory.forget("to_delete").await.expect("Failed to forget");
        assert!(!deleted_again);
    }

    #[tokio::test]
    #[ignore = "requires Qdrant"]
    async fn qdrant_list_by_category() {
        let embedding = Arc::new(MockEmbedding::new(128));
        let memory = QdrantMemory::connect("http://localhost:6334", "test_memories", embedding)
            .await
            .expect("Failed to connect to Qdrant");

        // Store entries in different categories
        memory
            .store("core1", "core content 1", MemoryCategory::Core)
            .await
            .ok();
        memory
            .store("project1", "project content 1", MemoryCategory::Project)
            .await
            .ok();

        // List by category
        let core_entries = memory
            .list(Some(&MemoryCategory::Core))
            .await
            .expect("Failed to list");

        let has_core1 = core_entries.iter().any(|e| e.key == "core1");
        assert!(has_core1, "Should find core1 in core category");

        // Cleanup
        memory.forget("core1").await.ok();
        memory.forget("project1").await.ok();
    }

    #[tokio::test]
    #[ignore = "requires Qdrant"]
    async fn qdrant_count() {
        let embedding = Arc::new(MockEmbedding::new(128));
        let memory = QdrantMemory::connect("http://localhost:6334", "test_count", embedding)
            .await
            .expect("Failed to connect to Qdrant");

        // Store some entries
        memory.store("c1", "content 1", MemoryCategory::Core).await.ok();
        memory.store("c2", "content 2", MemoryCategory::Core).await.ok();
        memory.store("p1", "project 1", MemoryCategory::Project).await.ok();

        let total = memory.count(None).await.expect("Failed to count");
        assert!(total >= 3);

        let core_count = memory.count(Some(&MemoryCategory::Core)).await.expect("Failed to count");
        assert!(core_count >= 2);

        // Cleanup
        memory.forget("c1").await.ok();
        memory.forget("c2").await.ok();
        memory.forget("p1").await.ok();
    }

    #[tokio::test]
    #[ignore = "requires Qdrant"]
    async fn qdrant_health_check() {
        let embedding = Arc::new(MockEmbedding::new(128));
        let memory = QdrantMemory::connect("http://localhost:6334", "health_test", embedding)
            .await
            .expect("Failed to connect to Qdrant");

        assert!(memory.health_check().await);
    }

    #[test]
    fn qdrant_value_from_json_string() {
        let json = serde_json::json!("hello");
        let val = qdrant_value_from_json(json);
        assert!(matches!(val.kind, Some(qdrant_client::qdrant::value::Kind::StringValue(s)) if s == "hello"));
    }

    #[test]
    fn qdrant_value_from_json_number() {
        let json = serde_json::json!(42);
        let val = qdrant_value_from_json(json);
        assert!(matches!(val.kind, Some(qdrant_client::qdrant::value::Kind::IntegerValue(42))));
    }

    #[test]
    fn qdrant_value_from_json_bool() {
        let json = serde_json::json!(true);
        let val = qdrant_value_from_json(json);
        assert!(matches!(val.kind, Some(qdrant_client::qdrant::value::Kind::BoolValue(true))));
    }
}
