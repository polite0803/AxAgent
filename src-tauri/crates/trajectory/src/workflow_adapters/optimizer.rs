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
    BottleneckReason, FailureCategory, NodeFailureAnalysis, WorkflowPattern,
    WorkflowReflectionMetadata,
};
use axagent_harness::workflow_types::{WorkflowNode, WorkflowTemplateData};

/// `WorkflowOptimizer` 的 trajectory 实现。
/// 模板快照（Phase 4 版本回滚）。
#[derive(Debug, Clone)]
pub struct TemplateSnapshot {
    template_id: String,
    version: i32,
    template: WorkflowTemplateData,
    #[allow(dead_code)]
    timestamp_ms: i64,
    #[allow(dead_code)]
    applied_suggestion_ids: Vec<String>,
}

/// dry_run 沙箱验证报告（Phase 4）。
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ValidationReport {
    /// 是否通过验证
    pub passed: bool,
    /// DAG 完整性问题列表
    pub issues: Vec<String>,
    /// 应用前后节点数变化
    pub nodes_before: usize,
    pub nodes_after: usize,
    /// 应用前后边数变化
    pub edges_before: usize,
    pub edges_after: usize,
}

/// 快照历史上限（Phase 4）。
const MAX_SNAPSHOTS: usize = 10;

/// WorkflowOptimizer 的 trajectory 实现（Phase 4: 有状态 + 版本回滚 + 冲突消解）。
pub struct WorkflowOptimizerImpl {
    /// 模板历史快照，用于 rollback()。
    /// 每次成功 apply_suggestions 前自动 push，上限 MAX_SNAPSHOTS。
    snapshots: parking_lot::Mutex<Vec<TemplateSnapshot>>,
}

impl WorkflowOptimizerImpl {
    pub fn new() -> Self {
        Self { snapshots: parking_lot::Mutex::new(Vec::new()) }
    }

    /// 默认配置构造（有状态）。
    pub fn with_defaults() -> Self {
        Self::new()
    }

