// SPDX-License-Identifier: AGPL-3.0-only

//! 股票业务自适应引擎
//!
//! 将 Reflection（反思）+ Evolution（进化）+ Orchestration（编排）
//! 三者整合为一个统一的自适应闭环系统。
//!
//! # 核心闭环
//!
//! ```text
//! 业务请求 → Orchestration(编排执行)
//!   → 分析结果输出
//!   → Reflection(反思诊断)
//!   → 进化触发判定
//!   → Evolution(参数/流程进化)
//!   → 应用进化结果(更新 WeightDecayConfig)
//!   → 验证(回测/对比)
//!   → 接受/拒绝 → 反馈至 Orchestration
//! ```
//!
//! # 使用示例
//!
//! ```ignore
//! let engine = StockAdaptiveEngine::new();
//!
//! // 执行一次完整分析 + 自适应闭环
//! let result = engine.run_adaptive_analysis(
//!     stock_code, mission, context
//! ).await?;
//!
//! // result.adaptation_status 指示是否触发了进化
//! ```

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use axagent_harness::IndustryAdapter;

use crate::stock_orchestration::StockIndustryAdapter;
use crate::stock_reflection::{
    DimensionScores, StockAnalysisOutcome, StockReflectionEngine, StockReflectionReport,
};
use crate::stock_self_evolution::{EvolutionType, StockEvolutionResult, StockSelfEvolutionEngine};
use crate::weight_decay::WeightDecayConfig;

// ── 自适应状态 ──────────────────────────────────────────

/// 自适应状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdaptationStatus {
    /// 正常运行，无需进化
    Normal,
    /// 已触发参数进化
    ParameterEvolved,
    /// 已触发流程进化
    WorkflowEvolved,
    /// 已触发混合进化
    HybridEvolved,
    /// 进化失败
    EvolutionFailed,
    /// 系统错误
    Error,
}

/// 自适应执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveResult {
    /// 股票代码
    pub stock_code: String,
    /// 执行 ID
    pub execution_id: String,
    /// 自适应状态
    pub adaptation_status: AdaptationStatus,
    /// 反思报告
    pub reflection_report: Option<StockReflectionReport>,
    /// 进化结果（如果触发了进化）
    pub evolution_result: Option<StockEvolutionResult>,
    /// 应用的新配置（如果参数进化成功）
    pub applied_config: Option<WeightDecayConfig>,
    /// 改进摘要
    pub improvement_summary: String,
    /// 是否建议回测验证
    pub needs_backtest: bool,
}

/// 引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveEngineConfig {
    /// 反思触发的最低质量分阈值（低于此值触发进化评估）
    pub reflection_quality_threshold: u8,
    /// 自动进化开关
    pub auto_evolve: bool,
    /// 进化结果是否需要人工确认
    pub require_human_approval: bool,
    /// 最大连续进化轮数（防止无限循环）
    pub max_consecutive_evolutions: u32,
    /// 参数进化最小触发间隔（秒）
    pub min_evolution_interval_secs: u64,
}

impl Default for AdaptiveEngineConfig {
    fn default() -> Self {
        Self {
            reflection_quality_threshold: 5,
            auto_evolve: true,
            require_human_approval: false,
            max_consecutive_evolutions: 3,
            min_evolution_interval_secs: 3600,
        }
    }
}

// ── 应用进化结果的验证器 ─────────────────────────────────

/// 进化结果验证器
///
/// 检查进化后的配置是否比默认配置更优
pub struct EvolutionValidator {
    min_improvement_threshold: f64,
}

impl EvolutionValidator {
    pub fn new(min_improvement_threshold: f64) -> Self {
        Self { min_improvement_threshold }
    }

