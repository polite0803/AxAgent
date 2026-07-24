// SPDX-License-Identifier: AGPL-3.0-only

//! CoevolutionTask — 协同进化任务（后台周期执行）
//!
//! 在定时触发时，执行以下操作：
//! - 从 trajectory 存储读取最近的轨迹
//! - 计算近期轨迹的成功率作为 agent 性能信号
//! - 调用 [`CoevolutionEnvironment::update_performance`] 更新难度基线
//! - 调用 [`CoevolutionEnvironment::generate_task`] 生成针对薄弱类别的新任务
//! - 把难度变化 / 生成任务作为 [`LearningInsight`] 写回 insight_system
//!
//! 与 `start_skill_evolution` 中对 `CoevolutionEnvironment` 的使用关系：
//! - `start_skill_evolution` 在技能进化成功后被动调用 `update_performance`
//! - 本任务主动周期性地用整体轨迹成功率驱动协同进化，并生成新任务
//! - 两者共享同一 `CoevolutionEnvironment` 实例（通过 `AppState.coevolution_env`）

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// 协同进化任务执行上下文，集中持有各子功能需要的依赖。
///
/// 所有字段均为 `Arc` 克隆，调用方（如 `init/services.rs`）从 `AppState` 组装。
/// 缺失依赖时对应子功能会跳过并记录到 `errors`。
#[derive(Default, Clone)]
pub struct CoevolutionTaskContext {
    /// 协同进化环境（持有难度基线、性能历史、任务模板）
    pub coevolution_env:
        Option<Arc<tokio::sync::Mutex<axagent_trajectory::CoevolutionEnvironment>>>,
    /// 轨迹存储（读取近期轨迹计算成功率）
    pub trajectory_storage: Option<Arc<axagent_trajectory::TrajectoryStorage>>,
    /// 洞察系统（写入难度变化 / 生成任务的洞察）
    pub insight_system: Option<Arc<tokio::sync::RwLock<axagent_trajectory::LearningInsightSystem>>>,
}

impl CoevolutionTaskContext {
    /// 从单个 coevolution_env 构造最小上下文（仅支持难度更新，无法读取轨迹）。
    pub fn with_env(
        env: Arc<tokio::sync::Mutex<axagent_trajectory::CoevolutionEnvironment>>,
    ) -> Self {
        Self { coevolution_env: Some(env), ..Default::default() }
    }
}

/// 协同进化任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoevolutionTaskResult {
    /// 分析的轨迹数量
    pub trajectories_analyzed: usize,
    /// 计算得到的成功率 [0.0, 1.0]
    pub success_rate: f64,
    /// 执行前的难度系数
    pub difficulty_before: f64,
    /// 执行后的难度系数
    pub difficulty_after: f64,
    /// 难度是否发生变化
    pub difficulty_changed: bool,
    /// 生成的任务类别（针对薄弱类别）
    pub generated_task_category: Option<String>,
    /// 生成的任务难度等级
    pub generated_task_difficulty: Option<axagent_trajectory::DifficultyLevel>,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
    /// 错误信息（如有）
    pub errors: Vec<String>,
}

/// 协同进化任务执行器
pub struct CoevolutionTaskExecutor;

