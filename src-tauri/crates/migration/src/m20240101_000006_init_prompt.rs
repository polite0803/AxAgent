use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20240101_000006_init_prompt"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PromptTemplates::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PromptTemplates::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PromptTemplates::Name).string().not_null())
                    .col(ColumnDef::new(PromptTemplates::Description).string().null())
                    .col(ColumnDef::new(PromptTemplates::Content).string().not_null())
                    .col(
                        ColumnDef::new(PromptTemplates::VariablesSchema)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PromptTemplates::Version)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(PromptTemplates::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(PromptTemplates::AbTestEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(PromptTemplates::AbTestVariant)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PromptTemplates::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PromptTemplates::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PromptTemplateVersions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PromptTemplateVersions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PromptTemplateVersions::TemplateId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PromptTemplateVersions::Version)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PromptTemplateVersions::Name)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PromptTemplateVersions::Description)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PromptTemplateVersions::Content)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PromptTemplateVersions::VariablesSchema)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PromptTemplateVersions::Changelog)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PromptTemplateVersions::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(PromptTemplateVersions::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(PromptTemplates::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum PromptTemplates {
    Table,
    Id,
    Name,
    Description,
    Content,
    VariablesSchema,
    Version,
    IsActive,
    AbTestEnabled,
    AbTestVariant,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum PromptTemplateVersions {
    Table,
    Id,
    TemplateId,
    Version,
    Name,
    Description,
    Content,
    VariablesSchema,
    Changelog,
    CreatedAt,
}