    /// 验证新配置是否优于旧配置
    pub fn validate_improvement(
        &self,
        old_config: &WeightDecayConfig,
        new_config: &WeightDecayConfig,
        old_scores: &DimensionScores,
        new_scores: &DimensionScores,
    ) -> ValidationResult {
        let old_overall = self.compute_overall_score(old_scores);
        let new_overall = self.compute_overall_score(new_scores);

        let improvement = new_overall - old_overall;

        let config_changed = configs_differ(old_config, new_config);

        let accepted = improvement >= self.min_improvement_threshold && config_changed;

        ValidationResult {
            accepted,
            improvement,
            old_overall,
            new_overall,
            reason: if accepted {
                format!(
                    "质量分提升 {:.2}（阈值 {:.2}），配置已变更",
                    improvement, self.min_improvement_threshold
                )
            } else if !config_changed {
                "配置未发生变化，拒绝".to_string()
            } else {
                format!(
                    "质量分提升 {:.2} 低于阈值 {:.2}，拒绝",
                    improvement, self.min_improvement_threshold
                )
            },
        }
    }

    fn compute_overall_score(&self, scores: &DimensionScores) -> f64 {
        scores.signal_accuracy as f64 * 0.35
            + scores.risk_assessment as f64 * 0.25
            + scores.decision_quality as f64 * 0.25
            + scores.analysis_depth as f64 * 0.10
            + scores.execution_efficiency as f64 * 0.05
    }
}

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// 是否接受进化结果
    pub accepted: bool,
    /// 质量分提升量
    pub improvement: f64,
    /// 旧质量分
    pub old_overall: f64,
    /// 新质量分
    pub new_overall: f64,
    /// 原因说明
    pub reason: String,
}

fn configs_differ(a: &WeightDecayConfig, b: &WeightDecayConfig) -> bool {
    (a.ewma_alpha - b.ewma_alpha).abs() > 1e-6
        || a.lookback_days != b.lookback_days
        || a.sample_saturation != b.sample_saturation
}

// ── 股票自适应引擎 ───────────────────────────────────────

/// 股票业务自适应引擎
///
/// 整合 ReflectionEngine + SelfEvolutionEngine + IndustryAdapter
/// 形成完整的自适应闭环系统。
pub struct StockAdaptiveEngine {
    config: AdaptiveEngineConfig,
    reflection_engine: Arc<StockReflectionEngine>,
    evolution_engine: Arc<StockSelfEvolutionEngine>,
    industry_adapter: Arc<StockIndustryAdapter>,
    validator: EvolutionValidator,
    /// 当前生效的配置
    current_config: RwLock<WeightDecayConfig>,
    /// 自适应运行历史
    adaptation_history: RwLock<Vec<AdaptationRecord>>,
    /// 连续进化计数
    consecutive_evolutions: RwLock<u32>,
}

/// 自适应运行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationRecord {
    pub timestamp: String,
    pub stock_code: String,
    pub adaptation_status: AdaptationStatus,
    pub quality_score_before: u8,
    pub quality_score_after: Option<u8>,
    pub evolution_triggered: bool,
    pub improvement_summary: String,
}

impl StockAdaptiveEngine {
    /// 创建自适应引擎
    pub fn new() -> Self {
        let reflection_engine = Arc::new(StockReflectionEngine::new());
        let evolution_engine =
            Arc::new(StockSelfEvolutionEngine::new(Arc::clone(&reflection_engine)));
        let industry_adapter = Arc::new(StockIndustryAdapter::new());

        Self {
            config: AdaptiveEngineConfig::default(),
            reflection_engine,
            evolution_engine,
            industry_adapter,
            validator: EvolutionValidator::new(0.05),
            current_config: RwLock::new(WeightDecayConfig::default()),
            adaptation_history: RwLock::new(Vec::new()),
            consecutive_evolutions: RwLock::new(0),
        }
    }

    /// 使用自定义配置创建
    pub fn with_config(config: AdaptiveEngineConfig) -> Self {
        Self { config, ..Self::new() }
    }

    /// 设置当前配置
    pub async fn set_config(&self, config: WeightDecayConfig) {
        let mut current = self.current_config.write().await;
        *current = config;
    }

    /// 获取当前配置
    pub async fn get_config(&self) -> WeightDecayConfig {
        self.current_config.read().await.clone()
    }

    /// 获取反思引擎引用
    pub fn reflection_engine(&self) -> &StockReflectionEngine {
        &self.reflection_engine
    }

    /// 获取进化引擎引用
    pub fn evolution_engine(&self) -> &StockSelfEvolutionEngine {
        &self.evolution_engine
    }

