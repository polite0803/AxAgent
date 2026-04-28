use axagent_migration::MigratorTrait;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement,
};
use tracing::info;

use crate::error::Result;
use crate::repo::provider;
use crate::types::*;

pub struct DbHandle {
    pub conn: DatabaseConnection,
}

pub async fn create_pool(db_path: &str) -> Result<DbHandle> {
    let url = if db_path.starts_with("sqlite:") {
        format!("{}?mode=rwc", db_path)
    } else {
        format!("sqlite:{}?mode=rwc", db_path)
    };

    let mut opt = ConnectOptions::new(&url);
    opt.max_connections(8)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(15))
        .sqlx_logging(false);

    let conn = Database::connect(opt).await?;

    conn.execute(Statement::from_string(
        DbBackend::Sqlite,
        "PRAGMA journal_mode=WAL;",
    ))
    .await?;
    conn.execute(Statement::from_string(
        DbBackend::Sqlite,
        "PRAGMA foreign_keys=ON;",
    ))
    .await?;
    conn.execute(Statement::from_string(
        DbBackend::Sqlite,
        "PRAGMA busy_timeout=5000;",
    ))
    .await?;
    conn.execute(Statement::from_string(
        DbBackend::Sqlite,
        "PRAGMA synchronous=NORMAL;",
    ))
    .await?;
    conn.execute(Statement::from_string(
        DbBackend::Sqlite,
        "PRAGMA cache_size=-64000;",
    ))
    .await?;
    conn.execute(Statement::from_string(
        DbBackend::Sqlite,
        "PRAGMA temp_store=MEMORY;",
    ))
    .await?;

    // Run SeaORM migrations
    axagent_migration::Migrator::up(&conn, None).await?;

    info!("Database initialized at {}", db_path);
    Ok(DbHandle { conn })
}

pub struct BuiltinProvider {
    pub builtin_id: &'static str,
    pub name: &'static str,
    pub provider_type: ProviderType,
    pub api_host: &'static str,
    pub models: Vec<(
        &'static str,
        &'static str,
        Vec<ModelCapability>,
        Option<u32>,
    )>,
}

