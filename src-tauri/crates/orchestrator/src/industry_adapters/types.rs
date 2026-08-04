// SPDX-License-Identifier: AGPL-3.0-only

//! 行业适配器类型定义
//!
//! 定义行业适配器使用的核心数据结构，包括：
//! - 行业上下文 (IndustryContext)
//! - 任务类型 (MissionType)
//! - 反思模板 (ReflectionTemplate)
//! - 进化约束 (EvolutionConstraints)
//! - 验收标准 (AcceptanceCriterion)
//! - 学习配置 (IndustryLearningConfig)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── 任务类型 ──────────────────────────────────────────────────

/// 行业任务类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionType {
    /// 调研分析
    Research,
    /// 内容生成
    Generation,
    /// 审查评估
    Review,
    /// 修复优化
    Fix,
    /// 规划设计
    Planning,
    /// 监控运维
    Monitoring,
    /// 报告输出
    Reporting,
    /// 对话咨询
    Consultation,
}

impl MissionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Research => "research",
            Self::Generation => "generation",
            Self::Review => "review",
            Self::Fix => "fix",
            Self::Planning => "planning",
            Self::Monitoring => "monitoring",
            Self::Reporting => "reporting",
            Self::Consultation => "consultation",
        }
    }
}

// ── 行业上下文 ──────────────────────────────────────────────────

/// 行业执行上下文
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndustryContext {
    /// 会话 ID
    pub session_id: Option<String>,
    /// 用户 ID
    pub user_id: Option<String>,
    /// 工作空间 ID
    pub workspace_id: Option<String>,
    /// 业务输入数据
    #[serde(default)]
    pub inputs: serde_json::Value,
    /// 历史执行结果
    #[serde(default)]
    pub history: Vec<serde_json::Value>,
    /// 关联的知识库 ID 列表
    #[serde(default)]
    pub knowledge_ids: Vec<String>,
    /// 额外元数据
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl IndustryContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_inputs(mut self, inputs: serde_json::Value) -> Self {
        self.inputs = inputs;
        self
    }

    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

// ── 反思模板 ──────────────────────────────────────────────────

/// 行业特定反思模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionTemplate {
    /// 模板 ID
    pub id: String,
    /// 模板名称
    pub name: String,
    /// 质量评估权重配置
    #[serde(default)]
    pub quality_weights: QualityWeights,
    /// 质量检查点列表
    #[serde(default)]
    pub checkpoints: Vec<ReflectionCheckpoint>,
    /// 反思提示词模板
    #[serde(default)]
    pub prompts: Vec<String>,
    /// 结构化验收标准（pass/fail AC）
    #[serde(default)]
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    /// 是否启用结构化验收
    #[serde(default)]
    pub structured_verification_enabled: bool,
}

impl Default for ReflectionTemplate {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "默认反思模板".to_string(),
            quality_weights: QualityWeights::default(),
            checkpoints: Vec::new(),
            prompts: vec!["请评估本次执行的质量和效率".to_string()],
            acceptance_criteria: Vec::new(),
            structured_verification_enabled: false,
        }
    }
}

impl ReflectionTemplate {
    /// 评估验收标准，返回 pass/fail 结果
    pub fn evaluate_acceptance(&self, scores: &HashMap<String, f64>) -> AcceptanceResult {
        if self.acceptance_criteria.is_empty() {
            return AcceptanceResult {
                passed: true,
                total_criteria: 0,
                passed_criteria: 0,
                failed_criteria: 0,
                details: Vec::new(),
                overall_score: 1.0,
            };
        }

        let mut passed_criteria = 0;
        let mut failed_criteria = 0;
        let mut details = Vec::new();
        let mut total_weight = 0.0;
        let mut weighted_score = 0.0;

        for criterion in &self.acceptance_criteria {
            let score = scores.get(&criterion.id).copied().unwrap_or(0.0);
            let passed = score >= criterion.threshold;

            if passed {
                passed_criteria += 1;
            } else {
                failed_criteria += 1;
            }

            total_weight += criterion.weight;
            weighted_score += score * criterion.weight;

            details.push(CriterionResult {
                criterion_id: criterion.id.clone(),
                criterion_name: criterion.name.clone(),
                score,
                threshold: criterion.threshold,
                passed,
                is_critical: criterion.is_critical,
            });
        }

        let overall_score = if total_weight > 0.0 {
            weighted_score / total_weight
        } else {
            0.0
        };

        // 关键标准失败则整体失败
        let has_critical_failure = details.iter().any(|d| d.is_critical && !d.passed);

        AcceptanceResult {
            passed: !has_critical_failure && failed_criteria == 0,
            total_criteria: self.acceptance_criteria.len(),
            passed_criteria,
            failed_criteria,
            details,
            overall_score,
        }
    }

