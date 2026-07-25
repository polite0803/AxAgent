//! 实时行情推送 — 轮询版（HTTP polling 兜底，后续可升级 WebSocket）
//!
//! ## 设计
//!
//! 用 tokio 任务池轮询 `AStockClient.get_quote()`，检测变动后通过回调推送。
//! 按活跃度分两级频率：
//! - **Active**（用户当前查看 / 有持仓）：~2s
//! - **Background**（仅监控）：~10s
//!
//! ## 使用
//!
//! ```rust,no_run
//! let watcher = RealTimeQuoteWatcher::new(client, callback);
//! watcher.watch("600519", WatchPriority::Active).await;
//! watcher.start().await; // 启动后台轮询任务
//! ```

use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::{AStockClient, StockQuote};

/// 监控优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WatchPriority {
    /// 主动监控（~2s 轮询）
    Active,
    /// 背景监控（~10s 轮询）
    Background,
}

impl WatchPriority {
    fn interval_ms(&self) -> u64 {
        match self {
            WatchPriority::Active => 2000,
            WatchPriority::Background => 10000,
        }
    }
}

/// 行情变更事件
#[derive(Debug, Clone)]
pub struct QuoteChangeEvent {
    pub stock_code: String,
    pub previous: Option<StockQuote>,
    pub current: StockQuote,
    /// 当日涨跌幅（相对于前收盘）
    pub change_pct: f64,
    /// 触发类型: "tick" | "price_change" | "significant_move"
    pub trigger: String,
}

/// 行情变更回调（由 wiring 层注入 Tauri `app.emit`）
pub type QuoteCallback = Arc<dyn Fn(QuoteChangeEvent) -> BoxFuture<'static, ()> + Send + Sync>;

/// 实时行情监视器
pub struct RealTimeQuoteWatcher {
    client: Arc<AStockClient>,
    /// 被监控的股票代码 → 优先级
    watches: Arc<RwLock<HashMap<String, WatchPriority>>>,
    /// 上次快照（用于检测变化）
    last_quotes: Arc<RwLock<HashMap<String, StockQuote>>>,
    /// 变更回调
    callback: Option<QuoteCallback>,
    /// 是否正在运行
    running: Arc<AtomicBool>,
}

impl RealTimeQuoteWatcher {
    pub fn new(client: Arc<AStockClient>, callback: Option<QuoteCallback>) -> Self {
        Self {
            client,
            watches: Arc::new(RwLock::new(HashMap::new())),
            last_quotes: Arc::new(RwLock::new(HashMap::new())),
            callback,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 添加监控股票
    pub async fn watch(&self, code: &str, priority: WatchPriority) {
        self.watches.write().await.insert(code.to_string(), priority);
    }

    /// 批量添加监控股票
    pub async fn watch_many(&self, codes: &[&str], priority: WatchPriority) {
        let mut w = self.watches.write().await;
        for code in codes {
            w.insert(code.to_string(), priority);
        }
    }

    /// 移除监控
    pub async fn unwatch(&self, code: &str) {
        self.watches.write().await.remove(code);
        self.last_quotes.write().await.remove(code);
    }

    /// 获取当前监控列表
    pub async fn watched_stocks(&self) -> Vec<String> {
        self.watches.read().await.keys().cloned().collect()
    }

    /// 设置优先级
    pub async fn set_priority(&self, code: &str, priority: WatchPriority) {
        self.watches.write().await.insert(code.to_string(), priority);
    }

    /// 启动后台轮询（spawn 一个 tokio 任务）
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        self.running.store(true, Ordering::Relaxed);

        let watches = self.watches.clone();
        let last_quotes = self.last_quotes.clone();
        let client = self.client.clone();
        let callback = self.callback.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            tracing::info!("[RealTimeQuoteWatcher] 启动");

            loop {
                if !running.load(Ordering::Relaxed) {
                    break;
                }

                let codes: Vec<(String, WatchPriority)> =
                    { watches.read().await.iter().map(|(k, v)| (k.clone(), *v)).collect() };

                if codes.is_empty() {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }

                let active_codes: Vec<&str> = codes
                    .iter()
                    .filter(|(_, p)| *p == WatchPriority::Active)
                    .map(|(c, _)| c.as_str())
                    .collect();
                let bg_codes: Vec<&str> = codes
                    .iter()
                    .filter(|(_, p)| *p == WatchPriority::Background)
                    .map(|(c, _)| c.as_str())
                    .collect();

                if !active_codes.is_empty() {
                    poll_and_emit(&client, &last_quotes, &callback, &active_codes).await;
                }

                static BG_COUNTER: AtomicU32 = AtomicU32::new(0);
                let counter = BG_COUNTER.fetch_add(1, Ordering::Relaxed);
                if counter.is_multiple_of(5) && !bg_codes.is_empty() {
                    poll_and_emit(&client, &last_quotes, &callback, &bg_codes).await;
                }

                let min_interval = codes.iter().map(|(_, p)| p.interval_ms()).min().unwrap_or(2000);
                tokio::time::sleep(Duration::from_millis(min_interval)).await;
            }

            tracing::info!("[RealTimeQuoteWatcher] 已停止");
        })
    }

    /// 停止轮询
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

