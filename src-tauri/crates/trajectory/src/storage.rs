//! Trajectory storage module using SQLite

use crate::fts5::{FTS5Config, FTS5Query, FTS5Result, FTS5Search};
use crate::memory::{Entity, Relationship};
use crate::skill::{Skill, SkillAnalytics};
use crate::trajectory::{
    MessageRole, RLTrainingEntry, RewardSignal, Trajectory,
    TrajectoryExportOptions, TrajectoryOutcome, TrajectoryPattern, TrajectoryQuery,
    TrajectoryStep,
};
use anyhow::{Context, Result};
use chrono::Utc;
use directories::ProjectDirs;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tracing::info;
use uuid::Uuid;

pub struct TrajectoryStorage {
    conn: Arc<RwLock<Connection>>,
    fts_searcher: FTS5Search,
}

impl TrajectoryStorage {
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path()?;
        Self::new_with_path(&db_path)
    }

    pub fn new_with_path(path: &PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path).context("Failed to open SQLite database")?;
        let conn_arc = Arc::new(RwLock::new(conn));
        let fts_searcher = FTS5Search::new(conn_arc.clone(), FTS5Config::default());

        conn_arc.read().unwrap().execute_batch(
            "
            CREATE TABLE IF NOT EXISTS trajectories (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                topic TEXT NOT NULL,
                summary TEXT NOT NULL,
                outcome TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                quality_overall REAL NOT NULL,
                quality_task_completion REAL NOT NULL,
                quality_tool_efficiency REAL NOT NULL,
                quality_reasoning_quality REAL NOT NULL,
                quality_user_satisfaction REAL NOT NULL,
                value_score REAL NOT NULL,
                patterns TEXT NOT NULL,
                created_at TEXT NOT NULL,
                replay_count INTEGER NOT NULL DEFAULT 0,
                last_replay_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_trajectories_session ON trajectories(session_id);
            CREATE INDEX IF NOT EXISTS idx_trajectories_user ON trajectories(user_id);
            CREATE INDEX IF NOT EXISTS idx_trajectories_topic ON trajectories(topic);
            CREATE INDEX IF NOT EXISTS idx_trajectories_outcome ON trajectories(outcome);
            CREATE INDEX IF NOT EXISTS idx_trajectories_created ON trajectories(created_at);

            CREATE TABLE IF NOT EXISTS trajectory_steps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                trajectory_id TEXT NOT NULL,
                step_index INTEGER NOT NULL,
                timestamp_ms INTEGER NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                reasoning TEXT,
                tool_calls TEXT,
                tool_results TEXT,
                FOREIGN KEY (trajectory_id) REFERENCES trajectories(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_trajectory_steps ON trajectory_steps(trajectory_id, step_index);

            CREATE TABLE IF NOT EXISTS trajectory_rewards (
                id TEXT PRIMARY KEY,
                trajectory_id TEXT NOT NULL,
                reward_type TEXT NOT NULL,
                value REAL NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (trajectory_id) REFERENCES trajectories(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                skill_type TEXT NOT NULL,
                content TEXT NOT NULL,
                category TEXT NOT NULL,
                tags TEXT NOT NULL,
                scenarios TEXT NOT NULL DEFAULT '[]',
                parameters TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                usage_count INTEGER NOT NULL DEFAULT 0,
                success_rate REAL NOT NULL DEFAULT 0.0,
                avg_execution_time_ms REAL NOT NULL DEFAULT 0.0
            );

            CREATE TABLE IF NOT EXISTS skill_executions (
                id TEXT PRIMARY KEY,
                skill_id TEXT NOT NULL,
                trajectory_id TEXT,
                success INTEGER NOT NULL,
                execution_time_ms INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE,
                FOREIGN KEY (trajectory_id) REFERENCES trajectories(id) ON DELETE SET NULL
            );

            CREATE INDEX IF NOT EXISTS idx_skill_executions ON skill_executions(skill_id, created_at);

            CREATE TABLE IF NOT EXISTS patterns (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                pattern_type TEXT NOT NULL,
                trajectory_ids TEXT NOT NULL,
                frequency INTEGER NOT NULL,
                success_rate REAL NOT NULL,
                average_quality REAL NOT NULL,
                average_value_score REAL NOT NULL,
                reward_profile TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_patterns_type ON patterns(pattern_type);
            CREATE INDEX IF NOT EXISTS idx_patterns_success ON patterns(success_rate);

            CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                properties TEXT NOT NULL DEFAULT '{}',
                aliases TEXT NOT NULL DEFAULT '[]',
                first_seen_at TEXT NOT NULL,
                last_seen_at TEXT NOT NULL,
                mention_count INTEGER NOT NULL DEFAULT 1,
                confidence REAL NOT NULL DEFAULT 0.5,
                created_at TEXT,
                updated_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
            CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name);
            CREATE INDEX IF NOT EXISTS idx_entities_confidence ON entities(confidence);

            CREATE TABLE IF NOT EXISTS relationships (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                properties TEXT NOT NULL DEFAULT '{}',
                weight REAL NOT NULL DEFAULT 1.0,
                created_at TEXT NOT NULL,
                FOREIGN KEY (source_id) REFERENCES entities(id) ON DELETE CASCADE,
                FOREIGN KEY (target_id) REFERENCES entities(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_relationships_source ON relationships(source_id);
            CREATE INDEX IF NOT EXISTS idx_relationships_target ON relationships(target_id);
            CREATE INDEX IF NOT EXISTS idx_relationships_type ON relationships(relation_type);

            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                platform TEXT NOT NULL DEFAULT 'web',
                user_id TEXT NOT NULL DEFAULT 'default',
                model TEXT NOT NULL DEFAULT 'unknown',
                system_prompt TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                parent_session_id TEXT,
                token_input INTEGER NOT NULL DEFAULT 0,
                token_output INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at);
            CREATE INDEX IF NOT EXISTS idx_sessions_platform ON sessions(platform);
            CREATE INDEX IF NOT EXISTS idx_sessions_parent ON sessions(parent_session_id);

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                tool_calls TEXT,
                tool_results TEXT,
                usage TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
            CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);

            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                memory_type TEXT NOT NULL,
                content TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(memory_type);

            CREATE VIRTUAL TABLE IF NOT EXISTS trajectories_fts USING fts5(
                id,
                session_id,
                topic,
                summary,
                content,
                outcome,
                quality_score,
                created_at
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS skills_fts USING fts5(
                id,
                name,
                description,
                content,
                category,
                tags,
                created_at
            );

            CREATE TABLE IF NOT EXISTS patterns (
                id TEXT PRIMARY KEY,
                pattern TEXT NOT NULL,
                pattern_type TEXT NOT NULL,
                success INTEGER NOT NULL DEFAULT 0,
                failure INTEGER NOT NULL DEFAULT 0,
                last_used TEXT NOT NULL,
                created_at TEXT NOT NULL,
                metadata TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_patterns_type ON patterns(pattern_type);

            CREATE TABLE IF NOT EXISTS preferences (
                id TEXT PRIMARY KEY,
                key TEXT NOT NULL UNIQUE,
                value TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.0,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_preferences_key ON preferences(key);
            ",
        )
        .context("Failed to create database tables")?;

        conn_arc.read().unwrap().execute_batch(
            "ALTER TABLE skills ADD COLUMN scenarios TEXT NOT NULL DEFAULT '[]';",
        ).ok();

        info!("Trajectory storage initialized with FTS5 tables");

        Ok(Self {
            conn: conn_arc,
            fts_searcher,
        })
    }

    fn get_db_path() -> Result<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "clawcode", "trajectory") {
            Ok(proj_dirs.data_dir().join("trajectories.db"))
        } else {
            Ok(PathBuf::from("trajectories.db"))
        }
    }

    fn row_to_trajectory(row: &rusqlite::Row) -> Trajectory {
        let outcome_str: String = row.get(5).unwrap_or_default();
        let outcome = serde_json::from_str(&format!("\"{}\"", outcome_str))
            .unwrap_or(TrajectoryOutcome::Success);

        let patterns_str: String = row.get(13).unwrap_or_default();
        let patterns: Vec<String> = serde_json::from_str(&patterns_str).unwrap_or_default();

        let created_at_str: String = row.get(14).unwrap_or_default();
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let last_replay_at_str: Option<String> = row.get(16).ok();
        let last_replay_at = last_replay_at_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        });

        Trajectory {
            id: row.get(0).unwrap_or_default(),
            session_id: row.get(1).unwrap_or_default(),
            user_id: row.get(2).unwrap_or_default(),
            topic: row.get(3).unwrap_or_default(),
            summary: row.get(4).unwrap_or_default(),
            outcome,
            duration_ms: row.get::<_, i64>(6).unwrap_or(0) as u64,
            quality: crate::trajectory::TrajectoryQuality {
                overall: row.get::<_, f64>(7).unwrap_or(0.0),
                task_completion: row.get::<_, f64>(8).unwrap_or(0.0),
                tool_efficiency: row.get::<_, f64>(9).unwrap_or(0.0),
                reasoning_quality: row.get::<_, f64>(10).unwrap_or(0.0),
                user_satisfaction: row.get::<_, f64>(11).unwrap_or(0.0),
            },
            value_score: row.get::<_, f64>(12).unwrap_or(0.0),
            patterns,
            steps: Vec::new(),
            rewards: Vec::new(),
            created_at,
            replay_count: row.get::<_, i32>(15).unwrap_or(0) as u32,
            last_replay_at,
        }
    }

    pub fn save_trajectory(&self, trajectory: &Trajectory) -> Result<()> {
        let conn = self.conn.write().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO trajectories
             (id, session_id, user_id, topic, summary, outcome, duration_ms,
              quality_overall, quality_task_completion, quality_tool_efficiency,
              quality_reasoning_quality, quality_user_satisfaction,
              value_score, patterns, created_at, replay_count, last_replay_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                trajectory.id,
                trajectory.session_id,
                trajectory.user_id,
                trajectory.topic,
                trajectory.summary,
                format!("{:?}", trajectory.outcome).to_lowercase(),
                trajectory.duration_ms as i64,
                trajectory.quality.overall,
                trajectory.quality.task_completion,
                trajectory.quality.tool_efficiency,
                trajectory.quality.reasoning_quality,
                trajectory.quality.user_satisfaction,
                trajectory.value_score,
                serde_json::to_string(&trajectory.patterns)?,
                trajectory.created_at.to_rfc3339(),
                trajectory.replay_count as i32,
                trajectory.last_replay_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;

        conn.execute(
            "DELETE FROM trajectory_steps WHERE trajectory_id = ?1",
            params![trajectory.id],
        )?;

        for (idx, step) in trajectory.steps.iter().enumerate() {
            conn.execute(
                "INSERT INTO trajectory_steps
                 (trajectory_id, step_index, timestamp_ms, role, content, reasoning, tool_calls, tool_results)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    trajectory.id,
                    idx as i32,
                    step.timestamp_ms as i64,
                    format!("{:?}", step.role).to_lowercase(),
                    step.content,
                    step.reasoning,
                    step.tool_calls.as_ref().map(|c| serde_json::to_string(c).ok()).flatten(),
                    step.tool_results.as_ref().map(|r| serde_json::to_string(r).ok()).flatten(),
                ],
            )?;
        }

        conn.execute(
            "DELETE FROM trajectory_rewards WHERE trajectory_id = ?1",
            params![trajectory.id],
        )?;

        for reward in &trajectory.rewards {
            conn.execute(
                "INSERT INTO trajectory_rewards
                 (id, trajectory_id, reward_type, value, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    Uuid::new_v4().to_string(),
                    trajectory.id,
                    format!("{:?}", reward.reward_type),
                    reward.value,
                    chrono::DateTime::from_timestamp_millis(reward.timestamp_ms as i64)
                        .unwrap_or_else(chrono::Utc::now)
                        .to_rfc3339(),
                ],
            )?;
        }

        Ok(())
    }

    pub fn get_trajectory(&self, id: &str) -> Result<Option<Trajectory>> {
        let conn = self.conn.read().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, session_id, user_id, topic, summary, outcome, duration_ms,
                    quality_overall, quality_task_completion, quality_tool_efficiency,
                    quality_reasoning_quality, quality_user_satisfaction,
                    value_score, patterns, created_at, replay_count, last_replay_at
             FROM trajectories WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| Ok(Some(Self::row_to_trajectory(row))));

        match result {
            Ok(Some(mut traj)) => {
                drop(stmt);
                let steps = self.get_trajectory_steps(&traj.id)?;
                let rewards = self.get_trajectory_rewards(&traj.id)?;
                traj.steps = steps;
                traj.rewards = rewards;
                Ok(Some(traj))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to get trajectory: {}", e)),
        }
    }

    pub fn get_trajectories(&self, limit: Option<usize>) -> Result<Vec<Trajectory>> {
        let conn = self.conn.read().unwrap();

        let query = match limit {
            Some(l) => format!(
                "SELECT id, session_id, user_id, topic, summary, outcome, duration_ms,
                        quality_overall, quality_task_completion, quality_tool_efficiency,
                        quality_reasoning_quality, quality_user_satisfaction,
                        value_score, patterns, created_at, replay_count, last_replay_at
                 FROM trajectories ORDER BY created_at DESC LIMIT {}",
                l
            ),
            None => String::from(
                "SELECT id, session_id, user_id, topic, summary, outcome, duration_ms,
                        quality_overall, quality_task_completion, quality_tool_efficiency,
                        quality_reasoning_quality, quality_user_satisfaction,
                        value_score, patterns, created_at, replay_count, last_replay_at
                 FROM trajectories ORDER BY created_at DESC",
            ),
        };

        let mut stmt = conn.prepare(&query)?;

        let trajectories: Vec<Trajectory> = stmt
            .query_map([], |row| Ok(Self::row_to_trajectory(row)))?
            .filter_map(|r| r.ok())
            .collect();

        drop(stmt);

        let mut result = Vec::new();
        for mut traj in trajectories {
            let steps = self.get_trajectory_steps(&traj.id)?;
            let rewards = self.get_trajectory_rewards(&traj.id)?;
            traj.steps = steps;
            traj.rewards = rewards;
            result.push(traj);
        }

        Ok(result)
    }

    fn get_trajectory_steps(&self, trajectory_id: &str) -> Result<Vec<TrajectoryStep>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare(
            "SELECT step_index, timestamp_ms, role, content, reasoning, tool_calls, tool_results
             FROM trajectory_steps WHERE trajectory_id = ?1 ORDER BY step_index",
        )?;

        let steps = stmt
            .query_map(params![trajectory_id], |row| {
                let tool_calls_str: Option<String> = row.get(5)?;
                let tool_results_str: Option<String> = row.get(6)?;

                Ok(TrajectoryStep {
                    timestamp_ms: row.get::<_, i64>(1)? as u64,
                    role: serde_json::from_str(&format!("\"{}\"", row.get::<_, String>(2)?))
                        .unwrap_or(MessageRole::Assistant),
                    content: row.get(3)?,
                    reasoning: row.get(4)?,
                    tool_calls: tool_calls_str.and_then(|s| serde_json::from_str(&s).ok()),
                    tool_results: tool_results_str.and_then(|s| serde_json::from_str(&s).ok()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(steps)
    }

    fn get_trajectory_rewards(&self, trajectory_id: &str) -> Result<Vec<RewardSignal>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, reward_type, value, created_at
             FROM trajectory_rewards WHERE trajectory_id = ?1",
        )?;

        let rewards = stmt
            .query_map(params![trajectory_id], |row| {
                let reward_type_str: String = row.get(1)?;
                let value: f64 = row.get(2)?;
                let created_at_str: String = row.get(3)?;

                let reward_type = match reward_type_str.as_str() {
                    "task_completion" => crate::trajectory::RewardType::TaskCompletion,
                    "tool_efficiency" => crate::trajectory::RewardType::ToolEfficiency,
                    "reasoning_quality" => crate::trajectory::RewardType::ReasoningQuality,
                    "user_feedback" => crate::trajectory::RewardType::UserFeedback,
                    _ => crate::trajectory::RewardType::UserFeedback,
                };

                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                Ok(RewardSignal {
                    reward_type,
                    value,
                    step_index: 0,
                    timestamp_ms: created_at.timestamp_millis() as u64,
                    metadata: serde_json::Value::Null,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rewards)
    }

    pub fn get_session_trajectories(&self, session_id: &str) -> Result<Vec<Trajectory>> {
        let conn = self.conn.read().unwrap();

        let mut stmt =
            conn.prepare("SELECT id FROM trajectories WHERE session_id = ?1 ORDER BY created_at")?;

        let ids: Vec<String> = stmt
            .query_map(params![session_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        drop(stmt);

        let mut trajectories = Vec::new();
        for id in ids {
            if let Some(traj) = self.get_trajectory(&id)? {
                trajectories.push(traj);
            }
        }

        Ok(trajectories)
    }

    pub fn query_trajectories(&self, query: &TrajectoryQuery) -> Result<Vec<Trajectory>> {
        let conn = self.conn.read().unwrap();

        let mut sql = String::from(
            "SELECT id, session_id, user_id, topic, summary, outcome, duration_ms,
                    quality_overall, quality_task_completion, quality_tool_efficiency,
                    quality_reasoning_quality, quality_user_satisfaction,
                    value_score, patterns, created_at, replay_count, last_replay_at
             FROM trajectories WHERE 1=1",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref session_id) = query.session_id {
            sql.push_str(" AND session_id = ?");
            params_vec.push(Box::new(session_id.clone()));
        }

        if let Some(ref user_id) = query.user_id {
            sql.push_str(" AND user_id = ?");
            params_vec.push(Box::new(user_id.clone()));
        }

        if let Some(ref topic) = query.topic {
            sql.push_str(" AND topic LIKE ?");
            params_vec.push(Box::new(format!("%{}%", topic)));
        }

        if let Some(min_quality) = query.min_quality {
            sql.push_str(" AND quality_overall >= ?");
            params_vec.push(Box::new(min_quality));
        }

        if let Some(min_value) = query.min_value_score {
            sql.push_str(" AND value_score >= ?");
            params_vec.push(Box::new(min_value));
        }

        if let Some(ref outcome) = query.outcome {
            sql.push_str(" AND outcome = ?");
            params_vec.push(Box::new(format!("{:?}", outcome)));
        }

        if let Some(ref time_range) = query.time_range {
            let (start, end) = time_range;
            sql.push_str(" AND created_at >= ?");
            params_vec.push(Box::new(start.to_rfc3339()));
            sql.push_str(" AND created_at <= ?");
            params_vec.push(Box::new(end.to_rfc3339()));
        }

        sql.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let trajectories: Vec<Trajectory> = {
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params_refs.as_slice(), |row| {
                Ok(Self::row_to_trajectory(row))
            })?;

            let mut trajectories = Vec::new();
            for row_result in rows {
                if let Ok(mut traj) = row_result {
                    let steps = self.get_trajectory_steps(&traj.id)?;
                    let rewards = self.get_trajectory_rewards(&traj.id)?;
                    traj.steps = steps;
                    traj.rewards = rewards;
                    trajectories.push(traj);
                }
            }
            trajectories
        };

        Ok(trajectories)
    }

    pub fn save_pattern(&self, pattern: &TrajectoryPattern) -> Result<()> {
        let conn = self.conn.write().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO patterns
             (id, name, description, pattern_type, trajectory_ids, frequency,
              success_rate, average_quality, average_value_score, reward_profile, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                pattern.id,
                pattern.name,
                pattern.description,
                pattern.pattern_type,
                serde_json::to_string(&pattern.trajectory_ids)?,
                pattern.frequency as i32,
                pattern.success_rate,
                pattern.average_quality,
                pattern.average_value_score,
                serde_json::to_string(&pattern.reward_profile)?,
                pattern.created_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub fn get_patterns(&self) -> Result<Vec<TrajectoryPattern>> {
        let conn = self.conn.read().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, description, pattern_type, trajectory_ids, frequency,
                    success_rate, average_quality, average_value_score, reward_profile, created_at
             FROM patterns ORDER BY frequency DESC",
        )?;

        let patterns = stmt
            .query_map([], |row| {
                let trajectory_ids_str: String = row.get(4)?;
                let reward_profile_str: String = row.get(9)?;
                let created_at_str: String = row.get(10)?;

                Ok(TrajectoryPattern {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    pattern_type: row.get(3)?,
                    trajectory_ids: serde_json::from_str(&trajectory_ids_str).unwrap_or_default(),
                    frequency: row.get::<_, i32>(5)? as u32,
                    success_rate: row.get(6)?,
                    average_quality: row.get(7)?,
                    average_value_score: row.get(8)?,
                    reward_profile: serde_json::from_str(&reward_profile_str).unwrap_or_default(),
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(patterns)
    }

    pub fn get_patterns_by_success_rate(
        &self,
        min_success_rate: f64,
        limit: Option<usize>,
    ) -> Result<Vec<TrajectoryPattern>> {
        let conn = self.conn.read().unwrap();
        let limit = limit.unwrap_or(100);

        let mut stmt = conn.prepare(
            "SELECT id, name, description, pattern_type, trajectory_ids, frequency,
                    success_rate, average_quality, average_value_score, reward_profile, created_at
             FROM patterns WHERE success_rate >= ?1 ORDER BY success_rate DESC LIMIT ?2",
        )?;

        let patterns = stmt
            .query_map(params![min_success_rate, limit as i64], |row| {
                let trajectory_ids_str: String = row.get(4)?;
                let reward_profile_str: String = row.get(9)?;
                let created_at_str: String = row.get(10)?;

                Ok(TrajectoryPattern {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    pattern_type: row.get(3)?,
                    trajectory_ids: serde_json::from_str(&trajectory_ids_str).unwrap_or_default(),
                    frequency: row.get::<_, i32>(5)? as u32,
                    success_rate: row.get(6)?,
                    average_quality: row.get(7)?,
                    average_value_score: row.get(8)?,
                    reward_profile: serde_json::from_str(&reward_profile_str).unwrap_or_default(),
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(patterns)
    }

    pub fn save_skill(&self, skill: &Skill) -> Result<()> {
        let conn = self.conn.write().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO skills
             (id, name, description, skill_type, content, category, tags, scenarios, parameters,
              created_at, updated_at, usage_count, success_rate, avg_execution_time_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                skill.id,
                skill.name,
                skill.description,
                skill.category.clone(),
                skill.content,
                skill.category,
                serde_json::to_string(&skill.tags)?,
                serde_json::to_string(&skill.scenarios)?,
                serde_json::json!({}).to_string(),
                skill.created_at.to_rfc3339(),
                skill.updated_at.to_rfc3339(),
                skill.total_usages as i32,
                skill.success_rate,
                skill.avg_execution_time_ms,
            ],
        )?;

        Ok(())
    }

    pub fn get_skill(&self, id: &str) -> Result<Option<Skill>> {
        let conn = self.conn.read().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, description, skill_type, content, category, tags, scenarios, parameters,
                    created_at, updated_at, usage_count, success_rate, avg_execution_time_ms
             FROM skills WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            let tags_str: String = row.get(6)?;
            let scenarios_str: String = row.get(7)?;
            let _parameters_str: String = row.get(8)?;
            let created_at_str: String = row.get(9)?;
            let updated_at_str: String = row.get(10)?;

            Ok(Skill {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                version: "1.0.0".to_string(),
                content: row.get(4)?,
                category: row.get(5)?,
                tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                platforms: Vec::new(),
                scenarios: serde_json::from_str(&scenarios_str).unwrap_or_default(),
                quality_score: 0.0,
                success_rate: row.get(12)?,
                avg_execution_time_ms: row.get(13)?,
                total_usages: row.get::<_, i32>(11)? as u32,
                successful_usages: 0,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                last_used_at: None,
                metadata: crate::skill::SkillMetadata::default(),
            })
        });

        match result {
            Ok(skill) => Ok(Some(skill)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to get skill: {}", e)),
        }
    }

    pub fn get_skills(&self) -> Result<Vec<Skill>> {
        let conn = self.conn.read().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, description, skill_type, content, category, tags, scenarios, parameters,
                    created_at, updated_at, usage_count, success_rate, avg_execution_time_ms
             FROM skills ORDER BY usage_count DESC",
        )?;

        let skills = stmt
            .query_map([], |row| {
                let tags_str: String = row.get(6)?;
                let scenarios_str: String = row.get(7)?;
                let _parameters_str: String = row.get(8)?;
                let created_at_str: String = row.get(9)?;
                let updated_at_str: String = row.get(10)?;

                Ok(Skill {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    version: "1.0.0".to_string(),
                    content: row.get(4)?,
                    category: row.get(5)?,
                    tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                    platforms: Vec::new(),
                    scenarios: serde_json::from_str(&scenarios_str).unwrap_or_default(),
                    quality_score: 0.0,
                    success_rate: row.get(12)?,
                    avg_execution_time_ms: row.get(13)?,
                    total_usages: row.get::<_, i32>(11)? as u32,
                    successful_usages: 0,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    last_used_at: None,
                    metadata: crate::skill::SkillMetadata::default(),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(skills)
    }

    pub fn delete_skill(&self, id: &str) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute(
            "DELETE FROM skill_executions WHERE skill_id = ?1",
            params![id],
        )?;
        conn.execute("DELETE FROM skills WHERE id = ?1", params![id])?;
        info!("Deleted skill {}", id);
        Ok(())
    }

    pub fn record_skill_execution(
        &self,
        skill_id: &str,
        trajectory_id: Option<&str>,
        success: bool,
        execution_time_ms: u64,
    ) -> Result<()> {
        let conn = self.conn.write().unwrap();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO skill_executions (id, skill_id, trajectory_id, success, execution_time_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                skill_id,
                trajectory_id,
                success as i32,
                execution_time_ms as i64,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_skill_analytics(&self, skill_id: &str) -> Result<SkillAnalytics> {
        let conn = self.conn.read().unwrap();

        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_executions WHERE skill_id = ?1",
                params![skill_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let successes: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_executions WHERE skill_id = ?1 AND success = 1",
                params![skill_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let avg_time: f64 = conn
            .query_row(
                "SELECT AVG(execution_time_ms) FROM skill_executions WHERE skill_id = ?1",
                params![skill_id],
                |row| row.get(0),
            )
            .unwrap_or(0.0);

        let trend: Vec<bool> = {
            let mut stmt = conn.prepare(
                "SELECT success FROM skill_executions WHERE skill_id = ?1 ORDER BY created_at DESC LIMIT 100",
            )?;
            let result: Vec<bool> = stmt.query_map(params![skill_id], |row| {
                Ok(row.get::<_, i32>(0)? != 0)
            })?
            .filter_map(|r| r.ok())
            .collect();
            result
        };

        Ok(SkillAnalytics {
            total_executions: total as u32,
            success_rate: if total > 0 {
                successes as f64 / total as f64
            } else {
                0.0
            },
            avg_execution_time_ms: avg_time,
            recent_executions: trend.len() as u32,
        })
    }

    pub fn search_trajectories_fts(&self, fts_query: &FTS5Query) -> Result<Vec<String>> {
        let conn = self.conn.read().unwrap();
        let pattern = format!("%{}%", fts_query.query);
        let limit = fts_query.limit;

        let mut stmt = conn.prepare(
            "SELECT id FROM trajectories WHERE topic LIKE ?1 OR summary LIKE ?1 LIMIT ?2",
        )?;

        let ids: Vec<String> = stmt
            .query_map(params![pattern, limit as i64], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ids)
    }

    pub fn export_trajectories(
        &self,
        options: &TrajectoryExportOptions,
    ) -> Result<Vec<RLTrainingEntry>> {
        let query = TrajectoryQuery {
            session_id: None,
            user_id: None,
            topic: None,
            min_quality: options.min_quality,
            min_value_score: options.min_value_score,
            outcome: options.outcome_filter,
            time_range: None,
            limit: options.limit,
        };

        let trajectories = self.query_trajectories(&query)?;
        let mut entries = Vec::new();

        for traj in trajectories {
            entries.push(traj.export_as_rl());
        }

        Ok(entries)
    }

    pub fn get_trajectory_stats(&self) -> Result<TrajectoryStatistics> {
        let trajectories = self.get_trajectories(None)?;
        let total = trajectories.len();

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

        let mut total_quality = 0.0;
        let mut total_value = 0.0;
        let mut successes = 0;

        for traj in &trajectories {
            total_quality += traj.quality.overall;
            total_value += traj.value_score;
            if traj.outcome == TrajectoryOutcome::Success {
                successes += 1;
            }
        }

        let recent = trajectories.len().min(10);
        let success_rate = successes as f64 / total as f64;

        Ok(TrajectoryStatistics {
            total_trajectories: total,
            total_sessions: 0,
            total_patterns: 0,
            avg_quality: total_quality / total as f64,
            avg_value_score: total_value / total as f64,
            success_rate,
            recent_trajectories: recent,
        })
    }

    pub fn init_memory_tables(&self) -> Result<()> {
        info!("Memory tables initialized");
        Ok(())
    }

    pub fn get_all_memories(&self) -> Result<Vec<crate::memory::MemoryEntry>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare("SELECT id, content, memory_type, updated_at FROM memories")?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::memory::MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                memory_type: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        let memories: Vec<crate::memory::MemoryEntry> = rows.filter_map(|r| r.ok()).collect();
        Ok(memories)
    }

    pub fn save_memory(&self, memory: &crate::memory::MemoryEntry) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO memories (id, content, memory_type, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                memory.id,
                memory.content,
                memory.memory_type,
                memory.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn delete_memory(&self, id: &str) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_all_skills(&self) -> Result<Vec<Skill>> {
        self.get_skills()
    }

    pub fn create_fts_tables(&self) -> Result<()> {
        self.fts_searcher.create_fts_tables()
    }

    pub fn search_fts(&self, query: FTS5Query) -> Result<Vec<FTS5Result>> {
        self.fts_searcher.search(query)
    }

    pub fn index_trajectory_fts(&self, trajectory: &Trajectory) -> Result<()> {
        let session_id = trajectory.session_id.clone();
        self.fts_searcher.index_trajectory(trajectory, &session_id)
    }

    pub fn index_skill_fts(&self, skill: &Skill) -> Result<()> {
        self.fts_searcher.index_skill(
            &skill.id,
            &skill.name,
            &skill.description,
            &skill.content,
            &skill.category,
            &skill.tags,
        )
    }

    pub fn index_memory_fts(
        &self,
        id: &str,
        memory_type: &str,
        content: &str,
        entities: &[String],
    ) -> Result<()> {
        self.fts_searcher
            .index_memory(id, memory_type, content, entities)
    }

    pub fn delete_memory_fts(&self, id: &str) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute("DELETE FROM memories_fts WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn optimize_fts(&self) -> Result<()> {
        self.fts_searcher.optimize()
    }

    pub fn get_all_patterns(&self) -> Result<Vec<TrajectoryPattern>> {
        self.get_patterns()
    }

    pub fn save_entity(&self, entity: &Entity) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO entities (id, name, entity_type, properties, aliases, first_seen_at, last_seen_at, mention_count, confidence, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                entity.id,
                entity.name,
                serde_json::to_string(&entity.entity_type).unwrap_or_default(),
                serde_json::to_string(&entity.properties).unwrap_or_else(|_| "{}".to_string()),
                serde_json::to_string(&entity.aliases).unwrap_or_else(|_| "[]".to_string()),
                entity.first_seen_at.to_rfc3339(),
                entity.last_seen_at.to_rfc3339(),
                entity.mention_count,
                entity.confidence,
                entity.created_at.map(|dt| dt.to_rfc3339()),
                entity.updated_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    pub fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM entities WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(self.row_to_entity(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_entities(&self) -> Result<Vec<Entity>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM entities ORDER BY last_seen_at DESC")?;
        let mut rows = stmt.query([])?;
        let mut entities = Vec::new();
        while let Some(row) = rows.next()? {
            entities.push(self.row_to_entity(row)?);
        }
        Ok(entities)
    }

    pub fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<Entity>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM entities WHERE name LIKE ?1 OR aliases LIKE ?1 LIMIT ?2"
        )?;
        let pattern = format!("%{}%", query);
        let mut rows = stmt.query(params![pattern, limit])?;
        let mut entities = Vec::new();
        while let Some(row) = rows.next()? {
            entities.push(self.row_to_entity(row)?);
        }
        Ok(entities)
    }

    pub fn delete_entity(&self, id: &str) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute("DELETE FROM entities WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn save_relationship(&self, relationship: &Relationship) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO relationships (id, source_id, target_id, relation_type, properties, weight, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                relationship.id,
                relationship.source_id,
                relationship.target_id,
                serde_json::to_string(&relationship.relation_type).unwrap_or_default(),
                serde_json::to_string(&relationship.properties).unwrap_or_else(|_| "{}".to_string()),
                relationship.weight,
                relationship.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_relationships_by_entity(&self, entity_id: &str) -> Result<Vec<Relationship>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM relationships WHERE source_id = ?1 OR target_id = ?1"
        )?;
        let mut rows = stmt.query(params![entity_id])?;
        let mut relationships = Vec::new();
        while let Some(row) = rows.next()? {
            relationships.push(self.row_to_relationship(row)?);
        }
        Ok(relationships)
    }

    pub fn get_all_relationships(&self) -> Result<Vec<Relationship>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM relationships ORDER BY created_at DESC")?;
        let mut rows = stmt.query([])?;
        let mut relationships = Vec::new();
        while let Some(row) = rows.next()? {
            relationships.push(self.row_to_relationship(row)?);
        }
        Ok(relationships)
    }

    pub fn delete_relationship(&self, id: &str) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute("DELETE FROM relationships WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn row_to_entity(&self, row: &rusqlite::Row) -> Result<Entity> {
        use crate::memory::EntityType;
        let entity_type_str: String = row.get(2)?;
        let entity_type: EntityType = serde_json::from_str(&format!("\"{}\"", entity_type_str))
            .unwrap_or(EntityType::Concept);
        let properties_str: String = row.get(3).unwrap_or_default();
        let properties: HashMap<String, serde_json::Value> =
            serde_json::from_str(&properties_str).unwrap_or_default();
        let aliases_str: String = row.get(4).unwrap_or_default();
        let aliases: Vec<String> = serde_json::from_str(&aliases_str).unwrap_or_default();
        let first_seen_str: String = row.get(5)?;
        let first_seen_at = chrono::DateTime::parse_from_rfc3339(&first_seen_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let last_seen_str: String = row.get(6)?;
        let last_seen_at = chrono::DateTime::parse_from_rfc3339(&last_seen_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let created_at_str: Option<String> = row.get(9)?;
        let created_at = created_at_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        });
        let updated_at_str: Option<String> = row.get(10)?;
        let updated_at = updated_at_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        });
        Ok(Entity {
            id: row.get(0)?,
            name: row.get(1)?,
            entity_type,
            properties,
            aliases,
            first_seen_at,
            last_seen_at,
            mention_count: row.get(7)?,
            confidence: row.get(8)?,
            created_at,
            updated_at,
        })
    }

    fn row_to_relationship(&self, row: &rusqlite::Row) -> Result<Relationship> {
        use crate::memory::RelationshipType;
        let relation_type_str: String = row.get(3)?;
        let relation_type: RelationshipType =
            serde_json::from_str(&format!("\"{}\"", relation_type_str))
                .unwrap_or(RelationshipType::RelatedTo);
        let properties_str: String = row.get(4).unwrap_or_default();
        let properties: HashMap<String, serde_json::Value> =
            serde_json::from_str(&properties_str).unwrap_or_default();
        let created_at_str: String = row.get(6)?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        Ok(Relationship {
            id: row.get(0)?,
            source_id: row.get(1)?,
            target_id: row.get(2)?,
            relation_type,
            properties,
            weight: row.get(5)?,
            created_at,
        })
    }

    pub fn save_session(&self, session: &Session) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sessions (id, title, platform, user_id, model, system_prompt, created_at, updated_at, parent_session_id, token_input, token_output)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                session.id,
                session.title,
                session.platform,
                session.user_id,
                session.model,
                session.system_prompt,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                session.parent_session_id,
                session.token_input,
                session.token_output,
            ],
        )?;
        Ok(())
    }

    pub fn get_session(&self, id: &str) -> Result<Option<Session>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare("SELECT id, title, platform, user_id, model, system_prompt, created_at, updated_at, parent_session_id, token_input, token_output FROM sessions WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(self.row_to_session(row)))
        } else {
            Ok(None)
        }
    }

    pub fn get_all_sessions(&self) -> Result<Vec<Session>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare("SELECT id, title, platform, user_id, model, system_prompt, created_at, updated_at, parent_session_id, token_input, token_output FROM sessions ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([], |row| Ok(self.row_to_session(row)))?;
        let sessions: Vec<Session> = rows.filter_map(|r| r.ok()).collect();
        Ok(sessions)
    }

    pub fn update_session(&self, id: &str, updates: &SessionUpdate) -> Result<()> {
        let conn = self.conn.write().unwrap();
        if let Some(title) = &updates.title {
            conn.execute("UPDATE sessions SET title = ?1, updated_at = ?2 WHERE id = ?3", params![title, Utc::now().to_rfc3339(), id])?;
        }
        if let Some(token_input) = updates.token_input {
            conn.execute("UPDATE sessions SET token_input = ?1, updated_at = ?2 WHERE id = ?3", params![token_input, Utc::now().to_rfc3339(), id])?;
        }
        if let Some(token_output) = updates.token_output {
            conn.execute("UPDATE sessions SET token_output = ?1, updated_at = ?2 WHERE id = ?3", params![token_output, Utc::now().to_rfc3339(), id])?;
        }
        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn row_to_session(&self, row: &rusqlite::Row) -> Session {
        let created_at_str: String = row.get(6).unwrap_or_default();
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let updated_at_str: String = row.get(7).unwrap_or_default();
        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        Session {
            id: row.get(0).unwrap_or_default(),
            title: row.get(1).unwrap_or_default(),
            platform: row.get(2).unwrap_or_default(),
            user_id: row.get(3).unwrap_or_default(),
            model: row.get(4).unwrap_or_default(),
            system_prompt: row.get(5).unwrap_or_default(),
            created_at,
            updated_at,
            parent_session_id: row.get(8).unwrap_or_default(),
            token_input: row.get(9).unwrap_or(0),
            token_output: row.get(10).unwrap_or(0),
        }
    }

    pub fn save_message(&self, message: &Message) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO messages (id, session_id, role, content, tool_calls, tool_results, usage, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                message.id,
                message.session_id,
                message.role,
                message.content,
                message.tool_calls,
                message.tool_results,
                message.usage,
                message.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_messages_by_session(&self, session_id: &str) -> Result<Vec<Message>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare("SELECT id, session_id, role, content, tool_calls, tool_results, usage, created_at FROM messages WHERE session_id = ?1 ORDER BY created_at ASC")?;
        let rows = stmt.query_map(params![session_id], |row| Ok(self.row_to_message(row)))?;
        let messages: Vec<Message> = rows.filter_map(|r| r.ok()).collect();
        Ok(messages)
    }

    pub fn search_messages(&self, query: &str, limit: usize) -> Result<Vec<Message>> {
        let conn = self.conn.read().unwrap();
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare("SELECT id, session_id, role, content, tool_calls, tool_results, usage, created_at FROM messages WHERE content LIKE ?1 ORDER BY created_at DESC LIMIT ?2")?;
        let rows = stmt.query_map(params![pattern, limit], |row| Ok(self.row_to_message(row)))?;
        let messages: Vec<Message> = rows.filter_map(|r| r.ok()).collect();
        Ok(messages)
    }

    fn row_to_message(&self, row: &rusqlite::Row) -> Message {
        let created_at_str: String = row.get(7).unwrap_or_default();
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        Message {
            id: row.get(0).unwrap_or_default(),
            session_id: row.get(1).unwrap_or_default(),
            role: row.get(2).unwrap_or_default(),
            content: row.get(3).unwrap_or_default(),
            tool_calls: row.get(4).unwrap_or_default(),
            tool_results: row.get(5).unwrap_or_default(),
            usage: row.get(6).unwrap_or_default(),
            created_at,
        }
    }

    pub fn save_learning_pattern(&self, pattern: &Pattern) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO patterns (id, pattern, pattern_type, success, failure, last_used, created_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                pattern.id,
                pattern.pattern,
                pattern.pattern_type,
                pattern.success,
                pattern.failure,
                pattern.last_used.to_rfc3339(),
                pattern.created_at.to_rfc3339(),
                pattern.metadata,
            ],
        )?;
        Ok(())
    }

    pub fn get_patterns_list(&self) -> Result<Vec<Pattern>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare("SELECT id, pattern, pattern_type, success, failure, last_used, created_at, metadata FROM patterns")?;
        let rows = stmt.query_map([], |row| Ok(self.row_to_pattern(row)))?;
        let patterns: Vec<Pattern> = rows.filter_map(|r| r.ok()).collect();
        Ok(patterns)
    }

    fn row_to_pattern(&self, row: &rusqlite::Row) -> Pattern {
        let last_used_str: String = row.get(5).unwrap_or_default();
        let last_used = chrono::DateTime::parse_from_rfc3339(&last_used_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let created_at_str: String = row.get(6).unwrap_or_default();
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        Pattern {
            id: row.get(0).unwrap_or_default(),
            pattern: row.get(1).unwrap_or_default(),
            pattern_type: row.get(2).unwrap_or_default(),
            success: row.get(3).unwrap_or(0),
            failure: row.get(4).unwrap_or(0),
            last_used,
            created_at,
            metadata: row.get(7).unwrap_or_default(),
        }
    }

    pub fn update_pattern_stats(&self, id: &str, success_delta: i32, failure_delta: i32) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute(
            "UPDATE patterns SET success = success + ?1, failure = failure + ?2, last_used = ?3 WHERE id = ?4",
            params![success_delta, failure_delta, Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn save_preference(&self, pref: &Preference) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO preferences (id, key, value, confidence, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                pref.id,
                pref.key,
                pref.value,
                pref.confidence,
                pref.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_preferences_list(&self) -> Result<Vec<Preference>> {
        let conn = self.conn.read().unwrap();
        let mut stmt = conn.prepare("SELECT id, key, value, confidence, updated_at FROM preferences")?;
        let rows = stmt.query_map([], |row| Ok(self.row_to_preference(row)))?;
        let prefs: Vec<Preference> = rows.filter_map(|r| r.ok()).collect();
        Ok(prefs)
    }

    fn row_to_preference(&self, row: &rusqlite::Row) -> Preference {
        let updated_at_str: String = row.get(4).unwrap_or_default();
        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        Preference {
            id: row.get(0).unwrap_or_default(),
            key: row.get(1).unwrap_or_default(),
            value: row.get(2).unwrap_or_default(),
            confidence: row.get(3).unwrap_or(0.0),
            updated_at,
        }
    }

    pub fn update_preference_by_key(&self, key: &str, updates: &Preference) -> Result<()> {
        let conn = self.conn.write().unwrap();
        conn.execute(
            "UPDATE preferences SET value = ?1, confidence = ?2, updated_at = ?3 WHERE key = ?4",
            params![updates.value, updates.confidence, Utc::now().to_rfc3339(), key],
        )?;
        Ok(())
    }

    pub fn get_statistics(&self) -> Result<TrajectoryStatistics> {
        let trajectories = self.get_trajectories(Some(100))?;
        let patterns = self.get_patterns()?;
        let sessions: std::collections::HashSet<String> = trajectories.iter().map(|t| t.session_id.clone()).collect();

        let total = trajectories.len();
        let avg_quality = if total > 0 {
            trajectories.iter().map(|t| t.quality.overall).sum::<f64>() / total as f64
        } else {
            0.0
        };
        let avg_value = if total > 0 {
            trajectories.iter().map(|t| t.value_score).sum::<f64>() / total as f64
        } else {
            0.0
        };

        Ok(TrajectoryStatistics {
            total_trajectories: total,
            total_sessions: sessions.len(),
            total_patterns: patterns.len(),
            avg_quality,
            avg_value_score: avg_value,
            success_rate: 0.0,
            recent_trajectories: total,
        })
    }
}

use tokio::sync::{mpsc, mpsc::Sender};
use std::collections::VecDeque;

pub struct TrajectoryQueue {
    storage: Arc<TrajectoryStorage>,
    sender: Sender<Trajectory>,
    handle: tokio::task::JoinHandle<()>,
}

impl TrajectoryQueue {
    pub fn new(storage: Arc<TrajectoryStorage>, buffer_size: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<Trajectory>(buffer_size);
        let storage_clone = storage.clone();

        let handle = tokio::spawn(async move {
            let mut batch: VecDeque<Trajectory> = VecDeque::with_capacity(32);
            let mut flush_interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

            loop {
                tokio::select! {
                    Some(trajectory) = rx.recv() => {
                        batch.push_back(trajectory);
                        if batch.len() >= 32 {
                            Self::flush_batch(&storage_clone, &mut batch).await;
                        }
                    }
                    _ = flush_interval.tick() => {
                        if !batch.is_empty() {
                            Self::flush_batch(&storage_clone, &mut batch).await;
                        }
                    }
                }
            }
        });

        Self {
            storage,
            sender: tx,
            handle,
        }
    }

    async fn flush_batch(storage: &Arc<TrajectoryStorage>, batch: &mut VecDeque<Trajectory>) {
        while let Some(trajectory) = batch.pop_front() {
            if let Err(e) = storage.save_trajectory(&trajectory) {
                tracing::warn!("[TrajectoryQueue] Failed to save trajectory: {}", e);
            }
        }
    }

    pub fn try_enqueue(&self, trajectory: Trajectory) -> bool {
        self.sender.try_send(trajectory).is_ok()
    }

    pub async fn enqueue(&self, trajectory: Trajectory) -> Result<(), mpsc::error::TrySendError<Trajectory>> {
        self.sender.send(trajectory.clone()).await.map_err(|_| mpsc::error::TrySendError::Closed(trajectory))
    }

    pub fn storage(&self) -> &Arc<TrajectoryStorage> {
        &self.storage
    }

    pub fn shutdown(self) {
        self.handle.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_trajectory_queue_enqueue() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_trajectory.db");
        let storage = Arc::new(TrajectoryStorage::new_with_path(&db_path).unwrap());
        let queue = TrajectoryQueue::new(storage.clone(), 10);

        let trajectory = Trajectory::new(
            "test-session".to_string(),
            "test-user".to_string(),
            "Test topic".to_string(),
            "Test summary".to_string(),
            TrajectoryOutcome::Success,
            1000,
            vec![],
        );

        assert!(queue.try_enqueue(trajectory));
    }
}

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

unsafe impl Send for TrajectoryStorage {}
unsafe impl Sync for TrajectoryStorage {}
