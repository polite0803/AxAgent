// SPDX-License-Identifier: AGPL-3.0-only

//! `WorkflowOptimizerImpl`:基于反思结果生成可执行优化建议(不修改模板)。
//!
//! MVP 策略:
//! - 不依赖 LLM,纯启发式规则
//! - 从 `Reflection::metadata` 反序列化 `WorkflowReflectionMetadata`,
//!   基于 `bottleneck_nodes` 与 `failed_node_analysis` 生成 `WorkflowSuggestion`
//! - `apply_suggestions` 应用变更到 `WorkflowTemplateData` 克隆,不修改原模板
//! - `estimate_impact` 基于 `SuggestionCategory` + `SuggestionPriority` 启发式评分
//!
//! 与 `commands/workflow_ai_diagnose.rs`(LLM 单次诊断)互补,本实现走规则路径。

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use axagent_harness::reflection_types::Reflection;
use axagent_harness::workflow_optimization::{
    ProposedChange, SuggestionCategory, SuggestionPriority, WorkflowOptimizer, WorkflowSuggestion,
};
use axagent_harness::workflow_reflection::{
    BottleneckReason, FailureCategory, WorkflowReflectionMetadata,
};
use axagent_harness::workflow_types::{WorkflowNode, WorkflowTemplateData};

/// `WorkflowOptimizer` 的 trajectory 实现。
pub struct WorkflowOptimizerImpl;

impl WorkflowOptimizerImpl {
    pub fn new() -> Self {
        Self
    }

    /// 默认配置构造(无状态)。
    pub fn with_defaults() -> Self {
        Self::new()
    }

    /// 从 Reflection.metadata 反序列化 WorkflowReflectionMetadata。
    fn extract_metadata(reflection: &Reflection) -> Option<WorkflowReflectionMetadata> {
        reflection
            .metadata
            .as_ref()
            .and_then(|v| serde_json::from_value::<WorkflowReflectionMetadata>(v.clone()).ok())
    }

