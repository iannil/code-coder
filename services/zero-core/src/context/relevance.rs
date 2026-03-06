//! Relevance Scoring
//!
//! Provides relevance scoring for content in the context of a codebase.
//! Used to prioritize which files and content to include in prompts.

use std::collections::{HashMap, HashSet};

use regex::Regex;
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

// ============================================================================
// Types
// ============================================================================

/// Relevance score with breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceScore {
    /// Overall relevance score (0.0 - 1.0)
    pub score: f64,
    /// Keyword match score component
    #[serde(rename = "keywordScore")]
    pub keyword_score: f64,
    /// Structural relevance score component
    #[serde(rename = "structuralScore")]
    pub structural_score: f64,
    /// Recency score component
    #[serde(rename = "recencyScore")]
    pub recency_score: f64,
    /// Matched keywords
    #[serde(rename = "matchedKeywords")]
    pub matched_keywords: Vec<String>,
}

impl Default for RelevanceScore {
    fn default() -> Self {
        Self {
            score: 0.0,
            keyword_score: 0.0,
            structural_score: 0.0,
            recency_score: 0.0,
            matched_keywords: Vec::new(),
        }
    }
}

/// Configuration for relevance scorer
#[derive(Debug, Clone)]
pub struct RelevanceScorerConfig {
    /// Weight for keyword matching (0.0 - 1.0)
    pub keyword_weight: f64,
    /// Weight for structural relevance (0.0 - 1.0)
    pub structural_weight: f64,
    /// Weight for recency (0.0 - 1.0)
    pub recency_weight: f64,
    /// Minimum score threshold
    pub min_score: f64,
    /// Whether to use case-insensitive matching
    pub case_insensitive: bool,
}

impl Default for RelevanceScorerConfig {
    fn default() -> Self {
        Self {
            keyword_weight: 0.5,
            structural_weight: 0.3,
            recency_weight: 0.2,
            min_score: 0.1,
            case_insensitive: true,
        }
    }
}

/// File metadata for scoring
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// File path
    pub path: String,
    /// File content
    pub content: String,
    /// Last modified timestamp (unix seconds)
    pub modified: Option<u64>,
    /// File extension
    pub extension: Option<String>,
}

// ============================================================================
// Relevance Scorer
// ============================================================================

/// Relevance scorer for content
pub struct RelevanceScorer {
    config: RelevanceScorerConfig,
    /// Query keywords extracted from the input
    query_keywords: HashSet<String>,
    /// Important code patterns to boost
    code_patterns: Vec<(Regex, f64)>,
    /// File extension importance weights
    extension_weights: HashMap<String, f64>,
}

impl RelevanceScorer {
    /// Create a new relevance scorer
    pub fn new() -> Self {
        Self::with_config(RelevanceScorerConfig::default())
    }

    /// Create a new relevance scorer with custom configuration
    pub fn with_config(config: RelevanceScorerConfig) -> Self {
        let mut extension_weights = HashMap::new();

        // High priority extensions
        extension_weights.insert("ts".to_string(), 1.0);
        extension_weights.insert("tsx".to_string(), 1.0);
        extension_weights.insert("rs".to_string(), 1.0);
        extension_weights.insert("go".to_string(), 1.0);
        extension_weights.insert("py".to_string(), 1.0);
        extension_weights.insert("js".to_string(), 0.9);
        extension_weights.insert("jsx".to_string(), 0.9);

        // Medium priority
        extension_weights.insert("java".to_string(), 0.8);
        extension_weights.insert("cs".to_string(), 0.8);
        extension_weights.insert("cpp".to_string(), 0.8);
        extension_weights.insert("c".to_string(), 0.8);
        extension_weights.insert("rb".to_string(), 0.8);

        // Config files
        extension_weights.insert("json".to_string(), 0.6);
        extension_weights.insert("yaml".to_string(), 0.6);
        extension_weights.insert("yml".to_string(), 0.6);
        extension_weights.insert("toml".to_string(), 0.6);

        // Lower priority
        extension_weights.insert("md".to_string(), 0.4);
        extension_weights.insert("txt".to_string(), 0.3);

        // Code patterns that indicate important content
        let code_patterns = vec![
            (Regex::new(r"(?i)\bexport\s+(default\s+)?(function|class|const|interface|type)\b").unwrap(), 0.3),
            (Regex::new(r"(?i)\bpub\s+(fn|struct|enum|trait|mod)\b").unwrap(), 0.3),
            (Regex::new(r"(?i)\b(async|await)\b").unwrap(), 0.1),
            (Regex::new(r"(?i)\b(impl|implements|extends)\b").unwrap(), 0.2),
            (Regex::new(r"(?i)#\[derive\(").unwrap(), 0.15),
            (Regex::new(r"(?i)@(Component|Injectable|Module|Controller)\b").unwrap(), 0.2),
            (Regex::new(r"(?i)\btest\b.*\bfn\b|\bfn\b.*\btest\b").unwrap(), 0.1),
        ];

        Self {
            config,
            query_keywords: HashSet::new(),
            code_patterns,
            extension_weights,
        }
    }

