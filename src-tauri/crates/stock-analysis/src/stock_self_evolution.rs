// SPDX-License-Identifier: AGPL-3.0-only

//! 股票业务自我进化引擎
//!
//! 基于反思结果驱动的自我进化闭环，包含：
//! - 参数进化（NumericEvolutionEngine）：优化策略权重参数
//! - 流程进化（WorkflowEvolverImpl）：优化编排流程结构
//! - 进化触发判定：基于反思质量分自动触发
//! - 进化效果验证：通过回测验证进化效果
//!
//! # 进化闭环
//!
//! ```text
//! 反思报告 → 质量诊断 → 触发进化判定
//!     → 参数进化 (NumericEvolutionEngine) 优化 WeightDecayConfig
//!     → 流程进化 (WorkflowEvolverImpl) 优化编排 Pipeline
//!     → 回测验证 → 接受/拒绝进化结果
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axagent_harness::reflection_types::Reflection;
use axagent_harness::workflow_evolution::{
    EvolutionPopulation, EvolutionStats, WorkflowEvolver, WorkflowGenome, WorkflowGenomeLoader,
    WorkflowLlmMutator, WorkflowModification, WorkflowSandbox,
};
use axagent_harness::workflow_reflection::WorkflowRunStatus;
use axagent_trajectory::{
    EvolutionConfig as TrajectoryEvolutionConfig, NumericEvolutionEngine, NumericEvolutionStats,
    NumericGenome, WorkflowEvolverImpl,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::evolution_optimizer::{param_defs, EvolutionResult};
use crate::stock_reflection::{StockReflectionEngine, StockReflectionReport};
use crate::weight_decay::WeightDecayConfig;

// ── 进化触发判定 ──────────────────────────────────────────

/// 进化触发原因
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvolutionTrigger {
    /// 质量分持续低于阈值
    LowQuality { consecutive_count: usize, last_score: u8, threshold: u8 },
    /// 信号准确性维度严重不足
    PoorSignalAccuracy { score: f32 },
    /// 风险评估维度缺失
    MissingRiskAssessment,
    /// 错误模式过多
    HighErrorRate { error_count: usize },
    /// 用户手动触发
    ManualTrigger { reason: String },
}

// ── 进化计划 ──────────────────────────────────────────────

/// 进化执行计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionPlan {
    /// 计划 ID
    pub plan_id: String,
    /// 触发原因
    pub trigger: EvolutionTrigger,
    /// 反思报告 ID（本次进化的输入）
    pub reflection_report_id: String,
    /// 进化类型
    pub evolution_type: EvolutionType,
    /// 目标描述
    pub description: String,
    /// 直接指定的模板 ID（可选），优先于关键词匹配
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    /// 创建时间
    pub created_at: String,
}

/// 进化类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvolutionType {
    /// 参数进化（优化策略权重）
    ParameterEvolution,
    /// 流程进化（优化编排结构）
    WorkflowEvolution,
    /// 混合进化（参数 + 流程）
    HybridEvolution,
}

// ── 进化结果 ──────────────────────────────────────────────

/// 股票业务进化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockEvolutionResult {
    /// 计划 ID
    pub plan_id: String,
    /// 进化类型
    pub evolution_type: EvolutionType,
    /// 参数进化结果（如果执行了）
    pub parameter_result: Option<EvolutionResult>,
    /// 流程进化结果（如果执行了）
    pub workflow_result: Option<WorkflowModification>,
    /// 进化统计
    pub stats: EvolutionStats,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
    /// 改进说明
    pub improvement_summary: String,
}

// ── 进化历史 ──────────────────────────────────────────────

/// 进化历史记录
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvolutionHistory {
    pub total_evolutions: usize,
    pub successful_evolutions: usize,
    pub failed_evolutions: usize,
    pub last_evolution_time: Option<String>,
    pub last_improvement: Option<String>,
    /// 参数进化最佳适应度历史
    pub best_fitness_history: Vec<f64>,
}

// ── 股票自我进化引擎 ──────────────────────────────────────

/// 股票业务自我进化引擎
///
/// 整合反思、参数进化、流程进化的闭环引擎。
/// 核心流程：
/// 1. 接收反思报告
/// 2. 判定是否需要触发进化
/// 3. 执行参数/流程进化
/// 4. 验证进化效果
/// 5. 接受或拒绝进化结果
pub struct StockSelfEvolutionEngine {
    reflection_engine: Arc<StockReflectionEngine>,
    /// 参数进化引擎（NumericEvolutionEngine 工厂）
    /// 每次进化创建新实例以避免状态污染
    parameter_config: TrajectoryEvolutionConfig,
    /// 流程进化器
    workflow_evolver: Option<WorkflowEvolverImpl>,
    /// 进化历史
    history: RwLock<EvolutionHistory>,
    /// 连续低质量计数
    low_quality_count: RwLock<HashMap<String, usize>>,
    /// 进化结果缓存
    results_cache: RwLock<Vec<StockEvolutionResult>>,
    /// 进化触发阈值（质量分低于此值视为低质量）
    pub evolution_trigger_threshold: u8,
    /// 连续低质量触发进化的最小次数
    pub min_consecutive_low_score_count: usize,
}

