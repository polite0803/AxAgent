use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ScheduledTasks::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(ScheduledTasks::WorkflowId)
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
                    .table(ScheduledTasks::Table)
                    .drop_column(ScheduledTasks::WorkflowId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
pub enum ScheduledTasks {
    Table,
    WorkflowId,
}