    /// 获取关键验收标准
    pub fn critical_criteria(&self) -> Vec<&AcceptanceCriterion> {
        self.acceptance_criteria.iter().filter(|c| c.is_critical).collect()
    }

    /// 获取非关键验收标准
    pub fn non_critical_criteria(&self) -> Vec<&AcceptanceCriterion> {
        self.acceptance_criteria.iter().filter(|c| !c.is_critical).collect()
    }
}

/// 验收结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceResult {
    /// 是否通过
    pub passed: bool,
    /// 总标准数
    pub total_criteria: usize,
    /// 通过的标准数
    pub passed_criteria: usize,
    /// 失败的标准数
    pub failed_criteria: usize,
    /// 各标准的评估详情
    pub details: Vec<CriterionResult>,
    /// 综合得分（加权平均）
    pub overall_score: f64,
}

/// 单个标准的评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionResult {
    /// 标准 ID
    pub criterion_id: String,
    /// 标准名称
    pub criterion_name: String,
    /// 实际得分
    pub score: f64,
    /// 阈值
    pub threshold: f64,
    /// 是否通过
    pub passed: bool,
    /// 是否为关键标准
    pub is_critical: bool,
}

/// 质量评估权重
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityWeights {
    /// 任务完成度权重
    #[serde(default = "default_weight")]
    pub task_completion: f64,
    /// 输出质量权重
    #[serde(default = "default_weight")]
    pub output_quality: f64,
    /// 效率权重
    #[serde(default = "default_weight")]
    pub efficiency: f64,
    /// 成本效益权重
    #[serde(default = "default_weight")]
    pub cost_efficiency: f64,
}

fn default_weight() -> f64 {
    0.25
}

impl Default for QualityWeights {
    fn default() -> Self {
        Self { task_completion: 0.3, output_quality: 0.3, efficiency: 0.2, cost_efficiency: 0.2 }
    }
}

/// 反思检查点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionCheckpoint {
    /// 检查点 ID
    pub id: String,
    /// 检查点名称
    pub name: String,
    /// 检查维度
    pub dimension: String,
    /// 检查描述
    pub description: String,
    /// 权重
    #[serde(default = "default_weight")]
    pub weight: f64,
}

// ── 进化约束 ──────────────────────────────────────────────────

/// 行业特定进化约束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConstraints {
    /// 受保护的步骤 ID 列表（不可删除）
    #[serde(default)]
    pub protected_steps: Vec<ProtectedStep>,
    /// 步骤依赖关系
    #[serde(default)]
    pub step_dependencies: Vec<StepDependency>,
    /// 最小步骤数
    #[serde(default = "default_min_steps")]
    pub min_steps: usize,
    /// 最大步骤数
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,
    /// 是否必须按序执行
    #[serde(default)]
    pub must_follow_order: bool,
    /// 禁止的优化类型
    #[serde(default)]
    pub forbidden_optimizations: Vec<ForbiddenOptimization>,
    /// 质量阈值
    #[serde(default)]
    pub quality_thresholds: QualityThresholds,
}

fn default_min_steps() -> usize {
    3
}
fn default_max_steps() -> usize {
    30
}

impl Default for EvolutionConstraints {
    fn default() -> Self {
        Self {
            protected_steps: Vec::new(),
            step_dependencies: Vec::new(),
            min_steps: 3,
            max_steps: 30,
            must_follow_order: false,
            forbidden_optimizations: Vec::new(),
            quality_thresholds: QualityThresholds::default(),
        }
    }
}

