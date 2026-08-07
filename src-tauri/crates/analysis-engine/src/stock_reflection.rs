// SPDX-License-Identifier: AGPL-3.0-only

//! 股票业务反思引擎
//!
//! 将 `trajectory::WorkflowReflectorImpl` 集成到股票业务，提供：
//! - 股票领域特定的反思维度（信号准确性、风险评估、决策质量）
//! - 分析结果到 `WorkflowExecutionRecord` 的映射
//! - 与 `reflection_lesson_validator` 的对接
//! - 股票反思报告生成
//!
//! # 反思流程
//!
//! ```text
//! 分析完成 → 构造 ExecutionRecord → WorkflowReflectorImpl::reflect()
//!     → 质量评分 + 瓶颈识别 + 失败分类
//!     → 生成 StockReflectionReport → 可选触发自我进化
//! ```

use async_trait::async_trait;
use axagent_harness::reflection_types::Reflection;
use axagent_harness::workflow_reflection::{
    BottleneckNode, NodeExecutionSnapshot, NodeFailureAnalysis, WorkflowExecutionRecord,
    WorkflowPattern, WorkflowReflectionMetadata, WorkflowReflector, WorkflowRunStatus,
};
use axagent_harness::workflow_types::{
    EdgeType, EndNode, EndNodeConfig, NodeStatus, Position, RetryConfig, WorkflowEdge,
    WorkflowNode, WorkflowNodeBase,
};
use axagent_trajectory::{ReflectorConfig, WorkflowReflectorImpl};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// ── 股票反思维度权重 ──────────────────────────────────────────

/// 股票反思各维度的默认权重
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockReflectionWeights {
    pub signal_accuracy: f32,
    pub risk_assessment: f32,
    pub decision_quality: f32,
    pub analysis_depth: f32,
    pub execution_efficiency: f32,
}

impl Default for StockReflectionWeights {
    fn default() -> Self {
        Self {
            signal_accuracy: 0.35,
            risk_assessment: 0.25,
            decision_quality: 0.25,
            analysis_depth: 0.1,
            execution_efficiency: 0.05,
        }
    }
}

// ── 股票反思报告 ──────────────────────────────────────────────

/// 股票业务反思报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockReflectionReport {
    /// 执行 ID
    pub execution_id: String,
    /// 工作流 ID
    pub workflow_id: String,
    /// 总体质量分 (1-10)
    pub overall_score: u8,
    /// 各维度得分
    pub dimension_scores: DimensionScores,
    /// 瓶颈节点列表
    pub bottleneck_nodes: Vec<BottleneckNode>,
    /// 错误模式
    pub error_patterns: Vec<String>,
    /// 可复用模式
    pub reusable_patterns: Vec<String>,
    /// 失败节点分析（如果有）
    pub failed_node_analysis: Option<NodeFailureAnalysis>,
    /// 改进建议
    pub improvement_suggestions: Vec<String>,
    /// 是否建议触发自我进化
    pub should_trigger_evolution: bool,
    /// 建议触发进化的原因
    pub evolution_trigger_reason: Option<String>,
    /// 原始反思结果
    #[serde(skip)]
    pub raw_reflection: Option<Reflection>,
}

/// 各维度得分
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DimensionScores {
    /// 信号准确性 (0-10)
    pub signal_accuracy: f32,
    /// 风险评估 (0-10)
    pub risk_assessment: f32,
    /// 决策质量 (0-10)
    pub decision_quality: f32,
    /// 分析深度 (0-10)
    pub analysis_depth: f32,
    /// 执行效率 (0-10)
    pub execution_efficiency: f32,
}

// ── 分析结果输入 ──────────────────────────────────────────────

/// 股票分析结果（反思引擎的输入）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StockAnalysisOutcome {
    /// 分析 ID
    pub analysis_id: String,
    /// 股票代码
    pub stock_code: String,
    /// 执行 ID
    pub execution_id: String,
    /// 分析流水线各步骤执行结果
    pub step_results: Vec<AnalysisStepResult>,
    /// 最终决策（买入/卖出/持有）
    pub decision: String,
    /// 决策置信度 (0.0-1.0)
    pub confidence: f32,
    /// 决策理由
    pub decision_rationale: String,
    /// 信号列表
    pub signals: Vec<String>,
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果有）
    pub error: Option<String>,
    /// 执行耗时 (ms)
    pub duration_ms: u64,
}