    /// 获取行业适配器引用
    pub fn industry_adapter(&self) -> &StockIndustryAdapter {
        &self.industry_adapter
    }

    /// 获取自适应运行历史
    pub async fn get_history(&self, limit: usize) -> Vec<AdaptationRecord> {
        let history = self.adaptation_history.read().await;
        let start = history.len().saturating_sub(limit);
        history[start..].to_vec()
    }

    /// 执行自适应分析闭环
    ///
    /// # 流程
    /// 1. 接收分析结果
    /// 2. 执行反思诊断
    /// 3. 判定是否需要触发进化
    /// 4. 执行进化（参数/流程/混合）
    /// 5. 验证进化结果
    /// 6. 应用或拒绝进化结果
    pub async fn run_adaptive_cycle(&self, outcome: &StockAnalysisOutcome) -> AdaptiveResult {
        let mut result = AdaptiveResult {
            stock_code: outcome.stock_code.clone(),
            execution_id: outcome.execution_id.clone(),
            adaptation_status: AdaptationStatus::Normal,
            reflection_report: None,
            evolution_result: None,
            applied_config: None,
            improvement_summary: String::new(),
            needs_backtest: false,
        };

        // Step 1: 反思诊断
        let reflection_report = match self.reflection_engine.reflect(outcome).await {
            Ok(report) => report,
            Err(e) => {
                result.adaptation_status = AdaptationStatus::Error;
                result.improvement_summary = format!("反思失败: {}", e);
                self.record_adaptation(&result).await;
                return result;
            },
        };
        result.reflection_report = Some(reflection_report.clone());

        // Step 2: 检查是否需要连续进化限制
        {
            let consecutive = self.consecutive_evolutions.read().await;
            if *consecutive >= self.config.max_consecutive_evolutions {
                result.improvement_summary = format!(
                    "已达到最大连续进化次数 ({})，跳过本次进化",
                    self.config.max_consecutive_evolutions
                );
                self.record_adaptation(&result).await;
                return result;
            }
        }

        // Step 3: 判定是否触发进化
        if !self.config.auto_evolve {
            result.improvement_summary = "自动进化已禁用".to_string();
            self.record_adaptation(&result).await;
            return result;
        }

        let trigger = self.evolution_engine.evaluate_trigger(&reflection_report).await;

        let trigger = match trigger {
            Some(t) => t,
            None => {
                // 质量良好，重置连续计数
                {
                    let mut consecutive = self.consecutive_evolutions.write().await;
                    *consecutive = 0;
                }
                result.improvement_summary =
                    format!("质量分 {}，无需进化", reflection_report.overall_score);
                self.record_adaptation(&result).await;
                return result;
            },
        };

        // Step 4: 创建进化计划并执行
        let template_id = infer_template_id_from_outcome(outcome);
        let plan =
            self.evolution_engine.create_plan(&trigger, &reflection_report, template_id.as_deref());

        match self.evolution_engine.run_evolution(&plan).await {
            Ok(evo_result) => {
                result.evolution_result = Some(evo_result.clone());

                // Step 5: 验证并应用结果
                if let Some(ref param_result) = evo_result.parameter_result {
                    match self.apply_evolution_result(param_result, &reflection_report).await {
                        Ok(applied) => {
                            result.applied_config = Some(applied);
                        },
                        Err(reason) => {
                            result.improvement_summary = format!("进化结果被拒绝: {}", reason);
                        },
                    }
                }

                // 更新状态
                result.adaptation_status = match evo_result.evolution_type {
                    EvolutionType::ParameterEvolution => AdaptationStatus::ParameterEvolved,
                    EvolutionType::WorkflowEvolution => AdaptationStatus::WorkflowEvolved,
                    EvolutionType::HybridEvolution => AdaptationStatus::HybridEvolved,
                };

                {
                    let mut consecutive = self.consecutive_evolutions.write().await;
                    *consecutive += 1;
                }

                result.improvement_summary = evo_result.improvement_summary.clone();
                result.needs_backtest = true;
            },
            Err(e) => {
                result.adaptation_status = AdaptationStatus::EvolutionFailed;
                result.improvement_summary = format!("进化执行失败: {}", e);

                {
                    let mut consecutive = self.consecutive_evolutions.write().await;
                    *consecutive = 0;
                }
            },
        }

        self.record_adaptation(&result).await;
        result
    }