/// 受保护的步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedStep {
    /// 步骤 ID
    pub step_id: String,
    /// 保护原因
    pub reason: String,
}

/// 步骤依赖关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDependency {
    /// 前置步骤 ID
    pub from: String,
    /// 后置步骤 ID
    pub to: String,
    /// 依赖类型：hard（必须）/soft（建议）
    #[serde(rename = "type")]
    pub dep_type: DependencyType,
}

/// 依赖类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    /// 硬依赖：必须按序执行
    Hard,
    /// 软依赖：可跳过但不推荐
    Soft,
}

/// 禁止的优化类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForbiddenOptimization {
    /// 优化类型
    pub optimization_type: String,
    /// 禁止原因
    pub reason: String,
}

/// 质量阈值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityThresholds {
    /// 最低准确率
    #[serde(default = "default_threshold_80")]
    pub min_accuracy: f64,
    /// 最低成功率
    #[serde(default = "default_threshold_70")]
    pub min_success_rate: f64,
    /// 最低质量分数
    #[serde(default = "default_threshold_60")]
    pub min_quality_score: f64,
}

fn default_threshold_80() -> f64 {
    0.8
}
fn default_threshold_70() -> f64 {
    0.7
}
fn default_threshold_60() -> f64 {
    0.6
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self { min_accuracy: 0.8, min_success_rate: 0.7, min_quality_score: 0.6 }
    }
}

// ── 验收标准 ──────────────────────────────────────────────────

/// 验收标准
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    /// 标准 ID
    pub id: String,
    /// 标准名称
    pub name: String,
    /// 标准描述
    pub description: String,
    /// 评估维度
    pub dimension: String,
    /// 合格阈值
    #[serde(default = "default_threshold_70")]
    pub threshold: f64,
    /// 是否为关键标准（不达标则整体失败）
    #[serde(default)]
    pub is_critical: bool,
    /// 权重
    #[serde(default = "default_weight")]
    pub weight: f64,
}

// ── 学习配置 ──────────────────────────────────────────────────

/// 行业学习配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryLearningConfig {
    /// 配置版本
    #[serde(default = "default_version")]
    pub version: u32,
    /// 反思配置
    #[serde(default)]
    pub reflection: ReflectionConfig,
    /// 进化配置
    #[serde(default)]
    pub evolution: EvolutionConfig,
    /// 自改进配置
    #[serde(default)]
    pub self_improvement: SelfImprovementConfig,
    /// 强化学习配置
    #[serde(default)]
    pub reinforcement_learning: ReinforcementLearningConfig,
}

fn default_version() -> u32 {
    1
}

impl Default for IndustryLearningConfig {
    fn default() -> Self {
        Self {
            version: 1,
            reflection: ReflectionConfig::default(),
            evolution: EvolutionConfig::default(),
            self_improvement: SelfImprovementConfig::default(),
            reinforcement_learning: ReinforcementLearningConfig::default(),
        }
    }
}

/// 反思配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionConfig {
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 自动触发时机
    #[serde(default)]
    pub auto_reflect_on: Vec<AutoReflectTrigger>,
    /// 反思模板路径
    #[serde(default)]
    pub template_path: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for ReflectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_reflect_on: vec![
                AutoReflectTrigger::WorkflowComplete,
                AutoReflectTrigger::TaskFailure,
            ],
            template_path: None,
        }
    }
}

/// 自动反思触发时机
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoReflectTrigger {
    /// 工作流完成
    WorkflowComplete,
    /// 任务失败
    TaskFailure,
    /// 用户反馈
    UserFeedback,
    /// 审批通过
    ApprovalPassed,
    /// 审批拒绝
    ApprovalRejected,
}

/// 进化配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvolutionConfig {
    /// 工作流进化配置
    #[serde(default)]
    pub workflow_evolver: WorkflowEvolverConfig,
    /// 技能进化配置
    #[serde(default)]
    pub skill_evolver: SkillEvolverConfig,
}

/// 工作流进化器配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowEvolverConfig {
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
    /// 自动触发配置
    #[serde(default)]
    pub auto_trigger: AutoTriggerConfig,
}

