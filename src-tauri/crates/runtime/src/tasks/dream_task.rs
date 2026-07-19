// SPDX-License-Identifier: AGPL-3.0-only

//! DreamTask — 梦境任务（后台上下文整合与压缩）
//!
//! 在会话结束时或定时触发，执行以下操作：
//! - 轨迹整合 (ConsolidateTrajectory)
//! - 记忆压缩 (CompressMemories)
//! - 技能更新 (UpdateSkills)
//! - 僵尸 agent 清理 (CleanupDeadAgents)
//! - 向量索引优化 (OptimizeIndexes)
//!
//! 核心 Dream 巩固逻辑（经验回放、知识蒸馏、对比学习、建议生成）
//! 由 `axagent_trajectory::DreamConsolidator` 实现。
//! DreamTaskExecutor 负责调度编排这些后台清理任务。

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 僵尸 SubAgent 的存活阈值（秒）。
/// 创建超过该时长且仍处于 Running/Pending 状态的 SubAgent 视为僵尸。
const STALE_SUBAGENT_TTL_SECS: i64 = 3600;

/// 单次 DreamTask 中允许进化的弱技能数量上限。
/// 避免一次性进化过多技能导致 LLM 调用过载。
const MAX_SKILLS_EVOLVE_PER_RUN: usize = 2;

/// 梦境任务触发方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DreamTrigger {
    /// 会话结束时触发
    OnSessionEnd,
    /// 定时触发 (cron 表达式)
    Scheduled { cron: String },
    /// 内存超阈值触发
    OnThreshold { memory_mb: u64 },
}

/// 梦境任务执行范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DreamScope {
    /// 轨迹整合
    ConsolidateTrajectory,
    /// 记忆压缩
    CompressMemories,
    /// 技能更新
    UpdateSkills,
    /// 清理僵尸 agent
    CleanupDeadAgents,
    /// 向量索引优化
    OptimizeIndexes,
    /// 全部执行
    FullCleanup,
}

/// 梦境任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamTask {
    pub id: String,
    pub trigger: DreamTrigger,
    pub scope: DreamScope,
    pub created_at: DateTime<Utc>,
    pub status: DreamTaskStatus,
    pub result: Option<DreamTaskResult>,
}

/// 梦境任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DreamTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// 梦境任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamTaskResult {
    /// 压缩的轨迹数量
    pub trajectories_compressed: usize,
    /// 优化的技能数量
    pub skills_updated: usize,
    /// 清理的 agent 数量
    pub agents_cleaned: usize,
    /// 释放的内存 (MB)
    pub memory_freed_mb: u64,
    /// 执行耗时 (毫秒)
    pub duration_ms: u64,
    /// 执行摘要
    pub summary: String,
    /// 错误信息（如有）
    pub errors: Vec<String>,
}

impl DreamTask {
    /// 创建一个会话结束时的全量梦境任务
    pub fn on_session_end() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            trigger: DreamTrigger::OnSessionEnd,
            scope: DreamScope::FullCleanup,
            created_at: Utc::now(),
            status: DreamTaskStatus::Pending,
            result: None,
        }
    }

    /// 创建一个定时梦境任务
    pub fn scheduled(cron: &str, scope: DreamScope) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            trigger: DreamTrigger::Scheduled { cron: cron.to_string() },
            scope,
            created_at: Utc::now(),
            status: DreamTaskStatus::Pending,
            result: None,
        }
    }

    /// 创建一个内存阈值触发的梦境任务
    pub fn on_threshold(memory_mb: u64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            trigger: DreamTrigger::OnThreshold { memory_mb },
            scope: DreamScope::CompressMemories,
            created_at: Utc::now(),
            status: DreamTaskStatus::Pending,
            result: None,
        }
    }

    /// 是否启用梦境任务（检查 feature flag）
    pub fn is_enabled() -> bool {
        axagent_runtime_core::feature_flags::global_feature_flags().dream_task_sync()
    }

    /// 获取人类可读的触发描述
    pub fn trigger_description(&self) -> String {
        match &self.trigger {
            DreamTrigger::OnSessionEnd => "会话结束".to_string(),
            DreamTrigger::Scheduled { cron } => format!("定时: {}", cron),
            DreamTrigger::OnThreshold { memory_mb } => format!("内存超过 {}MB", memory_mb),
        }
    }

    /// 获取人类可读的范围描述
    pub fn scope_description(&self) -> &'static str {
        match self.scope {
            DreamScope::ConsolidateTrajectory => "轨迹整合",
            DreamScope::CompressMemories => "记忆压缩",
            DreamScope::UpdateSkills => "技能更新",
            DreamScope::CleanupDeadAgents => "清理僵尸 agent",
            DreamScope::OptimizeIndexes => "向量索引优化",
            DreamScope::FullCleanup => "全量清理",
        }
    }
}