    /// 基于 metadata 生成建议(单次反思)。
    fn suggest_from_metadata(metadata: &WorkflowReflectionMetadata) -> Vec<WorkflowSuggestion> {
        let mut out = Vec::new();

        // 1. 瓶颈节点 → 资源调优 / TuneRetry
        for b in &metadata.bottleneck_nodes {
            match b.reason {
                BottleneckReason::HighLatency => {
                    out.push(WorkflowSuggestion {
                        id: format!("sugg-{}", Uuid::new_v4()),
                        category: SuggestionCategory::ResourceTuning,
                        priority: SuggestionPriority::Medium,
                        target_node_id: Some(b.node_id.clone()),
                        description: format!(
                            "节点 {} 平均耗时过高({}),建议拆分任务或增加并发",
                            b.node_id, b.detail
                        ),
                        proposed_change: ProposedChange::UpdateConfig {
                            node_id: b.node_id.clone(),
                            patch: serde_json::json!({ "_note": "consider_parallel_or_split" }),
                        },
                        confidence: b.impact_score,
                        estimated_impact: Some(0.3),
                    });
                },
                BottleneckReason::HighFailureRate => {
                    out.push(WorkflowSuggestion {
                        id: format!("sugg-{}", Uuid::new_v4()),
                        category: SuggestionCategory::ErrorHandling,
                        priority: SuggestionPriority::High,
                        target_node_id: Some(b.node_id.clone()),
                        description: format!(
                            "节点 {} 失败率过高({}),建议增加 retry 与超时容错",
                            b.node_id, b.detail
                        ),
                        proposed_change: ProposedChange::TuneRetry {
                            node_id: b.node_id.clone(),
                            max_attempts: 3,
                            backoff_ms: 1_000,
                        },
                        confidence: b.impact_score,
                        estimated_impact: Some(0.6),
                    });
                },
                BottleneckReason::HighRetryCount => {
                    out.push(WorkflowSuggestion {
                        id: format!("sugg-{}", Uuid::new_v4()),
                        category: SuggestionCategory::ErrorHandling,
                        priority: SuggestionPriority::High,
                        target_node_id: Some(b.node_id.clone()),
                        description: format!(
                            "节点 {} 重试次数过多({}),建议检查上游输出或增加退避",
                            b.node_id, b.detail
                        ),
                        proposed_change: ProposedChange::TuneRetry {
                            node_id: b.node_id.clone(),
                            max_attempts: 5,
                            backoff_ms: 2_000,
                        },
                        confidence: b.impact_score,
                        estimated_impact: Some(0.5),
                    });
                },
                BottleneckReason::ResourceHeavy | BottleneckReason::SequentialBlocking => {
                    out.push(WorkflowSuggestion {
                        id: format!("sugg-{}", Uuid::new_v4()),
                        category: SuggestionCategory::NodeReplacement,
                        priority: SuggestionPriority::Medium,
                        target_node_id: Some(b.node_id.clone()),
                        description: format!(
                            "节点 {} 资源占用高/顺序阻塞({}),建议替换为更高效实现",
                            b.node_id, b.detail
                        ),
                        proposed_change: ProposedChange::ReplaceNode {
                            node_id: b.node_id.clone(),
                            new_type: "agent".to_string(),
                            new_config: serde_json::json!({ "_note": "replace_with_agent" }),
                        },
                        confidence: b.impact_score,
                        estimated_impact: Some(0.4),
                    });
                },
            }
        }

        // 2. 失败节点分析 → 错误处理建议
        if let Some(analysis) = &metadata.failed_node_analysis {
            let priority = match analysis.failure_category {
                FailureCategory::Timeout
                | FailureCategory::PermissionDenied
                | FailureCategory::ToolUnavailable => SuggestionPriority::Critical,
                _ => SuggestionPriority::High,
            };
            out.push(WorkflowSuggestion {
                id: format!("sugg-{}", Uuid::new_v4()),
                category: SuggestionCategory::ErrorHandling,
                priority,
                target_node_id: Some(analysis.node_id.clone()),
                description: format!(
                    "节点 {} 失败分类:{:?} - 恢复策略:{}",
                    analysis.node_id, analysis.failure_category, analysis.recovery_strategy
                ),
                proposed_change: ProposedChange::UpdateConfig {
                    node_id: analysis.node_id.clone(),
                    patch: serde_json::json!({
                        "_recovery_strategy": analysis.recovery_strategy,
                        "_failure_category": format!("{:?}", analysis.failure_category),
                    }),
                },
                confidence: 0.7,
                estimated_impact: Some(0.7),
            });
        }

        // 3. proposed_changes 直接转为 WorkflowSuggestion(若存在)
        for change in &metadata.proposed_changes {
            // workflow_reflection::ProposedChange 与本模块 ProposedChange 是两个独立 enum,
            // 这里通过序列化-反序列化做跨模块转换(同构但独立定义)。
            let value = match serde_json::to_value(change) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let Ok(opt_change) = serde_json::from_value::<ProposedChange>(value.clone()) else {
                continue;
            };
            let target_node_id = match &opt_change {
                ProposedChange::UpdateConfig { node_id, .. }
                | ProposedChange::ReplaceNode { node_id, .. }
                | ProposedChange::RemoveNode { node_id }
                | ProposedChange::RefinePrompt { node_id, .. }
                | ProposedChange::TuneRetry { node_id, .. } => Some(node_id.clone()),
                ProposedChange::RewireEdge { from, .. } => Some(from.clone()),
                ProposedChange::AddNode { after, .. } => Some(after.clone()),
            };
            out.push(WorkflowSuggestion {
                id: format!("sugg-{}", Uuid::new_v4()),
                category: SuggestionCategory::NodeConfig,
                priority: SuggestionPriority::Medium,
                target_node_id,
                description: format!("来自反思的提议变更:{}", value),
                proposed_change: opt_change,
                confidence: 0.6,
                estimated_impact: None,
            });
        }

        out
    }

    /// 基于 reflection.quality_score 调整建议优先级。
    fn adjust_priority_by_quality(
        mut suggestions: Vec<WorkflowSuggestion>,
        quality: u8,
    ) -> Vec<WorkflowSuggestion> {
        if quality <= 3 {
            // 低质量分:全部升级到 Critical/High
            for s in &mut suggestions {
                if matches!(s.priority, SuggestionPriority::Medium | SuggestionPriority::Low) {
                    s.priority = SuggestionPriority::High;
                }
            }
        }
        suggestions
    }

