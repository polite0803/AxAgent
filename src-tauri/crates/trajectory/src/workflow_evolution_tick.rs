// SPDX-License-Identifier: AGPL-3.0-only

//! Phase 5:`WorkflowEvolutionTick` 后台定时任务 — 把 Phase 1-4 的
//! `WorkflowOptimizer` 算法接上自动触发入口。
//!
//! 数据流:
//! ```text
//! tick_interval(默认 6h)
//!   └─▶ 遍历 WorkflowTemplateRepo.list_templates()
//!        └─▶ TrajectoryStorage.get_workflow_reflections(template_id, limit)
//!             └─▶ ≥ min_reflections 门槛?
//!                  ├─▶ NO → 跳过,记 tracing::debug
//!                  └─▶ YES → optimizer.suggest_batch(template, reflections)
//!                       └─▶ 过滤: score ≥ auto_apply_threshold
//!                            ├─▶ YES → optimizer.apply_suggestions → repo.save_template
//!                            └─▶ NO → 只产出建议,不自动写库
//! ```
//!
//! 设计约束:
//! - 不依赖 dao crate,持久化通过 harness `WorkflowTemplateRepo` trait 注入
//!   (wiring 层负责把 dao 的实现传进来)
//! - 幂等:同一模板同一批次 reflections 重复 tick 结果一致(版本号只在实际变更时 +1)
//! - 安全:单 tick 最多 auto_apply `max_auto_apply_per_tick` 条,避免一次改太多
//! - 优雅:tick 函数返回 `tokio::task::JoinHandle`,调用方可 await 或 abort

use std::sync::Arc;
use std::time::Duration;

use axagent_harness::WorkflowTemplateRepo;
use axagent_harness::workflow_optimization::{WorkflowOptimizer, WorkflowSuggestion};
use axagent_harness::workflow_types::WorkflowTemplateData;
use tokio::task::JoinHandle;

use crate::storage::TrajectoryStorage;

// ── 配置 ──

/// tick 配置(可由 wiring 层构造并传入)。
#[derive(Debug, Clone)]
pub struct EvolutionTickConfig {
    /// tick 间隔(默认 6h)。
    pub interval: Duration,
    /// 每个 template 读取的 reflection 数量(默认 20)。
    pub reflections_per_template: usize,
    /// 触发优化的最小反思数门槛(默认 3 — 单次 reflection 不足以支撑 Phase 2/3 批量策略)。
    pub min_reflections: usize,
    /// auto-apply 综合阈值(0.0 - 1.0,默认 0.7 — score ≥ 70% 才自动应用)。
    pub auto_apply_threshold: f32,
    /// 单次 tick 每个模板最多自动应用的建议数(默认 3,避免一次改太多)。
    pub max_auto_apply_per_template: usize,
    /// 单次 tick 全局最多自动应用的模板数(默认 5)。
    pub max_templates_per_tick: usize,
    /// dry-run 模式(不实际持久化,只产出建议并打 info 日志,默认 false)。
    pub dry_run: bool,
}

impl Default for EvolutionTickConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(6 * 60 * 60), // 6h
            reflections_per_template: 20,
            min_reflections: 3,
            auto_apply_threshold: 0.7,
            max_auto_apply_per_template: 3,
            max_templates_per_tick: 5,
            dry_run: false,
        }
    }
}

// ── 报告 ──

/// 单次 tick 执行结果摘要(供 tracing + GUI 展示)。
#[derive(Debug, Default, Clone)]
pub struct EvolutionTickReport {
    /// 本轮扫描的模板总数。
    pub templates_scanned: usize,
    /// 满足 min_reflections 门槛、实际触发了 suggest_batch 的模板数。
    pub templates_optimized: usize,
    /// 成功自动应用并持久化的模板数。
    pub templates_auto_applied: usize,
    /// 总共生成的建议数。
    pub total_suggestions: usize,
    /// 总共自动应用的建议数。
    pub auto_applied_suggestions: usize,
    /// 被 auto_apply_threshold 拦下(只产出建议、未写库)的模板数。
    pub templates_pending_review: usize,
    /// 本轮 tick 耗时(ms)。
    pub elapsed_ms: u64,
    /// 错误列表(模板级,key = template_id)。
    pub errors: Vec<(String, String)>,
}

// ── 入口 ──

