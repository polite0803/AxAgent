// SPDX-License-Identifier: AGPL-3.0-only

//! Trajectory storage module using SeaORM

use crate::fts5::{FTS5Config, FTS5Query, FTS5Result, FTS5Search};
use crate::memory::{Entity, Relationship};
use crate::skill::Skill;
use crate::trajectory::{
    MessageRole, RLTrainingEntry, RewardSignal, Trajectory, TrajectoryExportOptions,
    TrajectoryOutcome, TrajectoryPattern, TrajectoryQuery, TrajectoryStep,
};
use anyhow::{Context, Result};
use axagent_entities::{
    knowledge_entities, knowledge_relations, memory_items, trajectories,
    trajectory_learned_patterns, trajectory_messages, trajectory_patterns, trajectory_preferences,
    trajectory_rewards, trajectory_sessions, trajectory_skill_executions, trajectory_skills,
    trajectory_steps, trajectory_workflow_reflections,
};
use chrono::Utc;
use futures::FutureExt;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

pub struct TrajectoryStorage {
    db: Arc<DatabaseConnection>,
    fts_searcher: Option<FTS5Search>,
    /// 保存轨迹时是否抽取因果观测（默认关闭，见 [`TrajectoryStorage::set_causal_enabled`]）
    causal_enabled: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrajectoryCleanupConfig {
    pub max_age_days: Option<u32>,
    pub max_trajectories: Option<u32>,
}

impl Default for TrajectoryCleanupConfig {
    fn default() -> Self {
        Self { max_age_days: Some(90), max_trajectories: Some(10000) }
    }
}

impl TrajectoryStorage {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db, fts_searcher: None, causal_enabled: false }
    }

    /// 底层数据库连接（供因果边/校准等跨表查询复用同一连接）
    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    pub fn with_fts(
        db: Arc<DatabaseConnection>,
        fts_conn: Arc<Mutex<rusqlite::Connection>>,
    ) -> Self {
        Self {
            db,
            fts_searcher: Some(FTS5Search::new(fts_conn, FTS5Config::default())),
            causal_enabled: false,
        }
    }

    /// 开启/关闭轨迹保存时的因果观测抽取。
    ///
    /// 关闭时 `save_trajectory` 的行为与启用前完全一致。
    pub fn set_causal_enabled(&mut self, enabled: bool) {
        self.causal_enabled = enabled;
    }

    /// 因果观测是否已开启
    pub fn is_causal_enabled(&self) -> bool {
        self.causal_enabled
    }

    /// 记录一次意图转移观测 `intent:A → intent:B`。
    ///
    /// 开关关闭时为空操作；失败只记日志，不影响调用方。
    pub async fn observe_intent_transition(&self, from: &str, to: &str, delay_ms: Option<i64>) {
        if !self.causal_enabled || from == to {
            return;
        }
        if let Err(e) = crate::causal::observe_edge(
            self.db.as_ref(),
            from,
            to,
            true,
            delay_ms,
            "intent_transition",
        )
        .await
        {
            tracing::warn!("causal: intent transition observation failed: {e:#}");
        }
    }

    /// 依据因果边为当前预测生成可解释建议。开关关闭时返回空表。
    pub async fn causal_suggestions(
        &self,
        prediction: &crate::proactive_assistant::ContextPrediction,
        max: usize,
    ) -> Vec<crate::proactive_assistant::ProactiveSuggestion> {
        if !self.causal_enabled {
            return Vec::new();
        }
        crate::causal::causal_suggestions_for_intent(self.db.as_ref(), prediction, max).await
    }

    /// 从数据库文件路径创建带 FTS5 全文搜索的存储实例。
    /// 自动创建 FTS5 虚拟表（如不存在）。
    pub async fn with_fts_path(db: Arc<DatabaseConnection>, db_file_path: &str) -> Result<Self> {
        let db_file_path = db_file_path.to_string();
        let conn = tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(&db_file_path)
                .context("Failed to open FTS5 database")?;
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
                .context("Failed to set FTS5 connection pragmas")?;
            Ok::<_, anyhow::Error>(conn)
        })
        .await??;
        let conn = Arc::new(Mutex::new(conn));
        let fts = FTS5Search::new(conn, FTS5Config::default());
        fts.create_fts_tables().await?;
        Ok(Self { db, fts_searcher: Some(fts), causal_enabled: false })
    }

    // ── Trajectories ──

    /// 保存轨迹（事务化：轨迹主体 + steps + rewards 在同一事务中）
    /// FTS 索引在事务外执行，避免与 SeaORM 事务争用。
    pub async fn save_trajectory(&self, t: &Trajectory) -> Result<()> {
        // P0-8: 整个写入流程包在事务中
        let txn = self.db.begin().await?;

        let am = trajectories::ActiveModel {
            id: Set(t.id.clone()),
            session_id: Set(t.session_id.clone()),
            user_id: Set(t.user_id.clone()),
            agent_name: Set(t.agent_name.clone()),
            topic: Set(t.topic.clone()),
            summary: Set(t.summary.clone()),
            outcome: Set(format!("{:?}", t.outcome).to_lowercase()),
            duration_ms: Set(t.duration_ms as i64),
            quality_overall: Set(t.quality.overall),
            quality_task_completion: Set(t.quality.task_completion),
            quality_tool_efficiency: Set(t.quality.tool_efficiency),
            quality_reasoning_quality: Set(t.quality.reasoning_quality),
            quality_user_satisfaction: Set(t.quality.user_satisfaction),
            value_score: Set(t.value_score),
            patterns: Set(serde_json::to_string(&t.patterns)?),
            created_at: Set(t.created_at.to_rfc3339()),
            replay_count: Set(t.replay_count as i32),
            last_replay_at: Set(t.last_replay_at.map(|dt| dt.to_rfc3339())),
            // 新轨迹默认有效（append-only 证据链，v120 新增字段）
            is_invalidated: Set(0),
        };
        // P1-2: on_conflict 不再更新 CreatedAt（保留原创建时间）
        trajectories::Entity::insert(am)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(trajectories::Column::Id)
                    .update_columns([
                        trajectories::Column::SessionId,
                        trajectories::Column::AgentName,
                        trajectories::Column::Topic,
                        trajectories::Column::Summary,
                        trajectories::Column::Outcome,
                        trajectories::Column::DurationMs,
                        trajectories::Column::QualityOverall,
                        trajectories::Column::QualityTaskCompletion,
                        trajectories::Column::QualityToolEfficiency,
                        trajectories::Column::QualityReasoningQuality,
                        trajectories::Column::QualityUserSatisfaction,
                        trajectories::Column::ValueScore,
                        trajectories::Column::Patterns,
                        trajectories::Column::ReplayCount,
                        trajectories::Column::LastReplayAt,
                        // 重保存视为重新启用该轨迹：清除失效标记（append-only 证据链可恢复）
                        trajectories::Column::IsInvalidated,
                    ])
                    .to_owned(),
            )
            .exec(&txn)
            .await?;

        trajectory_steps::Entity::delete_many()
            .filter(trajectory_steps::Column::TrajectoryId.eq(&t.id))
            .exec(&txn)
            .await?;
        for (idx, step) in t.steps.iter().enumerate() {
            trajectory_steps::ActiveModel {
                trajectory_id: Set(t.id.clone()),
                step_index: Set(idx as i32),
                timestamp_ms: Set(step.timestamp_ms as i64),
                role: Set(format!("{:?}", step.role).to_lowercase()),
                content: Set(step.content.clone()),
                reasoning: Set(step.reasoning.clone()),
                tool_calls: Set(step
                    .tool_calls
                    .as_ref()
                    .and_then(|c| serde_json::to_string(c).ok())),
                tool_results: Set(step
                    .tool_results
                    .as_ref()
                    .and_then(|r| serde_json::to_string(r).ok())),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }

        trajectory_rewards::Entity::delete_many()
            .filter(trajectory_rewards::Column::TrajectoryId.eq(&t.id))
            .exec(&txn)
            .await?;
        for r in &t.rewards {
            trajectory_rewards::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                trajectory_id: Set(t.id.clone()),
                reward_type: Set(format!("{:?}", r.reward_type)),
                value: Set(r.value),
                step_index: Set(r.step_index as i32),
                created_at: Set(chrono::DateTime::from_timestamp_millis(r.timestamp_ms as i64)
                    .unwrap_or_else(Utc::now)
                    .to_rfc3339()),
            }
            .insert(&txn)
            .await?;
        }

        txn.commit().await?;

        // FTS 索引在事务外执行
        let _ = self.index_trajectory_fts(t).await;

        // 因果观测同样在事务外执行。失败仅告警——因果边是增强特性，
        // 不得因观测失败影响已经落库的轨迹。
        if self.causal_enabled {
            match crate::causal::observe_from_trajectory(self.db.as_ref(), t).await {
                Ok(count) => {
                    tracing::debug!("causal: observed {count} edges from trajectory {}", t.id)
                },
                Err(e) => {
                    tracing::warn!("causal: observation failed for trajectory {}: {:#}", t.id, e)
                },
            }
        }

        Ok(())
    }

    pub async fn get_trajectory(&self, id: &str) -> Result<Option<Trajectory>> {
        match trajectories::Entity::find_by_id(id).one(self.db.as_ref()).await? {
            Some(m) => Ok(Some(model_to_trajectory(
                &m,
                self.get_trajectory_steps(&m.id).await?,
                self.get_trajectory_rewards(&m.id).await?,
            ))),
            None => Ok(None),
        }
    }

    /// 获取有效的轨迹列表（已标记失效的 append-only 证据不参与活动查询）。
    pub async fn get_trajectories(&self, limit: Option<usize>) -> Result<Vec<Trajectory>> {
        let models = trajectories::Entity::find()
            .filter(trajectories::Column::IsInvalidated.eq(0))
            .order_by_desc(trajectories::Column::CreatedAt)
            .all(self.db.as_ref())
            .await?;
        let mut r = Vec::new();
        let end = limit.unwrap_or(models.len()).min(models.len());
        for m in models.into_iter().take(end) {
            r.push(model_to_trajectory(
                &m,
                self.get_trajectory_steps(&m.id).await?,
                self.get_trajectory_rewards(&m.id).await?,
            ));
        }
        Ok(r)
    }

    /// P3-1（阶段三）：标记轨迹失效（软删除，append-only 证据存储）。
    ///
    /// 轨迹及其 steps/rewards/skill_executions 作为进化证据**不可物理删除**，
    /// 仅置 `is_invalidated = 1` 使其退出活动查询（get_trajectories /
    /// get_session_trajectories / query_trajectories）；同时清理 FTS 索引，
    /// 避免全文搜索命中已失效证据。证据本体保留，供贝叶斯后验回溯。
    pub async fn delete_trajectory(&self, id: &str) -> Result<()> {
        let m = trajectories::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .context("Trajectory not found")?;
        let mut am: trajectories::ActiveModel = m.into_active_model();
        am.is_invalidated = Set(1);
        am.update(self.db.as_ref()).await?;
        let _ = self.delete_trajectory_fts(id).await;
        info!("Invalidated trajectory {}", id);
        Ok(())
    }

    /// P1-5: 用字符串比较 ISO8601 / RFC3339 时间戳（字典序与时序一致）。
    /// 阶段三起清理为软删除：仅标记失效，证据本体保留（append-only）。
    pub async fn cleanup_old_trajectories_by_age(&self, max_age_days: u32) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(max_age_days as i64);
        let cutoff_str = cutoff.to_rfc3339();
        // ISO8601 / RFC3339 格式为 year-first、zero-padded，字符串字典序与时序一致，
        // 不需要 datetime() 函数（该函数是 SQLite 专有，PostgreSQL 不存在）。
        let old_trajectories = trajectories::Entity::find()
            .filter(trajectories::Column::IsInvalidated.eq(0))
            .filter(sea_orm::sea_query::Expr::cust(format!("created_at < '{}'", cutoff_str)))
            .all(self.db.as_ref())
            .await?;
        let count = old_trajectories.len();
        for traj in old_trajectories {
            self.delete_trajectory(&traj.id).await?;
        }
        Ok(count)
    }

    /// P1-4: 避免全表加载，使用 NOT IN 子查询找出需清理的 ID。
    /// 阶段三起清理为软删除：仅标记失效，证据本体保留（append-only）。
    pub async fn cleanup_old_trajectories_by_count(&self, max_trajectories: u32) -> Result<usize> {
        // 先查总数判断是否需要清理（仅统计有效轨迹）
        let total = trajectories::Entity::find()
            .filter(trajectories::Column::IsInvalidated.eq(0))
            .count(self.db.as_ref())
            .await?;
        if total <= max_trajectories as u64 {
            return Ok(0);
        }
        // 用 NOT IN 子查询找出需要保留的 ID 集合
        let to_delete_ids: Vec<String> = {
            use sea_orm::PaginatorTrait;
            // 取第二页（跳过前 max_trajectories 条），即为超出保留阈值的最旧轨迹
            let page_size: u64 = std::cmp::max(max_trajectories as u64, 1);
            let paginator = trajectories::Entity::find()
                .filter(trajectories::Column::IsInvalidated.eq(0))
                .order_by_desc(trajectories::Column::CreatedAt)
                .paginate(self.db.as_ref(), page_size);
            let extra = paginator.fetch_page(1).await?;
            extra.into_iter().map(|t| t.id).collect()
        };
        let count = to_delete_ids.len();
        for id in to_delete_ids {
            let _ = self.delete_trajectory(&id).await;
        }
        Ok(count)
    }

    pub async fn cleanup(&self, config: &TrajectoryCleanupConfig) -> Result<usize> {
        let mut total_deleted = 0;
        if let Some(max_age_days) = config.max_age_days {
            total_deleted += self.cleanup_old_trajectories_by_age(max_age_days).await?;
        }
        if let Some(max_trajectories) = config.max_trajectories {
            total_deleted += self.cleanup_old_trajectories_by_count(max_trajectories).await?;
        }
        Ok(total_deleted)
    }

    pub async fn get_session_trajectories(&self, session_id: &str) -> Result<Vec<Trajectory>> {
        let models = trajectories::Entity::find()
            .filter(trajectories::Column::SessionId.eq(session_id))
            .filter(trajectories::Column::IsInvalidated.eq(0))
            .order_by_asc(trajectories::Column::CreatedAt)
            .all(self.db.as_ref())
            .await?;
        let mut r = Vec::new();
        for m in models {
            r.push(model_to_trajectory(
                &m,
                self.get_trajectory_steps(&m.id).await?,
                self.get_trajectory_rewards(&m.id).await?,
            ));
        }
        Ok(r)
    }

    pub async fn query_trajectories(&self, query: &TrajectoryQuery) -> Result<Vec<Trajectory>> {
        let mut q = trajectories::Entity::find();
        // 已标记失效的 append-only 证据不参与活动查询（贝叶斯后验回溯走 get_trajectory）
        q = q.filter(trajectories::Column::IsInvalidated.eq(0));
        if let Some(ref sid) = query.session_id {
            q = q.filter(trajectories::Column::SessionId.eq(sid));
        }
        if let Some(ref uid) = query.user_id {
            q = q.filter(trajectories::Column::UserId.eq(uid));
        }
        if let Some(ref topic) = query.topic {
            q = q.filter(trajectories::Column::Topic.like(format!("%{}%", topic)));
        }
        if let Some(mq) = query.min_quality {
            q = q.filter(trajectories::Column::QualityOverall.gte(mq));
        }
        if let Some(mv) = query.min_value_score {
            q = q.filter(trajectories::Column::ValueScore.gte(mv));
        }
        if let Some(ref outcome) = query.outcome {
            q = q.filter(trajectories::Column::Outcome.eq(format!("{:?}", outcome)));
        }
        if let Some((start, end)) = query.time_range {
            q = q
                .filter(trajectories::Column::CreatedAt.gte(start.to_rfc3339()))
                .filter(trajectories::Column::CreatedAt.lte(end.to_rfc3339()));
        }
        q = q.order_by_desc(trajectories::Column::CreatedAt);
        let models = q.all(self.db.as_ref()).await?;
        let end = query.limit.unwrap_or(models.len()).min(models.len());
        let mut r = Vec::new();
        for m in models.into_iter().take(end) {
            r.push(model_to_trajectory(
                &m,
                self.get_trajectory_steps(&m.id).await?,
                self.get_trajectory_rewards(&m.id).await?,
            ));
        }
        Ok(r)
    }

    async fn get_trajectory_steps(&self, trajectory_id: &str) -> Result<Vec<TrajectoryStep>> {
        Ok(trajectory_steps::Entity::find()
            .filter(trajectory_steps::Column::TrajectoryId.eq(trajectory_id))
            .order_by_asc(trajectory_steps::Column::StepIndex)
            .all(self.db.as_ref())
            .await?
            .into_iter()
            .map(|s| TrajectoryStep {
                timestamp_ms: s.timestamp_ms as u64,
                role: serde_json::from_str(&format!("\"{}\"", s.role))
                    .unwrap_or(MessageRole::Assistant),
                content: s.content,
                reasoning: s.reasoning,
                tool_calls: s.tool_calls.and_then(|c| serde_json::from_str(&c).ok()),
                tool_results: s.tool_results.and_then(|r| serde_json::from_str(&r).ok()),
            })
            .collect())
    }

    async fn get_trajectory_rewards(&self, trajectory_id: &str) -> Result<Vec<RewardSignal>> {
        Ok(trajectory_rewards::Entity::find()
            .filter(trajectory_rewards::Column::TrajectoryId.eq(trajectory_id))
            .all(self.db.as_ref())
            .await?
            .into_iter()
            .map(|r| {
                let rt = match r.reward_type.as_str() {
                    "task_completion" => crate::trajectory::RewardType::TaskCompletion,
                    "tool_efficiency" => crate::trajectory::RewardType::ToolEfficiency,
                    "reasoning_quality" => crate::trajectory::RewardType::ReasoningQuality,
                    _ => crate::trajectory::RewardType::UserFeedback,
                };
                let ct = chrono::DateTime::parse_from_rfc3339(&r.created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                RewardSignal {
                    reward_type: rt,
                    value: r.value,
                    step_index: r.step_index as usize,
                    timestamp_ms: ct.timestamp_millis() as u64,
                    metadata: serde_json::Value::Null,
                }
            })
            .collect())
    }

    // ── Patterns ──

    pub async fn save_pattern(&self, p: &TrajectoryPattern) -> Result<()> {
        trajectory_patterns::Entity::insert(trajectory_patterns::ActiveModel {
            id: Set(p.id.clone()),
            name: Set(p.name.clone()),
            description: Set(p.description.clone()),
            pattern_type: Set(p.pattern_type.clone()),
            trajectory_ids: Set(serde_json::to_string(&p.trajectory_ids)?),
            frequency: Set(p.frequency as i32),
            success_rate: Set(p.success_rate),
            average_quality: Set(p.average_quality),
            average_value_score: Set(p.average_value_score),
            reward_profile: Set(serde_json::to_string(&p.reward_profile)?),
            created_at: Set(p.created_at.to_rfc3339()),
        })
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(trajectory_patterns::Column::Id)
                .update_columns([
                    trajectory_patterns::Column::Name,
                    trajectory_patterns::Column::Frequency,
                    trajectory_patterns::Column::SuccessRate,
                    trajectory_patterns::Column::AverageQuality,
                ])
                .to_owned(),
        )
        .exec(self.db.as_ref())
        .await?;
        Ok(())
    }

    pub async fn get_patterns(&self) -> Result<Vec<TrajectoryPattern>> {
        Ok(trajectory_patterns::Entity::find()
            .order_by_desc(trajectory_patterns::Column::Frequency)
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(model_to_traj_pattern)
            .collect())
    }

    pub async fn get_patterns_by_success_rate(
        &self,
        min_sr: f64,
        limit: Option<usize>,
    ) -> Result<Vec<TrajectoryPattern>> {
        let models = trajectory_patterns::Entity::find()
            .filter(trajectory_patterns::Column::SuccessRate.gte(min_sr))
            .order_by_desc(trajectory_patterns::Column::SuccessRate)
            .all(self.db.as_ref())
            .await?;
        let end = limit.unwrap_or(models.len()).min(models.len());
        Ok(models.iter().take(end).map(model_to_traj_pattern).collect())
    }

    // ── Workflow Reflections（优化 3：反思历史持久化） ──
    //
    // 由 `WorkflowReflectorImpl::with_storage()` 注入 storage 后，在每次
    // `reflect()` / `reflect_node()` 内调用 `save_workflow_reflection` 落库。
    // `WorkflowOptimizer` / `WorkflowEvolver` 通过 `get_workflow_reflections`
    // 读取跨会话历史反思驱动优化（替代内存 `get_history` 的进程内限制）。

    /// 把单次反思落库。
    ///
    /// - `workflow_id`：工作流 ID（用于按模板聚合历史）
    /// - `template_id`：可选模板 ID（来自 `WorkflowExecutionRecord.template_id`）
    /// - `reflection`：反思结果（含 quality_score / patterns / metadata）
    ///
    /// 主键 `id` 使用 `uuid`（每条反思独立 ID，与 `Reflection.task_id` 解耦，
    /// 因为 `task_id = execution_id` 可能在同一工作流的多次反思中重复——
    /// 节点级反思也以 `execution_id` 作为 `task_id`）。
    pub async fn save_workflow_reflection(
        &self,
        workflow_id: &str,
        template_id: Option<&str>,
        reflection: &axagent_harness::reflection_types::Reflection,
    ) -> Result<()> {
        let error_patterns_json =
            serde_json::to_string(&reflection.error_patterns).unwrap_or_else(|_| "[]".to_string());
        let reusable_patterns_json = serde_json::to_string(&reflection.reusable_patterns)
            .unwrap_or_else(|_| "[]".to_string());
        let metadata_json = reflection
            .metadata
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()))
            .unwrap_or_else(|| "{}".to_string());
        let now = Utc::now().to_rfc3339();

        trajectory_workflow_reflections::Entity::insert(
            trajectory_workflow_reflections::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                workflow_id: Set(workflow_id.to_string()),
                execution_id: Set(reflection.task_id.clone()),
                template_id: Set(template_id.map(|s| s.to_string())),
                quality_score: Set(i32::from(reflection.quality_score)),
                summary: Set(reflection.overall_summary.clone()),
                error_patterns_json: Set(error_patterns_json),
                reusable_patterns_json: Set(reusable_patterns_json),
                metadata_json: Set(metadata_json),
                timestamp: Set(reflection.timestamp.to_rfc3339()),
                created_at: Set(now),
            },
        )
        .exec(self.db.as_ref())
        .await?;
        Ok(())
    }

    /// 查询某工作流的最近 N 条反思（按时间戳倒序）。
    ///
    /// 用于 `WorkflowOptimizer::suggest()` / `WorkflowEvolver::run()` 读取跨会话历史，
    /// 替代 `WorkflowReflector::get_history()` 的内存限制（默认上限 100 条 / workflow）。
    pub async fn get_workflow_reflections(
        &self,
        workflow_id: &str,
        limit: usize,
    ) -> Result<Vec<axagent_harness::reflection_types::Reflection>> {
        use axagent_harness::reflection_types::{QualityMetrics, Reflection};

        let models = trajectory_workflow_reflections::Entity::find()
            .filter(trajectory_workflow_reflections::Column::WorkflowId.eq(workflow_id))
            .order_by_desc(trajectory_workflow_reflections::Column::Timestamp)
            .all(self.db.as_ref())
            .await?;

        let end = limit.min(models.len());
        Ok(models
            .into_iter()
            .take(end)
            .map(|m| {
                let error_patterns: Vec<String> =
                    serde_json::from_str(&m.error_patterns_json).unwrap_or_default();
                let reusable_patterns: Vec<String> =
                    serde_json::from_str(&m.reusable_patterns_json).unwrap_or_default();
                let metadata: Option<serde_json::Value> =
                    serde_json::from_str(&m.metadata_json).ok();
                let timestamp = chrono::DateTime::parse_from_rfc3339(&m.timestamp)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                // 重建 Reflection：持久化字段不包含 quality_analysis /
                // efficiency_analysis / knowledge_suggestions / improvement_suggestions
                // / quality_metrics（这些字段在重载时丢失，置为空值）。
                // 核心驱动字段（quality_score / patterns / metadata / summary）完整保留。
                Reflection {
                    task_id: m.execution_id,
                    timestamp,
                    quality_score: m.quality_score.clamp(0, 255) as u8,
                    quality_analysis: String::new(),
                    efficiency_analysis: String::new(),
                    error_patterns,
                    reusable_patterns,
                    knowledge_suggestions: Vec::new(),
                    improvement_suggestions: Vec::new(),
                    overall_summary: m.summary,
                    quality_metrics: None::<QualityMetrics>,
                    metadata,
                }
            })
            .collect())
    }

    // ── Skills ──

    pub async fn save_skill(&self, skill: &Skill) -> Result<()> {
        trajectory_skills::Entity::insert(trajectory_skills::ActiveModel {
            id: Set(skill.id.clone()),
            name: Set(skill.name.clone()),
            description: Set(skill.description.clone()),
            skill_type: Set(skill.category.clone()),
            content: Set(skill.content.clone()),
            category: Set(skill.category.clone()),
            tags: Set(serde_json::to_string(&skill.tags)?),
            scenarios: Set(serde_json::to_string(&skill.scenarios)?),
            parameters: Set(serde_json::json!({}).to_string()),
            created_at: Set(skill.created_at.to_rfc3339()),
            updated_at: Set(skill.updated_at.to_rfc3339()),
            usage_count: Set(skill.total_usages as i32),
            success_rate: Set(skill.success_rate),
            avg_execution_time_ms: Set(skill.avg_execution_time_ms as i64),
            consecutive_failures: Set(skill.consecutive_failures as i32),
            last_failure_at: Set(skill.last_failure_at.map(|dt| dt.to_rfc3339())),
        })
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(trajectory_skills::Column::Id)
                .update_columns([
                    trajectory_skills::Column::Name,
                    trajectory_skills::Column::Content,
                    trajectory_skills::Column::UpdatedAt,
                    trajectory_skills::Column::UsageCount,
                    trajectory_skills::Column::SuccessRate,
                    trajectory_skills::Column::AvgExecutionTimeMs,
                    trajectory_skills::Column::ConsecutiveFailures,
                    trajectory_skills::Column::LastFailureAt,
                ])
                .to_owned(),
        )
        .exec(self.db.as_ref())
        .await?;
        let _ = self.index_skill_fts(skill).await;
        Ok(())
    }

    pub async fn get_skill(&self, id: &str) -> Result<Option<Skill>> {
        Ok(trajectory_skills::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .map(|s| model_to_skill(&s)))
    }

    pub async fn get_skills(&self) -> Result<Vec<Skill>> {
        Ok(trajectory_skills::Entity::find()
            .order_by_desc(trajectory_skills::Column::UsageCount)
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(model_to_skill)
            .collect())
    }

    /// P1-3: 级联删除 skills + 关联 skill_executions + FTS
    pub async fn delete_skill(&self, id: &str) -> Result<()> {
        let txn = self.db.begin().await?;
        trajectory_skill_executions::Entity::delete_many()
            .filter(trajectory_skill_executions::Column::SkillId.eq(id))
            .exec(&txn)
            .await?;
        trajectory_skills::Entity::delete_by_id(id).exec(&txn).await?;
        txn.commit().await?;
        let _ = self.delete_skill_fts(id).await;
        info!("Deleted skill {}", id);
        Ok(())
    }

    pub async fn record_skill_execution(
        &self,
        sid: &str,
        tid: Option<&str>,
        success: bool,
        et: u64,
        ia: Option<&serde_json::Value>,
        or: Option<&serde_json::Value>,
    ) -> Result<()> {
        trajectory_skill_executions::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            skill_id: Set(sid.to_string()),
            trajectory_id: Set(tid.map(|s| s.to_string())),
            success: Set(success as i32),
            execution_time_ms: Set(et as i64),
            created_at: Set(Utc::now().to_rfc3339()),
            input_args: Set(ia.map(|v| serde_json::to_string(v).unwrap_or_default())),
            output_result: Set(or.map(|v| serde_json::to_string(v).unwrap_or_default())),
        }
        .insert(self.db.as_ref())
        .await?;

        // P1: 同步更新 skill 的统计字段（total_usages/success_rate/avg_execution_time/
        // consecutive_failures/last_failure_at）。这里直接在数据库层做增量更新，
        // 避免先 read 再 write 的竞态。读取 skill 时由 model_to_skill 还原这些字段。
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        // 累加使用次数
        let _ = trajectory_skills::Entity::update_many()
            .col_expr(
                trajectory_skills::Column::UsageCount,
                sea_orm::sea_query::Expr::col(trajectory_skills::Column::UsageCount).add(1),
            )
            .col_expr(
                trajectory_skills::Column::AvgExecutionTimeMs,
                sea_orm::sea_query::Expr::col(trajectory_skills::Column::AvgExecutionTimeMs)
                    .add(et as i64)
                    .div(2),
            )
            .col_expr(
                trajectory_skills::Column::UpdatedAt,
                sea_orm::sea_query::Expr::value(now_str.clone()),
            )
            .filter(trajectory_skills::Column::Id.eq(sid))
            .exec(self.db.as_ref())
            .await;

        // 根据 success 更新 success_rate 和 consecutive_failures
        // 简化处理：success=true 视为成功（清零失败计数），false 视为失败（累加）
        if success {
            let _ = trajectory_skills::Entity::update_many()
                .col_expr(
                    trajectory_skills::Column::ConsecutiveFailures,
                    sea_orm::sea_query::Expr::value(0i32),
                )
                .col_expr(
                    trajectory_skills::Column::SuccessRate,
                    // 简化：success=true 时把 success_rate 推向 1.0（保留旧值 70% + 30%）
                    sea_orm::sea_query::Expr::col(trajectory_skills::Column::SuccessRate)
                        .mul(0.7)
                        .add(0.3),
                )
                .filter(trajectory_skills::Column::Id.eq(sid))
                .exec(self.db.as_ref())
                .await;
        } else {
            let _ = trajectory_skills::Entity::update_many()
                .col_expr(
                    trajectory_skills::Column::ConsecutiveFailures,
                    sea_orm::sea_query::Expr::col(trajectory_skills::Column::ConsecutiveFailures)
                        .add(1),
                )
                .col_expr(
                    trajectory_skills::Column::LastFailureAt,
                    sea_orm::sea_query::Expr::value(now_str),
                )
                .col_expr(
                    trajectory_skills::Column::SuccessRate,
                    // 简化：success=false 时把 success_rate 推向 0.0（保留旧值 70%）
                    sea_orm::sea_query::Expr::col(trajectory_skills::Column::SuccessRate).mul(0.7),
                )
                .filter(trajectory_skills::Column::Id.eq(sid))
                .exec(self.db.as_ref())
                .await;
        }

        Ok(())
    }

    // ── Entities (stored in knowledge_entities table, v101 merge) ──

    const TRAJECTORY_KB_ID: &str = "__sys_trajectory__";

    pub async fn save_entity(&self, e: &Entity) -> Result<()> {
        use knowledge_entities::Column;
        use sea_orm::sea_query::OnConflict;

        let now_ts = Utc::now().timestamp();
        knowledge_entities::Entity::insert(knowledge_entities::ActiveModel {
            id: Set(e.id.clone()),
            knowledge_base_id: Set(Self::TRAJECTORY_KB_ID.to_string()),
            name: Set(e.name.clone()),
            entity_type: Set(serde_json::to_string(&e.entity_type).unwrap_or_default()),
            description: Set(None),
            source_path: Set(String::new()),
            source_language: Set(None),
            properties: Set(serde_json::Value::Object(
                e.properties.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            )),
            lifecycle: Set(None),
            behaviors: Set(None),
            metadata: Set(None),
            created_at: Set(now_ts),
            updated_at: Set(now_ts),
            aliases: Set(serde_json::to_string(&e.aliases).unwrap_or_else(|_| "[]".to_string())),
            mention_count: Set(e.mention_count as i32),
            confidence: Set(e.confidence),
            first_seen_at: Set(Some(e.first_seen_at.to_rfc3339())),
            last_seen_at: Set(Some(e.last_seen_at.to_rfc3339())),
            source_type: Set(String::from("knowledge_base")),
            source_id: Set(String::new()),
            node_type: Set(String::from("entity")),
            external_id: Set(None),
        })
        .on_conflict(
            OnConflict::column(knowledge_entities::Column::Id)
                .update_columns([
                    Column::Name,
                    Column::LastSeenAt,
                    Column::MentionCount,
                    Column::Confidence,
                    Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(self.db.as_ref())
        .await?;
        Ok(())
    }

    pub async fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        Ok(knowledge_entities::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .map(|e| ke_to_entity(&e)))
    }

    pub async fn get_all_entities(&self) -> Result<Vec<Entity>> {
        Ok(knowledge_entities::Entity::find()
            .filter(knowledge_entities::Column::KnowledgeBaseId.eq(Self::TRAJECTORY_KB_ID))
            .order_by_desc(knowledge_entities::Column::UpdatedAt)
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(ke_to_entity)
            .collect())
    }

    pub async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<Entity>> {
        let pattern = format!("%{}%", query);
        Ok(knowledge_entities::Entity::find()
            .filter(
                knowledge_entities::Column::KnowledgeBaseId
                    .eq(Self::TRAJECTORY_KB_ID)
                    .and(knowledge_entities::Column::Name.like(&pattern)),
            )
            .all(self.db.as_ref())
            .await?
            .iter()
            .take(limit)
            .map(ke_to_entity)
            .collect())
    }

    /// P1-3: 删除实体时级联删除其所有 relationships
    pub async fn delete_entity(&self, id: &str) -> Result<()> {
        let txn = self.db.begin().await?;
        knowledge_relations::Entity::delete_many()
            .filter(
                knowledge_relations::Column::SourceEntityId
                    .eq(id)
                    .or(knowledge_relations::Column::TargetEntityId.eq(id)),
            )
            .exec(&txn)
            .await?;
        knowledge_entities::Entity::delete_by_id(id).exec(&txn).await?;
        txn.commit().await?;
        Ok(())
    }

    // ── Relationships (stored in knowledge_relations table, v101 merge) ──

    pub async fn save_relationship(&self, rel: &Relationship) -> Result<()> {
        use knowledge_relations::Column;
        use sea_orm::sea_query::OnConflict;

        let now_ts = Utc::now().timestamp();
        knowledge_relations::Entity::insert(knowledge_relations::ActiveModel {
            id: Set(rel.id.clone()),
            knowledge_base_id: Set(Self::TRAJECTORY_KB_ID.to_string()),
            source_entity_id: Set(rel.source_id.clone()),
            target_entity_id: Set(rel.target_id.clone()),
            relation_type: Set(serde_json::to_string(&rel.relation_type).unwrap_or_default()),
            description: Set(None),
            properties: Set(if rel.properties.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(
                    rel.properties.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                ))
            }),
            metadata: Set(None),
            created_at: Set(now_ts),
            updated_at: Set(now_ts),
            weight: Set(rel.weight),
            source_type: Set(String::from("knowledge_base")),
            source_id: Set(String::new()),
        })
        .on_conflict(
            OnConflict::column(knowledge_relations::Column::Id)
                .update_columns([Column::Weight, Column::UpdatedAt])
                .to_owned(),
        )
        .exec(self.db.as_ref())
        .await?;
        Ok(())
    }

    pub async fn get_relationships_by_entity(&self, eid: &str) -> Result<Vec<Relationship>> {
        Ok(knowledge_relations::Entity::find()
            .filter(
                knowledge_relations::Column::SourceEntityId
                    .eq(eid)
                    .or(knowledge_relations::Column::TargetEntityId.eq(eid)),
            )
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(kr_to_relationship)
            .collect())
    }

    pub async fn get_all_relationships(&self) -> Result<Vec<Relationship>> {
        Ok(knowledge_relations::Entity::find()
            .filter(knowledge_relations::Column::KnowledgeBaseId.eq(Self::TRAJECTORY_KB_ID))
            .order_by_desc(knowledge_relations::Column::CreatedAt)
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(kr_to_relationship)
            .collect())
    }

    pub async fn delete_relationship(&self, id: &str) -> Result<()> {
        knowledge_relations::Entity::delete_by_id(id).exec(self.db.as_ref()).await?;
        Ok(())
    }

    // ── Sessions ──

    pub async fn save_session(&self, s: &TrajectorySession) -> Result<()> {
        trajectory_sessions::Entity::insert(trajectory_sessions::ActiveModel {
            id: Set(s.id.clone()),
            title: Set(s.title.clone()),
            platform: Set(s.platform.clone()),
            user_id: Set(s.user_id.clone()),
            model: Set(s.model.clone()),
            system_prompt: Set(s.system_prompt.clone()),
            created_at: Set(s.created_at.to_rfc3339()),
            updated_at: Set(s.updated_at.to_rfc3339()),
            parent_session_id: Set(s.parent_session_id.clone()),
            token_input: Set(s.token_input),
            token_output: Set(s.token_output),
        })
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(trajectory_sessions::Column::Id)
                .update_columns([
                    trajectory_sessions::Column::Title,
                    trajectory_sessions::Column::UpdatedAt,
                    trajectory_sessions::Column::TokenInput,
                    trajectory_sessions::Column::TokenOutput,
                ])
                .to_owned(),
        )
        .exec(self.db.as_ref())
        .await?;
        Ok(())
    }

    pub async fn get_session(&self, id: &str) -> Result<Option<TrajectorySession>> {
        Ok(trajectory_sessions::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .map(|s| model_to_sess(&s)))
    }

    pub async fn get_all_sessions(&self) -> Result<Vec<TrajectorySession>> {
        Ok(trajectory_sessions::Entity::find()
            .order_by_desc(trajectory_sessions::Column::UpdatedAt)
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(model_to_sess)
            .collect())
    }

    pub async fn update_session(&self, id: &str, updates: &SessionUpdate) -> Result<()> {
        let m = trajectory_sessions::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .context("Session not found")?;
        let mut am: trajectory_sessions::ActiveModel = m.into_active_model();
        if let Some(ref t) = updates.title {
            am.title = Set(t.clone());
        }
        if let Some(ti) = updates.token_input {
            am.token_input = Set(ti);
        }
        if let Some(to) = updates.token_output {
            am.token_output = Set(to);
        }
        am.updated_at = Set(Utc::now().to_rfc3339());
        am.update(self.db.as_ref()).await?;
        Ok(())
    }

    /// P1-3: 级联删除 session → 该 session 的所有 trajectories
    /// (trajectories 通过 session_id 关联；trajectory_steps/rewards 由 delete_trajectory 自身级联)
    pub async fn delete_session(&self, id: &str) -> Result<()> {
        // 先查出该 session 的所有 trajectory
        let traj_ids: Vec<String> = trajectories::Entity::find()
            .filter(trajectories::Column::SessionId.eq(id))
            .all(self.db.as_ref())
            .await?
            .into_iter()
            .map(|t| t.id)
            .collect();
        // 级联删除每条 trajectory
        for tid in &traj_ids {
            let _ = self.delete_trajectory(tid).await;
        }
        // 删除该 session 的所有 messages
        trajectory_messages::Entity::delete_many()
            .filter(trajectory_messages::Column::SessionId.eq(id))
            .exec(self.db.as_ref())
            .await?;
        // 最后删除 session 自身
        trajectory_sessions::Entity::delete_by_id(id).exec(self.db.as_ref()).await?;
        Ok(())
    }

    // ── Messages ──

    pub async fn save_message(&self, msg: &Message) -> Result<()> {
        trajectory_messages::ActiveModel {
            id: Set(msg.id.clone()),
            session_id: Set(msg.session_id.clone()),
            role: Set(msg.role.clone()),
            content: Set(msg.content.clone()),
            tool_calls: Set(msg.tool_calls.clone()),
            tool_results: Set(msg.tool_results.clone()),
            usage: Set(msg.usage.clone()),
            created_at: Set(msg.created_at.to_rfc3339()),
        }
        .insert(self.db.as_ref())
        .await?;
        let _ = self.index_message_fts(msg).await;
        Ok(())
    }

    pub async fn get_messages_by_session(&self, sid: &str) -> Result<Vec<Message>> {
        Ok(trajectory_messages::Entity::find()
            .filter(trajectory_messages::Column::SessionId.eq(sid))
            .order_by_asc(trajectory_messages::Column::CreatedAt)
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(model_to_msg)
            .collect())
    }

    pub async fn search_messages(&self, query: &str, limit: usize) -> Result<Vec<Message>> {
        Ok(trajectory_messages::Entity::find()
            .filter(trajectory_messages::Column::Content.like(format!("%{}%", query)))
            .order_by_desc(trajectory_messages::Column::CreatedAt)
            .all(self.db.as_ref())
            .await?
            .iter()
            .take(limit)
            .map(model_to_msg)
            .collect())
    }

    // ── Memories (stored in memory_items table, v101 merge) ──

    const TRAJECTORY_MEM_NS_ID: &str = "__sys_trajectory_memory__";

    pub async fn get_all_memories(&self) -> Result<Vec<crate::memory::MemoryEntry>> {
        use sea_orm::QueryFilter;
        Ok(memory_items::Entity::find()
            .filter(memory_items::Column::NamespaceId.eq(Self::TRAJECTORY_MEM_NS_ID))
            .all(self.db.as_ref())
            .await?
            .into_iter()
            .map(|m| crate::memory::MemoryEntry {
                id: m.id,
                content: m.content,
                memory_type: m.title, // memory_items.title maps to memory_type
                tier: crate::memory::MemoryTier::from_str(&m.tier),
                importance: m.importance,
                access_count: m.access_count as u64,
                last_accessed: m.last_accessed.unwrap_or(0),
                decay_rate: m.decay_rate,
                created_at: 0,
                updated_at: m.updated_at.parse().unwrap_or(0),
                expires_at: m.expires_at,
                nature: crate::memory::MemoryNature::from_str(&m.memory_nature),
                provenance: Some(crate::memory::MemoryProvenance {
                    conversation_id: m.source_conversation_id,
                    message_id: m.source_message_id,
                    extraction_method: "unknown".to_string(),
                }),
                tags: serde_json::from_str(&m.tags).unwrap_or_default(),
                namespace_id: Some(m.namespace_id),
            })
            .collect())
    }

    pub async fn save_memory(&self, mem: &crate::memory::MemoryEntry) -> Result<()> {
        use memory_items::Column;
        use sea_orm::sea_query::OnConflict;

        let source_conv_id = mem.provenance.as_ref().and_then(|p| p.conversation_id.clone());
        let source_msg_id = mem.provenance.as_ref().and_then(|p| p.message_id.clone());
        let now = chrono::Utc::now().timestamp_millis().to_string();

        memory_items::Entity::insert(memory_items::ActiveModel {
            id: Set(mem.id.clone()),
            namespace_id: Set(Self::TRAJECTORY_MEM_NS_ID.to_string()),
            title: Set(mem.memory_type.clone()),
            content: Set(mem.content.clone()),
            source: Set(source_conv_id.clone().unwrap_or_else(|| "trajectory".to_string())),
            index_status: Set("ready".to_string()),
            index_error: Set(None),
            updated_at: Set(now.clone()),
            tier: Set(mem.tier.as_str().to_string()),
            importance: Set(mem.importance),
            access_count: Set(mem.access_count as i32),
            last_accessed: Set(Some(mem.last_accessed)),
            decay_rate: Set(mem.decay_rate),
            expires_at: Set(mem.expires_at),
            source_conversation_id: Set(source_conv_id),
            source_message_id: Set(source_msg_id),
            memory_nature: Set(mem.nature.as_str().to_string()),
            tags: Set(serde_json::to_string(&mem.tags).unwrap_or_else(|_| "[]".to_string())),
            // v108: 自进化闭环 — trajectory 存储默认未确认 + 空适用范围
            applicability_tags: Set("[]".to_string()),
            confirmed: Set(0),
        })
        .on_conflict(
            OnConflict::column(memory_items::Column::Id)
                .update_columns([
                    Column::Content,
                    Column::UpdatedAt,
                    Column::Tier,
                    Column::Importance,
                    Column::AccessCount,
                    Column::LastAccessed,
                    Column::DecayRate,
                    Column::ExpiresAt,
                    Column::MemoryNature,
                    Column::Tags,
                    Column::SourceConversationId,
                    Column::SourceMessageId,
                ])
                .to_owned(),
        )
        .exec(self.db.as_ref())
        .await?;
        Ok(())
    }

    /// P1-3: 删除 memory 时也清理 FTS 索引
    pub async fn delete_memory(&self, id: &str) -> Result<()> {
        memory_items::Entity::delete_by_id(id).exec(self.db.as_ref()).await?;
        let _ = self.delete_memory_fts(id).await;
        Ok(())
    }

    // ── Learned Patterns ──

    pub async fn save_learning_pattern(&self, p: &Pattern) -> Result<()> {
        trajectory_learned_patterns::Entity::insert(trajectory_learned_patterns::ActiveModel {
            id: Set(p.id.clone()),
            pattern: Set(p.pattern.clone()),
            pattern_type: Set(p.pattern_type.clone()),
            success: Set(p.success),
            failure: Set(p.failure),
            last_used: Set(p.last_used.to_rfc3339()),
            created_at: Set(p.created_at.to_rfc3339()),
            metadata: Set(p.metadata.clone()),
        })
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(trajectory_learned_patterns::Column::Id)
                .update_columns([
                    trajectory_learned_patterns::Column::Success,
                    trajectory_learned_patterns::Column::Failure,
                    trajectory_learned_patterns::Column::LastUsed,
                ])
                .to_owned(),
        )
        .exec(self.db.as_ref())
        .await?;
        Ok(())
    }

    pub async fn get_patterns_list(&self) -> Result<Vec<Pattern>> {
        Ok(trajectory_learned_patterns::Entity::find()
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(|p| Pattern {
                id: p.id.clone(),
                pattern: p.pattern.clone(),
                pattern_type: p.pattern_type.clone(),
                success: p.success,
                failure: p.failure,
                last_used: chrono::DateTime::parse_from_rfc3339(&p.last_used)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                created_at: chrono::DateTime::parse_from_rfc3339(&p.created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                metadata: p.metadata.clone(),
            })
            .collect())
    }

    pub async fn update_pattern_stats(&self, id: &str, sd: i32, fd: i32) -> Result<()> {
        if let Some(m) =
            trajectory_learned_patterns::Entity::find_by_id(id).one(self.db.as_ref()).await?
        {
            let mut am: trajectory_learned_patterns::ActiveModel = m.into_active_model();
            am.success = Set(am.success.take().unwrap_or(0) + sd);
            am.failure = Set(am.failure.take().unwrap_or(0) + fd);
            am.last_used = Set(Utc::now().to_rfc3339());
            am.update(self.db.as_ref()).await?;
        }
        Ok(())
    }

    // ── Preferences ──

    pub async fn save_preference(&self, pref: &Preference) -> Result<()> {
        trajectory_preferences::Entity::insert(trajectory_preferences::ActiveModel {
            id: Set(pref.id.clone()),
            key: Set(pref.key.clone()),
            value: Set(pref.value.clone()),
            confidence: Set(pref.confidence),
            updated_at: Set(pref.updated_at.to_rfc3339()),
        })
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(trajectory_preferences::Column::Key)
                .update_columns([
                    trajectory_preferences::Column::Value,
                    trajectory_preferences::Column::Confidence,
                    trajectory_preferences::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(self.db.as_ref())
        .await?;
        Ok(())
    }

    pub async fn get_preferences_list(&self) -> Result<Vec<Preference>> {
        Ok(trajectory_preferences::Entity::find()
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(|p| Preference {
                id: p.id.clone(),
                key: p.key.clone(),
                value: p.value.clone(),
                confidence: p.confidence,
                updated_at: chrono::DateTime::parse_from_rfc3339(&p.updated_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
            .collect())
    }

    pub async fn update_preference_by_key(&self, key: &str, updates: &Preference) -> Result<()> {
        if let Some(m) = trajectory_preferences::Entity::find()
            .filter(trajectory_preferences::Column::Key.eq(key))
            .one(self.db.as_ref())
            .await?
        {
            let mut am: trajectory_preferences::ActiveModel = m.into_active_model();
            am.value = Set(updates.value.clone());
            am.confidence = Set(updates.confidence);
            am.updated_at = Set(Utc::now().to_rfc3339());
            am.update(self.db.as_ref()).await?;
        }
        Ok(())
    }

    // ── Utilities ──

    pub async fn get_trajectory_stats(&self) -> Result<TrajectoryStatistics> {
        let trajs = self.get_trajectories(None).await?;
        let total = trajs.len();
        if total == 0 {
            return Ok(TrajectoryStatistics {
                total_trajectories: 0,
                total_sessions: 0,
                total_patterns: 0,
                avg_quality: 0.0,
                avg_value_score: 0.0,
                success_rate: 0.0,
                recent_trajectories: 0,
            });
        }
        let mut tq = 0.0;
        let mut tv = 0.0;
        let mut sc = 0;
        for t in &trajs {
            tq += t.quality.overall;
            tv += t.value_score;
            if t.outcome == TrajectoryOutcome::Success {
                sc += 1;
            }
        }
        Ok(TrajectoryStatistics {
            total_trajectories: total,
            total_sessions: 0,
            total_patterns: 0,
            avg_quality: tq / total as f64,
            avg_value_score: tv / total as f64,
            success_rate: sc as f64 / total as f64,
            recent_trajectories: total.min(10),
        })
    }

    pub async fn export_trajectories(
        &self,
        opts: &TrajectoryExportOptions,
    ) -> Result<Vec<RLTrainingEntry>> {
        Ok(self
            .query_trajectories(&TrajectoryQuery {
                session_id: None,
                user_id: None,
                topic: None,
                min_quality: opts.min_quality,
                min_value_score: opts.min_value_score,
                outcome: opts.outcome_filter,
                time_range: None,
                limit: opts.limit,
            })
            .await?
            .into_iter()
            .map(|t| axagent_harness::trajectory_scorer::TrajectoryScorer::export_as_rl(&t))
            .collect())
    }

    /// P0-2: 修复嵌套 block_on - 全部用 async 查询
    pub async fn search_trajectories(&self, fts_query: &FTS5Query) -> Result<Vec<String>> {
        // 优先使用 FTS5 全文搜索，不可用时降级为 LIKE 查询
        if let Some(ref fts) = self.fts_searcher {
            let mut query = fts_query.clone();
            query.filter_type = Some("trajectories_fts".to_string());
            match fts.search(query).await {
                Ok(results) if !results.is_empty() => {
                    return Ok(results.into_iter().map(|r| r.id).collect());
                },
                _ => {},
            }
        }
        // 降级：直接 async 查询
        let pattern = format!("%{}%", fts_query.query);
        Ok(trajectories::Entity::find()
            .filter(
                trajectories::Column::Topic
                    .like(&pattern)
                    .or(trajectories::Column::Summary.like(&pattern)),
            )
            .all(self.db.as_ref())
            .await?
            .into_iter()
            .take(fts_query.limit)
            .map(|t| t.id)
            .collect())
    }

    pub fn init_memory_tables(&self) -> Result<()> {
        info!("Memory tables initialized");
        Ok(())
    }
    pub async fn get_all_skills(&self) -> Result<Vec<Skill>> {
        self.get_skills().await
    }
    pub async fn get_all_patterns(&self) -> Result<Vec<TrajectoryPattern>> {
        self.get_patterns().await
    }
    pub async fn get_statistics(&self) -> Result<TrajectoryStatistics> {
        self.get_trajectory_stats().await
    }

    // FTS delegates
    pub async fn create_fts_tables(&self) -> Result<()> {
        if let Some(ref fts) = self.fts_searcher {
            fts.create_fts_tables().await
        } else {
            Ok(())
        }
    }
    pub async fn search_fts(&self, query: FTS5Query) -> Result<Vec<FTS5Result>> {
        if let Some(ref fts) = self.fts_searcher {
            fts.search(query).await
        } else {
            Ok(Vec::new())
        }
    }
    pub async fn index_trajectory_fts(&self, t: &Trajectory) -> Result<()> {
        if let Some(ref fts) = self.fts_searcher {
            fts.index_trajectory(t, &t.session_id).await
        } else {
            Ok(())
        }
    }
    pub async fn index_skill_fts(&self, skill: &Skill) -> Result<()> {
        if let Some(ref fts) = self.fts_searcher {
            fts.index_skill(
                &skill.id,
                &skill.name,
                &skill.description,
                &skill.content,
                &skill.category,
                &skill.tags,
            )
            .await
        } else {
            Ok(())
        }
    }
    pub async fn index_message_fts(&self, msg: &Message) -> Result<()> {
        if let Some(ref fts) = self.fts_searcher {
            fts.index_message(msg).await
        } else {
            Ok(())
        }
    }
    pub async fn index_memory_fts(
        &self,
        id: &str,
        mt: &str,
        content: &str,
        entities: &[String],
    ) -> Result<()> {
        if let Some(ref fts) = self.fts_searcher {
            fts.index_memory(id, mt, content, entities).await
        } else {
            Ok(())
        }
    }
    pub async fn delete_memory_fts(&self, id: &str) -> Result<()> {
        // v101: trajectory_memories_fts was dropped; memory_items FTS is TBD
        // Gracefully handle missing FTS table.
        if let Some(ref fts) = self.fts_searcher {
            let _ = fts.delete_from_fts("memory_items_fts", id).await;
        }
        Ok(())
    }

    pub async fn delete_skill_fts(&self, id: &str) -> Result<()> {
        if let Some(ref fts) = self.fts_searcher {
            fts.delete_from_fts("trajectory_skills_fts", id).await
        } else {
            Ok(())
        }
    }
    pub async fn delete_trajectory_fts(&self, id: &str) -> Result<()> {
        if let Some(ref fts) = self.fts_searcher {
            fts.delete_from_fts("trajectories_fts", id).await
        } else {
            Ok(())
        }
    }
    pub async fn optimize_fts(&self) -> Result<()> {
        if let Some(ref fts) = self.fts_searcher {
            fts.optimize().await
        } else {
            Ok(())
        }
    }

    /// 对 FTS5 索引执行 VACUUM，回收已删除记录占用的磁盘空间。
    /// 与 `optimize_fts`（合并 segments）互补，通常在 cleanup 后调用。
    pub async fn vacuum_fts(&self) -> Result<()> {
        if let Some(ref fts) = self.fts_searcher {
            fts.vacuum().await
        } else {
            Ok(())
        }
    }
}

// ── Model conversion helpers ──

fn model_to_trajectory(
    m: &trajectories::Model,
    steps: Vec<TrajectoryStep>,
    rewards: Vec<RewardSignal>,
) -> Trajectory {
    Trajectory {
        id: m.id.clone(),
        session_id: m.session_id.clone(),
        user_id: m.user_id.clone(),
        agent_name: m.agent_name.clone(),
        topic: m.topic.clone(),
        summary: m.summary.clone(),
        outcome: serde_json::from_str(&format!("\"{}\"", m.outcome))
            .unwrap_or(TrajectoryOutcome::Success),
        duration_ms: m.duration_ms as u64,
        quality: crate::trajectory::TrajectoryQuality {
            overall: m.quality_overall,
            task_completion: m.quality_task_completion,
            tool_efficiency: m.quality_tool_efficiency,
            reasoning_quality: m.quality_reasoning_quality,
            user_satisfaction: m.quality_user_satisfaction,
        },
        value_score: m.value_score,
        patterns: serde_json::from_str(&m.patterns).unwrap_or_default(),
        steps,
        rewards,
        created_at: chrono::DateTime::parse_from_rfc3339(&m.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        replay_count: m.replay_count as u32,
        last_replay_at: m.last_replay_at.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)).ok()
        }),
    }
}

fn model_to_skill(s: &trajectory_skills::Model) -> Skill {
    Skill {
        id: s.id.clone(),
        name: s.name.clone(),
        description: s.description.clone(),
        version: "1.0.0".to_string(),
        content: s.content.clone(),
        category: s.category.clone(),
        tags: serde_json::from_str(&s.tags).unwrap_or_default(),
        platforms: Vec::new(),
        scenarios: serde_json::from_str(&s.scenarios).unwrap_or_default(),
        quality_score: 0.0,
        success_rate: s.success_rate,
        avg_execution_time_ms: s.avg_execution_time_ms as u64,
        total_usages: s.usage_count as u32,
        successful_usages: 0,
        created_at: chrono::DateTime::parse_from_rfc3339(&s.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&s.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        last_used_at: None,
        consecutive_failures: Ord::max(s.consecutive_failures, 0) as u32,
        last_failure_at: s.last_failure_at.as_ref().and_then(|t| {
            chrono::DateTime::parse_from_rfc3339(t).map(|dt| dt.with_timezone(&Utc)).ok()
        }),
        metadata: crate::skill::SkillMetadata::default(),
    }
}

fn model_to_traj_pattern(p: &trajectory_patterns::Model) -> TrajectoryPattern {
    TrajectoryPattern {
        id: p.id.clone(),
        name: p.name.clone(),
        description: p.description.clone(),
        pattern_type: p.pattern_type.clone(),
        trajectory_ids: serde_json::from_str(&p.trajectory_ids).unwrap_or_default(),
        frequency: p.frequency as u32,
        success_rate: p.success_rate,
        average_quality: p.average_quality,
        average_value_score: p.average_value_score,
        reward_profile: serde_json::from_str(&p.reward_profile).unwrap_or_default(),
        created_at: chrono::DateTime::parse_from_rfc3339(&p.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    }
}

// ── 阶段三 T3.1 / T3.5：append-only 证据存储集成测试 ─────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trajectory::{Trajectory, TrajectoryOutcome, TrajectoryStep};

    /// 构造最小 `Trajectory`（内部自动生成 uuid 主键）。
    fn sample_trajectory(session: &str) -> Trajectory {
        Trajectory::new(
            session.to_string(),
            "user-1".to_string(),
            "测试主题".to_string(),
            "测试摘要".to_string(),
            TrajectoryOutcome::Success,
            1000,
            Vec::<TrajectoryStep>::new(),
        )
    }

    /// 阶段三 T3.1：软删除（append-only）——`delete_trajectory` 仅置
    /// `is_invalidated = 1`，证据本体保留；活动查询（get_trajectories）不再可见。
    #[tokio::test]
    async fn delete_trajectory_marks_invalidated_keeps_evidence() {
        let db = axagent_dao::db::create_test_pool().await.expect("测试：创建连接池应成功").conn;
        let storage = TrajectoryStorage::new(Arc::new(db.clone()));
        let t = sample_trajectory("session-1");
        storage.save_trajectory(&t).await.expect("测试：保存轨迹应成功");

        assert_eq!(
            storage.get_trajectories(None).await.expect("测试：查询应成功").len(),
            1,
            "保存后活动查询应可见"
        );

        storage.delete_trajectory(&t.id).await.expect("测试：软删除应成功");

        assert!(
            storage.get_trajectories(None).await.expect("测试：查询应成功").is_empty(),
            "软删除后活动查询应不可见"
        );
        // 证据本体保留（append-only），is_invalidated = 1
        let row = trajectories::Entity::find_by_id(&t.id)
            .one(&db)
            .await
            .expect("测试：查询应成功")
            .expect("证据本体必须保留（append-only，不可物理删除）");
        assert_eq!(row.is_invalidated, 1);
    }

    /// 阶段三 T3.1：重新保存（on_conflict 清除失效标记）恢复活动可见。
    #[tokio::test]
    async fn resave_trajectory_reenables_invalidated_evidence() {
        let db = axagent_dao::db::create_test_pool().await.expect("测试：创建连接池应成功").conn;
        let storage = TrajectoryStorage::new(Arc::new(db.clone()));
        let t = sample_trajectory("session-1");
        storage.save_trajectory(&t).await.expect("测试：保存轨迹应成功");
        storage.delete_trajectory(&t.id).await.expect("测试：软删除应成功");

        storage.save_trajectory(&t).await.expect("测试：重新保存应成功");

        assert_eq!(
            storage.get_trajectories(None).await.expect("测试：查询应成功").len(),
            1,
            "重新保存应重新启用轨迹"
        );
        let row = trajectories::Entity::find_by_id(&t.id)
            .one(&db)
            .await
            .expect("测试：查询应成功")
            .expect("轨迹应存在");
        assert_eq!(row.is_invalidated, 0);
    }
}

fn ke_to_entity(e: &knowledge_entities::Model) -> Entity {
    use crate::memory::EntityType;
    Entity {
        id: e.id.clone(),
        name: e.name.clone(),
        entity_type: serde_json::from_str(&format!("\"{}\"", e.entity_type))
            .unwrap_or(EntityType::Concept),
        properties: match &e.properties {
            serde_json::Value::Object(map) => {
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            },
            _ => std::collections::HashMap::new(),
        },
        aliases: serde_json::from_str(&e.aliases).unwrap_or_default(),
        first_seen_at: e
            .first_seen_at
            .as_ref()
            .and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)).ok()
            })
            .unwrap_or_else(Utc::now),
        last_seen_at: e
            .last_seen_at
            .as_ref()
            .and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)).ok()
            })
            .unwrap_or_else(Utc::now),
        mention_count: e.mention_count as u32,
        confidence: e.confidence,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    }
}

