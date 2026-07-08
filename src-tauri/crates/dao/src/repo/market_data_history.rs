//! `market_data_history` 表的 L2 历史快照仓库
//!
//! 用于时间旅行模式下的市场数据持久化：同一 (vendor, method, code, as_of_date)
//! 的多次回放能命中同一份历史数据，避免对 vendor 重复请求。

use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde::{Deserialize, Serialize};

/// market_data_history 单条记录
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketDataEntry {
    pub id: Option<i64>,
    pub vendor: String,
    pub method: String,
    pub stock_code: String,
    pub as_of_date: String,
    pub data_window_start: Option<String>,
    pub data_window_end: Option<String>,
    pub payload_json: String,
    pub payload_hash: String,
    pub fetched_at: i64,
    pub last_accessed_at: i64,
    pub access_count: i64,
    pub expires_at: Option<i64>,
}

pub struct MarketDataHistoryRepo<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> MarketDataHistoryRepo<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 插入或忽略（命中 unique 索引则跳过；保留原始 fetch 时间）
    pub async fn upsert(&self, e: &MarketDataEntry) -> Result<(), sea_orm::DbErr> {
        let sql = "INSERT OR IGNORE INTO market_data_history (\
            vendor, method, stock_code, as_of_date, data_window_start, data_window_end, \
            payload_json, payload_hash, fetched_at, last_accessed_at, access_count, expires_at) \
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
            vec![
                e.vendor.clone().into(),
                e.method.clone().into(),
                e.stock_code.clone().into(),
                e.as_of_date.clone().into(),
                opt_string(&e.data_window_start),
                opt_string(&e.data_window_end),
                e.payload_json.clone().into(),
                e.payload_hash.clone().into(),
                e.fetched_at.into(),
                e.last_accessed_at.into(),
                e.access_count.into(),
                opt_i64(e.expires_at),
            ],
        );
        self.db.execute_raw(stmt).await?;
        Ok(())
    }

    /// 查找某个 (vendor, method, code, as_of) 下的最新一条记录
    pub async fn lookup(
        &self,
        vendor: &str,
        method: &str,
        code: &str,
        as_of: &str,
    ) -> Result<Option<MarketDataEntry>, sea_orm::DbErr> {
        let sql = "SELECT id, vendor, method, stock_code, as_of_date, data_window_start, \
            data_window_end, payload_json, payload_hash, fetched_at, last_accessed_at, \
            access_count, expires_at FROM market_data_history \
            WHERE vendor = ? AND method = ? AND stock_code = ? AND as_of_date = ? \
            AND (expires_at IS NULL OR expires_at > ?) \
            ORDER BY id DESC LIMIT 1";
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
            vec![
                vendor.into(),
                method.into(),
                code.into(),
                as_of.into(),
                chrono::Utc::now().timestamp().into(),
            ],
        );
        let row = self.db.query_one_raw(stmt).await?;
        Ok(row.map(row_to_entry))
    }

    /// 更新 last_accessed_at 与 access_count
    pub async fn touch(&self, id: i64) -> Result<(), sea_orm::DbErr> {
        let now = chrono::Utc::now().timestamp();
        let sql = "UPDATE market_data_history SET last_accessed_at = ?, access_count = access_count + 1 WHERE id = ?";
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
            vec![now.into(), id.into()],
        );
        self.db.execute_raw(stmt).await?;
        Ok(())
    }

    /// 删除过期数据（cron 周期调用）
    pub async fn purge_expired(&self, before: i64) -> Result<u64, sea_orm::DbErr> {
        let sql = "DELETE FROM market_data_history WHERE expires_at IS NOT NULL AND expires_at < ?";
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
            vec![before.into()],
        );
        let result = self.db.execute_raw(stmt).await?;
        Ok(result.rows_affected())
    }

    /// 按 LRU 清理到剩余 count 条
    pub async fn lru_trim(&self, keep_count: i64) -> Result<u64, sea_orm::DbErr> {
        let sql = "DELETE FROM market_data_history WHERE id NOT IN (\
            SELECT id FROM market_data_history ORDER BY last_accessed_at DESC LIMIT ?)";
        let stmt = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
            vec![keep_count.into()],
        );
        let result = self.db.execute_raw(stmt).await?;
        Ok(result.rows_affected())
    }
}

