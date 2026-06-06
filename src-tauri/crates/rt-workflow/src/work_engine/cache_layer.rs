use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;

type CacheStore = Arc<RwLock<HashMap<String, (Vec<u8>, Instant)>>>;

pub struct InMemoryCache {
    store: CacheStore,
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
    /// 主动清理过期条目（默认实现：no-op，子类可 override）。
    /// 返回清理掉的条目数量。
    async fn evict_expired(&self) -> usize {
        0
    }
}

#[async_trait]
impl CacheLayer for InMemoryCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        // 关键修复：先在 read 锁内判断命中/过期；命中时直接返回，
        // 命中但已过期时升级为 write 锁并主动 evict，避免过期键驻留。
        {
            let store = self.store.read().await;
            if let Some((value, expiry)) = store.get(key) {
                if Instant::now() < *expiry {
                    return Some(value.clone());
                }
            } else {
                return None;
            }
        }
        // 命中但已过期 → 主动驱逐
        let mut store = self.store.write().await;
        // 二次校验：拿到写锁后 expiry 仍需满足已过期，防止 read 锁与 write 锁之间
        // 有并发的 set 写入最新值。
        if let Some((_, expiry)) = store.get(key)
            && Instant::now() >= *expiry
        {
            store.remove(key);
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

    /// 主动清理所有已过期键，避免长时间运行后过期键累积。
    /// 通常由后台任务定期调用（例如每分钟一次）。
    async fn evict_expired(&self) -> usize {
        let now = Instant::now();
        let mut store = self.store.write().await;
        let before = store.len();
        store.retain(|_, (_, expiry)| now < *expiry);
        before - store.len()
    }
}

use tokio::sync::RwLock;
