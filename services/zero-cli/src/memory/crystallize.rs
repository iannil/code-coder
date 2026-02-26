//! Knowledge crystallization - extract and store successful solutions.
//!
//! This module provides the `Crystallizer` which extracts learnings from
//! successful code executions and stores them for future retrieval.
//! When the agent encounters a similar problem, it can search for
//! previously crystallized solutions.

use crate::sandbox::{ExecutionAttempt, ExecutionResult, Language};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zero_memory::{Memory, MemoryCategory};

/// A crystallized piece of knowledge extracted from successful execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizedKnowledge {
    /// Unique identifier for this knowledge entry
    pub id: String,
    /// Original problem description
    pub problem: String,
    /// Errors encountered during attempts
    pub errors: Vec<String>,
    /// Final working solution (code)
    pub solution: String,
    /// Programming language used
    pub language: Language,
    /// Tags for categorization and search
    pub tags: Vec<String>,
    /// When this knowledge was crystallized
    pub created_at: DateTime<Utc>,
    /// Number of retries before success
    pub retry_count: u32,
}

impl CrystallizedKnowledge {
    /// Create a new crystallized knowledge entry.
    pub fn new(
        problem: String,
        errors: Vec<String>,
        solution: String,
        language: Language,
        tags: Vec<String>,
        retry_count: u32,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id,
            problem,
            errors,
            solution,
            language,
            tags,
            created_at: Utc::now(),
            retry_count,
        }
    }
}

/// Crystallizer stores successful solutions for future retrieval.
///
/// After a code execution succeeds (possibly after multiple retries),
/// the Crystallizer extracts the learnings and stores them in memory.
/// This enables the agent to avoid repeating the same mistakes.
pub struct Crystallizer {
    memory: Arc<dyn Memory>,
}