fn opt_string(s: &Option<String>) -> sea_orm::Value {
    match s {
        Some(v) => sea_orm::Value::String(Some(v.clone())),
        None => sea_orm::Value::String(None),
    }
}

fn opt_i64(s: Option<i64>) -> sea_orm::Value {
    match s {
        Some(v) => sea_orm::Value::BigInt(Some(v)),
        None => sea_orm::Value::BigInt(None),
    }
}

fn row_to_entry(row: sea_orm::QueryResult) -> MarketDataEntry {
    // Use a helper that returns Option by trying to get, defaulting on error
    fn try_str(row: &sea_orm::QueryResult, idx: usize) -> Option<String> {
        row.try_get_by::<Option<String>, _>(idx).ok().flatten()
    }
    fn try_i64(row: &sea_orm::QueryResult, idx: usize) -> Option<i64> {
        row.try_get_by::<i64, _>(idx).ok()
    }

    MarketDataEntry {
        id: try_i64(&row, 0),
        vendor: try_str(&row, 1).unwrap_or_default(),
        method: try_str(&row, 2).unwrap_or_default(),
        stock_code: try_str(&row, 3).unwrap_or_default(),
        as_of_date: try_str(&row, 4).unwrap_or_default(),
        data_window_start: try_str(&row, 5),
        data_window_end: try_str(&row, 6),
        payload_json: try_str(&row, 7).unwrap_or_default(),
        payload_hash: try_str(&row, 8).unwrap_or_default(),
        fetched_at: try_i64(&row, 9).unwrap_or(0),
        last_accessed_at: try_i64(&row, 10).unwrap_or(0),
        access_count: try_i64(&row, 11).unwrap_or(0),
        expires_at: try_i64(&row, 12),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(as_of: &str) -> MarketDataEntry {
        MarketDataEntry {
            id: None,
            vendor: "eastmoney".into(),
            method: "klines".into(),
            stock_code: "000001".into(),
            as_of_date: as_of.into(),
            data_window_start: Some("2025-06-01".into()),
            data_window_end: Some(as_of.into()),
            payload_json: r#"[{"date":"2026-06-01","close":10.5}]"#.into(),
            payload_hash: "h1".into(),
            fetched_at: 1_700_000_000,
            last_accessed_at: 1_700_000_000,
            access_count: 0,
            expires_at: Some(1_900_000_000),
        }
    }

    #[test]
    fn entry_serialization_round_trip() {
        let e = sample_entry("2026-06-01");
        let s = serde_json::to_string(&e).unwrap();
        let back: MarketDataEntry = serde_json::from_str(&s).unwrap();
        assert_eq!(e.vendor, back.vendor);
        assert_eq!(e.as_of_date, back.as_of_date);
        assert_eq!(e.payload_hash, back.payload_hash);
        assert_eq!(e.expires_at, back.expires_at);
    }

    #[test]
    fn opt_string_some() {
        let v = opt_string(&Some("abc".into()));
        match v {
            sea_orm::Value::String(Some(s)) => assert_eq!(&*s, "abc"),
            other => panic!("expected String(Some), got {other:?}"),
        }
    }

    #[test]
    fn opt_string_none() {
        let v = opt_string(&None);
        assert!(matches!(v, sea_orm::Value::String(None)));
    }

    #[test]
    fn opt_i64_some() {
        let v = opt_i64(Some(42));
        assert!(matches!(v, sea_orm::Value::BigInt(Some(42))));
    }

    #[test]
    fn opt_i64_none() {
        let v = opt_i64(None);
        assert!(matches!(v, sea_orm::Value::BigInt(None)));
    }
}
