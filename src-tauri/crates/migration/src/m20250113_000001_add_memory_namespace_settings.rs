use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add embedding_dimensions column (nullable integer)
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryNamespaces::Table)
                    .add_column(
                        ColumnDef::new(MemoryNamespaces::EmbeddingDimensions)
                            .integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add retrieval_threshold column (nullable float, app default 0.1)
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryNamespaces::Table)
                    .add_column(
                        ColumnDef::new(MemoryNamespaces::RetrievalThreshold)
                            .float()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add retrieval_top_k column (nullable integer, app default 5)
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryNamespaces::Table)
                    .add_column(
                        ColumnDef::new(MemoryNamespaces::RetrievalTopK)
                            .integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite does not support DROP COLUMN; skip for down migration
        Ok(())
    }
}

#[derive(DeriveIden)]
enum MemoryNamespaces {
    Table,
    EmbeddingDimensions,
    RetrievalThreshold,
    RetrievalTopK,
}
