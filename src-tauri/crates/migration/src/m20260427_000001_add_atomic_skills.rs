use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // atomic_skills table
        manager
            .create_table(
                Table::create()
                    .table(AtomicSkills::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AtomicSkills::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(AtomicSkills::Name).string().not_null().unique_key())
                    .col(
                        ColumnDef::new(AtomicSkills::Description)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(AtomicSkills::InputSchema).string().null())
                    .col(ColumnDef::new(AtomicSkills::OutputSchema).string().null())
                    .col(ColumnDef::new(AtomicSkills::EntryType).string().not_null())
                    .col(ColumnDef::new(AtomicSkills::EntryRef).string().not_null())
                    .col(
                        ColumnDef::new(AtomicSkills::Category)
                            .string()
                            .not_null()
                            .default("general"),
                    )
                    .col(ColumnDef::new(AtomicSkills::Tags).string().null())
                    .col(
                        ColumnDef::new(AtomicSkills::Version)
                            .string()
                            .not_null()
                            .default("1.0.0"),
                    )
                    .col(
                        ColumnDef::new(AtomicSkills::Enabled)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(AtomicSkills::Source)
                            .string()
                            .not_null()
                            .default("atomic"),
                    )
                    .col(ColumnDef::new(AtomicSkills::CreatedAt).integer().not_null())
                    .col(ColumnDef::new(AtomicSkills::UpdatedAt).integer().not_null())
                    .to_owned(),
            )
            .await?;

        // Semantic uniqueness index
        manager
            .create_index(
                Index::create()
                    .name("idx_atomic_skills_semantic")
                    .table(AtomicSkills::Table)
                    .col(AtomicSkills::EntryType)
                    .col(AtomicSkills::EntryRef)
                    .col(AtomicSkills::InputSchema)
                    .col(AtomicSkills::OutputSchema)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // skill_references table
        manager
            .create_table(
                Table::create()
                    .table(SkillReferences::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SkillReferences::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SkillReferences::SkillId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SkillReferences::WorkflowId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SkillReferences::NodeId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SkillReferences::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // skill_references indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_skill_references_skill")
                    .table(SkillReferences::Table)
                    .col(SkillReferences::SkillId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_skill_references_workflow")
                    .table(SkillReferences::Table)
                    .col(SkillReferences::WorkflowId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SkillReferences::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(AtomicSkills::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum AtomicSkills {
    Table,
    Id,
    Name,
    Description,
    InputSchema,
    OutputSchema,
    EntryType,
    EntryRef,
    Category,
    Tags,
    Version,
    Enabled,
    Source,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum SkillReferences {
    Table,
    Id,
    SkillId,
    WorkflowId,
    NodeId,
    CreatedAt,
}
