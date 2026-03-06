//! Token counting for LLM context management
//!
//! Provides accurate token counting with caching for performance.
//! Supports multiple tokenizer models with fallback to fast estimation.
//!
//! # Features
//!
//! - **Accurate counting**: Uses tiktoken-rs for exact token counts (when `tokenizer` feature enabled)
//! - **Fast estimation**: O(1) estimation mode for quick approximations
//! - **Caching**: LRU cache to avoid re-counting repeated text
//! - **Batch operations**: Efficient bulk token counting
//! - **Model support**: cl100k_base (GPT-4, Claude), p50k_base (GPT-3.5)

use std::sync::Mutex;
use lru::LruCache;
use std::num::NonZeroUsize;

/// Supported tokenizer models
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenizerModel {
    /// cl100k_base - Used by GPT-4, Claude 3+
    Cl100kBase,
    /// p50k_base - Used by GPT-3.5
    P50kBase,
    /// Fast estimation (no tiktoken)
    Estimate,
}

impl Default for TokenizerModel {
    fn default() -> Self {
        Self::Cl100kBase
    }
}

/// Configuration for the token counter
#[derive(Debug, Clone)]
pub struct TokenCounterConfig {
    /// Tokenizer model to use
    pub model: TokenizerModel,
    /// Cache size (number of entries)
    pub cache_size: usize,
    /// Enable caching
    pub cache_enabled: bool,
}

impl Default for TokenCounterConfig {
    fn default() -> Self {
        Self {
            model: TokenizerModel::Cl100kBase,
            cache_size: 10_000,
            cache_enabled: true,
        }
    }
}

/// Result of a batch token count operation
#[derive(Debug, Clone)]
pub struct BatchCountResult {
    /// Token counts for each input
    pub counts: Vec<usize>,
    /// Total tokens across all inputs
    pub total: usize,
    /// Number of cache hits
    pub cache_hits: usize,
}

/// High-performance token counter with caching
pub struct TokenCounter {
    model: TokenizerModel,
    cache: Option<Mutex<LruCache<u64, usize>>>,
    #[cfg(feature = "tokenizer")]
    tiktoken: Option<tiktoken_rs::CoreBPE>,
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new(TokenCounterConfig::default())
    }
}

impl TokenCounter {
    /// Create a new token counter with configuration
    pub fn new(config: TokenCounterConfig) -> Self {
        let cache = if config.cache_enabled {
            let size = NonZeroUsize::new(config.cache_size.max(1)).unwrap();
            Some(Mutex::new(LruCache::new(size)))
        } else {
            None
        };

        #[cfg(feature = "tokenizer")]
        let tiktoken = match config.model {
            TokenizerModel::Cl100kBase => tiktoken_rs::cl100k_base().ok(),
            TokenizerModel::P50kBase => tiktoken_rs::p50k_base().ok(),
            TokenizerModel::Estimate => None,
        };

        Self {
            model: config.model,
            cache,
            #[cfg(feature = "tokenizer")]
            tiktoken,
        }
    }

    /// Create a counter with just estimation (no tiktoken)
    pub fn estimate_only() -> Self {
        Self::new(TokenCounterConfig {
            model: TokenizerModel::Estimate,
            cache_size: 10_000,
            cache_enabled: true,
        })
    }

    /// Count tokens in text
    pub fn count(&self, text: &str) -> usize {
        // Check cache first
        if let Some(ref cache) = self.cache {
            let hash = self.hash_text(text);
            if let Ok(mut guard) = cache.lock() {
                if let Some(&count) = guard.get(&hash) {
                    return count;
                }
            }
        }

        // Count tokens
        let count = self.count_uncached(text);

        // Update cache
        if let Some(ref cache) = self.cache {
            let hash = self.hash_text(text);
            if let Ok(mut guard) = cache.lock() {
                guard.put(hash, count);
            }
        }

        count
    }

    /// Count tokens without using cache
    #[inline]
    fn count_uncached(&self, text: &str) -> usize {
        #[cfg(feature = "tokenizer")]
        if let Some(ref bpe) = self.tiktoken {
            return bpe.encode_ordinary(text).len();
        }

        // Fast estimation fallback
        Self::estimate_tokens(text)
    }

