use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250510_000002_conversation_memory_status"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let columns = [
            "ALTER TABLE conversations ADD COLUMN memory_status TEXT NOT NULL DEFAULT 'none'",
            "ALTER TABLE conversations ADD COLUMN last_memory_extracted_at TEXT",
        ];

        for sql in &columns {
            if let Err(_e) = db.execute_unprepared(sql).await {}
        }

        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_conversations_memory_status ON conversations(memory_status)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
