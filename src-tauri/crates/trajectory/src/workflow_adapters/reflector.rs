// SPDX-License-Identifier: AGPL-3.0-only

//! `WorkflowReflectorImpl`:基于启发式规则的工作流反思器实现。
//!
//! MVP 策略:
//! - 不依赖 LLM,基于 `WorkflowExecutionRecord.nodes` 统计计算质量分
//! - 内存保留历史反思,按 `workflow_id` 索引,limit 由调用方决定
//! - 工作流专有数据(瓶颈节点、节点级模式等)写入 `Reflection::metadata`
//!
//! 后续增强:注入 LLM provider 做语义级反思、跨模板模式挖掘等。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use axagent_harness::reflection_types::{QualityMetrics, Reflection};
use axagent_harness::workflow_reflection::{
    BottleneckNode, BottleneckReason, FailureCategory, NodeExecutionSnapshot, NodeFailureAnalysis,
    ProposedChange, WorkflowExecutionRecord, WorkflowExecutionRecord as WfRecord, WorkflowPattern,
    WorkflowReflectionMetadata, WorkflowReflector, WorkflowRunStatus,
};
use axagent_harness::workflow_types::NodeStatus;

use crate::storage::TrajectoryStorage;

/// 启发式阈值(可由 wiring 层覆盖)。
#[derive(Debug, Clone)]
pub struct ReflectorConfig {
    /// 节点平均耗时超过此阈值(ms)视为高延迟瓶颈。
    pub high_latency_ms: u64,
    /// 节点失败率超过此阈值(0.0-1.0)视为高失败率瓶颈。
    pub high_failure_rate: f32,
    /// 节点重试次数超过此阈值视为高重试瓶颈。
    pub high_retry_count: u32,
    /// 内存保留的最大历史反思数(按 workflow_id)。
    pub max_history_per_workflow: usize,
}

impl Default for ReflectorConfig {
    fn default() -> Self {
        Self {
            high_latency_ms: 5_000,
            high_failure_rate: 0.3,
            high_retry_count: 2,
            max_history_per_workflow: 100,
        }
    }
}

/// `WorkflowReflector` 的 trajectory 实现。
///
/// 内部用 `RwLock<HashMap<workflow_id, Vec<Reflection>>>` 保留内存历史,
/// 可选 `storage: Option<Arc<TrajectoryStorage>>` 在每次反思后同步落库到
/// `trajectory_workflow_reflections` 表(优化 3)。`storage = None` 时退化为
/// 纯内存模式(单测 / 离线场景)。
pub struct WorkflowReflectorImpl {
    config: ReflectorConfig,
    history: RwLock<HashMap<String, Vec<Reflection>>>,
    storage: Option<Arc<TrajectoryStorage>>,
}

impl WorkflowReflectorImpl {
    pub fn new(config: ReflectorConfig) -> Self {
        Self { config, history: RwLock::new(HashMap::new()), storage: None }
    }

    /// 默认配置构造(纯内存模式,不落库)。
    pub fn with_defaults() -> Self {
        Self::new(ReflectorConfig::default())
    }

    /// 注入 `TrajectoryStorage` 启用持久化(优化 3)。
    ///
    /// 注入后,`reflect()` / `reflect_node()` 在写入内存历史后,会异步把反思
    /// 落库到 `trajectory_workflow_reflections` 表。失败仅记录日志,不阻塞主流程
    /// (反思落库是 best-effort,失败不应影响工作流执行)。
    pub fn with_storage(config: ReflectorConfig, storage: Arc<TrajectoryStorage>) -> Self {
        Self { config, history: RwLock::new(HashMap::new()), storage: Some(storage) }
    }

    /// 把反思写入内存历史(按 workflow_id 索引,trim 到 max_history)。
    async fn store_history(&self, workflow_id: &str, reflection: Reflection) {
        let mut guard = self.history.write().await;
        let vec = guard.entry(workflow_id.to_string()).or_default();
        vec.push(reflection);
        if vec.len() > self.config.max_history_per_workflow {
            let drop_count = vec.len() - self.config.max_history_per_workflow;
            vec.drain(0..drop_count);
        }
    }

