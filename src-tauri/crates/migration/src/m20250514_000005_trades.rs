use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Trades::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Trades::Id).string().primary_key().not_null())
                    .col(ColumnDef::new(Trades::StockCode).string().not_null())
                    .col(ColumnDef::new(Trades::StockName).string().not_null())
                    .col(ColumnDef::new(Trades::Direction).string().not_null())
                    .col(ColumnDef::new(Trades::Price).double().not_null())
                    .col(ColumnDef::new(Trades::Quantity).integer().not_null())
                    .col(ColumnDef::new(Trades::TradeDate).string().not_null())
                    .col(ColumnDef::new(Trades::TradeTime).string().not_null())
                    .col(ColumnDef::new(Trades::Fee).double())
                    .col(ColumnDef::new(Trades::RealizedPnl).double())
                    .col(ColumnDef::new(Trades::Notes).text())
                    .col(
                        ColumnDef::new(Trades::CreatedAt)
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
            .drop_table(Table::drop().table(Trades::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Trades {
    Table,
    Id,
    StockCode,
    StockName,
    Direction,
    Price,
    Quantity,
    TradeDate,
    TradeTime,
    Fee,
    RealizedPnl,
    Notes,
    CreatedAt,
}
