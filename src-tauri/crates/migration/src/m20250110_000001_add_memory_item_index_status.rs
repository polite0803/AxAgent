use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add index_status column to memory_items: "pending" | "indexing" | "ready" | "failed" | "skipped"
        // - "pending": just created, not yet indexed
        // - "indexing": embedding generation in progress
        // - "ready": successfully indexed
        // - "failed": indexing failed
        // - "skipped": namespace has no embedding provider
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryItems::Table)
                    .add_column(
                        ColumnDef::new(MemoryItems::IndexStatus)
                            .string()
                            .not_null()
                            .default("pending")
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryItems::Table)
                    .drop_column(MemoryItems::IndexStatus)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum MemoryItems {
    Table,
    IndexStatus,
}