    /// 把反思落库到 `trajectory_workflow_reflections`(best-effort)。
    ///
    /// 落库失败仅记录 warn 日志,不返回错误。理由:
    /// - 反思落库是辅助功能,失败不应影响工作流主流程
    /// - `WorkflowOptimizer` / `WorkflowEvolver` 仍可从内存 `get_history` 获取近期反思
    /// - 跨进程重启场景才依赖落库数据,生产环境偶发 DB 错误不应触发工作流回滚
    async fn persist_to_storage(
        &self,
        workflow_id: &str,
        template_id: Option<&str>,
        reflection: &Reflection,
    ) {
        if let Some(ref storage) = self.storage
            && let Err(e) =
                storage.save_workflow_reflection(workflow_id, template_id, reflection).await
        {
            tracing::warn!(
                "Failed to persist workflow reflection (workflow_id={}, execution_id={}): {}",
                workflow_id,
                reflection.task_id,
                e
            );
        }
    }

    /// 质量分计算(启发式):
    /// - 基础分 = 10 - 失败率 * 5 - 重试率 * 2 - 高延迟惩罚
    /// - 失败率高 → 大幅扣分
    /// - 全部成功 + 无重试 → 9-10 分
    fn compute_quality_score(
        &self,
        total: usize,
        failed: usize,
        total_retries: u32,
        high_latency_count: usize,
    ) -> u8 {
        if total == 0 {
            return 5;
        }
        let failure_rate = failed as f32 / total as f32;
        let retry_rate = if total > 0 {
            total_retries as f32 / total as f32
        } else {
            0.0
        };
        let latency_penalty = (high_latency_count as f32 / total as f32) * 1.5;
        let raw = 10.0 - failure_rate * 5.0 - retry_rate * 2.0 - latency_penalty;
        raw.round().clamp(1.0, 10.0) as u8
    }

    /// 从节点快照列表计算瓶颈节点。
    fn find_bottlenecks(&self, nodes: &[NodeExecutionSnapshot]) -> Vec<BottleneckNode> {
        let total = nodes.len();
        if total == 0 {
            return Vec::new();
        }
        let mut by_id: HashMap<String, (u32, u32, u64, u32)> = HashMap::new();
        for n in nodes {
            let entry = by_id.entry(n.node_id.clone()).or_insert((0, 0, 0, 0));
            entry.0 += 1; // 出现次数
            if matches!(n.status, NodeStatus::Failed) {
                entry.1 += 1; // 失败次数
            }
            if let Some(ms) = n.execution_time_ms {
                entry.2 += ms;
            }
            entry.3 += n.attempts.saturating_sub(1); // 重试次数(去除首次)
        }
        let mut out = Vec::new();
        for (node_id, (count, failed, total_ms, retries)) in by_id {
            let avg_ms = if count > 0 {
                total_ms / count as u64
            } else {
                0
            };
            let failure_rate = failed as f32 / count as f32;
            if avg_ms >= self.config.high_latency_ms {
                out.push(BottleneckNode {
                    node_id: node_id.clone(),
                    node_type: nodes
                        .iter()
                        .find(|n| n.node_id == node_id)
                        .map(|n| n.node_type.clone())
                        .unwrap_or_default(),
                    reason: BottleneckReason::HighLatency,
                    impact_score: (avg_ms as f32 / 10_000.0).min(1.0),
                    detail: format!(
                        "平均耗时 {avg_ms}ms 超过阈值 {}ms",
                        self.config.high_latency_ms
                    ),
                });
            }
            if failure_rate >= self.config.high_failure_rate {
                out.push(BottleneckNode {
                    node_id: node_id.clone(),
                    node_type: nodes
                        .iter()
                        .find(|n| n.node_id == node_id)
                        .map(|n| n.node_type.clone())
                        .unwrap_or_default(),
                    reason: BottleneckReason::HighFailureRate,
                    impact_score: failure_rate,
                    detail: format!("失败率 {:.0}%({}/{})", failure_rate * 100.0, failed, count),
                });
            }
            if retries >= self.config.high_retry_count {
                out.push(BottleneckNode {
                    node_id: node_id.clone(),
                    node_type: nodes
                        .iter()
                        .find(|n| n.node_id == node_id)
                        .map(|n| n.node_type.clone())
                        .unwrap_or_default(),
                    reason: BottleneckReason::HighRetryCount,
                    impact_score: (retries as f32 / 10.0).min(1.0),
                    detail: format!("重试 {retries} 次超过阈值 {}", self.config.high_retry_count),
                });
            }
        }
        out
    }

