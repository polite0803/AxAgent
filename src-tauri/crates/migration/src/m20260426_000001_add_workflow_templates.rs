use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkflowTemplates::Table)
                    .col(
                        ColumnDef::new(WorkflowTemplates::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkflowTemplates::Name).string().not_null())
                    .col(ColumnDef::new(WorkflowTemplates::Description).text().null())
                    .col(
                        ColumnDef::new(WorkflowTemplates::Icon)
                            .string()
                            .not_null()
                            .default("Bot"),
                    )
                    .col(ColumnDef::new(WorkflowTemplates::Tags).text().null())
                    .col(
                        ColumnDef::new(WorkflowTemplates::Version)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(WorkflowTemplates::IsPreset)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(WorkflowTemplates::IsEditable)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(WorkflowTemplates::IsPublic)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(WorkflowTemplates::TriggerConfig)
                            .text()
                            .null(),
                    )
                    .col(ColumnDef::new(WorkflowTemplates::Nodes).text().not_null())
                    .col(ColumnDef::new(WorkflowTemplates::Edges).text().not_null())
                    .col(ColumnDef::new(WorkflowTemplates::InputSchema).text().null())
                    .col(
                        ColumnDef::new(WorkflowTemplates::OutputSchema)
                            .text()
                            .null(),
                    )
                    .col(ColumnDef::new(WorkflowTemplates::Variables).text().null())
                    .col(ColumnDef::new(WorkflowTemplates::ErrorConfig).text().null())
                    .col(
                        ColumnDef::new(WorkflowTemplates::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowTemplates::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_workflow_templates_preset")
                    .table(WorkflowTemplates::Table)
                    .col(WorkflowTemplates::IsPreset)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_workflow_templates_updated")
                    .table(WorkflowTemplates::Table)
                    .col(WorkflowTemplates::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkflowTemplates::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum WorkflowTemplates {
    Table,
    Id,
    Name,
    Description,
    Icon,
    Tags,
    Version,
    IsPreset,
    IsEditable,
    IsPublic,
    TriggerConfig,
    Nodes,
    Edges,
    InputSchema,
    OutputSchema,
    Variables,
    ErrorConfig,
    CreatedAt,
    UpdatedAt,
}