    /// 把单个 suggestion 应用到模板(克隆后修改)。
    fn apply_one(template: &mut WorkflowTemplateData, suggestion: &WorkflowSuggestion) {
        match &suggestion.proposed_change {
            ProposedChange::TuneRetry { node_id, max_attempts, backoff_ms } => {
                // P1-4 修复:通过 base_mut() 真正写回 retry 配置。
                let mut applied = false;
                for node in &mut template.nodes {
                    if node.base_id() == node_id {
                        let retry = &mut node.base_mut().retry;
                        retry.enabled = true;
                        retry.max_retries = *max_attempts;
                        retry.base_delay_ms = *backoff_ms;
                        applied = true;
                        break;
                    }
                }
                if applied {
                    tracing::debug!(
                        "[Optimizer] TuneRetry applied to {} (max_retries={}, backoff_ms={})",
                        node_id,
                        max_attempts,
                        backoff_ms
                    );
                } else {
                    tracing::warn!("[Optimizer] TuneRetry target node not found: {}", node_id);
                }
            },
            ProposedChange::RefinePrompt { node_id, new_prompt } => {
                // P1-4 修复:真正写回 AgentNode.config.system_prompt / LLMNode.config.prompt。
                // 仅这两种节点有 prompt 字段;其他变体(Condition/Parallel/...)视为不适用,告警跳过。
                let mut applied = false;
                let mut unsupported = false;
                for node in &mut template.nodes {
                    if node.base_id() != node_id {
                        continue;
                    }
                    match node {
                        WorkflowNode::Agent(agent) => {
                            agent.config.system_prompt = new_prompt.clone();
                            applied = true;
                            break;
                        },
                        WorkflowNode::Llm(llm) => {
                            llm.config.prompt = new_prompt.clone();
                            applied = true;
                            break;
                        },
                        WorkflowNode::LlmClassifier(c) => {
                            c.config.prompt = new_prompt.clone();
                            applied = true;
                            break;
                        },
                        _ => {
                            unsupported = true;
                            break;
                        },
                    }
                }
                if applied {
                    tracing::debug!(
                        "[Optimizer] RefinePrompt applied to {} (new length={})",
                        node_id,
                        new_prompt.len()
                    );
                } else if unsupported {
                    tracing::warn!(
                        "[Optimizer] RefinePrompt target node {} has no prompt field, skipped",
                        node_id
                    );
                } else {
                    tracing::warn!("[Optimizer] RefinePrompt target node not found: {}", node_id);
                }
            },
            ProposedChange::RemoveNode { node_id } => {
                template.nodes.retain(|n| n.base_id() != node_id);
                // 同时清理关联边
                template.edges.retain(|e| e.source != *node_id && e.target != *node_id);
                tracing::debug!("[Optimizer] RemoveNode applied to {}", node_id);
            },
            ProposedChange::RewireEdge { from, to: _, new_target } => {
                for edge in &mut template.edges {
                    if edge.source == *from {
                        edge.target = new_target.clone();
                    }
                }
                tracing::debug!("[Optimizer] RewireEdge applied from {}", from);
            },
            // AddNode / ReplaceNode / UpdateConfig 需 WorkflowNode 构造器,留待后续增强
            _ => {
                tracing::debug!(
                    "[Optimizer] suggestion kind not applied (requires node constructor): {:?}",
                    suggestion.category
                );
            },
        }
    }
}

impl Default for WorkflowOptimizerImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowOptimizer for WorkflowOptimizerImpl {
    async fn suggest(
        &self,
        _template: &WorkflowTemplateData,
        reflection: &Reflection,
    ) -> Result<Vec<WorkflowSuggestion>, String> {
        let metadata = match Self::extract_metadata(reflection) {
            Some(m) => m,
            None => {
                // metadata 缺失时返回空(任务级反思无可优化建议)
                return Ok(Vec::new());
            },
        };
        let suggestions = Self::suggest_from_metadata(&metadata);
        Ok(Self::adjust_priority_by_quality(suggestions, reflection.quality_score))
    }

