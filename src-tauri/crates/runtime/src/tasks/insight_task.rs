// SPDX-License-Identifier: AGPL-3.0-only

//! InsightGeneratorTask — 学习洞察生成任务（后台周期执行）
//!
//! 在定时触发时，执行以下操作：
//! - 从 trajectory 存储读取最近的轨迹
//! - 分析成功率、质量分布、常见模式，生成趋势性洞察
//! - 调用 [`LearningInsightSystem::generate_daily_report`] 生成日报
//! - 把关键趋势作为 [`LearningInsight`] 写回 insight_system
//!
//! 与 `start_insight_generation` 的关系：
//! - `start_insight_generation` 从 `RealTimeLearning` 的反馈信号生成洞察（10 分钟一次）
//! - 本任务从轨迹存储的整体趋势生成洞察 + 日报（周期更长，默认 6 小时）
//! - 两者互补：前者关注"实时反馈"，后者关注"长期趋势"

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// 洞察生成任务执行上下文
#[derive(Default, Clone)]
pub struct InsightGeneratorTaskContext {
    /// 轨迹存储（读取近期轨迹分析趋势）
    pub trajectory_storage: Option<Arc<axagent_trajectory::TrajectoryStorage>>,
    /// 洞察系统（生成日报 + 写入趋势洞察）
    pub insight_system: Option<Arc<tokio::sync::RwLock<axagent_trajectory::LearningInsightSystem>>>,
}

/// 洞察生成任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightGeneratorTaskResult {
    /// 分析的轨迹数量
    pub trajectories_analyzed: usize,
    /// 成功轨迹数量
    pub success_count: usize,
    /// 失败轨迹数量
    pub failure_count: usize,
    /// 平均质量分数
    pub avg_quality: f64,
    /// 平均价值分数
    pub avg_value: f64,
    /// 写入的趋势洞察数量
    pub insights_written: usize,
    /// 是否生成日报
    pub daily_report_generated: bool,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
    /// 错误信息（如有）
    pub errors: Vec<String>,
}

/// 洞察生成任务执行器
pub struct InsightGeneratorTaskExecutor;

