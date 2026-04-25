use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkflowTemplateVersions::Table)
                    .col(
                        ColumnDef::new(WorkflowTemplateVersions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkflowTemplateVersions::TemplateId).string().not_null())
                    .col(ColumnDef::new(WorkflowTemplateVersions::Name).string().not_null())
                    .col(ColumnDef::new(WorkflowTemplateVersions::Description).text().null())
                    .col(ColumnDef::new(WorkflowTemplateVersions::Icon).string().not_null().default("Bot"))
                    .col(ColumnDef::new(WorkflowTemplateVersions::Tags).text().null())
                    .col(ColumnDef::new(WorkflowTemplateVersions::Version).integer().not_null())
                    .col(ColumnDef::new(WorkflowTemplateVersions::IsPreset).boolean().not_null().default(false))
                    .col(ColumnDef::new(WorkflowTemplateVersions::IsEditable).boolean().not_null().default(true))
                    .col(ColumnDef::new(WorkflowTemplateVersions::IsPublic).boolean().not_null().default(false))
                    .col(ColumnDef::new(WorkflowTemplateVersions::TriggerConfig).text().null())
                    .col(ColumnDef::new(WorkflowTemplateVersions::Nodes).text().not_null())
                    .col(ColumnDef::new(WorkflowTemplateVersions::Edges).text().not_null())
                    .col(ColumnDef::new(WorkflowTemplateVersions::InputSchema).text().null())
                    .col(ColumnDef::new(WorkflowTemplateVersions::OutputSchema).text().null())
                    .col(ColumnDef::new(WorkflowTemplateVersions::Variables).text().null())
                    .col(ColumnDef::new(WorkflowTemplateVersions::ErrorConfig).text().null())
                    .col(ColumnDef::new(WorkflowTemplateVersions::CreatedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_wf_ver_template_id")
                    .table(WorkflowTemplateVersions::Table)
                    .col(WorkflowTemplateVersions::TemplateId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_wf_ver_template_version")
                    .table(WorkflowTemplateVersions::Table)
                    .col(WorkflowTemplateVersions::TemplateId)
                    .col(WorkflowTemplateVersions::Version)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkflowTemplateVersions::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum WorkflowTemplateVersions {
    Table,
    Id,
    TemplateId,
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
}