/// 单个分析步骤的执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisStepResult {
    pub step_id: String,
    pub step_name: String,
    pub node_type: String,
    pub status: String,
    pub duration_ms: u64,
    pub attempts: u32,
    pub error: Option<String>,
    pub output_summary: Option<String>,
}

// ── 股票反思引擎 ──────────────────────────────────────────────

/// 股票业务反思引擎
///
/// 封装 `WorkflowReflectorImpl`，提供股票领域特定的：
/// - 执行记录构造
/// - 维度化质量评分
/// - 进化触发判定
pub struct StockReflectionEngine {
    reflector: WorkflowReflectorImpl,
    weights: StockReflectionWeights,
    /// 历史反思报告缓存
    report_cache: RwLock<Vec<StockReflectionReport>>,
    /// 触发自我进化的最低质量分阈值
    pub evolution_trigger_threshold: u8,
    /// 触发自我进化的最小连续低质量次数
    pub evolution_trigger_min_low_score_count: usize,
}

impl StockReflectionEngine {
    /// 创建股票反思引擎
    pub fn new() -> Self {
        Self {
            reflector: WorkflowReflectorImpl::with_defaults(),
            weights: StockReflectionWeights::default(),
            report_cache: RwLock::new(Vec::new()),
            evolution_trigger_threshold: 5,
            evolution_trigger_min_low_score_count: 3,
        }
    }

    /// 使用自定义权重创建
    pub fn with_weights(weights: StockReflectionWeights) -> Self {
        Self { weights, ..Self::new() }
    }

    /// 使用自定义 reflector 配置创建
    pub fn with_config(config: ReflectorConfig) -> Self {
        Self { reflector: WorkflowReflectorImpl::new(config), ..Self::new() }
    }

    /// 将股票分析结果转换为 `WorkflowExecutionRecord`
    ///
    /// 填充完整的工作流元数据：模板 ID、节点连线、时间戳、模板节点快照。
    pub fn build_execution_record(outcome: &StockAnalysisOutcome) -> WorkflowExecutionRecord {
        let workflow_id = format!("stock-analysis-{}", outcome.stock_code);
        let template_id = detect_workflow_template_id(outcome);

        let status = if outcome.success {
            WorkflowRunStatus::Completed
        } else if outcome.step_results.iter().any(|s| s.status == "completed") {
            WorkflowRunStatus::PartiallyCompleted
        } else {
            WorkflowRunStatus::Failed
        };

        let now_ms = chrono::Utc::now().timestamp_millis();
        let total_duration = outcome.duration_ms as i64;
        let start_offset = if total_duration > 0 {
            now_ms - total_duration
        } else {
            now_ms
        };

        let nodes: Vec<NodeExecutionSnapshot> = outcome
            .step_results
            .iter()
            .enumerate()
            .map(|(idx, step)| {
                let node_status = match step.status.as_str() {
                    "completed" => NodeStatus::Completed,
                    "failed" => NodeStatus::Failed,
                    _ => NodeStatus::Pending,
                };
                let step_start = start_offset + (idx as i64) * (step.duration_ms as i64);
                let step_end = step_start + step.duration_ms as i64;
                NodeExecutionSnapshot {
                    node_id: step.step_id.clone(),
                    node_type: step.node_type.clone(),
                    node_name: Some(step.step_name.clone()),
                    status: node_status,
                    attempts: step.attempts,
                    input: None,
                    output: step
                        .output_summary
                        .as_ref()
                        .map(|s| serde_json::Value::String(s.clone())),
                    execution_time_ms: Some(step.duration_ms),
                    error: step.error.clone(),
                    started_at: step_start,
                    completed_at: Some(step_end),
                    sub_workflow_id: None,
                }
            })
            .collect();

        let edges = build_edges_from_steps(&outcome.step_results);
        let template_nodes = build_template_nodes_from_steps(&outcome.step_results);

        let error_context = outcome.error.as_ref().map(|e| {
            axagent_harness::workflow_types::WorkflowErrorContext::new(
                "stock_analysis".to_string(),
                "股票分析".to_string(),
                "ANALYSIS_ERROR".to_string(),
                e.clone(),
                workflow_id.clone(),
                outcome.execution_id.clone(),
                None,
            )
        });

        WorkflowExecutionRecord {
            workflow_id,
            execution_id: outcome.execution_id.clone(),
            template_id: Some(template_id),
            template_version: Some(1),
            status,
            started_at: start_offset,
            completed_at: Some(now_ms),
            duration_ms: outcome.duration_ms,
            nodes,
            edges,
            template_nodes,
            input: Some(serde_json::json!({
                "stock_code": outcome.stock_code,
                "analysis_id": outcome.analysis_id,
            })),
            output: Some(serde_json::json!({
                "decision": outcome.decision,
                "confidence": outcome.confidence,
                "signals": outcome.signals,
            })),
            error_context,
        }
    }

