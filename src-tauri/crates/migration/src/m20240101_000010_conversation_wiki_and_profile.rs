use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .add_column(
                        ColumnDef::new(Conversations::EnabledWikiIds)
                            .text()
                            .not_null()
                            .default("[]"),
                    )
                    .add_column(
                        ColumnDef::new(Conversations::AgentProfileId)
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .drop_column(Conversations::EnabledWikiIds)
                    .drop_column(Conversations::AgentProfileId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Conversations {
    Table,
    EnabledWikiIds,
    AgentProfileId,
}