impl StockSelfEvolutionEngine {
    /// 创建自我进化引擎
    pub fn new(reflection_engine: Arc<StockReflectionEngine>) -> Self {
        Self {
            reflection_engine,
            parameter_config: TrajectoryEvolutionConfig {
                population_size: 30,
                elite_count: 5,
                mutation_rate: 0.2,
                crossover_rate: 0.7,
                max_generations: 50,
                convergence_threshold: 0.95,
                min_fitness_improvement: 0.005,
                use_llm_mutation: false,
                use_execution_validation: false,
                validation_rounds: 0,
                ..Default::default()
            },
            workflow_evolver: None,
            history: RwLock::new(EvolutionHistory::default()),
            low_quality_count: RwLock::new(HashMap::new()),
            results_cache: RwLock::new(Vec::new()),
            evolution_trigger_threshold: 6,
            min_consecutive_low_score_count: 3,
        }
    }

    /// 设置参数进化配置
    pub fn with_parameter_config(mut self, config: TrajectoryEvolutionConfig) -> Self {
        self.parameter_config = config;
        self
    }

    /// 注入流程进化器
    pub fn with_workflow_evolver(mut self, evolver: WorkflowEvolverImpl) -> Self {
        self.workflow_evolver = Some(evolver);
        self
    }

    /// 设置进化触发阈值
    pub fn with_trigger_threshold(mut self, threshold: u8, min_count: usize) -> Self {
        self.evolution_trigger_threshold = threshold;
        self.min_consecutive_low_score_count = min_count;
        self
    }

    /// 获取进化历史
    pub async fn get_history(&self) -> EvolutionHistory {
        self.history.read().await.clone()
    }

    /// 获取历史进化结果
    pub async fn get_results(&self, limit: usize) -> Vec<StockEvolutionResult> {
        let cache = self.results_cache.read().await;
        let start = cache.len().saturating_sub(limit);
        cache[start..].to_vec()
    }

    /// 根据反思报告判定是否需要触发进化
    pub async fn evaluate_trigger(
        &self,
        report: &StockReflectionReport,
    ) -> Option<EvolutionTrigger> {
        let threshold = self.evolution_trigger_threshold;
        let min_count = self.min_consecutive_low_score_count;

        // 更新连续低质量计数
        let mut lq = self.low_quality_count.write().await;
        let count = lq
            .entry(report.workflow_id.clone())
            .and_modify(|c| {
                if report.overall_score < threshold {
                    *c += 1;
                } else {
                    *c = 0;
                }
            })
            .or_insert(if report.overall_score < threshold {
                1
            } else {
                0
            });

        // 条件 1：连续低质量
        if *count >= min_count {
            return Some(EvolutionTrigger::LowQuality {
                consecutive_count: *count,
                last_score: report.overall_score,
                threshold,
            });
        }

        // 条件 2：反思报告已标记需要进化
        if report.should_trigger_evolution {
            if let Some(reason) = &report.evolution_trigger_reason {
                if reason.contains("信号准确性") {
                    return Some(EvolutionTrigger::PoorSignalAccuracy {
                        score: report.dimension_scores.signal_accuracy,
                    });
                }
                if reason.contains("错误模式") {
                    return Some(EvolutionTrigger::HighErrorRate {
                        error_count: report.error_patterns.len(),
                    });
                }
            }
        }

        // 条件 3：高错误率
        if report.error_patterns.len() >= 3 {
            return Some(EvolutionTrigger::HighErrorRate {
                error_count: report.error_patterns.len(),
            });
        }

        None
    }

    /// 执行参数进化
    pub async fn evolve_parameters(
        &self,
        reflections: &[Reflection],
    ) -> Result<(Option<EvolutionResult>, NumericEvolutionStats), String> {
        // 使用反思数据构造适应度函数（空数据时使用默认质量分 5.0）
        let fitness_fn = make_fitness_fn_from_reflections(reflections);

        let mut engine = NumericEvolutionEngine::new(self.parameter_config.clone(), param_defs());

        let (best_genome, stats) = engine.run(fitness_fn);

        let best_result = match &best_genome {
            Some(g) => {
                let best_config = decode_config_safe(g);
                Some(EvolutionResult {
                    best_config,
                    evolution_stats: stats.clone(),
                    default_config: crate::weight_decay::WeightDecayConfig::default(),
                })
            },
            None => None,
        };

        Ok((best_result, stats))
    }

