// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait, Database, DbBackend,
    EntityTrait, QueryFilter, Set, Statement,
};
use tracing::info;

use crate::repo::provider;
use axagent_entities::providers;
use axagent_harness::core_error::Result;
use axagent_harness::types::*;
use axagent_harness::util_fns::now_ts;

// 再导出 sea-orm 的连接类型，使 axagent-harness 可以基于此定义 Persistence trait，
// 消费者（agent/tools/runtime）只需 `use axagent_harness::DatabaseConnection`，
// 无需在自己的 Cargo.toml 中直接依赖 sea-orm。
pub use sea_orm::DatabaseConnection;

pub struct DbHandle {
    pub conn: DatabaseConnection,
    pub path: String,
}

impl axagent_harness::Persistence for DbHandle {
    fn connection(&self) -> &axagent_harness::DatabaseConnection {
        &self.conn
    }

    fn db_path(&self) -> &str {
        &self.path
    }
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

    conn.execute_raw(Statement::from_string(DbBackend::Sqlite, "PRAGMA journal_mode=WAL;"))
        .await?;
    conn.execute_raw(Statement::from_string(DbBackend::Sqlite, "PRAGMA foreign_keys=ON;"))
        .await?;
    conn.execute_raw(Statement::from_string(DbBackend::Sqlite, "PRAGMA busy_timeout=5000;"))
        .await?;
    conn.execute_raw(Statement::from_string(DbBackend::Sqlite, "PRAGMA synchronous=NORMAL;"))
        .await?;
    conn.execute_raw(Statement::from_string(DbBackend::Sqlite, "PRAGMA cache_size=-64000;"))
        .await?;
    conn.execute_raw(Statement::from_string(DbBackend::Sqlite, "PRAGMA temp_store=MEMORY;"))
        .await?;

    // Run schema initialization
    crate::ddl::run_initialization(&conn).await?;

    // Seed built-in providers
    seed_builtin_providers(&conn).await?;

    // 数据迁移：硬编码路径 → 模板变量
    // (注：path_vars 迁移在 init/database.rs 中由调用方负责)
    crate::repo::local_tool::migrate_legacy_keys(&conn).await;

    // 注意：预设模板不再在启动时自动播种。
    // 工作流模板按需导入，通过前端工作流管理页面的"从预设导入"按钮触发 seed_preset_templates Tauri 命令。

    info!("Database initialized at {}", db_path);
    Ok(DbHandle {
        conn,
        path: db_path.to_string(),
    })
}

pub fn default_db_path() -> String {
    #[cfg(mobile)]
    let home = dirs::data_dir()
        .or_else(dirs::home_dir)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .or_else(|| std::env::var("ANDROID_DATA").ok())
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| {
            tracing::warn!("Could not determine home directory for DB path, using current dir");
            PathBuf::from(".")
        });
    #[cfg(not(mobile))]
    let home = dirs::home_dir().unwrap_or_else(|| {
        tracing::warn!("Could not determine home directory for DB path, using current dir");
        PathBuf::from(".")
    });

    let path = home.join(".axagent").join("data").join("axagent.db");
    path.to_string_lossy().to_string()
}

pub fn profile_db_path(profile_name: &str) -> String {
    #[cfg(mobile)]
    let home = dirs::data_dir()
        .or_else(dirs::home_dir)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .or_else(|| std::env::var("ANDROID_DATA").ok())
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| {
            tracing::warn!("Could not determine home directory for DB path, using current dir");
            PathBuf::from(".")
        });
    #[cfg(not(mobile))]
    let home = dirs::home_dir().unwrap_or_else(|| {
        tracing::warn!("Could not determine home directory for DB path, using current dir");
        PathBuf::from(".")
    });

    let path = home
        .join(".axagent")
        .join("profiles")
        .join(profile_name)
        .join("data")
        .join("axagent.db");
    path.to_string_lossy().to_string()
}

pub async fn create_pool_for_profile(profile_name: &str) -> Result<DbHandle> {
    let db_path = if profile_name == "default" {
        default_db_path()
    } else {
        profile_db_path(profile_name)
    };
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    create_pool(&db_path).await
}

