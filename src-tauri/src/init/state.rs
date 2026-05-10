use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock as TokioRwLock;

use super::database::DatabaseInitResult;
use crate::commands::proactive::ProactiveService;
use crate::semantic_cache::{CacheConfig, SemanticCache};
use crate::AppState;
#[cfg(mobile)]
use axagent_core::cloud_storage::CloudStorageConfig;
use axagent_core::cloud_storage::SyncEngine;

pub fn create_app_state(db_result: DatabaseInitResult) -> AppState {
    let DatabaseInitResult {
        db_handle,
        master_key,
        db_path,
        app_dir,
        ..
    } = db_result;

    let sea_db = db_handle.conn.clone();

    let vector_store = axagent_core::vector_store::VectorStore::new(sea_db.clone());
    let vector_store_arc = Arc::new(vector_store);

    {
        let db_conn = sea_db.clone();
        let mk = master_key;
        let vs = vector_store_arc.clone();
        axagent_tools::builtin_handlers::set_knowledge_search_callback(std::sync::Arc::new(
            move |base_id: &str, query: &str, top_k: usize| {
                let db = db_conn.clone();
                let vs2 = vs.clone();
                let bid = base_id.to_string();
                let q = query.to_string();
                Box::pin(async move {
                    let results =
                        crate::indexing::search_knowledge(&db, &mk, &vs2, &bid, &q, top_k).await?;
                    Ok(results
                        .into_iter()
                        .map(|r| axagent_tools::builtin_handlers::KnowledgeSearchHit {
                            document_id: r.document_id,
                            chunk_index: r.chunk_index,
                            content: r.content,
                            score: r.score,
                        })
                        .collect())
                })
            },
        ));
    }

    let rt = tokio::runtime::Runtime::new().expect("Failed to create init runtime");
    let _ = rt.block_on(axagent_core::repo::mcp_server::ensure_preset_servers(&sea_db));
    rt.block_on(axagent_core::path_vars::migrate_hardcoded_paths(&sea_db));
    rt.block_on(axagent_core::repo::local_tool::migrate_legacy_keys(&sea_db));

    let app_settings = rt
        .block_on(axagent_core::repo::settings::get_settings(&sea_db))
        .unwrap_or_default();

    axagent_core::storage_paths::init_documents_root(
        app_settings
            .documents_root_override
            .as_ref()
            .map(PathBuf::from),
    );
    axagent_core::storage_paths::ensure_documents_dirs().unwrap_or_else(|e| {
        tracing::warn!("Failed to create documents storage dirs (non-critical on mobile): {}", e);
    });

    let shared_trajectory_storage: Arc<axagent_trajectory::TrajectoryStorage> = {
        let db_file_path = db_path.strip_prefix("sqlite:").unwrap_or(&db_path);
        let storage = axagent_trajectory::TrajectoryStorage::with_fts_path(
            Arc::new(sea_db.clone()),
            db_file_path,
        )
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to init trajectory FTS5, falling back to no-FTS: {}", e);
            axagent_trajectory::TrajectoryStorage::new(Arc::new(sea_db.clone()))
        });
        Arc::new(storage)
    };

    let memory_service = {
        let ms = axagent_trajectory::MemoryService::new(shared_trajectory_storage.clone())
            .unwrap_or_else(|e| {
                tracing::error!("Failed to create MemoryService: {}", e);
                panic!("MemoryService is required for application startup: {}", e);
            });
        if let Err(e) = ms.initialize() {
            tracing::warn!("Failed to initialize MemoryService: {}", e);
        }
        Arc::new(TokioRwLock::new(ms))
    };

    let platform_manager =
        Arc::new(axagent_runtime::message_gateway::platform_manager::PlatformManager::new());

    let platform_bridge =
        Arc::new(axagent_runtime::message_gateway::platform_bridge::PlatformBridge::new(
            sea_db.clone(),
            master_key,
            platform_manager.clone(),
        ));

    rt.block_on(platform_manager.set_message_callback(platform_bridge.clone()));

    let sync_engine = create_sync_engine(&sea_db, &app_settings);

    AppState {
        sea_db: sea_db.clone(),
        master_key,
        gateway: Arc::new(Mutex::new(None)),
        close_to_tray: Arc::new(AtomicBool::new(false)),
        app_data_dir: app_dir.clone(),
        db_path,
        auto_backup_handle: Arc::new(Mutex::new(None)),
        webdav_sync_handle: Arc::new(Mutex::new(None)),
        api_server_handle: Arc::new(Mutex::new(None)),
        vector_store: vector_store_arc,
        indexing_semaphore: Arc::new(tokio::sync::Semaphore::new(2)),
        stream_cancel_flags: Arc::new(Mutex::new(std::collections::HashMap::new())),
        agent_permission_senders: Arc::new(Mutex::new(std::collections::HashMap::new())),
        agent_ask_senders: Arc::new(Mutex::new(std::collections::HashMap::new())),
        agent_always_allowed: Arc::new(Mutex::new(std::collections::HashMap::new())),
        agent_prompters: Arc::new(Mutex::new(std::collections::HashMap::new())),
        agent_session_manager: Arc::new(axagent_agent::SessionManager::new(sea_db.clone())),
        agent_cancel_tokens: Arc::new(Mutex::new(std::collections::HashMap::new())),
        agent_paused: Arc::new(Mutex::new(std::collections::HashSet::new())),
        running_agents: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
        workflow_engine: Arc::new(axagent_runtime::workflow_engine::WorkflowEngine::new()),
        shared_memory: Arc::new(TokioRwLock::new(
            axagent_runtime::shared_memory::SharedMemory::new(),
        )),
        sub_agent_registry: Arc::new(TokioRwLock::new(
            axagent_trajectory::SubAgentRegistry::new().unwrap_or_default(),
        )),
        memory_service: memory_service.clone(),
        nudge_service: Arc::new(tokio::sync::Mutex::new(axagent_trajectory::NudgeService::new())),
        trajectory_storage: shared_trajectory_storage.clone(),
        closed_loop_service: Arc::new(axagent_trajectory::ClosedLoopService::new(
            shared_trajectory_storage.clone(),
        )),
        insight_system: Arc::new(TokioRwLock::new(
            axagent_trajectory::LearningInsightSystem::new().with_storage_limits(200, 30),
        )),
        realtime_learning: Arc::new(tokio::sync::Mutex::new(
            axagent_trajectory::RealTimeLearning::new(),
        )),
        pattern_learner: Arc::new(TokioRwLock::new(axagent_trajectory::PatternLearner::new(
            axagent_trajectory::PatternConfig::default(),
        ))),
        cross_session_learner: Arc::new(TokioRwLock::new(
            axagent_trajectory::CrossSessionLearner::new(),
        )),
        rl_engine: Arc::new(TokioRwLock::new(axagent_trajectory::RLEngine::new(
            axagent_trajectory::RLConfig::default(),
            axagent_trajectory::RewardWeights::default(),
        ))),
        batch_processor: Arc::new(axagent_trajectory::BatchProcessor::new(
            shared_trajectory_storage.clone(),
            axagent_trajectory::BatchConfig::default(),
        )),
        skill_evolution_engine: Arc::new(tokio::sync::Mutex::new(
            axagent_trajectory::SkillEvolutionEngine::new(),
        )),
        skill_proposal_service: Arc::new(TokioRwLock::new(
            axagent_trajectory::SkillProposalService::new(shared_trajectory_storage.clone()),
        )),
        auto_memory_extractor: {
            let auto_ms = axagent_trajectory::MemoryService::new(shared_trajectory_storage.clone())
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to create MemoryService for AutoMemory: {}", e);
                    panic!("MemoryService is required");
                });
            if let Err(e) = auto_ms.initialize() {
                tracing::warn!("Failed to initialize MemoryService for AutoMemory: {}", e);
            }
            let auto_ms = Arc::new(tokio::sync::RwLock::new(auto_ms));
            let auto_pl =
                Arc::new(tokio::sync::RwLock::new(axagent_trajectory::PatternLearner::new(
                    axagent_trajectory::PatternConfig::default(),
                )));
            Arc::new(TokioRwLock::new(axagent_trajectory::AutoMemoryExtractor::new(
                shared_trajectory_storage.clone(),
                auto_ms,
                auto_pl,
            )))
        },
        parallel_execution_service: Arc::new(tokio::sync::RwLock::new(
            axagent_trajectory::ParallelExecutionService::new(10),
        )),
        scheduled_task_service: Arc::new(tokio::sync::RwLock::new(
            axagent_trajectory::ScheduledTaskService::new(100),
        )),
        platform_manager: platform_manager.clone(),
        platform_bridge: platform_bridge.clone(),
        user_profile: Arc::new(TokioRwLock::new(axagent_trajectory::UserProfile::new())),
        local_tool_registry: {
            let mut registry = axagent_agent::LocalToolRegistry::init_from_registry();
            rt.block_on(registry.load_enabled_state(&sea_db));
            Arc::new(tokio::sync::Mutex::new(registry))
        },
        work_engine: Arc::new(tokio::sync::RwLock::new(
            axagent_runtime::work_engine::WorkEngine::new(Arc::new(sea_db.clone())),
        )),
        skill_decomposer: Arc::new(tokio::sync::RwLock::new(
            axagent_trajectory::SkillDecomposer::new(),
        )),
        proactive_service: Arc::new(tokio::sync::RwLock::new(ProactiveService::new())),
        dashboard_registry: Some(Arc::new(
            axagent_runtime::dashboard_registry::DashboardRegistry::new(),
        )),
        webhook_subscription_manager: Some(Arc::new(
            axagent_runtime::webhook_subscription::WebhookSubscriptionManager::new(),
        )),
        semantic_cache: {
            let cache = rt
                .block_on(SemanticCache::new(sea_db.clone(), CacheConfig::default()))
                .unwrap_or_else(|e| {
                    tracing::error!(
                        "Failed to init semantic cache (will use empty fallback): {}",
                        e
                    );
                    let rt2 =
                        tokio::runtime::Runtime::new().expect("Failed to create fallback runtime");
                    rt2.block_on(SemanticCache::new(sea_db.clone(), CacheConfig::default()))
                        .unwrap_or_else(|e2| {
                            tracing::error!("Semantic cache fallback also failed: {}", e2);
                            panic!("Semantic cache initialization failed: {}", e2);
                        })
                });
            Arc::new(tokio::sync::Mutex::new(cache))
        },
        browser_client: Arc::new(tokio::sync::Mutex::new(None)),
        dream_consolidator: Arc::new(axagent_trajectory::DreamConsolidator::new()),
        sync_engine,
    }
}

