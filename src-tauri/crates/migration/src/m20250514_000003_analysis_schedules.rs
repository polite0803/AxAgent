use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AnalysisSchedules::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AnalysisSchedules::Id)
                            .string()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AnalysisSchedules::StockCode)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AnalysisSchedules::StockName)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AnalysisSchedules::CronExpression)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AnalysisSchedules::ProviderId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AnalysisSchedules::IsEnabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(AnalysisSchedules::LastRunAt).big_integer())
                    .col(ColumnDef::new(AnalysisSchedules::NextRunAt).big_integer())
                    .col(
                        ColumnDef::new(AnalysisSchedules::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AnalysisSchedules::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AnalysisSchedules::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AnalysisSchedules {
    Table,
    Id,
    StockCode,
    StockName,
    CronExpression,
    ProviderId,
    IsEnabled,
    LastRunAt,
    NextRunAt,
    CreatedAt,
    UpdatedAt,
}