    /// 从 `WorkflowExecutionRecord` 聚合统计量。
    fn aggregate_stats(record: &WfRecord) -> (usize, usize, u32, u64) {
        let total = record.nodes.len();
        let mut failed = 0;
        let mut total_retries = 0u32;
        let mut max_latency_ms = 0u64;
        for n in &record.nodes {
            if matches!(n.status, NodeStatus::Failed) {
                failed += 1;
            }
            total_retries += n.attempts.saturating_sub(1);
            if let Some(ms) = n.execution_time_ms {
                max_latency_ms = max_latency_ms.max(ms);
            }
        }
        (total, failed, total_retries, max_latency_ms)
    }

    /// 构造 `QualityMetrics`(六维度启发式打分)。
    fn build_quality_metrics(
        &self,
        total: usize,
        failed: usize,
        total_retries: u32,
        high_latency_count: usize,
        duration_ms: u64,
    ) -> QualityMetrics {
        let success_rate = if total > 0 {
            1.0 - (failed as f32 / total as f32)
        } else {
            1.0
        };
        let task_success_score = success_rate * 10.0;
        let tool_efficiency_score = if total > 0 {
            10.0 - (total_retries as f32 / total as f32) * 5.0
        } else {
            10.0
        };
        let iteration_efficiency_score = if total > 0 {
            10.0 - (total_retries as f32 / total as f32) * 3.0
        } else {
            10.0
        };
        let time_efficiency_score = if duration_ms > 0 {
            // 假设 1s/节点为基准,超过则线性扣分
            let baseline_ms = total as u64 * 1_000;
            let ratio = baseline_ms as f32 / duration_ms.max(1) as f32;
            (ratio * 10.0).min(10.0)
        } else {
            10.0
        };
        let error_recovery_score = if failed > 0 {
            // 失败时若工作流仍能完成(PartiallyCompleted),给予一定恢复分
            5.0
        } else {
            10.0
        };
        let goal_completion_score = if failed == 0 {
            10.0
        } else if failed < total {
            7.0
        } else {
            2.0
        };
        let overall_weighted_score = (task_success_score * 0.3
            + tool_efficiency_score * 0.15
            + iteration_efficiency_score * 0.1
            + time_efficiency_score * 0.15
            + error_recovery_score * 0.15
            + goal_completion_score * 0.15)
            .clamp(0.0, 10.0);
        // 避免 unused warning(high_latency_count 用于上下文,但未参与公式)
        let _ = high_latency_count;
        QualityMetrics {
            task_success_score,
            tool_efficiency_score,
            iteration_efficiency_score,
            time_efficiency_score,
            error_recovery_score,
            goal_completion_score,
            overall_weighted_score,
        }
    }

    /// 分类失败原因(基于错误消息启发式)。
    fn categorize_failure(error_msg: &str) -> FailureCategory {
        let lower = error_msg.to_lowercase();
        if lower.contains("timeout") || lower.contains("timed out") {
            FailureCategory::Timeout
        } else if lower.contains("permission") || lower.contains("denied") {
            FailureCategory::PermissionDenied
        } else if lower.contains("not found") || lower.contains("unavailable") {
            FailureCategory::ToolUnavailable
        } else if lower.contains("schema") || lower.contains("validation") {
            FailureCategory::OutputSchemaMismatch
        } else if lower.contains("config") {
            FailureCategory::ConfigError
        } else if lower.contains("llm") || lower.contains("model") {
            FailureCategory::LlmError
        } else if lower.contains("input") {
            FailureCategory::InputMismatch
        } else if lower.contains("external") || lower.contains("service") {
            FailureCategory::ExternalService
        } else {
            FailureCategory::Unknown
        }
    }
}

