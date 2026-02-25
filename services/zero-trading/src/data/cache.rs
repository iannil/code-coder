//! Data cache for market data.
//!
//! Provides in-memory caching with TTL for K-line data to reduce API calls.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::RwLock;

use super::{Candle, Timeframe};

/// Cache entry with TTL
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    data: T,
    expires_at: DateTime<Utc>,
}

impl<T> CacheEntry<T> {
    fn new(data: T, ttl_secs: i64) -> Self {
        Self {
            data,
            expires_at: Utc::now() + Duration::seconds(ttl_secs),
        }
    }

    fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Market data cache
pub struct DataCache {
    /// Candle cache: key = "symbol:timeframe"
    candles: RwLock<HashMap<String, CacheEntry<Vec<Candle>>>>,
    /// Default TTL for candle data in seconds
    candle_ttl: i64,
}

impl DataCache {
    /// Create a new data cache
    pub fn new() -> Self {
        Self {
            candles: RwLock::new(HashMap::new()),
            candle_ttl: 60, // 1 minute default TTL
        }
    }

    /// Create with custom TTL
    pub fn with_ttl(candle_ttl_secs: i64) -> Self {
        Self {
            candles: RwLock::new(HashMap::new()),
            candle_ttl: candle_ttl_secs,
        }
    }

    /// Get cache key for candles
    fn candle_key(symbol: &str, timeframe: Timeframe) -> String {
        format!("{}:{}", symbol, timeframe)
    }

    /// Get cached candles if not expired
    pub fn get_candles(&self, symbol: &str, timeframe: Timeframe) -> Option<Vec<Candle>> {
        let key = Self::candle_key(symbol, timeframe);
        let cache = self.candles.read().ok()?;

        cache.get(&key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.data.clone())
            }
        })
    }

    /// Cache candles
    pub fn set_candles(&self, symbol: &str, timeframe: Timeframe, candles: Vec<Candle>) {
        let key = Self::candle_key(symbol, timeframe);
        let entry = CacheEntry::new(candles, self.candle_ttl);

        if let Ok(mut cache) = self.candles.write() {
            cache.insert(key, entry);
        }
    }

    /// Cache candles with custom TTL
    pub fn set_candles_with_ttl(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        candles: Vec<Candle>,
        ttl_secs: i64,
    ) {
        let key = Self::candle_key(symbol, timeframe);
        let entry = CacheEntry::new(candles, ttl_secs);

        if let Ok(mut cache) = self.candles.write() {
            cache.insert(key, entry);
        }
    }

    /// Invalidate cached candles for a symbol
    pub fn invalidate(&self, symbol: &str, timeframe: Option<Timeframe>) {
        if let Ok(mut cache) = self.candles.write() {
            match timeframe {
                Some(tf) => {
                    let key = Self::candle_key(symbol, tf);
                    cache.remove(&key);
                }
                None => {
                    // Remove all timeframes for this symbol
                    cache.retain(|k, _| !k.starts_with(&format!("{}:", symbol)));
                }
            }
        }
    }

    /// Clear all expired entries
    pub fn clear_expired(&self) {
        if let Ok(mut cache) = self.candles.write() {
            cache.retain(|_, entry| !entry.is_expired());
        }
    }

    /// Clear all cache
    pub fn clear_all(&self) {
        if let Ok(mut cache) = self.candles.write() {
            cache.clear();
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let candles = self.candles.read().ok();
        let (total, expired) = candles
            .map(|c| {
                let total = c.len();
                let expired = c.values().filter(|e| e.is_expired()).count();
                (total, expired)
            })
            .unwrap_or((0, 0));

        CacheStats {
            total_entries: total,
            expired_entries: expired,
            active_entries: total - expired,
        }
    }
}

impl Default for DataCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_candle(symbol: &str, timeframe: Timeframe) -> Candle {
        Candle {
            symbol: symbol.to_string(),
            timeframe,
            timestamp: Utc::now(),
            open: 10.0,
            high: 11.0,
            low: 9.5,
            close: 10.5,
            volume: 1000.0,
            amount: 10500.0,
        }
    }

    #[test]
    fn test_cache_set_get() {
        let cache = DataCache::new();
        let candles = vec![make_test_candle("000001.SZ", Timeframe::Daily)];

        cache.set_candles("000001.SZ", Timeframe::Daily, candles.clone());

        let cached = cache.get_candles("000001.SZ", Timeframe::Daily);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
    }

    #[test]
    fn test_cache_miss() {
        let cache = DataCache::new();
        let cached = cache.get_candles("000001.SZ", Timeframe::Daily);
        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_invalidate() {
        let cache = DataCache::new();
        let candles = vec![make_test_candle("000001.SZ", Timeframe::Daily)];

        cache.set_candles("000001.SZ", Timeframe::Daily, candles);
        cache.invalidate("000001.SZ", Some(Timeframe::Daily));

        let cached = cache.get_candles("000001.SZ", Timeframe::Daily);
        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = DataCache::new();
        let candles = vec![make_test_candle("000001.SZ", Timeframe::Daily)];

        cache.set_candles("000001.SZ", Timeframe::Daily, candles.clone());
        cache.set_candles("000001.SZ", Timeframe::H1, candles);

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.active_entries, 2);
    }
}
