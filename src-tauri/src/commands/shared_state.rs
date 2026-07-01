// SPDX-License-Identifier: AGPL-3.0-only

//! 全局共享状态 — RLOptimizer、ExperiencePipeline、FeedbackOrchestrator 的单例持有者。
//!
//! 使用方式：
//! - rl.rs 通过 SHARED_OPTIMIZER 访问 RLOptimizer
//! - tracer.rs 通过 SHARED_PIPELINE / SHARED_ORCHESTRATOR 摄入反馈
//! - 在 app 启动时调用 init_shared_state() 初始化

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
        let pipeline = ExperiencePipeline::new(SHARED_OPTIMIZER.clone(), 100); // 每 100 条经验自动 train
        Arc::new(RwLock::new(pipeline))
    };

    /// 全局 FeedbackOrchestrator，监听反馈事件并决策优化动作
    pub static ref SHARED_ORCHESTRATOR: Arc<FeedbackOrchestrator> = {
        Arc::new(FeedbackOrchestrator::new())
    };
}

/// App 启动时调用（预留，当前 lazy_static 已自动初始化）。
#[allow(dead_code)]
pub fn init_shared_state() {
    lazy_static::initialize(&SHARED_OPTIMIZER);
    lazy_static::initialize(&SHARED_PIPELINE);
    lazy_static::initialize(&SHARED_ORCHESTRATOR);
    tracing::info!(
        "[shared_state] RLOptimizer + ExperiencePipeline + FeedbackOrchestrator initialized"
    );
}