#[async_trait]
impl WorkflowReflector for WorkflowReflectorImpl {
    async fn reflect(&self, record: &WorkflowExecutionRecord) -> Result<Reflection, String> {
        let (total, failed, total_retries, max_latency) = Self::aggregate_stats(record);
        let bottlenecks = self.find_bottlenecks(&record.nodes);
        let high_latency_count = bottlenecks
            .iter()
            .filter(|b| matches!(b.reason, BottleneckReason::HighLatency))
            .count();
        let metrics = self.build_quality_metrics(
            total,
            failed,
            total_retries,
            high_latency_count,
            record.duration_ms,
        );
        let quality_score =
            self.compute_quality_score(total, failed, total_retries, high_latency_count);

        // 构造错误模式与可复用模式
        let mut error_patterns = Vec::new();
        let mut reusable_patterns = Vec::new();
        for n in &record.nodes {
            if matches!(n.status, NodeStatus::Failed)
                && let Some(err) = &n.error
            {
                let cat = Self::categorize_failure(err);
                error_patterns.push(format!(
                    "{}({}): {}",
                    n.node_id,
                    failure_category_str(&cat),
                    err
                ));
            }
            if matches!(n.status, NodeStatus::Completed)
                && let Some(out) = &n.output
                && let Some(s) = out.as_str()
                && s.len() > 10
            {
                reusable_patterns.push(format!(
                    "{}({}): 输出有效长度 {}",
                    n.node_id,
                    n.node_type,
                    s.len()
                ));
            }
        }
        // 高延迟瓶颈作为可改进模式
        for b in &bottlenecks {
            error_patterns.push(format!("瓶颈 {}:{:?}", b.node_id, b.reason));
        }

        // 失败节点分析(若工作流整体失败,取首个失败节点)
        let failed_node_analysis = if failed > 0 {
            record.nodes.iter().find(|n| matches!(n.status, NodeStatus::Failed)).map(|n| {
                NodeFailureAnalysis {
                    node_id: n.node_id.clone(),
                    root_cause: n.error.clone().unwrap_or_else(|| "未知错误".to_string()),
                    failure_category: Self::categorize_failure(
                        &n.error.clone().unwrap_or_default(),
                    ),
                    recovery_strategy: "建议先检查节点配置与上游输出,必要时增加 retry".to_string(),
                    related_nodes: Vec::new(),
                }
            })
        } else {
            None
        };

        // 提议的变更(基于瓶颈)
        let mut proposed_changes = Vec::new();
        for b in &bottlenecks {
            match b.reason {
                BottleneckReason::HighRetryCount => {
                    proposed_changes.push(ProposedChange::TuneRetry {
                        node_id: b.node_id.clone(),
                        max_attempts: 5,
                        backoff_ms: 2_000,
                    });
                },
                BottleneckReason::HighFailureRate => {
                    proposed_changes.push(ProposedChange::TuneRetry {
                        node_id: b.node_id.clone(),
                        max_attempts: 3,
                        backoff_ms: 1_000,
                    });
                },
                _ => {},
            }
        }

        let metadata = WorkflowReflectionMetadata {
            workflow_id: record.workflow_id.clone(),
            execution_id: record.execution_id.clone(),
            bottleneck_nodes: bottlenecks,
            node_patterns: Vec::new(),
            failed_node_analysis,
            proposed_changes,
        };
        let metadata_json = serde_json::to_value(&metadata).map_err(|e| e.to_string())?;

        let status_str = format!("{:?}", record.status);
        let summary = format!(
            "工作流 {} 执行 {} — 节点 {total} 失败 {failed} 重试 {total_retries} 最大延迟 {max_latency}ms",
            record.workflow_id, status_str
        );
        let mut reflection = Reflection::new(record.execution_id.clone())
            // 先设置 metrics(内部会用 overall_weighted_score 计算 quality_score),
            // 再用 with_quality 覆盖为 compute_quality_score 的更严格结果(失败/重试/延迟惩罚)
            .with_quality_metrics(metrics)
            .with_quality(quality_score, format!("质量分 {quality_score}/10:{summary}"))
            .with_patterns(error_patterns, reusable_patterns)
            .with_summary(summary);
        reflection.timestamp = Utc::now();
        reflection.metadata = Some(metadata_json);

        // 写入内存历史
        self.store_history(&record.workflow_id, reflection.clone()).await;
        // 落库到 trajectory_workflow_reflections(优化 3,best-effort)
        self.persist_to_storage(&record.workflow_id, record.template_id.as_deref(), &reflection)
            .await;
        Ok(reflection)
    }