/// 启动后台定时进化 tick(返回 JoinHandle,可 await / abort)。
///
/// 依赖注入:
/// - `storage`: trajectory 内部的 reflection + pattern 持久化
/// - `optimizer`: harness `WorkflowOptimizer` trait(实现为 trajectory 的 WorkflowOptimizerImpl)
/// - `template_repo`: harness `WorkflowTemplateRepo` trait(可选,None 时 tick 仍跑但不持久化)
pub fn start_workflow_evolution_tick(
    storage: Arc<TrajectoryStorage>,
    optimizer: Arc<dyn WorkflowOptimizer>,
    template_repo: Option<Arc<dyn WorkflowTemplateRepo>>,
    config: EvolutionTickConfig,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick_interval = tokio::time::interval(config.interval);

        // 首次 tick 延迟 30s,等启动完成
        tokio::time::sleep(Duration::from_secs(30)).await;

        loop {
            tick_interval.tick().await;

            let report =
                tick_once(&storage, optimizer.as_ref(), template_repo.as_deref(), &config).await;

            tracing::info!(
                "[EvolutionTick] done. scanned={} optimized={} auto_applied={} suggestions={} elapsed={}ms",
                report.templates_scanned,
                report.templates_optimized,
                report.templates_auto_applied,
                report.total_suggestions,
                report.elapsed_ms,
            );

            if !report.errors.is_empty() {
                tracing::warn!(
                    "[EvolutionTick] {} errors: {:?}",
                    report.errors.len(),
                    report.errors.iter().map(|(id, e)| format!("{}:{}", id, e)).collect::<Vec<_>>(),
                );
            }
        }
    })
}

// ── 单次 tick 核心逻辑 ──

/// 执行单次 tick(独立函数,方便单元测试 / 手动触发)。
pub async fn tick_once(
    storage: &TrajectoryStorage,
    optimizer: &dyn WorkflowOptimizer,
    template_repo: Option<&dyn WorkflowTemplateRepo>,
    config: &EvolutionTickConfig,
) -> EvolutionTickReport {
    let start = std::time::Instant::now();
    let mut report = EvolutionTickReport::default();

    // 没有 template_repo 就无法遍历模板,直接返回空报告
    let Some(repo) = template_repo else {
        tracing::debug!("[EvolutionTick] no template_repo injected, skipping");
        report.elapsed_ms = start.elapsed().as_millis() as u64;
        return report;
    };

    let templates = match repo.list_templates().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("[EvolutionTick] list_templates failed: {}", e);
            report.errors.push(("__all__".to_string(), e));
            report.elapsed_ms = start.elapsed().as_millis() as u64;
            return report;
        },
    };

    report.templates_scanned = templates.len();

    let mut auto_applied_count: usize = 0;

    for template in &templates {
        if auto_applied_count >= config.max_templates_per_tick {
            tracing::debug!(
                "[EvolutionTick] reached max_templates_per_tick({}), skipping remaining",
                config.max_templates_per_tick,
            );
            break;
        }

        // 跳过预设模板 / 不可编辑模板
        if template.is_preset || !template.is_editable {
            continue;
        }

        // 拉取 reflection
        let reflections = match storage
            .get_workflow_reflections(&template.id, config.reflections_per_template)
            .await
            .map_err(|e| e.to_string())
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    "[EvolutionTick] get_workflow_reflections({}) failed: {}",
                    template.id,
                    e,
                );
                report.errors.push((template.id.clone(), e.to_string()));
                continue;
            },
        };

        // 门槛检查 — Phase 2/3 批量策略需要至少 min_reflections 条
        if reflections.len() < config.min_reflections {
            tracing::debug!(
                "[EvolutionTick] template {} has {} reflections (need >= {}), skipping",
                template.id,
                reflections.len(),
                config.min_reflections,
            );
            continue;
        }

        // 跑优化 — suggest_batch 内部已包含 Phase 1-4 所有策略
        let all_suggestions = match optimizer.suggest_batch(template, &reflections).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("[EvolutionTick] suggest_batch({}) failed: {}", template.id, e,);
                report.errors.push((template.id.clone(), e));
                continue;
            },
        };

        report.templates_optimized += 1;
        report.total_suggestions += all_suggestions.len();

        if all_suggestions.is_empty() {
            tracing::debug!("[EvolutionTick] no suggestions for template {}", template.id);
            continue;
        }

        // 按 score 过滤,选 auto-apply 候选
        let auto_apply_candidates = select_auto_apply(&all_suggestions, config);

        if auto_apply_candidates.is_empty() {
            report.templates_pending_review += 1;
            tracing::debug!(
                "[EvolutionTick] template {} has {} suggestions but none passed auto-apply threshold ({})",
                template.id,
                all_suggestions.len(),
                config.auto_apply_threshold,
            );
            continue;
        }

        // 应用建议
        match optimizer.apply_suggestions(template, &auto_apply_candidates).await {
            Ok(mut new_template) => {
                // 版本号 +1
                new_template.version = new_template.version.saturating_add(1);

                if config.dry_run {
                    tracing::info!(
                        "[EvolutionTick][DRY-RUN] would apply {} suggestions to template {} (v{} → v{})",
                        auto_apply_candidates.len(),
                        template.id,
                        template.version,
                        new_template.version,
                    );
                } else {
                    // 持久化
                    if let Err(e) = repo.save_template(&new_template).await {
                        tracing::error!(
                            "[EvolutionTick] save_template({}) failed: {}",
                            template.id,
                            e,
                        );
                        report.errors.push((template.id.clone(), e));
                        continue;
                    }

                    tracing::info!(
                        "[EvolutionTick] auto-applied {} suggestions to template {} (v{} → v{})",
                        auto_apply_candidates.len(),
                        template.id,
                        template.version,
                        new_template.version,
                    );
                }

                report.templates_auto_applied += 1;
                report.auto_applied_suggestions += auto_apply_candidates.len();
                auto_applied_count += 1;
            },
            Err(e) => {
                tracing::warn!("[EvolutionTick] apply_suggestions({}) failed: {}", template.id, e,);
                report.errors.push((template.id.clone(), e));
            },
        }
    }

    report.elapsed_ms = start.elapsed().as_millis() as u64;
    report
}

