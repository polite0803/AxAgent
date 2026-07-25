use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use axagent_harness::market_data::{MarketDataProvider, StockQuote};

/// 前端事件推送桥接 trait
///
/// 由 wiring 层（如 Tauri AppHandle 包装）注入具体实现，
/// 使 stock-analysis crate 不直接依赖 Tauri 框架。
/// 未注入时（None）告警仅走内部 broadcast channel。
pub trait MonitorEventEmitter: Send + Sync {
    fn emit(&self, event: &str, payload: serde_json::Value);
}

/// T+0 自动重跑配置 (P2-6)
///
/// 当 RealtimeMonitor 检测到异常行情时,除了发告警,还会触发 T+0 工作流重跑
/// (即在交易时段内立即用最新行情重新跑一次 stock analysis workflow)。
/// - `enabled`: 全局开关
/// - `change_pct_threshold`: 涨跌幅超过 N% 触发 (None 表示不按此条件触发)
/// - `turnover_rate_threshold`: 换手率超过 N% 触发 (None 表示不按此条件触发)
/// - `min_interval_minutes`: 同一只股票两次 T+0 触发的最小间隔 (默认 30 分钟),
///   避免短时间内连续触发造成工作流风暴
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TZeroConfig {
    pub enabled: bool,
    pub change_pct_threshold: Option<f64>,
    pub turnover_rate_threshold: Option<f64>,
    pub min_interval_minutes: i64,
}

impl Default for TZeroConfig {
    fn default() -> Self {
        Self {
            enabled: false, // 默认关闭 — 用户需在设置里显式开启
            change_pct_threshold: Some(3.0),
            turnover_rate_threshold: Some(8.0),
            min_interval_minutes: 30,
        }
    }
}

/// T+0 触发回调: 接收 stock_code,异步启动工作流重跑
pub type TZeroCallback = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> + Send + Sync,
>;

/// 监控条件
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonitorConfig {
    pub stock_code: String,
    pub stock_name: String,
    /// 止损价（跌破告警）
    pub stop_loss: Option<f64>,
    /// 止盈价（突破告警）
    pub take_profit: Option<f64>,
    /// 突破压力位告警
    pub resistance_break: Option<f64>,
    /// 跌破支撑位告警
    pub support_break: Option<f64>,
    /// 涨跌幅超过N%告警
    pub change_pct_alert: Option<f64>,
    /// 成交量异常（换手率>N告警）
    pub turnover_rate_alert: Option<f64>,
    /// 是否启用
    pub enabled: bool,
}

/// 告警事件
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorAlert {
    pub stock_code: String,
    pub stock_name: String,
    /// "stop_loss" | "take_profit" | "resistance" | "support" | "change" | "volume"
    pub alert_type: String,
    pub alert_message: String,
    pub current_price: f64,
    pub change_pct: f64,
    pub timestamp: String,
    /// 操作建议（如 "考虑减仓50%，现价低于止损"）
    pub suggested_action: Option<String>,
}

/// 实时监控引擎
pub struct RealtimeMonitor {
    client: Arc<dyn MarketDataProvider>,
    configs: RwLock<HashMap<String, MonitorConfig>>,
    alert_tx: tokio::sync::broadcast::Sender<MonitorAlert>,
    running: RwLock<bool>,
    event_emitter: RwLock<Option<Arc<dyn MonitorEventEmitter>>>,
    last_alerts: RwLock<HashMap<String, i64>>,
    poll_interval_secs: RwLock<u64>,
    alert_cooldown_secs: RwLock<i64>,
    // P2-6: T+0 自动重跑相关字段
    t0_config: RwLock<TZeroConfig>,
    t0_callback: RwLock<Option<TZeroCallback>>,
    /// 记录每只股票最近一次 T+0 触发的时间戳(秒),用于 cooldown 控制
    t0_last_trigger_ts: RwLock<HashMap<String, i64>>,
    /// P3-3: 跨股票信号聚合器（组合级告警）
    /// 不设置时为 None，单股告警不会聚合到组合层
    aggregator: RwLock<Option<Arc<crate::cross_stock_aggregator::CrossStockSignalAggregator>>>,
}