pub fn get_builtin_providers() -> Vec<BuiltinProvider> {
    use ModelCapability::*;

    vec![
        BuiltinProvider {
            builtin_id: "openai",
            name: "OpenAI",
            provider_type: ProviderType::OpenAI,
            api_host: "https://api.openai.com",
            models: vec![
                (
                    "gpt-4o",
                    "GPT-4o",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(128000),
                ),
                (
                    "gpt-4o-mini",
                    "GPT-4o Mini",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(128000),
                ),
                (
                    "o3-mini",
                    "o3-mini",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(200000),
                ),
                (
                    "gpt-4.1",
                    "GPT-4.1",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(1047576),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "openai_responses",
            name: "OpenAI Responses",
            provider_type: ProviderType::OpenAIResponses,
            api_host: "https://api.openai.com",
            models: vec![
                (
                    "gpt-4o",
                    "GPT-4o",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(128000),
                ),
                (
                    "gpt-4o-mini",
                    "GPT-4o Mini",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(128000),
                ),
                (
                    "o3-mini",
                    "o3-mini",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(200000),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "gemini",
            name: "Gemini",
            provider_type: ProviderType::Gemini,
            api_host: "https://generativelanguage.googleapis.com",
            models: vec![
                (
                    "gemini-2.5-flash",
                    "Gemini 2.5 Flash",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(1048576),
                ),
                (
                    "gemini-2.5-pro",
                    "Gemini 2.5 Pro",
                    vec![TextChat, Vision, FunctionCalling, Reasoning],
                    Some(1048576),
                ),
                (
                    "gemini-2.0-flash",
                    "Gemini 2.0 Flash",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(1048576),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "anthropic",
            name: "Claude",
            provider_type: ProviderType::Anthropic,
            api_host: "https://api.anthropic.com",
            models: vec![
                (
                    "claude-sonnet-4-20250514",
                    "Claude Sonnet 4",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(200000),
                ),
                (
                    "claude-3-5-haiku-20241022",
                    "Claude 3.5 Haiku",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(200000),
                ),
                (
                    "claude-opus-4-20250514",
                    "Claude Opus 4",
                    vec![TextChat, Vision, FunctionCalling, Reasoning],
                    Some(200000),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "deepseek",
            name: "DeepSeek",
            provider_type: ProviderType::OpenAI,
            api_host: "https://api.deepseek.com",
            models: vec![
                (
                    "deepseek-chat",
                    "DeepSeek Chat",
                    vec![TextChat, FunctionCalling],
                    Some(65536),
                ),
                (
                    "deepseek-reasoner",
                    "DeepSeek Reasoner",
                    vec![TextChat, Reasoning],
                    Some(65536),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "xai",
            name: "xAI",
            provider_type: ProviderType::OpenAI,
            api_host: "https://api.x.ai",
            models: vec![
                (
                    "grok-3",
                    "Grok 3",
                    vec![TextChat, FunctionCalling],
                    Some(131072),
                ),
                (
                    "grok-3-mini",
                    "Grok 3 Mini",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(131072),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "glm",
            name: "GLM",
            provider_type: ProviderType::OpenAI,
            api_host: "https://open.bigmodel.cn/api/paas",
            models: vec![
                (
                    "glm-4-plus",
                    "GLM-4 Plus",
                    vec![TextChat, FunctionCalling],
                    Some(128000),
                ),
                (
                    "glm-4-flash",
                    "GLM-4 Flash",
                    vec![TextChat, FunctionCalling],
                    Some(128000),
                ),
                (
                    "glm-4.7",
                    "GLM-4.7",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(128000),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "minimax",
            name: "MiniMax",
            provider_type: ProviderType::OpenAI,
            api_host: "https://api.minimaxi.com",
            models: vec![
                (
                    "MiniMax-M1",
                    "MiniMax-M1",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(1000000),
                ),
                (
                    "MiniMax-S1",
                    "MiniMax-S1",
                    vec![TextChat, FunctionCalling],
                    Some(1000000),
                ),
                (
                    "minimaxai/minimax-m2.7",
                    "MiniMax-M2.7",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(1000000),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "nvidia",
            name: "NVIDIA",
            provider_type: ProviderType::OpenAI,
            api_host: "https://integrate.api.nvidia.com/v1",
            models: vec![
                (
                    "meta/llama-3.1-405b-instruct",
                    "Llama 3.1 405B",
                    vec![TextChat, FunctionCalling],
                    Some(128000),
                ),
                (
                    "meta/llama-3.1-70b-instruct",
                    "Llama 3.1 70B",
                    vec![TextChat, FunctionCalling],
                    Some(128000),
                ),
                (
                    "nvidia/llama-3.1-nemotron-70b-instruct",
                    "Llama 3.1 Nemotron 70B",
                    vec![TextChat, FunctionCalling],
                    Some(128000),
                ),
                (
                    "nvidia/llama-3.3-nemotron-super-49b-v1",
                    "Llama 3.3 Nemotron Super 49B",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(128000),
                ),
                (
                    "minimaxai/minimax-m2.7",
                    "MiniMax-M2.7",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(1000000),
                ),
                (
                    "zhipuai/glm-4.7",
                    "GLM-4.7",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(128000),
                ),
            ],
        },
    ]
}

#[allow(dead_code)]
async fn seed_builtin_providers(db: &DatabaseConnection) -> Result<()> {
    let existing = provider::list_providers(db).await?;
    if !existing.is_empty() {
        return Ok(());
    }

    info!("Seeding built-in providers...");

    let builtins = get_builtin_providers();

    for (idx, bp) in builtins.into_iter().enumerate() {
        let prov = provider::create_provider(
            db,
            CreateProviderInput {
                name: bp.name.to_string(),
                provider_type: bp.provider_type,
                api_host: bp.api_host.to_string(),
                api_path: None,
                enabled: true,
                builtin_id: None,
            },
        )
        .await?;

        let models: Vec<Model> = bp
            .models
            .into_iter()
            .map(|(model_id, name, caps, max_tokens)| Model {
                provider_id: prov.id.clone(),
                model_id: model_id.to_string(),
                name: name.to_string(),
                group_name: None,
                model_type: ModelType::detect(model_id),
                capabilities: caps,
                max_tokens,
                enabled: true,
                param_overrides: None,
            })
            .collect();

        provider::save_models(db, &prov.id, &models).await?;

        // Set sort order based on insertion index
        provider::update_provider(
            db,
            &prov.id,
            UpdateProviderInput {
                sort_order: Some(idx as i32),
                ..Default::default()
            },
        )
        .await?;
    }

    info!("Seeded {} built-in providers", 9);
    Ok(())
}

pub async fn create_test_pool() -> Result<DbHandle> {
    create_pool("sqlite::memory:").await
}
