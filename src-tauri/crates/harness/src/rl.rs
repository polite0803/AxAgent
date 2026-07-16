// SPDX-License-Identifier: AGPL-3.0-only

//! RL 契约 — RLEngine + RLTrainer traits + DTOs
//!
//! 本模块是 RL 相关类型的**唯一权威定义**（AGENTS.md 第 12 条）。
//! trajectory / agent 等 crate 通过 `pub use axagent_harness::rl::*` 引用，
//! 不得重复定义 `RLConfig` / `RewardWeights` 等同义类型。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 默认值函数 ──────────────────────────────────────────────────────

fn default_learning_rate() -> f64 {
    0.001
}
fn default_batch_size() -> usize {
    32
}
fn default_gamma() -> f64 {
    0.99
}
fn default_exploration_rate() -> f64 {
    0.1
}
fn default_epsilon_decay() -> f64 {
    0.995
}
fn default_epsilon_min() -> f64 {
    0.01
}
fn default_lambda() -> f64 {
    0.95
}
fn default_use_td_lambda() -> bool {
    true
}
fn default_reward_scale() -> f64 {
    1.0
}
fn default_entropy_coefficient() -> f64 {
    0.01
}
fn default_value_coefficient() -> f64 {
    0.5
}

fn default_weight_task_completion() -> f64 {
    0.4
}
fn default_weight_tool_efficiency() -> f64 {
    0.2
}
fn default_weight_reasoning_quality() -> f64 {
    0.15
}
fn default_weight_error_recovery() -> f64 {
    0.15
}
fn default_weight_user_feedback() -> f64 {
    0.05
}
fn default_weight_pattern_match() -> f64 {
    0.05
}

// ── 统一 RLConfig（超集） ───────────────────────────────────────────
//
// 合并了原 harness / trajectory / agent 三套 RLConfig 的所有字段：
// - harness: learning_rate, discount_factor, exploration_rate, batch_size
// - trajectory: gamma, lambda, reward_scale, entropy_coefficient,
//               value_coefficient, use_td_lambda
// - agent: learning_rate, batch_size, gamma, epsilon, epsilon_decay,
//          epsilon_min
//
// 统一命名规则：
// - `gamma` 取代 `discount_factor`（通过 serde alias 向后兼容）
// - `exploration_rate` 取代 `epsilon`（通过 serde alias 向后兼容）
// - 所有数值统一用 f64（agent crate 使用时 `as f32` 转换）
// - 所有字段都有 `#[serde(default)]`，反序列化时缺失字段自动填默认值

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLConfig {
    // ===== 基础训练参数 =====
    #[serde(default = "default_learning_rate")]
    pub learning_rate: f64,

    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    // ===== 折扣因子（统一命名 gamma，alias 兼容旧 discount_factor）=====
    #[serde(default = "default_gamma", alias = "discount_factor")]
    pub gamma: f64,

    // ===== 探索参数 =====
    /// 探索率（alias 兼容旧 epsilon 字段）
    #[serde(default = "default_exploration_rate", alias = "epsilon")]
    pub exploration_rate: f64,

    #[serde(default = "default_epsilon_decay")]
    pub epsilon_decay: f64,

    #[serde(default = "default_epsilon_min")]
    pub epsilon_min: f64,

    // ===== TD(λ) 相关（trajectory crate 专用）=====
    #[serde(default = "default_lambda")]
    pub lambda: f64,

    #[serde(default = "default_use_td_lambda")]
    pub use_td_lambda: bool,

    #[serde(default = "default_reward_scale")]
    pub reward_scale: f64,

    #[serde(default = "default_entropy_coefficient")]
    pub entropy_coefficient: f64,

    #[serde(default = "default_value_coefficient")]
    pub value_coefficient: f64,
}

impl Default for RLConfig {
    fn default() -> Self {
        Self {
            learning_rate: default_learning_rate(),
            batch_size: default_batch_size(),
            gamma: default_gamma(),
            exploration_rate: default_exploration_rate(),
            epsilon_decay: default_epsilon_decay(),
            epsilon_min: default_epsilon_min(),
            lambda: default_lambda(),
            use_td_lambda: default_use_td_lambda(),
            reward_scale: default_reward_scale(),
            entropy_coefficient: default_entropy_coefficient(),
            value_coefficient: default_value_coefficient(),
        }
    }
}