    async fn suggest_batch(
        &self,
        template: &WorkflowTemplateData,
        reflections: &[Reflection],
    ) -> Result<Vec<WorkflowSuggestion>, String> {
        let mut all = Vec::new();
        for r in reflections {
            let mut s = self.suggest(template, r).await?;
            all.append(&mut s);
        }
        // 按 priority 排序(Critical > High > Medium > Low)
        all.sort_by(|a, b| {
            let pa = priority_weight(a.priority);
            let pb = priority_weight(b.priority);
            pb.cmp(&pa)
        });
        Ok(all)
    }

    async fn apply_suggestions(
        &self,
        template: &WorkflowTemplateData,
        suggestions: &[WorkflowSuggestion],
    ) -> Result<WorkflowTemplateData, String> {
        let mut new_template = template.clone();
        for s in suggestions {
            Self::apply_one(&mut new_template, s);
        }
        Ok(new_template)
    }

    async fn estimate_impact(
        &self,
        _template: &WorkflowTemplateData,
        suggestion: &WorkflowSuggestion,
    ) -> Result<f32, String> {
        // 若已有 estimated_impact 字段,直接返回
        if let Some(impact) = suggestion.estimated_impact {
            return Ok(impact);
        }
        // 否则基于 category + priority 启发式评分
        let base = match suggestion.category {
            SuggestionCategory::ErrorHandling => 0.6,
            SuggestionCategory::PromptRefine => 0.4,
            SuggestionCategory::NodeReplacement => 0.5,
            SuggestionCategory::NodeConfig => 0.4,
            SuggestionCategory::EdgeRewire => 0.3,
            SuggestionCategory::VariableMisconfig => 0.4,
            SuggestionCategory::ResourceTuning => 0.5,
        };
        let priority_boost = match suggestion.priority {
            SuggestionPriority::Critical => 0.2,
            SuggestionPriority::High => 0.1,
            SuggestionPriority::Medium => 0.0,
            SuggestionPriority::Low => -0.1,
        };
        Ok((base + priority_boost + suggestion.confidence * 0.1).clamp(0.0, 1.0))
    }
}

/// 优先级权重(用于排序)。
fn priority_weight(p: SuggestionPriority) -> u8 {
    match p {
        SuggestionPriority::Critical => 4,
        SuggestionPriority::High => 3,
        SuggestionPriority::Medium => 2,
        SuggestionPriority::Low => 1,
    }
}