    /// 执行股票分析反思
    pub async fn reflect(
        &self,
        outcome: &StockAnalysisOutcome,
    ) -> Result<StockReflectionReport, String> {
        let record = Self::build_execution_record(outcome);
        let reflection = self.reflector.reflect(&record).await?;

        let dimension_scores = self.compute_dimension_scores(outcome, &reflection);
        let bottleneck_nodes = self.extract_bottlenecks(&reflection);
        let failed_node_analysis = self.extract_failure_analysis(&reflection);

        let (should_trigger, reason) =
            self.should_trigger_evolution(&reflection, &dimension_scores);

        let report = StockReflectionReport {
            execution_id: outcome.execution_id.clone(),
            workflow_id: record.workflow_id.clone(),
            overall_score: reflection.quality_score,
            dimension_scores,
            improvement_suggestions: self
                .build_improvement_suggestions(&reflection, &bottleneck_nodes),
            bottleneck_nodes,
            error_patterns: reflection.error_patterns.clone(),
            reusable_patterns: reflection.reusable_patterns.clone(),
            failed_node_analysis,
            should_trigger_evolution: should_trigger,
            evolution_trigger_reason: reason,
            raw_reflection: Some(reflection),
        };

        self.cache_report(report.clone()).await;
        Ok(report)
    }

    /// 基于已有 `WorkflowExecutionRecord` 直接反思
    pub async fn reflect_record(
        &self,
        record: &WorkflowExecutionRecord,
    ) -> Result<Reflection, String> {
        self.reflector.reflect(record).await
    }

    /// 批量分析历史记录的模式
    pub async fn aggregate_patterns(
        &self,
        records: &[WorkflowExecutionRecord],
    ) -> Result<Vec<WorkflowPattern>, String> {
        self.reflector.aggregate_patterns(records).await
    }

    /// 获取历史反思报告
    pub async fn get_recent_reports(&self, limit: usize) -> Vec<StockReflectionReport> {
        let cache = self.report_cache.read().await;
        let start = cache.len().saturating_sub(limit);
        cache[start..].to_vec()
    }

    /// 获取底层 reflector 引用
    pub fn reflector(&self) -> &WorkflowReflectorImpl {
        &self.reflector
    }

    // ── 内部方法 ──────────────────────────────────────────────

    async fn cache_report(&self, report: StockReflectionReport) {
        let mut cache = self.report_cache.write().await;
        cache.push(report);
        if cache.len() > 500 {
            let drop = cache.len() - 500;
            cache.drain(0..drop);
        }
    }