// ── 统一 RewardWeights（超集） ──────────────────────────────────────
//
// 合并了原 harness（4 字段）和 trajectory（6 字段）两套 RewardWeights。
// 采用 trajectory 的 6 字段版本（更完整），harness 原有的
// efficiency/code_quality/user_satisfaction 由语义更精确的
// tool_efficiency/error_recovery/user_feedback 替代。

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardWeights {
    #[serde(default = "default_weight_task_completion")]
    pub task_completion: f64,

    #[serde(default = "default_weight_tool_efficiency")]
    pub tool_efficiency: f64,

    #[serde(default = "default_weight_reasoning_quality")]
    pub reasoning_quality: f64,

    #[serde(default = "default_weight_error_recovery")]
    pub error_recovery: f64,

    #[serde(default = "default_weight_user_feedback")]
    pub user_feedback: f64,

    #[serde(default = "default_weight_pattern_match")]
    pub pattern_match: f64,
}

impl Default for RewardWeights {
    fn default() -> Self {
        Self {
            task_completion: default_weight_task_completion(),
            tool_efficiency: default_weight_tool_efficiency(),
            reasoning_quality: default_weight_reasoning_quality(),
            error_recovery: default_weight_error_recovery(),
            user_feedback: default_weight_user_feedback(),
            pattern_match: default_weight_pattern_match(),
        }
    }
}

// ── Training DTOs（harness 契约层通用） ────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingEpisode {
    pub episode_id: String,
    pub steps: Vec<TrainingStep>,
    pub total_reward: f64,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStep {
    pub observation: String,
    pub action: String,
    pub reward: f64,
    pub next_observation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingReport {
    pub episodes_trained: usize,
    pub avg_reward: f64,
    pub max_reward: f64,
    pub total_steps: usize,
    pub duration_secs: f64,
}

// ── RL trait 契约 ──────────────────────────────────────────────────

/// 通用 RL 引擎契约（基于 `TrainingEpisode` 抽象）
///
/// 注意：此 trait 是早期设计的通用抽象，目前仅 `NoopRLEngine`（test_support）实现。
/// 实际生产代码使用的是 `TrajectoryRewardEngine` trait（基于 `Trajectory` 数据模型）。
/// 两者并存是为了保留通用抽象的同时不破坏现有基于 Trajectory 的实现。
#[async_trait]
pub trait RLEngine: Send + Sync {
    async fn compute_rewards(&self, episodes: &[TrainingEpisode]) -> Result<Vec<f64>, String>;
    async fn compute_advantages(&self, rewards: &[f64]) -> Vec<f64>;
    async fn reset(&self);
}

/// 基于 `Trajectory` 数据模型的 RL 奖励引擎契约
///
/// 此 trait 是 trajectory crate `RLEngine` struct 的正式契约。
/// 与通用 `RLEngine` trait 的区别：
/// - 输入：`&mut Trajectory`（直接操作轨迹数据）vs `&[TrainingEpisode]`（抽象 episode）
/// - 输出：`Vec<RewardSignal>`（多维奖励信号）vs `Vec<f64>`（标量奖励）
///
/// consumer crate（agent/gateway）应优先使用此 trait 而非通用 `RLEngine`，
/// 以获得更精确的奖励信号。
#[async_trait]
pub trait TrajectoryRewardEngine: Send + Sync {
    /// 计算轨迹的多维奖励信号（支持 LLM judge，无 judge 时退化为启发式）
    async fn compute_rewards(
        &self,
        trajectory: &mut crate::trajectory_types::Trajectory,
    ) -> Vec<crate::trajectory_types::RewardSignal>;

    /// 估计轨迹的状态价值函数
    fn estimate_value_function(&self, trajectory: &crate::trajectory_types::Trajectory)
    -> Vec<f64>;

    /// 计算优势函数（Advantage = Q - V）
    fn compute_advantages(
        &self,
        rewards: &[crate::trajectory_types::RewardSignal],
        values: &[f64],
    ) -> Vec<f64>;

    /// 奖励塑形（reward shaping）
    fn shape_rewards(&self, rewards: &mut [crate::trajectory_types::RewardSignal]);
}

#[async_trait]
pub trait RLTrainer: Send + Sync {
    async fn train_episode(&self, episode: TrainingEpisode) -> Result<TrainingReport, String>;
    async fn get_progress(&self) -> Result<TrainingReport, String>;
}