impl WorkflowOptimizerImpl {
    /// 转为 `Arc<dyn WorkflowOptimizer>` 供 wiring 层注入。
    pub fn into_arc(self) -> Arc<dyn WorkflowOptimizer> {
        Arc::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::workflow_reflection::{BottleneckNode, NodeFailureAnalysis};
    use axagent_harness::workflow_types::WorkflowTemplateData;

    /// 构造一个空 WorkflowTemplateData(WorkflowTemplateData 未实现 Default)。
    fn make_empty_template() -> WorkflowTemplateData {
        WorkflowTemplateData {
            id: "tpl-1".to_string(),
            name: "TestTemplate".to_string(),
            description: None,
            icon: String::new(),
            tags: Vec::new(),
            version: 1,
            is_preset: false,
            is_editable: true,
            is_public: false,
            visibility: Default::default(),
            trigger_config: None,
            nodes: Vec::new(),
            edges: Vec::new(),
            input_schema: None,
            output_schema: None,
            variables: Vec::new(),
            error_config: None,
            error_workflow_id: None,
            tool_defs: Vec::new(),
            mission_hash: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn make_reflection(quality: u8, metadata: WorkflowReflectionMetadata) -> Reflection {
        let mut r =
            Reflection::new("exec-1".to_string()).with_quality(quality, format!("q{quality}"));
        r.metadata = serde_json::to_value(&metadata).ok();
        r
    }

    fn make_metadata() -> WorkflowReflectionMetadata {
        WorkflowReflectionMetadata {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            bottleneck_nodes: vec![BottleneckNode {
                node_id: "n1".to_string(),
                node_type: "llm".to_string(),
                reason: BottleneckReason::HighFailureRate,
                impact_score: 0.8,
                detail: "失败率 80%".to_string(),
            }],
            node_patterns: Vec::new(),
            failed_node_analysis: Some(NodeFailureAnalysis {
                node_id: "n1".to_string(),
                root_cause: "timeout".to_string(),
                failure_category: FailureCategory::Timeout,
                recovery_strategy: "增加 timeout".to_string(),
                related_nodes: Vec::new(),
            }),
            proposed_changes: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_suggest_from_metadata() {
        let o = WorkflowOptimizerImpl::new();
        let template = make_empty_template();
        let reflection = make_reflection(3, make_metadata());
        let suggestions = o.suggest(&template, &reflection).await.expect("测试：异步操作应成功");
        assert!(!suggestions.is_empty(), "expected suggestions from metadata");
        // 低质量分下应该有 High 优先级建议
        assert!(suggestions.iter().any(|s| matches!(
            s.priority,
            SuggestionPriority::High | SuggestionPriority::Critical
        )));
    }

    #[tokio::test]
    async fn test_suggest_no_metadata_returns_empty() {
        let o = WorkflowOptimizerImpl::new();
        let template = make_empty_template();
        let reflection =
            Reflection::new("exec-1".to_string()).with_quality(5, "no meta".to_string());
        let suggestions = o.suggest(&template, &reflection).await.expect("测试：异步操作应成功");
        assert!(suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_estimate_impact_with_explicit_field() {
        let o = WorkflowOptimizerImpl::new();
        let template = make_empty_template();
        let suggestion = WorkflowSuggestion {
            id: "s1".to_string(),
            category: SuggestionCategory::ErrorHandling,
            priority: SuggestionPriority::High,
            target_node_id: None,
            description: "test".to_string(),
            proposed_change: ProposedChange::TuneRetry {
                node_id: "n1".to_string(),
                max_attempts: 3,
                backoff_ms: 1000,
            },
            confidence: 0.8,
            estimated_impact: Some(0.7),
        };
        let impact = o.estimate_impact(&template, &suggestion).await.expect("测试：异步操作应成功");
        assert!((impact - 0.7).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_estimate_impact_heuristic() {
        let o = WorkflowOptimizerImpl::new();
        let template = make_empty_template();
        let suggestion = WorkflowSuggestion {
            id: "s1".to_string(),
            category: SuggestionCategory::ErrorHandling,
            priority: SuggestionPriority::Critical,
            target_node_id: None,
            description: "test".to_string(),
            proposed_change: ProposedChange::TuneRetry {
                node_id: "n1".to_string(),
                max_attempts: 3,
                backoff_ms: 1000,
            },
            confidence: 0.5,
            estimated_impact: None,
        };
        let impact = o.estimate_impact(&template, &suggestion).await.expect("测试：异步操作应成功");
        // ErrorHandling(0.6) + Critical(0.2) + confidence*0.1(0.05) = 0.85
        assert!((impact - 0.85).abs() < 0.01, "got impact {}", impact);
    }

    #[tokio::test]
    async fn test_apply_suggestions_removes_node() {
        let o = WorkflowOptimizerImpl::new();
        // 添加一个测试节点(用 default 的 WorkflowNode,但 default 可能没有节点,这里仅测试空模板)
        let template = make_empty_template();
        let suggestion = WorkflowSuggestion {
            id: "s1".to_string(),
            category: SuggestionCategory::NodeReplacement,
            priority: SuggestionPriority::High,
            target_node_id: Some("n1".to_string()),
            description: "remove".to_string(),
            proposed_change: ProposedChange::RemoveNode { node_id: "n1".to_string() },
            confidence: 0.9,
            estimated_impact: None,
        };
        let new_template = o
            .apply_suggestions(&template, std::slice::from_ref(&suggestion))
            .await
            .expect("测试：异步操作应成功");
        // 空模板移除节点应保持空(不 panic)
        assert!(new_template.nodes.is_empty());
    }
}