// ── 辅助:score 计算 + 自动应用筛选 ──

/// 计算建议的自动应用综合得分(0.0 - 1.0)。
///
/// 公式同 Phase 4 `suggestion_score`,但做归一化到 [0, 1]:
/// score = (priority_weight × 10 + confidence × 50 + impact × 20 + category_boost) / 200
fn auto_apply_score(s: &WorkflowSuggestion) -> f32 {
    let prio = priority_weight(s.priority) as f32 * 10.0;
    let conf = s.confidence * 50.0;
    let impact = s.estimated_impact.unwrap_or(0.0) * 20.0;
    let cat_boost = match s.category {
        axagent_harness::workflow_optimization::SuggestionCategory::ErrorHandling => 10.0,
        axagent_harness::workflow_optimization::SuggestionCategory::NodeReplacement => 8.0,
        axagent_harness::workflow_optimization::SuggestionCategory::ResourceTuning => 7.0,
        axagent_harness::workflow_optimization::SuggestionCategory::NodeConfig => 6.0,
        axagent_harness::workflow_optimization::SuggestionCategory::PromptRefine => 5.0,
        axagent_harness::workflow_optimization::SuggestionCategory::VariableMisconfig => 5.0,
        axagent_harness::workflow_optimization::SuggestionCategory::EdgeRewire => 3.0,
    };
    (prio + conf + impact + cat_boost) / 120.0
}

fn priority_weight(p: axagent_harness::workflow_optimization::SuggestionPriority) -> u32 {
    use axagent_harness::workflow_optimization::SuggestionPriority;
    match p {
        SuggestionPriority::Critical => 4,
        SuggestionPriority::High => 3,
        SuggestionPriority::Medium => 2,
        SuggestionPriority::Low => 1,
    }
}

/// 过滤出 auto-apply 候选(score ≥ threshold) + 截断到 max_per_template。
fn select_auto_apply(
    suggestions: &[WorkflowSuggestion],
    config: &EvolutionTickConfig,
) -> Vec<WorkflowSuggestion> {
    let mut scored: Vec<(f32, &WorkflowSuggestion)> =
        suggestions.iter().map(|s| (auto_apply_score(s), s)).collect();

    // 分排序:score 降序
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    scored
        .into_iter()
        .filter(|(score, _)| *score >= config.auto_apply_threshold)
        .take(config.max_auto_apply_per_template)
        .map(|(_, s)| s.clone())
        .collect()
}

// ── 测试辅助:从外部注入的 Reflection 列表直接跑 tick 一轮 ──

