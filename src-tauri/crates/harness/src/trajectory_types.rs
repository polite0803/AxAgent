// SPDX-License-Identifier: AGPL-3.0-only

//! Trajectory + LLM bridge types — shared DTOs and traits.
//! Zero implementation logic; all default impls stay in axagent-trajectory.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

pub use crate::types::MessageRole;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrajectoryToolResult {
    pub tool_use_id: String,
    pub tool_name: String,
    pub output: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrajectoryStep {
    pub timestamp_ms: u64,
    pub role: MessageRole,
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_results: Option<Vec<TrajectoryToolResult>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrajectoryOutcome {
    Success,
    Failure,
    Partial,
    Abandoned,
}

impl TrajectoryOutcome {
    pub fn try_from_str(s: &str) -> Self {
        match s {
            "success" => TrajectoryOutcome::Success,
            "failure" => TrajectoryOutcome::Failure,
            "partial" => TrajectoryOutcome::Partial,
            "abandoned" => TrajectoryOutcome::Abandoned,
            _ => TrajectoryOutcome::Failure,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TrajectoryOutcome::Success => "success",
            TrajectoryOutcome::Failure => "failure",
            TrajectoryOutcome::Partial => "partial",
            TrajectoryOutcome::Abandoned => "abandoned",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrajectoryQuality {
    pub overall: f64,
    pub task_completion: f64,
    pub tool_efficiency: f64,
    pub reasoning_quality: f64,
    pub user_satisfaction: f64,
}

impl Default for TrajectoryQuality {
    fn default() -> Self {
        Self {
            overall: 0.5,
            task_completion: 0.5,
            tool_efficiency: 0.5,
            reasoning_quality: 0.5,
            user_satisfaction: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trajectory {
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    /// 结构化 agent 标识：记录该轨迹由哪个 Agent（AgentProfile 名称）执行。
    /// 进化系统据此精准聚合每个 Agent 的证据，无需回退文本匹配。
    pub agent_name: Option<String>,
    pub topic: String,
    pub summary: String,
    pub outcome: TrajectoryOutcome,
    pub duration_ms: u64,
    pub quality: TrajectoryQuality,
    pub value_score: f64,
    pub patterns: Vec<String>,
    pub steps: Vec<TrajectoryStep>,
    pub rewards: Vec<RewardSignal>,
    pub created_at: DateTime<Utc>,
    pub replay_count: u32,
    pub last_replay_at: Option<DateTime<Utc>>,
}

impl Trajectory {
    pub fn new(
        session_id: String,
        user_id: String,
        topic: String,
        summary: String,
        outcome: TrajectoryOutcome,
        duration_ms: u64,
        steps: Vec<TrajectoryStep>,
    ) -> Self {
        let id = Uuid::new_v4().to_string();

        Self {
            id,
            session_id,
            user_id,
            agent_name: None,
            topic,
            summary,
            outcome,
            duration_ms,
            quality: TrajectoryQuality::default(),
            value_score: 0.5,
            patterns: Vec::new(),
            rewards: Vec::new(),
            steps,
            created_at: Utc::now(),
            replay_count: 0,
            last_replay_at: None,
        }
    }

    /// 链式设置结构化 agent 标识（记录点就近填充，避免散落赋值）。
    pub fn with_agent_name(mut self, agent_name: impl Into<String>) -> Self {
        self.agent_name = Some(agent_name.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewardType {
    TaskCompletion,
    ToolEfficiency,
    ReasoningQuality,
    ErrorRecovery,
    UserFeedback,
    PatternMatch,
    LlmReasoningQuality,
    LlmToolEfficiency,
}

impl RewardType {
    pub fn try_from_str(s: &str) -> Self {
        match s {
            "task_completion" => RewardType::TaskCompletion,
            "tool_efficiency" => RewardType::ToolEfficiency,
            "reasoning_quality" => RewardType::ReasoningQuality,
            "error_recovery" => RewardType::ErrorRecovery,
            "user_feedback" => RewardType::UserFeedback,
            "pattern_match" => RewardType::PatternMatch,
            "llm_reasoning_quality" => RewardType::LlmReasoningQuality,
            "llm_tool_efficiency" => RewardType::LlmToolEfficiency,
            _ => RewardType::TaskCompletion,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RewardType::TaskCompletion => "task_completion",
            RewardType::ToolEfficiency => "tool_efficiency",
            RewardType::ReasoningQuality => "reasoning_quality",
            RewardType::ErrorRecovery => "error_recovery",
            RewardType::UserFeedback => "user_feedback",
            RewardType::PatternMatch => "pattern_match",
            RewardType::LlmReasoningQuality => "llm_reasoning_quality",
            RewardType::LlmToolEfficiency => "llm_tool_efficiency",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewardSignal {
    pub reward_type: RewardType,
    pub value: f64,
    pub step_index: usize,
    pub timestamp_ms: u64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrajectoryPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub pattern_type: String,
    pub trajectory_ids: Vec<String>,
    pub frequency: u32,
    pub success_rate: f64,
    pub average_quality: f64,
    pub average_value_score: f64,
    pub reward_profile: Vec<(RewardType, f64)>,
    pub created_at: DateTime<Utc>,
}

impl TrajectoryPattern {
    pub fn new(name: String, description: String, pattern_type: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            pattern_type,
            trajectory_ids: Vec::new(),
            frequency: 0,
            success_rate: 0.0,
            average_quality: 0.0,
            average_value_score: 0.0,
            reward_profile: Vec::new(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayContext {
    pub trajectory_id: String,
    pub current_step: usize,
    pub original_trajectory: Trajectory,
    pub deviations: Vec<TrajectoryStep>,
    pub evaluation: f64,
    pub next_suggested_action: Option<String>,
    pub accumulated_reward: f64,
}

impl ReplayContext {
    pub fn new(trajectory: Trajectory) -> Self {
        Self {
            trajectory_id: trajectory.id.clone(),
            current_step: 0,
            original_trajectory: trajectory,
            deviations: Vec::new(),
            evaluation: 0.5,
            next_suggested_action: None,
            accumulated_reward: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryQuery {
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub topic: Option<String>,
    pub outcome: Option<TrajectoryOutcome>,
    pub min_quality: Option<f64>,
    pub min_value_score: Option<f64>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub limit: Option<usize>,
}

impl Default for TrajectoryQuery {
    fn default() -> Self {
        Self {
            session_id: None,
            user_id: None,
            topic: None,
            outcome: None,
            min_quality: None,
            min_value_score: None,
            time_range: None,
            limit: Some(100),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrajectoryExportOptions {
    pub format: ExportFormat,
    pub min_quality: Option<f64>,
    pub min_value_score: Option<f64>,
    pub outcome_filter: Option<TrajectoryOutcome>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Jsonl,
    RlTraining,
    Compressed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub max_episodes: u32,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub reward_threshold: f64,
    pub export_format: ExportFormat,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            max_episodes: 100,
            batch_size: 32,
            learning_rate: 0.001,
            reward_threshold: 0.6,
            export_format: ExportFormat::Jsonl,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RLTrainingEntry {
    pub prompt: String,
    pub completion: String,
    pub trajectory_id: String,
    pub topic: String,
    pub quality: f64,
    pub value_score: f64,
    pub rewards: Vec<RewardSignal>,
}

pub struct TrajectoryBuilder {
    pub session_id: String,
    pub user_id: String,
    pub steps: Vec<TrajectoryStep>,
}

impl TrajectoryBuilder {
    pub fn new(session_id: String, user_id: String) -> Self {
        Self { session_id, user_id, steps: Vec::new() }
    }

    pub fn add_step(mut self, step: TrajectoryStep) -> Self {
        self.steps.push(step);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressedTrajectory {
    pub id: String,
    pub topic: String,
    pub outcome: String,
    #[serde(rename = "qualityScore")]
    pub quality_score: f64,
    #[serde(rename = "valueScore")]
    pub value_score: f64,
    #[serde(rename = "stepSummaries")]
    pub step_summaries: Vec<String>,
    #[serde(rename = "toolSequence")]
    pub tool_sequence: Vec<String>,
    #[serde(rename = "finalReward")]
    pub final_reward: f64,
}

// ── LLM bridge types and traits (from other trajectory modules) ──

/// 进化产物的类型标注 —— 决定产物由哪套执行引擎承载。
///
/// 分层执行决策（阶段四）：
/// - `RhaiScript`：计算型产物（纯计算 / 数据处理 / 简单内部工具编排），
///   由 Rhai 脚本引擎直接执行，复用 `RhaiEngineAdapter`。
/// - `WorkflowDag`：编排型产物（涉及内部能力调用序列、分支、聚合），
///   映射为 `WorkflowGenome` 由 rt-workflow 引擎执行。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EvolutionArtifactKind {
    #[default]
    RhaiScript,
    WorkflowDag,
}

impl EvolutionArtifactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EvolutionArtifactKind::RhaiScript => "rhai_script",
            EvolutionArtifactKind::WorkflowDag => "workflow_dag",
        }
    }

    pub fn try_from_str(s: &str) -> Self {
        match s {
            "workflow_dag" => EvolutionArtifactKind::WorkflowDag,
            _ => EvolutionArtifactKind::RhaiScript,
        }
    }

    /// 根据产物代码特征推断类型：含编排结构化标记 → `WorkflowDag`，否则 → `RhaiScript`。
    /// 用于 LLM 生成代码未显式标注时的兜底分类。
    pub fn infer(code: &str) -> Self {
        let lowered = code.to_ascii_lowercase();
        if lowered.contains("workflow")
            || lowered.contains("depends_on")
            || lowered.contains("child_nodes")
            || lowered.contains("\"edges\"")
        {
            EvolutionArtifactKind::WorkflowDag
        } else {
            EvolutionArtifactKind::RhaiScript
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedTool {
    pub id: String,
    pub name: String,
    pub code: String,
    pub description: String,
    pub test_coverage: f64,
    pub created_at: i64,
    pub usage_count: u32,
    pub success_rate: f64,
    /// 产物类型标注：决定执行引擎（Rhai 脚本 / 工作流 DAG）。
    /// 默认 `RhaiScript`，老数据反序列化时自动补齐。
    #[serde(default)]
    pub artifact_kind: EvolutionArtifactKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCreationRequest {
    pub pattern_description: String,
    pub context: String,
    pub available_tools: Vec<String>,
}

pub trait LlmToolProvider: Send + Sync {
    fn generate_tool_code(
        &self,
        request: &ToolCreationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GeneratedTool, String>> + Send + '_>>;

    fn improve_tool_code(
        &self,
        tool: &GeneratedTool,
        error: &str,
    ) -> Pin<Box<dyn Future<Output = Result<GeneratedTool, String>> + Send + '_>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureStep {
    pub order: usize,
    pub action: String,
    pub tool: Option<String>,
    pub condition: Option<String>,
    pub error_handling: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmMutationRequest {
    pub skill_name: String,
    pub current_steps: Vec<ProcedureStep>,
    pub failure_evidence: Vec<String>,
    pub success_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMutationResponse {
    pub revised_steps: Vec<ProcedureStep>,
    pub reasoning: String,
    pub confidence: f64,
}

pub type LlmMutationFuture<'a> =
    Pin<Box<dyn Future<Output = Result<LlmMutationResponse, String>> + Send + 'a>>;

pub trait LlmEvolutionProvider: Send + Sync {
    fn generate_mutation(&self, request: &LlmMutationRequest) -> LlmMutationFuture<'_>;
    fn evaluate_quality(
        &self,
        content: &str,
        context: &str,
    ) -> Pin<Box<dyn Future<Output = Result<f64, String>> + Send + '_>>;
}

pub type LlmJudgeFuture<'a> = Pin<Box<dyn Future<Output = Result<f64, String>> + Send + 'a>>;

pub trait LlmJudge: Send + Sync {
    fn evaluate_reasoning(&self, reasoning: &str, context: &str) -> LlmJudgeFuture<'_>;
    fn evaluate_tool_efficiency(
        &self,
        tool_name: &str,
        args: &str,
        result: &str,
    ) -> LlmJudgeFuture<'_>;
}

pub trait LlmTextGradProvider: Send + Sync {
    fn compute_gradient(
        &self,
        node_content: &str,
        output_feedback: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + '_>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RewardCategory {
    Correctness,
    Coherence,
    Completeness,
    Efficiency,
    Safety,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepReward {
    pub step_index: usize,
    pub reward: f64,
    pub reasoning: String,
    pub categories: Vec<(RewardCategory, f64)>,
}

pub trait PrmLlmProvider: Send + Sync {
    fn evaluate_step(
        &self,
        step_content: &str,
        context: &str,
        previous_steps: &[String],
    ) -> Pin<Box<dyn Future<Output = Result<StepReward, String>> + Send + '_>>;
}

impl GeneratedTool {
    /// 构造计算型工具（默认产物类型为 `RhaiScript`）。
    pub fn new(name: &str, code: &str, description: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            code: code.to_string(),
            description: description.to_string(),
            test_coverage: 0.0,
            created_at: Utc::now().timestamp(),
            usage_count: 0,
            success_rate: 0.0,
            artifact_kind: EvolutionArtifactKind::RhaiScript,
        }
    }

    /// 按产物类型构造工具（`RhaiScript` 计算型 / `WorkflowDag` 编排型）。
    pub fn with_artifact_kind(
        name: &str,
        code: &str,
        description: &str,
        artifact_kind: EvolutionArtifactKind,
    ) -> Self {
        let mut tool = Self::new(name, code, description);
        tool.artifact_kind = artifact_kind;
        tool
    }
}

impl ToolCreationRequest {
    pub fn new(pattern_description: &str, context: &str, available_tools: Vec<String>) -> Self {
        Self {
            pattern_description: pattern_description.to_string(),
            context: context.to_string(),
            available_tools,
        }
    }
}
