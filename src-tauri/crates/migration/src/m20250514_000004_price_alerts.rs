use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PriceAlerts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PriceAlerts::Id)
                            .string()
                            .primary_key()
                            .not_null(),
                    )
                    .col(ColumnDef::new(PriceAlerts::StockCode).string().not_null())
                    .col(ColumnDef::new(PriceAlerts::StockName).string().not_null())
                    .col(ColumnDef::new(PriceAlerts::Condition).string().not_null())
                    .col(ColumnDef::new(PriceAlerts::TargetPrice).double().not_null())
                    .col(
                        ColumnDef::new(PriceAlerts::IsTriggered)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(PriceAlerts::TriggeredAt).big_integer())
                    .col(
                        ColumnDef::new(PriceAlerts::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PriceAlerts::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PriceAlerts::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum PriceAlerts {
    Table,
    Id,
    StockCode,
    StockName,
    Condition,
    TargetPrice,
    IsTriggered,
    TriggeredAt,
    CreatedAt,
    UpdatedAt,
}