    /// Count tokens in multiple texts efficiently
    pub fn count_batch(&self, texts: &[&str]) -> BatchCountResult {
        let mut counts = Vec::with_capacity(texts.len());
        let mut total = 0;
        let mut cache_hits = 0;

        for text in texts {
            // Check cache
            let mut from_cache = false;
            if let Some(ref cache) = self.cache {
                let hash = self.hash_text(text);
                if let Ok(mut guard) = cache.lock() {
                    if let Some(&count) = guard.get(&hash) {
                        counts.push(count);
                        total += count;
                        cache_hits += 1;
                        from_cache = true;
                    }
                }
            }

            if !from_cache {
                let count = self.count_uncached(text);
                counts.push(count);
                total += count;

                // Update cache
                if let Some(ref cache) = self.cache {
                    let hash = self.hash_text(text);
                    if let Ok(mut guard) = cache.lock() {
                        guard.put(hash, count);
                    }
                }
            }
        }

        BatchCountResult {
            counts,
            total,
            cache_hits,
        }
    }

    /// Truncate text to fit within a token budget
    pub fn truncate_to_tokens(&self, text: &str, max_tokens: usize) -> String {
        let current = self.count(text);
        if current <= max_tokens {
            return text.to_string();
        }

        // Binary search for the right truncation point
        let chars: Vec<char> = text.chars().collect();
        let mut low = 0;
        let mut high = chars.len();

        while low < high {
            let mid = (low + high + 1) / 2;
            let truncated: String = chars[..mid].iter().collect();
            let count = self.count_uncached(&truncated);

            if count <= max_tokens {
                low = mid;
            } else {
                high = mid - 1;
            }
        }

        chars[..low].iter().collect()
    }

    /// Check if text fits within a token budget
    pub fn fits_budget(&self, text: &str, budget: usize) -> bool {
        self.count(text) <= budget
    }