impl RealtimeMonitor {
    pub fn new(client: Arc<dyn MarketDataProvider>) -> Self {
        let (alert_tx, _) = tokio::sync::broadcast::channel(128);
        Self {
            client,
            configs: RwLock::new(HashMap::new()),
            alert_tx,
            running: RwLock::new(false),
            event_emitter: RwLock::new(None),
            last_alerts: RwLock::new(HashMap::new()),
            poll_interval_secs: RwLock::new(30),
            alert_cooldown_secs: RwLock::new(300),
            t0_config: RwLock::new(TZeroConfig::default()),
            t0_callback: RwLock::new(None),
            t0_last_trigger_ts: RwLock::new(HashMap::new()),
            aggregator: RwLock::new(None),
        }
    }

    /// P2-6: 设置 T+0 自动重跑配置
    pub async fn set_t0_config(&self, config: TZeroConfig) {
        *self.t0_config.write().await = config;
    }

    /// P2-6: 设置 T+0 触发回调。
    /// 回调签名: 收到 stock_code,异步启动 workflow 重跑,返回 workflow_id 或错误信息。
    /// 不设置 callback 即关闭 T+0 触发(T+0 配置只是"如果 callback 存在就触发")。
    pub async fn set_t0_callback(&self, cb: TZeroCallback) {
        *self.t0_callback.write().await = Some(cb);
    }

    /// P2-6: 查询当前 T+0 配置
    pub async fn t0_config(&self) -> TZeroConfig {
        self.t0_config.read().await.clone()
    }

    /// 设置前端事件推送器以桥接告警到前端
    pub async fn set_event_emitter(&self, emitter: Arc<dyn MonitorEventEmitter>) {
        *self.event_emitter.write().await = Some(emitter);
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<MonitorAlert> {
        self.alert_tx.subscribe()
    }

    /// P3-3: 注入跨股票信号聚合器，启用组合级告警。
    ///
    /// 不调用此方法时，RealtimeMonitor 仅产出单股告警；
    /// 调用后，每次 `check_alerts` 触发告警时会同步喂给聚合器，
    /// 当多只股票同方向触发信号时，聚合器产出 `PortfolioSignal` 并通过
    /// `subscribe_portfolio_signals()` 推送给订阅者。
    pub async fn set_aggregator(
        &self,
        agg: Arc<crate::cross_stock_aggregator::CrossStockSignalAggregator>,
    ) {
        *self.aggregator.write().await = Some(agg);
    }

    /// P3-3: 订阅组合级信号流（需先 `set_aggregator`）
    pub async fn subscribe_portfolio_signals(
        &self,
    ) -> Option<tokio::sync::broadcast::Receiver<crate::cross_stock_aggregator::PortfolioSignal>>
    {
        self.aggregator.read().await.as_ref().map(|agg| agg.subscribe())
    }

    /// 添加监控标的
    pub async fn add_config(&self, config: MonitorConfig) {
        let mut configs = self.configs.write().await;
        configs.insert(config.stock_code.clone(), config);
    }

    /// 移除监控标的
    pub async fn remove_config(&self, stock_code: &str) {
        let mut configs = self.configs.write().await;
        configs.remove(stock_code);
    }

    pub fn serialize_configs(configs: &HashMap<String, MonitorConfig>) -> String {
        serde_json::to_string(&configs.values().collect::<Vec<_>>()).unwrap_or_default()
    }

    pub fn deserialize_configs(json: &str) -> Vec<MonitorConfig> {
        serde_json::from_str::<Vec<MonitorConfig>>(json).unwrap_or_default()
    }

    /// 获取所有监控配置
    pub async fn list_configs(&self) -> Vec<MonitorConfig> {
        let configs = self.configs.read().await;
        configs.values().cloned().collect()
    }

    /// 启动监控循环（轮询间隔从 `poll_interval_secs` 读取，默认 30 秒）
    pub async fn start(&self) {
        {
            let mut running = self.running.write().await;
            if *running {
                return;
            }
            *running = true;
        }

        let interval_secs = *self.poll_interval_secs.read().await;
        let mut ticker = interval(Duration::from_secs(interval_secs));
        loop {
            ticker.tick().await;
            {
                let running = self.running.read().await;
                if !*running {
                    break;
                }
            }

            let configs = {
                let c = self.configs.read().await;
                c.values().cloned().collect::<Vec<_>>()
            };

            for config in configs {
                if !config.enabled {
                    continue;
                }
                // 非交易时段跳过
                if !axagent_astock_data::calendar::is_trading_time() {
                    continue;
                }

                if let Ok(quote) = self.client.get_quote(&config.stock_code).await {
                    self.check_alerts(&config, &quote).await;
                }
            }
        }
    }

    /// 停止监控循环
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }

