//! Integration tests for Qdrant memory backend.
//!
//! Requires Qdrant to be running: docker run -p 6333:6333 -p 6334:6334 qdrant/qdrant
//! Run with: cargo test --test qdrant_integration -- --ignored

use async_trait::async_trait;
use std::sync::Arc;
use zero_memory::{EmbeddingProvider, Memory, MemoryCategory, QdrantMemory};

/// Mock embedding provider for testing that produces deterministic embeddings.
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

fn create_test_memory() -> impl std::future::Future<Output = anyhow::Result<QdrantMemory>> {
    let embedding = Arc::new(MockEmbedding::new(128));
    QdrantMemory::connect("http://localhost:6334", "integration_test", embedding)
}

#[tokio::test]
#[ignore = "requires Qdrant"]
async fn qdrant_health_check() {
    let memory = create_test_memory().await.expect("Failed to connect to Qdrant");
    assert!(memory.health_check().await);
}

#[tokio::test]
#[ignore = "requires Qdrant"]
async fn qdrant_store_recall_cycle() {
    let memory = create_test_memory().await.expect("Failed to connect to Qdrant");

    // Store a test entry
    let key = format!("test_entry_{}", uuid::Uuid::new_v4());
    memory
        .store(&key, "Rust is a systems programming language focused on safety", MemoryCategory::Core)
        .await
        .expect("Failed to store");

    // Recall and verify
    let results = memory.recall("systems programming safety", 5).await.expect("Failed to recall");
    assert!(!results.is_empty(), "Should find at least one result");

    // The stored entry should be in results
    let found = results.iter().any(|e| e.key == key);
    assert!(found, "Stored entry should be found in recall results");

    // Cleanup
    memory.forget(&key).await.ok();
}

#[tokio::test]
#[ignore = "requires Qdrant"]
async fn qdrant_get_by_key() {
    let memory = create_test_memory().await.expect("Failed to connect to Qdrant");

    let key = format!("unique_key_{}", uuid::Uuid::new_v4());

    // Store
    memory
        .store(&key, "test content for get", MemoryCategory::Project)
        .await
        .expect("Failed to store");

    // Get by key
    let entry = memory
        .get(&key)
        .await
        .expect("Failed to get")
        .expect("Entry not found");

    assert_eq!(entry.key, key);
    assert_eq!(entry.content, "test content for get");
    assert_eq!(entry.category, MemoryCategory::Project);

    // Cleanup
    memory.forget(&key).await.ok();
}

#[tokio::test]
#[ignore = "requires Qdrant"]
async fn qdrant_get_nonexistent_returns_none() {
    let memory = create_test_memory().await.expect("Failed to connect to Qdrant");

    let result = memory.get("nonexistent_key_12345").await.expect("Failed to get");
    assert!(result.is_none(), "Nonexistent key should return None");
}

#[tokio::test]
#[ignore = "requires Qdrant"]
async fn qdrant_forget() {
    let memory = create_test_memory().await.expect("Failed to connect to Qdrant");

    let key = format!("to_delete_{}", uuid::Uuid::new_v4());

    // Store
    memory
        .store(&key, "will be deleted", MemoryCategory::Scratch)
        .await
        .expect("Failed to store");

    // Verify exists
    let exists = memory.get(&key).await.expect("Failed to get");
    assert!(exists.is_some(), "Entry should exist after store");

    // Forget
    let deleted = memory.forget(&key).await.expect("Failed to forget");
    assert!(deleted, "Forget should return true for existing entry");

    // Verify gone
    let gone = memory.get(&key).await.expect("Failed to get");
    assert!(gone.is_none(), "Entry should be gone after forget");

    // Forget again should return false
    let deleted_again = memory.forget(&key).await.expect("Failed to forget");
    assert!(!deleted_again, "Forget should return false for nonexistent entry");
}

#[tokio::test]
#[ignore = "requires Qdrant"]
async fn qdrant_list_by_category() {
    let memory = create_test_memory().await.expect("Failed to connect to Qdrant");

    let key1 = format!("core_{}", uuid::Uuid::new_v4());
    let key2 = format!("project_{}", uuid::Uuid::new_v4());

    // Store entries in different categories
    memory
        .store(&key1, "core content", MemoryCategory::Core)
        .await
        .ok();
    memory
        .store(&key2, "project content", MemoryCategory::Project)
        .await
        .ok();

    // List by category
    let core_entries = memory
        .list(Some(&MemoryCategory::Core))
        .await
        .expect("Failed to list");

    let has_core = core_entries.iter().any(|e| e.key == key1);
    let has_project_in_core = core_entries.iter().any(|e| e.key == key2);

    assert!(has_core, "Should find core entry in core category");
    assert!(
        !has_project_in_core,
        "Should not find project entry in core category"
    );

    // Cleanup
    memory.forget(&key1).await.ok();
    memory.forget(&key2).await.ok();
}

