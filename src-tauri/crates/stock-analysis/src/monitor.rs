use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use axagent_astock_data::{AStockClient, StockQuote};
use tauri::Emitter;

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
    client: Arc<AStockClient>,
    configs: RwLock<HashMap<String, MonitorConfig>>,
    alert_tx: tokio::sync::broadcast::Sender<MonitorAlert>,
    running: RwLock<bool>,
    app_handle: RwLock<Option<tauri::AppHandle>>,
    last_alerts: RwLock<HashMap<String, i64>>,
    poll_interval_secs: RwLock<u64>,
    alert_cooldown_secs: RwLock<i64>,
    // P2-6: T+0 自动重跑相关字段
    t0_config: RwLock<TZeroConfig>,
    t0_callback: RwLock<Option<TZeroCallback>>,
    /// 记录每只股票最近一次 T+0 触发的时间戳(秒),用于 cooldown 控制
    t0_last_trigger_ts: RwLock<HashMap<String, i64>>,
}

impl RealtimeMonitor {
    pub fn new(client: Arc<AStockClient>) -> Self {
        let (alert_tx, _) = tokio::sync::broadcast::channel(128);
        Self {
            client,
            configs: RwLock::new(HashMap::new()),
            alert_tx,
            running: RwLock::new(false),
            app_handle: RwLock::new(None),
            last_alerts: RwLock::new(HashMap::new()),
            poll_interval_secs: RwLock::new(30),
            alert_cooldown_secs: RwLock::new(300),
            t0_config: RwLock::new(TZeroConfig::default()),
            t0_callback: RwLock::new(None),
            t0_last_trigger_ts: RwLock::new(HashMap::new()),
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

    /// 设置 Tauri AppHandle 以桥接告警到前端
    pub async fn set_app_handle(&self, handle: tauri::AppHandle) {
        *self.app_handle.write().await = Some(handle);
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<MonitorAlert> {
        self.alert_tx.subscribe()
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

        // 发送告警 — 内部 broadcast channel + Tauri 前端事件桥接
        let app_handle = self.app_handle.read().await.clone();
        for alert in &alerts {
            let _ = self.alert_tx.send(alert.clone());
            if let Some(ref app) = app_handle {
                let _ = app.emit(
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
            self.maybe_trigger_t0(&alerts, &config.stock_code, quote)
                .await;
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
        let now_ts = chrono::Utc::now().timestamp();
        let cooldown_secs = t0_cfg.min_interval_minutes * 60;
        {
            let last_map = self.t0_last_trigger_ts.read().await;
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
        }

        // 3) 记录触发时间, 然后 emit 事件
        {
            let mut last_map = self.t0_last_trigger_ts.write().await;
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

        // 4) 触发 T+0 事件 (前端可订阅用于 toast 提示 + 调 run_stock_workflow)
        let app_handle = self.app_handle.read().await.clone();
        if let Some(ref app) = app_handle {
            let _ = app.emit(
                "stock-monitor-t0-rerun-requested",
                serde_json::json!({
                    "stockCode": stock_code,
                    "reason": reason,
                    "currentPrice": quote.price,
                    "changePct": quote.change_pct,
                    "turnoverRate": quote.turnover_rate,
                    "timestamp": now_ts,
                }),
            );
        }
    }
}
