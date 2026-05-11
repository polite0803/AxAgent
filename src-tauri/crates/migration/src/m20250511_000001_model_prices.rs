use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250511_000001_model_prices"
    }
}

#[derive(DeriveIden)]
enum Models {
    Table,
    InputPricePerMtok,
    OutputPricePerMtok,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Models::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Models::InputPricePerMtok)
                            .double()
                            .default(Value::Double(None)),
                    )
                    .add_column_if_not_exists(
                        ColumnDef::new(Models::OutputPricePerMtok)
                            .double()
                            .default(Value::Double(None)),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Models::Table)
                    .drop_column(Models::InputPricePerMtok)
                    .drop_column(Models::OutputPricePerMtok)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