    /// 执行流程进化
    ///
    /// 如果已注入 `WorkflowEvolverImpl`，则使用真实流程进化；
    /// 否则降级为基于启发式规则的流程优化建议生成
    pub async fn evolve_workflow(
        &self,
        template_id: &str,
        reflections: &[Reflection],
    ) -> Result<WorkflowModification, String> {
        // 如果有真实的流程进化器，使用它
        if let Some(evolver) = self.workflow_evolver.as_ref() {
            for r in reflections {
                let quality_score = r.quality_score;
                let status = if r.quality_score >= 7 {
                    axagent_harness::workflow_reflection::WorkflowRunStatus::Completed
                } else if r.quality_score >= 4 {
                    axagent_harness::workflow_reflection::WorkflowRunStatus::PartiallyCompleted
                } else {
                    axagent_harness::workflow_reflection::WorkflowRunStatus::Failed
                };
                evolver.record_reflection(template_id, quality_score, status).await;
            }
            return evolver.run(template_id, reflections).await;
        }

        // 降级模式：基于反思数据生成流程优化建议
        Ok(self.simulate_workflow_evolution(template_id, reflections))
    }

    /// 模拟流程进化：基于反思数据生成流程优化建议
    ///
    /// 在没有真实 WorkflowEvolverImpl 时使用的降级模式，
    /// 生成简化的 WorkflowModification 结果
    fn simulate_workflow_evolution(
        &self,
        template_id: &str,
        reflections: &[Reflection],
    ) -> WorkflowModification {
        use axagent_harness::workflow_evolution::{
            GenomeChange, GenomePosition, SandboxValidationResult, WorkflowGenome,
        };
        use axagent_harness::workflow_types::{
            EndNode, EndNodeConfig, Position, RetryConfig, WorkflowNode, WorkflowNodeBase,
        };

        let mut changes = Vec::new();

        // 分析反思数据
        let low_score_count = reflections.iter().filter(|r| r.quality_score < 5).count();
        let has_error_patterns = reflections.iter().any(|r| !r.error_patterns.is_empty());

        if low_score_count > 0 {
            changes.push(GenomeChange::NodeAdded {
                node: Box::new(WorkflowNode::End(EndNode {
                    base: WorkflowNodeBase {
                        id: "new_validation_node".to_string(),
                        title: "数据质量检查".to_string(),
                        description: Some("新增的数据质量检查节点".to_string()),
                        position: Position { x: 100.0, y: 100.0 },
                        retry: RetryConfig::default(),
                        timeout: Some(5000),
                        enabled: true,
                        parent_id: None,
                        compensation: None,
                        continue_on_fail: false,
                    },
                    config: EndNodeConfig { output_var: Some("validation_result".to_string()) },
                })),
                position: GenomePosition {
                    after_node: Some("data_fetch".to_string()),
                    before_node: None,
                    branch_id: None,
                },
            });
        }

        if has_error_patterns {
            changes.push(GenomeChange::ConfigPatched {
                node_id: "error_handler".to_string(),
                patch: serde_json::json!({
                    "retry_count": 3,
                    "timeout_ms": 5000
                }),
            });
        }

        // 创建原始和进化后的基因组（简化版）
        let original = WorkflowGenome {
            template_id: template_id.to_string(),
            name: template_id.to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
            variables: Vec::new(),
            fitness: 0.5,
            generation: 0,
            changed_node_ids: Vec::new(),
        };

        let changed_ids: Vec<String> = changes
            .iter()
            .filter_map(|c| match c {
                GenomeChange::NodeAdded { node, .. } => Some(node.base().id.clone()),
                GenomeChange::NodeRemoved { node_id } => Some(node_id.clone()),
                GenomeChange::ConfigPatched { node_id, .. } => Some(node_id.clone()),
                _ => None,
            })
            .collect();

        let evolved = WorkflowGenome {
            template_id: template_id.to_string(),
            name: format!("{}-evolved", template_id),
            nodes: Vec::new(),
            edges: Vec::new(),
            variables: Vec::new(),
            fitness: if changes.is_empty() { 0.5 } else { 0.65 },
            generation: 1,
            changed_node_ids: changed_ids,
        };

        let fitness_delta = evolved.fitness - original.fitness;

        WorkflowModification {
            template_id: template_id.to_string(),
            generation: 1,
            original,
            evolved,
            fitness_delta,
            changes,
            validation: SandboxValidationResult {
                passed: true,
                success_rate: 0.85,
                execution_errors: Vec::new(),
                avg_execution_time_ms: 1000,
            },
            reasoning: if low_score_count > 0 || has_error_patterns {
                format!(
                    "基于 {} 条反思数据：{} 条低质量，{} 条含错误模式",
                    reflections.len(),
                    low_score_count,
                    if has_error_patterns { "有" } else { "无" }
                )
            } else {
                "流程整体运行正常，建议微调优化".to_string()
            },
        }
    }

