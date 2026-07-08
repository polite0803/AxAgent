//! moka + redb 双层缓存

use crate::error::DataError;
use moka::future::Cache;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct L1Config {
    pub max_capacity: u64,
    pub default_ttl: Duration,
}

impl Default for L1Config {
    fn default() -> Self {
        Self {
            max_capacity: 4096,
            default_ttl: Duration::from_secs(60),
        }
    }
}

pub struct L1Cache {
    inner: Cache<String, String>,
}

impl L1Cache {
    pub fn new(cfg: L1Config) -> Arc<Self> {
        let inner = Cache::builder()
            .max_capacity(cfg.max_capacity)
            .time_to_live(cfg.default_ttl)
            .build();
        Arc::new(Self { inner })
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        self.inner.get(key).await
    }

    pub async fn set(&self, key: String, value: String, _ttl: Duration) {
        self.inner.insert(key, value).await;
    }

    pub async fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }
}

const TABLE: TableDefinition<&str, &str> = TableDefinition::new("axagent_cache");

pub struct L2Cache {
    db: Database,
}

impl L2Cache {
    pub fn open(path: PathBuf) -> Result<Arc<Self>, DataError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(DataError::IoError)?;
        }
        let db = Database::create(&path).map_err(|e| DataError::VendorError {
            vendor: "redb".into(),
            message: format!("open {}: {e}", path.display()),
        })?;
        let txn = db.begin_write().map_err(|e| DataError::VendorError {
            vendor: "redb".into(),
            message: format!("begin_write: {e}"),
        })?;
        {
            let _ = txn.open_table(TABLE).map_err(|e| DataError::VendorError {
                vendor: "redb".into(),
                message: format!("open_table: {e}"),
            })?;
        }
        txn.commit().map_err(|e| DataError::VendorError {
            vendor: "redb".into(),
            message: format!("commit init: {e}"),
        })?;
        Ok(Arc::new(Self { db }))
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let txn = self.db.begin_read().ok()?;
        let table = txn.open_table(TABLE).ok()?;
        let val = table.get(key).ok()??;
        Some(val.value().to_string())
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), DataError> {
        let txn = self.db.begin_write().map_err(|e| DataError::VendorError {
            vendor: "redb".into(),
            message: format!("begin_write: {e}"),
        })?;
        {
            let mut table = txn.open_table(TABLE).map_err(|e| DataError::VendorError {
                vendor: "redb".into(),
                message: format!("open_table: {e}"),
            })?;
            table
                .insert(key, value)
                .map_err(|e| DataError::VendorError {
                    vendor: "redb".into(),
                    message: format!("insert: {e}"),
                })?;
        }
        txn.commit().map_err(|e| DataError::VendorError {
            vendor: "redb".into(),
            message: format!("commit: {e}"),
        })?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<(), DataError> {
        let txn = self.db.begin_write().map_err(|e| DataError::VendorError {
            vendor: "redb".into(),
            message: format!("begin_write: {e}"),
        })?;
        {
            let mut table = txn.open_table(TABLE).map_err(|e| DataError::VendorError {
                vendor: "redb".into(),
                message: format!("open_table: {e}"),
            })?;
            table.remove(key).map_err(|e| DataError::VendorError {
                vendor: "redb".into(),
                message: format!("remove: {e}"),
            })?;
        }
        txn.commit().map_err(|e| DataError::VendorError {
            vendor: "redb".into(),
            message: format!("commit: {e}"),
        })?;
        Ok(())
    }

    pub fn approx_len(&self) -> usize {
        let Ok(txn) = self.db.begin_read() else { return 0 };
        let Ok(table) = txn.open_table(TABLE) else { return 0 };
        let mut count = 0usize;
        let Ok(iter) = table.iter() else { return 0 };
        for _ in iter {
            count += 1;
            if count > 100_000 {
                break;
            }
        }
        count
    }
}

pub struct TwoTierCache {
    pub l1: Arc<L1Cache>,
    pub l2: Option<Arc<L2Cache>>,
}

impl TwoTierCache {
    pub fn new(l1: Arc<L1Cache>, l2: Option<Arc<L2Cache>>) -> Self {
        Self { l1, l2 }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        if let Some(v) = self.l1.get(key).await {
            return Some(v);
        }
        if let Some(l2) = &self.l2 {
            if let Some(v) = l2.get(key) {
                self.l1
                    .set(key.to_string(), v.clone(), Duration::from_secs(60))
                    .await;
                return Some(v);
            }
        }
        None
    }

    pub async fn set(&self, key: String, value: String, ttl: Duration) {
        self.l1.set(key.clone(), value.clone(), ttl).await;
        if let Some(l2) = &self.l2 {
            let _ = l2.set(&key, &value);
        }
    }

    pub async fn stats(&self) -> CacheStats {
        CacheStats {
            l1_entries: self.l1.entry_count().await,
            l2_entries: self.l2.as_ref().map(|l| l.approx_len()).unwrap_or(0),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub l1_entries: u64,
    pub l2_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmp_l2_path(name: &str) -> PathBuf {
        let mut p = env::temp_dir();
        p.push(format!("axagent_l2_{}_{}.redb", name, std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[tokio::test]
    async fn l1_basic_set_get() {
        let l1 = L1Cache::new(L1Config::default());
        l1.set("k1".into(), "v1".into(), Duration::from_secs(10))
            .await;
        assert_eq!(l1.get("k1").await.as_deref(), Some("v1"));
        assert!(l1.get("missing").await.is_none());
    }

    #[test]
    fn l2_set_get_roundtrip() {
        let path = tmp_l2_path("roundtrip");
        let l2 = L2Cache::open(path).unwrap();
        l2.set("k", "v").unwrap();
        assert_eq!(l2.get("k").as_deref(), Some("v"));
        assert!(l2.get("missing").is_none());
    }

    #[test]
    fn l2_delete() {
        let path = tmp_l2_path("delete");
        let l2 = L2Cache::open(path).unwrap();
        l2.set("k", "v").unwrap();
        l2.delete("k").unwrap();
        assert!(l2.get("k").is_none());
    }

    #[test]
    fn l2_persistence_across_reopen() {
        let path = tmp_l2_path("persist");
        {
            let l2 = L2Cache::open(path.clone()).unwrap();
            l2.set("code", "600519").unwrap();
        }
        let l2 = L2Cache::open(path).unwrap();
        assert_eq!(l2.get("code").as_deref(), Some("600519"));
    }

    #[tokio::test]
    async fn two_tier_l1_only() {
        let l1 = L1Cache::new(L1Config::default());
        let c = TwoTierCache::new(l1, None);
        c.set("k".into(), "v".into(), Duration::from_secs(5)).await;
        assert_eq!(c.get("k").await.as_deref(), Some("v"));
    }

    #[tokio::test]
    async fn two_tier_l2_fallback() {
        let l1 = L1Cache::new(L1Config::default());
        let l2 = L2Cache::open(tmp_l2_path("tier")).unwrap();
        l2.set("k2", "v2").unwrap();

        let c = TwoTierCache::new(l1, Some(l2));
        assert_eq!(c.get("k2").await.as_deref(), Some("v2"));
    }
}
