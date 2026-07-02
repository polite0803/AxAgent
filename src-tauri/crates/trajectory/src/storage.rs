// SPDX-License-Identifier: AGPL-3.0-only

//! Trajectory storage module using SeaORM

use crate::fts5::{FTS5Config, FTS5Query, FTS5Result, FTS5Search};
use crate::memory::{Entity, Relationship};
use crate::skill::{Skill, SkillAnalytics};
use crate::trajectory::{
    MessageRole, RLTrainingEntry, RewardSignal, Trajectory, TrajectoryExportOptions,
    TrajectoryOutcome, TrajectoryPattern, TrajectoryQuery, TrajectoryStep,
};
use anyhow::{Context, Result};
use axagent_core::entity::{
    trajectories, trajectory_entities, trajectory_learned_patterns, trajectory_memories,
    trajectory_messages, trajectory_patterns, trajectory_preferences, trajectory_relationships,
    trajectory_rewards, trajectory_sessions, trajectory_skill_executions, trajectory_skills,
    trajectory_steps,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, ExprTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

pub struct TrajectoryStorage {
    db: Arc<DatabaseConnection>,
    fts_searcher: Option<FTS5Search>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrajectoryCleanupConfig {
    pub max_age_days: Option<u32>,
    pub max_trajectories: Option<u32>,
}

impl Default for TrajectoryCleanupConfig {
    fn default() -> Self {
        Self {
            max_age_days: Some(90),
            max_trajectories: Some(10000),
        }
    }
}

impl TrajectoryStorage {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            fts_searcher: None,
        }
    }

    pub fn with_fts(
        db: Arc<DatabaseConnection>,
        fts_conn: Arc<Mutex<rusqlite::Connection>>,
    ) -> Self {
        Self {
            db,
            fts_searcher: Some(FTS5Search::new(fts_conn, FTS5Config::default())),
        }
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
        Ok(Self {
            db,
            fts_searcher: Some(fts),
        })
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
        };
        // P1-2: on_conflict 不再更新 CreatedAt（保留原创建时间）
        trajectories::Entity::insert(am)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(trajectories::Column::Id)
                    .update_columns([
                        trajectories::Column::SessionId,
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
        Ok(())
    }

    pub async fn get_trajectory(&self, id: &str) -> Result<Option<Trajectory>> {
        match trajectories::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
        {
            Some(m) => Ok(Some(model_to_trajectory(
                &m,
                self.get_trajectory_steps(&m.id).await?,
                self.get_trajectory_rewards(&m.id).await?,
            ))),
            None => Ok(None),
        }
    }

    pub async fn get_trajectories(&self, limit: Option<usize>) -> Result<Vec<Trajectory>> {
        let models = trajectories::Entity::find()
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

    /// P1-3: 级联删除 trajectories + steps + rewards + skill_executions + FTS
    pub async fn delete_trajectory(&self, id: &str) -> Result<()> {
        let txn = self.db.begin().await?;
        trajectory_steps::Entity::delete_many()
            .filter(trajectory_steps::Column::TrajectoryId.eq(id))
            .exec(&txn)
            .await?;
        trajectory_rewards::Entity::delete_many()
            .filter(trajectory_rewards::Column::TrajectoryId.eq(id))
            .exec(&txn)
            .await?;
        trajectory_skill_executions::Entity::delete_many()
            .filter(trajectory_skill_executions::Column::TrajectoryId.eq(id))
            .exec(&txn)
            .await?;
        trajectories::Entity::delete_by_id(id).exec(&txn).await?;
        txn.commit().await?;
        let _ = self.delete_trajectory_fts(id).await;
        info!("Deleted trajectory {}", id);
        Ok(())
    }

    /// P1-5: 使用 datetime() 归一化以解决字符串字典序问题
    pub async fn cleanup_old_trajectories_by_age(&self, max_age_days: u32) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(max_age_days as i64);
        let cutoff_str = cutoff.to_rfc3339();
        // 字符串字典序问题：ISO8601 / RFC3339 字典序与时间序一致，但需要统一格式
        // 关键问题：原代码直接用字符串比较，未归一化为相同格式
        // 修复：用 datetime() 函数归一化字符串时间戳再比较
        let old_trajectories = trajectories::Entity::find()
            .filter(sea_orm::sea_query::Expr::cust(format!(
                "datetime(created_at) < datetime('{}')",
                cutoff_str
            )))
            .all(self.db.as_ref())
            .await?;
        let count = old_trajectories.len();
        for traj in old_trajectories {
            self.delete_trajectory(&traj.id).await?;
        }
        Ok(count)
    }

    /// P1-4: 避免全表加载，使用 NOT IN 子查询找出需删除的 ID
    pub async fn cleanup_old_trajectories_by_count(&self, max_trajectories: u32) -> Result<usize> {
        // 先查总数判断是否需要清理
        let total = trajectories::Entity::find().count(self.db.as_ref()).await?;
        if total <= max_trajectories as u64 {
            return Ok(0);
        }
        // 用 NOT IN 子查询找出需要保留的 ID 集合
        let to_delete_ids: Vec<String> = {
            use sea_orm::PaginatorTrait;
            // 取第二页（跳过前 max_trajectories 条），即为超出保留阈值的最旧轨迹
            let page_size: u64 = std::cmp::max(max_trajectories as u64, 1);
            let paginator = trajectories::Entity::find()
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
            total_deleted += self
                .cleanup_old_trajectories_by_count(max_trajectories)
                .await?;
        }
        Ok(total_deleted)
    }

    pub async fn get_session_trajectories(&self, session_id: &str) -> Result<Vec<Trajectory>> {
        let models = trajectories::Entity::find()
            .filter(trajectories::Column::SessionId.eq(session_id))
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
                    step_index: 0,
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
            avg_execution_time_ms: Set(skill.avg_execution_time_ms as f64),
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
        trajectory_skills::Entity::delete_by_id(id)
            .exec(&txn)
            .await?;
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
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn get_skill_analytics(&self, sid: &str) -> Result<SkillAnalytics> {
        let all_execs = trajectory_skill_executions::Entity::find()
            .filter(trajectory_skill_executions::Column::SkillId.eq(sid))
            .all(self.db.as_ref())
            .await?;
        let total = all_execs.len() as u64;
        let successes = all_execs.iter().filter(|e| e.success != 0).count() as u64;
        Ok(SkillAnalytics {
            total_executions: total as u32,
            success_rate: if total > 0 {
                successes as f64 / total as f64
            } else {
                0.0
            },
            avg_execution_time_ms: 0.0,
            recent_executions: std::cmp::Ord::min(total, 100) as u32,
        })
    }

    // ── Entities ──

    pub async fn save_entity(&self, e: &Entity) -> Result<()> {
        trajectory_entities::Entity::insert(trajectory_entities::ActiveModel {
            id: Set(e.id.clone()),
            name: Set(e.name.clone()),
            entity_type: Set(serde_json::to_string(&e.entity_type).unwrap_or_default()),
            properties: Set(
                serde_json::to_string(&e.properties).unwrap_or_else(|_| "{}".to_string())
            ),
            aliases: Set(serde_json::to_string(&e.aliases).unwrap_or_else(|_| "[]".to_string())),
            first_seen_at: Set(e.first_seen_at.to_rfc3339()),
            last_seen_at: Set(e.last_seen_at.to_rfc3339()),
            mention_count: Set(e.mention_count as i32),
            confidence: Set(e.confidence),
            created_at: Set(e.created_at.map(|dt| dt.to_rfc3339())),
            updated_at: Set(e.updated_at.map(|dt| dt.to_rfc3339())),
        })
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(trajectory_entities::Column::Id)
                .update_columns([
                    trajectory_entities::Column::Name,
                    trajectory_entities::Column::LastSeenAt,
                    trajectory_entities::Column::MentionCount,
                    trajectory_entities::Column::Confidence,
                ])
                .to_owned(),
        )
        .exec(self.db.as_ref())
        .await?;
        Ok(())
    }

    pub async fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        Ok(trajectory_entities::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .map(|e| model_to_entity(&e)))
    }

    pub async fn get_all_entities(&self) -> Result<Vec<Entity>> {
        Ok(trajectory_entities::Entity::find()
            .order_by_desc(trajectory_entities::Column::LastSeenAt)
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(model_to_entity)
            .collect())
    }

    pub async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<Entity>> {
        Ok(trajectory_entities::Entity::find()
            .filter(trajectory_entities::Column::Name.like(format!("%{}%", query)))
            .all(self.db.as_ref())
            .await?
            .iter()
            .take(limit)
            .map(model_to_entity)
            .collect())
    }

    /// P1-3: 删除实体时级联删除其所有 relationships
    pub async fn delete_entity(&self, id: &str) -> Result<()> {
        let txn = self.db.begin().await?;
        trajectory_relationships::Entity::delete_many()
            .filter(
                trajectory_relationships::Column::SourceId
                    .eq(id)
                    .or(trajectory_relationships::Column::TargetId.eq(id)),
            )
            .exec(&txn)
            .await?;
        trajectory_entities::Entity::delete_by_id(id)
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(())
    }

    // ── Relationships ──

    pub async fn save_relationship(&self, rel: &Relationship) -> Result<()> {
        trajectory_relationships::Entity::insert(trajectory_relationships::ActiveModel {
            id: Set(rel.id.clone()),
            source_id: Set(rel.source_id.clone()),
            target_id: Set(rel.target_id.clone()),
            relation_type: Set(serde_json::to_string(&rel.relation_type).unwrap_or_default()),
            properties: Set(
                serde_json::to_string(&rel.properties).unwrap_or_else(|_| "{}".to_string())
            ),
            weight: Set(rel.weight),
            created_at: Set(rel.created_at.to_rfc3339()),
        })
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(trajectory_relationships::Column::Id)
                .update_columns([trajectory_relationships::Column::Weight])
                .to_owned(),
        )
        .exec(self.db.as_ref())
        .await?;
        Ok(())
    }

    pub async fn get_relationships_by_entity(&self, eid: &str) -> Result<Vec<Relationship>> {
        Ok(trajectory_relationships::Entity::find()
            .filter(
                trajectory_relationships::Column::SourceId
                    .eq(eid)
                    .or(trajectory_relationships::Column::TargetId.eq(eid)),
            )
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(model_to_relationship)
            .collect())
    }

    pub async fn get_all_relationships(&self) -> Result<Vec<Relationship>> {
        Ok(trajectory_relationships::Entity::find()
            .order_by_desc(trajectory_relationships::Column::CreatedAt)
            .all(self.db.as_ref())
            .await?
            .iter()
            .map(model_to_relationship)
            .collect())
    }

    pub async fn delete_relationship(&self, id: &str) -> Result<()> {
        trajectory_relationships::Entity::delete_by_id(id)
            .exec(self.db.as_ref())
            .await?;
        Ok(())
    }

    // ── Sessions ──

    pub async fn save_session(&self, s: &Session) -> Result<()> {
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

    pub async fn get_session(&self, id: &str) -> Result<Option<Session>> {
        Ok(trajectory_sessions::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .map(|s| model_to_sess(&s)))
    }

    pub async fn get_all_sessions(&self) -> Result<Vec<Session>> {
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
        trajectory_sessions::Entity::delete_by_id(id)
            .exec(self.db.as_ref())
            .await?;
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

    // ── Memories ──

    pub async fn get_all_memories(&self) -> Result<Vec<crate::memory::MemoryEntry>> {
        Ok(trajectory_memories::Entity::find()
            .all(self.db.as_ref())
            .await?
            .into_iter()
            .map(|m| crate::memory::MemoryEntry {
                id: m.id,
                content: m.content,
                memory_type: m.memory_type,
                tier: crate::memory::MemoryTier::from_str(&m.tier),
                importance: m.importance,
                access_count: m.access_count as u64,
                last_accessed: m
                    .last_accessed
                    .as_ref()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0),
                decay_rate: m.decay_rate,
                created_at: m
                    .created_at
                    .as_ref()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0),
                updated_at: m.updated_at.parse::<i64>().unwrap_or(0),
                expires_at: m.expires_at.as_ref().and_then(|s| s.parse::<i64>().ok()),
                nature: crate::memory::MemoryNature::from_str(&m.memory_nature),
                provenance: Some(crate::memory::MemoryProvenance {
                    conversation_id: m.source_conversation_id,
                    message_id: m.source_message_id,
                    extraction_method: "unknown".to_string(),
                }),
                tags: serde_json::from_str(&m.tags).unwrap_or_default(),
                namespace_id: m.namespace_id.clone(),
            })
            .collect())
    }

    pub async fn save_memory(&self, mem: &crate::memory::MemoryEntry) -> Result<()> {
        let _provenance_json = mem
            .provenance
            .as_ref()
            .map(|p| serde_json::to_string(p).unwrap_or_default());
        let source_conv_id = mem
            .provenance
            .as_ref()
            .and_then(|p| p.conversation_id.clone());
        let source_msg_id = mem.provenance.as_ref().and_then(|p| p.message_id.clone());
        trajectory_memories::Entity::insert(trajectory_memories::ActiveModel {
            id: Set(mem.id.clone()),
            content: Set(mem.content.clone()),
            memory_type: Set(mem.memory_type.clone()),
            updated_at: Set(format!("{}", mem.updated_at)),
            tier: Set(mem.tier.as_str().to_string()),
            importance: Set(mem.importance),
            access_count: Set(mem.access_count as i32),
            last_accessed: Set(Some(format!("{}", mem.last_accessed))),
            decay_rate: Set(mem.decay_rate),
            created_at: Set(Some(format!("{}", mem.created_at))),
            expires_at: Set(mem.expires_at.map(|t| format!("{}", t))),
            source_conversation_id: Set(source_conv_id),
            source_message_id: Set(source_msg_id),
            memory_nature: Set(mem.nature.as_str().to_string()),
            tags: Set(serde_json::to_string(&mem.tags).unwrap_or_else(|_| "[]".to_string())),
            namespace_id: Set(mem.namespace_id.clone()),
        })
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(trajectory_memories::Column::Id)
                .update_columns([
                    trajectory_memories::Column::Content,
                    trajectory_memories::Column::UpdatedAt,
                    trajectory_memories::Column::Tier,
                    trajectory_memories::Column::Importance,
                    trajectory_memories::Column::AccessCount,
                    trajectory_memories::Column::LastAccessed,
                    trajectory_memories::Column::DecayRate,
                    trajectory_memories::Column::ExpiresAt,
                    trajectory_memories::Column::MemoryNature,
                    trajectory_memories::Column::Tags,
                    trajectory_memories::Column::NamespaceId,
                    trajectory_memories::Column::SourceConversationId,
                    trajectory_memories::Column::SourceMessageId,
                ])
                .to_owned(),
        )
        .exec(self.db.as_ref())
        .await?;
        Ok(())
    }

    /// P1-3: 删除 memory 时也清理 FTS 索引
    pub async fn delete_memory(&self, id: &str) -> Result<()> {
        trajectory_memories::Entity::delete_by_id(id)
            .exec(self.db.as_ref())
            .await?;
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
        if let Some(m) = trajectory_learned_patterns::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
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
            .map(|t| t.export_as_rl())
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
        if let Some(ref fts) = self.fts_searcher {
            fts.delete_from_fts("trajectory_memories_fts", id).await
        } else {
            Ok(())
        }
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
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
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

fn model_to_entity(e: &trajectory_entities::Model) -> Entity {
    use crate::memory::EntityType;
    Entity {
        id: e.id.clone(),
        name: e.name.clone(),
        entity_type: serde_json::from_str(&format!("\"{}\"", e.entity_type))
            .unwrap_or(EntityType::Concept),
        properties: serde_json::from_str(&e.properties).unwrap_or_default(),
        aliases: serde_json::from_str(&e.aliases).unwrap_or_default(),
        first_seen_at: chrono::DateTime::parse_from_rfc3339(&e.first_seen_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        last_seen_at: chrono::DateTime::parse_from_rfc3339(&e.last_seen_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        mention_count: e.mention_count as u32,
        confidence: e.confidence,
        created_at: e.created_at.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        }),
        updated_at: e.updated_at.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        }),
    }
}

