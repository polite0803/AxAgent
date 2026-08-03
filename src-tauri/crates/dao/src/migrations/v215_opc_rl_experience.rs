// SPDX-License-Identifier: AGPL-3.0-only

//! v215: OPC 强化学习经验持久化表

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let create_rl_experiences = r#"
CREATE TABLE IF NOT EXISTS opc_rl_experiences (
    id TEXT NOT NULL PRIMARY KEY,
    industry_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    quality_score REAL NOT NULL DEFAULT 0.0,
    efficiency_score REAL NOT NULL DEFAULT 0.0,
    cost_score REAL NOT NULL DEFAULT 0.0,
    innovation_score REAL NOT NULL DEFAULT 0.0,
    satisfaction_score REAL NOT NULL DEFAULT 0.0,
    total_reward REAL NOT NULL DEFAULT 0.0,
    step_count INTEGER NOT NULL DEFAULT 0,
    success INTEGER NOT NULL DEFAULT 0,
    metadata TEXT NOT NULL DEFAULT '{}'
)"#;

    let create_rl_training_stats = r#"
CREATE TABLE IF NOT EXISTS opc_rl_training_stats (
    industry_id TEXT NOT NULL PRIMARY KEY,
    total_experiences INTEGER NOT NULL DEFAULT 0,
    total_reward REAL NOT NULL DEFAULT 0.0,
    avg_reward REAL NOT NULL DEFAULT 0.0,
    success_rate REAL NOT NULL DEFAULT 0.0,
    last_trained_at INTEGER,
    policy_updated_at INTEGER,
    optimization_goals TEXT NOT NULL DEFAULT '[]'
)"#;

    let indices = [
        "CREATE INDEX IF NOT EXISTS idx_rl_experiences_industry ON opc_rl_experiences(industry_id)",
        "CREATE INDEX IF NOT EXISTS idx_rl_experiences_workflow ON opc_rl_experiences(workflow_id)",
        "CREATE INDEX IF NOT EXISTS idx_rl_experiences_timestamp ON opc_rl_experiences(timestamp_ms)",
    ];

    for stmt in [create_rl_experiences, create_rl_training_stats] {
        db.execute_unprepared(stmt).await?;
    }
    for stmt in &indices {
        db.execute_unprepared(stmt).await?;
    }

    Ok(())
}
