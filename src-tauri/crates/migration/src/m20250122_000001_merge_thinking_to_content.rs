use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Merge non-empty `thinking` column into `content` as <think> blocks,
        // then clear the `thinking` column.
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"UPDATE messages
               SET content = '<think>' || char(10) || thinking || char(10) || '</think>' || char(10, 10) || content,
                   thinking = NULL
               WHERE thinking IS NOT NULL AND thinking != ''"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Not reversible — thinking content is now embedded in content
        Ok(())
    }
}
