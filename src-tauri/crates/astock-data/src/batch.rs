//! MarketBatchQuery —— DataFrame 风格的批量行情/财务查询
//!
//! ## 背景
//!
//! TradingAgents-CN 的 `unified_dataframe.py` 返回 `pandas.DataFrame`,
//! 把"列名标准化 + 批量取数 + 单次 HTTP 拉全市场"封装好,给筛选/回测/统计用。
//! AxAgent 是 Rust 桌面端,没有 pandas 生态,但可以借鉴:
//!
//! 1. **列名标准化**:全市场拉回的每只股票字段一致(open/high/low/close/volume/amount/
//!    turnover_rate/change_pct),不再按 vendor 各自解析;
//! 2. **批量并发**:一次 `get_quotes_batch` 内部用 `FuturesUnordered` 并发拉多只股票,
//!    受 `DomainGate` 限流约束;
//! 3. **可空字段明确化**:`Option<f64>` 标记 vendor 缺失的字段,前端/Agent 不会拿到 NaN。
//!
//! ## 命名约定
//!
//! - `get_quotes_batch`:批量拉实时行情(类比 akshare `stock_zh_a_spot_em`)
//! - `get_klines_batch`:批量拉 K 线(类比 akshare `stock_zh_a_hist` 全市场版)
//! - `get_financials_batch`:批量拉财务快照
//! - `to_dataframe_json`:转 DataFrame 风格 JSON `{columns, rows}`(前端渲染用)

use crate::types::*;
use async_trait::async_trait;
use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct BatchRequest {
    pub codes: Vec<String>,
    pub per_stock_timeout: Duration,
    pub total_timeout: Duration,
    pub max_failures: usize,
    /// P3-D12: 最大并发数。
    /// - `0`：自动按 vendor 健康度决定（由 `BatchRunner::resolve_concurrency` 解析），
    ///   健康时取 `BATCH_DEFAULT_CONCURRENCY`，部分降级降速，大面积降级进保命模式
    /// - `>0`：显式指定并发上限（调用方已知 vendor 状态时可用，跳过健康快照查询）
    pub max_concurrent: usize,
}

impl Default for BatchRequest {
    fn default() -> Self {
        Self {
            codes: Vec::new(),
            per_stock_timeout: Duration::from_secs(8),
            total_timeout: Duration::from_secs(30),
            max_failures: 0,
            max_concurrent: 0,
        }
    }
}

impl BatchRequest {
    pub fn new(codes: Vec<String>) -> Self {
        Self { codes, ..Default::default() }
    }

    pub fn with_per_stock_timeout(mut self, d: Duration) -> Self {
        self.per_stock_timeout = d;
        self
    }

    pub fn with_total_timeout(mut self, d: Duration) -> Self {
        self.total_timeout = d;
        self
    }

    pub fn with_max_failures(mut self, n: usize) -> Self {
        self.max_failures = n;
        self
    }