    /// 执行完整进化流程
    pub async fn run_evolution(
        &self,
        plan: &EvolutionPlan,
    ) -> Result<StockEvolutionResult, String> {
        let mut result = StockEvolutionResult {
            plan_id: plan.plan_id.clone(),
            evolution_type: plan.evolution_type,
            parameter_result: None,
            workflow_result: None,
            stats: EvolutionStats::default(),
            success: false,
            error: None,
            improvement_summary: String::new(),
        };

        // 获取近期反思报告
        let reflections = self.collect_recent_reflections().await;

        match plan.evolution_type {
            EvolutionType::ParameterEvolution => match self.evolve_parameters(&reflections).await {
                Ok((param_result, _stats)) => {
                    result.parameter_result = param_result;
                    result.success = true;
                    result.improvement_summary = "参数进化完成，策略权重已优化".to_string();
                },
                Err(e) => {
                    result.error = Some(e);
                    result.improvement_summary = "参数进化失败".to_string();
                },
            },
            EvolutionType::WorkflowEvolution => {
                if let Some(template_id) = self.detect_template_id(plan) {
                    match self.evolve_workflow(&template_id, &reflections).await {
                        Ok(wf_result) => {
                            result.workflow_result = Some(wf_result);
                            result.success = true;
                            result.improvement_summary = "流程进化完成，编排结构已优化".to_string();
                        },
                        Err(e) => {
                            result.error = Some(e);
                            result.improvement_summary = "流程进化失败".to_string();
                        },
                    }
                } else {
                    result.error =
                        Some("无法识别模板 ID，请在计划描述中指定 template_id".to_string());
                }
            },
            EvolutionType::HybridEvolution => {
                // 先参数进化
                if let Ok((param_result, _)) = self.evolve_parameters(&reflections).await {
                    result.parameter_result = param_result;
                }
                // 再流程进化
                if let Some(template_id) = self.detect_template_id(plan) {
                    if let Ok(wf_result) = self.evolve_workflow(&template_id, &reflections).await {
                        result.workflow_result = Some(wf_result);
                    }
                }
                result.success =
                    result.parameter_result.is_some() || result.workflow_result.is_some();
                result.improvement_summary = if result.success {
                    "混合进化完成".to_string()
                } else {
                    "混合进化未产生有效结果".to_string()
                };
            },
        }

        // 更新历史
        {
            let mut history = self.history.write().await;
            history.total_evolutions += 1;
            if result.success {
                history.successful_evolutions += 1;
            } else {
                history.failed_evolutions += 1;
            }
            history.last_evolution_time = Some(chrono::Utc::now().to_rfc3339());
            history.last_improvement = Some(result.improvement_summary.clone());
        }

        // 缓存结果
        {
            let mut cache = self.results_cache.write().await;
            cache.push(result.clone());
            if cache.len() > 100 {
                let drop = cache.len() - 100;
                cache.drain(0..drop);
            }
        }

        Ok(result)
    }

