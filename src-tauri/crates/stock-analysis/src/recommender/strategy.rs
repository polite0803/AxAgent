//! 智能荐股 — 子策略 trait

use crate::recommender::pool::SeedItem;
use crate::recommender::types::{Period, RecoPick, Style};
use async_trait::async_trait;
use axagent_astock_data::AStockClient;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};

/// 策略运行上下文
pub struct RecoContext<'a> {
    pub client: &'a AStockClient,
    /// 当前周期的 seed pool
    pub seed: &'a [SeedItem],
    /// per-code 互斥锁表：key=stock_code, value=该 code 的互斥锁
    /// 防止 4 个子策略对同一只股票并发打 vendor
    pub per_code_locks: Arc<PerCodeLocks>,
    /// 当前选中的 period
    pub period: Period,
    /// 模板变量映射表，策略用其读取可配置参数
    pub vars: &'a HashMap<String, Value>,
}

/// 从模板变量映射表中读取 f64 值，不存在或无法解析时返回 `default`
pub fn read_f64(vars: &HashMap<String, Value>, key: &str, default: f64) -> f64 {
    vars.get(key)
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        })
        .unwrap_or(default)
}

/// per-code 互斥锁表（线程安全）
#[derive(Default)]
pub struct PerCodeLocks {
    map: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl PerCodeLocks {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// 锁定指定 code：先在 map 里查 / 创建一个 `Arc<Mutex<()>>`，再 `lock_owned` 拿到 owned guard
    ///
    /// 关键点：owned guard 跟 Arc 绑，map 锁释放后也不影响 guard 有效性
    /// （Arc 本身在 map 里也存了一份，map 不会丢弃它）
    pub async fn lock_for(&self, code: &str) -> OwnedMutexGuard<()> {
        let code_lock: Arc<Mutex<()>> = {
            let mut g = self.map.lock().await;
            g.entry(code.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        code_lock.lock_owned().await
    }
}

#[async_trait]
pub trait RecommendStrategy: Send + Sync {
    fn id(&self) -> &'static str;
    fn style(&self) -> Style;
    fn period(&self) -> Period;
    /// 需要的 vendor 列表（任一启用即可，与 frontend PANEL_VENDORS 保持一致）
    fn required_vendors(&self) -> &'static [&'static str];
    /// 扫描 seed pool，返回本策略命中的 picks
    async fn scan(&self, ctx: &RecoContext<'_>) -> Result<Vec<RecoPick>, String>;
}