impl InsightGeneratorTaskExecutor {
    /// 执行洞察生成任务并返回结果
    ///
    /// 若 `INSIGHT_GENERATOR_TASK` feature flag 未启用，直接返回空结果。
    pub async fn execute(ctx: &InsightGeneratorTaskContext) -> InsightGeneratorTaskResult {
        // 检查 INSIGHT_GENERATOR_TASK feature flag
        if !axagent_runtime_core::feature_flags::global_feature_flags()
            .insight_generator_task_sync()
        {
            tracing::warn!(
                "InsightGeneratorTask 未启用，跳过执行（设置 AXAGENT_FF_INSIGHT_GENERATOR_TASK=1 或 features.InsightGeneratorTask=true）"
            );
            return InsightGeneratorTaskResult {
                trajectories_analyzed: 0,
                success_count: 0,
                failure_count: 0,
                avg_quality: 0.0,
                avg_value: 0.0,
                insights_written: 0,
                daily_report_generated: false,
                duration_ms: 0,
                errors: vec![],
            };
        }

        let start = std::time::Instant::now();
        let mut result = InsightGeneratorTaskResult {
            trajectories_analyzed: 0,
            success_count: 0,
            failure_count: 0,
            avg_quality: 0.0,
            avg_value: 0.0,
            insights_written: 0,
            daily_report_generated: false,
            duration_ms: 0,
            errors: Vec::new(),
        };

        // 1. 拉取近期轨迹
        let storage = match &ctx.trajectory_storage {
            Some(s) => s,
            None => {
                tracing::warn!("[InsightGeneratorTask] 跳过：未提供 trajectory_storage");
                result.errors.push("跳过：未提供 trajectory_storage".to_string());
                result.duration_ms = start.elapsed().as_millis() as u64;
                return result;
            },
        };
        let trajectories = match storage.get_trajectories(Some(50)).await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("[InsightGeneratorTask] 拉取轨迹失败: {}", e);
                result.errors.push(format!("拉取轨迹失败: {e}"));
                result.duration_ms = start.elapsed().as_millis() as u64;
                return result;
            },
        };
        if trajectories.is_empty() {
            tracing::info!("[InsightGeneratorTask] 无近期轨迹，跳过本轮");
            result.duration_ms = start.elapsed().as_millis() as u64;
            return result;
        }

        // 2. 计算趋势指标
        result.trajectories_analyzed = trajectories.len();
        result.success_count = trajectories
            .iter()
            .filter(|t| matches!(t.outcome, axagent_trajectory::TrajectoryOutcome::Success))
            .count();
        result.failure_count = trajectories
            .iter()
            .filter(|t| matches!(t.outcome, axagent_trajectory::TrajectoryOutcome::Failure))
            .count();
        result.avg_quality =
            trajectories.iter().map(|t| t.quality.overall).sum::<f64>() / trajectories.len() as f64;
        result.avg_value =
            trajectories.iter().map(|t| t.value_score).sum::<f64>() / trajectories.len() as f64;

        tracing::info!(
            "[InsightGeneratorTask] 分析 {} 条轨迹: 成功 {}, 失败 {}, 平均质量 {:.2}, 平均价值 {:.2}",
            result.trajectories_analyzed,
            result.success_count,
            result.failure_count,
            result.avg_quality,
            result.avg_value,
        );

        // 3. 生成趋势洞察 + 日报
        if let Some(insight_system_arc) = &ctx.insight_system {
            let mut is = insight_system_arc.write().await;
            let now_ms = chrono::Utc::now().timestamp_millis();
            let success_rate = result.success_count as f64 / result.trajectories_analyzed as f64;

            // 3.1 成功率趋势洞察
            if success_rate < 0.4 && result.trajectories_analyzed >= 5 {
                is.add_insight(axagent_trajectory::LearningInsight {
                    id: format!("insight_trend_low_{}", now_ms),
                    category: axagent_trajectory::InsightCategory::Warning,
                    title: format!("成功率偏低: {:.0}%", success_rate * 100.0),
                    description: format!(
                        "近期 {} 条轨迹成功率仅 {:.0}%（成功 {} / 失败 {}）。平均质量 {:.2}，平均价值 {:.2}。建议检查近期工具调用或技能配置。",
                        result.trajectories_analyzed,
                        success_rate * 100.0,
                        result.success_count,
                        result.failure_count,
                        result.avg_quality,
                        result.avg_value,
                    ),
                    confidence: (1.0 - success_rate).min(0.9),
                    evidence: vec![format!("sample_size={}", result.trajectories_analyzed)],
                    suggested_action: Some(
                        "审查失败轨迹的 tool_results，定位高频失败工具".to_string(),
                    ),
                    created_at: now_ms,
                });
                result.insights_written += 1;
            } else if success_rate > 0.8 && result.trajectories_analyzed >= 5 {
                is.add_insight(axagent_trajectory::LearningInsight {
                    id: format!("insight_trend_high_{}", now_ms),
                    category: axagent_trajectory::InsightCategory::Improvement,
                    title: format!("成功率优秀: {:.0}%", success_rate * 100.0),
                    description: format!(
                        "近期 {} 条轨迹成功率 {:.0}%，平均质量 {:.2}。可考虑提升协同进化难度以保持挑战性。",
                        result.trajectories_analyzed,
                        success_rate * 100.0,
                        result.avg_quality,
                    ),
                    confidence: success_rate,
                    evidence: vec![format!("sample_size={}", result.trajectories_analyzed)],
                    suggested_action: Some("提升 CoevolutionEnvironment 难度基线".to_string()),
                    created_at: now_ms,
                });
                result.insights_written += 1;
            }

            // 3.2 质量分布洞察
            if result.avg_quality < 0.4 && result.trajectories_analyzed >= 5 {
                is.add_insight(axagent_trajectory::LearningInsight {
                    id: format!("insight_quality_low_{}", now_ms),
                    category: axagent_trajectory::InsightCategory::Warning,
                    title: format!("轨迹质量偏低: {:.2}", result.avg_quality),
                    description: format!(
                        "近期 {} 条轨迹平均质量分数 {:.2}（低于 0.4 阈值）。可能原因：工具调用冗余、上下文过长、技能匹配不准。",
                        result.trajectories_analyzed,
                        result.avg_quality,
                    ),
                    confidence: (0.5 - result.avg_quality).clamp(0.1, 0.9),
                    evidence: vec![],
                    suggested_action: Some("触发技能进化或上下文压缩".to_string()),
                    created_at: now_ms,
                });
                result.insights_written += 1;
            }

            // 3.3 生成日报（汇总近期洞察）
            let report = is.generate_daily_report();
            result.daily_report_generated = true;
            tracing::info!(
                "[InsightGeneratorTask] 生成日报: {} 个章节, {} 条建议, 摘要='{}'",
                report.sections.len(),
                report.recommendations.len(),
                report.summary,
            );

            tracing::info!(
                "[InsightGeneratorTask] 写入 {} 条趋势洞察, 日报已生成",
                result.insights_written
            );
        } else {
            tracing::warn!("[InsightGeneratorTask] 未提供 insight_system，跳过洞察写入");
            result.errors.push("跳过洞察写入：未提供 insight_system".to_string());
        }

        result.duration_ms = start.elapsed().as_millis() as u64;
        result
    }
}
