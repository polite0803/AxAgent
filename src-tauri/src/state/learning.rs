//! Learning & trajectory optimization domain state.
//!
//! Owns the learning engines that operate on trajectory data:
//! text-grad optimization, intrinsic motivation, co-evolution, and
//! process reward modeling.

use std::sync::Arc;
use tokio::sync::Mutex;

#[allow(dead_code)]
pub struct LearningState {
    pub text_grad_engine: Arc<Mutex<axagent_trajectory::TextGradEngine>>,
    pub intrinsic_motivation: Arc<Mutex<axagent_trajectory::IntrinsicMotivationEngine>>,
    pub coevolution_env: Arc<Mutex<axagent_trajectory::CoevolutionEnvironment>>,
    pub process_reward_model: Arc<Mutex<axagent_trajectory::ProcessRewardModel>>,
}

#[allow(dead_code)]
impl LearningState {
    pub fn new(
        text_grad_engine: Arc<Mutex<axagent_trajectory::TextGradEngine>>,
        intrinsic_motivation: Arc<Mutex<axagent_trajectory::IntrinsicMotivationEngine>>,
        coevolution_env: Arc<Mutex<axagent_trajectory::CoevolutionEnvironment>>,
        process_reward_model: Arc<Mutex<axagent_trajectory::ProcessRewardModel>>,
    ) -> Self {
        Self { text_grad_engine, intrinsic_motivation, coevolution_env, process_reward_model }
    }
}