async fn poll_and_emit(
    client: &AStockClient,
    last_quotes: &RwLock<HashMap<String, StockQuote>>,
    callback: &Option<QuoteCallback>,
    codes: &[&str],
) {
    for code in codes {
        match client.get_quote(code).await {
            Ok(quote) => {
                let prev = last_quotes.write().await.insert(code.to_string(), quote.clone());
                let change_pct = if let Some(ref p) = prev {
                    if p.pre_close > 0.0 {
                        (quote.price - p.pre_close) / p.pre_close * 100.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                // 仅在有变化或首次获取时触发回调
                let should_emit = match &prev {
                    Some(prev_q) => {
                        // 价格变化超过 0.01 元 或 涨跌幅变化
                        (quote.price - prev_q.price).abs() > 0.01
                    },
                    None => true,
                };

                if should_emit {
                    let trigger = if prev.is_none() {
                        "tick"
                    } else if change_pct.abs() >= 3.0 {
                        "significant_move"
                    } else {
                        "price_change"
                    };

                    let event = QuoteChangeEvent {
                        stock_code: code.to_string(),
                        previous: prev,
                        current: quote,
                        change_pct,
                        trigger: trigger.to_string(),
                    };

                    if let Some(ref cb) = callback {
                        cb(event).await;
                    }
                }
            },
            Err(e) => {
                tracing::trace!("[RealTimeQuoteWatcher] {code} 行情拉取失败: {e}");
            },
        }
    }
}

// ── P3: MarketDataStreamer 实现 ──────────────────────────────────────────

/// HTTP 轮询行情流式推送器（P3: WebSocket 升级架构占位）
///
/// 实现 `axagent_harness::market_data::MarketDataStreamer` trait,
/// 包装 `AStockClient` 用 tokio::interval 轮询生成 `QuoteUpdate` 流。
///
/// 与 `RealTimeQuoteWatcher` 的区别：
/// - `RealTimeQuoteWatcher`：固定后台轮询，通过 `QuoteCallback` 推送
/// - `HttpPollingStreamer`：按需 `subscribe(codes)`，返回 mpsc Receiver
///   供 consumer（如 gateway ws handler）拉取
///
/// 当前是默认数据源。未来可由 `WebSocketStreamer` 替代（架构无需改 consumer）。
pub struct HttpPollingStreamer {
    client: Arc<AStockClient>,
    /// 轮询间隔（毫秒），默认 2000ms
    poll_interval_ms: u64,
}

impl HttpPollingStreamer {
    pub fn new(client: Arc<AStockClient>) -> Self {
        Self { client, poll_interval_ms: 2000 }
    }

    /// 自定义轮询间隔（例如 5000ms 降低压力）
    pub fn with_interval(mut self, interval_ms: u64) -> Self {
        self.poll_interval_ms = interval_ms;
        self
    }
}

#[async_trait::async_trait]
impl axagent_harness::market_data::MarketDataStreamer for HttpPollingStreamer {
    async fn subscribe(
        &self,
        codes: Vec<String>,
    ) -> Result<
        tokio::sync::mpsc::Receiver<axagent_harness::market_data::QuoteUpdate>,
        axagent_harness::core_error::AxAgentError,
    > {
        use axagent_harness::core_error::AxAgentError;
        use axagent_harness::market_data::QuoteUpdate;

        if codes.is_empty() {
            return Err(AxAgentError::Validation("subscribe codes 不能为空".to_string()));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let client = self.client.clone();
        let interval_ms = self.poll_interval_ms;
        let source = self.source_type().to_string();

        tokio::spawn(async move {
            let mut last_quotes: HashMap<String, StockQuote> = HashMap::new();

            loop {
                let mut any_change = false;
                for code in &codes {
                    match client.get_quote(code).await {
                        Ok(quote) => {
                            let prev = last_quotes.insert(code.clone(), quote.clone());
                            let change_pct = if let Some(ref p) = prev {
                                if p.pre_close > 0.0 {
                                    (quote.price - p.pre_close) / p.pre_close * 100.0
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            };

                            // 仅在价格变化超过 0.01 元 或 首次获取时推送
                            let should_emit = match &prev {
                                Some(prev_q) => (quote.price - prev_q.price).abs() > 0.01,
                                None => true,
                            };

                            if should_emit {
                                any_change = true;
                                let trigger = if prev.is_none() {
                                    "tick"
                                } else if change_pct.abs() >= 3.0 {
                                    "significant_move"
                                } else {
                                    "price_change"
                                };

                                let update = QuoteUpdate {
                                    stock_code: code.clone(),
                                    current: quote,
                                    change_pct,
                                    trigger: trigger.to_string(),
                                    source: source.clone(),
                                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                                };

                                if tx.send(update).await.is_err() {
                                    // 接收方已 drop，结束轮询
                                    tracing::debug!(
                                        "[HttpPollingStreamer] 接收方关闭，停止 {code} 轮询"
                                    );
                                    return;
                                }
                            }
                        },
                        Err(e) => {
                            tracing::trace!("[HttpPollingStreamer] {code} 行情拉取失败: {e}");
                        },
                    }
                }

                // 无变化时缩短 sleep，有变化时按配置间隔轮询
                let sleep_ms = if any_change {
                    interval_ms
                } else {
                    interval_ms.min(5000)
                };
                tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
            }
        });

        Ok(rx)
    }

    fn source_type(&self) -> &'static str {
        "http_poll"
    }
}
