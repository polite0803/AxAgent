use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkflowSnapshots::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkflowSnapshots::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WorkflowSnapshots::WorkflowId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowSnapshots::SnapshotJson)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowSnapshots::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(WorkflowSnapshots::StepId).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkflowSnapshots::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum WorkflowSnapshots {
    Table,
    Id,
    WorkflowId,
    SnapshotJson,
    CreatedAt,
    StepId,
}