pub struct BuiltinProvider {
    pub builtin_id: &'static str,
    pub name: &'static str,
    pub provider_type: ProviderType,
    pub api_host: &'static str,
    pub models: Vec<(&'static str, &'static str, Vec<ModelCapability>, Option<u32>)>,
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
                    "gpt-5.5",
                    "GPT-5.5",
                    vec![TextChat, Vision, FunctionCalling, Reasoning],
                    Some(1048576),
                ),
                ("gpt-5.4", "GPT-5.4", vec![TextChat, Vision, FunctionCalling], Some(1048576)),
                (
                    "gpt-5.4-mini",
                    "GPT-5.4 Mini",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(1048576),
                ),
                ("o4-mini", "o4-mini", vec![TextChat, Reasoning, FunctionCalling], Some(200000)),
            ],
        },
        BuiltinProvider {
            builtin_id: "openai_responses",
            name: "OpenAI Responses",
            provider_type: ProviderType::OpenAIResponses,
            api_host: "https://api.openai.com",
            models: vec![
                (
                    "gpt-5.5",
                    "GPT-5.5",
                    vec![TextChat, Vision, FunctionCalling, Reasoning],
                    Some(1048576),
                ),
                ("gpt-5.4", "GPT-5.4", vec![TextChat, Vision, FunctionCalling], Some(1048576)),
                (
                    "gpt-5.4-mini",
                    "GPT-5.4 Mini",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(1048576),
                ),
                ("o4-mini", "o4-mini", vec![TextChat, Reasoning, FunctionCalling], Some(200000)),
            ],
        },
        BuiltinProvider {
            builtin_id: "gemini",
            name: "Gemini",
            provider_type: ProviderType::Gemini,
            api_host: "https://generativelanguage.googleapis.com",
            models: vec![
                (
                    "gemini-3.5-flash",
                    "Gemini 3.5 Flash",
                    vec![TextChat, Vision, FunctionCalling, Reasoning],
                    Some(1048576),
                ),
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
            ],
        },
        BuiltinProvider {
            builtin_id: "anthropic",
            name: "Claude",
            provider_type: ProviderType::Anthropic,
            api_host: "https://api.anthropic.com",
            models: vec![
                (
                    "claude-sonnet-4-6",
                    "Claude Sonnet 4.6",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(200000),
                ),
                (
                    "claude-haiku-4-5",
                    "Claude Haiku 4.5",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(200000),
                ),
                (
                    "claude-opus-4-8",
                    "Claude Opus 4.8",
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
                    "deepseek-v4-flash",
                    "DeepSeek V4 Flash",
                    vec![TextChat, FunctionCalling],
                    Some(1048576),
                ),
                (
                    "deepseek-v4-pro",
                    "DeepSeek V4 Pro",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(1048576),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "qwen",
            name: "通义千问",
            provider_type: ProviderType::OpenAI,
            api_host: "https://dashscope.aliyuncs.com/compatible-mode/v1",
            models: vec![
                (
                    "qwen3.7-max",
                    "Qwen3.7 Max",
                    vec![TextChat, Vision, FunctionCalling, Reasoning],
                    Some(1048576),
                ),
                (
                    "qwen3.6-plus",
                    "Qwen3.6 Plus",
                    vec![TextChat, Vision, FunctionCalling, Reasoning],
                    Some(1048576),
                ),
                (
                    "qwen3.6-flash",
                    "Qwen3.6 Flash",
                    vec![TextChat, Vision, FunctionCalling, Reasoning],
                    Some(1048576),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "kimi",
            name: "Kimi",
            provider_type: ProviderType::OpenAI,
            api_host: "https://api.moonshot.cn/v1",
            models: vec![
                (
                    "kimi-k2.6",
                    "Kimi K2.6",
                    vec![TextChat, Vision, FunctionCalling, Reasoning],
                    Some(262144),
                ),
                (
                    "kimi-k2.5",
                    "Kimi K2.5",
                    vec![TextChat, Vision, FunctionCalling, Reasoning],
                    Some(262144),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "doubao",
            name: "豆包",
            provider_type: ProviderType::OpenAI,
            api_host: "https://ark.cn-beijing.volces.com/api/v3",
            models: vec![
                (
                    "doubao-1.5-pro-256k",
                    "Doubao 1.5 Pro 256K",
                    vec![TextChat, Vision, FunctionCalling],
                    Some(262144),
                ),
                (
                    "doubao-1.5-lite-32k",
                    "Doubao 1.5 Lite 32K",
                    vec![TextChat, FunctionCalling],
                    Some(32768),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "siliconflow",
            name: "硅基流动",
            provider_type: ProviderType::OpenAI,
            api_host: "https://api.siliconflow.cn/v1",
            models: vec![
                (
                    "Pro/deepseek-ai/DeepSeek-R1",
                    "DeepSeek R1 (Pro)",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(65536),
                ),
                (
                    "Pro/deepseek-ai/DeepSeek-V3",
                    "DeepSeek V3 (Pro)",
                    vec![TextChat, FunctionCalling],
                    Some(65536),
                ),
                (
                    "Qwen/Qwen3-235B-A22B",
                    "Qwen3 235B",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(262144),
                ),
                (
                    "Qwen/Qwen3-32B",
                    "Qwen3 32B",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(262144),
                ),
            ],
        },
        BuiltinProvider {
            builtin_id: "glm",
            name: "GLM",
            provider_type: ProviderType::OpenAI,
            api_host: "https://open.bigmodel.cn/api/paas/v4",
            models: vec![
                ("glm-5", "GLM-5", vec![TextChat, Reasoning, FunctionCalling], Some(128000)),
                ("glm-4-plus", "GLM-4 Plus", vec![TextChat, FunctionCalling], Some(128000)),
                ("glm-4-flash", "GLM-4 Flash", vec![TextChat, FunctionCalling], Some(128000)),
            ],
        },
        BuiltinProvider {
            builtin_id: "minimax",
            name: "MiniMax",
            provider_type: ProviderType::OpenAI,
            api_host: "https://api.minimax.io",
            models: vec![
                (
                    "MiniMax-M3",
                    "MiniMax-M3",
                    vec![TextChat, Reasoning, FunctionCalling],
                    Some(1000000),
                ),
                ("MiniMax-S1", "MiniMax-S1", vec![TextChat, FunctionCalling], Some(1000000)),
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

async fn seed_builtin_providers(db: &DatabaseConnection) -> Result<()> {
    info!("Seeding built-in providers...");

    let builtins = get_builtin_providers();

    for (idx, bp) in builtins.into_iter().enumerate() {
        // Check if provider with this builtin_id already exists
        let existing = providers::Entity::find()
            .filter(providers::Column::BuiltinId.eq(bp.builtin_id))
            .one(db)
            .await?;

        if let Some(existing_prov) = existing {
            // Update api_host for existing built-in providers if it has a known-broken value
            let old_hosts: &[(&str, &str)] = &[
                ("https://api.minimaxi.com", "https://api.minimax.io"),
                ("https://open.bigmodel.cn/api/paas", "https://open.bigmodel.cn/api/paas/v4"),
            ];
            for (old_host, new_host) in old_hosts {
                if existing_prov.api_host == *old_host {
                    let mut active: providers::ActiveModel = existing_prov.into();
                    active.api_host = Set(new_host.to_string());
                    active.updated_at = Set(now_ts());
                    active.update(db).await?;
                    info!(
                        "Updated api_host for builtin provider '{}': {} -> {}",
                        bp.builtin_id, old_host, new_host
                    );
                    break;
                }
            }
            continue;
        }

        let prov = provider::create_provider(
            db,
            CreateProviderInput {
                name: bp.name.to_string(),
                provider_type: bp.provider_type,
                api_host: bp.api_host.to_string(),
                api_path: None,
                enabled: true,
                builtin_id: Some(bp.builtin_id.to_string()),
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
                max_output_tokens: None,
                enabled: true,
                param_overrides: None,
                input_price_per_mtok: None,
                output_price_per_mtok: None,
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

    info!("Seeded built-in providers");
    Ok(())
}

pub async fn create_test_pool() -> Result<DbHandle> {
    let mut opt = ConnectOptions::new("sqlite::memory:?mode=rwc");
    opt.max_connections(1)
        .min_connections(1)
        .sqlx_logging(false);
    let conn = Database::connect(opt).await?;
    conn.execute_raw(Statement::from_string(DbBackend::Sqlite, "PRAGMA foreign_keys=ON;"))
        .await?;
    crate::ddl::run_initialization(&conn).await?;
    Ok(DbHandle {
        conn,
        path: ":memory:".to_string(),
    })
}