impl Crystallizer {
    /// Create a new Crystallizer with the given memory backend.
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }

    /// Crystallize knowledge from a successful execution.
    ///
    /// # Arguments
    /// * `problem` - Original problem description
    /// * `attempts` - All execution attempts (including failed ones)
    /// * `final_result` - The final successful execution result
    ///
    /// # Returns
    /// The crystallized knowledge entry
    pub async fn crystallize(
        &self,
        problem: &str,
        attempts: &[ExecutionAttempt],
        _final_result: &ExecutionResult,
    ) -> Result<CrystallizedKnowledge> {
        // Extract errors from failed attempts
        let errors: Vec<String> = attempts
            .iter()
            .filter(|a| !a.result.success())
            .map(|a| {
                if a.result.stderr.is_empty() {
                    a.result.stdout.clone()
                } else {
                    a.result.stderr.clone()
                }
            })
            .filter(|e| !e.is_empty())
            .collect();

        // Get the final successful code
        let solution = attempts.last().map(|a| a.code.clone()).unwrap_or_default();

        // Determine language from last attempt
        let language = attempts
            .last()
            .map(|a| a.language)
            .unwrap_or(Language::Python);

        // Generate tags for searchability
        let tags = self.generate_tags(problem, &errors);

        // Calculate retry count
        let retry_count = attempts.len().saturating_sub(1) as u32;

        // Create the knowledge entry
        let knowledge = CrystallizedKnowledge::new(
            problem.to_string(),
            errors.clone(),
            solution,
            language,
            tags,
            retry_count,
        );

        // Serialize and store in memory
        let content = serde_json::to_string_pretty(&knowledge)?;
        let key = format!("crystal:{}", knowledge.id);

        self.memory
            .store(
                &key,
                &content,
                MemoryCategory::Custom("crystallized".into()),
            )
            .await?;

        tracing::info!(
            knowledge_id = %knowledge.id,
            retry_count = retry_count,
            language = %language,
            tags = ?knowledge.tags,
            "Crystallized knowledge from successful execution"
        );

        Ok(knowledge)
    }

    /// Search for previously crystallized solutions.
    ///
    /// # Arguments
    /// * `problem` - The problem to search for
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// List of relevant crystallized knowledge entries
    pub async fn search_solutions(
        &self,
        problem: &str,
        limit: usize,
    ) -> Result<Vec<CrystallizedKnowledge>> {
        let entries = self.memory.recall(problem, limit * 2).await?;

        let mut results = Vec::new();
        for entry in entries {
            // Filter to only crystallized entries
            if !entry.key.starts_with("crystal:") {
                continue;
            }

            // Parse the knowledge entry
            match serde_json::from_str::<CrystallizedKnowledge>(&entry.content) {
                Ok(knowledge) => {
                    results.push(knowledge);
                    if results.len() >= limit {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        key = %entry.key,
                        error = %e,
                        "Failed to parse crystallized knowledge"
                    );
                }
            }
        }

        Ok(results)
    }

    /// Generate searchable tags from problem and errors.
    fn generate_tags(&self, problem: &str, errors: &[String]) -> Vec<String> {
        let mut tags = Vec::new();

        // Extract keywords from problem
        let problem_lower = problem.to_lowercase();

        // Common programming concepts
        let concepts = [
            "api",
            "http",
            "json",
            "file",
            "database",
            "sql",
            "regex",
            "parse",
            "format",
            "convert",
            "sort",
            "filter",
            "map",
            "async",
            "await",
            "thread",
            "concurrent",
            "parallel",
            "cache",
            "error",
            "exception",
            "timeout",
            "retry",
            "auth",
            "token",
            "encrypt",
            "hash",
            "compress",
            "decompress",
            "serialize",
            "deserialize",
            "websocket",
            "stream",
            "buffer",
            "memory",
            "performance",
            "optimize",
            "debug",
            "test",
            "mock",
            "validate",
            "sanitize",
            "escape",
            "encode",
            "decode",
            "split",
            "join",
            "merge",
            "diff",
            "patch",
        ];

        for concept in concepts {
            if problem_lower.contains(concept) {
                tags.push(concept.to_string());
            }
        }

        // Extract error-related tags
        for error in errors {
            let error_lower = error.to_lowercase();

            // Common error patterns
            if error_lower.contains("import") || error_lower.contains("modulenotfound") {
                tags.push("import-error".to_string());
            }
            if error_lower.contains("type") || error_lower.contains("typeerror") {
                tags.push("type-error".to_string());
            }
            if error_lower.contains("syntax") || error_lower.contains("syntaxerror") {
                tags.push("syntax-error".to_string());
            }
            if error_lower.contains("permission") || error_lower.contains("denied") {
                tags.push("permission-error".to_string());
            }
            if error_lower.contains("connection") || error_lower.contains("network") {
                tags.push("network-error".to_string());
            }
            if error_lower.contains("memory") || error_lower.contains("oom") {
                tags.push("memory-error".to_string());
            }
            if error_lower.contains("timeout") {
                tags.push("timeout-error".to_string());
            }
            if error_lower.contains("null") || error_lower.contains("none") {
                tags.push("null-error".to_string());
            }
            if error_lower.contains("index") || error_lower.contains("bounds") {
                tags.push("index-error".to_string());
            }
            if error_lower.contains("key") && error_lower.contains("error") {
                tags.push("key-error".to_string());
            }
        }

        // Deduplicate tags
        tags.sort();
        tags.dedup();

        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::Duration;
    use zero_memory::MemoryEntry;

    /// Mock memory implementation for testing.
    struct MockMemory {
        storage: Mutex<HashMap<String, MemoryEntry>>,
    }

    impl MockMemory {
        fn new() -> Self {
            Self {
                storage: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl Memory for MockMemory {
        fn name(&self) -> &str {
            "mock"
        }

        async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> Result<()> {
            let entry = MemoryEntry::new(key, content, category);
            self.storage.lock().unwrap().insert(key.to_string(), entry);
            Ok(())
        }

        async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
            let storage = self.storage.lock().unwrap();
            let query_lower = query.to_lowercase();

            let mut results: Vec<_> = storage
                .values()
                .filter(|e| e.content.to_lowercase().contains(&query_lower))
                .cloned()
                .collect();

            results.truncate(limit);
            Ok(results)
        }

        async fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
            Ok(self.storage.lock().unwrap().get(key).cloned())
        }

        async fn list(&self, category: Option<&MemoryCategory>) -> Result<Vec<MemoryEntry>> {
            let storage = self.storage.lock().unwrap();
            let entries: Vec<_> = storage
                .values()
                .filter(|e| category.map_or(true, |c| &e.category == c))
                .cloned()
                .collect();
            Ok(entries)
        }

        async fn forget(&self, key: &str) -> Result<bool> {
            Ok(self.storage.lock().unwrap().remove(key).is_some())
        }

        async fn count(&self, category: Option<&MemoryCategory>) -> Result<usize> {
            let storage = self.storage.lock().unwrap();
            let count = storage
                .values()
                .filter(|e| category.map_or(true, |c| &e.category == c))
                .count();
            Ok(count)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    fn create_test_attempt(
        code: &str,
        language: Language,
        success: bool,
        stderr: &str,
    ) -> ExecutionAttempt {
        ExecutionAttempt {
            code: code.to_string(),
            language,
            result: ExecutionResult {
                exit_code: if success { 0 } else { 1 },
                stdout: String::new(),
                stderr: stderr.to_string(),
                duration: Duration::from_millis(100),
                timed_out: false,
            },
            reflection: None,
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn crystallize_stores_knowledge() {
        let memory = Arc::new(MockMemory::new());
        let crystallizer = Crystallizer::new(memory.clone());

        let attempts = vec![
            create_test_attempt(
                "print('hello'",
                Language::Python,
                false,
                "SyntaxError: unexpected EOF",
            ),
            create_test_attempt("print('hello')", Language::Python, true, ""),
        ];

        let final_result = ExecutionResult {
            exit_code: 0,
            stdout: "hello".to_string(),
            stderr: String::new(),
            duration: Duration::from_millis(50),
            timed_out: false,
        };

        let knowledge = crystallizer
            .crystallize("Print hello world", &attempts, &final_result)
            .await
            .unwrap();

        assert_eq!(knowledge.problem, "Print hello world");
        assert_eq!(knowledge.solution, "print('hello')");
        assert_eq!(knowledge.language, Language::Python);
        assert_eq!(knowledge.retry_count, 1);
        assert!(!knowledge.errors.is_empty());
        assert!(knowledge.tags.contains(&"syntax-error".to_string()));

        // Verify stored in memory
        let stored = memory
            .get(&format!("crystal:{}", knowledge.id))
            .await
            .unwrap();
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn search_solutions_finds_relevant() {
        let memory = Arc::new(MockMemory::new());
        let crystallizer = Crystallizer::new(memory.clone());

        // Store some knowledge
        let attempts = vec![create_test_attempt(
            "import requests\nresponse = requests.get('https://api.example.com')",
            Language::Python,
            true,
            "",
        )];

        let final_result = ExecutionResult {
            exit_code: 0,
            stdout: "{}".to_string(),
            stderr: String::new(),
            duration: Duration::from_millis(200),
            timed_out: false,
        };

        crystallizer
            .crystallize("Make HTTP API request", &attempts, &final_result)
            .await
            .unwrap();

        // Search for similar problem
        let results = crystallizer
            .search_solutions("API request", 10)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].problem.contains("HTTP API"));
    }

    #[test]
    fn generate_tags_extracts_keywords() {
        let memory = Arc::new(MockMemory::new());
        let crystallizer = Crystallizer::new(memory);

        let tags = crystallizer.generate_tags("Parse JSON from HTTP API response", &[]);

        assert!(tags.contains(&"json".to_string()));
        assert!(tags.contains(&"http".to_string()));
        assert!(tags.contains(&"api".to_string()));
        assert!(tags.contains(&"parse".to_string()));
    }

    #[test]
    fn generate_tags_extracts_error_patterns() {
        let memory = Arc::new(MockMemory::new());
        let crystallizer = Crystallizer::new(memory);

        let errors = vec![
            "ModuleNotFoundError: No module named 'requests'".to_string(),
            "TypeError: expected str, got int".to_string(),
        ];

        let tags = crystallizer.generate_tags("simple task", &errors);

        assert!(tags.contains(&"import-error".to_string()));
        assert!(tags.contains(&"type-error".to_string()));
    }

    #[test]
    fn crystallized_knowledge_creation() {
        let knowledge = CrystallizedKnowledge::new(
            "test problem".to_string(),
            vec!["error1".to_string()],
            "solution code".to_string(),
            Language::Python,
            vec!["tag1".to_string()],
            2,
        );

        assert!(!knowledge.id.is_empty());
        assert_eq!(knowledge.problem, "test problem");
        assert_eq!(knowledge.retry_count, 2);
    }
}