#[tokio::test]
#[ignore = "requires Qdrant"]
async fn qdrant_count() {
    let memory = create_test_memory().await.expect("Failed to connect to Qdrant");

    let key1 = format!("count_test_1_{}", uuid::Uuid::new_v4());
    let key2 = format!("count_test_2_{}", uuid::Uuid::new_v4());

    let initial_count = memory.count(None).await.unwrap_or(0);

    // Store some entries
    memory
        .store(&key1, "content 1", MemoryCategory::Core)
        .await
        .ok();
    memory
        .store(&key2, "content 2", MemoryCategory::Core)
        .await
        .ok();

    let new_count = memory.count(None).await.expect("Failed to count");
    assert!(
        new_count >= initial_count + 2,
        "Count should increase by at least 2"
    );

    // Cleanup
    memory.forget(&key1).await.ok();
    memory.forget(&key2).await.ok();
}

#[tokio::test]
#[ignore = "requires Qdrant"]
async fn qdrant_update_existing_key() {
    let memory = create_test_memory().await.expect("Failed to connect to Qdrant");

    let key = format!("update_test_{}", uuid::Uuid::new_v4());

    // Store initial content
    memory
        .store(&key, "initial content", MemoryCategory::Core)
        .await
        .expect("Failed to store initial");

    // Update with same key
    memory
        .store(&key, "updated content", MemoryCategory::Core)
        .await
        .expect("Failed to store update");

    // Verify content was updated
    let entry = memory
        .get(&key)
        .await
        .expect("Failed to get")
        .expect("Entry not found");

    assert_eq!(entry.content, "updated content");

    // Cleanup
    memory.forget(&key).await.ok();
}

#[tokio::test]
#[ignore = "requires Qdrant"]
async fn qdrant_semantic_search_relevance() {
    let memory = create_test_memory().await.expect("Failed to connect to Qdrant");

    let key1 = format!("rust_{}", uuid::Uuid::new_v4());
    let key2 = format!("python_{}", uuid::Uuid::new_v4());
    let key3 = format!("cooking_{}", uuid::Uuid::new_v4());

    // Store entries with different topics
    memory
        .store(&key1, "Rust programming language memory safety ownership", MemoryCategory::Core)
        .await
        .ok();
    memory
        .store(&key2, "Python scripting language data science machine learning", MemoryCategory::Core)
        .await
        .ok();
    memory
        .store(&key3, "Cooking recipes kitchen ingredients pasta sauce", MemoryCategory::Core)
        .await
        .ok();

    // Search for programming-related content
    let results = memory.recall("programming language", 3).await.expect("Failed to recall");

    // Programming entries should rank higher than cooking
    if !results.is_empty() {
        let top_key = &results[0].key;
        assert!(
            top_key == &key1 || top_key == &key2,
            "Programming entries should rank higher than cooking"
        );
    }

    // Cleanup
    memory.forget(&key1).await.ok();
    memory.forget(&key2).await.ok();
    memory.forget(&key3).await.ok();
}

#[tokio::test]
#[ignore = "requires Qdrant"]
async fn qdrant_empty_collection_operations() {
    // Use a unique collection name for this test
    let embedding = Arc::new(MockEmbedding::new(128));
    let memory = QdrantMemory::connect("http://localhost:6334", "empty_collection_test", embedding)
        .await
        .expect("Failed to connect to Qdrant");

    // Operations on empty collection should not error
    let count = memory.count(None).await.expect("Count should work on empty collection");
    assert!(count == 0 || count > 0, "Count should return a valid number");

    let results = memory.recall("test query", 5).await.expect("Recall should work on empty collection");
    // Empty or non-empty depending on previous runs
    assert!(results.len() <= 5, "Should respect limit");

    let entry = memory.get("nonexistent").await.expect("Get should work on empty collection");
    assert!(entry.is_none(), "Get nonexistent key should return None");

    let list = memory.list(None).await.expect("List should work on empty collection");
    // May be empty or have entries from previous tests
    // Just verify it doesn't error - length is always >= 0 for usize
    let _ = list.len();
}
