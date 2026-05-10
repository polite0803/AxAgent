use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250510_000005_wiki_page_versions"
    }
}

#[derive(DeriveIden)]
enum WikiPageVersions {
    Table,
    Id,
    WikiId,
    NoteId,
    Title,
    Content,
    ContentHash,
    Author,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WikiPageVersions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WikiPageVersions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WikiPageVersions::WikiId).text().not_null())
                    .col(ColumnDef::new(WikiPageVersions::NoteId).text().not_null())
                    .col(ColumnDef::new(WikiPageVersions::Title).text().not_null())
                    .col(ColumnDef::new(WikiPageVersions::Content).text().not_null())
                    .col(ColumnDef::new(WikiPageVersions::ContentHash).text().not_null())
                    .col(ColumnDef::new(WikiPageVersions::Author).text().not_null())
                    .col(ColumnDef::new(WikiPageVersions::CreatedAt).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_wiki_page_versions_note_id")
                    .table(WikiPageVersions::Table)
                    .col(WikiPageVersions::NoteId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_wiki_page_versions_wiki_id")
                    .table(WikiPageVersions::Table)
                    .col(WikiPageVersions::WikiId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WikiPageVersions::Table).to_owned())
            .await
    }
}
