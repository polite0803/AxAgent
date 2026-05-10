use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250510_000001_memory_tier_decay"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let columns = [
            "ALTER TABLE trajectory_memories ADD COLUMN tier TEXT NOT NULL DEFAULT 'working'",
            "ALTER TABLE trajectory_memories ADD COLUMN importance REAL NOT NULL DEFAULT 0.5",
            "ALTER TABLE trajectory_memories ADD COLUMN access_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE trajectory_memories ADD COLUMN last_accessed TEXT",
            "ALTER TABLE trajectory_memories ADD COLUMN decay_rate REAL NOT NULL DEFAULT 0.01",
            "ALTER TABLE trajectory_memories ADD COLUMN created_at TEXT",
            "ALTER TABLE trajectory_memories ADD COLUMN expires_at TEXT",
            "ALTER TABLE trajectory_memories ADD COLUMN source_conversation_id TEXT",
            "ALTER TABLE trajectory_memories ADD COLUMN source_message_id TEXT",
            "ALTER TABLE trajectory_memories ADD COLUMN memory_nature TEXT NOT NULL DEFAULT 'semantic'",
            "ALTER TABLE trajectory_memories ADD COLUMN tags TEXT NOT NULL DEFAULT '[]'",
        ];

        for sql in &columns {
            if let Err(_e) = db.execute_unprepared(sql).await {
                // Column may already exist from a prior partial migration
            }
        }

        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_traj_memories_tier ON trajectory_memories(tier)",
        )
        .await?;

        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_traj_memories_importance ON trajectory_memories(importance)",
        )
        .await?;

        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_traj_memories_expires ON trajectory_memories(expires_at)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