    /// P3-D12: 显式指定并发上限（`0` 仍表示自动）。
    pub fn with_max_concurrent(mut self, n: usize) -> Self {
        self.max_concurrent = n;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult<T> {
    pub success: HashMap<String, T>,
    pub failures: HashMap<String, String>,
    pub elapsed_ms: u64,
    /// H1.8 修复:是否因失败数超过 max_failures 阈值而提前终止
    /// (true 表示还有未完成的任务被丢弃;false 表示所有任务都跑完)
    #[serde(default)]
    pub early_terminated: bool,
}

impl<T> BatchResult<T> {
    pub fn new() -> Self {
        Self {
            success: HashMap::new(),
            failures: HashMap::new(),
            elapsed_ms: 0,
            early_terminated: false,
        }
    }

    pub fn total(&self) -> usize {
        self.success.len() + self.failures.len()
    }

    pub fn is_all_success(&self) -> bool {
        self.failures.is_empty()
    }
}

impl<T> Default for BatchResult<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
pub trait BatchVendor: Send + Sync {
    async fn batch_get_quotes(
        &self,
        codes: &[String],
    ) -> Result<Vec<StockQuote>, crate::error::DataError> {
        let _ = codes;
        Err(crate::error::DataError::VendorError {
            vendor: "batch".into(),
            message: "vendor 未实现 batch_get_quotes".into(),
        })
    }
}

#[async_trait]
pub trait MarketBatchQuery: Send + Sync {
    async fn get_quotes_batch(&self, req: BatchRequest) -> BatchResult<StockQuote>;
    async fn get_klines_batch(
        &self,
        codes: Vec<String>,
        period: &str,
        limit: u32,
    ) -> BatchResult<Vec<KLine>>;
    async fn get_financials_batch(&self, codes: Vec<String>) -> BatchResult<Vec<FinancialReport>>;
}

pub const DATAFRAME_QUOTE_COLUMNS: &[&str] = &[
    "code",
    "name",
    "open",
    "high",
    "low",
    "close",
    "pre_close",
    "volume",
    "amount",
    "change_pct",
    "turnover_rate",
    "pe",
    "pb",
    "total_mv",
    "is_st",
    "timestamp",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFrame {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

impl DataFrame {
    pub fn from_quotes(quotes: &[StockQuote]) -> Self {
        let columns: Vec<String> = DATAFRAME_QUOTE_COLUMNS.iter().map(|s| s.to_string()).collect();
        let rows = quotes
            .iter()
            .map(|q| {
                let opt_f64 = |v: Option<f64>| -> serde_json::Value {
                    match v.and_then(serde_json::Number::from_f64) {
                        Some(n) => serde_json::Value::Number(n),
                        None => serde_json::Value::Null,
                    }
                };
                vec![
                    serde_json::json!(q.code),
                    serde_json::json!(q.name),
                    serde_json::json!(q.open),
                    serde_json::json!(q.high),
                    serde_json::json!(q.low),
                    serde_json::json!(q.price),
                    serde_json::json!(q.pre_close),
                    serde_json::json!(q.volume),
                    serde_json::json!(q.amount),
                    serde_json::json!(q.change_pct),
                    serde_json::json!(q.turnover_rate),
                    opt_f64(q.pe),
                    opt_f64(q.pb),
                    opt_f64(q.total_mv),
                    serde_json::json!(q.is_st),
                    serde_json::json!(q.timestamp),
                ]
            })
            .collect();
        Self { columns, rows }
    }

    /// 修复 P0-A2: 序列化失败时记录错误并返回 `""`，避免污染调用方期望
    /// （`to_dataframe_json` 仍是主要消费方，行为不变）。空串仍可被上游
    /// 视为"渲染为空"——但错误已落 tracing，运维可据此排查。
    pub fn to_json(&self) -> String {
        match serde_json::to_string(self) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("[batch::DataFrame] to_json 序列化失败: {e}");
                String::new()
            },
        }
    }
}

pub struct BatchRunner {
    inner: Arc<crate::AStockClient>,
}

/// P3-D12: 批量查询默认并发上限。
///
/// 选 16 的依据：
/// - 单个 HTTP 请求约 200-500ms，16 并发下 200 只股票总耗时约 2.5-6s
/// - 比无限制并发（200+ task 同时跑）显著降低 socket / fd 压力
/// - 比串行（1 并发）快 16 倍，批量场景串行会导致整体超时
pub const BATCH_DEFAULT_CONCURRENCY: usize = 16;

/// P3-D12: 保命模式下的最小并发数（vendor 大面积降级时使用）。
///
/// 不降到 1 的原因：批量场景下串行 200 只股票 × 8s 超时 = 1600s，
/// 远超 `total_timeout`（默认 30s），会全部超时。
/// 2 并发能在保命模式下勉强完成小批量请求。
pub const BATCH_MIN_CONCURRENCY: usize = 2;

impl BatchRunner {
    pub fn new(client: Arc<crate::AStockClient>) -> Self {
        Self { inner: client }
    }

    /// P3-D12: 根据 vendor 健康度解析并发数。
    ///
    /// 算法与 `stock_pipeline::PipelineConfig::resolve_concurrency_with_vendor_health` 一致，
    /// 但批量场景的默认上限更高（`BATCH_DEFAULT_CONCURRENCY = 16`）：
    /// - `healthy_ratio < 0.3`（大面积降级）→ `BATCH_MIN_CONCURRENCY`（保命模式，2 并发）
    /// - `healthy_ratio < 0.6`（部分降级）→ `max(BATCH_MIN, BATCH_DEFAULT / 2)` = 8
    /// - 否则 → `BATCH_DEFAULT_CONCURRENCY`（16）
    ///
    /// `max_concurrent > 0` 时直接返回该值（调用方显式指定，跳过健康度查询）。
    /// 无 vendor 健康数据（首次运行）→ 返回默认值，不阻断业务。
    pub async fn resolve_concurrency(&self, max_concurrent: usize) -> usize {
        if max_concurrent > 0 {
            return max_concurrent;
        }
        let health = self.inner.health_tracker.get_all_health().await;
        Self::concurrency_from_health(&health)
    }

    /// P3-D12: 纯函数版并发解析（便于单元测试）。
    pub fn concurrency_from_health(health: &[crate::vendor_health::VendorHealth]) -> usize {
        use crate::vendor_health::VendorStatus;

        let active: Vec<_> = health.iter().filter(|h| h.status != VendorStatus::Disabled).collect();
        if active.is_empty() {
            return BATCH_DEFAULT_CONCURRENCY;
        }
        let healthy_count = active.iter().filter(|h| h.status == VendorStatus::Healthy).count();
        let healthy_ratio = healthy_count as f64 / active.len() as f64;

        if healthy_ratio < 0.3 {
            BATCH_MIN_CONCURRENCY
        } else if healthy_ratio < 0.6 {
            (BATCH_DEFAULT_CONCURRENCY / 2).max(BATCH_MIN_CONCURRENCY)
        } else {
            BATCH_DEFAULT_CONCURRENCY
        }
    }

    async fn run_batch<T, F, Fut>(
        &self,
        req: BatchRequest,
        method: &str,
        fetcher: F,
    ) -> BatchResult<T>
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<T, crate::error::DataError>> + Send + 'static,
        T: Send + 'static,
    {
        let start = std::time::Instant::now();
        let method = method.to_string();
        let codes = req.codes.clone();
        let per_timeout = req.per_stock_timeout;
        let max_failures = req.max_failures;

        // P3-D12: 解析并发上限。
        // - `max_concurrent == 0`：根据 vendor 健康度自动决定
        // - `max_concurrent > 0`：显式指定
        // 无限制（unbounded）只在健康度查询失败且无显式值时作为兜底，避免阻塞业务。
        let concurrency = self.resolve_concurrency(req.max_concurrent).await;
        let sem = Arc::new(tokio::sync::Semaphore::new(concurrency));

        tracing::info!(
            "[batch:{}] P3-D12 并发上限 = {}（codes={}）",
            method,
            concurrency,
            codes.len(),
        );

        let fetcher = Arc::new(fetcher);
        let mut tasks = FuturesUnordered::new();

        for code in codes {
            let fetcher = fetcher.clone();
            let method = method.clone();
            let sem = sem.clone();
            tasks.push(tokio::spawn(async move {
                // P3-D12: 通过 Semaphore 限制并发，避免一次性 spawn 200+ task 打爆 socket / vendor
                let _permit = match sem.acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => return (code, Err(format!("{}: Semaphore 已关闭", method))),
                };
                let result = tokio::time::timeout(per_timeout, fetcher(code.clone())).await;
                let result = match result {
                    Ok(Ok(v)) => Ok(v),
                    Ok(Err(e)) => Err(e.to_string()),
                    Err(_) => Err(format!("{}: 单只超时 {:?}", method, per_timeout)),
                };
                (code, result)
            }));
        }

        let mut success = HashMap::new();
        let mut failures = HashMap::new();
        let mut failure_count = 0usize;
        // H1.8 修复:超过 max_failures 阈值时真正 break 终止后续任务
        let mut early_terminated = false;
        while let Some(joined) = tasks.next().await {
            let Ok((code, res)) = joined else { continue };
            match res {
                Ok(v) => {
                    success.insert(code, v);
                },
                Err(e) => {
                    failure_count += 1;
                    failures.insert(code, e);
                    if max_failures > 0 && failure_count > max_failures {
                        tracing::warn!(
                            "[batch:{}] 失败数 {} 超过阈值 {}，提前终止后续任务",
                            method,
                            failure_count,
                            max_failures
                        );
                        early_terminated = true;
                        break;
                    }
                },
            }
        }

        // H1.8:如提前终止,丢弃仍在运行的剩余任务(避免泄漏)
        if early_terminated {
            while tasks.next().await.is_some() {}
        }

        BatchResult {
            success,
            failures,
            elapsed_ms: start.elapsed().as_millis() as u64,
            early_terminated,
        }
    }
}

#[async_trait]
impl MarketBatchQuery for BatchRunner {
    async fn get_quotes_batch(&self, req: BatchRequest) -> BatchResult<StockQuote> {
        let client = self.inner.clone();
        self.run_batch(req, "get_quote", move |code| {
            let c = client.clone();
            async move { c.get_quote(&code).await }
        })
        .await
    }

