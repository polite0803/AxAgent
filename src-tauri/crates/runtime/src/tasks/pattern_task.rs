// SPDX-License-Identifier: AGPL-3.0-only

//! PatternAnalyzerTask — 跨会话模式分析任务（后台周期执行）
//!
//! 在定时触发时，执行以下操作：
//! - 从 trajectory 存储读取最近的会话轨迹
//! - 调用 [`axagent_trajectory::analyze_trajectories`] 提取跨会话行为模式
//!   （代码风格 / 时间分布 / 工具偏好 / 主题）
//! - 把关键发现作为 [`LearningInsight`] 写回 insight_system
//!
//! 与 `start_pattern_learning` 的关系：
//! - `start_pattern_learning` 用 `PatternLearner`（pattern.rs）从 Trajectory
//!   学习 `TrajectoryPattern`，结果是可持久化的模式记录（含 success_rate）
//! - 本任务用 `PatternAnalyzer`（pattern_analyzer.rs）从 Trajectory 提取
//!   更细粒度的用户行为模式（代码风格 / 工具偏好 / 时间分布），用于
//!   丰富用户画像与生成行为洞察
//! - 两者互补：前者关注"任务级模式"，后者关注"用户行为模式"

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// 模式分析任务执行上下文
#[derive(Default, Clone)]
pub struct PatternAnalyzerTaskContext {
    /// 轨迹存储（读取近期轨迹）
    pub trajectory_storage: Option<Arc<axagent_trajectory::TrajectoryStorage>>,
    /// 洞察系统（写入行为模式洞察）
    pub insight_system: Option<Arc<tokio::sync::RwLock<axagent_trajectory::LearningInsightSystem>>>,
}

/// 模式分析任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnalyzerTaskResult {
    /// 分析的轨迹数量
    pub trajectories_analyzed: usize,
    /// 转换并喂给 PatternAnalyzer 的 BehaviorEvent 总数
    pub total_events_analyzed: usize,
    /// 提取的代码风格模式数量
    pub coding_patterns_count: usize,
    /// 提取的时间分布模式数量
    pub temporal_patterns_count: usize,
    /// 提取的工具偏好模式数量
    pub tool_preference_patterns_count: usize,
    /// 提取的主题模式数量
    pub topic_patterns_count: usize,
    /// 写入 insight_system 的洞察数量
    pub insights_written: usize,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
    /// 错误信息（如有）
    pub errors: Vec<String>,
}

/// 模式分析任务执行器
pub struct PatternAnalyzerTaskExecutor;