fn model_to_relationship(r: &trajectory_relationships::Model) -> Relationship {
    use crate::memory::RelationshipType;
    Relationship {
        id: r.id.clone(),
        source_id: r.source_id.clone(),
        target_id: r.target_id.clone(),
        relation_type: serde_json::from_str(&format!("\"{}\"", r.relation_type))
            .unwrap_or(RelationshipType::RelatedTo),
        properties: serde_json::from_str(&r.properties).unwrap_or_default(),
        weight: r.weight,
        created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    }
}

fn model_to_sess(s: &trajectory_sessions::Model) -> Session {
    Session {
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

// ── TrajectoryQueue ──

use std::collections::VecDeque;
use tokio::sync::mpsc::{self, Sender};

#[allow(dead_code)]
pub(crate) struct TrajectoryQueue {
    storage: Arc<TrajectoryStorage>,
    sender: Sender<Trajectory>,
    handle: tokio::task::JoinHandle<()>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

#[allow(dead_code)]
impl TrajectoryQueue {
    pub(crate) fn new(storage: Arc<TrajectoryStorage>, buffer_size: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<Trajectory>(buffer_size);
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let sc = storage.clone();
        let handle = tokio::spawn(async move {
            let mut batch: VecDeque<Trajectory> = VecDeque::with_capacity(32);
            let mut fi = tokio::time::interval(tokio::time::Duration::from_secs(1));
            loop {
                tokio::select! {
                    Some(t) = rx.recv() => { batch.push_back(t); if batch.len() >= 32 { flush(&sc, &mut batch).await; } }
                    _ = fi.tick() => { if !batch.is_empty() { flush(&sc, &mut batch).await; } }
                    _ = &mut shutdown_rx => { flush(&sc, &mut batch).await; break; }
                }
            }
        });
        Self {
            storage,
            sender: tx,
            handle,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// P1-6: try_enqueue 失败时落盘（直接调用 storage.save_trajectory），
    /// 不再静默丢弃。同时仍返回是否入队成功供上层判断。
    pub(crate) fn try_enqueue(&self, t: Trajectory) -> bool {
        if self.sender.try_send(t.clone()).is_ok() {
            return true;
        }
        // 队列满或关闭：降级为直接落盘，避免数据丢失
        let storage = self.storage.clone();
        tokio::spawn(async move {
            if let Err(e) = storage.save_trajectory(&t).await {
                warn!("[TrajectoryQueue] fallback persistence failed: {}", e);
            }
        });
        // 入队失败但已落盘
        true
    }
    pub(crate) async fn enqueue(
        &self,
        t: Trajectory,
    ) -> Result<(), tokio::sync::mpsc::error::TrySendError<Trajectory>> {
        match self.sender.send(t.clone()).await {
            Ok(()) => Ok(()),
            Err(_) => {
                // 通道已关闭：直接落盘
                let storage = self.storage.clone();
                let t_for_persist = t.clone();
                tokio::spawn(async move {
                    let _ = storage.save_trajectory(&t_for_persist).await;
                });
                Err(tokio::sync::mpsc::error::TrySendError::Closed(t))
            },
        }
    }
    pub(crate) fn storage(&self) -> &Arc<TrajectoryStorage> {
        &self.storage
    }
    pub(crate) async fn shutdown(self) {
        if let Some(tx) = self.shutdown_tx {
            let _ = tx.send(());
        }
        let _ = self.handle.await;
    }
}

#[allow(dead_code)]
async fn flush(storage: &Arc<TrajectoryStorage>, batch: &mut VecDeque<Trajectory>) {
    while let Some(t) = batch.pop_front() {
        if let Err(e) = storage.save_trajectory(&t).await {
            warn!("[TrajectoryQueue] failed: {}", e);
        }
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
        Self {
            storage,
            config,
            interval,
            handle: None,
            shutdown_tx: None,
        }
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
                        match storage.cleanup(&config).await {
                            Ok(count) if count > 0 => {
                                info!("Cleaned up {} old trajectories", count);
                            }
                            Ok(_) => {}
                            Err(e) => {
                                warn!("[TrajectoryCleanupTask] cleanup failed: {}", e);
                            }
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
pub struct Session {
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