    /// 以自定义参数启动监控循环。
    /// 设置轮询间隔和告警冷却时间后自动调用 `start()`。
    pub async fn start_with_config(&self, poll_interval_secs: u64, alert_cooldown_secs: i64) {
        *self.poll_interval_secs.write().await = poll_interval_secs;
        *self.alert_cooldown_secs.write().await = alert_cooldown_secs;
        self.start().await;
    }

    /// P1-1: 运行时热更新告警冷却时间（无需重启 monitor）。
    ///
    /// 用户在 settings 修改 `monitor_alert_cooldown_secs` 后立即生效，
    /// `check_alerts` 下一次循环就会读取新值做去重判定。
    pub async fn set_alert_cooldown_secs(&self, secs: i64) {
        *self.alert_cooldown_secs.write().await = secs;
    }

    /// P1-1: 查询当前告警冷却时间（供 Tauri 命令读取展示给前端）
    pub async fn alert_cooldown_secs(&self) -> i64 {
        *self.alert_cooldown_secs.read().await
    }

    /// P1-1: 运行时热更新轮询间隔（无需重启 monitor）。
    pub async fn set_poll_interval_secs(&self, secs: u64) {
        *self.poll_interval_secs.write().await = secs;
    }

    /// P1-1: 查询当前轮询间隔
    pub async fn poll_interval_secs(&self) -> u64 {
        *self.poll_interval_secs.read().await
    }