fn kr_to_relationship(r: &knowledge_relations::Model) -> Relationship {
    use crate::memory::RelationshipType;
    Relationship {
        id: r.id.clone(),
        source_id: r.source_entity_id.clone(),
        target_id: r.target_entity_id.clone(),
        relation_type: serde_json::from_str(&format!("\"{}\"", r.relation_type))
            .unwrap_or(RelationshipType::RelatedTo),
        properties: match &r.properties {
            Some(serde_json::Value::Object(map)) => {
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            },
            _ => std::collections::HashMap::new(),
        },
        weight: r.weight,
        created_at: Utc::now(),
    }
}

fn model_to_sess(s: &trajectory_sessions::Model) -> TrajectorySession {
    TrajectorySession {
        id: s.id.clone(),
        title: s.title.clone(),
        platform: s.platform.clone(),
        user_id: s.user_id.clone(),
        model: s.model.clone(),
        system_prompt: s.system_prompt.clone(),
        created_at: chrono::DateTime::parse_from_rfc3339(&s.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&s.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        parent_session_id: s.parent_session_id.clone(),
        token_input: s.token_input,
        token_output: s.token_output,
    }
}

fn model_to_msg(m: &trajectory_messages::Model) -> Message {
    Message {
        id: m.id.clone(),
        session_id: m.session_id.clone(),
        role: m.role.clone(),
        content: m.content.clone(),
        tool_calls: m.tool_calls.clone(),
        tool_results: m.tool_results.clone(),
        usage: m.usage.clone(),
        created_at: chrono::DateTime::parse_from_rfc3339(&m.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    }
}

// ── Trajectory Cleanup Task ──

pub struct TrajectoryCleanupTask {
    storage: Arc<TrajectoryStorage>,
    config: TrajectoryCleanupConfig,
    interval: std::time::Duration,
    handle: Option<tokio::task::JoinHandle<()>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl TrajectoryCleanupTask {
    pub fn new(
        storage: Arc<TrajectoryStorage>,
        config: TrajectoryCleanupConfig,
        interval: std::time::Duration,
    ) -> Self {
        Self { storage, config, interval, handle: None, shutdown_tx: None }
    }

    pub fn start(&mut self) {
        if self.handle.is_some() {
            return;
        }
        let storage = self.storage.clone();
        let config = self.config.clone();
        let interval = self.interval;
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let result = AssertUnwindSafe(async {
                            match storage.cleanup(&config).await {
                                Ok(count) if count > 0 => {
                                    info!("Cleaned up {} old trajectories", count);
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    warn!("[TrajectoryCleanupTask] cleanup failed: {}", e);
                                }
                            }
                        })
                        .catch_unwind()
                        .await;
                        if let Err(p) = result {
                            let msg = if let Some(s) = p.downcast_ref::<String>() {
                                s.clone()
                            } else if let Some(s) = p.downcast_ref::<&'static str>() {
                                (*s).to_owned()
                            } else {
                                "Unknown panic in trajectory cleanup".to_string()
                            };
                            warn!("[TrajectoryCleanupTask] PANIC in cleanup loop: {}", msg);
                        }
                    }
                    _ = &mut shutdown_rx => {
                        info!("Trajectory cleanup task shutting down");
                        break;
                    }
                }
            }
        });
        self.handle = Some(handle);
        self.shutdown_tx = Some(shutdown_tx);
    }

    pub async fn shutdown(self) {
        if let Some(tx) = self.shutdown_tx {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle {
            let _ = handle.await;
        }
    }
}

// ── Public types ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrajectorySession {
    pub id: String,
    pub title: String,
    pub platform: String,
    pub user_id: String,
    pub model: String,
    pub system_prompt: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub parent_session_id: Option<String>,
    pub token_input: i64,
    pub token_output: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionUpdate {
    pub title: Option<String>,
    pub token_input: Option<i64>,
    pub token_output: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<String>,
    pub tool_results: Option<String>,
    pub usage: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Pattern {
    pub id: String,
    pub pattern: String,
    pub pattern_type: String,
    pub success: i32,
    pub failure: i32,
    pub last_used: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Preference {
    pub id: String,
    pub key: String,
    pub value: String,
    pub confidence: f64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrajectoryStatistics {
    pub total_trajectories: usize,
    pub total_sessions: usize,
    pub total_patterns: usize,
    pub avg_quality: f64,
    pub avg_value_score: f64,
    pub success_rate: f64,
    pub recent_trajectories: usize,
}