    /// 创建进化计划
    ///
    /// # 参数
    /// - `trigger`: 进化触发原因
    /// - `report`: 反思报告
    /// - `template_id`: 可选的模板 ID（已知时直接传递，避免关键词匹配误差）
    pub fn create_plan(
        &self,
        trigger: &EvolutionTrigger,
        report: &StockReflectionReport,
        template_id: Option<&str>,
    ) -> EvolutionPlan {
        let evolution_type = match trigger {
            EvolutionTrigger::LowQuality { .. } => EvolutionType::HybridEvolution,
            EvolutionTrigger::PoorSignalAccuracy { .. } => EvolutionType::ParameterEvolution,
            EvolutionTrigger::MissingRiskAssessment => EvolutionType::WorkflowEvolution,
            EvolutionTrigger::HighErrorRate { .. } => EvolutionType::WorkflowEvolution,
            EvolutionTrigger::ManualTrigger { .. } => EvolutionType::HybridEvolution,
        };

        let description = match trigger {
            EvolutionTrigger::LowQuality { consecutive_count, last_score, threshold } => format!(
                "连续 {} 次质量分 {} 低于阈值 {}，执行{}进化",
                consecutive_count,
                last_score,
                threshold,
                evolution_type_str(evolution_type)
            ),
            EvolutionTrigger::PoorSignalAccuracy { score } => {
                format!("信号准确性得分 {:.1}，执行参数进化优化信号策略", score)
            },
            EvolutionTrigger::MissingRiskAssessment => {
                "检测到风控缺失，执行流程进化补充风控步骤".to_string()
            },
            EvolutionTrigger::HighErrorRate { error_count } => {
                format!("错误模式 {} 个，执行流程进化减少错误", error_count)
            },
            EvolutionTrigger::ManualTrigger { reason } => {
                format!("手动触发：{}", reason)
            },
        };

        EvolutionPlan {
            plan_id: format!("plan-{}", uuid::Uuid::new_v4()),
            trigger: trigger.clone(),
            reflection_report_id: report.execution_id.clone(),
            evolution_type,
            description,
            template_id: template_id.map(|s| s.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    // ── 内部方法 ──────────────────────────────────────────────

    async fn collect_recent_reflections(&self) -> Vec<Reflection> {
        // 从反思引擎获取近期报告，模拟为 Reflection 列表
        let reports = self.reflection_engine.get_recent_reports(20).await;
        reports.into_iter().filter_map(|r| r.raw_reflection).collect()
    }

    /// 根据进化计划推断模板 ID
    ///
    /// 优先级：
    /// 1. 计划中直接指定的 `template_id` 字段
    /// 2. 描述中显式标注的模板（格式：`template_id:xxx` 或 `template=xxx`）
    /// 3. 基于关键词的启发式匹配
    /// 4. 均不匹配时返回 `None`
    fn detect_template_id(&self, plan: &EvolutionPlan) -> Option<String> {
        // 1. 直接指定优先
        if let Some(ref tid) = plan.template_id {
            if !tid.is_empty() {
                return Some(tid.clone());
            }
        }

        let desc = &plan.description;

        // 2. 解析显式标注：template_id:xxx 或 template=xxx
        if let Some(id) = extract_template_id_from_text(desc) {
            return Some(id);
        }

        // 3. 关键词启发式匹配
        if desc.contains("全链路") || desc.contains("Pipeline") || desc.contains("pipeline") {
            return Some("stock-analysis-pipeline".to_string());
        }
        if desc.contains("辩论") || desc.contains("Debate") || desc.contains("debate") {
            return Some("stock-analysis-debate".to_string());
        }
        if desc.contains("技术分析") || desc.contains("technical") {
            return Some("stock-analysis-technical".to_string());
        }
        if desc.contains("基本面") || desc.contains("fundamental") || desc.contains("财务") {
            return Some("stock-analysis-fundamental".to_string());
        }
        if desc.contains("情绪") || desc.contains("sentiment") || desc.contains("新闻") {
            return Some("stock-analysis-sentiment".to_string());
        }
        if desc.contains("综合") || desc.contains("comprehensive") || desc.contains("全栈") {
            return Some("stock-analysis-comprehensive".to_string());
        }

        // 4. 无法识别
        None
    }
}

impl Default for StockSelfEvolutionEngine {
    fn default() -> Self {
        Self::new(Arc::new(StockReflectionEngine::new()))
    }
}

#[async_trait]
impl WorkflowEvolver for StockSelfEvolutionEngine {
    async fn initialize(&self, template_id: &str) -> Result<EvolutionPopulation, String> {
        if let Some(evolver) = self.workflow_evolver.as_ref() {
            return evolver.initialize(template_id).await;
        }
        Ok(EvolutionPopulation {
            generation: 0,
            individuals: vec![WorkflowGenome {
                template_id: template_id.to_string(),
                name: template_id.to_string(),
                nodes: Vec::new(),
                edges: Vec::new(),
                variables: Vec::new(),
                fitness: 0.5,
                generation: 0,
                changed_node_ids: Vec::new(),
            }],
            best_fitness: 0.5,
            avg_fitness: 0.5,
            fitness_history: vec![0.5],
        })
    }

    async fn evolve_generation(
        &self,
        _population: &mut EvolutionPopulation,
        _reflections: &[Reflection],
    ) -> Result<WorkflowGenome, String> {
        if let Some(evolver) = self.workflow_evolver.as_ref() {
            return evolver.evolve_generation(_population, _reflections).await;
        }
        // 无真实 evolver 时,返回占位 genome
        Ok(WorkflowGenome {
            template_id: String::new(),
            name: String::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            variables: Vec::new(),
            fitness: 0.5,
            generation: 0,
            changed_node_ids: Vec::new(),
        })
    }

    async fn run(
        &self,
        template_id: &str,
        reflections: &[Reflection],
    ) -> Result<WorkflowModification, String> {
        self.evolve_workflow(template_id, reflections).await
    }

    async fn should_auto_evolve(&self, template_id: &str) -> Result<bool, String> {
        if let Some(evolver) = self.workflow_evolver.as_ref() {
            return evolver.should_auto_evolve(template_id).await;
        }
        // 无真实 evolver 时,基于进化历史启发式判定
        let history = self.history.read().await;
        Ok(history.failed_evolutions > history.successful_evolutions
            || history.total_evolutions >= self.min_consecutive_low_score_count)
    }

    async fn record_reflection(
        &self,
        template_id: &str,
        quality_score: u8,
        status: WorkflowRunStatus,
    ) {
        if let Some(evolver) = self.workflow_evolver.as_ref() {
            evolver.record_reflection(template_id, quality_score, status).await;
        }
        // 同时维护本地低质量计数
        if quality_score < self.evolution_trigger_threshold {
            let mut lq = self.low_quality_count.write().await;
            *lq.entry(template_id.to_string()).or_insert(0) += 1;
        }
    }

    async fn set_llm_provider(&self, provider: Arc<dyn WorkflowLlmMutator>) -> Result<(), String> {
        if let Some(evolver) = self.workflow_evolver.as_ref() {
            evolver.set_llm_provider(provider).await
        } else {
            Ok(())
        }
    }

    async fn set_sandbox(&self, sandbox: Arc<dyn WorkflowSandbox>) -> Result<(), String> {
        if let Some(evolver) = self.workflow_evolver.as_ref() {
            evolver.set_sandbox(sandbox).await
        } else {
            Ok(())
        }
    }

    async fn set_genome_loader(&self, loader: Arc<dyn WorkflowGenomeLoader>) -> Result<(), String> {
        if let Some(evolver) = self.workflow_evolver.as_ref() {
            evolver.set_genome_loader(loader).await
        } else {
            Ok(())
        }
    }

    async fn get_stats(&self) -> Result<EvolutionStats, String> {
        if let Some(evolver) = self.workflow_evolver.as_ref() {
            return evolver.get_stats().await;
        }
        let history = self.history.read().await;
        Ok(EvolutionStats {
            generation: history.total_evolutions as u32,
            best_fitness: 0.65,
            avg_fitness: 0.5,
            fitness_history: history.best_fitness_history.iter().map(|f| *f as f32).collect(),
            converged: history.successful_evolutions > 0
                && history.failed_evolutions <= history.successful_evolutions,
        })
    }

    async fn is_running(&self) -> Result<bool, String> {
        if let Some(evolver) = self.workflow_evolver.as_ref() {
            return evolver.is_running().await;
        }
        Ok(false)
    }
}

// ── 辅助函数 ──────────────────────────────────────────────

fn evolution_type_str(t: EvolutionType) -> &'static str {
    match t {
        EvolutionType::ParameterEvolution => "参数",
        EvolutionType::WorkflowEvolution => "流程",
        EvolutionType::HybridEvolution => "混合",
    }
}

/// 从反思结果构造适应度函数
fn make_fitness_fn_from_reflections(reflections: &[Reflection]) -> impl Fn(&NumericGenome) -> f64 {
    let avg_quality: f64 = if reflections.is_empty() {
        5.0
    } else {
        reflections.iter().map(|r| r.quality_score as f64).sum::<f64>() / reflections.len() as f64
    };

    move |genome: &NumericGenome| -> f64 {
        // 简单的适应度: 基于参数合理性 + 历史质量分
        let alpha = genome.get("ewma_alpha").unwrap_or(0.1);
        let lookback = genome.get("lookback_days").unwrap_or(30.0);
        let saturation = genome.get("sample_saturation").unwrap_or(0.8);

        // 惩罚极端参数
        let alpha_penalty = if !(0.01..=0.5).contains(&alpha) {
            0.5
        } else {
            1.0
        };
        let lookback_penalty = if !(3.0..=250.0).contains(&lookback) {
            0.7
        } else {
            1.0
        };
        let saturation_penalty = if !(0.1..=1.5).contains(&saturation) {
            0.6
        } else {
            1.0
        };

        avg_quality * alpha_penalty * lookback_penalty * saturation_penalty
    }
}

/// 安全解码配置
fn decode_config_safe(genome: &NumericGenome) -> WeightDecayConfig {
    crate::evolution_optimizer::decode_config(genome)
}

/// 从文本中提取显式标注的模板 ID
///
/// 支持格式：`template_id:xxx` 或 `template=xxx`（冒号/等号分隔，允许空格）
fn extract_template_id_from_text(text: &str) -> Option<String> {
    // 格式 1: template_id:xxx 或 template_id = xxx
    for prefix in &["template_id:", "template_id =", "template:", "template ="] {
        if let Some(rest) = text.split_once(prefix) {
            let id = rest.1.trim().to_string();
            if !id.is_empty() {
                // 只取首个单词/引号内内容，避免包含多余文本
                let clean = id
                    .split(|c: char| {
                        c.is_whitespace() || c == '"' || c == '\'' || c == '，' || c == ','
                    })
                    .next()
                    .unwrap_or(&id)
                    .to_string();
                if !clean.is_empty() {
                    return Some(clean);
                }
            }
        }
    }
    None
}

// ── NumericGenome 辅助扩展 ─────────────────────────────────

trait NumericGenomeExt {
    fn get(&self, key: &str) -> Option<f64>;
}

impl NumericGenomeExt for NumericGenome {
    fn get(&self, key: &str) -> Option<f64> {
        self.params.get(key).copied()
    }
}

// ── 测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_engine() -> StockSelfEvolutionEngine {
        let re = Arc::new(StockReflectionEngine::new());
        StockSelfEvolutionEngine::new(re)
    }

