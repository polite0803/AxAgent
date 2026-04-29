use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Wikis {
    Table,
    Id,
    Name,
    RootPath,
    SchemaVersion,
    Description,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum WikiSources {
    Table,
    Id,
    WikiId,
    SourceType,
    SourcePath,
    Title,
    MimeType,
    SizeBytes,
    ContentHash,
    MetadataJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum WikiPages {
    Table,
    Id,
    WikiId,
    NoteId,
    PageType,
    Title,
    SourceIds,
    QualityScore,
    LastLintedAt,
    LastCompiledAt,
    CompiledSourceHash,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum WikiOperations {
    Table,
    Id,
    WikiId,
    OperationType,
    TargetType,
    TargetId,
    Status,
    DetailsJson,
    ErrorMessage,
    CreatedAt,
    CompletedAt,
}

#[derive(Iden)]
enum WikiSyncQueue {
    Table,
    Id,
    WikiId,
    EventType,
    TargetType,
    TargetId,
    Payload,
    Status,
    RetryCount,
    ErrorMessage,
    CreatedAt,
    ProcessedAt,
}

#[derive(Iden)]
enum NoteLinks {
    Table,
    Id,
    VaultId,
    SourceNoteId,
    TargetNoteId,
    LinkText,
    LinkType,
    CreatedAt,
}

#[derive(Iden)]
enum NoteBacklinks {
    Table,
    Id,
    VaultId,
    SourceNoteId,
    TargetNoteId,
    LinkText,
    LinkType,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Wikis::Table)
                    .col(ColumnDef::new(Wikis::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Wikis::Name).string().not_null())
                    .col(ColumnDef::new(Wikis::RootPath).string().not_null())
                    .col(ColumnDef::new(Wikis::SchemaVersion).string().not_null().default("1.0"))
                    .col(ColumnDef::new(Wikis::Description).string().null())
                    .col(ColumnDef::new(Wikis::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(Wikis::UpdatedAt).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WikiSources::Table)
                    .col(ColumnDef::new(WikiSources::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(WikiSources::WikiId).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(WikiSources::Table, WikiSources::WikiId)
                            .to(Wikis::Table, Wikis::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(WikiSources::SourceType).string().not_null())
                    .col(ColumnDef::new(WikiSources::SourcePath).string().not_null())
                    .col(ColumnDef::new(WikiSources::Title).string().not_null())
                    .col(ColumnDef::new(WikiSources::MimeType).string().not_null())
                    .col(ColumnDef::new(WikiSources::SizeBytes).big_integer().not_null())
                    .col(ColumnDef::new(WikiSources::ContentHash).string().not_null())
                    .col(ColumnDef::new(WikiSources::MetadataJson).json().null())
                    .col(ColumnDef::new(WikiSources::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(WikiSources::UpdatedAt).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WikiPages::Table)
                    .col(ColumnDef::new(WikiPages::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(WikiPages::WikiId).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(WikiPages::Table, WikiPages::WikiId)
                            .to(Wikis::Table, Wikis::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(WikiPages::NoteId).string().not_null())
                    .col(ColumnDef::new(WikiPages::PageType).string().not_null())
                    .col(ColumnDef::new(WikiPages::Title).string().not_null())
                    .col(ColumnDef::new(WikiPages::SourceIds).json().null())
                    .col(ColumnDef::new(WikiPages::QualityScore).decimal().null())
                    .col(ColumnDef::new(WikiPages::LastLintedAt).integer().null())
                    .col(ColumnDef::new(WikiPages::LastCompiledAt).integer().null())
                    .col(
                        ColumnDef::new(WikiPages::CompiledSourceHash)
                            .string()
                            .null(),
                    )
                    .col(ColumnDef::new(WikiPages::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(WikiPages::UpdatedAt).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WikiOperations::Table)
                    .col(ColumnDef::new(WikiOperations::Id).big_integer().not_null().primary_key().auto_increment())
                    .col(ColumnDef::new(WikiOperations::WikiId).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(WikiOperations::Table, WikiOperations::WikiId)
                            .to(Wikis::Table, Wikis::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(WikiOperations::OperationType)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(WikiOperations::TargetType).string().not_null())
                    .col(ColumnDef::new(WikiOperations::TargetId).string().not_null())
                    .col(ColumnDef::new(WikiOperations::Status).string().not_null())
                    .col(ColumnDef::new(WikiOperations::DetailsJson).json().null())
                    .col(ColumnDef::new(WikiOperations::ErrorMessage).text().null())
                    .col(ColumnDef::new(WikiOperations::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(WikiOperations::CompletedAt).integer().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WikiSyncQueue::Table)
                    .col(ColumnDef::new(WikiSyncQueue::Id).big_integer().not_null().primary_key().auto_increment())
                    .col(ColumnDef::new(WikiSyncQueue::WikiId).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(WikiSyncQueue::Table, WikiSyncQueue::WikiId)
                            .to(Wikis::Table, Wikis::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(WikiSyncQueue::EventType).string().not_null())
                    .col(ColumnDef::new(WikiSyncQueue::TargetType).string().not_null())
                    .col(ColumnDef::new(WikiSyncQueue::TargetId).string().not_null())
                    .col(ColumnDef::new(WikiSyncQueue::Payload).json().null())
                    .col(ColumnDef::new(WikiSyncQueue::Status).string().not_null().default("pending"))
                    .col(
                        ColumnDef::new(WikiSyncQueue::RetryCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(WikiSyncQueue::ErrorMessage).text().null())
                    .col(ColumnDef::new(WikiSyncQueue::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(WikiSyncQueue::ProcessedAt).integer().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(NoteLinks::Table)
                    .col(ColumnDef::new(NoteLinks::Id).integer().not_null().primary_key())
                    .col(ColumnDef::new(NoteLinks::VaultId).string().not_null())
                    .col(ColumnDef::new(NoteLinks::SourceNoteId).string().not_null())
                    .col(ColumnDef::new(NoteLinks::TargetNoteId).string().not_null())
                    .col(ColumnDef::new(NoteLinks::LinkText).string().null())
                    .col(ColumnDef::new(NoteLinks::LinkType).string().not_null())
                    .col(ColumnDef::new(NoteLinks::CreatedAt).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(NoteBacklinks::Table)
                    .col(ColumnDef::new(NoteBacklinks::Id).integer().not_null().primary_key())
                    .col(ColumnDef::new(NoteBacklinks::VaultId).string().not_null())
                    .col(ColumnDef::new(NoteBacklinks::SourceNoteId).string().not_null())
                    .col(ColumnDef::new(NoteBacklinks::TargetNoteId).string().not_null())
                    .col(ColumnDef::new(NoteBacklinks::LinkText).string().null())
                    .col(ColumnDef::new(NoteBacklinks::LinkType).string().not_null())
                    .col(ColumnDef::new(NoteBacklinks::CreatedAt).integer().not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WikiSyncQueue::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WikiOperations::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WikiPages::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WikiSources::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Wikis::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(NoteLinks::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(NoteBacklinks::Table).to_owned())
            .await?;
        Ok(())
    }
}