// SPDX-License-Identifier: AGPL-3.0-only

//! 全局共享状态 — RLOptimizer、ExperiencePipeline、FeedbackOrchestrator 的单例持有者。
//!
//! 使用方式：
//! - rl.rs 通过 SHARED_OPTIMIZER 访问 RLOptimizer
//! - tracer.rs 通过 SHARED_PIPELINE / SHARED_ORCHESTRATOR 摄入反馈
//!
//! SHARED_OPTIMIZER 支持文件级持久化：
//! - 保存路径：`{app_data_dir}/rl_optimizer.json`
//! - `init_shared_state()` 在初始化时自动从文件加载（存在时）
//! - 后台服务每分钟自动保存一次

use axagent_agent::rl_optimizer::RLOptimizer;
use axagent_agent::{ExperiencePipeline, FeedbackOrchestrator};
use std::sync::Arc;
use tokio::sync::RwLock;

lazy_static::lazy_static! {
    /// 全局唯一的 RLOptimizer 实例
    pub static ref SHARED_OPTIMIZER: Arc<RwLock<RLOptimizer>> = {
        Arc::new(RwLock::new(RLOptimizer::new(
            "shared_rl_optimizer".to_string(),
            "AxAgent Global RL Optimizer".to_string(),
        )))
    };

    /// 全局 ExperiencePipeline，桥接 Reflection/Feedback → ExperiencePool
    pub static ref SHARED_PIPELINE: Arc<RwLock<ExperiencePipeline>> = {
        let pipeline = ExperiencePipeline::new(SHARED_OPTIMIZER.clone(), 100);
        Arc::new(RwLock::new(pipeline))
    };

    /// 全局 FeedbackOrchestrator，监听反馈事件并决策优化动作
    pub static ref SHARED_ORCHESTRATOR: Arc<FeedbackOrchestrator> = {
        Arc::new(FeedbackOrchestrator::new())
    };

    /// 全局 RL ToolRanker，读取 SHARED_OPTIMIZER 策略权重重排工具列表。
    /// 每次 get_chat_tools() 调用时实时读取最新权重。
    pub static ref SHARED_TOOL_RANKER: Arc<dyn axagent_harness::ToolRanker + Send + Sync> = {
        Arc::new(RlToolRanker(SHARED_OPTIMIZER.clone()))
    };
}

/// 包装 `Arc<RwLock<RLOptimizer>>` 实现 `ToolRanker`，
/// 每次 `rank_tools()` 调用实时读取最新策略权重。
struct RlToolRanker(pub Arc<RwLock<RLOptimizer>>);

impl axagent_harness::ToolRanker for RlToolRanker {
    fn rank_tools(
        &self,
        tools: Vec<axagent_harness::types::ChatTool>,
    ) -> Vec<axagent_harness::types::ChatTool> {
        let optimizer = self.0.blocking_read();
        optimizer.rank_tools(tools)
    }
}

/// RLOptimizer 持久化文件路径（相对于 app_data_dir）
const RL_OPTIMIZER_FILE: &str = "rl_optimizer.json";

/// 初始化共享状态：尝试从文件加载 RLOptimizer，失败时使用空实例。
pub fn init_shared_state(app_data_dir: &std::path::Path) {
    let path = app_data_dir.join(RL_OPTIMIZER_FILE);
    if path.exists() {
        match RLOptimizer::load_from_file(&path) {
            Ok(loaded) => {
                *SHARED_OPTIMIZER.blocking_write() = loaded;
                tracing::info!("RLOptimizer loaded from {}", path.display());
            },
            Err(e) => {
                tracing::warn!(
                    "RLOptimizer load from {} failed: {}, using default",
                    path.display(),
                    e
                );
            },
        }
    }
}

/// 保存 RLOptimizer 状态到文件（由后台定时任务调用）。
pub fn save_rl_optimizer(app_data_dir: &std::path::Path) {
    let path = app_data_dir.join(RL_OPTIMIZER_FILE);
    match SHARED_OPTIMIZER.try_read() {
        Ok(opt) => {
            if let Err(e) = opt.save_to_file(&path) {
                tracing::warn!("RLOptimizer save failed: {}", e);
            }
        },
        Err(_) => {
            tracing::warn!("RLOptimizer save skipped: lock busy");
        },
    }
}