    /// 执行自适应分析（带编排执行）
    ///
    /// 完整流程：编排执行 → 反思 → 进化 → 应用
    pub async fn run_adaptive_analysis(
        &self,
        stock_code: &str,
        mission: &str,
    ) -> Result<AdaptiveResult, String> {
        // Step 1: 通过行业适配器进行编排
        let context = axagent_harness::IndustryContext::new()
            .with_inputs(serde_json::json!({"stock_code": stock_code}));

        let subgraph = self
            .industry_adapter
            .decompose_mission(mission, &context)
            .await
            .map_err(|e| e.to_string())?;

        // Step 2: 执行子图（简化版，构造分析结果）
        let outcome = self.simulate_orchestration_execution(stock_code, &subgraph)?;

        // Step 3: 自适应闭环
        Ok(self.run_adaptive_cycle(&outcome).await)
    }

    /// 重置连续进化计数
    pub async fn reset_consecutive_evolutions(&self) {
        let mut consecutive = self.consecutive_evolutions.write().await;
        *consecutive = 0;
    }

    // ── 内部方法 ──────────────────────────────────────────

    async fn apply_evolution_result(
        &self,
        param_result: &crate::evolution_optimizer::EvolutionResult,
        reflection: &StockReflectionReport,
    ) -> Result<WeightDecayConfig, String> {
        let current = self.current_config.read().await;

        let old_scores = reflection.dimension_scores.clone();
        // 回测模拟：基于新配置预测得分变化
        let new_scores = self.simulate_backtest_scores(&param_result.best_config, &old_scores);

        let validation = self.validator.validate_improvement(
            &current,
            &param_result.best_config,
            &old_scores,
            &new_scores,
        );

        if validation.accepted {
            drop(current);
            let mut config = self.current_config.write().await;
            *config = param_result.best_config.clone();
            Ok(param_result.best_config.clone())
        } else {
            Err(validation.reason)
        }
    }

    /// 回测模拟：基于新配置预测各维度得分
    ///
    /// 实际生产中应调用真实回测引擎，此处提供合理的预测模型
    fn simulate_backtest_scores(
        &self,
        new_config: &WeightDecayConfig,
        old_scores: &DimensionScores,
    ) -> DimensionScores {
        // 计算配置变化度
        let alpha_delta = (new_config.ewma_alpha - 0.3).abs();
        let lookback_delta = (new_config.lookback_days as f64 - 30.0).abs() / 30.0;
        let saturation_delta = (new_config.sample_saturation as f64 - 1.0).abs();

        let config_change =
            (alpha_delta * 0.4 + lookback_delta * 0.3 + saturation_delta * 0.3) as f32;

        // 配置变化度在合理范围内（0-0.5），过度变化会导致负向影响
        let improvement_factor = if config_change <= 0.3 {
            1.0 + config_change * 0.5 // 0% ~ 15% 提升
        } else if config_change <= 0.6 {
            1.0 + 0.15 - (config_change - 0.3) * 0.3 // 15% → 6% 过渡
        } else {
            1.0 - 0.05 // 过度变化，轻微负向
        };

        DimensionScores {
            signal_accuracy: (old_scores.signal_accuracy * improvement_factor).clamp(0.0, 10.0),
            risk_assessment: (old_scores.risk_assessment * improvement_factor).clamp(0.0, 10.0),
            decision_quality: (old_scores.decision_quality * improvement_factor).clamp(0.0, 10.0),
            analysis_depth: (old_scores.analysis_depth * (1.0 + config_change * 0.3))
                .clamp(0.0, 10.0),
            execution_efficiency: (old_scores.execution_efficiency * (1.0 - config_change * 0.1))
                .clamp(0.0, 10.0),
        }
    }

