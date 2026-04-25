use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GeneratedTools::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GeneratedTools::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GeneratedTools::ToolName)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(GeneratedTools::OriginalName)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GeneratedTools::OriginalDescription)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GeneratedTools::InputSchema)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GeneratedTools::OutputSchema)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GeneratedTools::Implementation)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GeneratedTools::SourceInfo)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GeneratedTools::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GeneratedTools::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum GeneratedTools {
    Table,
    Id,
    ToolName,
    OriginalName,
    OriginalDescription,
    InputSchema,
    OutputSchema,
    Implementation,
    SourceInfo,
    CreatedAt,
}
