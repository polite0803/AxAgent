use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkflowExecutions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkflowExecutions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WorkflowExecutions::WorkflowId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowExecutions::Status)
                            .string()
                            .not_null()
                            .default("running"),
                    )
                    .col(ColumnDef::new(WorkflowExecutions::InputParams).string().null())
                    .col(ColumnDef::new(WorkflowExecutions::OutputResult).string().null())
                    .col(ColumnDef::new(WorkflowExecutions::NodeExecutions).string().null())
                    .col(
                        ColumnDef::new(WorkflowExecutions::TotalTimeMs)
                            .integer()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(WorkflowExecutions::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowExecutions::UpdatedAt)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Index for querying executions by workflow
        manager
            .create_index(
                Index::create()
                    .name("idx_workflow_executions_workflow")
                    .table(WorkflowExecutions::Table)
                    .col(WorkflowExecutions::WorkflowId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkflowExecutions::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum WorkflowExecutions {
    Table,
    Id,
    WorkflowId,
    Status,
    InputParams,
    OutputResult,
    NodeExecutions,
    TotalTimeMs,
    CreatedAt,
    UpdatedAt,
}