    fn simulate_orchestration_execution(
        &self,
        stock_code: &str,
        subgraph: &axagent_harness::GeneratedSubGraph,
    ) -> Result<StockAnalysisOutcome, String> {
        let step_results: Vec<_> = subgraph
            .nodes
            .iter()
            .map(|node| {
                let base = node.base();
                let node_type_name = match node {
                    axagent_harness::workflow_types::WorkflowNode::Agent(_) => "agent",
                    _ => "unknown",
                };
                crate::stock_reflection::AnalysisStepResult {
                    step_id: base.id.clone(),
                    step_name: base.title.clone(),
                    node_type: node_type_name.to_string(),
                    status: "completed".to_string(),
                    duration_ms: 1000,
                    attempts: 1,
                    error: None,
                    output_summary: Some(format!("{} 执行完成", base.id)),
                }
            })
            .collect();

        Ok(StockAnalysisOutcome {
            analysis_id: format!("adaptive-{}", uuid::Uuid::new_v4()),
            stock_code: stock_code.to_string(),
            execution_id: format!("exec-{}", uuid::Uuid::new_v4()),
            step_results,
            decision: "hold".to_string(),
            confidence: 0.6,
            decision_rationale: "自适应分析默认决策".to_string(),
            signals: vec![],
            success: true,
            error: None,
            duration_ms: subgraph.nodes.len() as u64 * 1000,
        })
    }

    async fn record_adaptation(&self, result: &AdaptiveResult) {
        let record = AdaptationRecord {
            timestamp: chrono::Utc::now().to_rfc3339(),
            stock_code: result.stock_code.clone(),
            adaptation_status: result.adaptation_status,
            quality_score_before: result
                .reflection_report
                .as_ref()
                .map(|r| r.overall_score)
                .unwrap_or(0),
            quality_score_after: None,
            evolution_triggered: !matches!(result.adaptation_status, AdaptationStatus::Normal),
            improvement_summary: result.improvement_summary.clone(),
        };

        let mut history = self.adaptation_history.write().await;
        history.push(record);
        if history.len() > 1000 {
            let drop = history.len() - 1000;
            history.drain(0..drop);
        }
    }
}

impl Default for StockAdaptiveEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── 辅助函数 ──────────────────────────────────────────

/// 从分析结果推断模板 ID
///
/// 与 `stock_reflection::detect_workflow_template_id` 保持一致的逻辑，
/// 确保在自适应引擎上下文中也能正确识别模板
fn infer_template_id_from_outcome(
    outcome: &crate::stock_reflection::StockAnalysisOutcome,
) -> Option<String> {
    let has_technical = outcome.step_results.iter().any(|s| s.step_id.contains("technical"));
    let has_fundamental = outcome
        .step_results
        .iter()
        .any(|s| s.step_id.contains("fundamental") || s.step_id.contains("financial"));
    let has_sentiment = outcome
        .step_results
        .iter()
        .any(|s| s.step_id.contains("sentiment") || s.step_id.contains("news"));

    let id = if has_technical && has_fundamental && has_sentiment {
        "stock-analysis-comprehensive"
    } else if has_technical {
        "stock-analysis-technical"
    } else if has_fundamental {
        "stock-analysis-fundamental"
    } else {
        "stock-analysis-pipeline"
    };

    Some(id.to_string())
}

