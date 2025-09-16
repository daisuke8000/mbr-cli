use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Cache entry with TTL support
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    value: T,
    created_at: Instant,
    ttl: Duration,
}

impl<T> CacheEntry<T> {
    fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            created_at: Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// TTL-based in-memory cache with thread-safe access
#[derive(Debug)]
pub struct TtlCache<K, V> {
    storage: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    default_ttl: Duration,
}

impl<K, V> Clone for TtlCache<K, V> {
    fn clone(&self) -> Self {
        Self {
            storage: Arc::clone(&self.storage),
            default_ttl: self.default_ttl,
        }
    }
}

impl<K, V> TtlCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Create a new TTL cache with default TTL
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }

    /// Create a cache with default 5-minute TTL
    pub fn with_default_ttl() -> Self {
        Self::new(Duration::from_secs(300)) // 5 minutes
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Option<V> {
        let mut storage = self.storage.write().ok()?;

        if let Some(entry) = storage.get(key) {
            if entry.is_expired() {
                storage.remove(key);
                None
            } else {
                Some(entry.value.clone())
            }
        } else {
            None
        }
    }

    /// Insert a value with default TTL
    pub fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl);
    }

    /// Insert a value with custom TTL
    pub fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        if let Ok(mut storage) = self.storage.write() {
            storage.insert(key, CacheEntry::new(value, ttl));
        }
    }

    /// Remove a specific key
    pub fn remove(&self, key: &K) -> Option<V> {
        self.storage
            .write()
            .ok()?
            .remove(key)
            .map(|entry| entry.value)
    }

    /// Clear all entries
    pub fn clear(&self) {
        if let Ok(mut storage) = self.storage.write() {
            storage.clear();
        }
    }

    /// Remove all expired entries
    pub fn cleanup_expired(&self) {
        if let Ok(mut storage) = self.storage.write() {
            storage.retain(|_, entry| !entry.is_expired());
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        if let Ok(storage) = self.storage.read() {
            let total_entries = storage.len();
            let expired_count = storage.values().filter(|entry| entry.is_expired()).count();

            CacheStats {
                total_entries,
                active_entries: total_entries - expired_count,
                expired_entries: expired_count,
            }
        } else {
            CacheStats::default()
        }
    }

    /// Check if cache contains a non-expired key
    pub fn contains_key(&self, key: &K) -> bool {
        if let Ok(storage) = self.storage.read() {
            storage.get(key).is_some_and(|entry| !entry.is_expired())
        } else {
            false
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_entries: usize,
    pub active_entries: usize,
    pub expired_entries: usize,
}

/// API response cache specifically for Metabase API responses
pub struct ApiResponseCache {
    questions: TtlCache<u32, String>,
    dashboards: TtlCache<u32, String>,
    collections: TtlCache<String, String>, // String key for different collection queries
}

impl Default for ApiResponseCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiResponseCache {
    /// Create a new API response cache with sensible defaults
    pub fn new() -> Self {
        Self {
            questions: TtlCache::new(Duration::from_secs(300)),      // 5 minutes
            dashboards: TtlCache::new(Duration::from_secs(600)),    // 10 minutes
            collections: TtlCache::new(Duration::from_secs(1800)),  // 30 minutes
        }
    }

    /// Cache question response
    pub fn cache_question(&self, id: u32, response: String) {
        self.questions.insert(id, response);
    }

    /// Get cached question response
    pub fn get_question(&self, id: u32) -> Option<String> {
        self.questions.get(&id)
    }

    /// Cache dashboard response
    pub fn cache_dashboard(&self, id: u32, response: String) {
        self.dashboards.insert(id, response);
    }

    /// Get cached dashboard response
    pub fn get_dashboard(&self, id: u32) -> Option<String> {
        self.dashboards.get(&id)
    }

    /// Cache collection response (key can be "list", "tree", etc.)
    pub fn cache_collection(&self, key: String, response: String) {
        self.collections.insert(key, response);
    }

    /// Get cached collection response
    pub fn get_collection(&self, key: &str) -> Option<String> {
        self.collections.get(&key.to_string())
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.questions.clear();
        self.dashboards.clear();
        self.collections.clear();
    }

    /// Cleanup expired entries across all caches
    pub fn cleanup_expired(&self) {
        self.questions.cleanup_expired();
        self.dashboards.cleanup_expired();
        self.collections.cleanup_expired();
    }

    /// Get aggregate cache statistics
    pub fn stats(&self) -> HashMap<String, CacheStats> {
        let mut stats = HashMap::new();
        stats.insert("questions".to_string(), self.questions.stats());
        stats.insert("dashboards".to_string(), self.dashboards.stats());
        stats.insert("collections".to_string(), self.collections.stats());
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_ttl_cache_basic_operations() {
        let cache = TtlCache::new(Duration::from_millis(100));

        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        assert_eq!(cache.get(&"key2".to_string()), None);
    }

    #[test]
    fn test_ttl_expiration() {
        let cache = TtlCache::new(Duration::from_millis(50));

        cache.insert("key".to_string(), "value".to_string());
        assert_eq!(cache.get(&"key".to_string()), Some("value".to_string()));

        thread::sleep(Duration::from_millis(60));
        assert_eq!(cache.get(&"key".to_string()), None);
    }

    #[test]
    fn test_cache_cleanup() {
        let cache = TtlCache::new(Duration::from_millis(50));

        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());

        let stats_before = cache.stats();
        assert_eq!(stats_before.total_entries, 2);

        thread::sleep(Duration::from_millis(60));
        cache.cleanup_expired();

        let stats_after = cache.stats();
        assert_eq!(stats_after.active_entries, 0);
    }

    #[test]
    fn test_custom_ttl() {
        let cache = TtlCache::new(Duration::from_secs(1));

        cache.insert_with_ttl("short".to_string(), "value".to_string(), Duration::from_millis(10));
        cache.insert_with_ttl("long".to_string(), "value".to_string(), Duration::from_secs(10));

        thread::sleep(Duration::from_millis(20));

        assert_eq!(cache.get(&"short".to_string()), None);
        assert_eq!(cache.get(&"long".to_string()), Some("value".to_string()));
    }

    #[test]
    fn test_api_response_cache() {
        let cache = ApiResponseCache::new();

        cache.cache_question(1, "question_response".to_string());
        cache.cache_dashboard(1, "dashboard_response".to_string());
        cache.cache_collection("list".to_string(), "collection_list".to_string());

        assert_eq!(cache.get_question(1), Some("question_response".to_string()));
        assert_eq!(cache.get_dashboard(1), Some("dashboard_response".to_string()));
        assert_eq!(cache.get_collection("list"), Some("collection_list".to_string()));

        let stats = cache.stats();
        assert_eq!(stats["questions"].active_entries, 1);
        assert_eq!(stats["dashboards"].active_entries, 1);
        assert_eq!(stats["collections"].active_entries, 1);
    }

    #[test]
    fn test_cache_contains_key() {
        let cache = TtlCache::new(Duration::from_millis(100));

        assert!(!cache.contains_key(&"key".to_string()));
        cache.insert("key".to_string(), "value".to_string());
        assert!(cache.contains_key(&"key".to_string()));

        thread::sleep(Duration::from_millis(110));
        assert!(!cache.contains_key(&"key".to_string()));
    }
}