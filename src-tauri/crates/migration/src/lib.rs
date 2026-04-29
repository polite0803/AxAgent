pub use sea_orm_migration::prelude::*;

mod m20240101_000001_init;
mod m20260429_000001_add_wiki_tables;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_init::Migration),
            Box::new(m20260429_000001_add_wiki_tables::Migration),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm_migration::sea_orm::{ConnectOptions, Database, DatabaseConnection};

    async fn sqlite_test_db() -> DatabaseConnection {
        let mut opts = ConnectOptions::new("sqlite::memory:");
        opts.max_connections(1)
            .min_connections(1)
            .sqlx_logging(false);
        Database::connect(opts)
            .await
            .expect("connect sqlite test db")
    }

    #[tokio::test]
    async fn migrator_up_adds_category_default_template_columns_on_sqlite() {
        let db = sqlite_test_db().await;

        Migrator::up(&db, None)
            .await
            .expect("run sqlite migrations");

        let manager = SchemaManager::new(&db);
        for column in [
            "default_provider_id",
            "default_model_id",
            "default_temperature",
            "default_max_tokens",
            "default_top_p",
            "default_frequency_penalty",
        ] {
            assert!(
                manager
                    .has_column("conversation_categories", column)
                    .await
                    .expect("check migrated column"),
                "missing column {column}"
            );
        }
    }

    #[tokio::test]
    async fn migrator_refresh_round_trips_latest_sqlite_schema() {
        let db = sqlite_test_db().await;

        Migrator::up(&db, None)
            .await
            .expect("run sqlite migrations");
        Migrator::refresh(&db)
            .await
            .expect("refresh sqlite migrations");
    }
}