    /// 计算股票特定维度的得分
    fn compute_dimension_scores(
        &self,
        outcome: &StockAnalysisOutcome,
        reflection: &Reflection,
    ) -> DimensionScores {
        let w = &self.weights;
        let mut scores = DimensionScores::default();

        // 信号准确性：基于置信度和信号数量
        let signal_count = outcome.signals.len() as f32;
        let signal_base = (outcome.confidence * 10.0).clamp(0.0, 10.0);
        let signal_bonus: f32 = if signal_count >= 3.0 { 1.0 } else { 0.0 };
        scores.signal_accuracy =
            ((signal_base + signal_bonus) * (1.0 + w.signal_accuracy * 0.5)).clamp(0.0, 10.0);

        // 风险评估：基于是否有风险相关步骤完成
        let has_risk_step = outcome
            .step_results
            .iter()
            .any(|s| s.step_id.contains("risk") && s.status == "completed");
        let risk_base = if has_risk_step { 9.0 } else { 4.0 };
        scores.risk_assessment = (risk_base * (1.0 + w.risk_assessment * 0.3)).clamp(0.0, 10.0);

        // 决策质量：基于决策合理性（有明确理由 = 高分）
        let decision_base = if !outcome.decision_rationale.is_empty() {
            (reflection.quality_score as f32 * 0.9).clamp(0.0, 10.0)
        } else {
            3.0
        };
        scores.decision_quality =
            (decision_base * (1.0 + w.decision_quality * 0.3)).clamp(0.0, 10.0);

        // 分析深度：基于完成步骤数
        let completed_steps =
            outcome.step_results.iter().filter(|s| s.status == "completed").count();
        let total_steps = outcome.step_results.len().max(1) as f32;
        let depth_base = completed_steps as f32 / total_steps * 10.0;
        scores.analysis_depth = (depth_base * (1.0 + w.analysis_depth * 0.3)).clamp(0.0, 10.0);

        // 执行效率：基于耗时和重试
        let total_retries: u32 =
            outcome.step_results.iter().map(|s| s.attempts.saturating_sub(1)).sum();
        let retry_penalty = if total_retries > 3 { 2.0 } else { 0.0 };
        let efficiency_base = if outcome.duration_ms < 5000 {
            9.0
        } else if outcome.duration_ms < 15000 {
            7.0
        } else {
            5.0
        };
        scores.execution_efficiency = ((efficiency_base - retry_penalty) as f32
            * (1.0 + w.execution_efficiency * 0.3))
            .clamp(0.0, 10.0);

        scores
    }

    fn extract_bottlenecks(&self, reflection: &Reflection) -> Vec<BottleneckNode> {
        if let Some(metadata) = &reflection.metadata {
            if let Ok(meta) = serde_json::from_value::<WorkflowReflectionMetadata>(metadata.clone())
            {
                return meta.bottleneck_nodes;
            }
        }
        Vec::new()
    }

    fn extract_failure_analysis(&self, reflection: &Reflection) -> Option<NodeFailureAnalysis> {
        if let Some(metadata) = &reflection.metadata {
            if let Ok(meta) = serde_json::from_value::<WorkflowReflectionMetadata>(metadata.clone())
            {
                return meta.failed_node_analysis;
            }
        }
        None
    }

    /// 判定是否应触发自我进化
    fn should_trigger_evolution(
        &self,
        reflection: &Reflection,
        scores: &DimensionScores,
    ) -> (bool, Option<String>) {
        // 条件 1：总体质量分低于阈值
        if reflection.quality_score <= self.evolution_trigger_threshold {
            return (
                true,
                Some(format!(
                    "总体质量分 {} 低于阈值 {}",
                    reflection.quality_score, self.evolution_trigger_threshold
                )),
            );
        }

        // 条件 2：信号准确性或风险评估维度严重不足
        if scores.signal_accuracy < 4.0 {
            return (
                true,
                Some(format!("信号准确性得分 {:.1} 严重不足，需要优化", scores.signal_accuracy)),
            );
        }
        if scores.risk_assessment < 5.0 {
            return (
                true,
                Some(format!("风险评估得分 {:.1} 不达标，需要加强风控", scores.risk_assessment)),
            );
        }

        // 条件 3：有失败节点
        if !reflection.error_patterns.is_empty() {
            let error_count = reflection.error_patterns.len();
            if error_count >= 3 {
                return (true, Some(format!("检测到 {} 个错误模式，建议进化优化", error_count)));
            }
        }

        (false, None)
    }