    async fn reflect_node(
        &self,
        record: &WorkflowExecutionRecord,
        failed_node: &NodeExecutionSnapshot,
    ) -> Result<Reflection, String> {
        let err = failed_node.error.clone().unwrap_or_else(|| "未知错误".to_string());
        let category = Self::categorize_failure(&err);
        let analysis = NodeFailureAnalysis {
            node_id: failed_node.node_id.clone(),
            root_cause: err.clone(),
            failure_category: category,
            recovery_strategy: match category {
                FailureCategory::Timeout => "增加 timeout 配置或拆分大任务".to_string(),
                FailureCategory::PermissionDenied => "检查凭证/权限设置".to_string(),
                FailureCategory::ToolUnavailable => "确认工具已注册并启用".to_string(),
                FailureCategory::OutputSchemaMismatch => {
                    "调整节点输出 schema 或下游消费方式".to_string()
                },
                FailureCategory::ConfigError => "修正节点 config 字段".to_string(),
                FailureCategory::LlmError => "检查 provider/model 配置与限流".to_string(),
                FailureCategory::InputMismatch => "核对 input_mapping 与上游输出".to_string(),
                FailureCategory::ExternalService => "检查外部服务可用性与重试".to_string(),
                FailureCategory::LogicError => "审查节点执行逻辑".to_string(),
                FailureCategory::Unknown => "需要进一步人工排查".to_string(),
            },
            related_nodes: Vec::new(),
        };

        // 节点级 reflection 质量分:失败 = 2,超时 = 3,其他 = 4
        let quality_score: u8 = match category {
            FailureCategory::Timeout => 3,
            FailureCategory::ConfigError | FailureCategory::InputMismatch => 4,
            _ => 2,
        };

        let metadata = WorkflowReflectionMetadata {
            workflow_id: record.workflow_id.clone(),
            execution_id: record.execution_id.clone(),
            bottleneck_nodes: vec![BottleneckNode {
                node_id: failed_node.node_id.clone(),
                node_type: failed_node.node_type.clone(),
                reason: BottleneckReason::HighFailureRate,
                impact_score: 1.0,
                detail: format!("失败分类:{:?} - {}", category, err),
            }],
            node_patterns: Vec::new(),
            failed_node_analysis: Some(analysis),
            proposed_changes: vec![ProposedChange::TuneRetry {
                node_id: failed_node.node_id.clone(),
                max_attempts: 3,
                backoff_ms: 1_000,
            }],
        };
        let metadata_json = serde_json::to_value(&metadata).map_err(|e| e.to_string())?;

        let summary = format!(
            "节点 {}({})执行失败 - 分类:{:?}",
            failed_node.node_id, failed_node.node_type, category
        );
        let mut reflection = Reflection::new(record.execution_id.clone())
            .with_quality(quality_score, summary.clone())
            .with_patterns(vec![err], Vec::new())
            .with_summary(summary);
        reflection.timestamp = Utc::now();
        reflection.metadata = Some(metadata_json);

        // 节点级反思也写入历史(以 workflow_id 索引)
        self.store_history(&record.workflow_id, reflection.clone()).await;
        // 落库到 trajectory_workflow_reflections(优化 3,best-effort)
        self.persist_to_storage(&record.workflow_id, record.template_id.as_deref(), &reflection)
            .await;
        Ok(reflection)
    }

