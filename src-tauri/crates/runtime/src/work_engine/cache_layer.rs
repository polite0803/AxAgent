use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;

#[allow(clippy::type_complexity)]
pub struct InMemoryCache {
    store: Arc<RwLock<HashMap<String, (Vec<u8>, Instant)>>>,
    default_ttl: Duration,
}

impl InMemoryCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: ttl,
        }
    }
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self::new(300)
    }
}

#[derive(Debug)]
pub enum CacheError {
    NotFound,
    Expired,
    Serialization(String),
}

#[async_trait]
pub trait CacheLayer: Send + Sync {
    async fn get(&self, key: &str) -> Option<Vec<u8>>;
    async fn set(&self, key: &str, value: &[u8], ttl_secs: u64) -> Result<(), CacheError>;
    async fn delete(&self, key: &str) -> Result<(), CacheError>;
    async fn clear(&self) -> Result<(), CacheError>;
}

#[async_trait]
impl CacheLayer for InMemoryCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let store = self.store.read().await;
        if let Some((value, expiry)) = store.get(key) {
            if Instant::now() < *expiry {
                return Some(value.clone());
            }
        }
        None
    }

    async fn set(&self, key: &str, value: &[u8], ttl_secs: u64) -> Result<(), CacheError> {
        let ttl = if ttl_secs > 0 {
            Duration::from_secs(ttl_secs)
        } else {
            self.default_ttl
        };
        let expiry = Instant::now() + ttl;
        let mut store = self.store.write().await;
        store.insert(key.to_string(), (value.to_vec(), expiry));
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), CacheError> {
        let mut store = self.store.write().await;
        store.remove(key);
        Ok(())
    }

    async fn clear(&self) -> Result<(), CacheError> {
        let mut store = self.store.write().await;
        store.clear();
        Ok(())
    }
}

use tokio::sync::RwLock;