/// 自动触发配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTriggerConfig {
    /// 连续失败次数阈值
    #[serde(default = "default_consecutive_failures")]
    pub consecutive_failures: u32,
    /// 最小使用次数
    #[serde(default = "default_min_usages")]
    pub min_usages: u32,
    /// 成功率阈值
    #[serde(default = "default_success_threshold")]
    pub success_threshold: f64,
}

fn default_consecutive_failures() -> u32 {
    3
}
fn default_min_usages() -> u32 {
    5
}
fn default_success_threshold() -> f64 {
    0.6
}

impl Default for AutoTriggerConfig {
    fn default() -> Self {
        Self { consecutive_failures: 3, min_usages: 5, success_threshold: 0.6 }
    }
}

/// 技能进化器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEvolverConfig {
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
    /// 自动触发配置
    #[serde(default)]
    pub auto_trigger: AutoTriggerConfig,
}

impl Default for SkillEvolverConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_trigger: AutoTriggerConfig {
                consecutive_failures: 5,
                min_usages: 10,
                success_threshold: 0.5,
            },
        }
    }
}

/// 自改进配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovementConfig {
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
    /// 最大迭代轮数
    #[serde(default = "default_max_rounds")]
    pub max_rounds: u32,
    /// 收敛阈值
    #[serde(default = "default_convergence_threshold")]
    pub convergence_threshold: f64,
    /// 升级阈值（连续无进展多少次后请求人工介入）
    #[serde(default = "default_escalate_threshold")]
    pub escalate_threshold: u32,
}

fn default_max_rounds() -> u32 {
    3
}
fn default_convergence_threshold() -> f64 {
    0.85
}
fn default_escalate_threshold() -> u32 {
    2
}

impl Default for SelfImprovementConfig {
    fn default() -> Self {
        Self { enabled: false, max_rounds: 3, convergence_threshold: 0.85, escalate_threshold: 2 }
    }
}

/// 强化学习配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinforcementLearningConfig {
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
    /// 奖励模型名称
    #[serde(default)]
    pub reward_model: Option<String>,
    /// 自动训练阈值（经验池达到多少条后触发训练）
    #[serde(default = "default_rl_threshold")]
    pub auto_train_threshold: usize,
    /// 学习率
    #[serde(default = "default_rl_learning_rate")]
    pub learning_rate: f64,
    /// 折扣因子（gamma）
    #[serde(default = "default_rl_gamma")]
    pub gamma: f64,
    /// 探索率（epsilon）
    #[serde(default = "default_rl_epsilon")]
    pub epsilon: f64,
    /// 奖励权重配置
    #[serde(default)]
    pub reward_weights: RewardWeightConfig,
    /// 行业特定优化目标
    #[serde(default)]
    pub optimization_goals: Vec<String>,
}

fn default_rl_threshold() -> usize {
    50
}

fn default_rl_learning_rate() -> f64 {
    0.01
}

fn default_rl_gamma() -> f64 {
    0.95
}

fn default_rl_epsilon() -> f64 {
    0.1
}

/// RL 奖励权重配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardWeightConfig {
    /// 质量奖励权重
    #[serde(default = "default_weight_quality")]
    pub quality: f64,
    /// 效率奖励权重
    #[serde(default = "default_weight_efficiency")]
    pub efficiency: f64,
    /// 成本奖励权重
    #[serde(default = "default_weight_cost")]
    pub cost: f64,
    /// 创新奖励权重
    #[serde(default = "default_weight_innovation")]
    pub innovation: f64,
    /// 用户满意度奖励权重
    #[serde(default = "default_weight_satisfaction")]
    pub satisfaction: f64,
}

fn default_weight_quality() -> f64 {
    0.35
}
fn default_weight_efficiency() -> f64 {
    0.25
}
fn default_weight_cost() -> f64 {
    0.15
}
fn default_weight_innovation() -> f64 {
    0.15
}
fn default_weight_satisfaction() -> f64 {
    0.1
}