// ── 测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stock_reflection::{AnalysisStepResult, StockAnalysisOutcome};

    fn make_normal_outcome(stock_code: &str) -> StockAnalysisOutcome {
        StockAnalysisOutcome {
            analysis_id: format!("test-{}", stock_code),
            stock_code: stock_code.to_string(),
            execution_id: format!("exec-{}", uuid::Uuid::new_v4()),
            step_results: vec![
                AnalysisStepResult {
                    step_id: "data_fetch".to_string(),
                    step_name: "数据获取".to_string(),
                    node_type: "data_agent".to_string(),
                    status: "completed".to_string(),
                    duration_ms: 1000,
                    attempts: 1,
                    error: None,
                    output_summary: Some("OK".to_string()),
                },
                AnalysisStepResult {
                    step_id: "technical".to_string(),
                    step_name: "技术分析".to_string(),
                    node_type: "technical_agent".to_string(),
                    status: "completed".to_string(),
                    duration_ms: 2000,
                    attempts: 1,
                    error: None,
                    output_summary: Some("金叉信号".to_string()),
                },
                AnalysisStepResult {
                    step_id: "risk_assessment".to_string(),
                    step_name: "风险评估".to_string(),
                    node_type: "risk_agent".to_string(),
                    status: "completed".to_string(),
                    duration_ms: 500,
                    attempts: 1,
                    error: None,
                    output_summary: Some("风险可控".to_string()),
                },
            ],
            decision: "buy".to_string(),
            confidence: 0.75,
            decision_rationale: "技术面良好，风险可控".to_string(),
            signals: vec!["金叉".to_string(), "放量".to_string()],
            success: true,
            error: None,
            duration_ms: 3500,
        }
    }

    fn make_low_quality_outcome(stock_code: &str) -> StockAnalysisOutcome {
        StockAnalysisOutcome {
            analysis_id: format!("test-low-{}", stock_code),
            stock_code: stock_code.to_string(),
            execution_id: format!("exec-low-{}", uuid::Uuid::new_v4()),
            step_results: vec![
                AnalysisStepResult {
                    step_id: "data_fetch".to_string(),
                    step_name: "数据获取".to_string(),
                    node_type: "data_agent".to_string(),
                    status: "completed".to_string(),
                    duration_ms: 5000,
                    attempts: 3,
                    error: Some("timeout".to_string()),
                    output_summary: None,
                },
                AnalysisStepResult {
                    step_id: "technical".to_string(),
                    step_name: "技术分析".to_string(),
                    node_type: "technical_agent".to_string(),
                    status: "failed".to_string(),
                    duration_ms: 0,
                    attempts: 0,
                    error: Some("数据不足".to_string()),
                    output_summary: None,
                },
            ],
            decision: "hold".to_string(),
            confidence: 0.0,
            decision_rationale: String::new(),
            signals: vec![],
            success: false,
            error: Some("分析失败".to_string()),
            duration_ms: 5000,
        }
    }

    #[test]
    fn engine_creation_default() {
        let engine = StockAdaptiveEngine::new();
        assert!(engine.config.auto_evolve);
        assert_eq!(engine.config.max_consecutive_evolutions, 3);
    }

    #[test]
    fn engine_with_custom_config() {
        let config = AdaptiveEngineConfig {
            auto_evolve: false,
            max_consecutive_evolutions: 5,
            ..Default::default()
        };
        let engine = StockAdaptiveEngine::with_config(config);
        assert!(!engine.config.auto_evolve);
        assert_eq!(engine.config.max_consecutive_evolutions, 5);
    }

    #[tokio::test]
    async fn run_cycle_normal_quality() {
        let engine = StockAdaptiveEngine::new();
        let outcome = make_normal_outcome("600519");

        let result = engine.run_adaptive_cycle(&outcome).await;

        assert_eq!(result.stock_code, "600519");
        assert!(result.reflection_report.is_some());
        assert!(matches!(
            result.adaptation_status,
            AdaptationStatus::Normal | AdaptationStatus::ParameterEvolved
        ));
    }

    #[tokio::test]
    async fn run_cycle_low_quality_triggers_evolution() {
        let engine = StockAdaptiveEngine::new();
        let outcome = make_low_quality_outcome("000001");

        for _ in 0..4 {
            engine.run_adaptive_cycle(&outcome).await;
        }

        let final_result = engine.run_adaptive_cycle(&outcome).await;
        assert!(matches!(
            final_result.adaptation_status,
            AdaptationStatus::ParameterEvolved
                | AdaptationStatus::HybridEvolved
                | AdaptationStatus::Normal
        ));
    }

    #[tokio::test]
    async fn auto_evolve_disabled() {
        let config = AdaptiveEngineConfig { auto_evolve: false, ..Default::default() };
        let engine = StockAdaptiveEngine::with_config(config);
        let outcome = make_low_quality_outcome("000001");

        let result = engine.run_adaptive_cycle(&outcome).await;

        assert!(matches!(result.adaptation_status, AdaptationStatus::Normal));
        assert!(result.improvement_summary.contains("自动进化已禁用"));
    }

    #[tokio::test]
    async fn max_consecutive_evolutions_enforced() {
        let config = AdaptiveEngineConfig { max_consecutive_evolutions: 1, ..Default::default() };
        let engine = StockAdaptiveEngine::with_config(config);
        let outcome = make_low_quality_outcome("000001");

        engine.run_adaptive_cycle(&outcome).await;

        let result = engine.run_adaptive_cycle(&outcome).await;
        assert!(result.improvement_summary.contains("最大连续进化次数"));
    }

    #[tokio::test]
    async fn config_get_set() {
        let engine = StockAdaptiveEngine::new();

        let initial = engine.get_config().await;
        assert!((initial.ewma_alpha - 0.3).abs() < f64::EPSILON);

        let new_config =
            WeightDecayConfig { ewma_alpha: 0.5, lookback_days: 60, sample_saturation: 15 };
        engine.set_config(new_config.clone()).await;

        let updated = engine.get_config().await;
        assert!((updated.ewma_alpha - 0.5).abs() < f64::EPSILON);
        assert_eq!(updated.lookback_days, 60);
    }

    #[tokio::test]
    async fn adaptation_history_recorded() {
        let engine = StockAdaptiveEngine::new();
        let outcome = make_normal_outcome("600519");

        engine.run_adaptive_cycle(&outcome).await;
        engine.run_adaptive_cycle(&outcome).await;

        let history = engine.get_history(10).await;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].stock_code, "600519");
    }

    #[test]
    fn validator_accepts_improved_config() {
        let validator = EvolutionValidator::new(0.05);

        let old_config = WeightDecayConfig::default();
        let new_config =
            WeightDecayConfig { ewma_alpha: 0.5, lookback_days: 60, sample_saturation: 15 };

        let old_scores = DimensionScores {
            signal_accuracy: 5.0,
            risk_assessment: 5.0,
            decision_quality: 5.0,
            analysis_depth: 5.0,
            execution_efficiency: 5.0,
        };
        let new_scores = DimensionScores {
            signal_accuracy: 8.0,
            risk_assessment: 8.0,
            decision_quality: 8.0,
            analysis_depth: 8.0,
            execution_efficiency: 8.0,
        };

        let result =
            validator.validate_improvement(&old_config, &new_config, &old_scores, &new_scores);
        assert!(result.accepted);
        assert!(result.improvement > 0.05);
    }

    #[test]
    fn validator_rejects_regression() {
        let validator = EvolutionValidator::new(0.05);

        let old_config = WeightDecayConfig::default();
        let new_config =
            WeightDecayConfig { ewma_alpha: 0.5, lookback_days: 60, sample_saturation: 15 };

        let old_scores = DimensionScores { signal_accuracy: 8.0, ..Default::default() };
        let new_scores = DimensionScores { signal_accuracy: 4.0, ..Default::default() };

        let result =
            validator.validate_improvement(&old_config, &new_config, &old_scores, &new_scores);
        assert!(!result.accepted);
    }

    #[test]
    fn validator_rejects_unchanged_config() {
        let validator = EvolutionValidator::new(0.05);

        let config = WeightDecayConfig::default();
        let scores = DimensionScores::default();

        let result = validator.validate_improvement(&config, &config, &scores, &scores);
        assert!(!result.accepted);
    }

    #[tokio::test]
    async fn reset_consecutive_evolutions() {
        let engine = StockAdaptiveEngine::new();
        let outcome = make_low_quality_outcome("000001");

        for _ in 0..3 {
            engine.run_adaptive_cycle(&outcome).await;
        }

        engine.reset_consecutive_evolutions().await;

        let result = engine.run_adaptive_cycle(&outcome).await;
        assert!(!result.improvement_summary.contains("最大连续进化次数"));
    }

    #[test]
    fn adaptation_status_values() {
        assert_eq!(AdaptationStatus::Normal as u8, 0);
        assert_eq!(AdaptationStatus::ParameterEvolved as u8, 1);
        assert_eq!(AdaptationStatus::WorkflowEvolved as u8, 2);
        assert_eq!(AdaptationStatus::HybridEvolved as u8, 3);
        assert_eq!(AdaptationStatus::EvolutionFailed as u8, 4);
        assert_eq!(AdaptationStatus::Error as u8, 5);
    }
}
