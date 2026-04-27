use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(KnowledgeDocuments::Table)
                    .add_column(
                        ColumnDef::new(KnowledgeDocuments::SourceConversationId)
                            .string()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_knowledge_documents_source_conversation_id")
                    .table(KnowledgeDocuments::Table)
                    .col(KnowledgeDocuments::SourceConversationId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_knowledge_documents_source_conversation_id")
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(KnowledgeDocuments::Table)
                    .drop_column(KnowledgeDocuments::SourceConversationId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum KnowledgeDocuments {
    Table,
    SourceConversationId,
}
