//! Learning & trajectory optimization domain state.
//!
//! Owns the learning engines that operate on trajectory data:
//! text-grad optimization, intrinsic motivation, co-evolution, and
//! process reward modeling.
//!
//! Also owns the industry-specific learning engine and adapter registry
//! for OPC (One-Person Company) vertical industry scenarios.

use std::sync::Arc;
use tokio::sync::Mutex;

use axagent_orchestrator::{IndustryAdapterRegistry, IndustryLearningEngine};

pub struct LearningEngineState {
    pub text_grad_engine: Arc<Mutex<axagent_trajectory::TextGradEngine>>,
    pub intrinsic_motivation: Arc<Mutex<axagent_trajectory::IntrinsicMotivationEngine>>,
    pub coevolution_env: Arc<Mutex<axagent_trajectory::CoevolutionEnvironment>>,
    pub process_reward_model: Arc<Mutex<axagent_trajectory::ProcessRewardModel>>,
    /// OPC 行业学习引擎 — 实现反思、进化、自我改进
    pub industry_learning_engine: Arc<IndustryLearningEngine>,
    /// OPC 行业适配器注册表 — 管理 9 个垂直行业的适配器
    pub industry_adapter_registry: Arc<Mutex<IndustryAdapterRegistry>>,
}

impl LearningEngineState {
    pub fn new(
        text_grad_engine: Arc<Mutex<axagent_trajectory::TextGradEngine>>,
        intrinsic_motivation: Arc<Mutex<axagent_trajectory::IntrinsicMotivationEngine>>,
        coevolution_env: Arc<Mutex<axagent_trajectory::CoevolutionEnvironment>>,
        process_reward_model: Arc<Mutex<axagent_trajectory::ProcessRewardModel>>,
        industry_learning_engine: Arc<IndustryLearningEngine>,
        industry_adapter_registry: Arc<Mutex<IndustryAdapterRegistry>>,
    ) -> Self {
        Self {
            text_grad_engine,
            intrinsic_motivation,
            coevolution_env,
            process_reward_model,
            industry_learning_engine,
            industry_adapter_registry,
        }
    }
}
