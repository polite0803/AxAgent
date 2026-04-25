use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkflowTemplates::Table)
                    .add_column(ColumnDef::new(WorkflowTemplates::CompositeSource).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkflowTemplates::Table)
                    .drop_column(WorkflowTemplates::CompositeSource)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum WorkflowTemplates {
    Table,
    CompositeSource,
}
