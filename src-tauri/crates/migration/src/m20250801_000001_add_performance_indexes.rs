//! Performance indexes migration
//! 
//! This migration adds performance-critical indexes to improve query speed.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Conversation indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_conversations_created_at")
                    .table(Conversations::Table)
                    .col(Conversations::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_conversations_is_archived")
                    .table(Conversations::Table)
                    .col(Conversations::IsArchived)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_conversations_category_id")
                    .table(Conversations::Table)
                    .col(Conversations::CategoryId)
                    .to_owned(),
            )
            .await?;

        // Message indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_messages_conversation_id")
                    .table(Messages::Table)
                    .col(Messages::ConversationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_messages_created_at")
                    .table(Messages::Table)
                    .col(Messages::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // Memory item indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_memory_items_namespace")
                    .table(MemoryItems::Table)
                    .col(MemoryItems::Namespace)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_memory_items_created_at")
                    .table(MemoryItems::Table)
                    .col(MemoryItems::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // Knowledge document indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_knowledge_documents_base_id")
                    .table(KnowledgeDocuments::Table)
                    .col(KnowledgeDocuments::BaseId)
                    .to_owned(),
            )
            .await?;

        // Provider indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_providers_is_enabled")
                    .table(Providers::Table)
                    .col(Providers::IsEnabled)
                    .to_owned(),
            )
            .await?;

        // MCP server indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_mcp_servers_is_enabled")
                    .table(McpServers::Table)
                    .col(McpServers::IsEnabled)
                    .to_owned(),
            )
            .await?;

        // Skill states indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_skill_states_enabled")
                    .table(SkillStates::Table)
                    .col(SkillStates::Enabled)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop all indexes in reverse order
        manager.drop_index(Index::drop().name("idx_skill_states_enabled").to_owned()).await?;
        manager.drop_index(Index::drop().name("idx_mcp_servers_is_enabled").to_owned()).await?;
        manager.drop_index(Index::drop().name("idx_providers_is_enabled").to_owned()).await?;
        manager.drop_index(Index::drop().name("idx_knowledge_documents_base_id").to_owned()).await?;
        manager.drop_index(Index::drop().name("idx_memory_items_created_at").to_owned()).await?;
        manager.drop_index(Index::drop().name("idx_memory_items_namespace").to_owned()).await?;
        manager.drop_index(Index::drop().name("idx_messages_created_at").to_owned()).await?;
        manager.drop_index(Index::drop().name("idx_messages_conversation_id").to_owned()).await?;
        manager.drop_index(Index::drop().name("idx_conversations_category_id").to_owned()).await?;
        manager.drop_index(Index::drop().name("idx_conversations_is_archived").to_owned()).await?;
        manager.drop_index(Index::drop().name("idx_conversations_created_at").to_owned()).await?;

        Ok(())
    }
}

// Table enums for index creation
#[derive(Iden)]
enum Conversations {
    Table,
    CreatedAt,
    IsArchived,
    CategoryId,
}

#[derive(Iden)]
enum Messages {
    Table,
    ConversationId,
    CreatedAt,
}

#[derive(Iden)]
enum MemoryItems {
    Table,
    Namespace,
    CreatedAt,
}

#[derive(Iden)]
enum KnowledgeDocuments {
    Table,
    BaseId,
}

#[derive(Iden)]
enum Providers {
    Table,
    IsEnabled,
}

#[derive(Iden)]
enum McpServers {
    Table,
    IsEnabled,
}

#[derive(Iden)]
enum SkillStates {
    Table,
    Enabled,
}
