use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RlPolicies::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RlPolicies::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RlPolicies::Name).string().not_null())
                    .col(ColumnDef::new(RlPolicies::PolicyType).string().not_null())
                    .col(ColumnDef::new(RlPolicies::ModelId).string().not_null())
                    .col(
                        ColumnDef::new(RlPolicies::RewardSignalsJson)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(RlPolicies::ExperiencesJson)
                            .string()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(RlPolicies::TotalExperiences)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(RlPolicies::EpisodesCompleted)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(RlPolicies::AvgReward)
                            .float()
                            .not_null()
                            .default(0.0),
                    )
                    .col(ColumnDef::new(RlPolicies::LastUpdate).string().not_null())
                    .col(ColumnDef::new(RlPolicies::CreatedAt).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_rl_policies_type")
                    .table(RlPolicies::Table)
                    .col(RlPolicies::PolicyType)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RlPolicies::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum RlPolicies {
    Table,
    Id,
    Name,
    PolicyType,
    ModelId,
    RewardSignalsJson,
    ExperiencesJson,
    TotalExperiences,
    EpisodesCompleted,
    AvgReward,
    LastUpdate,
    CreatedAt,
}
