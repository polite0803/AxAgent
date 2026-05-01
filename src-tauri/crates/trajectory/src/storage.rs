//! Trajectory storage module using SeaORM

use axagent_core::entity::{
    trajectories, trajectory_entities, trajectory_learned_patterns,
    trajectory_memories, trajectory_messages, trajectory_patterns, trajectory_preferences,
    trajectory_relationships, trajectory_rewards, trajectory_sessions,
    trajectory_skill_executions, trajectory_skills, trajectory_steps,
};
use crate::fts5::{FTS5Config, FTS5Query, FTS5Result, FTS5Search};
use crate::memory::{Entity, Relationship};
use crate::skill::{Skill, SkillAnalytics};
use crate::trajectory::{
    MessageRole, RLTrainingEntry, RewardSignal, Trajectory, TrajectoryExportOptions,
    TrajectoryOutcome, TrajectoryPattern, TrajectoryQuery, TrajectoryStep,
};
use anyhow::{Context, Result};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

pub struct TrajectoryStorage {
    db: Arc<DatabaseConnection>,
    fts_searcher: Option<FTS5Search>,
}

impl TrajectoryStorage {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db, fts_searcher: None }
    }

    pub fn with_fts(db: Arc<DatabaseConnection>, fts_conn: Arc<std::sync::RwLock<rusqlite::Connection>>) -> Self {
        Self { db, fts_searcher: Some(FTS5Search::new(fts_conn, FTS5Config::default())) }
    }

    fn rt() -> tokio::runtime::Handle {
        tokio::runtime::Handle::try_current()
            .unwrap_or_else(|_| tokio::runtime::Runtime::new().unwrap().handle().clone())
    }

    // ── Trajectories ──

    pub fn save_trajectory(&self, t: &Trajectory) -> Result<()> {
        Self::rt().block_on(async {
            let am = trajectories::ActiveModel {
                id: Set(t.id.clone()), session_id: Set(t.session_id.clone()),
                user_id: Set(t.user_id.clone()), topic: Set(t.topic.clone()),
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
            trajectories::Entity::insert(am)
                .on_conflict(sea_orm::sea_query::OnConflict::column(trajectories::Column::Id)
                    .update_columns([
                        trajectories::Column::SessionId, trajectories::Column::Topic,
                        trajectories::Column::Summary, trajectories::Column::Outcome,
                        trajectories::Column::DurationMs, trajectories::Column::QualityOverall,
                        trajectories::Column::QualityTaskCompletion, trajectories::Column::QualityToolEfficiency,
                        trajectories::Column::QualityReasoningQuality, trajectories::Column::QualityUserSatisfaction,
                        trajectories::Column::ValueScore, trajectories::Column::Patterns,
                        trajectories::Column::CreatedAt, trajectories::Column::ReplayCount,
                        trajectories::Column::LastReplayAt,
                    ]).to_owned())
                .exec(self.db.as_ref()).await?;

            trajectory_steps::Entity::delete_many()
                .filter(trajectory_steps::Column::TrajectoryId.eq(&t.id))
                .exec(self.db.as_ref()).await?;
            for (idx, step) in t.steps.iter().enumerate() {
                trajectory_steps::ActiveModel {
                    trajectory_id: Set(t.id.clone()), step_index: Set(idx as i32),
                    timestamp_ms: Set(step.timestamp_ms as i64),
                    role: Set(format!("{:?}", step.role).to_lowercase()),
                    content: Set(step.content.clone()),
                    reasoning: Set(step.reasoning.clone()),
                    tool_calls: Set(step.tool_calls.as_ref().and_then(|c| serde_json::to_string(c).ok())),
                    tool_results: Set(step.tool_results.as_ref().and_then(|r| serde_json::to_string(r).ok())),
                    ..Default::default()
                }.insert(self.db.as_ref()).await?;
            }

            trajectory_rewards::Entity::delete_many()
                .filter(trajectory_rewards::Column::TrajectoryId.eq(&t.id))
                .exec(self.db.as_ref()).await?;
            for r in &t.rewards {
                trajectory_rewards::ActiveModel {
                    id: Set(Uuid::new_v4().to_string()), trajectory_id: Set(t.id.clone()),
                    reward_type: Set(format!("{:?}", r.reward_type)), value: Set(r.value),
                    created_at: Set(chrono::DateTime::from_timestamp_millis(r.timestamp_ms as i64)
                        .unwrap_or_else(Utc::now).to_rfc3339()),
                }.insert(self.db.as_ref()).await?;
            }
            Ok(())
        })
    }

    pub fn get_trajectory(&self, id: &str) -> Result<Option<Trajectory>> {
        Self::rt().block_on(async {
            Ok(match trajectories::Entity::find_by_id(id).one(self.db.as_ref()).await? {
                Some(m) => Some(model_to_trajectory(&m,
                    self.get_trajectory_steps(&m.id)?, self.get_trajectory_rewards(&m.id)?)),
                None => None,
            })
        })
    }

    pub fn get_trajectories(&self, limit: Option<usize>) -> Result<Vec<Trajectory>> {
        Self::rt().block_on(async {
            let models = trajectories::Entity::find()
                .order_by_desc(trajectories::Column::CreatedAt)
                .all(self.db.as_ref()).await?;
            let mut r = Vec::new();
            let end = limit.unwrap_or(models.len()).min(models.len());
            for m in models.into_iter().take(end) {
                r.push(model_to_trajectory(&m,
                    self.get_trajectory_steps(&m.id)?, self.get_trajectory_rewards(&m.id)?));
            }
            Ok(r)
        })
    }

    pub fn get_session_trajectories(&self, session_id: &str) -> Result<Vec<Trajectory>> {
        Self::rt().block_on(async {
            let models = trajectories::Entity::find()
                .filter(trajectories::Column::SessionId.eq(session_id))
                .order_by_asc(trajectories::Column::CreatedAt)
                .all(self.db.as_ref()).await?;
            let mut r = Vec::new();
            for m in models {
                r.push(model_to_trajectory(&m,
                    self.get_trajectory_steps(&m.id)?, self.get_trajectory_rewards(&m.id)?));
            }
            Ok(r)
        })
    }

    pub fn query_trajectories(&self, query: &TrajectoryQuery) -> Result<Vec<Trajectory>> {
        Self::rt().block_on(async {
            let mut q = trajectories::Entity::find();
            if let Some(ref sid) = query.session_id { q = q.filter(trajectories::Column::SessionId.eq(sid)); }
            if let Some(ref uid) = query.user_id { q = q.filter(trajectories::Column::UserId.eq(uid)); }
            if let Some(ref topic) = query.topic { q = q.filter(trajectories::Column::Topic.like(format!("%{}%", topic))); }
            if let Some(mq) = query.min_quality { q = q.filter(trajectories::Column::QualityOverall.gte(mq)); }
            if let Some(mv) = query.min_value_score { q = q.filter(trajectories::Column::ValueScore.gte(mv)); }
            if let Some(ref outcome) = query.outcome { q = q.filter(trajectories::Column::Outcome.eq(format!("{:?}", outcome))); }
            if let Some((start, end)) = query.time_range {
                q = q.filter(trajectories::Column::CreatedAt.gte(start.to_rfc3339()))
                    .filter(trajectories::Column::CreatedAt.lte(end.to_rfc3339()));
            }
            q = q.order_by_desc(trajectories::Column::CreatedAt);
            let models = q.all(self.db.as_ref()).await?;
            let end = query.limit.unwrap_or(models.len()).min(models.len());
            let mut r = Vec::new();
            for m in models.into_iter().take(end) {
                r.push(model_to_trajectory(&m,
                    self.get_trajectory_steps(&m.id)?, self.get_trajectory_rewards(&m.id)?));
            }
            Ok(r)
        })
    }

    fn get_trajectory_steps(&self, trajectory_id: &str) -> Result<Vec<TrajectoryStep>> {
        Self::rt().block_on(async {
            Ok(trajectory_steps::Entity::find()
                .filter(trajectory_steps::Column::TrajectoryId.eq(trajectory_id))
                .order_by_asc(trajectory_steps::Column::StepIndex)
                .all(self.db.as_ref()).await?.into_iter().map(|s| TrajectoryStep {
                    timestamp_ms: s.timestamp_ms as u64,
                    role: serde_json::from_str(&format!("\"{}\"", s.role)).unwrap_or(MessageRole::Assistant),
                    content: s.content, reasoning: s.reasoning,
                    tool_calls: s.tool_calls.and_then(|c| serde_json::from_str(&c).ok()),
                    tool_results: s.tool_results.and_then(|r| serde_json::from_str(&r).ok()),
                }).collect())
        })
    }

    fn get_trajectory_rewards(&self, trajectory_id: &str) -> Result<Vec<RewardSignal>> {
        Self::rt().block_on(async {
            Ok(trajectory_rewards::Entity::find()
                .filter(trajectory_rewards::Column::TrajectoryId.eq(trajectory_id))
                .all(self.db.as_ref()).await?.into_iter().map(|r| {
                    let rt = match r.reward_type.as_str() {
                        "task_completion" => crate::trajectory::RewardType::TaskCompletion,
                        "tool_efficiency" => crate::trajectory::RewardType::ToolEfficiency,
                        "reasoning_quality" => crate::trajectory::RewardType::ReasoningQuality,
                        _ => crate::trajectory::RewardType::UserFeedback,
                    };
                    let ct = chrono::DateTime::parse_from_rfc3339(&r.created_at)
                        .map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
                    RewardSignal { reward_type: rt, value: r.value, step_index: 0,
                        timestamp_ms: ct.timestamp_millis() as u64, metadata: serde_json::Value::Null }
                }).collect())
        })
    }

    // ── Patterns ──

    pub fn save_pattern(&self, p: &TrajectoryPattern) -> Result<()> {
        Self::rt().block_on(async {
            trajectory_patterns::Entity::insert(trajectory_patterns::ActiveModel {
                id: Set(p.id.clone()), name: Set(p.name.clone()),
                description: Set(p.description.clone()), pattern_type: Set(p.pattern_type.clone()),
                trajectory_ids: Set(serde_json::to_string(&p.trajectory_ids)?),
                frequency: Set(p.frequency as i32), success_rate: Set(p.success_rate),
                average_quality: Set(p.average_quality), average_value_score: Set(p.average_value_score),
                reward_profile: Set(serde_json::to_string(&p.reward_profile)?),
                created_at: Set(p.created_at.to_rfc3339()),
            }).on_conflict(sea_orm::sea_query::OnConflict::column(trajectory_patterns::Column::Id)
                .update_columns([trajectory_patterns::Column::Name, trajectory_patterns::Column::Frequency,
                    trajectory_patterns::Column::SuccessRate, trajectory_patterns::Column::AverageQuality]).to_owned())
                .exec(self.db.as_ref()).await?;
            Ok(())
        })
    }

    pub fn get_patterns(&self) -> Result<Vec<TrajectoryPattern>> {
        Self::rt().block_on(async {
            Ok(trajectory_patterns::Entity::find().order_by_desc(trajectory_patterns::Column::Frequency)
                .all(self.db.as_ref()).await?.iter().map(model_to_traj_pattern).collect())
        })
    }

    pub fn get_patterns_by_success_rate(&self, min_sr: f64, limit: Option<usize>) -> Result<Vec<TrajectoryPattern>> {
        Self::rt().block_on(async {
            let models = trajectory_patterns::Entity::find()
                .filter(trajectory_patterns::Column::SuccessRate.gte(min_sr))
                .order_by_desc(trajectory_patterns::Column::SuccessRate)
                .all(self.db.as_ref()).await?;
            let end = limit.unwrap_or(models.len()).min(models.len());
            Ok(models.iter().take(end).map(model_to_traj_pattern).collect())
        })
    }

    // ── Skills ──

    pub fn save_skill(&self, skill: &Skill) -> Result<()> {
        Self::rt().block_on(async {
            trajectory_skills::Entity::insert(trajectory_skills::ActiveModel {
                id: Set(skill.id.clone()), name: Set(skill.name.clone()),
                description: Set(skill.description.clone()), skill_type: Set(skill.category.clone()),
                content: Set(skill.content.clone()), category: Set(skill.category.clone()),
                tags: Set(serde_json::to_string(&skill.tags)?),
                scenarios: Set(serde_json::to_string(&skill.scenarios)?),
                parameters: Set(serde_json::json!({}).to_string()),
                created_at: Set(skill.created_at.to_rfc3339()),
                updated_at: Set(skill.updated_at.to_rfc3339()),
                usage_count: Set(skill.total_usages as i32),
                success_rate: Set(skill.success_rate),
                avg_execution_time_ms: Set(skill.avg_execution_time_ms as f64),
            }).on_conflict(sea_orm::sea_query::OnConflict::column(trajectory_skills::Column::Id)
                .update_columns([trajectory_skills::Column::Name, trajectory_skills::Column::Content,
                    trajectory_skills::Column::UpdatedAt, trajectory_skills::Column::UsageCount,
                    trajectory_skills::Column::SuccessRate, trajectory_skills::Column::AvgExecutionTimeMs]).to_owned())
                .exec(self.db.as_ref()).await?;
            Ok(())
        })
    }

    pub fn get_skill(&self, id: &str) -> Result<Option<Skill>> {
        Self::rt().block_on(async {
            Ok(trajectory_skills::Entity::find_by_id(id).one(self.db.as_ref()).await?.map(|s| model_to_skill(&s)))
        })
    }

    pub fn get_skills(&self) -> Result<Vec<Skill>> {
        Self::rt().block_on(async {
            Ok(trajectory_skills::Entity::find().order_by_desc(trajectory_skills::Column::UsageCount)
                .all(self.db.as_ref()).await?.iter().map(model_to_skill).collect())
        })
    }

    pub fn delete_skill(&self, id: &str) -> Result<()> {
        Self::rt().block_on(async {
            trajectory_skill_executions::Entity::delete_many()
                .filter(trajectory_skill_executions::Column::SkillId.eq(id)).exec(self.db.as_ref()).await?;
            trajectory_skills::Entity::delete_by_id(id).exec(self.db.as_ref()).await?;
            info!("Deleted skill {}", id); Ok(())
        })
    }

    pub fn record_skill_execution(&self, sid: &str, tid: Option<&str>, success: bool, et: u64,
        ia: Option<&serde_json::Value>, or: Option<&serde_json::Value>) -> Result<()> {
        Self::rt().block_on(async {
            trajectory_skill_executions::ActiveModel {
                id: Set(Uuid::new_v4().to_string()), skill_id: Set(sid.to_string()),
                trajectory_id: Set(tid.map(|s| s.to_string())), success: Set(success as i32),
                execution_time_ms: Set(et as i64), created_at: Set(Utc::now().to_rfc3339()),
                input_args: Set(ia.map(|v| serde_json::to_string(v).unwrap_or_default())),
                output_result: Set(or.map(|v| serde_json::to_string(v).unwrap_or_default())),
            }.insert(self.db.as_ref()).await?;
            Ok(())
        })
    }

    pub fn get_skill_analytics(&self, sid: &str) -> Result<SkillAnalytics> {
        Self::rt().block_on(async {
            let all_execs = trajectory_skill_executions::Entity::find()
                .filter(trajectory_skill_executions::Column::SkillId.eq(sid)).all(self.db.as_ref()).await?;
            let total = all_execs.len() as u64;
            let successes = all_execs.iter().filter(|e| e.success != 0).count() as u64;
            Ok(SkillAnalytics { total_executions: total as u32,
                success_rate: if total > 0 { successes as f64 / total as f64 } else { 0.0 },
                avg_execution_time_ms: 0.0, recent_executions: total.min(100) as u32 })
        })
    }

    // ── Entities ──

    pub fn save_entity(&self, e: &Entity) -> Result<()> {
        Self::rt().block_on(async {
            trajectory_entities::Entity::insert(trajectory_entities::ActiveModel {
                id: Set(e.id.clone()), name: Set(e.name.clone()),
                entity_type: Set(serde_json::to_string(&e.entity_type).unwrap_or_default()),
                properties: Set(serde_json::to_string(&e.properties).unwrap_or_else(|_| "{}".to_string())),
                aliases: Set(serde_json::to_string(&e.aliases).unwrap_or_else(|_| "[]".to_string())),
                first_seen_at: Set(e.first_seen_at.to_rfc3339()),
                last_seen_at: Set(e.last_seen_at.to_rfc3339()),
                mention_count: Set(e.mention_count as i32), confidence: Set(e.confidence),
                created_at: Set(e.created_at.map(|dt| dt.to_rfc3339())),
                updated_at: Set(e.updated_at.map(|dt| dt.to_rfc3339())),
            }).on_conflict(sea_orm::sea_query::OnConflict::column(trajectory_entities::Column::Id)
                .update_columns([trajectory_entities::Column::Name, trajectory_entities::Column::LastSeenAt,
                    trajectory_entities::Column::MentionCount, trajectory_entities::Column::Confidence]).to_owned())
                .exec(self.db.as_ref()).await?;
            Ok(())
        })
    }

    pub fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        Self::rt().block_on(async {
            Ok(trajectory_entities::Entity::find_by_id(id).one(self.db.as_ref()).await?.map(|e| model_to_entity(&e)))
        })
    }

    pub fn get_all_entities(&self) -> Result<Vec<Entity>> {
        Self::rt().block_on(async {
            Ok(trajectory_entities::Entity::find().order_by_desc(trajectory_entities::Column::LastSeenAt)
                .all(self.db.as_ref()).await?.iter().map(model_to_entity).collect())
        })
    }

    pub fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<Entity>> {
        Self::rt().block_on(async {
            Ok(trajectory_entities::Entity::find()
                .filter(trajectory_entities::Column::Name.like(format!("%{}%", query)))
                .all(self.db.as_ref()).await?.iter().take(limit).map(model_to_entity).collect())
        })
    }

    pub fn delete_entity(&self, id: &str) -> Result<()> {
        Self::rt().block_on(async { trajectory_entities::Entity::delete_by_id(id).exec(self.db.as_ref()).await?; Ok(()) })
    }

    // ── Relationships ──

    pub fn save_relationship(&self, rel: &Relationship) -> Result<()> {
        Self::rt().block_on(async {
            trajectory_relationships::Entity::insert(trajectory_relationships::ActiveModel {
                id: Set(rel.id.clone()), source_id: Set(rel.source_id.clone()),
                target_id: Set(rel.target_id.clone()),
                relation_type: Set(serde_json::to_string(&rel.relation_type).unwrap_or_default()),
                properties: Set(serde_json::to_string(&rel.properties).unwrap_or_else(|_| "{}".to_string())),
                weight: Set(rel.weight), created_at: Set(rel.created_at.to_rfc3339()),
            }).on_conflict(sea_orm::sea_query::OnConflict::column(trajectory_relationships::Column::Id)
                .update_columns([trajectory_relationships::Column::Weight]).to_owned())
                .exec(self.db.as_ref()).await?;
            Ok(())
        })
    }

    pub fn get_relationships_by_entity(&self, eid: &str) -> Result<Vec<Relationship>> {
        Self::rt().block_on(async {
            Ok(trajectory_relationships::Entity::find()
                .filter(trajectory_relationships::Column::SourceId.eq(eid)
                    .or(trajectory_relationships::Column::TargetId.eq(eid)))
                .all(self.db.as_ref()).await?.iter().map(model_to_relationship).collect())
        })
    }

    pub fn get_all_relationships(&self) -> Result<Vec<Relationship>> {
        Self::rt().block_on(async {
            Ok(trajectory_relationships::Entity::find().order_by_desc(trajectory_relationships::Column::CreatedAt)
                .all(self.db.as_ref()).await?.iter().map(model_to_relationship).collect())
        })
    }

    pub fn delete_relationship(&self, id: &str) -> Result<()> {
        Self::rt().block_on(async { trajectory_relationships::Entity::delete_by_id(id).exec(self.db.as_ref()).await?; Ok(()) })
    }

    // ── Sessions ──

    pub fn save_session(&self, s: &Session) -> Result<()> {
        Self::rt().block_on(async {
            trajectory_sessions::Entity::insert(trajectory_sessions::ActiveModel {
                id: Set(s.id.clone()), title: Set(s.title.clone()),
                platform: Set(s.platform.clone()), user_id: Set(s.user_id.clone()),
                model: Set(s.model.clone()), system_prompt: Set(s.system_prompt.clone()),
                created_at: Set(s.created_at.to_rfc3339()), updated_at: Set(s.updated_at.to_rfc3339()),
                parent_session_id: Set(s.parent_session_id.clone()),
                token_input: Set(s.token_input), token_output: Set(s.token_output),
            }).on_conflict(sea_orm::sea_query::OnConflict::column(trajectory_sessions::Column::Id)
                .update_columns([trajectory_sessions::Column::Title, trajectory_sessions::Column::UpdatedAt,
                    trajectory_sessions::Column::TokenInput, trajectory_sessions::Column::TokenOutput]).to_owned())
                .exec(self.db.as_ref()).await?;
            Ok(())
        })
    }

    pub fn get_session(&self, id: &str) -> Result<Option<Session>> {
        Self::rt().block_on(async {
            Ok(trajectory_sessions::Entity::find_by_id(id).one(self.db.as_ref()).await?.map(|s| model_to_sess(&s)))
        })
    }

    pub fn get_all_sessions(&self) -> Result<Vec<Session>> {
        Self::rt().block_on(async {
            Ok(trajectory_sessions::Entity::find().order_by_desc(trajectory_sessions::Column::UpdatedAt)
                .all(self.db.as_ref()).await?.iter().map(model_to_sess).collect())
        })
    }

    pub fn update_session(&self, id: &str, updates: &SessionUpdate) -> Result<()> {
        Self::rt().block_on(async {
            let m = trajectory_sessions::Entity::find_by_id(id).one(self.db.as_ref()).await?
                .context("Session not found")?;
            let mut am: trajectory_sessions::ActiveModel = m.into_active_model();
            if let Some(ref t) = updates.title { am.title = Set(t.clone()); }
            if let Some(ti) = updates.token_input { am.token_input = Set(ti); }
            if let Some(to) = updates.token_output { am.token_output = Set(to); }
            am.updated_at = Set(Utc::now().to_rfc3339());
            am.update(self.db.as_ref()).await?;
            Ok(())
        })
    }

    pub fn delete_session(&self, id: &str) -> Result<()> {
        Self::rt().block_on(async { trajectory_sessions::Entity::delete_by_id(id).exec(self.db.as_ref()).await?; Ok(()) })
    }

    // ── Messages ──

    pub fn save_message(&self, msg: &Message) -> Result<()> {
        Self::rt().block_on(async {
            trajectory_messages::ActiveModel {
                id: Set(msg.id.clone()), session_id: Set(msg.session_id.clone()),
                role: Set(msg.role.clone()), content: Set(msg.content.clone()),
                tool_calls: Set(msg.tool_calls.clone()), tool_results: Set(msg.tool_results.clone()),
                usage: Set(msg.usage.clone()), created_at: Set(msg.created_at.to_rfc3339()),
            }.insert(self.db.as_ref()).await?;
            Ok(())
        })
    }

    pub fn get_messages_by_session(&self, sid: &str) -> Result<Vec<Message>> {
        Self::rt().block_on(async {
            Ok(trajectory_messages::Entity::find().filter(trajectory_messages::Column::SessionId.eq(sid))
                .order_by_asc(trajectory_messages::Column::CreatedAt)
                .all(self.db.as_ref()).await?.iter().map(model_to_msg).collect())
        })
    }

    pub fn search_messages(&self, query: &str, limit: usize) -> Result<Vec<Message>> {
        Self::rt().block_on(async {
            Ok(trajectory_messages::Entity::find()
                .filter(trajectory_messages::Column::Content.like(format!("%{}%", query)))
                .order_by_desc(trajectory_messages::Column::CreatedAt)
                .all(self.db.as_ref()).await?.iter().take(limit).map(model_to_msg).collect())
        })
    }

    // ── Memories ──

    pub fn get_all_memories(&self) -> Result<Vec<crate::memory::MemoryEntry>> {
        Self::rt().block_on(async {
            Ok(trajectory_memories::Entity::find().all(self.db.as_ref()).await?.into_iter().map(|m| {
                crate::memory::MemoryEntry { id: m.id, content: m.content, memory_type: m.memory_type, updated_at: 0i64 }
            }).collect())
        })
    }

    pub fn save_memory(&self, mem: &crate::memory::MemoryEntry) -> Result<()> {
        Self::rt().block_on(async {
            trajectory_memories::Entity::insert(trajectory_memories::ActiveModel {
                id: Set(mem.id.clone()), content: Set(mem.content.clone()),
                memory_type: Set(mem.memory_type.clone()), updated_at: Set(format!("{}", mem.updated_at)),
            }).on_conflict(sea_orm::sea_query::OnConflict::column(trajectory_memories::Column::Id)
                .update_columns([trajectory_memories::Column::Content, trajectory_memories::Column::UpdatedAt]).to_owned())
                .exec(self.db.as_ref()).await?;
            Ok(())
        })
    }

    pub fn delete_memory(&self, id: &str) -> Result<()> {
        Self::rt().block_on(async { trajectory_memories::Entity::delete_by_id(id).exec(self.db.as_ref()).await?; Ok(()) })
    }

    // ── Learned Patterns ──

    pub fn save_learning_pattern(&self, p: &Pattern) -> Result<()> {
        Self::rt().block_on(async {
            trajectory_learned_patterns::Entity::insert(trajectory_learned_patterns::ActiveModel {
                id: Set(p.id.clone()), pattern: Set(p.pattern.clone()),
                pattern_type: Set(p.pattern_type.clone()),
                success: Set(p.success), failure: Set(p.failure),
                last_used: Set(p.last_used.to_rfc3339()),
                created_at: Set(p.created_at.to_rfc3339()), metadata: Set(p.metadata.clone()),
            }).on_conflict(sea_orm::sea_query::OnConflict::column(trajectory_learned_patterns::Column::Id)
                .update_columns([trajectory_learned_patterns::Column::Success,
                    trajectory_learned_patterns::Column::Failure, trajectory_learned_patterns::Column::LastUsed]).to_owned())
                .exec(self.db.as_ref()).await?;
            Ok(())
        })
    }

    pub fn get_patterns_list(&self) -> Result<Vec<Pattern>> {
        Self::rt().block_on(async {
            Ok(trajectory_learned_patterns::Entity::find().all(self.db.as_ref()).await?.iter().map(|p| {
                Pattern {
                    id: p.id.clone(), pattern: p.pattern.clone(), pattern_type: p.pattern_type.clone(),
                    success: p.success, failure: p.failure,
                    last_used: chrono::DateTime::parse_from_rfc3339(&p.last_used).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    created_at: chrono::DateTime::parse_from_rfc3339(&p.created_at).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                    metadata: p.metadata.clone(),
                }
            }).collect())
        })
    }

    pub fn update_pattern_stats(&self, id: &str, sd: i32, fd: i32) -> Result<()> {
        Self::rt().block_on(async {
            if let Some(m) = trajectory_learned_patterns::Entity::find_by_id(id).one(self.db.as_ref()).await? {
                let mut am: trajectory_learned_patterns::ActiveModel = m.into_active_model();
                am.success = Set(am.success.unwrap() + sd);
                am.failure = Set(am.failure.unwrap() + fd);
                am.last_used = Set(Utc::now().to_rfc3339());
                am.update(self.db.as_ref()).await?;
            }
            Ok(())
        })
    }

    // ── Preferences ──

    pub fn save_preference(&self, pref: &Preference) -> Result<()> {
        Self::rt().block_on(async {
            trajectory_preferences::Entity::insert(trajectory_preferences::ActiveModel {
                id: Set(pref.id.clone()), key: Set(pref.key.clone()), value: Set(pref.value.clone()),
                confidence: Set(pref.confidence), updated_at: Set(pref.updated_at.to_rfc3339()),
            }).on_conflict(sea_orm::sea_query::OnConflict::column(trajectory_preferences::Column::Key)
                .update_columns([trajectory_preferences::Column::Value, trajectory_preferences::Column::Confidence,
                    trajectory_preferences::Column::UpdatedAt]).to_owned())
                .exec(self.db.as_ref()).await?;
            Ok(())
        })
    }

    pub fn get_preferences_list(&self) -> Result<Vec<Preference>> {
        Self::rt().block_on(async {
            Ok(trajectory_preferences::Entity::find().all(self.db.as_ref()).await?.iter().map(|p| Preference {
                id: p.id.clone(), key: p.key.clone(), value: p.value.clone(), confidence: p.confidence,
                updated_at: chrono::DateTime::parse_from_rfc3339(&p.updated_at).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
            }).collect())
        })
    }

    pub fn update_preference_by_key(&self, key: &str, updates: &Preference) -> Result<()> {
        Self::rt().block_on(async {
            if let Some(m) = trajectory_preferences::Entity::find()
                .filter(trajectory_preferences::Column::Key.eq(key)).one(self.db.as_ref()).await? {
                let mut am: trajectory_preferences::ActiveModel = m.into_active_model();
                am.value = Set(updates.value.clone());
                am.confidence = Set(updates.confidence);
                am.updated_at = Set(Utc::now().to_rfc3339());
                am.update(self.db.as_ref()).await?;
            }
            Ok(())
        })
    }

    // ── Utilities ──

    pub fn get_trajectory_stats(&self) -> Result<TrajectoryStatistics> {
        let trajs = self.get_trajectories(None)?;
        let total = trajs.len();
        if total == 0 { return Ok(TrajectoryStatistics { total_trajectories: 0, total_sessions: 0, total_patterns: 0, avg_quality: 0.0, avg_value_score: 0.0, success_rate: 0.0, recent_trajectories: 0 }); }
        let mut tq = 0.0; let mut tv = 0.0; let mut sc = 0;
        for t in &trajs {
            tq += t.quality.overall; tv += t.value_score;
            if t.outcome == TrajectoryOutcome::Success { sc += 1; }
        }
        Ok(TrajectoryStatistics { total_trajectories: total, total_sessions: 0, total_patterns: 0,
            avg_quality: tq / total as f64, avg_value_score: tv / total as f64,
            success_rate: sc as f64 / total as f64, recent_trajectories: total.min(10) })
    }

    pub fn export_trajectories(&self, opts: &TrajectoryExportOptions) -> Result<Vec<RLTrainingEntry>> {
        Ok(self.query_trajectories(&TrajectoryQuery {
            session_id: None, user_id: None, topic: None,
            min_quality: opts.min_quality, min_value_score: opts.min_value_score,
            outcome: opts.outcome_filter, time_range: None, limit: opts.limit,
        })?.into_iter().map(|t| t.export_as_rl()).collect())
    }

    pub fn search_trajectories_fts(&self, fts_query: &FTS5Query) -> Result<Vec<String>> {
        Self::rt().block_on(async {
            let pattern = format!("%{}%", fts_query.query);
            Ok(trajectories::Entity::find()
                .filter(trajectories::Column::Topic.like(&pattern)
                    .or(trajectories::Column::Summary.like(&pattern)))
                .all(self.db.as_ref()).await?.into_iter().take(fts_query.limit).map(|t| t.id).collect())
        })
    }

    pub fn init_memory_tables(&self) -> Result<()> { info!("Memory tables initialized"); Ok(()) }
    pub fn get_all_skills(&self) -> Result<Vec<Skill>> { self.get_skills() }
    pub fn get_all_patterns(&self) -> Result<Vec<TrajectoryPattern>> { self.get_patterns() }
    pub fn get_statistics(&self) -> Result<TrajectoryStatistics> { self.get_trajectory_stats() }

    // FTS delegates
    pub fn create_fts_tables(&self) -> Result<()> {
        self.fts_searcher.as_ref().map(|f| f.create_fts_tables()).unwrap_or(Ok(()))
    }
    pub fn search_fts(&self, query: FTS5Query) -> Result<Vec<FTS5Result>> {
        self.fts_searcher.as_ref().context("FTS not initialized")?.search(query)
    }
    pub fn index_trajectory_fts(&self, t: &Trajectory) -> Result<()> {
        if let Some(ref fts) = self.fts_searcher { fts.index_trajectory(t, &t.session_id) } else { Ok(()) }
    }
    pub fn index_skill_fts(&self, skill: &Skill) -> Result<()> {
        if let Some(ref fts) = self.fts_searcher {
            fts.index_skill(&skill.id, &skill.name, &skill.description, &skill.content, &skill.category, &skill.tags)
        } else { Ok(()) }
    }
    pub fn index_memory_fts(&self, id: &str, mt: &str, content: &str, entities: &[String]) -> Result<()> {
        if let Some(ref fts) = self.fts_searcher { fts.index_memory(id, mt, content, entities) } else { Ok(()) }
    }
    pub fn delete_memory_fts(&self, _id: &str) -> Result<()> { Ok(()) }
    pub fn optimize_fts(&self) -> Result<()> {
        self.fts_searcher.as_ref().map(|f| f.optimize()).unwrap_or(Ok(()))
    }
}

