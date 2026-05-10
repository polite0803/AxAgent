use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250510_000004_wiki_templates"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("wiki_templates"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("wiki_id")).text().not_null())
                    .col(ColumnDef::new(Alias::new("name")).text().not_null())
                    .col(ColumnDef::new(Alias::new("description")).text())
                    .col(ColumnDef::new(Alias::new("content")).text().not_null())
                    .col(ColumnDef::new(Alias::new("page_type")).text())
                    .col(
                        ColumnDef::new(Alias::new("is_builtin"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("updated_at"))
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_wiki_templates_wiki_id")
                    .table(Alias::new("wiki_templates"))
                    .col(Alias::new("wiki_id"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("wiki_templates")).to_owned())
            .await
    }
}