/// 梦境任务执行上下文，集中持有各子功能需要的依赖。
///
/// 所有字段均为 `Option`，缺失依赖时对应子功能会跳过并记录到 `errors`。
/// 调用方（如 `init/services.rs`）从 `AppState` 组装此上下文传入。
#[derive(Default, Clone)]
pub struct DreamTaskContext {
    /// 轨迹整合器（ConsolidateTrajectory 子功能）
    pub consolidator: Option<Arc<axagent_trajectory::DreamConsolidator>>,
    /// 轨迹存储（OptimizeIndexes 子功能 + 读取弱技能）
    pub trajectory_storage: Option<Arc<axagent_trajectory::TrajectoryStorage>>,
    /// 技能进化引擎（UpdateSkills 子功能）
    pub skill_evolution_engine:
        Option<Arc<tokio::sync::Mutex<axagent_trajectory::SkillEvolutionEngine>>>,
    /// 自动记忆提取器（CompressMemories 子功能）
    pub auto_memory_extractor:
        Option<Arc<tokio::sync::RwLock<axagent_trajectory::AutoMemoryExtractor>>>,
    /// SubAgent 注册表（CleanupDeadAgents 子功能）
    pub sub_agent_registry: Option<Arc<tokio::sync::RwLock<axagent_trajectory::SubAgentRegistry>>>,
}

impl DreamTaskContext {
    /// 从单个 consolidator 构造最小上下文（仅支持 ConsolidateTrajectory）。
    pub fn with_consolidator(consolidator: Arc<axagent_trajectory::DreamConsolidator>) -> Self {
        Self { consolidator: Some(consolidator), ..Default::default() }
    }
}

/// 梦境任务执行器
pub struct DreamTaskExecutor;

