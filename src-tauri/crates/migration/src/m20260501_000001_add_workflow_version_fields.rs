use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkflowTemplateVersions::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(WorkflowTemplateVersions::Changelog)
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WorkflowTemplateVersions::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(WorkflowTemplateVersions::IsActive)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_wf_ver_active")
                    .table(WorkflowTemplateVersions::Table)
                    .col(WorkflowTemplateVersions::TemplateId)
                    .col(WorkflowTemplateVersions::IsActive)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_wf_ver_template_version_active")
                    .table(WorkflowTemplateVersions::Table)
                    .col(WorkflowTemplateVersions::TemplateId)
                    .col(WorkflowTemplateVersions::Version)
                    .col(WorkflowTemplateVersions::IsActive)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkflowTemplateVersions::Table)
                    .drop_column(WorkflowTemplateVersions::Changelog)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WorkflowTemplateVersions::Table)
                    .drop_column(WorkflowTemplateVersions::IsActive)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum WorkflowTemplateVersions {
    Table,
    TemplateId,
    Version,
    Changelog,
    IsActive,
}