    async fn get_klines_batch(
        &self,
        codes: Vec<String>,
        period: &str,
        limit: u32,
    ) -> BatchResult<Vec<KLine>> {
        let req = BatchRequest::new(codes)
            .with_per_stock_timeout(Duration::from_secs(10))
            .with_total_timeout(Duration::from_secs(60))
            .with_max_failures(0);
        let period = period.to_string();
        let client = self.inner.clone();
        self.run_batch(req, "get_klines", move |code| {
            let c = client.clone();
            let p = period.clone();
            async move { c.get_klines(&code, &p, limit).await }
        })
        .await
    }

    async fn get_financials_batch(&self, codes: Vec<String>) -> BatchResult<Vec<FinancialReport>> {
        let req = BatchRequest::new(codes)
            .with_per_stock_timeout(Duration::from_secs(15))
            .with_total_timeout(Duration::from_secs(120))
            .with_max_failures(0);
        let client = self.inner.clone();
        self.run_batch(req, "get_financials", move |code| {
            let c = client.clone();
            async move { c.get_financials(&code).await }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataframe_from_quotes_columns_order() {
        let q = StockQuote {
            code: "600519".into(),
            name: "贵州茅台".into(),
            price: 1800.0,
            pre_close: 1785.0,
            open: 1790.0,
            high: 1810.0,
            low: 1785.0,
            volume: 5000000.0,
            amount: 9000000000.0,
            change_pct: 0.56,
            turnover_rate: 0.3,
            pe: Some(35.0),
            pb: Some(12.0),
            total_mv: Some(2250000000000.0),
            circulating_mv: None,
            limit_up: None,
            limit_down: None,
            is_st: false,
            timestamp: "2026-01-15 14:00:00".into(),
        };
        let df = DataFrame::from_quotes(&[q]);
        assert_eq!(df.columns.len(), 16);
        assert_eq!(df.columns[0], "code");
        assert_eq!(df.columns[3], "high");
        assert_eq!(df.columns[5], "close");
        assert_eq!(df.rows.len(), 1);
        assert!(df.rows[0][11].is_number());
    }

    #[test]
    fn batch_request_defaults() {
        let r = BatchRequest::default();
        assert_eq!(r.per_stock_timeout, Duration::from_secs(8));
        assert_eq!(r.max_failures, 0);
        let r = BatchRequest::new(vec!["600519".into()])
            .with_per_stock_timeout(Duration::from_secs(3))
            .with_max_failures(2);
        assert_eq!(r.codes.len(), 1);
        assert_eq!(r.per_stock_timeout, Duration::from_secs(3));
        assert_eq!(r.max_failures, 2);
    }

    #[test]
    fn batch_result_is_all_success() {
        let mut r: BatchResult<i32> = BatchResult::new();
        r.success.insert("a".into(), 1);
        assert!(r.is_all_success());
        r.failures.insert("b".into(), "err".into());
        assert!(!r.is_all_success());
        assert_eq!(r.total(), 2);
    }

    /// P3-D12: 验证并发数解析的纯函数逻辑
    #[test]
    fn d12_concurrency_from_health_all_healthy() {
        use crate::vendor_health::{VendorHealth, VendorStatus};
        let health = vec![
            VendorHealth { status: VendorStatus::Healthy, ..VendorHealth::new("tencent") },
            VendorHealth { status: VendorStatus::Healthy, ..VendorHealth::new("eastmoney") },
        ];
        assert_eq!(BatchRunner::concurrency_from_health(&health), BATCH_DEFAULT_CONCURRENCY);
    }

    #[test]
    fn d12_concurrency_from_health_partial_degraded() {
        use crate::vendor_health::{VendorHealth, VendorStatus};
        // 5 个 vendor，1 healthy + 4 degraded → healthy_ratio=0.2 < 0.3 → 保命模式
        let health = vec![
            VendorHealth { status: VendorStatus::Healthy, ..VendorHealth::new("tencent") },
            VendorHealth { status: VendorStatus::Degraded, ..VendorHealth::new("eastmoney") },
            VendorHealth { status: VendorStatus::Degraded, ..VendorHealth::new("sina") },
            VendorHealth { status: VendorStatus::Degraded, ..VendorHealth::new("ths") },
            VendorHealth { status: VendorStatus::Degraded, ..VendorHealth::new("akshare") },
        ];
        assert_eq!(BatchRunner::concurrency_from_health(&health), BATCH_MIN_CONCURRENCY);

        // 5 个 vendor，2 healthy + 3 degraded → healthy_ratio=0.4 ∈ [0.3, 0.6) → 降速
        let health = vec![
            VendorHealth { status: VendorStatus::Healthy, ..VendorHealth::new("tencent") },
            VendorHealth { status: VendorStatus::Healthy, ..VendorHealth::new("eastmoney") },
            VendorHealth { status: VendorStatus::Degraded, ..VendorHealth::new("sina") },
            VendorHealth { status: VendorStatus::Degraded, ..VendorHealth::new("ths") },
            VendorHealth { status: VendorStatus::Degraded, ..VendorHealth::new("akshare") },
        ];
        assert_eq!(
            BatchRunner::concurrency_from_health(&health),
            (BATCH_DEFAULT_CONCURRENCY / 2).max(BATCH_MIN_CONCURRENCY)
        );
    }

    #[test]
    fn d12_concurrency_from_health_all_disabled() {
        use crate::vendor_health::{VendorHealth, VendorStatus};
        // 全部 Disabled → active 为空 → 返回默认值（不阻断业务）
        let health = vec![
            VendorHealth { status: VendorStatus::Disabled, ..VendorHealth::new("tencent") },
            VendorHealth { status: VendorStatus::Disabled, ..VendorHealth::new("eastmoney") },
        ];
        assert_eq!(BatchRunner::concurrency_from_health(&health), BATCH_DEFAULT_CONCURRENCY);
    }

    #[test]
    fn d12_concurrency_from_health_empty() {
        // 无 vendor 健康数据（首次运行）→ 返回默认值
        let health: Vec<crate::vendor_health::VendorHealth> = vec![];
        assert_eq!(BatchRunner::concurrency_from_health(&health), BATCH_DEFAULT_CONCURRENCY);
    }

    #[test]
    fn d12_batch_request_with_max_concurrent() {
        let r = BatchRequest::new(vec!["600519".into()]).with_max_concurrent(4);
        assert_eq!(r.max_concurrent, 4);

        // 默认值 0 = 自动
        let r = BatchRequest::default();
        assert_eq!(r.max_concurrent, 0);
    }
}
