//! 荐股定时任务：配置解析与辅助函数
//!
//! 与 [stock_cron] 不同的是：
//! - task_type = `stock-recommendation`（用于在 [services] 的 cron executor 中路由到荐股 handler）
//! - 配置（periods / min_confidence / top_n）以 JSON 形式写入 `CronJob.prompt`
//! - 不绑定 workflow（不走 work_engine）
//!
//! [stock_cron]: crate::commands::stock_analysis::create_stock_cron
//! [services]: crate::init::services::start_cron_scheduler

use axagent_stock_analysis::recommender::Period;

/// 推荐 cron 配置（写入 `CronJob.prompt`）
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecoCronConfig {
    pub periods: Vec<Period>,
    pub min_confidence: u8,
    pub top_n: usize,
}

impl RecoCronConfig {
    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }
}