// ── Model conversion helpers ──

fn model_to_trajectory(m: &trajectories::Model, steps: Vec<TrajectoryStep>, rewards: Vec<RewardSignal>) -> Trajectory {
    Trajectory {
        id: m.id.clone(), session_id: m.session_id.clone(), user_id: m.user_id.clone(),
        topic: m.topic.clone(), summary: m.summary.clone(),
        outcome: serde_json::from_str(&format!("\"{}\"", m.outcome)).unwrap_or(TrajectoryOutcome::Success),
        duration_ms: m.duration_ms as u64,
        quality: crate::trajectory::TrajectoryQuality { overall: m.quality_overall, task_completion: m.quality_task_completion, tool_efficiency: m.quality_tool_efficiency, reasoning_quality: m.quality_reasoning_quality, user_satisfaction: m.quality_user_satisfaction },
        value_score: m.value_score, patterns: serde_json::from_str(&m.patterns).unwrap_or_default(),
        steps, rewards,
        created_at: chrono::DateTime::parse_from_rfc3339(&m.created_at).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
        replay_count: m.replay_count as u32,
        last_replay_at: m.last_replay_at.as_ref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)).ok()),
    }
}

fn model_to_skill(s: &trajectory_skills::Model) -> Skill {
    Skill {
        id: s.id.clone(), name: s.name.clone(), description: s.description.clone(),
        version: "1.0.0".to_string(), content: s.content.clone(), category: s.category.clone(),
        tags: serde_json::from_str(&s.tags).unwrap_or_default(), platforms: Vec::new(),
        scenarios: serde_json::from_str(&s.scenarios).unwrap_or_default(),
        quality_score: 0.0, success_rate: s.success_rate, avg_execution_time_ms: s.avg_execution_time_ms as u64,
        total_usages: s.usage_count as u32, successful_usages: 0,
        created_at: chrono::DateTime::parse_from_rfc3339(&s.created_at).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&s.updated_at).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
        last_used_at: None, metadata: crate::skill::SkillMetadata::default(),
    }
}