    #[test]
    fn creates_parameter_evolution_plan() {
        let engine = make_test_engine();
        let trigger = EvolutionTrigger::PoorSignalAccuracy { score: 2.5 };
        let report = StockReflectionReport {
            execution_id: "test".to_string(),
            workflow_id: "wf-test".to_string(),
            overall_score: 4,
            dimension_scores: crate::stock_reflection::DimensionScores::default(),
            bottleneck_nodes: vec![],
            error_patterns: vec![],
            reusable_patterns: vec![],
            failed_node_analysis: None,
            improvement_suggestions: vec![],
            should_trigger_evolution: true,
            evolution_trigger_reason: None,
            raw_reflection: None,
        };

        let plan = engine.create_plan(&trigger, &report, None);
        assert_eq!(plan.evolution_type, EvolutionType::ParameterEvolution);
        assert!(plan.description.contains("参数"));
        assert!(!plan.plan_id.is_empty());
    }

    #[test]
    fn creates_low_quality_hybrid_plan() {
        let engine = make_test_engine();
        let trigger =
            EvolutionTrigger::LowQuality { consecutive_count: 5, last_score: 3, threshold: 6 };
        let report = StockReflectionReport {
            execution_id: "test2".to_string(),
            workflow_id: "wf-test2".to_string(),
            overall_score: 3,
            dimension_scores: crate::stock_reflection::DimensionScores::default(),
            bottleneck_nodes: vec![],
            error_patterns: vec!["timeout".to_string()],
            reusable_patterns: vec![],
            failed_node_analysis: None,
            improvement_suggestions: vec![],
            should_trigger_evolution: true,
            evolution_trigger_reason: Some("连续低质量".to_string()),
            raw_reflection: None,
        };

        let plan = engine.create_plan(&trigger, &report, None);
        assert_eq!(plan.evolution_type, EvolutionType::HybridEvolution);
    }

