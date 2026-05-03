use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let table = Alias::new("wikis");

        // note_count: 笔记计数
        if !manager.has_column("wikis", "note_count").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(table.clone())
                        .add_column(
                            ColumnDef::new(Alias::new("note_count"))
                                .integer()
                                .not_null()
                                .default(0),
                        )
                        .to_owned(),
                )
                .await?;
        }

        // source_count: 数据源计数
        if !manager.has_column("wikis", "source_count").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(table.clone())
                        .add_column(
                            ColumnDef::new(Alias::new("source_count"))
                                .integer()
                                .not_null()
                                .default(0),
                        )
                        .to_owned(),
                )
                .await?;
        }

        // embedding_provider: 嵌入模型提供商
        if !manager.has_column("wikis", "embedding_provider").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(table.clone())
                        .add_column(
                            ColumnDef::new(Alias::new("embedding_provider"))
                                .string()
                                .null(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let table = Alias::new("wikis");

        for col in &["note_count", "source_count", "embedding_provider"] {
            if manager.has_column("wikis", col).await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(table.clone())
                            .drop_column(Alias::new(*col))
                            .to_owned(),
                    )
                    .await?;
            }
        }

        Ok(())
    }
}