fn model_to_traj_pattern(p: &trajectory_patterns::Model) -> TrajectoryPattern {
    TrajectoryPattern {
        id: p.id.clone(), name: p.name.clone(), description: p.description.clone(),
        pattern_type: p.pattern_type.clone(),
        trajectory_ids: serde_json::from_str(&p.trajectory_ids).unwrap_or_default(),
        frequency: p.frequency as u32, success_rate: p.success_rate,
        average_quality: p.average_quality, average_value_score: p.average_value_score,
        reward_profile: serde_json::from_str(&p.reward_profile).unwrap_or_default(),
        created_at: chrono::DateTime::parse_from_rfc3339(&p.created_at).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
    }
}

fn model_to_entity(e: &trajectory_entities::Model) -> Entity {
    use crate::memory::EntityType;
    Entity {
        id: e.id.clone(), name: e.name.clone(),
        entity_type: serde_json::from_str(&format!("\"{}\"", e.entity_type)).unwrap_or(EntityType::Concept),
        properties: serde_json::from_str(&e.properties).unwrap_or_default(),
        aliases: serde_json::from_str(&e.aliases).unwrap_or_default(),
        first_seen_at: chrono::DateTime::parse_from_rfc3339(&e.first_seen_at).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
        last_seen_at: chrono::DateTime::parse_from_rfc3339(&e.last_seen_at).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
        mention_count: e.mention_count as u32, confidence: e.confidence,
        created_at: e.created_at.as_ref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)).ok()),
        updated_at: e.updated_at.as_ref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)).ok()),
    }
}