    async fn check_alerts(&self, config: &MonitorConfig, quote: &StockQuote) {
        let mut alerts = Vec::new();
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 止损检查
        if let Some(stop) = config.stop_loss {
            if quote.price <= stop {
                let suggested =
                    Some(format!("建议: 考虑减仓50%，现价{:.2}低于止损{:.2}", quote.price, stop));
                alerts.push(MonitorAlert {
                    stock_code: config.stock_code.clone(),
                    stock_name: config.stock_name.clone(),
                    alert_type: "stop_loss".into(),
                    alert_message: format!(
                        "跌破止损价 {}: 现价 {:.2} <= 止损 {:.2}",
                        config.stock_name, quote.price, stop
                    ),
                    current_price: quote.price,
                    change_pct: quote.change_pct,
                    timestamp: now.clone(),
                    suggested_action: suggested,
                });
            }
        }

        // 止盈检查
        if let Some(tp) = config.take_profit {
            if quote.price >= tp {
                let suggested =
                    Some(format!("建议: 考虑卖出50%锁利，现价{:.2}≥止盈{:.2}", quote.price, tp));
                alerts.push(MonitorAlert {
                    stock_code: config.stock_code.clone(),
                    stock_name: config.stock_name.clone(),
                    alert_type: "take_profit".into(),
                    alert_message: format!(
                        "突破止盈价 {}: 现价 {:.2} >= 止盈 {:.2}",
                        config.stock_name, quote.price, tp
                    ),
                    current_price: quote.price,
                    change_pct: quote.change_pct,
                    timestamp: now.clone(),
                    suggested_action: suggested,
                });
            }
        }

        // 压力位突破
        if let Some(res) = config.resistance_break {
            if quote.price >= res {
                let suggested = Some(format!("建议: 压力位突破{:.2}，关注持续性和量能配合", res));
                alerts.push(MonitorAlert {
                    stock_code: config.stock_code.clone(),
                    stock_name: config.stock_name.clone(),
                    alert_type: "resistance".into(),
                    alert_message: format!(
                        "突破压力位 {}: 现价 {:.2} >= 压力 {:.2}",
                        config.stock_name, quote.price, res
                    ),
                    current_price: quote.price,
                    change_pct: quote.change_pct,
                    timestamp: now.clone(),
                    suggested_action: suggested,
                });
            }
        }

        // 支撑位跌破
        if let Some(sup) = config.support_break {
            if quote.price <= sup {
                let suggested =
                    Some(format!("建议: 支撑位{:.2}跌破，考虑减仓或设立更严格止损", sup));
                alerts.push(MonitorAlert {
                    stock_code: config.stock_code.clone(),
                    stock_name: config.stock_name.clone(),
                    alert_type: "support".into(),
                    alert_message: format!(
                        "跌破支撑位 {}: 现价 {:.2} <= 支撑 {:.2}",
                        config.stock_name, quote.price, sup
                    ),
                    current_price: quote.price,
                    change_pct: quote.change_pct,
                    timestamp: now.clone(),
                    suggested_action: suggested,
                });
            }
        }

        // 涨跌幅异常
        if let Some(pct) = config.change_pct_alert {
            if quote.change_pct.abs() >= pct {
                let dir = if quote.change_pct > 0.0 { "涨" } else { "跌" };
                let suggested = Some(format!(
                    "建议: 异常{}幅{:.2}%，关注消息面和资金流向",
                    dir, quote.change_pct
                ));
                alerts.push(MonitorAlert {
                    stock_code: config.stock_code.clone(),
                    stock_name: config.stock_name.clone(),
                    alert_type: "change".into(),
                    alert_message: format!(
                        "异常{}幅 {}: {:.2}%",
                        dir, config.stock_name, quote.change_pct
                    ),
                    current_price: quote.price,
                    change_pct: quote.change_pct,
                    timestamp: now.clone(),
                    suggested_action: suggested,
                });
            }
        }

        // 换手率异常
        if let Some(ratio) = config.turnover_rate_alert {
            if quote.turnover_rate >= ratio {
                let suggested =
                    Some(format!("建议: 换手率异常{:.2}%，关注主力进出痕迹", quote.turnover_rate));
                alerts.push(MonitorAlert {
                    stock_code: config.stock_code.clone(),
                    stock_name: config.stock_name.clone(),
                    alert_type: "volume".into(),
                    alert_message: format!(
                        "换手率异常 {}: {:.2}% > {:.2}%",
                        config.stock_name, quote.turnover_rate, ratio
                    ),
                    current_price: quote.price,
                    change_pct: quote.change_pct,
                    timestamp: now.clone(),
                    suggested_action: suggested,
                });
            }
        }

        let now_ts = chrono::Utc::now().timestamp();
        let cooldown_secs = *self.alert_cooldown_secs.read().await;
        let mut last = self.last_alerts.write().await;
        alerts.retain(|a| {
            let key = format!("{}:{}", a.stock_code, a.alert_type);
            if let Some(&last_ts) = last.get(&key) {
                if now_ts - last_ts < cooldown_secs {
                    return false;
                }
            }
            last.insert(key, now_ts);
            true
        });

        // 发送告警 — 内部 broadcast channel + 前端事件桥接
        let emitter = self.event_emitter.read().await.clone();
        for alert in &alerts {
            let _ = self.alert_tx.send(alert.clone());
            if let Some(ref e) = emitter {
                e.emit(
                    "stock-monitor-alert",
                    serde_json::json!({
                        "stockCode": alert.stock_code,
                        "stockName": alert.stock_name,
                        "alertType": alert.alert_type,
                        "alertMessage": alert.alert_message,
                        "currentPrice": alert.current_price,
                        "changePct": alert.change_pct,
                        "suggestedAction": alert.suggested_action,
                        "timestamp": alert.timestamp,
                    }),
                );
            }
        }

        // P2-6: T+0 自动重跑
        // 在告警发送完毕后,根据本次触发的 alert 集合判断是否需要重跑工作流。
        // - 必须 enabled=true
        // - 必须有 alert 触发 (没触发就不重跑,避免每分钟空转)
        // - 必须通过 cooldown 检查 (同一只股票两次触发间隔 ≥ min_interval_minutes)
        if !alerts.is_empty() {
            self.maybe_trigger_t0(&alerts, &config.stock_code, quote).await;
        }

        // P3-3: 将本次告警喂给跨股票信号聚合器
        // 聚合器在多只股票同方向触发时生成 PortfolioSignal（组合级告警）
        if !alerts.is_empty() {
            self.feed_aggregator(&alerts, config, quote).await;
        }
    }