impl DreamTaskExecutor {
    /// 执行梦境任务并返回结果
    ///
    /// 若 DREAM_TASK feature flag 未启用，直接返回空结果。
    /// 各子功能依赖通过 `ctx` 注入；缺失依赖会跳过并在 `errors` 中记录原因。
    pub async fn execute(task: &DreamTask, ctx: &DreamTaskContext) -> DreamTaskResult {
        // 检查 DREAM_TASK feature flag
        if !DreamTask::is_enabled() {
            tracing::warn!(
                "DreamTask 未启用，跳过执行（设置 AXAGENT_FF_DREAM_TASK=1 或 features.DreamTask=true）"
            );
            return DreamTaskResult {
                trajectories_compressed: 0,
                skills_updated: 0,
                agents_cleaned: 0,
                memory_freed_mb: 0,
                duration_ms: 0,
                summary: "DreamTask 未启用".to_string(),
                errors: vec![],
            };
        }

        let start = std::time::Instant::now();
        let mut result = DreamTaskResult {
            trajectories_compressed: 0,
            skills_updated: 0,
            agents_cleaned: 0,
            memory_freed_mb: 0,
            duration_ms: 0,
            summary: String::new(),
            errors: Vec::new(),
        };

        // FullCleanup 模式下执行所有清理步骤
        let is_full = matches!(task.scope, DreamScope::FullCleanup);

        if is_full || matches!(task.scope, DreamScope::ConsolidateTrajectory) {
            tracing::info!("[DreamTask] 执行轨迹整合...");
            if let Some(ref consolidator) = ctx.consolidator {
                let consolidation_result = consolidator.consolidate_force().await;
                result.trajectories_compressed = consolidation_result.memories_extracted;
                tracing::info!(
                    "[DreamTask] 轨迹整合完成: {} 条记忆, {} 条知识, {} 个对比洞察",
                    consolidation_result.memories_extracted,
                    consolidation_result.distilled_knowledge_count,
                    consolidation_result.contrastive_insights_count,
                );
            } else {
                tracing::warn!("[DreamTask] 未提供 DreamConsolidator 实例，跳过轨迹整合");
                result.errors.push("轨迹整合跳过：未提供 consolidator".to_string());
            }
        }

        if is_full || matches!(task.scope, DreamScope::CompressMemories) {
            tracing::info!("[DreamTask] 执行记忆压缩...");
            Self::compress_memories(ctx, &mut result).await;
        }

        if is_full || matches!(task.scope, DreamScope::UpdateSkills) {
            tracing::info!("[DreamTask] 执行技能更新...");
            Self::update_skills(ctx, &mut result).await;
        }

        if is_full || matches!(task.scope, DreamScope::CleanupDeadAgents) {
            tracing::info!("[DreamTask] 清理僵尸 agent...");
            Self::cleanup_dead_agents(ctx, &mut result).await;
        }

        if is_full || matches!(task.scope, DreamScope::OptimizeIndexes) {
            tracing::info!("[DreamTask] 优化向量索引...");
            Self::optimize_indexes(ctx, &mut result).await;
        }

        result.duration_ms = start.elapsed().as_millis() as u64;
        result.summary = format!(
            "梦境任务完成: 压缩{}条轨迹, 更新{}个技能, 清理{}个agent, 释放{}MB, 耗时{}ms",
            result.trajectories_compressed,
            result.skills_updated,
            result.agents_cleaned,
            result.memory_freed_mb,
            result.duration_ms,
        );

        tracing::info!("[DreamTask] {}", result.summary);
        result
    }