fn model_to_relationship(r: &trajectory_relationships::Model) -> Relationship {
    use crate::memory::RelationshipType;
    Relationship {
        id: r.id.clone(), source_id: r.source_id.clone(), target_id: r.target_id.clone(),
        relation_type: serde_json::from_str(&format!("\"{}\"", r.relation_type)).unwrap_or(RelationshipType::RelatedTo),
        properties: serde_json::from_str(&r.properties).unwrap_or_default(),
        weight: r.weight,
        created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
    }
}

fn model_to_sess(s: &trajectory_sessions::Model) -> Session {
    Session {
        id: s.id.clone(), title: s.title.clone(), platform: s.platform.clone(),
        user_id: s.user_id.clone(), model: s.model.clone(), system_prompt: s.system_prompt.clone(),
        created_at: chrono::DateTime::parse_from_rfc3339(&s.created_at).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&s.updated_at).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
        parent_session_id: s.parent_session_id.clone(), token_input: s.token_input, token_output: s.token_output,
    }
}

fn model_to_msg(m: &trajectory_messages::Model) -> Message {
    Message {
        id: m.id.clone(), session_id: m.session_id.clone(), role: m.role.clone(),
        content: m.content.clone(), tool_calls: m.tool_calls.clone(),
        tool_results: m.tool_results.clone(), usage: m.usage.clone(),
        created_at: chrono::DateTime::parse_from_rfc3339(&m.created_at).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
    }
}