    async fn aggregate_patterns(
        &self,
        records: &[WorkflowExecutionRecord],
    ) -> Result<Vec<WorkflowPattern>, String> {
        // 简单频次统计:按 (node_id, node_type, status) 三元组聚合
        let mut counter: HashMap<(String, String, String), u32> = HashMap::new();
        for r in records {
            for n in &r.nodes {
                let key = (n.node_id.clone(), n.node_type.clone(), format!("{:?}", n.status));
                *counter.entry(key).or_insert(0) += 1;
            }
        }
        let mut patterns = Vec::new();
        for ((node_id, node_type, status), freq) in counter {
            if freq >= 2 {
                patterns.push(WorkflowPattern {
                    id: format!("pattern-{node_id}-{status}"),
                    name: format!("{node_type} {status}"),
                    description: format!(
                        "节点 {node_id}({node_type}) 在 {freq} 次执行中状态为 {status}"
                    ),
                    node_ids: vec![node_id],
                    frequency: freq,
                    confidence: (freq as f32 / records.len() as f32).min(1.0),
                });
            }
        }
        // 按频率降序
        patterns.sort_by_key(|b| std::cmp::Reverse(b.frequency));
        Ok(patterns)
    }

    async fn get_history(
        &self,
        workflow_id: &str,
        limit: usize,
    ) -> Result<Vec<Reflection>, String> {
        let guard = self.history.read().await;
        let vec = guard.get(workflow_id).cloned().unwrap_or_default();
        let start = vec.len().saturating_sub(limit);
        Ok(vec[start..].to_vec())
    }
}

// 辅助:FailureCategory 显示字符串(free function,避免 inherent impl 跨 crate)
fn failure_category_str(c: &FailureCategory) -> &'static str {
    match c {
        FailureCategory::ConfigError => "配置错误",
        FailureCategory::InputMismatch => "输入不匹配",
        FailureCategory::OutputSchemaMismatch => "输出 schema 不匹配",
        FailureCategory::ToolUnavailable => "工具不可用",
        FailureCategory::Timeout => "超时",
        FailureCategory::PermissionDenied => "权限拒绝",
        FailureCategory::LlmError => "LLM 错误",
        FailureCategory::LogicError => "逻辑错误",
        FailureCategory::ExternalService => "外部服务",
        FailureCategory::Unknown => "未知",
    }
}

// 默认实现的 Arc 别名,便于 wiring 层注入
impl WorkflowReflectorImpl {
    /// 转为 `Arc<dyn WorkflowReflector>` 供 wiring 层注入。
    pub fn into_arc(self) -> Arc<dyn WorkflowReflector> {
        Arc::new(self)
    }
}