fn create_sync_engine(
    _sea_db: &sea_orm::DatabaseConnection,
    _app_settings: &axagent_core::types::AppSettings,
) -> Option<Arc<SyncEngine>> {
    #[cfg(mobile)]
    {
        let cloud_config = load_cloud_storage_config(_sea_db, _app_settings)?;
        let backend = cloud_config.create_backend().ok()?;
        let device_id = hostname_or_uuid();
        let profile_name = cloud_config.profile_name.clone();
        Some(Arc::new(SyncEngine::new(backend, &profile_name, &device_id)))
    }
    #[cfg(not(mobile))]
    {
        None
    }
}

#[cfg(mobile)]
fn load_cloud_storage_config(
    sea_db: &sea_orm::DatabaseConnection,
    _app_settings: &axagent_core::types::AppSettings,
) -> Option<CloudStorageConfig> {
    use axagent_core::cloud_storage::{BackendType, S3Config, S3ProviderPreset, SyncMode};
    let rt = tokio::runtime::Handle::current();
    let settings = rt
        .block_on(axagent_core::repo::settings::get_settings(sea_db))
        .ok()?;

    if !settings.cloud_sync_enabled {
        return None;
    }

    let backend_type = match settings.cloud_backend.as_deref() {
        Some("s3") => BackendType::S3,
        Some("webdav") => BackendType::WebDav,
        _ => return None,
    };

    let cloud_config = CloudStorageConfig {
        provider_preset: settings
            .s3_provider_preset
            .unwrap_or(S3ProviderPreset::Custom),
        backend_type,
        sync_enabled: true,
        sync_mode: SyncMode::Sync,
        profile_name: settings
            .sync_profile_name
            .clone()
            .unwrap_or_else(|| "default".to_string()),
        webdav: settings
            .webdav_host
            .as_ref()
            .map(|h| axagent_core::cloud_storage::WebDavConfig {
                host: h.clone(),
                username: settings.webdav_username.clone().unwrap_or_default(),
                password: settings.webdav_password.clone().unwrap_or_default(),
                path: settings.webdav_path.clone().unwrap_or_default(),
                accept_invalid_certs: settings.webdav_accept_invalid_certs,
            }),
        s3: settings.s3_endpoint.as_ref().map(|e| S3Config {
            endpoint: e.clone(),
            region: settings.s3_region.clone().unwrap_or_default(),
            bucket: settings.s3_bucket.clone().unwrap_or_default(),
            access_key_id: settings.s3_access_key_id.clone().unwrap_or_default(),
            secret_access_key: settings.s3_secret_access_key.clone().unwrap_or_default(),
            root: settings.s3_root.clone().unwrap_or_default(),
            use_path_style: settings.s3_use_path_style,
        }),
    };

    Some(cloud_config)
}

#[cfg(mobile)]
fn hostname_or_uuid() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}