// ── TrajectoryQueue ──

use std::collections::VecDeque;
use tokio::sync::mpsc::{self, Sender};

pub struct TrajectoryQueue {
    storage: Arc<TrajectoryStorage>,
    sender: Sender<Trajectory>,
    handle: tokio::task::JoinHandle<()>,
}

impl TrajectoryQueue {
    pub fn new(storage: Arc<TrajectoryStorage>, buffer_size: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<Trajectory>(buffer_size);
        let sc = storage.clone();
        let handle = tokio::spawn(async move {
            let mut batch: VecDeque<Trajectory> = VecDeque::with_capacity(32);
            let mut fi = tokio::time::interval(tokio::time::Duration::from_secs(1));
            loop {
                tokio::select! {
                    Some(t) = rx.recv() => { batch.push_back(t); if batch.len() >= 32 { flush(&sc, &mut batch).await; } }
                    _ = fi.tick() => { if !batch.is_empty() { flush(&sc, &mut batch).await; } }
                }
            }
        });
        Self { storage, sender: tx, handle }
    }

    pub fn try_enqueue(&self, t: Trajectory) -> bool { self.sender.try_send(t).is_ok() }
    pub async fn enqueue(&self, t: Trajectory) -> Result<(), tokio::sync::mpsc::error::TrySendError<Trajectory>> {
        self.sender.send(t.clone()).await.map_err(|_| tokio::sync::mpsc::error::TrySendError::Closed(t))
    }
    pub fn storage(&self) -> &Arc<TrajectoryStorage> { &self.storage }
    pub fn shutdown(self) { self.handle.abort(); }
}

