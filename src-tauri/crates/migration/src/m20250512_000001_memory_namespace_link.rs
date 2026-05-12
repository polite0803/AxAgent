use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250512_000001_memory_namespace_link"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("ALTER TABLE trajectory_memories ADD COLUMN namespace_id TEXT")
            .await?;

        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_traj_memories_namespace ON trajectory_memories(namespace_id)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
