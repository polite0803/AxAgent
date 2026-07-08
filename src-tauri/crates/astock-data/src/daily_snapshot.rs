//! 每日快照缓存(P5:本地 SQLite 缓存简化版)
//!
//! 为 NoHistoricalSemantic 方法(热门股/行业排名/概念板块/快讯等)
//! 提供"每日快照"缓存。这些数据本身没有历史语义——每日快照是在每天
//! 某个时间点调用 vendor 实时接口获取的"那一刻的今日数据"。
//!
//! 存储:复用 DiskCache(文件 JSON,LRU 淘汰),Key 格式 `daily:{method}:{date}`。
//! 隔离:L2 磁盘缓存是方法级的短暂缓存(30s-5min),每日快照是"日粒度"的持久缓存。
//!
//! 使用模式:
//! 1. 后台 cron 每天调用 sweep_daily() 一次,存入当日快照
//! 2. replay 模式遇到 NoHistoricalSemantic 方法,先查每日快照
//! 3. cache miss → 正常走 record_degradation + 返回空(不阻塞回测)
//!
//! 配置:通过 AStockClient.with_daily_snapshot_cache() 注入,默认关闭。
//! 启用后若 cache miss,不降级记录(因为是用户主动开启的,应预期可用)。

use crate::disk_cache::DiskCache;
use std::sync::Arc;

/// 每日快照缓存 Key 前缀
const SNAPSHOT_PREFIX: &str = "daily";

/// 支持的 NoHistoricalSemantic 方法列表
pub const SNAPSHOT_METHODS: &[&str] = &[
    "get_hot_stocks",
    "get_industry_ranking",
    "get_cls_flash",
    "get_concept_blocks", // NoHistoricalSemantic: 概念板块，vendor asof_capability 用此名
    "search_stock",
    "get_sector_info",
    "get_money_flow",
    "get_north_bound_holding",
    "get_margin_data",
    "get_index_quotes",
    "get_stock_announcements",
    // get_market_dragon_tiger / get_board_fund_flow 等全市场快照可后续补充
];

/// 需要遍历个股的 per-stock 快照方法（相对于全市场方法）
pub const PER_STOCK_METHODS: &[&str] = &[
    "get_money_flow",
    "get_north_bound_holding",
    "get_margin_data",
];

/// 每日快照缓存
///
/// 零成本抽象:只有配置了 DiskCache 才创建实例;None 表示"未启用"。
#[derive(Clone)]
pub struct DailySnapshotCache {
    disk: Arc<DiskCache>,
}

impl DailySnapshotCache {
    /// 从已存在的 DiskCache 创建每日快照缓存
    pub fn from_disk(disk: Arc<DiskCache>) -> Self {
        Self { disk }
    }

    /// 执行一次完整的每日快照采集（由 Tauri command 调用）
    /// : 全市场方法的结果 json，key 为 method 名
    /// : 逐个股票采集的结果，key 为 "{method}:{stock_code}"
    /// Tue Jun 16 18:13:05     2026: 快照日期 YYYY-MM-DD
    /// 存入全市场快照（热门股、行业排名、快讯、概念板块等）
    /// 由 Tauri command sweep_daily_snapshots 采集后调用
    pub fn set_snapshot(&self, method: &str, date: &str, json: &str) {
        let key = Self::cache_key(method, date);
        self.disk.set(key, json.to_string(), 0i64);
    }

    /// 存入个股级快照（资金流向、北向持仓等），key 含股票代码
    /// 调用方遍历股票列表逐只采集后逐只存入
    pub fn set_stock_snapshot(&self, method: &str, stock_code: &str, date: &str, json: &str) {
        let key = format!("{SNAPSHOT_PREFIX}:{method}:{stock_code}:{date}");
        self.disk.set(key, json.to_string(), 0i64);
    }

    fn cache_key(method: &str, date: &str) -> String {
        format!("{SNAPSHOT_PREFIX}:{method}:{date}")
    }