    /// P3-3: 将单股告警转换为 StockSignal 喂给聚合器
    async fn feed_aggregator(
        &self,
        alerts: &[MonitorAlert],
        config: &MonitorConfig,
        quote: &StockQuote,
    ) {
        use crate::cross_stock_aggregator::{SignalType, StockSignal};

        let agg = self.aggregator.read().await.clone();
        let Some(agg) = agg else {
            return;
        };

        let now_ts = chrono::Utc::now().timestamp();
        // 取本次最严重的告警作为该股票的信号（按 alert_type 优先级）
        // 优先级：stop_loss > support_break > take_profit > resistance_break > change > volume
        let signal_type = alerts
            .iter()
            .map(|a| match a.alert_type.as_str() {
                "stop_loss" => (SignalType::StopLossHit, 6),
                "support" => (SignalType::SupportBreak, 5),
                "take_profit" => (SignalType::TakeProfitHit, 4),
                "resistance" => (SignalType::ResistanceBreak, 3),
                "change" => (SignalType::ChangeSpike, 2),
                "volume" => (SignalType::VolumeSpike, 1),
                _ => (SignalType::ChangeSpike, 0),
            })
            .max_by_key(|(_, p)| *p)
            .map(|(t, _)| t)
            .unwrap_or(SignalType::ChangeSpike);

        // 信号强度估算：基于涨跌幅绝对值（+5% → 强度 0.5，+10% → 强度 1.0）
        let strength = (quote.change_pct.abs() / 10.0).clamp(0.1, 1.0);

        let signal = StockSignal {
            stock_code: config.stock_code.clone(),
            stock_name: config.stock_name.clone(),
            signal_type,
            strength,
            source: "realtime_monitor".to_string(),
            timestamp: now_ts,
            current_price: Some(quote.price),
            change_pct: Some(quote.change_pct),
        };

        // 喂给聚合器；若触发组合级信号，通过 broadcast 自动推送给订阅者
        if let Some(portfolio_signal) = agg.feed(signal).await {
            tracing::info!(
                "[portfolio_alert] 组合级信号触发: dir={:?} stocks={} strength={:.2} action={}",
                portfolio_signal.direction,
                portfolio_signal.stocks.len(),
                portfolio_signal.strength,
                portfolio_signal.suggested_action
            );
            // 通过 event_emitter 推送给前端
            let emitter = self.event_emitter.read().await.clone();
            if let Some(ref e) = emitter {
                e.emit(
                    "portfolio-signal",
                    serde_json::to_value(&portfolio_signal).unwrap_or(serde_json::Value::Null),
                );
            }
        }
    }

