use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryNamespaces::Table)
                    .add_column(ColumnDef::new(MemoryNamespaces::IconType).string().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MemoryNamespaces::Table)
                    .add_column(ColumnDef::new(MemoryNamespaces::IconValue).string().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MemoryNamespaces::Table)
                    .add_column(
                        ColumnDef::new(MemoryNamespaces::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[derive(DeriveIden)]
enum MemoryNamespaces {
    Table,
    IconType,
    IconValue,
    SortOrder,
}
