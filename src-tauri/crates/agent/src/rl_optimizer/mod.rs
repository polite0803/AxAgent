// SPDX-License-Identifier: AGPL-3.0-only

pub mod policy;
pub mod rl_training_loop;
pub mod trainer;

pub use rl_training_loop::ThresholdScheduler;

// RLConfig 的权威定义在 axagent_harness::rl，本 crate 通过 pub use 引用，
// 不再重复定义（AGENTS.md 第 12 条）。
pub use axagent_harness::rl::RLConfig;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLOptimizer {
    pub id: String,
    pub name: String,
    pub policies: HashMap<String, Policy>,
    pub experience_pool: ExperiencePool,
    pub config: RLConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperiencePool {
    pub experiences: Vec<Experience>,
    pub max_size: usize,
}

impl ExperiencePool {
    pub fn new(max_size: usize) -> Self {
        Self { experiences: Vec::new(), max_size }
    }

    pub fn add(&mut self, experience: Experience) {
        if self.experiences.len() >= self.max_size {
            self.experiences.remove(0);
        }
        self.experiences.push(experience);
    }

    pub fn sample(&self, batch_size: usize) -> Vec<&Experience> {
        let len = self.experiences.len();
        if len == 0 {
            return vec![];
        }
        let batch_size = batch_size.min(len);
        let mut indices: Vec<usize> = (0..len).collect();
        for i in 0..batch_size {
            let j = i + (fastrand::usize(..(len - i)));
            indices.swap(i, j);
        }
        indices.into_iter().take(batch_size).map(|i| &self.experiences[i]).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub state: TaskState,
    pub action: ToolSelection,
    pub reward: f32,
    pub next_state: TaskState,
    pub done: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub task_id: String,
    pub task_type: String,
    pub context: HashMap<String, serde_json::Value>,
    pub available_tools: Vec<String>,
    pub completed_tools: Vec<String>,
    pub error_count: u32,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelection {
    pub tool_id: String,
    pub tool_name: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub policy_type: PolicyType,
    pub model_id: String,
    pub reward_signals: Vec<PolicyRewardWeight>,
    pub training_stats: TrainingStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyType {
    ToolSelection,
    TaskDecomposition,
    ErrorRecovery,
}

/// 策略奖励权重配置（注意：与 harness::RewardSignal 语义不同。
/// harness::RewardSignal 是轨迹评估信号，本类型是策略训练中的权重配置）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRewardWeight {
    pub name: String,
    pub weight: f32,
    pub signal_type: PolicyRewardSignalType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyRewardSignalType {
    TaskCompletion,
    TimeEfficiency,
    ErrorRate,
    ToolDiversity,
    UserFeedback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStats {
    pub total_experiences: u64,
    pub episodes_completed: u64,
    pub avg_reward: f32,
    pub last_update: chrono::DateTime<chrono::Utc>,
}

impl RLOptimizer {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            policies: HashMap::new(),
            experience_pool: ExperiencePool::new(10000),
            config: RLConfig::default(),
        }
    }

    pub fn add_policy(&mut self, policy: Policy) {
        self.policies.insert(policy.id.clone(), policy);
    }

    pub fn record_experience(&mut self, experience: Experience) {
        self.experience_pool.add(experience);
    }

    pub fn select_tool(&self, state: &TaskState) -> Result<ToolSelection, RLError> {
        // epsilon-greedy: 以 exploration_rate 概率随机探索，否则选择最佳工具
        let epsilon = self.config.exploration_rate;
        let explore = fastrand::f64() < epsilon;

        // 从策略中获取工具权重
        let policy = self.policies.get("tool_selection");
        let mut tool_scores: Vec<(String, f32)> = state
            .available_tools
            .iter()
            .map(|tool| {
                let weight = policy
                    .and_then(|p| p.reward_signals.iter().find(|s| s.name == *tool))
                    .map(|s| s.weight)
                    .unwrap_or(0.5);
                (tool.clone(), weight)
            })
            .collect();

        if explore || tool_scores.is_empty() {
            // 探索：随机选择
            if !state.available_tools.is_empty() {
                let idx = fastrand::usize(..state.available_tools.len());
                let tool = &state.available_tools[idx];
                return Ok(ToolSelection {
                    tool_id: tool.clone(),
                    tool_name: tool.clone(),
                    parameters: HashMap::new(),
                    reasoning: format!("RL exploration (epsilon={:.3})", epsilon),
                });
            }
            return Ok(ToolSelection {
                tool_id: "default_tool".to_string(),
                tool_name: "Default Tool".to_string(),
                parameters: HashMap::new(),
                reasoning: "RL fallback selection".to_string(),
            });
        }

        // 利用：选择权重最高的工具
        tool_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let best = &tool_scores[0];

        Ok(ToolSelection {
            tool_id: best.0.clone(),
            tool_name: best.0.clone(),
            parameters: HashMap::new(),
            reasoning: format!("RL policy selection (weight={:.3})", best.1),
        })
    }

    pub fn get_policy_stats(&self, policy_id: &str) -> Option<TrainingStats> {
        self.policies.get(policy_id).map(|p| p.training_stats.clone())
    }

    /// 返回每个可用工具的 RL 学习权重（0.0 ~ 1.0）。
    /// 排序从高到低，供 `get_chat_tools()` 重排工具列表时使用。
    /// 未在策略中注册的工具默认权重 0.5。
    pub fn tool_ranking(&self, tool_names: &[String]) -> Vec<(String, f32)> {
        let policy = self.policies.get("tool_selection");
        let mut ranked: Vec<(String, f32)> = tool_names
            .iter()
            .map(|name| {
                let weight = policy
                    .and_then(|p| p.reward_signals.iter().find(|s| s.name == *name))
                    .map(|s| s.weight)
                    .unwrap_or(0.5);
                (name.clone(), weight)
            })
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    /// 用梯度更新策略权重，学习率由 `self.config.learning_rate` 控制。
    pub fn apply_gradients(&mut self, gradients: &HashMap<String, f64>) {
        let policy = self.policies.entry("tool_selection".to_string()).or_insert_with(|| Policy {
            id: "tool_selection".to_string(),
            name: "Tool Selection Policy".to_string(),
            policy_type: PolicyType::ToolSelection,
            model_id: "default".to_string(),
            reward_signals: Vec::new(),
            training_stats: TrainingStats {
                total_experiences: 0,
                episodes_completed: 0,
                avg_reward: 0.0,
                last_update: chrono::Utc::now(),
            },
        });

        for (tool_name, gradient) in gradients {
            let lr = self.config.learning_rate as f32;
            let found = policy.reward_signals.iter_mut().find(|s| s.name == *tool_name);
            if let Some(signal) = found {
                signal.weight = (signal.weight + lr * *gradient as f32).clamp(0.0, 1.0);
            } else {
                let initial = (0.5 + lr * *gradient as f32 * 0.1).clamp(0.0, 1.0);
                policy.reward_signals.push(PolicyRewardWeight {
                    name: tool_name.clone(),
                    weight: initial,
                    signal_type: PolicyRewardSignalType::ToolDiversity,
                });
            }
        }

        policy.training_stats.total_experiences += gradients.len() as u64;
        policy.training_stats.episodes_completed += 1;
        policy.training_stats.last_update = chrono::Utc::now();
    }

    /// 将 RLOptimizer 状态保存到 JSON 文件
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化 RLOptimizer 失败: {}", e))?;
        std::fs::write(path, &json).map_err(|e| format!("写入 RLOptimizer 文件失败: {}", e))
    }

    /// 从 JSON 文件加载 RLOptimizer 状态
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("读取 RLOptimizer 文件失败: {}", e))?;
        let opt: Self =
            serde_json::from_str(&json).map_err(|e| format!("反序列化 RLOptimizer 失败: {}", e))?;
        Ok(opt)
    }
}

// ── harness ToolRanker trait 实现 ──────────────────────
//
// 用 RL 策略学习到的工具权重对工具列表重排。
// 高权重工具排前面，间接影响 LLM 的工具选择偏好。
impl axagent_harness::tool::ToolRanker for RLOptimizer {
    fn rank_tools(
        &self,
        mut tools: Vec<axagent_harness::types::ChatTool>,
    ) -> Vec<axagent_harness::types::ChatTool> {
        let tool_names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        let ranked = self.tool_ranking(&tool_names);
        // 按权重从高到低稳定排序
        let weight_map: std::collections::HashMap<&str, f32> =
            ranked.iter().map(|(name, w)| (name.as_str(), *w)).collect();
        tools.sort_by(|a, b| {
            let wa = weight_map.get(a.function.name.as_str()).copied().unwrap_or(0.5);
            let wb = weight_map.get(b.function.name.as_str()).copied().unwrap_or(0.5);
            wb.partial_cmp(&wa).unwrap_or(std::cmp::Ordering::Equal)
        });
        tools
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RLError {
    #[error("Policy not found: {0}")]
    PolicyNotFound(String),
    #[error("Training error: {0}")]
    TrainingError(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
}