impl CoevolutionTaskExecutor {
    /// 执行协同进化任务并返回结果
    ///
    /// 若 `COEVOLUTION_TASK` feature flag 未启用，直接返回空结果。
    /// 各子功能依赖通过 `ctx` 注入；缺失依赖会跳过并在 `errors` 中记录原因。
    pub async fn execute(ctx: &CoevolutionTaskContext) -> CoevolutionTaskResult {
        // 检查 COEVOLUTION_TASK feature flag
        if !axagent_runtime_core::feature_flags::global_feature_flags().coevolution_task_sync() {
            tracing::warn!(
                "CoevolutionTask 未启用，跳过执行（设置 AXAGENT_FF_COEVOLUTION_TASK=1 或 features.CoevolutionTask=true）"
            );
            return CoevolutionTaskResult {
                trajectories_analyzed: 0,
                success_rate: 0.0,
                difficulty_before: 0.0,
                difficulty_after: 0.0,
                difficulty_changed: false,
                generated_task_category: None,
                generated_task_difficulty: None,
                duration_ms: 0,
                errors: vec![],
            };
        }

        let start = std::time::Instant::now();
        let mut result = CoevolutionTaskResult {
            trajectories_analyzed: 0,
            success_rate: 0.0,
            difficulty_before: 0.0,
            difficulty_after: 0.0,
            difficulty_changed: false,
            generated_task_category: None,
            generated_task_difficulty: None,
            duration_ms: 0,
            errors: Vec::new(),
        };

        // 1. 拉取近期轨迹，计算成功率
        let success_rate = match Self::compute_recent_success_rate(ctx, &mut result).await {
            Some(rate) => rate,
            None => {
                result.duration_ms = start.elapsed().as_millis() as u64;
                return result;
            },
        };
        result.success_rate = success_rate;

        // 2. 调用 CoevolutionEnvironment 更新性能 + 生成任务
        let env_arc = match &ctx.coevolution_env {
            Some(e) => e.clone(),
            None => {
                tracing::warn!("[CoevolutionTask] 跳过：未提供 coevolution_env");
                result.errors.push("跳过：未提供 coevolution_env".to_string());
                result.duration_ms = start.elapsed().as_millis() as u64;
                return result;
            },
        };

        let mut env = env_arc.lock().await;
        result.difficulty_before = env.difficulty_level();
        let should_increase = env.should_increase_difficulty();
        let should_decrease = env.should_decrease_difficulty();

        // 用近期成功率更新性能历史（驱动难度调整）
        env.update_performance(success_rate);
        result.difficulty_after = env.difficulty_level();
        result.difficulty_changed =
            (result.difficulty_after - result.difficulty_before).abs() > f64::EPSILON;

        // 生成针对薄弱类别的新任务
        let generated_task = env.generate_task();
        result.generated_task_category = Some(generated_task.category.clone());
        result.generated_task_difficulty = Some(generated_task.difficulty);

        tracing::info!(
            "[CoevolutionTask] 成功率={:.2}, 难度 {:.3}→{:.3} ({}), 生成任务: category={}, difficulty={:?}",
            success_rate,
            result.difficulty_before,
            result.difficulty_after,
            if result.difficulty_changed {
                if should_increase {
                    "↑"
                } else if should_decrease {
                    "↓"
                } else {
                    "微调"
                }
            } else {
                "不变"
            },
            generated_task.category,
            generated_task.difficulty,
        );

        drop(env);

        // 3. 把难度变化 / 生成任务作为洞察写入 insight_system
        if let Some(insight_system_arc) = &ctx.insight_system {
            let mut is = insight_system_arc.write().await;
            let category = if result.difficulty_changed && should_increase {
                axagent_trajectory::InsightCategory::Improvement
            } else if result.difficulty_changed && should_decrease {
                axagent_trajectory::InsightCategory::Warning
            } else {
                axagent_trajectory::InsightCategory::Pattern
            };
            let title = format!(
                "协同进化: 难度 {:.2}→{:.2}, 生成 {} 任务",
                result.difficulty_before, result.difficulty_after, generated_task.category,
            );
            let description = format!(
                "近期 {} 条轨迹成功率 {:.0}%。{}难度调整为 {:.3}。生成任务难度: {:?}，预期模式: {:?}",
                result.trajectories_analyzed,
                success_rate * 100.0,
                if should_increase {
                    "性能持续高于目标，"
                } else if should_decrease {
                    "性能持续低于目标，"
                } else {
                    ""
                },
                result.difficulty_after,
                generated_task.difficulty,
                generated_task.expected_patterns,
            );
            is.add_insight(axagent_trajectory::LearningInsight {
                id: format!("coevo_{}", chrono::Utc::now().timestamp_millis()),
                category,
                title,
                description,
                confidence: success_rate,
                evidence: vec![],
                suggested_action: Some(format!(
                    "针对薄弱类别 '{}' 创建练习技能或工作流",
                    generated_task.category
                )),
                created_at: chrono::Utc::now().timestamp_millis(),
            });
        } else {
            tracing::warn!("[CoevolutionTask] 未提供 insight_system，跳过洞察写入");
            result.errors.push("跳过洞察写入：未提供 insight_system".to_string());
        }

        result.duration_ms = start.elapsed().as_millis() as u64;
        result
    }

    /// 从 trajectory_storage 拉取近期轨迹，计算成功率（Success outcome 占比）
    async fn compute_recent_success_rate(
        ctx: &CoevolutionTaskContext,
        result: &mut CoevolutionTaskResult,
    ) -> Option<f64> {
        let storage = ctx.trajectory_storage.as_ref()?;
        let trajectories = match storage.get_trajectories(Some(20)).await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("[CoevolutionTask] 拉取轨迹失败: {}", e);
                result.errors.push(format!("拉取轨迹失败: {e}"));
                return None;
            },
        };
        if trajectories.is_empty() {
            tracing::info!("[CoevolutionTask] 无近期轨迹，跳过本轮");
            result.errors.push("无近期轨迹可分析".to_string());
            return None;
        }
        result.trajectories_analyzed = trajectories.len();
        let success_count = trajectories
            .iter()
            .filter(|t| matches!(t.outcome, axagent_trajectory::TrajectoryOutcome::Success))
            .count();
        Some(success_count as f64 / trajectories.len() as f64)
    }
}
