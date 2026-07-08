use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StockAnalyses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StockAnalyses::Id)
                            .string()
                            .primary_key()
                            .not_null(),
                    )
                    .col(ColumnDef::new(StockAnalyses::StockCode).string().not_null())
                    .col(ColumnDef::new(StockAnalyses::StockName).string().not_null())
                    .col(
                        ColumnDef::new(StockAnalyses::AnalysisDate)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockAnalyses::ProviderId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockAnalyses::ConversationId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(StockAnalyses::Status).string().not_null())
                    .col(ColumnDef::new(StockAnalyses::DecisionAction).string())
                    .col(ColumnDef::new(StockAnalyses::DecisionPositionPct).double())
                    .col(ColumnDef::new(StockAnalyses::DecisionReasoning).text())
                    .col(ColumnDef::new(StockAnalyses::DecisionJson).text())
                    .col(ColumnDef::new(StockAnalyses::BlackboardSnapshot).text())
                    .col(
                        ColumnDef::new(StockAnalyses::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StockAnalyses::UpdatedAt)
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
            .drop_table(Table::drop().table(StockAnalyses::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum StockAnalyses {
    Table,
    Id,
    StockCode,
    StockName,
    AnalysisDate,
    ProviderId,
    ConversationId,
    Status,
    DecisionAction,
    DecisionPositionPct,
    DecisionReasoning,
    DecisionJson,
    BlackboardSnapshot,
    CreatedAt,
    UpdatedAt,
}
