use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BackgroundTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BackgroundTasks::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(BackgroundTasks::Title).string().not_null())
                    .col(
                        ColumnDef::new(BackgroundTasks::Description)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(BackgroundTasks::TaskType)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(BackgroundTasks::Command).text())
                    .col(ColumnDef::new(BackgroundTasks::Prompt).text())
                    .col(
                        ColumnDef::new(BackgroundTasks::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(BackgroundTasks::Output)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(BackgroundTasks::ExitCode).integer())
                    .col(ColumnDef::new(BackgroundTasks::ConversationId).string())
                    .col(ColumnDef::new(BackgroundTasks::CreatedBy).string())
                    .col(
                        ColumnDef::new(BackgroundTasks::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BackgroundTasks::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(BackgroundTasks::FinishedAt).big_integer())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BackgroundTasks::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum BackgroundTasks {
    Table,
    Id,
    Title,
    Description,
    TaskType,
    Command,
    Prompt,
    Status,
    Output,
    ExitCode,
    ConversationId,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
    FinishedAt,
}