    fn build_improvement_suggestions(
        &self,
        reflection: &Reflection,
        bottlenecks: &[BottleneckNode],
    ) -> Vec<String> {
        let mut suggestions = reflection.improvement_suggestions.clone();

        for b in bottlenecks {
            match b.reason {
                axagent_harness::workflow_reflection::BottleneckReason::HighLatency => {
                    suggestions.push(format!(
                        "节点 {} 延迟过高({})，考虑优化 prompt 或拆分任务",
                        b.node_id, b.detail
                    ));
                },
                axagent_harness::workflow_reflection::BottleneckReason::HighFailureRate => {
                    suggestions.push(format!(
                        "节点 {} 失败率高，建议检查输入数据质量和错误处理",
                        b.node_id
                    ));
                },
                axagent_harness::workflow_reflection::BottleneckReason::HighRetryCount => {
                    suggestions.push(format!(
                        "节点 {} 重试过多，考虑增加 timeout 或优化 retry 策略",
                        b.node_id
                    ));
                },
                _ => {},
            }
        }

        if suggestions.is_empty() {
            suggestions.push("分析流程运行正常，无需改进".to_string());
        }

        suggestions
    }
}

impl Default for StockReflectionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowReflector for StockReflectionEngine {
    async fn reflect(&self, record: &WorkflowExecutionRecord) -> Result<Reflection, String> {
        self.reflector.reflect(record).await
    }

    async fn reflect_node(
        &self,
        record: &WorkflowExecutionRecord,
        failed_node: &NodeExecutionSnapshot,
    ) -> Result<Reflection, String> {
        self.reflector.reflect_node(record, failed_node).await
    }

    async fn reflect_batch(
        &self,
        records: &[WorkflowExecutionRecord],
    ) -> Result<Vec<Reflection>, String> {
        self.reflector.reflect_batch(records).await
    }

    async fn aggregate_patterns(
        &self,
        records: &[WorkflowExecutionRecord],
    ) -> Result<Vec<WorkflowPattern>, String> {
        self.reflector.aggregate_patterns(records).await
    }

    async fn get_history(
        &self,
        workflow_id: &str,
        limit: usize,
    ) -> Result<Vec<Reflection>, String> {
        self.reflector.get_history(workflow_id, limit).await
    }
}

// ── 辅助函数 ──────────────────────────────────────────────

/// 根据分析结果推断工作流模板 ID
fn detect_workflow_template_id(outcome: &StockAnalysisOutcome) -> String {
    if outcome.stock_code.is_empty() {
        return "stock-analysis-default".to_string();
    }
    let has_technical = outcome.step_results.iter().any(|s| s.step_id.contains("technical"));
    let has_fundamental = outcome
        .step_results
        .iter()
        .any(|s| s.step_id.contains("fundamental") || s.step_id.contains("financial"));
    let has_sentiment = outcome
        .step_results
        .iter()
        .any(|s| s.step_id.contains("sentiment") || s.step_id.contains("news"));

    if has_technical && has_fundamental && has_sentiment {
        "stock-analysis-comprehensive".to_string()
    } else if has_technical {
        "stock-analysis-technical".to_string()
    } else if has_fundamental {
        "stock-analysis-fundamental".to_string()
    } else {
        "stock-analysis-pipeline".to_string()
    }
}