    /// P2-6: 检查并触发 T+0 重跑
    ///
    /// 设计原则: monitor 不直接调用 workflow 重跑(避免循环依赖),而是 emit
    /// `stock-monitor-t0-rerun-requested` 事件到前端,由前端 listener 调
    /// `run_stock_workflow` 命令触发重跑。这样:
    /// 1) 后端 monitor 保持纯监控职责,不依赖 workflow engine;
    /// 2) 前端可在 T+0 重跑前做去重、节流、UI 提示;
    /// 3) 如果用户没开前端 UI,事件会丢失但不阻塞监控。
    async fn maybe_trigger_t0(
        &self,
        alerts: &[MonitorAlert],
        stock_code: &str,
        quote: &StockQuote,
    ) {
        let t0_cfg = self.t0_config.read().await.clone();
        if !t0_cfg.enabled {
            return;
        }

        // 1) 判断本次告警是否命中 T+0 触发条件
        let mut hit_change = false;
        let mut hit_volume = false;
        for a in alerts {
            if a.alert_type == "change" {
                if let Some(th) = t0_cfg.change_pct_threshold {
                    if quote.change_pct.abs() >= th {
                        hit_change = true;
                    }
                }
            } else if a.alert_type == "volume" {
                if let Some(th) = t0_cfg.turnover_rate_threshold {
                    if quote.turnover_rate >= th {
                        hit_volume = true;
                    }
                }
            }
        }
        if !hit_change && !hit_volume {
            return; // 告警触发但未命中 T+0 阈值
        }

        // 2) Cooldown: 同一只股票两次 T+0 之间间隔 ≥ min_interval_minutes
        //    读+写在同一个 write lock 内完成，避免 TOCTOU 竞态
        let now_ts = chrono::Utc::now().timestamp();
        let cooldown_secs = t0_cfg.min_interval_minutes * 60;
        {
            let mut last_map = self.t0_last_trigger_ts.write().await;
            if let Some(&last_ts) = last_map.get(stock_code) {
                if now_ts - last_ts < cooldown_secs {
                    tracing::debug!(
                        "[t0] skip {}: cooldown 未到 ({}s < {}s)",
                        stock_code,
                        now_ts - last_ts,
                        cooldown_secs
                    );
                    return;
                }
            }
            last_map.insert(stock_code.to_string(), now_ts);
        }
        let reason = if hit_change && hit_volume {
            "change+volume"
        } else if hit_change {
            "change"
        } else {
            "volume"
        };
        tracing::info!(
            "[t0] 触发 T+0 重跑: stock={} reason={} change_pct={:.2} turnover={:.2}",
            stock_code,
            reason,
            quote.change_pct,
            quote.turnover_rate
        );

        // 4) 触发 T+0 重跑
        // P1-1: 优先调用 t0_callback（后端直接重跑，不依赖前端在线），
        //        同时 emit 前端事件做 UI 提示（toast / 状态刷新）。
        // - 若 callback 存在并触发：事件 payload 携带 backendTriggered=true，
        //   前端据此跳过 startAnalysis（避免后端+前端重复重跑），仅做 toast 提示。
        // - 若 callback 不存在（未注入）：backendTriggered=false，前端走原逻辑调 startAnalysis。
        let mut backend_triggered = false;
        let callback = self.t0_callback.read().await.clone();
        if let Some(ref cb) = callback {
            // 后端重跑失败不阻塞监控循环，仅记录日志
            match cb(stock_code.to_string()).await {
                Ok(workflow_id) => {
                    backend_triggered = true;
                    tracing::info!(
                        "[t0] 后端重跑已触发: stock={} workflow_id={}",
                        stock_code,
                        workflow_id
                    );
                },
                Err(e) => {
                    tracing::warn!("[t0] 后端重跑失败: stock={} err={}", stock_code, e);
                },
            }
        }

        // 同时 emit 前端事件 —— 用于 UI toast 提示和状态刷新
        let emitter = self.event_emitter.read().await.clone();
        if let Some(ref e) = emitter {
            e.emit(
                "stock-monitor-t0-rerun-requested",
                serde_json::json!({
                    "stockCode": stock_code,
                    "reason": reason,
                    "currentPrice": quote.price,
                    "changePct": quote.change_pct,
                    "turnoverRate": quote.turnover_rate,
                    "timestamp": now_ts,
                    "backendTriggered": backend_triggered,
                }),
            );
        }
    }
}