// 显式标注 workflow_adapters 模块的可见性
pub(crate) fn _ensure_visibility(_: &WorkflowReflectorImpl) {}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::workflow_reflection::WorkflowRunStatus;
    use axagent_harness::workflow_types::WorkflowErrorContext;

    fn make_record(
        status: WorkflowRunStatus,
        nodes: Vec<NodeExecutionSnapshot>,
    ) -> WorkflowExecutionRecord {
        WorkflowExecutionRecord {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            template_id: None,
            template_version: None,
            status,
            started_at: 0,
            completed_at: Some(100),
            duration_ms: 100,
            nodes,
            edges: Vec::new(),
            template_nodes: Vec::new(),
            input: None,
            output: None,
            error_context: None,
        }
    }

    fn make_node(id: &str, status: NodeStatus, attempts: u32, ms: u64) -> NodeExecutionSnapshot {
        NodeExecutionSnapshot {
            node_id: id.to_string(),
            node_type: "llm".to_string(),
            node_name: None,
            status,
            attempts,
            input: None,
            output: None,
            execution_time_ms: Some(ms),
            error: None,
            started_at: 0,
            completed_at: Some(100),
            sub_workflow_id: None,
        }
    }

    #[tokio::test]
    async fn test_reflect_completed_workflow() {
        let r = WorkflowReflectorImpl::with_defaults();
        let record = make_record(
            WorkflowRunStatus::Completed,
            vec![
                make_node("n1", NodeStatus::Completed, 1, 100),
                make_node("n2", NodeStatus::Completed, 1, 200),
            ],
        );
        let reflection = r.reflect(&record).await.unwrap();
        assert!(
            reflection.quality_score >= 8,
            "expected high quality, got {}",
            reflection.quality_score
        );
        assert!(reflection.metadata.is_some());
    }

    #[tokio::test]
    async fn test_reflect_failed_workflow() {
        let r = WorkflowReflectorImpl::with_defaults();
        let record = make_record(
            WorkflowRunStatus::Failed,
            vec![
                make_node("n1", NodeStatus::Completed, 1, 100),
                NodeExecutionSnapshot {
                    error: Some("timeout".to_string()),
                    ..make_node("n2", NodeStatus::Failed, 3, 6_000)
                },
            ],
        );
        let reflection = r.reflect(&record).await.unwrap();
        assert!(
            reflection.quality_score <= 5,
            "expected low quality, got {}",
            reflection.quality_score
        );
    }

    #[tokio::test]
    async fn test_reflect_node_failure() {
        let r = WorkflowReflectorImpl::with_defaults();
        let record = make_record(WorkflowRunStatus::Failed, vec![]);
        let failed_node = NodeExecutionSnapshot {
            node_id: "n1".to_string(),
            node_type: "llm".to_string(),
            node_name: None,
            status: NodeStatus::Failed,
            attempts: 1,
            input: None,
            output: None,
            execution_time_ms: Some(100),
            error: Some("permission denied".to_string()),
            started_at: 0,
            completed_at: Some(100),
            sub_workflow_id: None,
        };
        let reflection = r.reflect_node(&record, &failed_node).await.unwrap();
        assert!(reflection.quality_score <= 4);
    }

    #[tokio::test]
    async fn test_get_history() {
        let r = WorkflowReflectorImpl::with_defaults();
        let record = make_record(
            WorkflowRunStatus::Completed,
            vec![make_node("n1", NodeStatus::Completed, 1, 100)],
        );
        r.reflect(&record).await.unwrap();
        let history = r.get_history("wf-1", 10).await.unwrap();
        assert_eq!(history.len(), 1);
        assert!(history[0].metadata.is_some());
    }

    #[tokio::test]
    async fn test_aggregate_patterns() {
        let r = WorkflowReflectorImpl::with_defaults();
        let records = vec![
            make_record(
                WorkflowRunStatus::Completed,
                vec![make_node("n1", NodeStatus::Completed, 1, 100)],
            ),
            make_record(
                WorkflowRunStatus::Completed,
                vec![make_node("n1", NodeStatus::Completed, 1, 100)],
            ),
        ];
        let patterns = r.aggregate_patterns(&records).await.unwrap();
        assert!(!patterns.is_empty());
        assert_eq!(patterns[0].frequency, 2);
    }

    #[test]
    fn test_categorize_failure() {
        assert!(matches!(
            WorkflowReflectorImpl::categorize_failure("operation timed out"),
            FailureCategory::Timeout
        ));
        assert!(matches!(
            WorkflowReflectorImpl::categorize_failure("permission denied for user"),
            FailureCategory::PermissionDenied
        ));
        assert!(matches!(
            WorkflowReflectorImpl::categorize_failure("random error"),
            FailureCategory::Unknown
        ));
    }

    // 避免未使用警告
    #[test]
    fn test_workflow_error_context_compiles() {
        let _ = WorkflowErrorContext::new(
            "n1".to_string(),
            "Node1".to_string(),
            "NODE_ERROR".to_string(),
            "err".to_string(),
            "wf-1".to_string(),
            "exec-1".to_string(),
            None,
        );
    }
}