async fn flush(storage: &Arc<TrajectoryStorage>, batch: &mut VecDeque<Trajectory>) {
    while let Some(t) = batch.pop_front() {
        if let Err(e) = storage.save_trajectory(&t) { tracing::warn!("[TrajectoryQueue] failed: {}", e); }
    }
}

// ── Public types ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub id: String, pub title: String, pub platform: String, pub user_id: String,
    pub model: String, pub system_prompt: String,
    pub created_at: chrono::DateTime<chrono::Utc>, pub updated_at: chrono::DateTime<chrono::Utc>,
    pub parent_session_id: Option<String>, pub token_input: i64, pub token_output: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionUpdate { pub title: Option<String>, pub token_input: Option<i64>, pub token_output: Option<i64> }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub id: String, pub session_id: String, pub role: String, pub content: String,
    pub tool_calls: Option<String>, pub tool_results: Option<String>,
    pub usage: Option<String>, pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Pattern {
    pub id: String, pub pattern: String, pub pattern_type: String,
    pub success: i32, pub failure: i32,
    pub last_used: chrono::DateTime<chrono::Utc>, pub created_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Preference {
    pub id: String, pub key: String, pub value: String, pub confidence: f64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrajectoryStatistics {
    pub total_trajectories: usize, pub total_sessions: usize, pub total_patterns: usize,
    pub avg_quality: f64, pub avg_value_score: f64, pub success_rate: f64, pub recent_trajectories: usize,
}

unsafe impl Send for TrajectoryStorage {}
unsafe impl Sync for TrajectoryStorage {}
