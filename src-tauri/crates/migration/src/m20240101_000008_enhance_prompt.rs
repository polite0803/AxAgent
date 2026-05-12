use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20240101_000008_enhance_prompt"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // prompt_templates 新增字段
        for (col, def) in [
            ("category", ColumnDef::new(Alias::new("category")).string().null()),
            ("tags", ColumnDef::new(Alias::new("tags")).string().null()),
            ("author", ColumnDef::new(Alias::new("author")).string().null()),
            ("source", ColumnDef::new(Alias::new("source")).string().null()),
            ("source_type", ColumnDef::new(Alias::new("source_type")).string().null()),
            (
                "format",
                ColumnDef::new(Alias::new("format"))
                    .string()
                    .null()
                    .default("plain"),
            ),
            ("metadata_json", ColumnDef::new(Alias::new("metadata_json")).string().null()),
            (
                "usage_count",
                ColumnDef::new(Alias::new("usage_count"))
                    .integer()
                    .not_null()
                    .default(0),
            ),
            (
                "is_favorite",
                ColumnDef::new(Alias::new("is_favorite"))
                    .boolean()
                    .not_null()
                    .default(false),
            ),
        ] {
            if !manager.has_column("prompt_templates", col).await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(PromptTemplates::Table)
                            .add_column(def)
                            .to_owned(),
                    )
                    .await?;
            }
        }

        // prompt_template_versions 新增字段
        for (col, def) in [
            ("category", ColumnDef::new(Alias::new("category")).string().null()),
            ("tags", ColumnDef::new(Alias::new("tags")).string().null()),
            ("author", ColumnDef::new(Alias::new("author")).string().null()),
            ("source", ColumnDef::new(Alias::new("source")).string().null()),
        ] {
            if !manager.has_column("prompt_template_versions", col).await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(PromptTemplateVersions::Table)
                            .add_column(def)
                            .to_owned(),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite 不支持 DROP COLUMN，降级时不做处理
        Ok(())
    }
}

#[derive(Iden)]
enum PromptTemplates {
    Table,
}

#[derive(Iden)]
enum PromptTemplateVersions {
    Table,
}
