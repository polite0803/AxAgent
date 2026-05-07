use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Wikis::Table)
                    .add_column(ColumnDef::new(Wikis::EmbeddingDimensions).integer().null())
                    .add_column(ColumnDef::new(Wikis::RetrievalThreshold).float().null())
                    .add_column(ColumnDef::new(Wikis::RetrievalTopK).integer().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Wikis::Table)
                    .drop_column(Wikis::EmbeddingDimensions)
                    .drop_column(Wikis::RetrievalThreshold)
                    .drop_column(Wikis::RetrievalTopK)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Wikis {
    Table,
    EmbeddingDimensions,
    RetrievalThreshold,
    RetrievalTopK,
}