/// 从步骤结果构建节点连线
///
/// 将线性步骤序列转为有序的直接连接边，反映执行顺序
fn build_edges_from_steps(steps: &[AnalysisStepResult]) -> Vec<WorkflowEdge> {
    if steps.len() < 2 {
        return Vec::new();
    }

    steps
        .windows(2)
        .enumerate()
        .map(|(idx, window)| WorkflowEdge {
            id: format!("edge-{}-{}", window[0].step_id, window[1].step_id),
            source: window[0].step_id.clone(),
            source_handle: None,
            target: window[1].step_id.clone(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: Some(format!("step_sequence_{}", idx)),
        })
        .collect()
}

/// 从步骤结果构建模板节点快照
///
/// 每个步骤映射为一个 `EndNode`，用于模板结构的快照记录
fn build_template_nodes_from_steps(steps: &[AnalysisStepResult]) -> Vec<WorkflowNode> {
    steps
        .iter()
        .map(|step| {
            WorkflowNode::End(EndNode {
                base: WorkflowNodeBase {
                    id: step.step_id.clone(),
                    title: step.step_name.clone(),
                    description: Some(format!(
                        "{} 节点 (类型: {})",
                        step.step_name, step.node_type
                    )),
                    position: Position { x: 0.0, y: 0.0 },
                    retry: RetryConfig::default(),
                    timeout: Some(step.duration_ms.max(1000)),
                    enabled: true,
                    parent_id: None,
                    compensation: None,
                    continue_on_fail: false,
                },
                config: EndNodeConfig { output_var: Some(format!("{}_output", step.step_id)) },
            })
        })
        .collect()
}

// ── 测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_execution_record_from_outcome() {
        let outcome = StockAnalysisOutcome {
            analysis_id: "a1".to_string(),
            stock_code: "600519".to_string(),
            execution_id: "e1".to_string(),
            step_results: vec![
                AnalysisStepResult {
                    step_id: "data_fetch".to_string(),
                    step_name: "数据获取".to_string(),
                    node_type: "data_agent".to_string(),
                    status: "completed".to_string(),
                    duration_ms: 1200,
                    attempts: 1,
                    error: None,
                    output_summary: Some("获取成功".to_string()),
                },
                AnalysisStepResult {
                    step_id: "technical_analysis".to_string(),
                    step_name: "技术面分析".to_string(),
                    node_type: "technical_agent".to_string(),
                    status: "completed".to_string(),
                    duration_ms: 2400,
                    attempts: 1,
                    error: None,
                    output_summary: Some("金叉信号".to_string()),
                },
            ],
            decision: "buy".to_string(),
            confidence: 0.75,
            decision_rationale: "技术面金叉，量价配合".to_string(),
            signals: vec!["金叉".to_string(), "放量".to_string()],
            success: true,
            error: None,
            duration_ms: 3600,
        };

        let record = StockReflectionEngine::build_execution_record(&outcome);
        assert_eq!(record.workflow_id, "stock-analysis-600519");
        assert_eq!(record.status, WorkflowRunStatus::Completed);
        assert_eq!(record.nodes.len(), 2);
        assert_eq!(record.nodes[0].status, NodeStatus::Completed);
    }

    #[test]
    fn builds_failed_execution_record() {
        let outcome = StockAnalysisOutcome {
            analysis_id: "a2".to_string(),
            stock_code: "000001".to_string(),
            execution_id: "e2".to_string(),
            step_results: vec![AnalysisStepResult {
                step_id: "data_fetch".to_string(),
                step_name: "数据获取".to_string(),
                node_type: "data_agent".to_string(),
                status: "failed".to_string(),
                duration_ms: 5000,
                attempts: 3,
                error: Some("timeout".to_string()),
                output_summary: None,
            }],
            decision: "hold".to_string(),
            confidence: 0.0,
            decision_rationale: String::new(),
            signals: vec![],
            success: false,
            error: Some("数据获取超时".to_string()),
            duration_ms: 5000,
        };

        let record = StockReflectionEngine::build_execution_record(&outcome);
        assert_eq!(record.status, WorkflowRunStatus::Failed);
        assert_eq!(record.nodes[0].status, NodeStatus::Failed);
        assert!(record.nodes[0].error.is_some());
    }

    #[test]
    fn dimension_scores_reflect_outcome() {
        let engine = StockReflectionEngine::new();
        let outcome = StockAnalysisOutcome {
            analysis_id: "a3".to_string(),
            stock_code: "300750".to_string(),
            execution_id: "e3".to_string(),
            step_results: vec![
                AnalysisStepResult {
                    step_id: "data_fetch".to_string(),
                    step_name: "数据获取".to_string(),
                    node_type: "data_agent".to_string(),
                    status: "completed".to_string(),
                    duration_ms: 1000,
                    attempts: 1,
                    error: None,
                    output_summary: None,
                },
                AnalysisStepResult {
                    step_id: "risk_assessment".to_string(),
                    step_name: "风险评估".to_string(),
                    node_type: "risk_agent".to_string(),
                    status: "completed".to_string(),
                    duration_ms: 800,
                    attempts: 1,
                    error: None,
                    output_summary: None,
                },
            ],
            decision: "buy".to_string(),
            confidence: 0.85,
            decision_rationale: "基本面优良".to_string(),
            signals: vec!["突破".to_string(), "放量".to_string(), "均线多头".to_string()],
            success: true,
            error: None,
            duration_ms: 1800,
        };

        let reflection = Reflection::new("e3".to_string()).with_quality(8, "质量良好".to_string());
        let scores = engine.compute_dimension_scores(&outcome, &reflection);

        assert!(scores.signal_accuracy >= 8.0, "信号分应高");
        assert!(scores.risk_assessment >= 8.0, "风险评估分应高");
        assert!(scores.decision_quality >= 7.0, "决策质量分应高");
        assert!(scores.analysis_depth >= 8.0, "分析深度应高");
    }

    #[test]
    fn detects_evolution_trigger_low_score() {
        let engine = StockReflectionEngine::new();
        let reflection = Reflection::new("e1".to_string()).with_quality(3, "低分".to_string());
        let scores = DimensionScores::default();

        let (should_trigger, reason) = engine.should_trigger_evolution(&reflection, &scores);
        assert!(should_trigger);
        assert!(reason.is_some());
    }

    #[test]
    fn detects_evolution_trigger_poor_signal() {
        let engine = StockReflectionEngine::new();
        let reflection = Reflection::new("e1".to_string()).with_quality(7, "中等".to_string());
        let scores = DimensionScores { signal_accuracy: 3.0, ..Default::default() };

        let (should_trigger, reason) = engine.should_trigger_evolution(&reflection, &scores);
        assert!(should_trigger);
        assert!(reason.unwrap().contains("信号准确性"));
    }

    #[test]
    fn no_trigger_on_good_score() {
        let engine = StockReflectionEngine::new();
        let reflection = Reflection::new("e1".to_string())
            .with_quality(9, "优秀".to_string())
            .with_patterns(vec![], vec!["正常输出".to_string()]);
        let scores = DimensionScores {
            signal_accuracy: 9.0,
            risk_assessment: 9.0,
            decision_quality: 8.0,
            analysis_depth: 8.0,
            execution_efficiency: 9.0,
        };

        let (should_trigger, _) = engine.should_trigger_evolution(&reflection, &scores);
        assert!(!should_trigger);
    }

    #[test]
    fn improvement_suggestions_generated() {
        let engine = StockReflectionEngine::new();
        let reflection = Reflection::new("e1".to_string());
        let bottlenecks = vec![BottleneckNode {
            node_id: "data_fetch".to_string(),
            node_type: "data_agent".to_string(),
            reason: axagent_harness::workflow_reflection::BottleneckReason::HighLatency,
            impact_score: 0.8,
            detail: "平均耗时 6000ms".to_string(),
        }];

        let suggestions = engine.build_improvement_suggestions(&reflection, &bottlenecks);
        assert!(!suggestions.is_empty());
        assert!(suggestions[0].contains("data_fetch"));
    }

    #[tokio::test]
    async fn reflect_produces_report() {
        let engine = StockReflectionEngine::new();
        let outcome = StockAnalysisOutcome {
            analysis_id: "a1".to_string(),
            stock_code: "600519".to_string(),
            execution_id: "e1".to_string(),
            step_results: vec![AnalysisStepResult {
                step_id: "data_fetch".to_string(),
                step_name: "数据获取".to_string(),
                node_type: "data_agent".to_string(),
                status: "completed".to_string(),
                duration_ms: 1000,
                attempts: 1,
                error: None,
                output_summary: Some("OK".to_string()),
            }],
            decision: "buy".to_string(),
            confidence: 0.7,
            decision_rationale: "技术面良好".to_string(),
            signals: vec!["金叉".to_string()],
            success: true,
            error: None,
            duration_ms: 2000,
        };

        let report = engine.reflect(&outcome).await.expect("反思应成功");
        assert_eq!(report.execution_id, "e1");
        assert!(report.overall_score >= 6);
        assert!(!report.improvement_suggestions.is_empty());
    }

    #[tokio::test]
    async fn caches_reports() {
        let engine = StockReflectionEngine::new();
        let outcome = StockAnalysisOutcome {
            analysis_id: "a1".to_string(),
            stock_code: "600519".to_string(),
            execution_id: "cache-test".to_string(),
            step_results: vec![],
            decision: "hold".to_string(),
            confidence: 0.5,
            decision_rationale: String::new(),
            signals: vec![],
            success: true,
            error: None,
            duration_ms: 500,
        };

        engine.reflect(&outcome).await.unwrap();
        let reports = engine.get_recent_reports(10).await;
        assert!(!reports.is_empty());
        assert_eq!(reports.last().unwrap().execution_id, "cache-test");
    }
}
