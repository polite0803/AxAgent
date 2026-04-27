use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ScheduledTasks::Table)
                    .col(
                        ColumnDef::new(ScheduledTasks::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ScheduledTasks::Name).string().not_null())
                    .col(
                        ColumnDef::new(ScheduledTasks::Description)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ScheduledTasks::TaskType).text().not_null())
                    .col(
                        ColumnDef::new(ScheduledTasks::CronExpression)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTasks::IntervalSeconds)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTasks::NextRunAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTasks::LastRunAt)
                            .big_integer()
                            .null(),
                    )
                    .col(ColumnDef::new(ScheduledTasks::LastResult).text().null())
                    .col(ColumnDef::new(ScheduledTasks::Status).text().not_null())
                    .col(ColumnDef::new(ScheduledTasks::Config).text().not_null())
                    .col(
                        ColumnDef::new(ScheduledTasks::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTasks::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ScheduledTasks::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum ScheduledTasks {
    Table,
    Id,
    Name,
    Description,
    TaskType,
    CronExpression,
    IntervalSeconds,
    NextRunAt,
    LastRunAt,
    LastResult,
    Status,
    Config,
    CreatedAt,
    UpdatedAt,
}