impl PatternAnalyzerTaskExecutor {
    /// 执行模式分析任务并返回结果
    ///
    /// 若 `PATTERN_ANALYZER_TASK` feature flag 未启用，直接返回空结果。
    pub async fn execute(ctx: &PatternAnalyzerTaskContext) -> PatternAnalyzerTaskResult {
        // 检查 PATTERN_ANALYZER_TASK feature flag
        if !axagent_runtime_core::feature_flags::global_feature_flags().pattern_analyzer_task_sync()
        {
            tracing::warn!(
                "PatternAnalyzerTask 未启用，跳过执行（设置 AXAGENT_FF_PATTERN_ANALYZER_TASK=1 或 features.PatternAnalyzerTask=true）"
            );
            return PatternAnalyzerTaskResult {
                trajectories_analyzed: 0,
                total_events_analyzed: 0,
                coding_patterns_count: 0,
                temporal_patterns_count: 0,
                tool_preference_patterns_count: 0,
                topic_patterns_count: 0,
                insights_written: 0,
                duration_ms: 0,
                errors: vec![],
            };
        }

        let start = std::time::Instant::now();
        let mut result = PatternAnalyzerTaskResult {
            trajectories_analyzed: 0,
            total_events_analyzed: 0,
            coding_patterns_count: 0,
            temporal_patterns_count: 0,
            tool_preference_patterns_count: 0,
            topic_patterns_count: 0,
            insights_written: 0,
            duration_ms: 0,
            errors: Vec::new(),
        };

        // 1. 拉取近期轨迹
        let storage = match &ctx.trajectory_storage {
            Some(s) => s,
            None => {
                tracing::warn!("[PatternAnalyzerTask] 跳过：未提供 trajectory_storage");
                result.errors.push("跳过：未提供 trajectory_storage".to_string());
                result.duration_ms = start.elapsed().as_millis() as u64;
                return result;
            },
        };
        let trajectories = match storage.get_trajectories(Some(30)).await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("[PatternAnalyzerTask] 拉取轨迹失败: {}", e);
                result.errors.push(format!("拉取轨迹失败: {e}"));
                result.duration_ms = start.elapsed().as_millis() as u64;
                return result;
            },
        };
        if trajectories.is_empty() {
            tracing::info!("[PatternAnalyzerTask] 无近期轨迹，跳过本轮");
            result.duration_ms = start.elapsed().as_millis() as u64;
            return result;
        }

        // 2. 调用 PatternAnalyzer 提取模式
        let summary = axagent_trajectory::analyze_trajectories(&trajectories);
        result.trajectories_analyzed = summary.trajectories_analyzed;
        result.total_events_analyzed = summary.total_events_analyzed;
        result.coding_patterns_count = summary.coding_patterns.len();
        result.temporal_patterns_count = summary.temporal_patterns.len();
        result.tool_preference_patterns_count = summary.tool_preference_patterns.len();
        result.topic_patterns_count = summary.topic_patterns.len();

        tracing::info!(
            "[PatternAnalyzerTask] 分析 {} 条轨迹 ({} events): 代码模式={}, 时间模式={}, 工具偏好={}, 主题={}",
            summary.trajectories_analyzed,
            summary.total_events_analyzed,
            summary.coding_patterns.len(),
            summary.temporal_patterns.len(),
            summary.tool_preference_patterns.len(),
            summary.topic_patterns.len(),
        );

        // 3. 把关键发现转为 LearningInsight 写入 insight_system
        if let Some(insight_system_arc) = &ctx.insight_system {
            let mut is = insight_system_arc.write().await;
            let now_ms = chrono::Utc::now().timestamp_millis();

            // 3.1 代码风格模式洞察
            if !summary.coding_patterns.is_empty() {
                let top = &summary.coding_patterns[0];
                is.add_insight(axagent_trajectory::LearningInsight {
                    id: format!("pattern_code_{}", now_ms),
                    category: axagent_trajectory::InsightCategory::Pattern,
                    title: format!("代码风格模式: {} = {}", top.pattern_type, top.value),
                    description: format!(
                        "在 {} 条轨迹中检测到代码风格模式 '{}' (置信度 {:.2}, 出现 {} 次)。共提取 {} 个代码模式。",
                        summary.trajectories_analyzed,
                        top.value,
                        top.confidence,
                        top.occurrences,
                        summary.coding_patterns.len(),
                    ),
                    confidence: top.confidence as f64,
                    evidence: summary
                        .coding_patterns
                        .iter()
                        .take(3)
                        .map(|p| format!("{}={}", p.pattern_type, p.value))
                        .collect(),
                    suggested_action: Some(
                        "考虑将此模式固化到 UserProfile 或技能模板中".to_string(),
                    ),
                    created_at: now_ms,
                });
                result.insights_written += 1;
            }

            // 3.2 工具偏好模式洞察
            if !summary.tool_preference_patterns.is_empty() {
                let top = &summary.tool_preference_patterns[0];
                is.add_insight(axagent_trajectory::LearningInsight {
                    id: format!("pattern_tool_{}", now_ms),
                    category: axagent_trajectory::InsightCategory::Preference,
                    title: format!("工具偏好: {} (频率 {:.0}%)", top.tool_name, top.usage_frequency * 100.0),
                    description: format!(
                        "最常使用工具 '{}'：使用频率 {:.0}%, 平均耗时 {}ms, 成功率 {:.0}%。共分析 {} 个工具。",
                        top.tool_name,
                        top.usage_frequency * 100.0,
                        top.avg_duration_ms,
                        top.success_rate * 100.0,
                        summary.tool_preference_patterns.len(),
                    ),
                    confidence: top.success_rate as f64,
                    evidence: summary
                        .tool_preference_patterns
                        .iter()
                        .take(3)
                        .map(|p| format!("{}({:.0}%)", p.tool_name, p.usage_frequency * 100.0))
                        .collect(),
                    suggested_action: if top.success_rate < 0.5 {
                        Some(format!(
                            "工具 '{}' 成功率偏低，考虑创建辅助技能或工作流优化",
                            top.tool_name
                        ))
                    } else {
                        None
                    },
                    created_at: now_ms,
                });
                result.insights_written += 1;
            }

            // 3.3 时间分布模式洞察
            if !summary.temporal_patterns.is_empty() {
                let top = &summary.temporal_patterns[0];
                is.add_insight(axagent_trajectory::LearningInsight {
                    id: format!("pattern_time_{}", now_ms),
                    category: axagent_trajectory::InsightCategory::Pattern,
                    title: format!("时间分布模式: {}", top.pattern_type),
                    description: format!(
                        "检测到 {} 模式：{}:00-{}:00 (UTC)，置信度 {:.2}。共 {} 个时间模式。",
                        top.pattern_type,
                        top.start_hour,
                        top.end_hour,
                        top.confidence,
                        summary.temporal_patterns.len(),
                    ),
                    confidence: top.confidence as f64,
                    evidence: summary
                        .temporal_patterns
                        .iter()
                        .take(3)
                        .map(|p| format!("{}({}-{}h)", p.pattern_type, p.start_hour, p.end_hour))
                        .collect(),
                    suggested_action: Some("在偏好时段安排高优先级任务".to_string()),
                    created_at: now_ms,
                });
                result.insights_written += 1;
            }

            tracing::info!(
                "[PatternAnalyzerTask] 写入 {} 条洞察到 insight_system",
                result.insights_written
            );
        } else {
            tracing::warn!("[PatternAnalyzerTask] 未提供 insight_system，跳过洞察写入");
            result.errors.push("跳过洞察写入：未提供 insight_system".to_string());
        }

        result.duration_ms = start.elapsed().as_millis() as u64;
        result
    }
}