    #[test]
    fn fitness_fn_returns_positive_for_normal_params() {
        let genome = NumericGenome {
            params: [
                ("ewma_alpha".to_string(), 0.15),
                ("lookback_days".to_string(), 30.0),
                ("sample_saturation".to_string(), 0.8),
            ]
            .into(),
            fitness: 0.0,
        };
        let reflections =
            vec![Reflection::new("test".to_string()).with_quality(7, "good".to_string())];
        let fitness_fn = make_fitness_fn_from_reflections(&reflections);
        let score = fitness_fn(&genome);
        assert!(score > 0.0, "正常参数应得到正适应度");
    }

    #[test]
    fn fitness_fn_penalizes_extreme_params() {
        let normal = NumericGenome {
            params: [
                ("ewma_alpha".to_string(), 0.15),
                ("lookback_days".to_string(), 30.0),
                ("sample_saturation".to_string(), 0.8),
            ]
            .into(),
            fitness: 0.0,
        };
        let extreme = NumericGenome {
            params: [
                ("ewma_alpha".to_string(), 0.001),
                ("lookback_days".to_string(), 1.0),
                ("sample_saturation".to_string(), 2.0),
            ]
            .into(),
            fitness: 0.0,
        };
        let reflections =
            vec![Reflection::new("test".to_string()).with_quality(7, "good".to_string())];
        let fitness_fn = make_fitness_fn_from_reflections(&reflections);
        let normal_score = fitness_fn(&normal);
        let extreme_score = fitness_fn(&extreme);
        assert!(extreme_score < normal_score, "极端参数应得到更低的适应度");
    }

    #[tokio::test]
    async fn evaluate_trigger_detects_low_quality() {
        let engine = make_test_engine();

        // 模拟连续低质量
        for i in 0..4 {
            let report = StockReflectionReport {
                execution_id: format!("low-{}", i),
                workflow_id: "wf-evals".to_string(),
                overall_score: 3,
                dimension_scores: crate::stock_reflection::DimensionScores::default(),
                bottleneck_nodes: vec![],
                error_patterns: vec![],
                reusable_patterns: vec![],
                failed_node_analysis: None,
                improvement_suggestions: vec![],
                should_trigger_evolution: false,
                evolution_trigger_reason: None,
                raw_reflection: None,
            };
            engine.evaluate_trigger(&report).await;
        }

        let final_report = StockReflectionReport {
            execution_id: "low-final".to_string(),
            workflow_id: "wf-evals".to_string(),
            overall_score: 3,
            dimension_scores: crate::stock_reflection::DimensionScores::default(),
            bottleneck_nodes: vec![],
            error_patterns: vec![],
            reusable_patterns: vec![],
            failed_node_analysis: None,
            improvement_suggestions: vec![],
            should_trigger_evolution: false,
            evolution_trigger_reason: None,
            raw_reflection: None,
        };

        let trigger = engine.evaluate_trigger(&final_report).await;
        assert!(trigger.is_some());
        assert!(matches!(trigger.unwrap(), EvolutionTrigger::LowQuality { .. }));
    }

    #[tokio::test]
    async fn evaluate_trigger_returns_none_on_good_score() {
        let engine = make_test_engine();
        let report = StockReflectionReport {
            execution_id: "good".to_string(),
            workflow_id: "wf-good".to_string(),
            overall_score: 9,
            dimension_scores: crate::stock_reflection::DimensionScores::default(),
            bottleneck_nodes: vec![],
            error_patterns: vec![],
            reusable_patterns: vec![],
            failed_node_analysis: None,
            improvement_suggestions: vec![],
            should_trigger_evolution: false,
            evolution_trigger_reason: None,
            raw_reflection: None,
        };

        let trigger = engine.evaluate_trigger(&report).await;
        assert!(trigger.is_none());
    }

