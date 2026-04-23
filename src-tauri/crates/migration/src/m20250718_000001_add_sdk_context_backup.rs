use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("agent_sessions"))
                    .add_column(
                        ColumnDef::new(Alias::new("sdk_context_backup_json"))
                            .text()
                            .null(),
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
                    .table(Alias::new("agent_sessions"))
                    .drop_column(Alias::new("sdk_context_backup_json"))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}
