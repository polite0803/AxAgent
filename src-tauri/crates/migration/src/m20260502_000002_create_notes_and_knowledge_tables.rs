use sea_orm_migration::prelude::*;

/// 补齐 notes 表和 knowledge_* 五张知识图谱表的迁移
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260502_000002_create_notes_and_knowledge_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ── 1. notes ──
        manager
            .create_table(
                Table::create()
                    .table(Notes::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Notes::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Notes::VaultId).string().not_null())
                    .col(ColumnDef::new(Notes::Title).string().not_null())
                    .col(ColumnDef::new(Notes::FilePath).string().not_null())
                    .col(ColumnDef::new(Notes::Content).text().not_null())
                    .col(ColumnDef::new(Notes::ContentHash).string().not_null())
                    .col(ColumnDef::new(Notes::Author).string().not_null())
                    .col(ColumnDef::new(Notes::PageType).string().null())
                    .col(ColumnDef::new(Notes::SourceRefs).json().null())
                    .col(ColumnDef::new(Notes::RelatedPages).json().null())
                    .col(ColumnDef::new(Notes::QualityScore).double().null())
                    .col(ColumnDef::new(Notes::LastLintedAt).big_integer().null())
                    .col(ColumnDef::new(Notes::LastCompiledAt).big_integer().null())
                    .col(ColumnDef::new(Notes::CompiledSourceHash).string().null())
                    .col(ColumnDef::new(Notes::UserEdited).integer().not_null().default(0))
                    .col(ColumnDef::new(Notes::UserEditedAt).big_integer().null())
                    .col(ColumnDef::new(Notes::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(Notes::UpdatedAt).big_integer().not_null())
                    .col(ColumnDef::new(Notes::IsDeleted).integer().not_null().default(0))
                    .to_owned(),
            )
            .await?;

        // ── 2. knowledge_entities ──
        manager
            .create_table(
                Table::create()
                    .table(KnowledgeEntities::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(KnowledgeEntities::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(KnowledgeEntities::KnowledgeBaseId).string().not_null())
                    .col(ColumnDef::new(KnowledgeEntities::Name).string().not_null())
                    .col(ColumnDef::new(KnowledgeEntities::EntityType).string().not_null())
                    .col(ColumnDef::new(KnowledgeEntities::Description).string().null())
                    .col(ColumnDef::new(KnowledgeEntities::SourcePath).string().not_null())
                    .col(ColumnDef::new(KnowledgeEntities::SourceLanguage).string().null())
                    .col(ColumnDef::new(KnowledgeEntities::Properties).json().not_null())
                    .col(ColumnDef::new(KnowledgeEntities::Lifecycle).json().null())
                    .col(ColumnDef::new(KnowledgeEntities::Behaviors).json().null())
                    .col(ColumnDef::new(KnowledgeEntities::Metadata).json().null())
                    .col(ColumnDef::new(KnowledgeEntities::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(KnowledgeEntities::UpdatedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        // ── 3. knowledge_attributes ──
        manager
            .create_table(
                Table::create()
                    .table(KnowledgeAttributes::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(KnowledgeAttributes::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(KnowledgeAttributes::KnowledgeBaseId).string().not_null())
                    .col(ColumnDef::new(KnowledgeAttributes::EntityId).string().not_null())
                    .col(ColumnDef::new(KnowledgeAttributes::Name).string().not_null())
                    .col(ColumnDef::new(KnowledgeAttributes::AttributeType).string().not_null())
                    .col(ColumnDef::new(KnowledgeAttributes::DataType).string().not_null())
                    .col(ColumnDef::new(KnowledgeAttributes::Description).string().null())
                    .col(ColumnDef::new(KnowledgeAttributes::IsRequired).boolean().not_null().default(false))
                    .col(ColumnDef::new(KnowledgeAttributes::DefaultValue).string().null())
                    .col(ColumnDef::new(KnowledgeAttributes::Constraints).json().null())
                    .col(ColumnDef::new(KnowledgeAttributes::ValidationRules).json().null())
                    .col(ColumnDef::new(KnowledgeAttributes::Metadata).json().null())
                    .col(ColumnDef::new(KnowledgeAttributes::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(KnowledgeAttributes::UpdatedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        // ── 4. knowledge_relations ──
        manager
            .create_table(
                Table::create()
                    .table(KnowledgeRelations::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(KnowledgeRelations::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(KnowledgeRelations::KnowledgeBaseId).string().not_null())
                    .col(ColumnDef::new(KnowledgeRelations::SourceEntityId).string().not_null())
                    .col(ColumnDef::new(KnowledgeRelations::TargetEntityId).string().not_null())
                    .col(ColumnDef::new(KnowledgeRelations::RelationType).string().not_null())
                    .col(ColumnDef::new(KnowledgeRelations::Description).string().null())
                    .col(ColumnDef::new(KnowledgeRelations::Properties).json().null())
                    .col(ColumnDef::new(KnowledgeRelations::Metadata).json().null())
                    .col(ColumnDef::new(KnowledgeRelations::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(KnowledgeRelations::UpdatedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        // ── 5. knowledge_flows ──
        manager
            .create_table(
                Table::create()
                    .table(KnowledgeFlows::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(KnowledgeFlows::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(KnowledgeFlows::KnowledgeBaseId).string().not_null())
                    .col(ColumnDef::new(KnowledgeFlows::Name).string().not_null())
                    .col(ColumnDef::new(KnowledgeFlows::FlowType).string().not_null())
                    .col(ColumnDef::new(KnowledgeFlows::Description).string().null())
                    .col(ColumnDef::new(KnowledgeFlows::SourcePath).string().not_null())
                    .col(ColumnDef::new(KnowledgeFlows::Steps).json().not_null())
                    .col(ColumnDef::new(KnowledgeFlows::DecisionPoints).json().null())
                    .col(ColumnDef::new(KnowledgeFlows::ErrorHandling).json().null())
                    .col(ColumnDef::new(KnowledgeFlows::Preconditions).json().null())
                    .col(ColumnDef::new(KnowledgeFlows::Postconditions).json().null())
                    .col(ColumnDef::new(KnowledgeFlows::Metadata).json().null())
                    .col(ColumnDef::new(KnowledgeFlows::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(KnowledgeFlows::UpdatedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        // ── 6. knowledge_interfaces ──
        manager
            .create_table(
                Table::create()
                    .table(KnowledgeInterfaces::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(KnowledgeInterfaces::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(KnowledgeInterfaces::KnowledgeBaseId).string().not_null())
                    .col(ColumnDef::new(KnowledgeInterfaces::Name).string().not_null())
                    .col(ColumnDef::new(KnowledgeInterfaces::InterfaceType).string().not_null())
                    .col(ColumnDef::new(KnowledgeInterfaces::Description).string().null())
                    .col(ColumnDef::new(KnowledgeInterfaces::SourcePath).string().not_null())
                    .col(ColumnDef::new(KnowledgeInterfaces::InputSchema).json().not_null())
                    .col(ColumnDef::new(KnowledgeInterfaces::OutputSchema).json().not_null())
                    .col(ColumnDef::new(KnowledgeInterfaces::ErrorCodes).json().null())
                    .col(ColumnDef::new(KnowledgeInterfaces::CommunicationPattern).string().null())
                    .col(ColumnDef::new(KnowledgeInterfaces::Version).string().null())
                    .col(ColumnDef::new(KnowledgeInterfaces::Metadata).json().null())
                    .col(ColumnDef::new(KnowledgeInterfaces::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(KnowledgeInterfaces::UpdatedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(KnowledgeInterfaces::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(KnowledgeFlows::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(KnowledgeRelations::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(KnowledgeAttributes::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(KnowledgeEntities::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(Notes::Table).if_exists().to_owned()).await?;
        Ok(())
    }
}

#[derive(Iden)] enum Notes { Table, Id, VaultId, Title, FilePath, Content, ContentHash, Author, PageType, SourceRefs, RelatedPages, QualityScore, LastLintedAt, LastCompiledAt, CompiledSourceHash, UserEdited, UserEditedAt, CreatedAt, UpdatedAt, IsDeleted }
#[derive(Iden)] enum KnowledgeEntities { Table, Id, KnowledgeBaseId, Name, EntityType, Description, SourcePath, SourceLanguage, Properties, Lifecycle, Behaviors, Metadata, CreatedAt, UpdatedAt }
#[derive(Iden)] enum KnowledgeAttributes { Table, Id, KnowledgeBaseId, EntityId, Name, AttributeType, DataType, Description, IsRequired, DefaultValue, Constraints, ValidationRules, Metadata, CreatedAt, UpdatedAt }
#[derive(Iden)] enum KnowledgeRelations { Table, Id, KnowledgeBaseId, SourceEntityId, TargetEntityId, RelationType, Description, Properties, Metadata, CreatedAt, UpdatedAt }
#[derive(Iden)] enum KnowledgeFlows { Table, Id, KnowledgeBaseId, Name, FlowType, Description, SourcePath, Steps, DecisionPoints, ErrorHandling, Preconditions, Postconditions, Metadata, CreatedAt, UpdatedAt }
#[derive(Iden)] enum KnowledgeInterfaces { Table, Id, KnowledgeBaseId, Name, InterfaceType, Description, SourcePath, InputSchema, OutputSchema, ErrorCodes, CommunicationPattern, Version, Metadata, CreatedAt, UpdatedAt }
