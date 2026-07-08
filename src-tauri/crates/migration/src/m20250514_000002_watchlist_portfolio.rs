use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WatchlistItems::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WatchlistItems::Id)
                            .string()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WatchlistItems::StockCode)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WatchlistItems::StockName)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(WatchlistItems::Notes).text())
                    .col(
                        ColumnDef::new(WatchlistItems::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WatchlistItems::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PortfolioHoldings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PortfolioHoldings::Id)
                            .string()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PortfolioHoldings::StockCode)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PortfolioHoldings::StockName)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PortfolioHoldings::Shares)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PortfolioHoldings::AvgCost)
                            .double()
                            .not_null(),
                    )
                    .col(ColumnDef::new(PortfolioHoldings::Notes).text())
                    .col(
                        ColumnDef::new(PortfolioHoldings::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PortfolioHoldings::UpdatedAt)
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
            .drop_table(Table::drop().table(PortfolioHoldings::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WatchlistItems::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum WatchlistItems {
    Table,
    Id,
    StockCode,
    StockName,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum PortfolioHoldings {
    Table,
    Id,
    StockCode,
    StockName,
    Shares,
    AvgCost,
    Notes,
    CreatedAt,
    UpdatedAt,
}
