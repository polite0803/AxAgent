use sea_orm_migration::prelude::*;

pub struct Migration;
impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20240101_000005_init_trajectory"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let tables: Vec<(&str, &str)> = vec![
            ("trajectory_trajectories", "id TEXT PRIMARY KEY, session_id TEXT NOT NULL, user_id TEXT NOT NULL, topic TEXT NOT NULL, summary TEXT NOT NULL, outcome TEXT NOT NULL, duration_ms INTEGER NOT NULL, quality_overall REAL NOT NULL, quality_task_completion REAL NOT NULL, quality_tool_efficiency REAL NOT NULL, quality_reasoning_quality REAL NOT NULL, quality_user_satisfaction REAL NOT NULL, value_score REAL NOT NULL, patterns TEXT NOT NULL, created_at TEXT NOT NULL, replay_count INTEGER NOT NULL DEFAULT 0, last_replay_at TEXT"),
            ("trajectory_steps", "id INTEGER PRIMARY KEY AUTOINCREMENT, trajectory_id TEXT NOT NULL, step_index INTEGER NOT NULL, timestamp_ms INTEGER NOT NULL, role TEXT NOT NULL, content TEXT NOT NULL, reasoning TEXT, tool_calls TEXT, tool_results TEXT"),
            ("trajectory_rewards", "id TEXT PRIMARY KEY, trajectory_id TEXT NOT NULL, reward_type TEXT NOT NULL, value REAL NOT NULL, created_at TEXT NOT NULL"),
            ("trajectory_skills", "id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL, skill_type TEXT NOT NULL, content TEXT NOT NULL, category TEXT NOT NULL, tags TEXT NOT NULL, scenarios TEXT NOT NULL DEFAULT '[]', parameters TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, usage_count INTEGER NOT NULL DEFAULT 0, success_rate REAL NOT NULL DEFAULT 0.0, avg_execution_time_ms REAL NOT NULL DEFAULT 0.0"),
            ("trajectory_skill_executions", "id TEXT PRIMARY KEY, skill_id TEXT NOT NULL, trajectory_id TEXT, success INTEGER NOT NULL, execution_time_ms INTEGER NOT NULL, created_at TEXT NOT NULL, input_args TEXT, output_result TEXT"),
            ("trajectory_patterns", "id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL, pattern_type TEXT NOT NULL, trajectory_ids TEXT NOT NULL, frequency INTEGER NOT NULL, success_rate REAL NOT NULL, average_quality REAL NOT NULL, average_value_score REAL NOT NULL, reward_profile TEXT NOT NULL, created_at TEXT NOT NULL"),
            ("trajectory_entities", "id TEXT PRIMARY KEY, name TEXT NOT NULL, entity_type TEXT NOT NULL, properties TEXT NOT NULL DEFAULT '{}', aliases TEXT NOT NULL DEFAULT '[]', first_seen_at TEXT NOT NULL, last_seen_at TEXT NOT NULL, mention_count INTEGER NOT NULL DEFAULT 1, confidence REAL NOT NULL DEFAULT 0.5, created_at TEXT, updated_at TEXT"),
            ("trajectory_relationships", "id TEXT PRIMARY KEY, source_id TEXT NOT NULL, target_id TEXT NOT NULL, relation_type TEXT NOT NULL, properties TEXT NOT NULL DEFAULT '{}', weight REAL NOT NULL DEFAULT 1.0, created_at TEXT NOT NULL"),
            ("trajectory_sessions", "id TEXT PRIMARY KEY, title TEXT NOT NULL, platform TEXT NOT NULL DEFAULT 'web', user_id TEXT NOT NULL DEFAULT 'default', model TEXT NOT NULL DEFAULT 'unknown', system_prompt TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL, updated_at TEXT NOT NULL, parent_session_id TEXT, token_input INTEGER NOT NULL DEFAULT 0, token_output INTEGER NOT NULL DEFAULT 0"),
            ("trajectory_messages", "id TEXT PRIMARY KEY, session_id TEXT NOT NULL, role TEXT NOT NULL, content TEXT NOT NULL, tool_calls TEXT, tool_results TEXT, usage TEXT, created_at TEXT NOT NULL"),
            ("trajectory_memories", "id TEXT PRIMARY KEY, memory_type TEXT NOT NULL, content TEXT NOT NULL, updated_at TEXT NOT NULL"),
            ("trajectory_learned_patterns", "id TEXT PRIMARY KEY, pattern TEXT NOT NULL, pattern_type TEXT NOT NULL, success INTEGER NOT NULL DEFAULT 0, failure INTEGER NOT NULL DEFAULT 0, last_used TEXT NOT NULL, created_at TEXT NOT NULL, metadata TEXT"),
            ("trajectory_preferences", "id TEXT PRIMARY KEY, key TEXT NOT NULL UNIQUE, value TEXT NOT NULL, confidence REAL NOT NULL DEFAULT 0.0, updated_at TEXT NOT NULL"),
        ];

        for (name, columns) in &tables {
            let sql = format!("CREATE TABLE IF NOT EXISTS {} ({})", name, columns);
            manager.get_connection().execute_unprepared(&sql).await?;
        }

        // Indexes
        let indexes = [
            "CREATE INDEX IF NOT EXISTS idx_traj_trajectories_session ON trajectory_trajectories(session_id)",
            "CREATE INDEX IF NOT EXISTS idx_traj_trajectories_user ON trajectory_trajectories(user_id)",
            "CREATE INDEX IF NOT EXISTS idx_traj_trajectories_created ON trajectory_trajectories(created_at)",
            "CREATE INDEX IF NOT EXISTS idx_traj_steps_traj ON trajectory_steps(trajectory_id, step_index)",
            "CREATE INDEX IF NOT EXISTS idx_traj_skill_exec ON trajectory_skill_executions(skill_id, created_at)",
            "CREATE INDEX IF NOT EXISTS idx_traj_patterns_type ON trajectory_patterns(pattern_type)",
            "CREATE INDEX IF NOT EXISTS idx_traj_entities_type ON trajectory_entities(entity_type)",
            "CREATE INDEX IF NOT EXISTS idx_traj_entities_name ON trajectory_entities(name)",
            "CREATE INDEX IF NOT EXISTS idx_traj_rel_source ON trajectory_relationships(source_id)",
            "CREATE INDEX IF NOT EXISTS idx_traj_rel_target ON trajectory_relationships(target_id)",
            "CREATE INDEX IF NOT EXISTS idx_traj_sessions_updated ON trajectory_sessions(updated_at)",
            "CREATE INDEX IF NOT EXISTS idx_traj_messages_session ON trajectory_messages(session_id)",
            "CREATE INDEX IF NOT EXISTS idx_traj_memories_type ON trajectory_memories(memory_type)",
            "CREATE INDEX IF NOT EXISTS idx_traj_learned_type ON trajectory_learned_patterns(pattern_type)",
            "CREATE INDEX IF NOT EXISTS idx_traj_prefs_key ON trajectory_preferences(key)",
        ];
        for idx in &indexes {
            manager.get_connection().execute_unprepared(idx).await?;
        }

        // FTS5 virtual tables for full-text search
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE VIRTUAL TABLE IF NOT EXISTS trajectories_fts USING fts5(\
                id UNINDEXED, session_id UNINDEXED, topic, summary, content, \
                outcome UNINDEXED, quality_score UNINDEXED, created_at UNINDEXED, \
                tokenize='porter unicode61')",
        )
        .await?;
        db.execute_unprepared(
            "CREATE VIRTUAL TABLE IF NOT EXISTS trajectory_memories_fts USING fts5(\
                id UNINDEXED, memory_type UNINDEXED, content, entities, \
                created_at UNINDEXED, tokenize='porter unicode61')",
        )
        .await?;
        db.execute_unprepared(
            "CREATE VIRTUAL TABLE IF NOT EXISTS trajectory_skills_fts USING fts5(\
                id UNINDEXED, name, description, content, category UNINDEXED, \
                tags, created_at UNINDEXED, tokenize='porter unicode61')",
        )
        .await?;
        db.execute_unprepared(
            "CREATE VIRTUAL TABLE IF NOT EXISTS trajectory_messages_fts USING fts5(\
                id UNINDEXED, session_id UNINDEXED, role UNINDEXED, content, \
                created_at UNINDEXED, tokenize='porter unicode61')",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // Drop FTS5 virtual tables first
        db.execute_unprepared("DROP TABLE IF EXISTS trajectory_messages_fts")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS trajectory_skills_fts")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS trajectory_memories_fts")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS trajectories_fts")
            .await?;

        let tables = [
            "trajectory_preferences",
            "trajectory_learned_patterns",
            "trajectory_memories",
            "trajectory_messages",
            "trajectory_sessions",
            "trajectory_relationships",
            "trajectory_entities",
            "trajectory_patterns",
            "trajectory_skill_executions",
            "trajectory_skills",
            "trajectory_rewards",
            "trajectory_steps",
            "trajectory_trajectories",
        ];
        for t in &tables {
            manager
                .get_connection()
                .execute_unprepared(&format!("DROP TABLE IF EXISTS {}", t))
                .await?;
        }
        Ok(())
    }
}
