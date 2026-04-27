use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("tool_executions"))
                    .add_column(
                        ColumnDef::new(Alias::new("skill_steps_json"))
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("tool_executions"))
                    .add_column(
                        ColumnDef::new(Alias::new("depends_on"))
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
                    .table(Alias::new("tool_executions"))
                    .drop_column(Alias::new("skill_steps_json"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("tool_executions"))
                    .drop_column(Alias::new("depends_on"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}