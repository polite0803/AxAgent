use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add context_compression column to conversations
        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .add_column(
                        ColumnDef::new(Conversations::ContextCompression)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        // Create conversation_summaries table
        manager
            .create_table(
                Table::create()
                    .table(ConversationSummaries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ConversationSummaries::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::ConversationId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::SummaryText)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::CompressedUntilMessageId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::TokenCount)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::ModelUsed)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConversationSummaries::UpdatedAt)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                ConversationSummaries::Table,
                                ConversationSummaries::ConversationId,
                            )
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_conv_summaries_conv_id")
                    .table(ConversationSummaries::Table)
                    .col(ConversationSummaries::ConversationId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ConversationSummaries::Table).to_owned())
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .drop_column(Conversations::ContextCompression)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Conversations {
    Table,
    Id,
    ContextCompression,
}

#[derive(DeriveIden)]
enum ConversationSummaries {
    Table,
    Id,
    ConversationId,
    SummaryText,
    CompressedUntilMessageId,
    TokenCount,
    ModelUsed,
    CreatedAt,
    UpdatedAt,
}