    /// Set query keywords for relevance calculation
    pub fn set_query(&mut self, query: &str) {
        self.query_keywords = Self::extract_keywords(query, self.config.case_insensitive);
    }

    /// Extract keywords from text
    fn extract_keywords(text: &str, case_insensitive: bool) -> HashSet<String> {
        // Word boundary regex for extracting keywords
        let word_re = Regex::new(r"\b[a-zA-Z_][a-zA-Z0-9_]*\b").unwrap();

        // Stop words to ignore
        let stop_words: HashSet<&str> = [
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
            "have", "has", "had", "do", "does", "did", "will", "would", "could",
            "should", "may", "might", "must", "shall", "can", "need", "dare",
            "of", "in", "to", "for", "with", "on", "at", "by", "from", "as",
            "into", "through", "during", "before", "after", "above", "below",
            "and", "or", "but", "if", "then", "else", "when", "where", "why",
            "how", "all", "each", "every", "both", "few", "more", "most",
            "this", "that", "these", "those", "it", "its", "what", "which",
            "who", "whom", "whose", "function", "const", "let", "var", "class",
        ].into_iter().collect();

        word_re
            .find_iter(text)
            .filter_map(|m| {
                let word = if case_insensitive {
                    m.as_str().to_lowercase()
                } else {
                    m.as_str().to_string()
                };

                if word.len() >= 2 && !stop_words.contains(word.as_str()) {
                    Some(word)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Score content relevance
    pub fn score(&self, file: &FileMetadata) -> RelevanceScore {
        let mut score = RelevanceScore::default();

        // Keyword matching (50% weight by default)
        let (keyword_score, matched) = self.compute_keyword_score(&file.content);
        score.keyword_score = keyword_score;
        score.matched_keywords = matched;

        // Structural relevance (30% weight by default)
        score.structural_score = self.compute_structural_score(file);

        // Recency (20% weight by default)
        score.recency_score = self.compute_recency_score(file.modified);

        // Compute weighted total
        score.score = score.keyword_score * self.config.keyword_weight
            + score.structural_score * self.config.structural_weight
            + score.recency_score * self.config.recency_weight;

        // Normalize
        let total_weight = self.config.keyword_weight
            + self.config.structural_weight
            + self.config.recency_weight;
        if total_weight > 0.0 {
            score.score /= total_weight;
        }

        score
    }

    /// Compute keyword match score
    fn compute_keyword_score(&self, content: &str) -> (f64, Vec<String>) {
        if self.query_keywords.is_empty() {
            return (0.0, Vec::new());
        }

        let content_keywords = Self::extract_keywords(content, self.config.case_insensitive);
        let matched: Vec<String> = self
            .query_keywords
            .intersection(&content_keywords)
            .cloned()
            .collect();

        let match_ratio = matched.len() as f64 / self.query_keywords.len() as f64;

        // Boost for exact phrase matches
        let content_lower = content.to_lowercase();
        let mut boost = 0.0;
        for keyword in &self.query_keywords {
            if content_lower.contains(keyword) {
                boost += 0.1;
            }
        }

        ((match_ratio + boost).min(1.0), matched)
    }

    /// Compute structural relevance score
    fn compute_structural_score(&self, file: &FileMetadata) -> f64 {
        let mut score = 0.0;

        // Extension weight
        if let Some(ref ext) = file.extension {
            score += self.extension_weights.get(ext).copied().unwrap_or(0.5);
        }

        // Code pattern matches
        for (pattern, weight) in &self.code_patterns {
            if pattern.is_match(&file.content) {
                score += weight;
            }
        }

        // Path relevance (prefer source files)
        let path_lower = file.path.to_lowercase();
        if path_lower.contains("/src/") || path_lower.contains("\\src\\") {
            score += 0.2;
        }
        if path_lower.contains("/lib/") || path_lower.contains("\\lib\\") {
            score += 0.1;
        }
        if path_lower.contains("/test") || path_lower.contains("/spec") {
            score -= 0.1; // Slightly lower for test files
        }
        if path_lower.contains("node_modules") || path_lower.contains("vendor") {
            score -= 0.5; // Much lower for dependencies
        }

        // Normalize to 0-1
        score.max(0.0).min(1.0)
    }

    /// Compute recency score based on modification time
    fn compute_recency_score(&self, modified: Option<u64>) -> f64 {
        let Some(modified) = modified else {
            return 0.5; // Default score when unknown
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if now <= modified {
            return 1.0;
        }

        let age_seconds = now - modified;
        let one_day = 86400;
        let one_week = one_day * 7;
        let one_month = one_day * 30;

        if age_seconds < one_day {
            1.0
        } else if age_seconds < one_week {
            0.8
        } else if age_seconds < one_month {
            0.5
        } else {
            0.3
        }
    }

    /// Score multiple files and return sorted by relevance
    pub fn score_files(&self, files: &[FileMetadata]) -> Vec<(FileMetadata, RelevanceScore)> {
        let mut scored: Vec<_> = files
            .iter()
            .map(|f| (f.clone(), self.score(f)))
            .filter(|(_, score)| score.score >= self.config.min_score)
            .collect();

        scored.sort_by(|a, b| b.1.score.partial_cmp(&a.1.score).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// Compute content hash for deduplication
    pub fn content_hash(content: &str) -> String {
        format!("{:016x}", xxh3_64(content.as_bytes()))
    }
}

impl Default for RelevanceScorer {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function: Score content relevance
pub fn score_relevance(query: &str, content: &str) -> RelevanceScore {
    let mut scorer = RelevanceScorer::new();
    scorer.set_query(query);
    scorer.score(&FileMetadata {
        path: "".to_string(),
        content: content.to_string(),
        modified: None,
        extension: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords() {
        let keywords = RelevanceScorer::extract_keywords("hello world test_function", true);
        assert!(keywords.contains("hello"));
        assert!(keywords.contains("world"));
        assert!(keywords.contains("test_function"));
        assert!(!keywords.contains("the")); // Stop word
    }

    #[test]
    fn test_keyword_score() {
        let mut scorer = RelevanceScorer::new();
        scorer.set_query("authentication login user");

        let (score, matched) = scorer.compute_keyword_score(
            "User authentication and login functionality",
        );

        assert!(score > 0.0);
        assert!(!matched.is_empty());
    }

    #[test]
    fn test_structural_score() {
        let scorer = RelevanceScorer::new();

        // TypeScript file in src
        let file = FileMetadata {
            path: "/project/src/auth.ts".to_string(),
            content: "export function login() {}".to_string(),
            modified: None,
            extension: Some("ts".to_string()),
        };

        let score = scorer.compute_structural_score(&file);
        assert!(score > 0.5);

        // File in node_modules
        let file = FileMetadata {
            path: "/project/node_modules/pkg/index.js".to_string(),
            content: "module.exports = {}".to_string(),
            modified: None,
            extension: Some("js".to_string()),
        };

        let score = scorer.compute_structural_score(&file);
        assert!(score < 0.5);
    }

    #[test]
    fn test_recency_score() {
        let scorer = RelevanceScorer::new();

        // Recent file
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let score = scorer.compute_recency_score(Some(now - 3600)); // 1 hour ago
        assert!(score > 0.8);

        // Old file
        let score = scorer.compute_recency_score(Some(now - 86400 * 60)); // 2 months ago
        assert!(score < 0.5);
    }

    #[test]
    fn test_full_score() {
        let mut scorer = RelevanceScorer::new();
        scorer.set_query("user authentication");

        let file = FileMetadata {
            path: "/project/src/auth/user.ts".to_string(),
            content: "export class UserAuthentication { login() {} }".to_string(),
            modified: None,
            extension: Some("ts".to_string()),
        };

        let score = scorer.score(&file);
        assert!(score.score > 0.3);
        assert!(score.keyword_score > 0.0);
        assert!(score.structural_score > 0.0);
    }

    #[test]
    fn test_score_files() {
        let mut scorer = RelevanceScorer::new();
        scorer.set_query("authentication");

        let files = vec![
            FileMetadata {
                path: "/src/auth.ts".to_string(),
                content: "Authentication handler".to_string(),
                modified: None,
                extension: Some("ts".to_string()),
            },
            FileMetadata {
                path: "/src/utils.ts".to_string(),
                content: "Utility functions".to_string(),
                modified: None,
                extension: Some("ts".to_string()),
            },
        ];

        let scored = scorer.score_files(&files);
        // First file should be more relevant
        if scored.len() >= 2 {
            assert!(scored[0].1.score >= scored[1].1.score);
        }
    }

    #[test]
    fn test_content_hash() {
        let hash1 = RelevanceScorer::content_hash("hello world");
        let hash2 = RelevanceScorer::content_hash("hello world");
        let hash3 = RelevanceScorer::content_hash("different content");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_relevance_score_default() {
        let score = RelevanceScore::default();
        assert_eq!(score.score, 0.0);
        assert!(score.matched_keywords.is_empty());
    }

    #[test]
    fn test_config_customization() {
        let config = RelevanceScorerConfig {
            keyword_weight: 0.8,
            structural_weight: 0.1,
            recency_weight: 0.1,
            min_score: 0.0,
            case_insensitive: true,
        };

        let mut scorer = RelevanceScorer::with_config(config);
        scorer.set_query("test");

        let file = FileMetadata {
            path: "/src/test.ts".to_string(),
            content: "test content here".to_string(),
            modified: None,
            extension: Some("ts".to_string()),
        };

        let score = scorer.score(&file);
        // With high keyword weight, keyword-rich content should score higher
        assert!(score.keyword_score > 0.0);
    }
}
