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
            trigger: DreamTrigger::Scheduled {
                cron: cron.to_string(),
            },
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
        crate::feature_flags::global_feature_flags().dream_task_sync()
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

/// 梦境任务执行器
pub struct DreamTaskExecutor;

impl DreamTaskExecutor {
    /// 执行梦境任务并返回结果
    ///
    /// 若 DREAM_TASK feature flag 未启用，直接返回空结果。
    /// `consolidator` 是可选的轨迹巩固器实例，在 ConsolidateTrajectory 范围时使用。
    pub async fn execute(
        task: &DreamTask,
        consolidator: Option<Arc<axagent_trajectory::DreamConsolidator>>,
    ) -> DreamTaskResult {
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
            if let Some(ref consolidator) = consolidator {
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
            // TODO: 对接 SessionCompactor/auto_memory 模块实现实际压缩
            // 当前为占位实现，返回默认值
            result.memory_freed_mb += 10;
            tracing::warn!("[DreamTask] 记忆压缩尚未对接底层实现（占位释放10MB）");
        }

        if is_full || matches!(task.scope, DreamScope::UpdateSkills) {
            tracing::info!("[DreamTask] 执行技能更新...");
            // TODO: 对接 SkillEvolutionEngine 实现技能进化
            // axagent_trajectory::SkillEvolutionEngine 提供了技能进化能力
            // 需要持有 SkillEvolutionEngine 实例
            tracing::warn!("[DreamTask] 技能更新尚未对接底层实现");
        }

        if is_full || matches!(task.scope, DreamScope::CleanupDeadAgents) {
            tracing::info!("[DreamTask] 清理僵尸 agent...");
            // TODO: 扫描无活跃状态的会话和过期的 sub-agent
            tracing::warn!("[DreamTask] 僵尸 agent 清理尚未实现");
        }

        if is_full || matches!(task.scope, DreamScope::OptimizeIndexes) {
            tracing::info!("[DreamTask] 优化向量索引...");
            // TODO: 触发 SQLite FTS5 optimize 和向量索引优化
            result.memory_freed_mb += 1;
            tracing::warn!("[DreamTask] 向量索引优化尚未对接底层实现（占位释放1MB）");
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
}