    /// 获取指定方法 + 日期的快照
    /// 返回 None 表示未命中缓存(或未启用)
    pub fn get(&self, method: &str, date: &str) -> Option<String> {
        let key = Self::cache_key(method, date);
        self.disk.get(&key)
    }

    /// 存入快照(TTL = 0 表示不过期,DiskCache 按 LRU 淘汰)
    pub fn set(&self, method: &str, date: &str, value: &str) {
        let key = Self::cache_key(method, date);
        self.disk.set(key, value.to_string(), 0i64);
    }

    /// 检查特定方法是否已缓存(避免反序列化大对象)
    pub fn has(&self, method: &str, date: &str) -> bool {
        let key = Self::cache_key(method, date);
        self.disk.get(&key).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_cache() -> DailySnapshotCache {
        let dir = std::env::temp_dir().join("astock_daily_snapshot_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_cache.json");
        let _ = std::fs::remove_file(&path);
        let disk = DiskCache::load_or_default(path);
        DailySnapshotCache::from_disk(disk)
    }

    #[test]
    fn test_cache_key_format() {
        let key = DailySnapshotCache::cache_key("get_hot_stocks", "2026-06-01");
        assert_eq!(key, "daily:get_hot_stocks:2026-06-01");
    }

    #[test]
    fn test_set_get_roundtrip() {
        let cache = make_cache();
        let data =
            r#"[{"stock_code":"000001","stockName":"平安银行","price":10.0,"changePct":1.0}]"#;
        cache.set("get_hot_stocks", "2026-06-01", data);
        let back = cache.get("get_hot_stocks", "2026-06-01");
        assert!(back.is_some());
        let back_str = back.unwrap();
        assert!(back_str.contains("000001"));
    }

    #[test]
    fn test_has_method() {
        let cache = make_cache();
        assert!(!cache.has("get_hot_stocks", "2026-06-01"));
        cache.set("get_hot_stocks", "2026-06-01", "test");
        assert!(cache.has("get_hot_stocks", "2026-06-01"));
    }

    #[test]
    fn test_miss_returns_none() {
        let cache = make_cache();
        let result = cache.get("get_hot_stocks", "2099-01-01");
        assert!(result.is_none());
    }

    #[test]
    fn test_multiple_dates() {
        let cache = make_cache();
        cache.set("get_hot_stocks", "2026-06-01", "data_jun1");
        cache.set("get_hot_stocks", "2026-06-02", "data_jun2");
        let d1 = cache.get("get_hot_stocks", "2026-06-01");
        let d2 = cache.get("get_hot_stocks", "2026-06-02");
        assert_eq!(d1, Some("data_jun1".to_string()));
        assert_eq!(d2, Some("data_jun2".to_string()));
    }

    #[test]
    fn test_different_methods_same_date() {
        let cache = make_cache();
        cache.set("get_hot_stocks", "2026-06-01", "hot");
        cache.set("get_industry_ranking", "2026-06-01", "industry");
        let hot = cache.get("get_hot_stocks", "2026-06-01");
        let ind = cache.get("get_industry_ranking", "2026-06-01");
        assert_eq!(hot, Some("hot".to_string()));
        assert_eq!(ind, Some("industry".to_string()));
    }

    #[test]
    fn test_snapshot_methods_list_contains_expected() {
        assert!(SNAPSHOT_METHODS.contains(&"get_hot_stocks"));
        assert!(SNAPSHOT_METHODS.contains(&"get_industry_ranking"));
        assert!(SNAPSHOT_METHODS.contains(&"get_cls_flash"));
        assert!(SNAPSHOT_METHODS.contains(&"get_concept_blocks"));
        assert!(SNAPSHOT_METHODS.contains(&"search_stock"));
        assert!(SNAPSHOT_METHODS.contains(&"get_sector_info"));
    }
}