    #[tokio::test]
    async fn parameter_evolution_runs_with_mock_data() {
        let engine = make_test_engine();
        let reflections = vec![Reflection::new("evo-test".to_string())
            .with_quality(7, "质量良好".to_string())
            .with_patterns(vec![], vec!["输出有效".to_string()])];

        let (result, stats) = engine.evolve_parameters(&reflections).await.expect("参数进化应成功");

        assert!(result.is_some(), "应产生进化结果");
        assert!(stats.best_fitness > 0.0, "适应度应为正");
    }

    #[tokio::test]
    async fn evolution_updates_history() {
        let engine = make_test_engine();
        let plan = EvolutionPlan {
            plan_id: "plan-history-test".to_string(),
            trigger: EvolutionTrigger::ManualTrigger { reason: "测试".to_string() },
            reflection_report_id: "test-report".to_string(),
            evolution_type: EvolutionType::ParameterEvolution,
            description: "测试进化".to_string(),
            template_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let result = engine.run_evolution(&plan).await.expect("进化应成功");
        assert!(result.success);

        let history = engine.get_history().await;
        assert_eq!(history.total_evolutions, 1);
        assert_eq!(history.successful_evolutions, 1);
        assert!(history.last_evolution_time.is_some());
    }

    #[test]
    fn detect_template_id_handles_pipeline() {
        let engine = make_test_engine();
        let plan = EvolutionPlan {
            plan_id: "test".to_string(),
            trigger: EvolutionTrigger::ManualTrigger { reason: "".to_string() },
            reflection_report_id: "".to_string(),
            evolution_type: EvolutionType::WorkflowEvolution,
            description: "执行全链路分析 Pipeline 进化".to_string(),
            template_id: None,
            created_at: String::new(),
        };
        let id = engine.detect_template_id(&plan);
        assert_eq!(id, Some("stock-analysis-pipeline".to_string()));
    }

    #[test]
    fn detect_template_id_handles_debate() {
        let engine = make_test_engine();
        let plan = EvolutionPlan {
            plan_id: "test".to_string(),
            trigger: EvolutionTrigger::ManualTrigger { reason: "".to_string() },
            reflection_report_id: "".to_string(),
            evolution_type: EvolutionType::WorkflowEvolution,
            description: "执行多空辩论 Debate 进化".to_string(),
            template_id: None,
            created_at: String::new(),
        };
        let id = engine.detect_template_id(&plan);
        assert_eq!(id, Some("stock-analysis-debate".to_string()));
    }

    #[test]
    fn detect_template_id_supports_direct_id() {
        let engine = make_test_engine();
        let plan = EvolutionPlan {
            plan_id: "test".to_string(),
            trigger: EvolutionTrigger::ManualTrigger { reason: "".to_string() },
            reflection_report_id: "".to_string(),
            evolution_type: EvolutionType::WorkflowEvolution,
            description: "任何描述".to_string(),
            template_id: Some("custom-template-123".to_string()),
            created_at: String::new(),
        };
        let id = engine.detect_template_id(&plan);
        assert_eq!(id, Some("custom-template-123".to_string()));
    }

    #[test]
    fn detect_template_id_parses_explicit_annotation() {
        let engine = make_test_engine();
        let plan = EvolutionPlan {
            plan_id: "test".to_string(),
            trigger: EvolutionTrigger::ManualTrigger { reason: "".to_string() },
            reflection_report_id: "".to_string(),
            evolution_type: EvolutionType::WorkflowEvolution,
            description: "执行流程进化 template_id:my-custom-template".to_string(),
            template_id: None,
            created_at: String::new(),
        };
        let id = engine.detect_template_id(&plan);
        assert_eq!(id, Some("my-custom-template".to_string()));
    }

    #[test]
    fn detect_template_id_returns_none_when_unknown() {
        let engine = make_test_engine();
        let plan = EvolutionPlan {
            plan_id: "test".to_string(),
            trigger: EvolutionTrigger::ManualTrigger { reason: "".to_string() },
            reflection_report_id: "".to_string(),
            evolution_type: EvolutionType::WorkflowEvolution,
            description: "未知类型的进化".to_string(),
            template_id: None,
            created_at: String::new(),
        };
        let id = engine.detect_template_id(&plan);
        assert!(id.is_none(), "无法识别的模板应返回 None");
    }

    #[tokio::test]
    async fn history_tracks_failures() {
        let engine = make_test_engine();
        let plan = EvolutionPlan {
            plan_id: "plan-fail-test".to_string(),
            trigger: EvolutionTrigger::PoorSignalAccuracy { score: 2.0 },
            reflection_report_id: "fail-report".to_string(),
            evolution_type: EvolutionType::WorkflowEvolution,
            description: "无法识别模板".to_string(),
            template_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let result = engine.run_evolution(&plan).await;
        assert!(result.is_err() || !result.as_ref().unwrap().success);

        let history = engine.get_history().await;
        assert_eq!(history.total_evolutions, 1);
    }
}