/// 对单个模板直接跑一轮 tick(不扫全仓库)。
///
/// 供 wiring 层调用(手动触发 / 调试) — 跳过 template_repo.list_templates,
/// 直接使用调用方传入的 reflections。
pub async fn run_tick_for_template(
    storage: &TrajectoryStorage,
    optimizer: &dyn WorkflowOptimizer,
    template: &WorkflowTemplateData,
    config: &EvolutionTickConfig,
) -> Result<Option<WorkflowTemplateData>, String> {
    let reflections = storage
        .get_workflow_reflections(&template.id, config.reflections_per_template)
        .await
        .map_err(|e| e.to_string())?;

    if reflections.len() < config.min_reflections {
        return Ok(None);
    }

    let all = optimizer.suggest_batch(template, &reflections).await?;
    let candidates = select_auto_apply(&all, config);

    if candidates.is_empty() {
        return Ok(None);
    }

    let mut new_template = optimizer.apply_suggestions(template, &candidates).await?;
    new_template.version = new_template.version.saturating_add(1);

    Ok(Some(new_template))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::workflow_optimization::{
        ProposedChange, SuggestionCategory, SuggestionPriority, WorkflowSuggestion,
    };
    use uuid::Uuid;

    fn make_suggestion(
        priority: SuggestionPriority,
        confidence: f32,
        impact: Option<f32>,
        cat: SuggestionCategory,
    ) -> WorkflowSuggestion {
        WorkflowSuggestion {
            id: format!("sugg-{}", Uuid::new_v4()),
            category: cat,
            priority,
            target_node_id: Some("n1".to_string()),
            description: "test".into(),
            proposed_change: ProposedChange::TuneRetry {
                node_id: "n1".into(),
                max_attempts: 3,
                backoff_ms: 1000,
            },
            confidence,
            estimated_impact: impact,
        }
    }

    #[test]
    fn test_auto_apply_score_normalization() {
        // Critical + 高置信度 + 高影响 + ErrorHandling → 高分
        let s = make_suggestion(
            SuggestionPriority::Critical,
            0.95,
            Some(0.8),
            SuggestionCategory::ErrorHandling,
        );
        let score = auto_apply_score(&s);
        assert!(score > 0.80, "critical error-handling should score high, got {}", score);

        // Low + 低置信度 + EdgeRewire → 低分
        let s2 = make_suggestion(
            SuggestionPriority::Low,
            0.3,
            Some(0.1),
            SuggestionCategory::EdgeRewire,
        );
        let score2 = auto_apply_score(&s2);
        assert!(score2 < 0.30, "low edge-rewire should score low, got {}", score2);
    }

    #[test]
    fn test_select_auto_apply_threshold() {
        let suggestions = vec![
            make_suggestion(
                SuggestionPriority::Critical,
                0.9,
                Some(0.8),
                SuggestionCategory::ErrorHandling,
            ),
            make_suggestion(
                SuggestionPriority::Medium,
                0.6,
                Some(0.4),
                SuggestionCategory::PromptRefine,
            ),
            make_suggestion(
                SuggestionPriority::Low,
                0.3,
                Some(0.1),
                SuggestionCategory::EdgeRewire,
            ),
        ];
        let cfg = EvolutionTickConfig {
            auto_apply_threshold: 0.5,
            max_auto_apply_per_template: 10,
            ..Default::default()
        };

        let selected = select_auto_apply(&suggestions, &cfg);
        // threshold 0.5: 只挑高分的 —— Critical(0.9) 与 Medium(0.6) 入选，Low(0.3) 淘汰
        assert!(selected.len() >= 2, "最高的两个都应入选，实际 {} 个", selected.len());
    }

    #[test]
    fn test_select_auto_apply_respects_max_per_template() {
        let mut suggestions = Vec::new();
        for _ in 0..10 {
            suggestions.push(make_suggestion(
                SuggestionPriority::Critical,
                0.9,
                Some(0.8),
                SuggestionCategory::ErrorHandling,
            ));
        }
        let cfg = EvolutionTickConfig {
            auto_apply_threshold: 0.5,
            max_auto_apply_per_template: 3,
            ..Default::default()
        };

        let selected = select_auto_apply(&suggestions, &cfg);
        assert_eq!(selected.len(), 3, "should be capped at max_per_template");
    }
}