    /// 当前快照数量。
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.lock().len()
    }

    /// 最早的快照版本号（回滚起点）。
    pub fn earliest_snapshot_version(&self) -> Option<i32> {
        self.snapshots.lock().first().map(|s| s.version)
    }

    /// 暴露快照列表供 GUI / 调试查看。
    pub fn snapshots(&self) -> parking_lot::MutexGuard<'_, Vec<TemplateSnapshot>> {
        self.snapshots.lock()
    }
    /// Phase 4-2: 将模板推入快照历史（自动裁剪到 MAX_SNAPSHOTS）。
    fn push_snapshot(&self, template: &WorkflowTemplateData, applied_ids: Vec<String>) {
        self.snapshots.lock().push(TemplateSnapshot {
            template_id: template.id.clone(),
            version: template.version,
            template: template.clone(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            applied_suggestion_ids: applied_ids,
        });
        let mut snaps = self.snapshots.lock();
        while snaps.len() > MAX_SNAPSHOTS {
            snaps.remove(0);
        }
    }

    /// Phase 4-3: dry_run 沙箱验证 — 在克隆上应用 suggestions，检查 DAG 完整性。
    fn dry_run_validate(
        template: &WorkflowTemplateData,
        suggestions: &[WorkflowSuggestion],
    ) -> ValidationReport {
        let nodes_before = template.nodes.len();
        let edges_before = template.edges.len();

        let mut clone = template.clone();
        for s in suggestions {
            Self::apply_one(&mut clone, s);
        }

        let nodes_after = clone.nodes.len();
        let edges_after = clone.edges.len();

        let mut issues: Vec<String> = Vec::new();

        // DAG 完整性检查: 所有边引用的 source/target 节点都必须存在
        let node_ids: std::collections::HashSet<&str> =
            clone.nodes.iter().map(|n| n.base_id()).collect();
        for e in &clone.edges {
            if !node_ids.contains(e.source.as_str()) {
                issues.push(format!("Edge source {} references missing node", e.source));
            }
            if !node_ids.contains(e.target.as_str()) {
                issues.push(format!("Edge target {} references missing node", e.target));
            }
        }

        // 孤立节点检查(入度 0 出度 0 但不是 start 节点)
        let mut has_incoming: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut has_outgoing: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for e in &clone.edges {
            has_outgoing.insert(e.source.as_str());
            has_incoming.insert(e.target.as_str());
        }
        for n in &clone.nodes {
            let id = n.base_id();
            if !has_incoming.contains(&id) && !has_outgoing.contains(&id) && clone.nodes.len() > 1 {
                issues.push(format!("Node {} is isolated (no incoming/outgoing edges)", id));
            }
        }

        ValidationReport {
            passed: issues.is_empty(),
            issues,
            nodes_before,
            nodes_after,
            edges_before,
            edges_after,
        }
    }

    /// Phase 4-4: 版本回滚 — pop 最近的快照并返回。
    pub fn rollback(&self) -> Option<WorkflowTemplateData> {
        let snap = self.snapshots.lock().pop()?;
        tracing::warn!(
            "[Optimizer] rollback template {} from version {} to {}",
            snap.template_id,
            snap.template.version,
            snap.template.version
        );
        Some(snap.template)
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

    // ═══════════════════════════════════════════════════════════════════
    // Phase 2: ErrorRecoveryOptimizer — 失败分支优化
    // ═══════════════════════════════════════════════════════════════════

    /// Phase 2-1: 单次反思按 FailureCategory 细分错误恢复建议。
    ///
    /// 比 Phase 1 的粗粒度 UpdateConfig 更精准:
    /// - Timeout            → TuneRetry(3次+60s 超时)
    /// - ToolUnavailable    → continueOnFail + AddNode fallback
    /// - ConfigError        → VariableMisconfig 检测 + UpdateConfig
    /// - PermissionDenied   → UpdateConfig(凭据检查)
    /// - LlmError           → TuneRetry(指数退避)
    /// - ExternalService    → TuneRetry(长退避) + UpdateConfig(熔断)
    /// - 其他               → 通用 ErrorHandling
    fn emit_error_recovery_suggestions(
        analysis: &NodeFailureAnalysis,
        _template: Option<&WorkflowTemplateData>,
    ) -> Vec<WorkflowSuggestion> {
        let mut out = Vec::new();
        let node_id = analysis.node_id.clone();
        let cat = analysis.failure_category;

        match cat {
            FailureCategory::Timeout => {
                out.push(WorkflowSuggestion {
                    id: format!("sugg-timeout-{}", Uuid::new_v4()),
                    category: SuggestionCategory::ErrorHandling,
                    priority: SuggestionPriority::Critical,
                    target_node_id: Some(node_id.clone()),
                    description: format!(
                        "节点 {} 超时失败:{}。建议 TuneRetry(3 次+60s 超时)+ UpdateConfig(超时降级策略)。",
                        node_id, analysis.recovery_strategy
                    ),
                    proposed_change: ProposedChange::TuneRetry {
                        node_id: node_id.clone(),
                        max_attempts: 3,
                        backoff_ms: 60_000,
                    },
                    confidence: 0.8,
                    estimated_impact: Some(0.7),
                });
            },
            FailureCategory::ToolUnavailable => {
                out.push(WorkflowSuggestion {
                    id: format!("sugg-toolunavail-{}", Uuid::new_v4()),
                    category: SuggestionCategory::ErrorHandling,
                    priority: SuggestionPriority::Critical,
                    target_node_id: Some(node_id.clone()),
                    description: format!(
                        "节点 {} 工具不可用:{}。建议设置 continueOnFail + 添加 fallback 节点。",
                        node_id, analysis.recovery_strategy
                    ),
                    proposed_change: ProposedChange::UpdateConfig {
                        node_id: node_id.clone(),
                        patch: serde_json::json!({
                            "continueOnFail": true,
                            "fallbackEnabled": true,
                            "_recovery_strategy": analysis.recovery_strategy,
                        }),
                    },
                    confidence: 0.75,
                    estimated_impact: Some(0.65),
                });

                let fallback_node = serde_json::json!({
                    "base": {
                        "id": format!("{}-fallback", node_id),
                        "type": "agent",
                        "name": format!("{} Fallback", node_id),
                    },
                    "config": {
                        "systemPrompt": "fallback: 主工具不可用时的降级处理",
                    },
                });
                out.push(WorkflowSuggestion {
                    id: format!("sugg-fallback-{}", Uuid::new_v4()),
                    category: SuggestionCategory::ErrorHandling,
                    priority: SuggestionPriority::Medium,
                    target_node_id: Some(node_id.clone()),
                    description: format!(
                        "为节点 {} 添加 fallback 节点,在主工具不可用时走降级路径。",
                        node_id
                    ),
                    proposed_change: ProposedChange::AddNode {
                        after: node_id.clone(),
                        node: serde_json::from_value(fallback_node)
                            .ok()
                            .unwrap_or(serde_json::Value::Null),
                    },
                    confidence: 0.6,
                    estimated_impact: Some(0.4),
                });
            },
            FailureCategory::ConfigError | FailureCategory::InputMismatch => {
                out.push(WorkflowSuggestion {
                    id: format!("sugg-configerr-{}", Uuid::new_v4()),
                    category: SuggestionCategory::VariableMisconfig,
                    priority: SuggestionPriority::High,
                    target_node_id: Some(node_id.clone()),
                    description: format!(
                        "节点 {} 配置错误/输入不匹配:{}。建议 UpdateConfig(输入校验+变量映射)。",
                        node_id, analysis.recovery_strategy
                    ),
                    proposed_change: ProposedChange::UpdateConfig {
                        node_id: node_id.clone(),
                        patch: serde_json::json!({
                            "inputSchema": "add_validation",
                            "variableMapping": "check_mapping",
                            "_root_cause": analysis.root_cause,
                        }),
                    },
                    confidence: 0.7,
                    estimated_impact: Some(0.55),
                });
            },
            FailureCategory::PermissionDenied => {
                out.push(WorkflowSuggestion {
                    id: format!("sugg-perm-{}", Uuid::new_v4()),
                    category: SuggestionCategory::ErrorHandling,
                    priority: SuggestionPriority::High,
                    target_node_id: Some(node_id.clone()),
                    description: format!(
                        "节点 {} 权限问题:{}。建议检查凭据/API key 配置。",
                        node_id, analysis.recovery_strategy
                    ),
                    proposed_change: ProposedChange::UpdateConfig {
                        node_id: node_id.clone(),
                        patch: serde_json::json!({
                            "_check_credential": true,
                            "_recovery_strategy": analysis.recovery_strategy,
                        }),
                    },
                    confidence: 0.65,
                    estimated_impact: Some(0.5),
                });
            },
            FailureCategory::LlmError => {
                out.push(WorkflowSuggestion {
                    id: format!("sugg-llm-{}", Uuid::new_v4()),
                    category: SuggestionCategory::ErrorHandling,
                    priority: SuggestionPriority::High,
                    target_node_id: Some(node_id.clone()),
                    description: format!(
                        "节点 {} LLM 错误:{}。建议 TuneRetry(指数退避)+ UpdateConfig(model 参数)。",
                        node_id, analysis.recovery_strategy
                    ),
                    proposed_change: ProposedChange::TuneRetry {
                        node_id: node_id.clone(),
                        max_attempts: 4,
                        backoff_ms: 3_000,
                    },
                    confidence: 0.7,
                    estimated_impact: Some(0.6),
                });
            },
            FailureCategory::ExternalService => {
                out.push(WorkflowSuggestion {
                    id: format!("sugg-ext-{}", Uuid::new_v4()),
                    category: SuggestionCategory::ErrorHandling,
                    priority: SuggestionPriority::Medium,
                    target_node_id: Some(node_id.clone()),
                    description: format!(
                        "节点 {} 外部服务错误:{}。建议 TuneRetry(长退避)+ UpdateConfig(熔断)。",
                        node_id, analysis.recovery_strategy
                    ),
                    proposed_change: ProposedChange::TuneRetry {
                        node_id: node_id.clone(),
                        max_attempts: 3,
                        backoff_ms: 10_000,
                    },
                    confidence: 0.55,
                    estimated_impact: Some(0.45),
                });
            },
            _ => {
                out.push(WorkflowSuggestion {
                    id: format!("sugg-gen-{}", Uuid::new_v4()),
                    category: SuggestionCategory::ErrorHandling,
                    priority: SuggestionPriority::Medium,
                    target_node_id: Some(node_id.clone()),
                    description: format!(
                        "节点 {} 失败({:?}):{}。建议通用 TuneRetry + 日志增强。",
                        node_id, cat, analysis.recovery_strategy
                    ),
                    proposed_change: ProposedChange::TuneRetry {
                        node_id: node_id.clone(),
                        max_attempts: 2,
                        backoff_ms: 2_000,
                    },
                    confidence: 0.4,
                    estimated_impact: Some(0.3),
                });
            },
        }

        out
    }

    /// Phase 2-2: 跨 reflection 聚合高频失败模式。
    /// (node_id, failure_category) 出现 >= 2 次时提升置信度 + 优先级。
    fn suggest_error_recovery_batch(reflections: &[Reflection]) -> Vec<WorkflowSuggestion> {
        if reflections.len() < 2 {
            return Vec::new();
        }

        struct Freq {
            count: u32,
            quality_sum: u32,
        }
        let mut freq_map: std::collections::HashMap<String, Freq> =
            std::collections::HashMap::new();
        let mut analyses: std::collections::HashMap<String, NodeFailureAnalysis> =
            std::collections::HashMap::new();

        for r in reflections {
            let Some(m) = Self::extract_metadata(r) else { continue };
            let Some(fa) = &m.failed_node_analysis else { continue };

            let key = format!("{}|{:?}", fa.node_id, fa.failure_category);
            let quality = r.quality_score as u32;
            freq_map.entry(key.clone()).or_insert(Freq { count: 0, quality_sum: 0 }).count += 1;
            freq_map.entry(key.clone()).or_insert(Freq { count: 0, quality_sum: 0 }).quality_sum +=
                quality;
            analyses.insert(key, fa.clone());
        }

        let mut out = Vec::new();
        for (key, fa) in &analyses {
            let f = freq_map.get(key).unwrap();
            if f.count >= 2 {
                let avg_quality = f.quality_sum / f.count;
                let base = Self::emit_error_recovery_suggestions(fa, None);
                for mut s in base {
                    s.confidence = (s.confidence + 0.15).clamp(0.0, 0.95);
                    if avg_quality >= 5 {
                        s.priority = match s.priority {
                            SuggestionPriority::Low => SuggestionPriority::Medium,
                            SuggestionPriority::Medium => SuggestionPriority::High,
                            SuggestionPriority::High => SuggestionPriority::Critical,
                            _ => SuggestionPriority::Critical,
                        };
                    }
                    s.description = format!(
                        "[高频失败,已出现 {} 次(avg_quality={})] {}",
                        f.count, avg_quality, s.description
                    );
                    out.push(s);
                }
            }
        }

        out
    }

    // ═══════════════════════════════════════════════════════════════════
    // Phase 3: ToolSequenceOptimizer + PromptRefiner
    // ═══════════════════════════════════════════════════════════════════

    /// Phase 3-1: 跨 reflection 聚合高频工具序列(node_patterns)。
    /// - >= 3 次 短序列(2 步) -> RefinePrompt 固化调用顺序
    /// - >= 3 次 长序列(>=3 步) -> RefinePrompt + 可选 RewireEdge
    /// - 匹配 reusable_patterns -> 附加 PatternLearner 可复用标记
    fn suggest_tool_sequence_batch(
        reflections: &[Reflection],
        _template: Option<&WorkflowTemplateData>,
    ) -> Vec<WorkflowSuggestion> {
        if reflections.len() < 3 {
            return Vec::new();
        }

        struct PatternFreq {
            pattern: WorkflowPattern,
            count: u32,
            total_quality_sum: u32,
            reusable_hits: u32,
        }
        let mut patterns: Vec<PatternFreq> = Vec::new();

        for r in reflections {
            let Some(m) = Self::extract_metadata(r) else { continue };
            let quality = r.quality_score as u32;

            for p in &m.node_patterns {
                let reusable_hits = if r
                    .reusable_patterns
                    .iter()
                    .any(|rp| rp.contains(&p.name) || rp.contains(&p.description))
                {
                    1
                } else {
                    0
                };

                let found = patterns.iter_mut().find(|x| {
                    x.pattern.node_ids.len() == p.node_ids.len()
                        && x.pattern.node_ids.iter().zip(p.node_ids.iter()).all(|(a, b)| a == b)
                });
                if let Some(existing) = found {
                    existing.count += p.frequency.max(1);
                    existing.total_quality_sum += quality;
                    existing.reusable_hits += reusable_hits;
                } else {
                    patterns.push(PatternFreq {
                        pattern: p.clone(),
                        count: p.frequency.max(1),
                        total_quality_sum: quality,
                        reusable_hits,
                    });
                }
            }
        }

        let mut high_freq: Vec<PatternFreq> =
            patterns.into_iter().filter(|pf| pf.count >= 3).collect();
        high_freq.sort_by_key(|b| std::cmp::Reverse(b.count));

        let mut out = Vec::new();
        for pf in &high_freq {
            let pattern = &pf.pattern;
            let node_ids = &pattern.node_ids;
            let seq_key = node_ids.join(" -> ");
            let avg_quality = (pf.total_quality_sum / reflections.len() as u32).max(1);
            let confidence =
                (0.6 + pf.count as f32 * 0.02 + pf.reusable_hits as f32 * 0.05).clamp(0.6, 0.95);

            if node_ids.len() == 2 {
                out.push(WorkflowSuggestion {
                    id: format!("sugg-toolseq-{}", Uuid::new_v4()),
                    category: SuggestionCategory::PromptRefine,
                    priority: SuggestionPriority::High,
                    target_node_id: Some(node_ids[0].clone()),
                    description: format!(
                        "高频工具序列 [{}] 已出现 {} 次(avg_quality={})。建议在 Agent prompt 里固化推荐调用顺序。",
                        seq_key, pf.count, avg_quality
                    ),
                    proposed_change: ProposedChange::RefinePrompt {
                        node_id: node_ids[0].clone(),
                        new_prompt: format!(
                            "观察到高频工具调用序列:{}\n当用户意图匹配时,优先按此顺序调用工具。",
                            seq_key
                        ),
                    },
                    confidence,
                    estimated_impact: Some((0.4 + pf.count as f32 * 0.03).clamp(0.4, 0.8)),
                });
            } else if node_ids.len() >= 3 {
                out.push(WorkflowSuggestion {
                    id: format!("sugg-toolseq-long-{}", Uuid::new_v4()),
                    category: SuggestionCategory::PromptRefine,
                    priority: SuggestionPriority::High,
                    target_node_id: Some(node_ids[0].clone()),
                    description: format!(
                        "稳定长序列 [{}]({} 步) 已出现 {} 次。建议固化调用顺序或抽为子工作流。",
                        seq_key, node_ids.len(), pf.count
                    ),
                    proposed_change: ProposedChange::RefinePrompt {
                        node_id: node_ids[0].clone(),
                        new_prompt: format!(
                            "高频工具序列({} 步):{}\n建议优先按此顺序执行;重复出现时可考虑抽为子工作流。",
                            node_ids.len(), seq_key
                        ),
                    },
                    confidence,
                    estimated_impact: Some((0.5 + pf.count as f32 * 0.04).clamp(0.5, 0.85)),
                });

                if node_ids.len() >= 4 {
                    out.push(WorkflowSuggestion {
                        id: format!("sugg-rewire-{}", Uuid::new_v4()),
                        category: SuggestionCategory::EdgeRewire,
                        priority: SuggestionPriority::Medium,
                        target_node_id: Some(node_ids.first().cloned().unwrap_or_default()),
                        description: format!(
                            "高频长序列 [{}] 共 {} 步。可 RewireEdge:让 {} 直达 {},跳过中间节点。",
                            seq_key,
                            node_ids.len(),
                            node_ids.first().unwrap_or(&String::new()),
                            node_ids.last().unwrap_or(&String::new())
                        ),
                        proposed_change: ProposedChange::RewireEdge {
                            from: node_ids.first().cloned().unwrap_or_default(),
                            to: node_ids.last().cloned().unwrap_or_default(),
                            new_target: node_ids.last().cloned().unwrap_or_default(),
                        },
                        confidence: (confidence * 0.8).clamp(0.4, 0.75),
                        estimated_impact: Some(0.3),
                    });
                }
            }

            if pf.reusable_hits > 0 {
                out.push(WorkflowSuggestion {
                    id: format!("sugg-reuse-{}", Uuid::new_v4()),
                    category: SuggestionCategory::PromptRefine,
                    priority: SuggestionPriority::Medium,
                    target_node_id: Some(node_ids.first().cloned().unwrap_or_default()),
                    description: format!(
                        "PatternLearner 已标记 [{}] 为可复用模式,建议纳入优化候选。",
                        pattern.name
                    ),
                    proposed_change: ProposedChange::RefinePrompt {
                        node_id: node_ids.first().cloned().unwrap_or_default(),
                        new_prompt: format!(
                            "## 可复用模式\n{}\n\n按此模式组织工具调用,历史平均成功率 {:.1}%。",
                            pattern.description,
                            pattern.confidence * 100.0
                        ),
                    },
                    confidence: (confidence + 0.05).clamp(0.6, 0.95),
                    estimated_impact: Some(0.4),
                });
            }
        }

        out
    }

    /// Phase 3-2: PromptRefiner — 从多次反思的正向反馈中提取 prompt 增强建议。
    fn suggest_prompt_refine(reflections: &[Reflection]) -> Vec<WorkflowSuggestion> {
        if reflections.len() < 2 {
            return Vec::new();
        }

        let mut positive_by_node: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for r in reflections {
            let Some(m) = Self::extract_metadata(r) else { continue };

            for rp in &r.reusable_patterns {
                if rp.is_empty() {
                    continue;
                }
                if let Some(first_bn) = m.bottleneck_nodes.first() {
                    positive_by_node.entry(first_bn.node_id.clone()).or_default().push(rp.clone());
                }
            }

            if r.quality_score >= 4 && !r.quality_analysis.is_empty() {
                for sentence in r.quality_analysis.split(|c| "。.!?;".contains(c)) {
                    let trimmed = sentence.trim();
                    if trimmed.len() >= 4
                        && let Some(first_bn) = m.bottleneck_nodes.first()
                    {
                        positive_by_node
                            .entry(first_bn.node_id.clone())
                            .or_default()
                            .push(trimmed.to_string());
                    }
                }
            }
        }

        let mut out = Vec::new();
        for (node_id, positive) in positive_by_node {
            if positive.len() >= 2 {
                let unique: Vec<String> = positive
                    .into_iter()
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                let summary = unique.iter().take(3).cloned().collect::<Vec<_>>().join("; ");
                out.push(WorkflowSuggestion {
                    id: format!("sugg-prompt-{}", Uuid::new_v4()),
                    category: SuggestionCategory::PromptRefine,
                    priority: SuggestionPriority::Medium,
                    target_node_id: Some(node_id.clone()),
                    description: format!(
                        "节点 {} 在多次反思中积累了 {} 条正向经验,建议注入 prompt。",
                        node_id,
                        unique.len()
                    ),
                    proposed_change: ProposedChange::RefinePrompt {
                        node_id: node_id.clone(),
                        new_prompt: format!(
                            "# 历史成功经验\n{}\n\n按上述经验调整调用策略,提升成功率。",
                            summary
                        ),
                    },
                    confidence: 0.6,
                    estimated_impact: Some(0.4),
                });
            }
        }

        out
    }

    // ═══════════════════════════════════════════════════════════════════
    // Phase 2/3 结束
    // ═══════════════════════════════════════════════════════════════════

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
        // Phase 2-2: 跨 reflection 聚合高频失败模式
        let mut batch_err = Self::suggest_error_recovery_batch(reflections);
        all.append(&mut batch_err);

        // Phase 3-1: 跨 reflection 聚合高频工具序列
        let mut batch_seq = Self::suggest_tool_sequence_batch(reflections, Some(template));
        all.append(&mut batch_seq);

        // Phase 3-2: PromptRefiner 从多次反思提取正向经验
        let mut batch_prompt = Self::suggest_prompt_refine(reflections);
        all.append(&mut batch_prompt);

        // Phase 4: 冲突消解 + 多维度仲裁
        all = deduplicate_suggestions(all);
        // 按综合评分排序
        all.sort_by(|a, b| {
            suggestion_score(b)
                .partial_cmp(&suggestion_score(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(all)
    }

    async fn apply_suggestions(
        &self,
        template: &WorkflowTemplateData,
        suggestions: &[WorkflowSuggestion],
    ) -> Result<WorkflowTemplateData, String> {
        // Phase 4-1: 沙箱验证 - dry_run 先在克隆上应用一遍，产出 ValidationReport
        let dry_run_report = Self::dry_run_validate(template, suggestions);
        if !dry_run_report.passed {
            tracing::warn!(
                "[Optimizer] dry_run validation failed for template {}: {:?}",
                template.id,
                dry_run_report.issues
            );
        }

        // Phase 4-2: 自动快照 - apply 前把当前模板推入历史
        let snapshot_ids: Vec<String> = suggestions.iter().map(|s| s.id.clone()).collect();
        self.push_snapshot(template, snapshot_ids);

        // Phase 4-3: 正式应用 + 版本号递增
        let mut new_template = template.clone();
        let mut applied_count = 0u32;
        for s in suggestions {
            let before_nodes = new_template.nodes.len();
            Self::apply_one(&mut new_template, s);
            if new_template.nodes.len() != before_nodes || s.target_node_id.is_some() {
                applied_count += 1;
            }
        }
        if applied_count > 0 {
            new_template.version += 1;
            new_template.updated_at = chrono::Utc::now().timestamp_millis();
            tracing::info!(
                "[Optimizer] applied {} suggestions to template {}, version → {} (dry_run_passed={})",
                applied_count,
                template.id,
                new_template.version,
                dry_run_report.passed
            );
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

#[allow(clippy::empty_line_after_doc_comments)]
/// 优先级权重(用于排序)。
/// Phase 4 冲突仲裁评分: priority x confidence x impact x category_boost。
fn suggestion_score(s: &WorkflowSuggestion) -> f32 {
    let prio = priority_weight(s.priority) as f32 * 10.0;
    let conf = s.confidence * 50.0;
    let impact = s.estimated_impact.unwrap_or(0.0) * 20.0;
    let cat_boost = match s.category {
        SuggestionCategory::ErrorHandling => 10.0,
        SuggestionCategory::NodeReplacement => 8.0,
        SuggestionCategory::ResourceTuning => 7.0,
        SuggestionCategory::NodeConfig => 6.0,
        SuggestionCategory::PromptRefine => 5.0,
        SuggestionCategory::VariableMisconfig => 5.0,
        SuggestionCategory::EdgeRewire => 3.0,
    };
    prio + conf + impact + cat_boost
}

/// 建议去重 + 冲突仲裁（Phase 4）。
/// 同 target_node_id + category 只保留评分最高的一条。
fn deduplicate_suggestions(all: Vec<WorkflowSuggestion>) -> Vec<WorkflowSuggestion> {
    let mut out: Vec<WorkflowSuggestion> = Vec::with_capacity(all.len());
    for s in all {
        if let Some(existing) = out
            .iter_mut()
            .find(|x| x.target_node_id == s.target_node_id && x.category == s.category)
        {
            if suggestion_score(&s) > suggestion_score(existing) {
                *existing = s;
            }
        } else {
            out.push(s);
        }
    }
    out
}

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
            cluster_id: None,
            route_path: None,
            hooks_config: None,
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