impl Default for RewardWeightConfig {
    fn default() -> Self {
        Self {
            quality: default_weight_quality(),
            efficiency: default_weight_efficiency(),
            cost: default_weight_cost(),
            innovation: default_weight_innovation(),
            satisfaction: default_weight_satisfaction(),
        }
    }
}

impl Default for ReinforcementLearningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            reward_model: None,
            auto_train_threshold: 50,
            learning_rate: 0.01,
            gamma: 0.95,
            epsilon: 0.1,
            reward_weights: RewardWeightConfig::default(),
            optimization_goals: vec!["quality_improvement".to_string()],
        }
    }
}

// ── 预设工作流步骤 ──────────────────────────────────────────

/// 预设工作流步骤
///
/// 定义行业的标准工作流步骤模板，用于初始化工作流编排。
/// 步骤可以是"骨架步骤"（必须执行）或"可选步骤"（根据任务动态添加）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetWorkflowStep {
    /// 步骤 ID
    pub id: String,
    /// 步骤名称
    pub name: String,
    /// 步骤描述
    pub description: String,
    /// 目标/任务
    pub goal: String,
    /// 负责角色
    pub role: String,
    /// 依赖的前置步骤 ID 列表
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// 是否为骨架步骤（不可跳过）
    #[serde(default)]
    pub is_skeleton: bool,
    /// 是否为可选步骤（AI 可根据任务动态添加）
    #[serde(default)]
    pub is_optional: bool,
    /// 步骤顺序（越小越先执行）
    #[serde(default)]
    pub order: u32,
    /// 是否使用多智能体协作
    #[serde(default)]
    pub multi_agent: bool,
    /// 多智能体协作模式（swarm/debate/blackboard）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordination_mode: Option<String>,
    /// 多智能体最大协作轮数
    #[serde(default = "default_preset_max_rounds")]
    pub max_rounds: u32,
    /// 是否支持并行执行（与其他步骤并行）
    #[serde(default)]
    pub parallel_supported: bool,
}

fn default_preset_max_rounds() -> u32 {
    3
}

impl PresetWorkflowStep {
    /// 创建骨架步骤
    pub fn skeleton(id: &str, name: &str, goal: &str, role: &str, order: u32) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: goal.to_string(),
            goal: goal.to_string(),
            role: role.to_string(),
            depends_on: Vec::new(),
            is_skeleton: true,
            is_optional: false,
            order,
            multi_agent: false,
            coordination_mode: None,
            max_rounds: 3,
            parallel_supported: false,
        }
    }

    /// 创建可选步骤
    pub fn optional(id: &str, name: &str, goal: &str, role: &str, order: u32) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: goal.to_string(),
            goal: goal.to_string(),
            role: role.to_string(),
            depends_on: Vec::new(),
            is_skeleton: false,
            is_optional: true,
            order,
            multi_agent: false,
            coordination_mode: None,
            max_rounds: 3,
            parallel_supported: false,
        }
    }

    /// 设置为 Swarm 多智能体模式
    pub fn with_swarm(mut self, max_rounds: u32) -> Self {
        self.multi_agent = true;
        self.coordination_mode = Some("swarm".to_string());
        self.max_rounds = max_rounds;
        self
    }

    /// 设置为 Debate 多智能体模式
    pub fn with_debate(mut self, max_rounds: u32) -> Self {
        self.multi_agent = true;
        self.coordination_mode = Some("debate".to_string());
        self.max_rounds = max_rounds;
        self
    }

    /// 设置为 Blackboard 多智能体模式
    pub fn with_blackboard(mut self, max_rounds: u32) -> Self {
        self.multi_agent = true;
        self.coordination_mode = Some("blackboard".to_string());
        self.max_rounds = max_rounds;
        self
    }

    /// 支持并行执行
    pub fn with_parallel(mut self) -> Self {
        self.parallel_supported = true;
        self
    }

    /// 添加依赖
    pub fn with_dependency(mut self, dep: &str) -> Self {
        self.depends_on.push(dep.to_string());
        self
    }

    /// 添加多个依赖
    pub fn with_dependencies(mut self, deps: &[&str]) -> Self {
        self.depends_on.extend(deps.iter().map(|s| s.to_string()));
        self
    }
}