    /// Get the current model
    pub fn model(&self) -> TokenizerModel {
        self.model
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        if let Some(ref cache) = self.cache {
            if let Ok(mut guard) = cache.lock() {
                guard.clear();
            }
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        if let Some(ref cache) = self.cache {
            if let Ok(guard) = cache.lock() {
                return (guard.len(), guard.cap().get());
            }
        }
        (0, 0)
    }

    /// Fast hash for cache key
    #[inline]
    fn hash_text(&self, text: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    /// Estimate token count without tiktoken
    ///
    /// Uses a heuristic based on:
    /// - ~4 characters per token for English
    /// - ~2.5 characters per token for code
    /// - Whitespace and punctuation adjustments
    #[inline]
    pub fn estimate_tokens(text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        // Count characters and character types
        let mut _alpha_count = 0;
        let mut digit_count = 0;
        let mut whitespace_count = 0;
        let mut punct_count = 0;
        let mut other_count = 0;

        for c in text.chars() {
            if c.is_alphabetic() {
                _alpha_count += 1;
            } else if c.is_ascii_digit() {
                digit_count += 1;
            } else if c.is_whitespace() {
                whitespace_count += 1;
            } else if c.is_ascii_punctuation() {
                punct_count += 1;
            } else {
                other_count += 1;
            }
        }

        let total_chars = text.len();

        // Code detection heuristic (high punctuation ratio)
        let is_code_like = punct_count as f64 / total_chars as f64 > 0.1;

        // Base estimation
        let base_tokens = if is_code_like {
            // Code typically has more tokens per character
            (total_chars + 2) / 3
        } else {
            // Natural language
            (total_chars + 3) / 4
        };

        // Adjust for whitespace (words create tokens)
        let word_bonus = whitespace_count / 4;

        // Adjust for numbers (often separate tokens)
        let digit_bonus = digit_count / 3;

        // Non-ASCII characters often become multiple tokens
        let other_bonus = other_count;

        base_tokens + word_bonus + digit_bonus + other_bonus
    }
}

/// Global token counter instance (estimate-only, no tiktoken dependency for basic usage)
static GLOBAL_COUNTER: once_cell::sync::Lazy<TokenCounter> = once_cell::sync::Lazy::new(|| {
    TokenCounter::new(TokenCounterConfig {
        model: TokenizerModel::Estimate,
        cache_size: 50_000,
        cache_enabled: true,
    })
});

/// Estimate tokens using the global counter
#[inline]
pub fn estimate_tokens(text: &str) -> usize {
    GLOBAL_COUNTER.count(text)
}

/// Estimate tokens for multiple texts using the global counter
pub fn estimate_tokens_batch(texts: &[&str]) -> BatchCountResult {
    GLOBAL_COUNTER.count_batch(texts)
}

/// Truncate text to fit within a token budget using the global counter
pub fn truncate_to_tokens(text: &str, max_tokens: usize) -> String {
    GLOBAL_COUNTER.truncate_to_tokens(text, max_tokens)
}

/// Check if text fits within a token budget using the global counter
pub fn fits_token_budget(text: &str, budget: usize) -> bool {
    GLOBAL_COUNTER.fits_budget(text, budget)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(TokenCounter::estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_short() {
        let count = TokenCounter::estimate_tokens("Hello");
        assert!(count >= 1 && count <= 3);
    }

    #[test]
    fn test_estimate_tokens_sentence() {
        let count = TokenCounter::estimate_tokens("The quick brown fox jumps over the lazy dog.");
        // ~44 chars, ~11 words, should be ~10-15 tokens
        assert!(count >= 8 && count <= 20, "Expected 8-20, got {}", count);
    }

    #[test]
    fn test_estimate_tokens_code() {
        let code = "fn main() { println!(\"Hello, world!\"); }";
        let count = TokenCounter::estimate_tokens(code);
        // Code has more punctuation, should estimate higher
        assert!(count >= 10, "Expected >= 10, got {}", count);
    }

    #[test]
    fn test_token_counter_caching() {
        let counter = TokenCounter::default();
        let text = "This is a test sentence for caching.";

        // First count - cache miss
        let count1 = counter.count(text);

        // Second count - should be cache hit
        let count2 = counter.count(text);

        assert_eq!(count1, count2);

        let (len, _) = counter.cache_stats();
        assert!(len >= 1);
    }

    #[test]
    fn test_batch_counting() {
        let counter = TokenCounter::default();
        let texts = vec!["Hello", "World", "Test"];

        let result = counter.count_batch(&texts);

        assert_eq!(result.counts.len(), 3);
        assert_eq!(result.total, result.counts.iter().sum::<usize>());
    }

    #[test]
    fn test_truncate_to_tokens() {
        let counter = TokenCounter::estimate_only();
        let text = "This is a very long sentence that needs to be truncated to fit within a token budget.";

        let truncated = counter.truncate_to_tokens(text, 5);
        let count = counter.count(&truncated);

        assert!(count <= 5, "Truncated text has {} tokens, expected <= 5", count);
        assert!(truncated.len() < text.len());
    }

    #[test]
    fn test_fits_budget() {
        let counter = TokenCounter::default();
        let short = "Hi";
        let long = "This is a much longer piece of text that will definitely exceed a small token budget.";

        assert!(counter.fits_budget(short, 10));
        assert!(!counter.fits_budget(long, 5));
    }

    #[test]
    fn test_global_functions() {
        let count = estimate_tokens("Hello world");
        assert!(count > 0);

        let result = estimate_tokens_batch(&["Hello", "World"]);
        assert_eq!(result.counts.len(), 2);

        assert!(fits_token_budget("Hi", 100));
    }

    #[test]
    fn test_unicode_tokens() {
        // Non-ASCII characters should count as more tokens
        let english = "Hello";
        let japanese = "こんにちは";

        let en_count = TokenCounter::estimate_tokens(english);
        let jp_count = TokenCounter::estimate_tokens(japanese);

        // Japanese should have more tokens per visual character
        assert!(jp_count >= en_count, "Japanese: {}, English: {}", jp_count, en_count);
    }

    #[test]
    fn test_config_no_cache() {
        let config = TokenCounterConfig {
            model: TokenizerModel::Estimate,
            cache_size: 0,
            cache_enabled: false,
        };
        let counter = TokenCounter::new(config);

        let count = counter.count("Test");
        assert!(count > 0);

        let (len, _) = counter.cache_stats();
        assert_eq!(len, 0);
    }
}