    /// 记忆压缩：对最近的 trajectory 提取记忆并应用到 MemoryService。
    ///
    /// 通过 `auto_memory_extractor.analyze_trajectory()` 提取结构化记忆，
    /// 再用 `apply_memories_to_service()` 写入 MemoryService（含去重）。
    /// 同时触发 FTS5 索引 optimize 以合并 segments，预估释放内存。
    async fn compress_memories(ctx: &DreamTaskContext, result: &mut DreamTaskResult) {
        // 1. 从 trajectory_storage 拉取最近的轨迹
        let recent_trajectories: Vec<axagent_trajectory::Trajectory> = match &ctx.trajectory_storage
        {
            Some(storage) => match storage.get_trajectories(Some(10)).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("[DreamTask] 记忆压缩拉取轨迹失败: {}", e);
                    result.errors.push(format!("记忆压缩拉取轨迹失败: {e}"));
                    return;
                },
            },
            None => {
                tracing::warn!("[DreamTask] 记忆压缩跳过：未提供 trajectory_storage");
                result.errors.push("记忆压缩跳过：未提供 trajectory_storage".to_string());
                return;
            },
        };

        if recent_trajectories.is_empty() {
            tracing::info!("[DreamTask] 记忆压缩：无近期轨迹可处理");
            return;
        }

        // 2. 用 auto_memory_extractor 提取记忆并应用
        let mut extracted_total = 0usize;
        if let Some(ref extractor_arc) = ctx.auto_memory_extractor {
            let mut extractor = extractor_arc.write().await;
            for trajectory in &recent_trajectories {
                if let Some(extraction) = extractor.analyze_trajectory(trajectory) {
                    let applied = extractor
                        .apply_memories_to_service(&extraction.extracted_memories)
                        .await
                        .unwrap_or(0);
                    extracted_total += applied;
                }
            }
            tracing::info!(
                "[DreamTask] 记忆压缩：从 {} 条轨迹中提取并应用 {} 条记忆",
                recent_trajectories.len(),
                extracted_total
            );
            // trajectories_compressed 字段语义：本次处理的轨迹相关条目数。
            // 此处用应用成功的记忆数表示压缩产出（与 ConsolidateTrajectory 一致）。
            result.trajectories_compressed += extracted_total;
        } else {
            tracing::warn!("[DreamTask] 记忆压缩跳过：未提供 auto_memory_extractor");
            result.errors.push("记忆压缩跳过：未提供 auto_memory_extractor".to_string());
        }

        // 3. 触发 FTS5 索引 optimize（合并 segments，减少磁盘占用）
        if let Some(ref storage) = ctx.trajectory_storage {
            match storage.optimize_fts().await {
                Ok(()) => {
                    // FTS5 optimize 不直接释放内存，但合并 segments 后查询路径更短，
                    // 间接降低内存占用。这里给一个保守的估算值。
                    result.memory_freed_mb += 5;
                    tracing::info!("[DreamTask] 记忆压缩：FTS5 索引已优化");
                },
                Err(e) => {
                    tracing::warn!("[DreamTask] 记忆压缩 FTS5 optimize 失败: {}", e);
                    result.errors.push(format!("FTS5 optimize 失败: {e}"));
                },
            }
        }
    }

    /// 技能更新：扫描弱技能并调用 SkillEvolutionEngine 进行进化。
    ///
    /// 弱技能判定（与 `start_skill_evolution` 一致）：
    /// - consecutive_failures >= 3，或
    /// - total_usages >= 3 且 success_rate < 0.5
    ///
    /// 单次最多进化 `MAX_SKILLS_EVOLVE_PER_RUN` 个技能，避免 LLM 调用过载。
    async fn update_skills(ctx: &DreamTaskContext, result: &mut DreamTaskResult) {
        let storage = match &ctx.trajectory_storage {
            Some(s) => s,
            None => {
                tracing::warn!("[DreamTask] 技能更新跳过：未提供 trajectory_storage");
                result.errors.push("技能更新跳过：未提供 trajectory_storage".to_string());
                return;
            },
        };
        let engine_arc = match &ctx.skill_evolution_engine {
            Some(e) => e,
            None => {
                tracing::warn!("[DreamTask] 技能更新跳过：未提供 skill_evolution_engine");
                result.errors.push("技能更新跳过：未提供 skill_evolution_engine".to_string());
                return;
            },
        };

        let skills: Vec<axagent_trajectory::Skill> = match storage.get_skills().await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("[DreamTask] 技能更新拉取技能失败: {}", e);
                result.errors.push(format!("技能更新拉取技能失败: {e}"));
                return;
            },
        };

        // 弱技能过滤（阈值与 start_skill_evolution 一致）
        let weak_skills: Vec<_> = skills
            .into_iter()
            .filter(|s| {
                s.consecutive_failures >= 3 || (s.total_usages >= 3 && s.success_rate < 0.5)
            })
            .collect();

        if weak_skills.is_empty() {
            tracing::info!("[DreamTask] 技能更新：无弱技能需要进化");
            return;
        }

        // 拉取测试轨迹（供进化评估使用）
        let test_trajectories: Vec<axagent_trajectory::Trajectory> =
            match storage.get_trajectories(Some(30)).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("[DreamTask] 技能更新拉取轨迹失败: {}", e);
                    result.errors.push(format!("技能更新拉取轨迹失败: {e}"));
                    return;
                },
            };
        let test_refs: Vec<&axagent_trajectory::Trajectory> = test_trajectories.iter().collect();

        let mut evolved_count = 0usize;
        for skill in weak_skills.iter().take(MAX_SKILLS_EVOLVE_PER_RUN) {
            let mut engine = engine_arc.lock().await;
            let evolution_result = engine.run(skill, &test_refs).await;
            if let Some(modification) = evolution_result
                && modification.validation_result.as_ref().is_some_and(|v| v.success)
            {
                // 进化成功，写回 storage
                let mut updated_skill = skill.clone();
                updated_skill.content = modification.new_content.clone();
                updated_skill.quality_score = modification.confidence;
                updated_skill.version = format!(
                    "{}.d{}",
                    updated_skill
                        .version
                        .trim_end_matches(|c: char| c == '.' || c.is_ascii_digit()),
                    chrono::Utc::now().timestamp_millis() % 10000
                );
                if let Err(e) = storage.save_skill(&updated_skill).await {
                    tracing::warn!("[DreamTask] 技能 '{}' 进化后保存失败: {}", skill.name, e);
                    result.errors.push(format!("技能 '{}' 保存失败: {e}", skill.name));
                } else {
                    evolved_count += 1;
                    tracing::info!(
                        "[DreamTask] 技能 '{}' 已进化 (confidence={:.3})",
                        skill.name,
                        modification.confidence
                    );
                }
            } else {
                tracing::info!("[DreamTask] 技能 '{}' 未产生有效进化", skill.name);
            }
        }

        result.skills_updated = evolved_count;
    }

    /// 僵尸 agent 清理：扫描 SubAgentRegistry，删除创建超过 1 小时且仍处于
    /// Running/Pending 状态的 SubAgent。
    ///
    /// 这些 SubAgent 通常是上游派发后未正常 complete/fail 的孤儿任务，
    /// 长期占用注册表会导致 list_all 变慢、内存泄漏。
    async fn cleanup_dead_agents(ctx: &DreamTaskContext, result: &mut DreamTaskResult) {
        let registry_arc = match &ctx.sub_agent_registry {
            Some(r) => r,
            None => {
                tracing::warn!("[DreamTask] 僵尸清理跳过：未提供 sub_agent_registry");
                result.errors.push("僵尸清理跳过：未提供 sub_agent_registry".to_string());
                return;
            },
        };

        let mut registry = registry_arc.write().await;
        let now = Utc::now();
        let stale_cutoff = now - chrono::Duration::seconds(STALE_SUBAGENT_TTL_SECS);

        // 先快照所有 stale agent 的 id（避免在持有写锁时调用 delete 产生借用问题）
        let stale_ids: Vec<String> = registry
            .list_all()
            .into_iter()
            .filter(|a| {
                matches!(
                    a.status,
                    axagent_trajectory::SubAgentStatus::Running
                        | axagent_trajectory::SubAgentStatus::Pending
                ) && a.created_at < stale_cutoff
            })
            .map(|a| a.id.clone())
            .collect();

        if stale_ids.is_empty() {
            tracing::info!("[DreamTask] 僵尸清理：无 stale SubAgent");
            return;
        }

        let mut cleaned = 0usize;
        for id in &stale_ids {
            if registry.delete(id).await {
                cleaned += 1;
            }
        }

        tracing::info!(
            "[DreamTask] 僵尸清理：删除 {} 个 stale SubAgent（阈值 {}s）",
            cleaned,
            STALE_SUBAGENT_TTL_SECS
        );
        result.agents_cleaned = cleaned;
        // 每个 SubAgent 估算占用约 1MB（含 task/result/metadata）
        result.memory_freed_mb += cleaned as u64;
    }

    /// 向量索引优化：触发 FTS5 optimize + vacuum。
    ///
    /// - `optimize`：合并 FTS5 segments，加速后续查询
    /// - `vacuum`：回收已删除记录占用的磁盘空间
    ///
    /// 通常在 trajectory_cleanup（删除旧轨迹）后调用效果最佳。
    async fn optimize_indexes(ctx: &DreamTaskContext, result: &mut DreamTaskResult) {
        let storage = match &ctx.trajectory_storage {
            Some(s) => s,
            None => {
                tracing::warn!("[DreamTask] 索引优化跳过：未提供 trajectory_storage");
                result.errors.push("索引优化跳过：未提供 trajectory_storage".to_string());
                return;
            },
        };

        // 1. FTS5 optimize
        if let Err(e) = storage.optimize_fts().await {
            tracing::warn!("[DreamTask] FTS5 optimize 失败: {}", e);
            result.errors.push(format!("FTS5 optimize 失败: {e}"));
        } else {
            tracing::info!("[DreamTask] FTS5 索引 optimize 完成");
        }

        // 2. FTS5 vacuum
        if let Err(e) = storage.vacuum_fts().await {
            tracing::warn!("[DreamTask] FTS5 vacuum 失败: {}", e);
            result.errors.push(format!("FTS5 vacuum 失败: {e}"));
        } else {
            tracing::info!("[DreamTask] FTS5 索引 vacuum 完成");
            // vacuum 实际释放磁盘空间，给一个保守估算
            result.memory_freed_mb += 2;
        }
    }
}