/// 获取行业预设工作流步骤
///
/// 每个行业可以定义自己的标准工作流步骤模板。
/// 这些步骤作为"骨架"，AI 可根据任务动态添加可选步骤。
///
/// # 多智能体协作策略
///
/// - **编码**: Swarm 模式（多个开发者协作实现）
/// - **代码审查**: Debate 模式（Reviewer vs Implementer 辩论）
/// - **测试**: Blackboard 模式（QA 团队共享测试结果）
/// - **编码 + 文档**: 支持并行执行
/// - **审查 + 安全审计**: 支持并行执行
pub fn get_software_dev_preset_steps() -> Vec<PresetWorkflowStep> {
    vec![
        // ── 骨架步骤（参考 WorkflowX 十步）──
        PresetWorkflowStep::skeleton(
            "requirements",
            "需求分析",
            "需求收集、分析与规格说明",
            "产品分析师",
            1,
        ),
        PresetWorkflowStep::skeleton(
            "architecture",
            "架构设计",
            "系统架构设计、技术选型",
            "架构师",
            2,
        )
        .with_dependency("requirements")
        .with_debate(2), // 架构师之间辩论技术选型
        // 编码步骤: Swarm 模式 + 支持与文档并行
        PresetWorkflowStep::skeleton(
            "coding",
            "编码实现",
            "功能实现与代码编写",
            "开发工程师",
            3,
        )
        .with_dependency("architecture")
        .with_swarm(3) // Swarm 协作: 多个开发者分工实现
        .with_parallel(), // 可以与文档编写并行
        // 代码审查: Debate 模式（Reviewer vs Implementer）
        PresetWorkflowStep::skeleton(
            "code_review",
            "代码审查",
            "代码审查与质量检查",
            "Code Reviewer",
            4,
        )
        .with_dependency("coding")
        .with_debate(2) // Debate: Reviewer 提出问题, Implementer 辩护
        .with_parallel(), // 可以与安全审计并行
        // 测试: Blackboard 模式（QA 团队共享测试结果）
        PresetWorkflowStep::skeleton("testing", "测试验证", "单元测试、集成测试", "测试工程师", 5)
            .with_dependency("coding")
            .with_blackboard(2), // Blackboard: 多个测试用例结果汇总
        PresetWorkflowStep::skeleton(
            "deployment",
            "部署上线",
            "构建、部署与上线",
            "DevOps 工程师",
            6,
        )
        .with_dependencies(&["code_review", "testing"]),
        // ── 可选步骤（AI 可根据任务动态添加）──
        // 安全审计: 与代码审查并行, Swarm 模式, 依赖 coding 完成
        PresetWorkflowStep::optional(
            "security_audit",
            "安全审计",
            "代码安全审计与漏洞扫描",
            "安全工程师",
            4,
        )
        .with_dependency("coding")
        .with_swarm(2) // 多个安全扫描器并行
        .with_parallel(),
        // 性能优化: 与代码审查并行, 依赖 coding 完成
        PresetWorkflowStep::optional(
            "performance_opt",
            "性能优化",
            "性能分析与优化建议",
            "性能工程师",
            4,
        )
        .with_dependency("coding")
        .with_parallel(),
        // 文档编写: 与代码审查并行, 依赖 coding 完成
        PresetWorkflowStep::optional(
            "documentation",
            "文档编写",
            "API 文档、技术文档",
            "技术写作者",
            4,
        )
        .with_dependency("coding")
        .with_parallel(),
        // 监控配置: 与部署并行
        PresetWorkflowStep::optional(
            "monitoring_setup",
            "监控配置",
            "监控与告警配置",
            "SRE 工程师",
            7,
        )
        .with_dependency("deployment"),
    ]
}

/// 获取行业预设步骤
///
/// 根据行业 ID 返回对应的预设工作流步骤。
pub fn get_preset_steps_for_industry(industry_id: &str) -> Vec<PresetWorkflowStep> {
    match industry_id {
        "software-dev" => get_software_dev_preset_steps(),
        _ => Vec::new(), // 其他行业暂时没有预设步骤
    }
}
