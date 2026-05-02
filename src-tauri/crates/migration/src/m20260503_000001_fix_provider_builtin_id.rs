use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let updates: &[(&str, &str)] = &[
            ("OpenAI", "openai"),
            ("OpenAI Responses", "openai_responses"),
            ("Gemini", "gemini"),
            ("Claude", "anthropic"),
            ("DeepSeek", "deepseek"),
            ("xAI", "xai"),
            ("GLM", "glm"),
            ("MiniMax", "minimax"),
            ("NVIDIA", "nvidia"),
        ];

        for (name, builtin_id) in updates {
            db.execute_unprepared(&format!(
                "UPDATE providers SET builtin_id = '{}' WHERE name = '{}' AND builtin_id IS NULL",
                builtin_id, name
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
