// SPDX-License-Identifier: AGPL-3.0-only

use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::Mutex;
use tokio::sync::RwLock as TokioRwLock;

use super::database::DatabaseInitResult;
use crate::AppState;
use crate::app_state::SemanticCacheState;
use crate::commands::proactive::ProactiveService;
use crate::semantic_cache::{CacheConfig, SemanticCache};
use crate::state::{BrowserClientField, LearningEngineState, SandboxExecutorField, ToolState};
use axagent_dao::repo::agent_session_repo::DaoAgentSessionRepository;
use axagent_dao::repo::feedback_data_lake::FeedbackDataLakeDao;
use axagent_dao::search_sources_impl::{
    MemoryUnifiedSource, ObsidianUnifiedSource, RagUnifiedSource, WikiUnifiedSource,
};
use axagent_harness::AgentSessionRepository;
use axagent_harness::PatternPromptGuard;
use axagent_harness::feedback_data_lake::register_feedback_lake;
use axagent_orchestrator::{IndustryAdapterRegistry, IndustryLearningEngine};
use axagent_plugins::{PluginManager, PluginManagerConfig};
use axagent_runtime_core::prompt_cache::PromptCache;
use axagent_storage::cloud_storage::{CloudStorageConfig, SyncEngine};
use tokio_util::sync::CancellationToken;

/// 构造 AppState。
///
/// 失败时返回结构化错误，由调用方决定如何处理（错误展示 / 重试 / 退出）。
/// 不再 `process::exit(1)`——harness 架构要求启动错误可被前端感知。
pub async fn create_app_state(db_result: DatabaseInitResult) -> Result<AppState, String> {
    let t_start = std::time::Instant::now();
    tracing::info!("[startup] create_app_state begin");

    // 命令元数据已通过 inventory 在编译时自动收集，无需手动初始化

    let DatabaseInitResult { db_handle, master_key, db_path, app_dir, .. } = db_result;

    // 初始化 RLOptimizer 共享状态（优先从文件加载，自动持久化）
    crate::commands::_shared_state::init_shared_state(&app_dir);

    // db_handle 进入 harness（Step 4）；同时克隆 conn 给其它需要 DatabaseConnection 的
    // 旧式组件（vector_store / trajectory_storage / cron / semantic_cache 等）。
    // 这些组件后续在 Step 5/6 也会迁到 harness 内部。
    let sea_db = db_handle.conn.clone();

    // 会话状态存储（能力按需加载闭环 P0-1）：CapabilityLoad 写、下轮注入器读。
    // 贯穿整条链路，故在此处最先构造，后续注入 AppState 与 CapabilityLoad。
    let session_state_store: Arc<dyn axagent_harness::SessionStateStore> =
        Arc::new(axagent_dao::DaoSessionStateStore::new(Arc::new(sea_db.clone())));

    let vector_store = axagent_search::vector_store::VectorStore::new(sea_db.clone());
    let vector_store_arc = Arc::new(vector_store);

    {
        let db_conn = sea_db.clone();
        let mk = master_key;
        let vs = vector_store_arc.clone();
        axagent_tools::knowledge_callback::set_knowledge_search_callback(std::sync::Arc::new(
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
                        .map(|r| axagent_tools::knowledge_callback::KnowledgeSearchHit {
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

    // 注入 tools 扩展层的 trait 实现（MigrationRunner + PluginAgentProvider + DelegateTaskRunner）。
    // 通过 OnceLock 全局注入，工具层不再依赖 axagent-migration / axagent-plugins / commands。
    axagent_tools::tools::init_extensions(
        std::sync::Arc::new(axagent_migration::DefaultMigrationRunner),
        std::sync::Arc::new(axagent_plugins::agent_provider::GlobalPluginAgentProvider),
        None,
    );
    // DelegateTaskRunner — 注入到 tools crate 供 DelegateTaskTool 使用
    crate::commands::multi_agent::init_delegate_task_runner(sea_db.clone(), master_key);

    // 注册 MultiAgentTriggerHook 到全局 HookChain（供后续 conversation loop 挂载使用）
    crate::commands::multi_agent::register_global_multi_agent_hook();

    // 注入 search 层的 5 个数据源 trait 实现。
    // search crate 不再依赖 axagent-dao / axagent-document-parser。
    axagent_search::sources::set_sources(
        std::sync::Arc::new(axagent_dao::search_sources_impl::DefaultKnowledgeSource {
            db: sea_db.clone(),
        }),
        std::sync::Arc::new(axagent_dao::search_sources_impl::DefaultMemorySource {
            db: sea_db.clone(),
        }),
        std::sync::Arc::new(axagent_dao::search_sources_impl::DefaultWikiSource {
            db: sea_db.clone(),
        }),
        std::sync::Arc::new(axagent_dao::search_sources_impl::DefaultSettingsSource {
            db: sea_db.clone(),
        }),
        std::sync::Arc::new(axagent_document_parser::parser_impl::DefaultDocumentParser),
    );

    // 同步注入 tools 层的 DocumentParser
    axagent_tools::parser::set_parser(std::sync::Arc::new(
        axagent_document_parser::parser_impl::DefaultDocumentParser,
    ));

    // 注册统一知识源实现（v2）：RAG / Wiki / Memory / Obsidian 四类知识源
    axagent_search::sources::set_unified_sources(vec![
        std::sync::Arc::new(RagUnifiedSource { db: sea_db.clone() }),
        std::sync::Arc::new(WikiUnifiedSource { db: sea_db.clone() }),
        std::sync::Arc::new(MemoryUnifiedSource { db: sea_db.clone() }),
        std::sync::Arc::new(ObsidianUnifiedSource { db: sea_db.clone() }),
    ]);

    // 注册全局反馈数据湖实现
    register_feedback_lake(std::sync::Arc::new(FeedbackDataLakeDao::new(std::sync::Arc::new(
        sea_db.clone(),
    ))));

    // ensure_preset_servers / migrate_hardcoded_paths / migrate_legacy_keys
    // 已合并到 axagent_dao::db::create_pool() 中，无需在此重复调用

    let app_settings = axagent_dao::repo::settings::get_settings(&sea_db).await.unwrap_or_default();

    axagent_storage::storage_paths::init_documents_root(
        app_settings.documents_root_override.as_ref().map(PathBuf::from),
    );
    axagent_storage::storage_paths::ensure_documents_dirs().unwrap_or_else(|e| {
        tracing::warn!("Failed to create documents storage dirs (non-critical on mobile): {}", e);
    });

    let shared_trajectory_storage: Arc<axagent_trajectory::TrajectoryStorage> = {
        let t_ts = std::time::Instant::now();
        // PostgreSQL 下 FTS5（基于 rusqlite）不可用，直接用无 FTS 的存储
        // （trajectory 全文检索降级为空结果；基表的 tsvector 列已在 v001 预留）。
        // SQLite 下走 with_fts_path 构建 FTS5 虚拟表。
        let storage = if sea_db.get_database_backend() == sea_orm::DbBackend::Postgres {
            axagent_trajectory::TrajectoryStorage::new(Arc::new(sea_db.clone()))
        } else {
            let db_file_path = db_path.strip_prefix("sqlite:").unwrap_or(&db_path);
            axagent_trajectory::TrajectoryStorage::with_fts_path(
                Arc::new(sea_db.clone()),
                db_file_path,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to init trajectory FTS5, falling back to no-FTS: {}", e);
                axagent_trajectory::TrajectoryStorage::new(Arc::new(sea_db.clone()))
            })
        };
        tracing::info!("[startup] TrajectoryStorage 初始化完成 ({}ms)", t_ts.elapsed().as_millis());
        Arc::new(storage)
    };

    let memory_service = {
        let ms = match axagent_trajectory::MemoryService::new(shared_trajectory_storage.clone()) {
            Ok(ms) => ms,
            Err(e) => {
                tracing::error!("Failed to create MemoryService: {} — retrying once", e);
                match axagent_trajectory::MemoryService::new(shared_trajectory_storage.clone()) {
                    Ok(ms) => ms,
                    Err(e2) => {
                        tracing::error!(
                            "MemoryService creation failed after retry: {} — creating with fresh storage",
                            e2
                        );
                        // 用新 TrajectoryStorage 兜底，避免 panic 导致 Android 静默崩溃
                        let fallback_storage =
                            std::sync::Arc::new(axagent_trajectory::TrajectoryStorage::new(
                                std::sync::Arc::new(sea_db.clone()),
                            ));
                        match axagent_trajectory::MemoryService::new(fallback_storage) {
                            Ok(ms) => ms,
                            Err(e3) => {
                                let msg = format!("MemoryService unreachable path reached: {}", e3);
                                crate::android_utils::report_fatal_error(&msg);
                                return Err(msg);
                            },
                        }
                    },
                }
            },
        };
        // P0-OPT: FTS5 初始化移到后台异步，加速首帧显示
        tracing::debug!("MemoryService created (FTS5 init deferred to background)");
        Arc::new(TokioRwLock::new(ms))
    };

    // ── 初始化 Harness 容器（统一管理核心基础设施注入） ──
    let provider_registry = axagent_providers::registry::ProviderRegistry::create_default();
    let harness =
        axagent_runtime::harness::RuntimeHarness::new(axagent_runtime::harness::HarnessDeps {
            persistence: Arc::new(db_handle) as axagent_harness::SharedPersistence,
            master_key,
            provider_registry: Arc::new(provider_registry)
                as Arc<dyn axagent_harness::registry::ProviderRegistry>,
        });
    let harness_registry = harness.provider_registry().clone();

    let platform_manager =
        Arc::new(axagent_runtime::message_gateway::platform_manager::PlatformManager::new());

    // ── Webhook 派发器（P0 修复：实例化 WebhookDispatcher + WebhookEventEmitter） ──
    // WebhookSubscriptionManager 必须先于 PlatformBridge 创建，
    // 这样 PlatformBridge::on_message 中的 MessageReceived / MessageSent 事件才能被派发。
    // P2-7: 注入 DbWebhookPersistence，启动时自动从 DB 恢复订阅，增删/状态变化时持久化。
    let webhook_persistence: Arc<dyn axagent_harness::WebhookPersistence> =
        Arc::new(DbWebhookPersistence { db: sea_db.clone() });
    let webhook_subscription_manager: Option<
        Arc<axagent_runtime::webhook_subscription::WebhookSubscriptionManager>,
    > = {
        let t_wh = std::time::Instant::now();
        let mgr = axagent_runtime::webhook_subscription::WebhookSubscriptionManager::new()
            .with_persistence(webhook_persistence)
            .await
            .map_err(|e| format!("初始化 WebhookSubscriptionManager 失败: {}", e))?;
        tracing::info!(
            "[startup] WebhookSubscriptionManager 初始化完成 ({}ms)",
            t_wh.elapsed().as_millis()
        );
        Some(Arc::new(mgr))
    };
    let webhook_dispatcher: Option<Arc<axagent_rt_webhook::webhook_dispatcher::WebhookDispatcher>> =
        webhook_subscription_manager.as_ref().map(|mgr| {
            Arc::new(axagent_rt_webhook::webhook_dispatcher::WebhookDispatcher::new(
                mgr.clone() as Arc<dyn axagent_harness::WebhookSubscriptionService>
            ))
        });
    // 把 dispatcher 转为 trait 对象传给 PlatformBridge
    let webhook_dispatch_trait: Option<Arc<dyn axagent_harness::WebhookDispatch>> =
        webhook_dispatcher.as_ref().map(|d| d.clone() as Arc<dyn axagent_harness::WebhookDispatch>);
    // WebhookEventEmitter 用于在工具执行 / Agent 结束时触发事件（注入到 AppState 供下游使用）
    // 以 trait 对象形式存储，命令层无需依赖 rt-webhook crate
    let webhook_event_emitter: Option<Arc<dyn axagent_harness::WebhookEventSink>> =
        webhook_dispatcher.as_ref().map(|d| {
            let emitter =
                axagent_rt_webhook::webhook_dispatcher::WebhookEventEmitter::new(d.clone());
            Arc::new(emitter) as Arc<dyn axagent_harness::WebhookEventSink>
        });

    // PlatformBridge 经 message.callback / webhook.dispatch 能力接缝取依赖：
    // 回调与派发器由下方注册进能力注册表，桥在收发消息时读接缝（外部插件可
    // 经 register_external_* 替换同一接缝，内置与插件平权）。
    let platform_bridge = harness.build_platform_bridge(platform_manager.clone());

    // ── P2 rt-messaging 接缝：接入能力注册表 ──────────────────────────────
    // message.callback（PlatformMessageCallback）与 webhook.dispatch（WebhookDispatch）
    // 的权威定义均在 harness。wiring 层在注入 PlatformManager / PlatformBridge 的
    // 同时注册进注册表，使外部插件可经 register_external_* 替换同一接缝
    // （内置与插件平权）。webhook.dispatch 仅在 dispatcher 存在时注册
    // （无 webhook 订阅管理 = 无派发需求，与未注入等价）。
    {
        let capability_registry = axagent_harness::get_capability_registry();
        let bridge_dyn: Arc<dyn axagent_harness::PlatformMessageCallback> = platform_bridge.clone();
        match capability_registry.register_message_callback(bridge_dyn) {
            Ok(_) => tracing::info!("message.callback 接缝已注册 (BuiltIn)"),
            Err(e) => tracing::warn!("message.callback 注册失败: {e}"),
        }
        if let Some(dispatcher) = webhook_dispatch_trait.clone() {
            match capability_registry.register_webhook_dispatch(dispatcher) {
                Ok(_) => tracing::info!("webhook.dispatch 接缝已注册 (BuiltIn)"),
                Err(e) => tracing::warn!("webhook.dispatch 注册失败: {e}"),
            }
        }

        // ── event.dispatch 接缝：注册内置类型化事件派发总线（P2 事件化） ──
        // 组件/插件经 get_event_dispatcher() 拿到总线，再 subscribe 挂装
        // 四派发模式（emit/waterfall/parallel/serial）的订阅者。
        let event_dispatch_bus = Arc::new(axagent_harness::EventDispatchBus::new());
        match capability_registry.register_event_dispatcher(event_dispatch_bus) {
            Ok(_) => tracing::info!("event.dispatch 接缝已注册 (BuiltIn)"),
            Err(e) => tracing::warn!("event.dispatch 注册失败: {e}"),
        }

        // ── session.log.invariant 接缝：注册内置会话日志不变量（P2 缺陷#3 05 项） ──
        // 记录模型可见内容并支持可重建校验（Model-visible means logged）。
        // 默认落盘实现，按 session 持久化为 JSONL 到 app_dir/session_logs，进程重启后可回放；
        // 外部插件可经 register_external_* 替换实现。
        let session_log_invariant: Arc<dyn axagent_harness::SessionLogInvariant> =
            match axagent_harness::DiskSessionLog::new(app_dir.join("session_logs")) {
                Ok(log) => Arc::new(log),
                Err(e) => {
                    tracing::warn!("会话日志落盘初始化失败，回退内存实现: {e}");
                    Arc::new(axagent_harness::InMemorySessionLog::new())
                },
            };
        match capability_registry.register_session_log_invariant(session_log_invariant) {
            Ok(_) => tracing::info!("session.log.invariant 接缝已注册 (BuiltIn, 落盘)"),
            Err(e) => tracing::warn!("session.log.invariant 注册失败: {e}"),
        }

        // ── platform.adapter 接缝：注册 8 个内置消息平台适配器 ──
        // 适配器由 PlatformManager 统一管理（reconcile / 生命周期），
        // 同时注册到能力注册表，供外部插件与消费者按平台名查询/替换。
        let adapters = platform_manager.list_all_adapters().await;
        for (name, adapter) in adapters {
            let adapter_dyn: Arc<dyn axagent_harness::MessagePlatformAdapter> = adapter;
            match capability_registry.register_platform_adapter(&name, adapter_dyn) {
                Ok(_) => tracing::info!("platform.adapter.{name} 已注册 (BuiltIn)"),
                Err(e) => tracing::warn!("platform.adapter.{name} 注册失败: {e}"),
            }
        }
    }

    let sync_engine = create_sync_engine(&sea_db, &app_settings).await;

    // ── 设备同步状态初始化 ──
    let local_device_id = axagent_device::manager::DeviceManagerImpl::create_local_device(
        "This Device".to_string(),
        hostname_or_uuid(),
        std::env::consts::OS.to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    )
    .device_id;
    // 使用数据库存储（支持 PostgreSQL/SQLite 双数据库）
    let sync_storage = Arc::new(axagent_dao::SyncStorageDb::new(sea_db.clone()));
    let device_sync_state = Arc::new(tokio::sync::RwLock::new(
        crate::commands::device_sync::DeviceSyncState::with_storage(
            local_device_id,
            sync_storage as Arc<dyn axagent_harness::device_sync::SyncStorage>,
        )
        .await,
    ));

    let config_home = app_dir.clone();
    let mut plugin_config = PluginManagerConfig::new(config_home.clone());
    plugin_config.external_dirs = axagent_kit::skill_dirs::all_skills_dirs();
    let npm_registry = Arc::new(axagent_npm::NpmRegistry::new());
    let plugin_manager = Arc::new(tokio::sync::RwLock::new(
        PluginManager::new(plugin_config)
            .with_npm_registry(npm_registry)
            .with_capability_registry(axagent_harness::get_capability_registry()),
    ));

    // ── Extract every AppState field into a local so that the same values
    //    can be shared between the top-level `AppState` and the new domain
    //    sub-states (`infra`, `gateway`, `task`, `agent`, `memory`, `skill`).
    let gateway_server: Arc<Mutex<Option<axagent_gateway::server::GatewayServer>>> =
        Arc::new(Mutex::new(None));
    let close_to_tray: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let auto_backup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>> =
        Arc::new(Mutex::new(None));
    let webdav_sync_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>> =
        Arc::new(Mutex::new(None));
    let api_server_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>> =
        Arc::new(Mutex::new(None));
    let trajectory_cleanup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>> =
        Arc::new(Mutex::new(None));
    let task_manager = Arc::new(axagent_runtime::task_manager::TaskManager::new());
    let shutdown_token = CancellationToken::new();
    let stream_cancel_flags: Arc<DashMap<String, Arc<AtomicBool>>> = Arc::new(DashMap::new());
    let agent_permission_senders: Arc<
        Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>,
    > = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let agent_ask_senders: Arc<
        Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>,
    > = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let agent_always_allowed: Arc<
        Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>,
    > = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let agent_prompters: Arc<
        Mutex<std::collections::HashMap<String, axagent_agent::ChannelPermissionPrompter>>,
    > = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let agent_plan_approvals: Arc<
        Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<bool>>>,
    > = Arc::new(Mutex::new(std::collections::HashMap::new()));
    // 能力补齐/进化改进提议的挂起审批槽：proposalId → 同意信号发送端。
    // 认知编排器三触发点生成提议后 await；前端同意/拒绝由 capability_gap_consent 回传。
    let evolution_consent_senders: Arc<
        Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<bool>>>,
    > = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let pending_capability_gaps: Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<String, axagent_harness::CapabilityGapProposal>,
        >,
    > = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
    let agent_session_repo: Arc<dyn AgentSessionRepository> =
        Arc::new(DaoAgentSessionRepository::new(Arc::new(sea_db.clone())));
    let agent_session_manager = Arc::new(axagent_agent::SessionManager::new(agent_session_repo));
    let agent_cancel_tokens: Arc<DashMap<String, Arc<AtomicBool>>> = Arc::new(DashMap::new());
    let agent_paused: Arc<Mutex<std::collections::HashSet<String>>> =
        Arc::new(Mutex::new(std::collections::HashSet::new()));
    // P0-3：暂停桥接。conversationId → 共享 PauseState（runtime 循环在此等待）。
    // agent_paused 是命令层可见性的权威集合；PauseState 是 runtime 层的实际闸门。
    let agent_pause_states: Arc<DashMap<String, Arc<axagent_runtime_core::PauseState>>> =
        Arc::new(DashMap::new());
    let running_agents: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>> =
        Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new()));
    let steer_queue: Arc<tokio::sync::Mutex<std::collections::HashMap<String, Vec<String>>>> =
        Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
    // P0-3 修复:启用 Reflector JSONL 持久化(进程重启后历史反思不丢失)。
    let reflector = Arc::new(
        axagent_agent::Reflector::new().with_persistence(app_dir.join("reflections.jsonl")),
    );
    let shared_memory: Arc<TokioRwLock<axagent_runtime::shared_memory::SharedMemory>> =
        Arc::new(TokioRwLock::new(axagent_runtime::shared_memory::SharedMemory::new()));
    let sub_agent_registry: Arc<TokioRwLock<axagent_trajectory::SubAgentRegistry>> = {
        let t_sar = std::time::Instant::now();
        let reg = Arc::new(TokioRwLock::new(
            axagent_trajectory::SubAgentRegistry::new().await.unwrap_or_default(),
        ));
        tracing::info!("[startup] SubAgentRegistry 初始化完成 ({}ms)", t_sar.elapsed().as_millis());
        reg
    };
    let nudge_service: Arc<tokio::sync::Mutex<axagent_trajectory::NudgeService>> =
        Arc::new(tokio::sync::Mutex::new(axagent_trajectory::NudgeService::new()));
    let closed_loop_service =
        Arc::new(axagent_trajectory::ClosedLoopService::new(shared_trajectory_storage.clone()));
    let insight_system: Arc<TokioRwLock<axagent_trajectory::LearningInsightSystem>> =
        Arc::new(TokioRwLock::new(
            axagent_trajectory::LearningInsightSystem::new().with_storage_limits(200, 30),
        ));
    let realtime_learning: Arc<tokio::sync::Mutex<axagent_trajectory::RealTimeLearning>> =
        Arc::new(tokio::sync::Mutex::new(axagent_trajectory::RealTimeLearning::new()));
    let pattern_learner: Arc<TokioRwLock<axagent_trajectory::PatternLearner>> =
        Arc::new(TokioRwLock::new(axagent_trajectory::PatternLearner::new(
            axagent_trajectory::PatternConfig::default(),
        )));
    let cross_session_learner: Arc<TokioRwLock<axagent_trajectory::CrossSessionLearner>> =
        Arc::new(TokioRwLock::new(axagent_trajectory::CrossSessionLearner::new()));
    let rl_engine: Arc<TokioRwLock<axagent_trajectory::RLEngine>> =
        Arc::new(TokioRwLock::new(axagent_trajectory::RLEngine::new(
            axagent_trajectory::RLConfig::default(),
            axagent_trajectory::RewardWeights::default(),
        )));
    let batch_processor = Arc::new(axagent_trajectory::BatchProcessor::new(
        shared_trajectory_storage.clone(),
        axagent_trajectory::BatchConfig::default(),
    ));
    let skill_evolution_engine: Arc<tokio::sync::Mutex<axagent_trajectory::SkillEvolutionEngine>> = {
        #[cfg(not(target_os = "android"))]
        {
            let mut engine = axagent_trajectory::SkillEvolutionEngine::new();
            // 阶段三 T3.4：注入沙箱执行器，进化产物先沙箱验证再落地。
            engine
                .set_sandbox(Arc::new(
                    axagent_trajectory::SkillSandboxExecutor::with_default_policy(),
                ))
                .await;
            // P0-OPT: LLM 变异器注入移到后台异步，加速首帧显示
            tracing::debug!(
                "SkillEvolutionEngine created (LLM mutator injection deferred to background)"
            );
            Arc::new(tokio::sync::Mutex::new(engine))
        }
        #[cfg(target_os = "android")]
        {
            Arc::new(tokio::sync::Mutex::new(axagent_trajectory::SkillEvolutionEngine::new()))
        }
    };
    let skill_proposal_service: Arc<TokioRwLock<axagent_trajectory::SkillProposalService>> =
        Arc::new(TokioRwLock::new(axagent_trajectory::SkillProposalService::new(
            shared_trajectory_storage.clone(),
        )));
    let auto_memory_extractor: Arc<TokioRwLock<axagent_trajectory::AutoMemoryExtractor>> = {
        let auto_ms = match axagent_trajectory::MemoryService::new(
            shared_trajectory_storage.clone(),
        ) {
            Ok(ms) => ms,
            Err(e) => {
                tracing::warn!(
                    "Failed to create MemoryService for AutoMemory: {} — falling back to primary memory service",
                    e
                );
                // 回退到主 memory_service（克隆引用），避免 panic 导致 Android 静默崩溃
                match axagent_trajectory::MemoryService::new(shared_trajectory_storage.clone()) {
                    Ok(ms) => ms,
                    Err(e2) => {
                        tracing::error!(
                            "AutoMemory MemoryService fallback also failed: {} — creating with fresh storage",
                            e2
                        );
                        let fallback_storage =
                            std::sync::Arc::new(axagent_trajectory::TrajectoryStorage::new(
                                std::sync::Arc::new(sea_db.clone()),
                            ));
                        match axagent_trajectory::MemoryService::new(fallback_storage) {
                            Ok(ms) => ms,
                            Err(e3) => {
                                let msg = format!("AutoMemory MemoryService unreachable: {}", e3,);
                                crate::android_utils::report_fatal_error(&msg);
                                return Err(msg);
                            },
                        }
                    },
                }
            },
        };
        // P0-OPT: AutoMemory FTS5 初始化也移到后台
        tracing::debug!("AutoMemoryExtractor created (FTS5 init deferred to background)");
        let auto_ms = Arc::new(tokio::sync::RwLock::new(auto_ms));
        let auto_pl = Arc::new(tokio::sync::RwLock::new(axagent_trajectory::PatternLearner::new(
            axagent_trajectory::PatternConfig::default(),
        )));
        Arc::new(TokioRwLock::new(axagent_trajectory::AutoMemoryExtractor::new(
            shared_trajectory_storage.clone(),
            auto_ms,
            auto_pl,
        )))
    };
    let parallel_execution_service: Arc<
        tokio::sync::RwLock<axagent_trajectory::ParallelExecutionService>,
    > = Arc::new(tokio::sync::RwLock::new(axagent_trajectory::ParallelExecutionService::new(10)));
    let cron_job_store: Arc<axagent_runtime_core::CronJobStore> = {
        let t_cron = std::time::Instant::now();
        let store =
            Arc::new(axagent_runtime_core::CronJobStore::new(Arc::new(sea_db.clone())).await);
        tracing::info!("[startup] CronJobStore 初始化完成 ({}ms)", t_cron.elapsed().as_millis());
        store
    };
    let user_profile: Arc<TokioRwLock<axagent_trajectory::UserProfile>> =
        Arc::new(TokioRwLock::new(axagent_trajectory::UserProfile::new()));
    let local_tool_registry: Arc<tokio::sync::Mutex<axagent_tools::registry::UnifiedToolRegistry>> = {
        let mut registry = axagent_tools::registry::UnifiedToolRegistry::new();
        // P0-OPT: load_enabled_state 推迟到后台（DB 查询，非关键路径）
        // 挂载 RL 策略工具排名器，每次 get_chat_tools() 实时读取最新权重。
        // 受 settings.rl_optimizer_enabled 门控：关闭时回退到默认工具顺序。
        registry.tool_ranker = if app_settings.rl_optimizer_enabled {
            Some(crate::commands::_shared_state::SHARED_TOOL_RANKER.clone())
        } else {
            None
        };
        // ── OS 级沙箱策略（PLAN-codex-parity P0-1c）──
        // 从 Settings 的 sandbox_mode 构造全局策略：此后所有 ToolRegistry（含
        // 每次请求临时 new() 的实例）构建 ToolContext 时自动回退读取。
        // 默认 "danger-full-access" → 受限子进程不启用，行为与既往一致。
        let sandbox_workspace = app_settings
            .default_workspace_dir
            .as_ref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            });
        axagent_tools::registry::set_global_sandbox_policy(
            axagent_harness::SandboxPolicy::from_mode_str(
                &app_settings.sandbox_mode,
                sandbox_workspace,
            ),
        );
        axagent_tools::registry::set_global_approval_policy(
            axagent_harness::ApprovalPolicy::from_policy_str(&app_settings.approval_policy),
        );
        tracing::info!(
            "[startup] 沙箱/审批策略已注入: sandbox_mode={} approval_policy={}",
            app_settings.sandbox_mode,
            app_settings.approval_policy
        );
        // ── 阶段二 T2.3：注册自指工具（system_evolution_*）──
        // 自指工具走 system_* 系统能力回调通道：不暴露给 LLM（注册后默认禁用，
        // 仅系统内部按名回调执行），动态部署/卸载等高权限操作不被 Agent 直接触发。
        // 系统注册表可见性由 register_system_capabilities 中的 SYSTEM_ONLY 护照保证。
        for tool in axagent_tools::runtime_mutation::create_all_self_referential_tools() {
            let name = tool.name().to_string();
            match registry.register_runtime_tool(tool, "system_evolution") {
                Ok(()) => {
                    registry.groups.disabled_tools.insert(name);
                },
                Err(e) => {
                    tracing::warn!(target: "evolution_engine", tool = %name, error = %e,
                        "注册自指工具失败");
                },
            }
        }
        Arc::new(tokio::sync::Mutex::new(registry))
    };
    // ── 阶段 5:工作流反思 / 进化 / 优化三层 trait 实现 ──
    // 同一份 Arc 实例注册进能力接缝（workflow.reflector / workflow.evolver /
    // workflow.optimizer），WorkEngine 与命令层均经接缝取用（单一权威来源）。
    // 启动即用,纯启发式;真正的 LLM 变异 / 沙箱验证由 wiring 层后续通过 setter 注入(此处 MVP 不注入)。
    //
    // 优化 3:反思器注入 `shared_trajectory_storage`,每次 reflect()/reflect_node()
    // 后同步落库到 `trajectory_workflow_reflections` 表,供跨会话查询 / 模式聚合 /
    // 进化决策使用。落库 best-effort,失败仅 warn 日志,不影响工作流主流程。
    //
    // 优化 4:启动时尝试构造 `ProviderLlmBridge` 注入 evolver 的 LLM 变异器;
    // 沙箱始终注入 `StructuralWorkflowSandbox`(静态结构校验,无副作用)。
    // 若没有启用的 provider(LLM bridge = None),仅跳过 LLM 注入,evolver 仍可用
    // 内置占位变异;沙箱结构校验始终生效。
    let workflow_reflector: Arc<dyn axagent_harness::WorkflowReflector> =
        axagent_trajectory::WorkflowReflectorImpl::with_storage(
            axagent_trajectory::ReflectorConfig::default(),
            shared_trajectory_storage.clone(),
        )
        .into_arc();
    let workflow_evolver: Arc<dyn axagent_harness::WorkflowEvolver> =
        axagent_trajectory::WorkflowEvolverImpl::with_defaults().into_arc();
    let workflow_optimizer: Arc<dyn axagent_harness::WorkflowOptimizer> =
        axagent_trajectory::WorkflowOptimizerImpl::with_defaults().into_arc();

    // P0-OPT: LLM 变异器注入移到后台异步，加速首帧显示
    tracing::debug!("WorkflowEvolver created (LLM injection deferred to background)");

    // P2-8:注入带有限试运行的沙箱(静态校验 + 模拟执行,始终注入)
    // 比 ReachabilityWorkflowSandbox 更强:额外做节点级配置合理性、累积超时上限、
    // 环检测,并用 tokio::time::timeout 做硬超时保护(5 秒)。
    // 沙箱统一经 workflow.sandbox 能力接缝分发：此处注册后，进化器在验证时读接缝
    // （外部插件可经 register_external_sandbox 可逆替换）。
    {
        let dry_run_sandbox: std::sync::Arc<dyn axagent_harness::WorkflowSandbox> =
            std::sync::Arc::new(super::workflow_injections::DryRunWorkflowSandbox::new());
        match axagent_harness::get_capability_registry().register_sandbox(dry_run_sandbox) {
            Ok(_) => tracing::info!("workflow.sandbox 接缝已注册 (BuiltIn)"),
            Err(e) => tracing::warn!("workflow.sandbox 注册失败: {e}"),
        }
    }

    // 方案 3A:注入基因组加载器(从 DB 加载真实模板构造初始种群)
    {
        let repo = axagent_harness::repositories::workflow_template_repository();
        let loader = super::workflow_injections::DaoWorkflowGenomeLoader::new(repo);
        if let Err(e) = workflow_evolver
            .set_genome_loader(std::sync::Arc::new(loader)
                as std::sync::Arc<dyn axagent_harness::WorkflowGenomeLoader>)
            .await
        {
            tracing::warn!("[Evolver] set_genome_loader failed: {e}");
        } else {
            tracing::info!("[Evolver] Genome loader injected (real template-based init)");
        }
    }

    let work_engine: Arc<axagent_runtime::work_engine::WorkEngine> =
        {
            // 反思 / 进化 / 优化 / 业务规则统一经能力接缝分发（下方注册），
            // WorkEngine 在消费点读接缝，不再注入副本。
            let engine = Arc::new(axagent_runtime::work_engine::WorkEngine::new(
                master_key,
                harness_registry.clone(),
            ));
            // Plan 模式：AgentExecutor 注入 engine 引用以创建/执行临时工作流
            engine.inject_into_agent_executor(engine.clone()).await;
            // 注册领域约束：所有角色走通用 DomainConstraints::by_role
            engine.set_domain_constraints(Arc::new(|role_name: &str| {
            axagent_rt_workflow::work_engine::domain_constraints::DomainConstraints::by_role(
                role_name,
            )
        })).await;
            // P0-3: 初始化 dispatcher — 注册所有内置 executor（Trigger/Fallback/Tool/...）
            // 及 pending 中的 Llm/Condition/LlmClassifier。缺此调用会导致 dispatch 时
            // panic("FallbackExecutor must be registered")。
            engine.init_dispatcher().await;
            // 2.7 P1:初始化 TriggerManager — 注入 WorkEngine 引用,
            // 使 Schedule/Webhook/Event 触发器在触发时能调用 run_workflow。
            // 缺此调用会导致 TriggerManager.engine 一直为 None,
            // register_schedule 返回 "引擎未就绪" 错误。
            engine.init_trigger_manager().await;
            // ApprovalNode HITL: 注入数据库连接供 ApprovalOps 回调使用
            engine.set_db(sea_db.clone());
            // 注：ToolRegistry 暂未注入到 WorkEngine。
            // 原因：local_tool_registry 是 Arc<tokio::sync::Mutex<UnifiedToolRegistry>>，
            // 而 ToolRegistry trait 方法是同步的，无法在同步方法中获取 tokio Mutex 锁。
            // 当前工具通过 ToolResolver 回调路径执行（services.rs 中注入），
            // tool_executor.rs 已支持 ToolRegistry find 不到工具时 fallback 到回调路径。
            // 未来注入需要重构为 parking_lot::Mutex 或创建异步适配器。
            engine
        };

    // ── P1 agent-loop 接缝：注入 SessionManager 适配器 ─────────────────────
    // wiring 层把 `WorkflowAgentTurnRunner` 同时：
    //   1. 注册进全局能力注册表（`agent.loop` 接缝，CapabilityOrigin::BuiltIn）
    //   2. 注入 `WorkEngine`（set_agent_turn_runner，AgentExecutor 执行前探测）
    // 使内置 Agent 主循环与外部插件平权——外部插件可经 register_external_agent_loop
    // 替换同一接缝。注入失败仅告警，不阻断启动（AgentExecutor 回退 inline ReAct）。
    {
        let agent_loop_runner: Arc<dyn axagent_harness::AgentTurnRunner> =
            Arc::new(super::agent_turn_adapter::WorkflowAgentTurnRunner::new(
                Arc::clone(&agent_session_manager),
                Arc::new(harness.clone()),
                Arc::clone(&agent_prompters),
            ));
        match axagent_harness::get_capability_registry()
            .register_agent_loop(agent_loop_runner.clone())
        {
            Ok(_handle) => {
                tracing::info!("agent-loop 接缝已注册 (agent.loop, BuiltIn)");
                work_engine.set_agent_turn_runner(agent_loop_runner);
            },
            Err(e) => tracing::warn!("agent-loop 注册失败,回退 inline ReAct: {e}"),
        }
    }

    // ── P2 workflow 反射/进化/优化 + 业务规则接缝：接入能力注册表 ──────────
    // 四个 trait（WorkflowReflector/Evolver/Optimizer/BusinessRuleEvaluator）的
    // 权威定义均在 harness，内置实现由 trajectory / rt-workflow 提供。wiring 层
    // 在注入 WorkEngine 的同时注册进注册表，使外部插件可经 register_external_*
    // 替换同一接缝（内置与插件平权）。business_rule 默认以空规则实现注入
    // （无规则 = 不拦截，与未注入等价）。
    {
        let capability_registry = axagent_harness::get_capability_registry();
        match capability_registry.register_workflow_reflector(workflow_reflector.clone()) {
            Ok(_) => tracing::info!("workflow.reflector 接缝已注册 (BuiltIn)"),
            Err(e) => tracing::warn!("workflow.reflector 注册失败: {e}"),
        }
        match capability_registry.register_workflow_evolver(workflow_evolver.clone()) {
            Ok(_) => tracing::info!("workflow.evolver 接缝已注册 (BuiltIn)"),
            Err(e) => tracing::warn!("workflow.evolver 注册失败: {e}"),
        }
        match capability_registry.register_workflow_optimizer(workflow_optimizer.clone()) {
            Ok(_) => tracing::info!("workflow.optimizer 接缝已注册 (BuiltIn)"),
            Err(e) => tracing::warn!("workflow.optimizer 注册失败: {e}"),
        }
        let br_engine =
            Arc::new(axagent_rt_workflow::business_rules::BusinessRuleEngine::new(Vec::new()));
        let br_engine_dyn: Arc<dyn axagent_harness::BusinessRuleEvaluator> = br_engine.clone();
        match capability_registry.register_business_rule(br_engine_dyn) {
            Ok(_) => tracing::info!("workflow.business_rule 接缝已注册 (BuiltIn)"),
            Err(e) => tracing::warn!("workflow.business_rule 注册失败: {e}"),
        }
    }

    // ── 统一事件总线实例化与注入 ──────────────────────────────────────────
    // 同一份 `Arc<dyn EventBus>` 注入到 agent / rt-workflow / orchestrator 三方,
    // 供跨 crate 事件订阅者消费。未注入时三方保持原有行为,不影响现有功能。
    // orchestrator 在命令层临时创建,通过 AppState.event_bus 字段在调用时注入。
    let event_bus: Arc<dyn axagent_harness::EventBus> =
        Arc::new(axagent_runtime_core::BroadcastEventBus::new(1024));
    agent_session_manager.set_event_bus(Arc::clone(&event_bus)).await;
    // 自进化闭环:注入 Reflector,启用每个 turn 完成时自动复盘
    // (解决 experience_pipeline.rs:243 注释的 "Reflector::reflect() 目前零调用" 问题)
    agent_session_manager.set_reflector(reflector.clone()).await;

    // P0-3: 注入 session_events 持久化 sink
    let session_event_sink: Arc<dyn axagent_harness::SessionEventSink> =
        Arc::new(DbSessionEventSink::new(sea_db.clone()));
    agent_session_manager.set_session_event_sink(session_event_sink).await;

    // 缺陷1修复:从 DB 读取前端 FeatureFlag,注入 SessionManager,
    // 使 finalOutputReflection / selfImprovingLoop 开关真正影响后端复盘行为:
    // - finalOutputReflection=true:turn 完成后同步等待 Reflector 评估
    // - selfImprovingLoop=true + 质量不达标:把改进建议写入 nudge 供下次 turn 使用
    let self_improvement_flags =
        crate::commands::app_config::read_self_improvement_flags(&sea_db).await;
    agent_session_manager.set_self_improvement_flags(self_improvement_flags).await;

    // P2 集成: McpAgentServer wiring。
    // - Agent trait 由 HarnessAgentAdapter 提供 (agent crate)
    // - AgentSessionBroker trait 由 SessionManager 提供 (Arc cast 到 dyn)
    // McpAgentServer 构造在 create_app_state 同步阶段完成，不阻塞首帧渲染。
    let mcp_agent_server: Arc<axagent_mcp::McpAgentServer> = Arc::new(
        axagent_mcp::McpAgentServer::new(
            Some(Arc::new(
                axagent_agent::harness_adapter::HarnessAgentAdapter::new("default")
                    .with_runtime(Arc::clone(&agent_session_manager)),
            ) as Arc<dyn axagent_harness::Agent>),
            Some(Arc::clone(&agent_session_manager) as Arc<dyn axagent_harness::AgentSessionBroker>),
        ),
    );

    work_engine.set_event_bus(Arc::clone(&event_bus));
    let skill_decomposer: Arc<tokio::sync::RwLock<axagent_trajectory::SkillDecomposer>> =
        Arc::new(tokio::sync::RwLock::new(axagent_trajectory::SkillDecomposer::new()));
    let proactive_service: Arc<tokio::sync::RwLock<ProactiveService>> =
        Arc::new(tokio::sync::RwLock::new(ProactiveService::new()));
    let dashboard_registry: Option<Arc<axagent_runtime::dashboard_registry::DashboardRegistry>> =
        Some(Arc::new(axagent_runtime::dashboard_registry::DashboardRegistry::new_with_config(
            axagent_runtime::dashboard_registry::DashboardRegistryConfig {
                plugin_dirs: vec![
                    axagent_storage::storage_paths::documents_root().join("dashboard-plugins"),
                ],
                auto_load: true,
            },
        )));
    // 注：webhook_subscription_manager 已在 PlatformBridge 之前创建（见上方 P0 修复块）
    // P0-OPT: SemanticCache 初始化推迟到后台 —— 先用内存 SQLite 占位（微秒级），
    // 真实文件缓存（CREATE TABLE + FTS5）在 run_deferred_init 中完成。
    let semantic_cache: Arc<tokio::sync::Mutex<SemanticCacheState>> = {
        let t0 = std::time::Instant::now();
        let mem_db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .map_err(|e| format!("内存 SQLite 连接失败: {}", e))?;
        let cache = SemanticCache::new(mem_db, CacheConfig::default())
            .await
            .map_err(|e| format!("内存 SemanticCache 初始化失败: {}", e))?;
        tracing::info!(
            "[startup] SemanticCache 内存占位初始化完成 ({}ms)",
            t0.elapsed().as_millis()
        );
        Arc::new(tokio::sync::Mutex::new(SemanticCacheState {
            cache: Arc::new(cache),
            enabled: true,
            in_memory_entries: Vec::new(),
            similarity_threshold: 0.85,
        }))
    };
    let prompt_cache = Arc::new(PromptCache::new());
    // ── Fleet 持久化仓库 ──
    let fleet_repository: Arc<dyn axagent_harness::fleet::FleetRepository> =
        Arc::new(axagent_trajectory::SeaOrmFleetRepository::new(sea_db.clone()));
    // ── Fleet 意图分类 LLM（真实 Provider 实现，供 dispatcher 路由）──
    let fleet_intent_llm: Arc<dyn axagent_harness::fleet::FleetIntentLlm> =
        Arc::new(crate::commands::fleet::executor::ProviderFleetIntentLlm::new(harness.clone()));

    // ── P3: 任务形态 LLM 兜底分类器（wiring 层注入）──
    let task_shape_llm_classifier: Arc<dyn axagent_harness::TaskShapeLlmClassifier> =
        Arc::new(axagent_runtime::ProviderTaskShapeLlmClassifier::new(harness.clone()));
    // ── P3: ApprovalGate 审批 oneshot 通道 ──
    let task_shape_approval_senders: Arc<
        tokio::sync::Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<bool>>>,
    > = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
    let tot_sessions: Arc<
        tokio::sync::Mutex<std::collections::HashMap<String, crate::app_state::TotSession>>,
    > = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
    let planner_sessions: Arc<
        tokio::sync::Mutex<std::collections::HashMap<String, crate::app_state::PlannerSession>>,
    > = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
    #[cfg(not(target_os = "android"))]
    let browser_client: Arc<
        tokio::sync::Mutex<Option<axagent_kit::browser_automation::PlaywrightClient>>,
    > = axagent_kit::browser_automation::shared_browser_pool().clone();
    #[cfg(target_os = "android")]
    let browser_client: Arc<tokio::sync::Mutex<Option<()>>> =
        Arc::new(tokio::sync::Mutex::new(None));
    let dream_data_provider = Arc::new(
        axagent_trajectory::TrajectoryDreamDataProvider::new(shared_trajectory_storage.clone())
            .with_memory_service(memory_service.clone()),
    );
    let dream_consolidator = Arc::new(
        axagent_trajectory::DreamConsolidator::new()
            .with_data_provider(dream_data_provider.clone()),
    );
    // Smart Router：ML 成本感知路由器实例化
    // P0-OPT: load_from_db 推迟到后台（读取历史决策），不阻塞首帧
    let cost_aware_router =
        Arc::new(crate::smart_router::CostAwareRouter::with_db(Arc::new(sea_db.clone())));
    tracing::info!("[startup] CostAwareRouter 实例化完成（load_from_db 推迟到后台）");
    // Orchestrator 流式报告器初始化（暂不绑定 AppHandle，后续按需注入）
    let stream_reporter: Arc<
        TokioRwLock<Option<Arc<dyn axagent_harness::streaming::AgentStreamReporter>>>,
    > = Arc::new(TokioRwLock::new(None));
    let text_grad_engine: Arc<tokio::sync::Mutex<axagent_trajectory::TextGradEngine>> =
        Arc::new(tokio::sync::Mutex::new(axagent_trajectory::TextGradEngine::new(
            axagent_trajectory::ComputationGraph::new(),
            axagent_trajectory::TextGradConfig::default(),
        )));
    let auto_tool_creator: Arc<tokio::sync::Mutex<axagent_trajectory::AutoToolCreator>> =
        Arc::new(tokio::sync::Mutex::new(axagent_trajectory::AutoToolCreator::new(
            axagent_trajectory::AutoToolCreatorConfig::default(),
            Box::new(axagent_trajectory::DefaultLlmToolProvider::new()),
            Box::new(axagent_trajectory::DefaultSandboxToolTester),
        )));
    let intrinsic_motivation: Arc<
        tokio::sync::Mutex<axagent_trajectory::IntrinsicMotivationEngine>,
    > = Arc::new(tokio::sync::Mutex::new(axagent_trajectory::IntrinsicMotivationEngine::new(
        axagent_trajectory::IntrinsicMotivationConfig::default(),
    )));
    let coevolution_env: Arc<tokio::sync::Mutex<axagent_trajectory::CoevolutionEnvironment>> =
        Arc::new(tokio::sync::Mutex::new(axagent_trajectory::CoevolutionEnvironment::new(
            axagent_trajectory::CoevolutionConfig::default(),
        )));
    let constitution = Arc::new(axagent_trajectory::ImmutableConstitution::new(
        vec![
            axagent_trajectory::ConstitutionalRule::NoSelfModificationOfReward,
            axagent_trajectory::ConstitutionalRule::NoCodeExecutionWithoutSandbox,
            axagent_trajectory::ConstitutionalRule::PreserveUserIntent,
            axagent_trajectory::ConstitutionalRule::MaxModificationSize(0.5),
        ],
        axagent_trajectory::ConstitutionConfig::default(),
    ));
    let process_reward_model: Arc<tokio::sync::Mutex<axagent_trajectory::ProcessRewardModel>> =
        Arc::new(tokio::sync::Mutex::new(
            axagent_trajectory::ProcessRewardModel::default().with_default_provider("general"),
        ));
    let sandbox_executor: Arc<axagent_trajectory::SkillSandboxExecutor> = {
        #[cfg(not(target_os = "android"))]
        {
            Arc::new(axagent_trajectory::SkillSandboxExecutor::with_default_policy())
        }
        #[cfg(target_os = "android")]
        {
            // Phantom: SkillState stores `Arc<()>` on Android. Bridge via Dummy.
            let _ = std::marker::PhantomData::<axagent_trajectory::SkillSandboxExecutor>;
            Arc::new(axagent_trajectory::SkillSandboxExecutor::with_default_policy())
        }
    };
    let file_authorizer = Arc::new(axagent_storage::file_authorizer::FileAuthorizer::new());
    // M3: 设置审计日志持久化路径
    file_authorizer.set_audit_log_path(app_dir.join("audit.log")).await;

    // ── 初始化 CredentialManager（AES-256-GCM 加密凭证存储） ──────────────
    let credential_store =
        axagent_credential::CredentialStore::new(app_dir.join("credentials"), master_key);
    let credential_manager = Arc::new(axagent_credential::CredentialManager::new(credential_store));
    let session_share_manager: crate::app_state::SessionShareStore =
        Arc::new(TokioRwLock::new(std::collections::HashMap::new()));
    let database_query_service: Arc<dyn axagent_harness::DatabaseQueryService> =
        Arc::new(crate::database_query_impl::SqlxDatabaseQueryService);
    #[cfg(not(mobile))]
    let pty_manager = Arc::new(axagent_runtime::pty::PtyManager::new());
    let sandbox_executor_field: SandboxExecutorField = {
        #[cfg(not(target_os = "android"))]
        {
            SandboxExecutorField::Real(sandbox_executor.clone())
        }
        #[cfg(target_os = "android")]
        {
            let _ = sandbox_executor; // silence unused
            SandboxExecutorField::Dummy
        }
    };
    let browser_client_field: BrowserClientField = {
        #[cfg(not(target_os = "android"))]
        {
            BrowserClientField::Real(browser_client.clone())
        }
        #[cfg(target_os = "android")]
        {
            let _ = browser_client; // silence unused
            BrowserClientField::Dummy
        }
    };

    // ── Construct the 6 domain sub-states (Phase 3 P1 Task 3.1) ──
    let infra_state = crate::state::InfraState::new(
        harness.clone(),
        vector_store_arc.clone(),
        Arc::new(tokio::sync::Semaphore::new(2)),
        file_authorizer.clone(),
        app_dir.clone(),
    );
    let gateway_state = crate::state::GatewayState::new(gateway_server.clone());
    let task_state = crate::state::TaskState::new(
        task_manager.clone(),
        auto_backup_handle.clone(),
        webdav_sync_handle.clone(),
        api_server_handle.clone(),
        trajectory_cleanup_handle.clone(),
        shutdown_token.clone(),
        close_to_tray.clone(),
        stream_cancel_flags.clone(),
        agent_permission_senders.clone(),
        agent_ask_senders.clone(),
        agent_always_allowed.clone(),
        agent_prompters.clone(),
        steer_queue.clone(),
    );
    let agent_state = crate::state::AgentState::new(
        agent_session_manager.clone(),
        agent_cancel_tokens.clone(),
        agent_paused.clone(),
        running_agents.clone(),
        reflector.clone(),
        platform_manager.clone(),
        platform_bridge.clone(),
        local_tool_registry.clone(),
        work_engine.clone(),
    );
    let memory_state = crate::state::MemoryState::new(
        shared_memory.clone(),
        sub_agent_registry.clone(),
        memory_service.clone(),
        nudge_service.clone(),
        closed_loop_service.clone(),
        shared_trajectory_storage.clone(),
        insight_system.clone(),
        realtime_learning.clone(),
        pattern_learner.clone(),
        cross_session_learner.clone(),
        rl_engine.clone(),
        batch_processor.clone(),
        auto_memory_extractor.clone(),
        parallel_execution_service.clone(),
        cron_job_store.clone(),
        user_profile.clone(),
        semantic_cache.clone(),
        prompt_cache.clone(),
        dream_consolidator.clone(),
        dream_data_provider.clone(),
        session_share_manager.clone(),
    );
    // ── 初始化 SkillLearningManager (技能学习闭环) ──
    let skill_learning_manager: Arc<TokioRwLock<axagent_trajectory::SkillLearningManager>> = {
        let config = axagent_trajectory::SkillLearningConfig::default();
        let manager = axagent_trajectory::SkillLearningManager::new(config);
        // 尝试从磁盘加载待处理审批操作（恢复进内存列表）
        if let Err(e) = manager.load_pending_operations_from_disk().await {
            tracing::warn!("Failed to load pending skill operations from disk: {}", e);
        }
        Arc::new(TokioRwLock::new(manager))
    };

    let skill_state = crate::state::SkillState::new(
        skill_evolution_engine.clone(),
        skill_proposal_service.clone(),
        skill_decomposer.clone(),
        skill_learning_manager.clone(),
        sandbox_executor_field,
        dashboard_registry.clone(),
        webhook_subscription_manager.clone(),
        plugin_manager.clone(),
        sync_engine.clone(),
        tot_sessions.clone(),
        planner_sessions.clone(),
        browser_client_field,
        constitution.clone(),
        proactive_service.clone(),
    );

    // ── M1: 新子状态分解 — 学习引擎与工具创建器 ──
    // 初始化 OPC 行业适配器注册表（P0-1-A：行业包驱动，替代 create_all_adapters 硬编码）
    let industry_registry = IndustryAdapterRegistry::new();
    // NOTE: opc_workflows 已随 AxAgent 清理移除，行业适配器暂空注册表
    // for adapter in crate::commands::opc_workflows::load_industry_adapters_from_packs(Some(&app_dir))
    // {
    //     industry_registry.register(adapter);
    // }
    let industry_adapter_registry = Arc::new(Mutex::new(industry_registry));

    // 初始化行业学习引擎（LLM 端口可选，未配置时使用规则回退；
    // RL 持久化存储已随 AxInvest 清理移除，使用内置内存存储）
    let industry_learning_engine = Arc::new(IndustryLearningEngine::new());

    let learning_state = LearningEngineState::new(
        text_grad_engine.clone(),
        intrinsic_motivation.clone(),
        coevolution_env.clone(),
        process_reward_model.clone(),
        industry_learning_engine,
        industry_adapter_registry,
    );
    let tool_state = ToolState::new(auto_tool_creator.clone());

    // 注册 MemoryRepository（给 MemoryFlush 等工具使用）
    axagent_harness::repositories::set_memory_repository(Arc::new(
        axagent_dao::memory_repository::DaoMemoryRepository::new(Arc::new(sea_db.clone())),
    ));

    // P0-OPT: 自进化闭环 namespace 创建 + Reflector 历史加载 推迟到后台
    // 这些是非关键路径初始化，不阻塞首帧渲染
    tracing::info!("[startup] Reflector Insights namespace + load_persistence 推迟到后台");

    // 2.7 P1:从持久化 settings 读取遥测级别初值,构造共享句柄。
    // `save_settings` 命令在用户修改级别后会更新此句柄;`FilteringSink`
    // 通过 `level_handle()` 引用同一 `Arc` 实现热更新。
    let telemetry_level =
        axagent_telemetry::TelemetryLevel::from_str_or_off(&app_settings.telemetry_level);
    let telemetry_level_handle = Arc::new(parking_lot::RwLock::new(telemetry_level));

    // 2.7 P1:构造生产 sink 链 — JsonlTelemetrySink(落盘) + FilteringSink(级别过滤)。
    //
    // `FilteringSink::new_with_handle` 与上面的 `telemetry_level_handle` 共享同一
    // `Arc<RwLock<TelemetryLevel>>`,因此 `save_settings` 更新 handle 时
    // sink 链立即响应(无需重建)。
    //
    // 落盘失败不阻断启动:用 `MemoryTelemetrySink` 兜底,事件至少保留在内存中
    // (供同进程内 SessionTracer 消费),并通过 warn 日志告知用户。
    let telemetry_sink: Arc<dyn axagent_telemetry::TelemetrySink> = {
        let jsonl_path = app_dir.join("telemetry.jsonl");
        let initialized_level = *telemetry_level_handle.read();
        match axagent_telemetry::JsonlTelemetrySink::new(&jsonl_path) {
            Ok(jsonl_sink) => {
                let filtering = axagent_telemetry::FilteringSink::new_with_handle(
                    Arc::new(jsonl_sink),
                    telemetry_level_handle.clone(),
                );
                tracing::info!(
                    "[telemetry] sink chain initialized: JsonlTelemetrySink({}) + FilteringSink(level={})",
                    jsonl_path.display(),
                    initialized_level
                );
                Arc::new(filtering)
            },
            Err(e) => {
                tracing::warn!(
                    "[telemetry] JsonlTelemetrySink init failed at {}: {} — falling back to MemoryTelemetrySink (events not persisted)",
                    jsonl_path.display(),
                    e
                );
                Arc::new(axagent_telemetry::MemoryTelemetrySink::default())
            },
        }
    };

    // 3.3 P2:构造 PersistentRunner(持久化重试调度器)。
    //
    // 使用默认配置(enabled: false),守护线程会空转不调度。
    // 未来用户通过配置启用后,守护线程立即开始检查 pending session。
    //
    // 注意:executor 闭包为占位实现,真正的 SessionManager 适配器需后续实现。
    let persistent_runner = {
        let config = axagent_runtime::persistent_runner::PersistentRunnerConfig::default();
        let runner =
            Arc::new(axagent_runtime::persistent_runner::PersistentRunner::new(&app_dir, config));
        tracing::info!(
            "[persistent_runner] 实例已构造(默认 enabled=false,守护线程将在 start_background_services 中启动)"
        );
        Some(runner)
    };

    // ── 能力发现系统初始化 ──────────────────────────────────────
    // 创建能力发现的嵌入提供者（真实嵌入服务，未配置时回退 Mock）
    let embedding_provider: Arc<dyn axagent_harness::rag_provider::EmbeddingProvider> =
        crate::capability_embedding::create_capability_embedding_provider(
            &sea_db,
            &master_key,
            &harness,
        )
        .await;

    // 创建能力索引器（具体实现）
    // Phase 1 反馈闭环：注入 DB 连接，护照读取时自动合并 capability_stats 执行统计
    let capability_indexer_impl = Arc::new(
        axagent_tools::CapabilityIndexerImpl::new(
            vector_store_arc.clone(),
            embedding_provider.clone(),
        )
        .with_db(sea_db.clone()),
    );

    // P0-OPT: 元数据恢复 + 护照批量注册移到后台异步，加速首帧显示
    tracing::debug!(
        "CapabilityIndexer created (metadata restore + passport registration deferred)"
    );

    // 转为 trait 对象供 Retriever 使用
    let capability_indexer_trait: Arc<dyn axagent_harness::CapabilityIndexer> =
        capability_indexer_impl.clone();
    // 注入 CapabilityView（渐进式披露 L1 定义层）— 必须在索引器构造之后，
    // 早期 init_extensions 调用点（本文件上方）索引器尚不存在。
    // 注入共享能力索引器 —— CapabilityView / CapabilityLoad / DiscoverSkills /
    // CapabilityBrowse 四个披露工具共用同一份（收敛前 view/load 各持一个 OnceLock）。
    axagent_tools::tools::capability_shared::set_capability_indexer(
        capability_indexer_trait.clone(),
    );
    // 注入 CapabilityLoad 的会话状态存储：加载动作依赖它落盘，供下轮 Processor 读回注入。
    axagent_tools::tools::capability_load::set_session_state_store(session_state_store.clone());
    // 注入 SaveAsWorkflow 的会话状态存储：持久化动作需要从这里读取已加载能力列表。
    axagent_tools::tools::save_as_workflow::set_session_state_store(session_state_store.clone());
    let capability_retriever = Arc::new(axagent_tools::CapabilityRetrieverImpl::new(
        vector_store_arc.clone(),
        embedding_provider.clone(),
        capability_indexer_trait.clone(),
    ));
    let capability_router = Arc::new(axagent_tools::capability_router_impl::build_default_router(
        capability_retriever.clone(),
        Some(sea_db.clone()),
    ));
    let capability_indexer = capability_indexer_impl;

    tracing::info!("[capability] 能力发现系统初始化完成");

    // P0-OPT: 能力护照注册 + 认知编排器初始化 + 主 DAG 加载全部移到后台异步，
    // 加速首帧显示（见 run_deferred_init）
    tracing::debug!("Cognitive router templates + main DAG load deferred to background");

    // ── 认知编排器初始化（三层路由树协调器） ──────────────────────
    // 全局用户消息唯一入口：L1 域路由 → L2 簇路由 → L3 RAR+图谱路由 → 执行模式决策。
    // L1/L2 用生产版规则实现；RAR 复用能力索引（与 capability_router 同源）；
    // 图谱初始为空，后续由能力注册/工作流模板同步填充。
    let domain_router: Arc<dyn axagent_harness::DomainRouter> =
        Arc::new(axagent_harness::DomainRouterImpl::new());
    let cluster_router: Arc<dyn axagent_harness::ClusterRouter> =
        Arc::new(axagent_harness::ClusterRouterImpl::new());
    let rar_router: Arc<dyn axagent_harness::RarRouter> = Arc::new(
        axagent_harness::DefaultRarRouter::new(
            embedding_provider.clone(),
            capability_indexer_trait.clone(),
        )
        .with_retriever(
            capability_retriever.clone() as Arc<dyn axagent_harness::CapabilityRetriever>
        ),
    );
    let workflow_graph: Arc<tokio::sync::RwLock<axagent_harness::WorkflowGraph>> =
        Arc::new(tokio::sync::RwLock::new(axagent_harness::WorkflowGraph::new()));
    // ── 双层决策：注入 L1 域路由 LLM 兜底推理器 ──
    // 规则未命中时调用默认提供商做轻量分类，输出合法域标识；
    // 任何失败回退 `None`（走纯规则 `route`），绝不断流。复用
    // `llm_helpers::chat_with_default_provider`（与 fleet/task_shape 同调用入口）。
    let l1_reasoner_harness = harness.clone();
    let l1_llm_reasoner: Arc<axagent_harness::LlmReasoner> = Arc::new(move |user_input: &str| {
        let harness = l1_reasoner_harness.clone();
        let input = user_input.to_string();
        Box::pin(async move {
            const SYS: &str = "你是 L1 域路由分类器。根据用户输入，从以下业务域标识中选最匹配的一个，\
                只输出该标识，不要解释、不要引号、不要标点：\
                general, devops, ai_media, data_analysis, content_creation, communication, finance, automation, system";
            let user = format!("用户输入：{input}");
            match axagent_runtime::llm_helpers::chat_with_default_provider(&harness, SYS, &user, 16)
                .await
            {
                Ok(text) => {
                    let candidate = text.trim().trim_matches('"').trim_matches('`');
                    if candidate.parse::<axagent_harness::CapabilityDomain>().is_ok() {
                        Some(candidate.to_string())
                    } else {
                        None
                    }
                },
                Err(e) => {
                    tracing::warn!(error = %e, "L1 域路由 LLM 兜底调用失败，回退纯规则路由");
                    None
                },
            }
        })
    });
    let cognitive_router: Arc<dyn axagent_harness::CognitiveRouter> = Arc::new(
        axagent_harness::DefaultCognitiveRouter::new(
            domain_router,
            cluster_router,
            rar_router,
            workflow_graph,
        )
        .with_llm_reasoner(l1_llm_reasoner),
    );

    tracing::info!("[cognitive] 认知编排器初始化完成");

    // ── T5A.3：进化产物执行统计存储（阶段四后置闭环）──
    // 与 EvolutionFeedbackSinkImpl 共享同一 Arc，注入 GeneratedToolAdapter 后按
    // (conversation_id, tool_id) 累计真实执行成败（D2 会话隔离），
    // 作为贝叶斯决策器的「真实执行证据」。
    let evolution_execution_stats: std::sync::Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<
                String,
                std::collections::HashMap<
                    String,
                    axagent_harness::workflow_evolution::ToolExecutionStats,
                >,
            >,
        >,
    > = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));

    // ── 阶段二 T2.3 + 阶段四 T4.3：注入自指工具访问器 ──
    // wiring 实现 EvolutionMutationAccess 持有 local_tool_registry 与 work_engine 的 Arc，
    // 使 system_evolution_* 工具可通过 RuntimeMutationAccess trait 访问运行时注册表，
    // 编排型进化产物在 deploy 时经 WorkEngineWorkflowDagExecutor 真正执行（分层执行）。
    // 必须在 work_engine 创建之后注入（此处 work_engine 已就绪）。
    // D3：一并注入数据库连接，使 deploy 生成的进化产物执行反馈可落库持久化（重启不丢）。
    axagent_tools::runtime_mutation::set_mutation_access(std::sync::Arc::new(
        crate::commands::evolution_engine::EvolutionMutationAccess::new(
            local_tool_registry.clone(),
            work_engine.clone(),
            evolution_execution_stats.clone(),
            harness.db().clone(),
        ),
    )
        as std::sync::Arc<dyn axagent_harness::RuntimeMutationAccess>);

    // ── 关键路径阻塞项：认知编排器模板初始化 ──
    // 必须在 create_app_state 同步阶段完成，不能放 run_deferred_init。
    // 否则 cognitive_query 先于 WorkEngine.load_workflow_template 执行时，
    // self.workflows HashMap 里没有 cognitive_router_main 模板，直接返回
    // WorkflowNotFound，前端看到"主动停止"。
    // 这两步都是纯 DB 操作（ensure 查/写 SQLite、load 读模板+HashMap insert），
    // 不涉及 LLM，耗时 <100ms，对首帧几乎无感。
    {
        let db = harness.db();
        if let Err(e) = crate::init::ensure_cognitive_router_templates(db).await {
            tracing::error!("[startup] 认知编排器模板初始化失败: {}", e);
        } else {
            tracing::info!("[startup] 认知编排器模板已写入 DB");
        }
        if let Err(e) =
            work_engine.load_workflow_template(crate::init::COGNITIVE_ROUTER_MAIN_ID).await
        {
            tracing::error!("[startup] 主 DAG 加载失败: {}", e);
        } else {
            tracing::info!("[startup] 主 DAG 已加载进 WorkEngine 内存");
        }
    }

    tracing::info!(
        elapsed = %t_start.elapsed().as_millis(),
        "[startup] create_app_state 关键路径完成（首帧可渲染）"
    );

    Ok(AppState {
        harness,
        gateway: gateway_server,
        close_to_tray,
        app_data_dir: app_dir.clone(),
        auto_backup_handle,
        webdav_sync_handle,
        api_server_handle,
        trajectory_cleanup_handle,
        task_manager,
        skill_watcher_shutdown: std::sync::OnceLock::new(),
        shutdown_token,
        vector_store: vector_store_arc,
        indexing_semaphore: Arc::new(tokio::sync::Semaphore::new(2)),
        stream_cancel_flags,
        agent_permission_senders,
        agent_ask_senders,
        agent_always_allowed,
        agent_prompters,
        agent_plan_approvals,
        evolution_consent_senders,
        pending_capability_gaps,
        agent_session_manager,
        agent_cancel_tokens,
        agent_paused,
        agent_pause_states,
        running_agents,
        steer_queue,
        reflector,
        mcp_agent_server,
        shared_memory,
        sub_agent_registry,
        memory_service: memory_service.clone(),
        nudge_service,
        closed_loop_service,
        trajectory_storage: shared_trajectory_storage,
        insight_system,
        realtime_learning,
        pattern_learner,
        cross_session_learner,
        rl_engine,
        batch_processor,
        skill_evolution_engine,
        skill_proposal_service,
        skill_learning_manager,
        auto_memory_extractor,
        parallel_execution_service,
        cron_job_store,
        cron_scheduler: Arc::new(tokio::sync::RwLock::new(None)),
        platform_manager,
        platform_bridge,
        user_profile,
        local_tool_registry,
        evolution_execution_stats,
        work_engine,
        scheduler_budget: Arc::new(tokio::sync::RwLock::new(
            crate::scheduler::gate::BudgetState::default(),
        )),
        skill_decomposer,
        proactive_service,
        dashboard_registry,
        webhook_subscription_manager,
        webhook_event_emitter,
        semantic_cache,
        prompt_cache,
        fleet_repository,
        fleet_intent_llm,
        tot_sessions,
        planner_sessions,
        browser_client,
        dream_consolidator,
        cost_aware_router,
        stream_reporter,
        text_grad_engine,
        auto_tool_creator,
        intrinsic_motivation,
        coevolution_env,
        constitution,
        process_reward_model,
        dream_data_provider,
        #[cfg(not(target_os = "android"))]
        sandbox_executor,
        #[cfg(target_os = "android")]
        sandbox_executor: Arc::new(()),
        sync_engine,
        device_sync_state,
        plugin_manager,
        file_authorizer,
        credential_manager,
        database_query_service,
        session_share_manager,
        #[cfg(not(mobile))]
        pty_manager,
        telemetry_level_handle,
        telemetry_sink,
        persistent_runner,
        event_bus,
        // 能力发现系统
        capability_router,
        capability_indexer,
        session_state_store,
        // 认知编排器
        cognitive_router,
        // 动态防护规则管理器
        prompt_guard: Arc::new(PatternPromptGuard::new()),
        // P3: 任务形态 LLM 兜底分类器 + 审批通道
        task_shape_llm_classifier,
        task_shape_approval_senders,
        // Phase 3 P1 Task 3.1: domain decomposition
        infra: infra_state,
        gateway_state,
        task: task_state,
        agent: agent_state,
        memory: memory_state,
        skill: skill_state,
        // M1: 新增学习与工具子状态
        learning: learning_state,
        tool: tool_state,
        // P0-4: 记忆写审批门（配置与待审批列表从磁盘恢复，P2-4 持久化）
        memory_write_approval_config: Arc::new(tokio::sync::RwLock::new(
            crate::commands::memory::load_memory_approval_config(),
        )),
        pending_memory_writes: Arc::new(tokio::sync::RwLock::new(
            crate::commands::memory::load_pending_memory_writes(),
        )),
    })
}

/// 自动收集并注册所有能力护照（工具/工作流/知识库/技能）
///
/// 在应用启动时调用，将已注册的工具、工作流模板、知识库、技能的能力护照
/// 批量索引到能力发现系统。
async fn register_all_capabilities(
    indexer: &Arc<axagent_tools::CapabilityIndexerImpl>,
    tool_registry: &Arc<tokio::sync::Mutex<axagent_tools::registry::UnifiedToolRegistry>>,
    db: &sea_orm::DatabaseConnection,
    skill_state: &crate::state::SkillState,
) {
    use axagent_harness::{
        CapabilityIndexer, CapabilityPassport, CapabilityPassportDto, PlanningComplexity,
        SecurityLevel, Visibility,
    };
    use std::collections::HashMap;

    let mut passports = Vec::new();

    // 1. 从工具注册表收集所有 ToolInfo 的护照
    {
        let registry = tool_registry.lock().await;
        for tool_info in registry.tools.list_all() {
            let mut passport = tool_info.to_passport_dto();
            // Phase 1.5 暴露闭环：工具护照绑定真实工具定义引用，
            // 主动模式（认知编排执行）命中该能力后凭 tool_ref 注入 chat_tools
            passport.tool_ref = Some(axagent_harness::CapabilityToolRef {
                tool_name: tool_info.name.clone(),
                registry: "builtin".to_string(),
            });
            passports.push(passport);
        }
    }

    // 2. 从工作流模板仓库收集
    match axagent_dao::repo::workflow_template::list_workflow_templates(db, None).await {
        Ok(templates) => {
            for model in &templates {
                let data = axagent_dao::repo::workflow_template::template_model_to_data(model);
                let passport = data.to_passport_dto();
                // 跳过系统预置模板（认知编排器等）：SystemOnly 护照禁止进入业务能力索引，
                // 避免被 RAR 检索 / 能力发现命中（结构隔离而非依赖熔断黑名单）。
                if passport.visibility.is_system_only() {
                    continue;
                }
                passports.push(passport);
            }
        },
        Err(e) => {
            tracing::warn!("[capability] 收集工作流模板护照失败: {}", e);
        },
    }

    // 3. 从知识库收集
    match axagent_dao::repo::knowledge::list_knowledge_bases(db).await {
        Ok(kbs) => {
            for kb in &kbs {
                passports.push(kb.to_passport_dto());
            }
        },
        Err(e) => {
            tracing::warn!("[capability] 收集知识库护照失败: {}", e);
        },
    }

    // 4. 技能：从 SkillState 缓存的 PluginManager 收集技能护照
    //    通过 plugin_registry_report() 容错加载（单个 SKILL.md 损坏不影响整体），
    //    技能归入 Core 域 + skill 集群，与工具/工作流并行参与 RAR 检索。
    //    仅收集内置（Builtin）技能；插件技能/Agent/声明能力统一由 4b 以
    //    Plugin 来源收集，避免启动收集与插件生命周期护照注册重复。
    {
        use axagent_harness::{
            CapabilityDomain, CapabilityEvolvability, CapabilityKind, CapabilityPassportDto,
            CapabilitySource, PlanningComplexity, SecurityLevel, Visibility,
        };
        let plugin_manager = skill_state.plugin_manager.read().await;
        if let Ok(report) = plugin_manager.plugin_registry_report() {
            for f in report.failures() {
                tracing::debug!("[capability] 技能加载失败（跳过护照注册）: {f}");
            }
            let plugins = report.into_registry_allowing_failures();
            for p in plugins.summaries() {
                let meta = &p.metadata;
                if meta.kind != axagent_plugins::PluginKind::Builtin {
                    continue;
                }
                // 从 plugin manager 取带 prompt_body 的 Skill 护照（读 SKILL.md 文件）
                let mut skill_prompt_body = None;
                let mut skill_description = meta.description.clone();
                for pp in plugin_manager.passports_for_plugin(&meta.id) {
                    if pp.kind == CapabilityKind::Skill && pp.name == meta.name {
                        skill_prompt_body = pp.prompt_body.clone();
                        if !pp.description.is_empty() {
                            skill_description = pp.description.clone();
                        }
                        break;
                    }
                }
                passports.push(CapabilityPassportDto {
                    capability_id: format!("skill:{}", meta.name),
                    name: meta.name.clone(),
                    description: skill_description,
                    summary: None,
                    kind: CapabilityKind::Skill,
                    version: None,
                    owner: None,
                    created_at: None,
                    updated_at: None,
                    domain: CapabilityDomain::General,
                    source: CapabilitySource::Builtin,
                    evolvable: CapabilityEvolvability::Local,
                    sub_category: "skill".to_string(),
                    visibility: Visibility::Public,
                    caller_permissions: Default::default(),
                    input_schema: None,
                    output_schema: None,
                    implementation: None,
                    tags: vec!["skill".to_string(), meta.source.clone()],
                    negative_scenarios: vec![],
                    security_level: SecurityLevel::Public,
                    modality_support: Default::default(),
                    output_capabilities: Default::default(),
                    estimated_cost_usd: None,
                    avg_duration_seconds: None,
                    execution_mode: axagent_harness::capability::ExecutionMode::Sync,
                    timeout_ms: None,
                    planning_complexity: PlanningComplexity::Simple,
                    model_iq_requirement: 0,
                    experiment_group: None,
                    agent_profile_id: None,
                    stats: Default::default(),
                    level: axagent_harness::CapabilityLevel::L1,
                    enabled: true,
                    exposure: axagent_harness::CapabilityExposure::Auto,
                    tool_ref: None,
                    aliases: Vec::new(),
                    steps: Vec::new(),
                    skill_steps: Vec::new(),
                    placeholders: Vec::new(),
                    prompt_body: skill_prompt_body,
                    template_body: None,
                    instantiates_to: None,
                    example_instance: None,
                    upstream: Vec::new(),
                    downstream: Vec::new(),
                    preconditions: Vec::new(),
                    attached_snippets: Vec::new(),
                });
            }
        }
    }

    // 4b. 插件护照：非内置插件的技能 + Agent + 声明能力，统一以 Plugin 来源收集。
    //     运行时启用/禁用/卸载由命令层增量同步（`passports_for_plugin`），
    //     启动时在此一次性收集，保证与插件生命周期注册的护照 ID 完全一致。
    {
        use axagent_harness::CapabilitySource;
        let plugin_manager = skill_state.plugin_manager.read().await;
        if let Ok(report) = plugin_manager.plugin_registry_report() {
            for p in report.into_registry_allowing_failures().summaries() {
                if p.metadata.kind == axagent_plugins::PluginKind::Builtin {
                    continue;
                }
                let plugin_id = &p.metadata.id;
                for passport in plugin_manager.passports_for_plugin(plugin_id) {
                    // 双保险：仅收集明确标记为插件来源的护照
                    if passport.source == CapabilitySource::Plugin {
                        passports.push(passport);
                    }
                }
            }
        }
    }

    // 5. 专家护照：让 AgentProfile 本身具备能力被发现（认知编排 Ask/Act/Delegate 可命中）。
    //    agent_profile_id 直接指向该专家，命中后 agent_query 按此专家执行。
    //    同时维护 role -> 默认专家 映射，供角色护照反查执行专家。
    let mut role_default_profiles: std::collections::HashMap<String, String> = HashMap::new();
    match axagent_dao::repo::agent_profile::list_agent_profiles(db, None).await {
        Ok(profiles) => {
            for p in &profiles {
                if !p.is_enabled {
                    continue;
                }
                if let Some(role) = &p.agent_role {
                    role_default_profiles.entry(role.clone()).or_insert_with(|| p.id.clone());
                }
                let mut tags = p.tags.clone();
                if !p.category.is_empty() {
                    tags.push(p.category.clone());
                }
                if let Some(role) = &p.agent_role {
                    tags.push(format!("role:{role}"));
                }
                if let Some(eid) = &p.expert_id {
                    tags.push(format!("expert:{eid}"));
                }
                passports.push(CapabilityPassportDto {
                    capability_id: format!("agent:{}", p.id),
                    name: p.name.clone(),
                    description: p.description.clone().unwrap_or_default(),
                    summary: None,
                    kind: axagent_harness::CapabilityKind::Agent,
                    version: None,
                    owner: None,
                    created_at: None,
                    updated_at: None,
                    domain: infer_profile_domain(&p.category, &p.name),
                    sub_category: if p.category.is_empty() {
                        "agent_profile".to_string()
                    } else {
                        p.category.clone()
                    },
                    visibility: Visibility::Public,
                    caller_permissions: Default::default(),
                    input_schema: None,
                    output_schema: None,
                    implementation: None,
                    tags,
                    negative_scenarios: vec![],
                    security_level: SecurityLevel::Public,
                    modality_support: Default::default(),
                    output_capabilities: Default::default(),
                    estimated_cost_usd: None,
                    avg_duration_seconds: None,
                    execution_mode: axagent_harness::capability::ExecutionMode::Sync,
                    timeout_ms: None,
                    planning_complexity: PlanningComplexity::Simple,
                    model_iq_requirement: 0,
                    experiment_group: None,
                    agent_profile_id: Some(p.id.clone()),
                    stats: Default::default(),
                    level: axagent_harness::CapabilityLevel::L1,
                    source: axagent_harness::CapabilitySource::Builtin,
                    evolvable: axagent_harness::CapabilityEvolvability::Local,
                    enabled: true,
                    exposure: axagent_harness::CapabilityExposure::Auto,
                    tool_ref: None,
                    aliases: Vec::new(),
                    steps: Vec::new(),
                    skill_steps: Vec::new(),
                    placeholders: Vec::new(),
                    prompt_body: None,
                    template_body: None,
                    instantiates_to: None,
                    example_instance: None,
                    upstream: Vec::new(),
                    downstream: Vec::new(),
                    preconditions: Vec::new(),
                    attached_snippets: Vec::new(),
                });
            }
        },
        Err(e) => {
            tracing::warn!("[capability] 收集专家护照失败: {}", e);
        },
    }

    // 6. 角色护照：让 AgentRole 本身具备能力被发现。
    //    角色无独立 agent_profile_id，反查其关联的默认专家（profile.agent_role = role.id）作为执行专家；
    // 角色护照的执行载体：优先复用已关联该角色的 AgentProfile（role_default_profiles）；
    // 角色独立存在（无任何 profile 关联）时，参照 ensure_agent_profile 先例自动补齐一个
    // 绑定该角色（agent_role）的最小 AgentProfile，作为能力命中的执行载体（幂等，可复用）。
    // 否则角色护照 agent_profile_id 为空，命中后 agent_query 走默认执行，角色配置会丢失。
    match axagent_dao::repo::agent_role::list_agent_roles(db, None).await {
        Ok(roles) => {
            for r in &roles {
                let mut exec_profile = role_default_profiles.get(&r.id).cloned();
                if exec_profile.is_none() {
                    let bridge_id = format!("role-bridge:{}", r.id);
                    match axagent_dao::repo::agent_profile::get_agent_profile(db, &bridge_id).await
                    {
                        // 已存在（幂等复用）
                        Ok(_) => {
                            exec_profile = Some(bridge_id);
                        },
                        // 不存在 → 创建绑定该角色的最小执行载体
                        Err(axagent_harness::AxAgentError::NotFound(_)) => {
                            match axagent_dao::repo::agent_profile::create_agent_profile(
                                db,
                                &bridge_id,
                                &r.name,
                                r.description.as_deref(),
                                "general",
                                "👤",
                                Some(&r.id),
                                "role-bridge",
                                &[],
                            )
                            .await
                            {
                                Ok(_) => {
                                    exec_profile = Some(bridge_id);
                                },
                                Err(e) => {
                                    tracing::warn!(
                                        role = %r.id,
                                        error = %e,
                                        "[capability] 为角色自动补齐最小 AgentProfile 失败，角色命中后将降级默认执行"
                                    );
                                },
                            }
                        },
                        Err(e) => {
                            tracing::warn!(
                                role = %r.id,
                                error = %e,
                                "[capability] 检查角色执行载体失败"
                            );
                        },
                    }
                }

                let mut tags = vec![r.name.clone()];
                for d in &r.active_domains {
                    tags.push(format!("domain:{d}"));
                }
                passports.push(CapabilityPassportDto {
                    capability_id: format!("agent_role:{}", r.id),
                    name: r.name.clone(),
                    description: r.description.clone().unwrap_or_default(),
                    summary: None,
                    kind: axagent_harness::CapabilityKind::Agent,
                    version: None,
                    owner: None,
                    created_at: None,
                    updated_at: None,
                    domain: infer_role_domain(&r.active_domains),
                    sub_category: "agent_role".to_string(),
                    visibility: Visibility::Public,
                    caller_permissions: Default::default(),
                    input_schema: None,
                    output_schema: None,
                    implementation: None,
                    tags,
                    negative_scenarios: vec![],
                    security_level: SecurityLevel::Public,
                    modality_support: Default::default(),
                    output_capabilities: Default::default(),
                    estimated_cost_usd: None,
                    avg_duration_seconds: None,
                    execution_mode: axagent_harness::capability::ExecutionMode::Sync,
                    timeout_ms: None,
                    planning_complexity: PlanningComplexity::Simple,
                    model_iq_requirement: 0,
                    experiment_group: None,
                    agent_profile_id: exec_profile,
                    stats: Default::default(),
                    level: axagent_harness::CapabilityLevel::L1,
                    source: axagent_harness::CapabilitySource::Builtin,
                    evolvable: axagent_harness::CapabilityEvolvability::Local,
                    enabled: true,
                    exposure: axagent_harness::CapabilityExposure::Auto,
                    tool_ref: None,
                    aliases: Vec::new(),
                    steps: Vec::new(),
                    skill_steps: Vec::new(),
                    placeholders: Vec::new(),
                    prompt_body: None,
                    template_body: None,
                    instantiates_to: None,
                    example_instance: None,
                    upstream: Vec::new(),
                    downstream: Vec::new(),
                    preconditions: Vec::new(),
                    attached_snippets: Vec::new(),
                });
            }
        },
        Err(e) => {
            tracing::warn!("[capability] 收集角色护照失败: {}", e);
        },
    }

    if passports.is_empty() {
        tracing::warn!("[capability] 未发现任何能力护照，能力发现系统将以空索引启动");
        return;
    }

    let total = passports.len();
    let results = indexer.index_batch(&passports).await;
    let success = results.iter().filter(|r| r.success).count();
    tracing::info!("[capability] 自动注册 {} 个能力护照，成功 {}", total, success);

    // 5. 注册系统级能力（CognitiveRouter 编排器等）
    let system_passports = register_system_capabilities(indexer).await;

    // P2-①：护照声明的 upstream/downstream 关系物化到 capability_relationships 表
    //（统一能力模型第四层 CapabilityRelationship 的物化镜像，供关系查询/审计）
    let mut all_passports = passports;
    all_passports.extend(system_passports);
    match axagent_dao::repo::capability_relationship::sync_from_passports(db, &all_passports).await
    {
        Ok(n) => {
            tracing::info!("[capability] 物化 {} 条能力关系到 capability_relationships", n);
        },
        Err(e) => {
            tracing::warn!(
                "[capability] 能力关系物化失败（不影响检索，护照内联一跳仍可用）: {}",
                e
            );
        },
    }
}

/// 从字符串推断能力域。优先精确匹配 CapabilityDomain 序列化名，再做领域关键词兜底。
fn domain_from_keyword(s: &str) -> Option<axagent_harness::CapabilityDomain> {
    use axagent_harness::CapabilityDomain;
    let lower = s.to_lowercase();
    if let Ok(d) = <CapabilityDomain as std::str::FromStr>::from_str(&lower) {
        return Some(d);
    }
    match lower.as_str() {
        // 运维 / 安全 / 基础设施
        "devops" | "infrastructure" | "security" | "engineering" | "backend" | "frontend"
        | "sre" | "cicd" | "deployment" | "development" => Some(CapabilityDomain::Devops),
        // 数据分析
        "data" | "analysis" | "analytics" | "sql" | "etl" | "bi" | "statistics" => {
            Some(CapabilityDomain::DataAnalysis)
        },
        // 内容创作
        "writing" | "content" | "design" | "marketing" | "copywriting" | "ui" | "ux"
        | "creative" | "translation" => Some(CapabilityDomain::ContentCreation),
        // 通信
        "communication" | "im" | "email" | "messaging" | "collaboration" => {
            Some(CapabilityDomain::Communication)
        },
        // 金融
        "finance" | "invest" | "quant" | "trading" | "banking" | "stock" => {
            Some(CapabilityDomain::Finance)
        },
        // 自动化
        "automation" | "opc" | "rpa" | "workflow" | "orchestration" => {
            Some(CapabilityDomain::Automation)
        },
        // AI 媒体
        "media" | "image" | "video" | "audio" | "sound" | "generation" => {
            Some(CapabilityDomain::AiMedia)
        },
        _ => None,
    }
}

/// 推断专家护照的域：优先 category，其次 name 关键词，兜底 General。
fn infer_profile_domain(category: &str, name: &str) -> axagent_harness::CapabilityDomain {
    domain_from_keyword(category)
        .or_else(|| domain_from_keyword(name))
        .unwrap_or(axagent_harness::CapabilityDomain::General)
}

/// 推断角色护照的域：扫描 active_domains 首个命中，兜底 General。
fn infer_role_domain(active_domains: &[String]) -> axagent_harness::CapabilityDomain {
    active_domains
        .iter()
        .find_map(|d| domain_from_keyword(d))
        .unwrap_or(axagent_harness::CapabilityDomain::General)
}

/// 注册系统级能力到系统注册表
///
/// 系统能力具有以下特征：
/// - visibility 为 SystemOnly，不可被用户发现
/// - domain 为 System，属于系统域
/// - 用于内部编排和基础设施服务
async fn register_system_capabilities(
    indexer: &Arc<axagent_tools::CapabilityIndexerImpl>,
) -> Vec<axagent_harness::CapabilityPassportDto> {
    use axagent_harness::{
        CallerPermissions, CapabilityDomain, CapabilityEvolvability, CapabilityIndexer,
        CapabilityKind, CapabilityPassportDto, CapabilitySource, OutputCapabilities,
        PlanningComplexity, SecurityLevel, Visibility,
    };

    let mut system_passports = vec![
        // CognitiveRouter — 三层路由编排器
        CapabilityPassportDto {
            capability_id: "system_cognitive_router".to_string(),
            name: "认知路由编排器".to_string(),
            description: "三层路由编排器（L1域→L2簇→L3能力），负责将用户查询路由到正确的能力"
                .to_string(),
            summary: None,
            kind: CapabilityKind::Workflow,
            version: None,
            owner: None,
            created_at: None,
            updated_at: None,
            domain: CapabilityDomain::System,
            sub_category: "cognitive_routing".to_string(),
            visibility: Visibility::SystemOnly,
            caller_permissions: CallerPermissions::new(),
            input_schema: None,
            output_schema: None,
            implementation: None,
            tags: vec![
                "system".to_string(),
                "router".to_string(),
                "cognitive".to_string(),
                "layered".to_string(),
            ],
            negative_scenarios: vec![],
            security_level: SecurityLevel::Public,
            modality_support: Default::default(),
            output_capabilities: OutputCapabilities::default(),
            estimated_cost_usd: Some(0.0),
            avg_duration_seconds: Some(0.1),
            execution_mode: axagent_harness::capability::ExecutionMode::Sync,
            timeout_ms: None,
            planning_complexity: PlanningComplexity::Complex,
            model_iq_requirement: 85,
            experiment_group: None,
            agent_profile_id: None,
            stats: Default::default(),
            level: axagent_harness::CapabilityLevel::L1,
            enabled: true,
            exposure: axagent_harness::CapabilityExposure::Auto,
            tool_ref: None,
            aliases: Vec::new(),
            steps: Vec::new(),
            skill_steps: Vec::new(),
            placeholders: Vec::new(),
            prompt_body: None,
            template_body: None,
            instantiates_to: None,
            example_instance: None,
            upstream: Vec::new(),
            downstream: Vec::new(),
            preconditions: Vec::new(),
            attached_snippets: Vec::new(),
            source: CapabilitySource::Builtin,
            evolvable: CapabilityEvolvability::Local,
        },
        // LayeredPromptEngine — 分层 Prompt 引擎
        CapabilityPassportDto {
            capability_id: "system_layered_prompt_engine".to_string(),
            name: "分层Prompt引擎".to_string(),
            description: "按Domain/Cluster/Capability/Context四层注入Prompt片段，支持Token预算管理"
                .to_string(),
            summary: None,
            kind: CapabilityKind::Tool,
            version: None,
            owner: None,
            created_at: None,
            updated_at: None,
            domain: CapabilityDomain::System,
            sub_category: "prompt_engine".to_string(),
            visibility: Visibility::SystemOnly,
            caller_permissions: CallerPermissions::new(),
            input_schema: None,
            output_schema: None,
            implementation: None,
            tags: vec![
                "system".to_string(),
                "prompt".to_string(),
                "engine".to_string(),
                "layered".to_string(),
            ],
            negative_scenarios: vec![],
            security_level: SecurityLevel::Public,
            modality_support: Default::default(),
            output_capabilities: OutputCapabilities::default(),
            estimated_cost_usd: Some(0.0),
            avg_duration_seconds: Some(0.05),
            execution_mode: axagent_harness::capability::ExecutionMode::Sync,
            timeout_ms: None,
            planning_complexity: PlanningComplexity::Simple,
            model_iq_requirement: 0,
            experiment_group: None,
            agent_profile_id: None,
            stats: Default::default(),
            level: axagent_harness::CapabilityLevel::L1,
            enabled: true,
            exposure: axagent_harness::CapabilityExposure::Auto,
            tool_ref: None,
            aliases: Vec::new(),
            steps: Vec::new(),
            skill_steps: Vec::new(),
            placeholders: Vec::new(),
            prompt_body: None,
            template_body: None,
            instantiates_to: None,
            example_instance: None,
            upstream: Vec::new(),
            downstream: Vec::new(),
            preconditions: Vec::new(),
            attached_snippets: Vec::new(),
            source: CapabilitySource::Builtin,
            evolvable: CapabilityEvolvability::Local,
        },
    ];

    // 自指工具（阶段二 T2.3）：system_evolution_* 的 SYSTEM_ONLY 护照。
    // 走 system_* 系统能力通道，业务能力检索（L2/RAR）不可达，自指熔断由路由层兜底。
    let self_evolution_tools: [(&str, &str, &str); 4] = [
        (
            "inspect",
            "自指检查",
            "检查当前运行时已注册的进化能力状态（工具/工作流/技能），供系统进化流程调用",
        ),
        ("define", "自指定义", "定义新工具（仅生成工具定义，不注册），供系统进化流程审查"),
        ("deploy", "自指部署", "部署（注册）工具到运行时注册表，注册后系统流程立即可调用"),
        (
            "undeploy",
            "自指卸载",
            "卸载运行时注册的工具（仅允许 runtime_tool_sources 中登记的工具）",
        ),
    ];
    for (suffix, name, description) in self_evolution_tools {
        system_passports.push(CapabilityPassportDto {
            capability_id: format!("system:self_evolution:{suffix}"),
            name: name.to_string(),
            description: description.to_string(),
            summary: None,
            kind: CapabilityKind::Tool,
            version: None,
            owner: None,
            created_at: None,
            updated_at: None,
            domain: CapabilityDomain::System,
            sub_category: "self_evolution".to_string(),
            visibility: Visibility::SystemOnly,
            caller_permissions: CallerPermissions::new(),
            input_schema: None,
            output_schema: None,
            implementation: None,
            tags: vec!["SYSTEM_ONLY".to_string(), "META".to_string(), "self_evolution".to_string()],
            negative_scenarios: vec![],
            security_level: SecurityLevel::Restricted,
            modality_support: Default::default(),
            output_capabilities: OutputCapabilities::default(),
            estimated_cost_usd: Some(0.0),
            avg_duration_seconds: Some(0.01),
            execution_mode: axagent_harness::capability::ExecutionMode::Sync,
            timeout_ms: None,
            planning_complexity: PlanningComplexity::Simple,
            model_iq_requirement: 0,
            experiment_group: None,
            agent_profile_id: None,
            stats: Default::default(),
            level: axagent_harness::CapabilityLevel::L1,
            source: axagent_harness::CapabilitySource::Builtin,
            evolvable: axagent_harness::CapabilityEvolvability::None,
            enabled: true,
            exposure: axagent_harness::CapabilityExposure::Auto,
            tool_ref: None,
            aliases: Vec::new(),
            steps: Vec::new(),
            skill_steps: Vec::new(),
            placeholders: Vec::new(),
            prompt_body: None,
            template_body: None,
            instantiates_to: None,
            example_instance: None,
            upstream: Vec::new(),
            downstream: Vec::new(),
            preconditions: Vec::new(),
            attached_snippets: Vec::new(),
        });
    }

    let results = indexer.index_batch(&system_passports).await;
    let success = results.iter().filter(|r| r.success).count();
    tracing::info!(
        "[system_registry] 注册 {} 个系统能力，成功 {}",
        system_passports.len(),
        success
    );
    system_passports
}

async fn create_sync_engine(
    _sea_db: &sea_orm::DatabaseConnection,
    _app_settings: &axagent_harness::types::AppSettings,
) -> Option<Arc<SyncEngine>> {
    let cloud_config = load_cloud_storage_config(_sea_db, _app_settings).await?;
    let backend = cloud_config.create_backend().ok()?;
    let device_id = hostname_or_uuid();
    let profile_name = cloud_config.profile_name.clone();
    Some(Arc::new(SyncEngine::new(backend, &profile_name, &device_id)))
}

async fn load_cloud_storage_config(
    sea_db: &sea_orm::DatabaseConnection,
    _app_settings: &axagent_harness::types::AppSettings,
) -> Option<CloudStorageConfig> {
    use axagent_storage::cloud_storage::{BackendType, S3Config, S3ProviderPreset, SyncMode};
    let settings = axagent_dao::repo::settings::get_settings(sea_db).await.ok()?;

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
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or(S3ProviderPreset::Custom),
        backend_type,
        sync_enabled: true,
        sync_mode: SyncMode::Sync,
        profile_name: settings.sync_profile_name.clone().unwrap_or_else(|| "default".to_string()),
        webdav: settings.webdav_host.as_ref().map(|h| {
            axagent_storage::cloud_storage::WebDavConfig {
                host: h.clone(),
                username: settings.webdav_username.clone().unwrap_or_default(),
                password: settings.webdav_password.clone().unwrap_or_default(),
                path: settings.webdav_path.clone().unwrap_or_default(),
                accept_invalid_certs: settings.webdav_accept_invalid_certs,
            }
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

fn hostname_or_uuid() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

// ── P2-7: Webhook 订阅持久化 DAO 实现 ──────────────────────────────────────
//
// 把 WebhookSubscriptionManager 的订阅列表序列化为 JSON，
// 通过 settings 表的 KV 接口存储（key = "webhook_subscriptions"）。
// 启动时调用 `with_persistence` 自动从 DB 恢复所有订阅到内存。

const WEBHOOK_PERSIST_KEY: &str = "webhook_subscriptions";

#[derive(Debug)]
struct DbWebhookPersistence {
    db: sea_orm::DatabaseConnection,
}

#[async_trait::async_trait]
impl axagent_harness::WebhookPersistence for DbWebhookPersistence {
    async fn load(&self) -> Result<Vec<axagent_harness::WebhookSubscription>, String> {
        match axagent_dao::repo::settings::get_setting(&self.db, WEBHOOK_PERSIST_KEY).await {
            Ok(Some(json_str)) => {
                let subs: Vec<axagent_harness::WebhookSubscription> =
                    serde_json::from_str(&json_str)
                        .map_err(|e| format!("解析 webhook_subscriptions 失败: {}", e))?;
                Ok(subs)
            },
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(format!("读取 webhook_subscriptions 失败: {}", e)),
        }
    }

    async fn save(
        &self,
        subscriptions: &[axagent_harness::WebhookSubscription],
    ) -> Result<(), String> {
        let json_str = serde_json::to_string(subscriptions)
            .map_err(|e| format!("序列化 webhook_subscriptions 失败: {}", e))?;
        axagent_dao::repo::settings::set_setting(&self.db, WEBHOOK_PERSIST_KEY, &json_str)
            .await
            .map_err(|e| format!("保存 webhook_subscriptions 失败: {}", e))
    }
}

// ── 后台延迟初始化 ──────────────────────────────────────────────────
// P0-OPT: 以下重型操作已从 create_app_state 移到此处，通过异步任务在后台执行。
// 目的是将 Tauri setup 回调时间从数秒降低到数百毫秒，加速首帧显示。

/// 后台延迟初始化入口 — 在 AppState 构造完成后异步执行。
///
/// 被推迟的操作：
/// 1. MemoryService FTS5 索引构建（最耗时）
/// 2. SkillEvolutionEngine LLM 变异器注入（需 DB 查询）
/// 3. WorkflowEvolver LLM 变异器注入（需 DB 查询）
/// 4. CapabilityIndexer 元数据恢复（向量存储读取）
/// 5. 能力护照批量注册 register_all_capabilities（多表查询 + 向量索引）
/// 6. 认知编排器工作流模板初始化
/// 7. WorkEngine 主 DAG 加载
pub async fn run_deferred_init(app_state: &crate::app_state::AppState) {
    tracing::info!("[startup] 开始后台延迟初始化（首帧已显示）...");
    let t0 = std::time::Instant::now();

    // ── 1. MemoryService FTS5 初始化 ──
    match app_state.memory_service.write().await.initialize().await {
        Ok(_) => {
            tracing::info!(elapsed = %t0.elapsed().as_millis(), "[startup] MemoryService FTS5 完成")
        },
        Err(e) => tracing::warn!("[startup] MemoryService FTS5 失败（不阻塞）: {}", e),
    }

    // ── 2. CapabilityIndexer 元数据恢复 ──
    match app_state.capability_indexer.restore_metadata_from_store().await {
        Ok(_) => tracing::info!("[startup] 能力索引元数据恢复完成"),
        Err(e) => tracing::warn!("[startup] 能力索引元数据恢复失败: {}", e),
    }

    // ── 3. register_all_capabilities（最重型操作） ──
    register_all_capabilities(
        &app_state.capability_indexer,
        &app_state.local_tool_registry,
        &app_state.harness.db(),
        &app_state.skill,
    )
    .await;

    // ── 4. SkillEvolutionEngine LLM 注入 ──
    #[cfg(not(target_os = "android"))]
    {
        let master_key = app_state.harness.master_key();
        let harness_registry = app_state.harness.provider_registry();
        if let Some(bridge) = axagent_runtime::llm_bridge::build_llm_bridge_from_db_with(
            master_key,
            harness_registry,
            None,
            None,
        )
        .await
        {
            let engine = app_state.skill_evolution_engine.lock().await;
            let provider: std::sync::Arc<
                dyn axagent_harness::trajectory_types::LlmEvolutionProvider,
            > = std::sync::Arc::new(bridge);
            engine.set_llm_provider(provider).await;
            tracing::info!("[startup] SkillEvolutionEngine LLM 注入完成");
        }
    }

    // ── 5. WorkflowEvolver LLM 注入 ──
    {
        let master_key = app_state.harness.master_key();
        let harness_registry = app_state.harness.provider_registry();
        if let Some(bridge) = axagent_runtime::llm_bridge::build_llm_bridge_from_db_with(
            master_key,
            harness_registry,
            None,
            None,
        )
        .await
        {
            let mutator = super::workflow_injections::ProviderWorkflowLlmMutator::new(bridge);
            // 进化器经 workflow.evolver 能力接缝获取（与 WorkEngine / 命令层同一 Arc）
            if let Some(evolver) = axagent_harness::get_capability_registry().get_workflow_evolver()
            {
                if let Err(e) = evolver
                    .set_llm_provider(std::sync::Arc::new(mutator)
                        as std::sync::Arc<dyn axagent_harness::WorkflowLlmMutator>)
                    .await
                {
                    tracing::warn!("[startup] WorkflowEvolver LLM 注入失败: {e}");
                } else {
                    tracing::info!("[startup] WorkflowEvolver LLM 注入完成");
                }
            } else {
                tracing::warn!("[startup] workflow.evolver 接缝未注册，跳过 LLM 变异器注入");
            }
        }
    }

    // 注意：认知编排器模板初始化 + 主 DAG 加载已提升到 create_app_state 同步阶段
    // （见 state.rs L1237-1258），避免 deferred_init fire-and-forget 导致的竞态窗口。

    // ── 8. SemanticCache 真实文件缓存替换 ──
    // P0-OPT: 用真实文件 SQLite 替换启动时的内存占位符
    {
        let t_sc = std::time::Instant::now();
        let db = app_state.harness.db();
        match SemanticCache::new(db.clone(), CacheConfig::default()).await {
            Ok(real_cache) => {
                let mut sc_state = app_state.semantic_cache.lock().await;
                sc_state.cache = Arc::new(real_cache);
                tracing::info!(
                    "[startup] SemanticCache 文件缓存替换完成 ({}ms)",
                    t_sc.elapsed().as_millis()
                );
            },
            Err(e) => {
                tracing::warn!("[startup] SemanticCache 文件缓存初始化失败 (保持内存占位): {}", e);
            },
        }
    }

    // ── 9. CostAwareRouter 历史加载 ──
    {
        let t_router = std::time::Instant::now();
        match app_state.cost_aware_router.load_from_db().await {
            Ok(_) => tracing::info!(
                "[startup] CostAwareRouter 历史加载完成 ({}ms)",
                t_router.elapsed().as_millis()
            ),
            Err(e) => tracing::warn!("[startup] CostAwareRouter 历史加载失败: {}", e),
        }
    }

    // ── 10. Reflector Insights namespace 创建 + 历史加载 ──
    {
        let t_ref = std::time::Instant::now();
        let db = app_state.harness.db();
        const REFLECTOR_INSIGHTS_NS_NAME: &str = "Reflector Insights";
        if let Ok(list) = axagent_dao::repo::memory::list_namespaces(&db).await {
            let exists = list.iter().any(|ns| ns.name == REFLECTOR_INSIGHTS_NS_NAME);
            if !exists {
                match axagent_dao::repo::memory::create_namespace(
                    &db,
                    axagent_harness::types::CreateMemoryNamespaceInput {
                        name: REFLECTOR_INSIGHTS_NS_NAME.to_string(),
                        scope: "global".to_string(),
                        embedding_provider: None,
                        embedding_dimensions: None,
                        retrieval_threshold: None,
                        retrieval_top_k: None,
                        icon_type: Some("bulb".to_string()),
                        icon_value: None,
                    },
                )
                .await
                {
                    Ok(ns) => tracing::info!(
                        "[startup] created Reflector Insights namespace: id={}",
                        ns.id
                    ),
                    Err(e) => tracing::warn!(
                        "[startup] failed to create Reflector Insights namespace: {}",
                        e
                    ),
                }
            }
        }

        // Reflector 历史加载
        match app_state.reflector.load_persistence().await {
            Ok(n) => tracing::info!(
                "[startup] Reflector 加载 {n} 条历史反思 ({}ms)",
                t_ref.elapsed().as_millis()
            ),
            Err(e) => tracing::warn!("[startup] Reflector load_persistence 失败: {}", e),
        }
    }

    // ── 11. 工具注册表启用状态加载 ──
    {
        let t_reg = std::time::Instant::now();
        let mut registry = app_state.local_tool_registry.lock().await;
        registry.load_enabled_state(&app_state.harness.db()).await;
        tracing::info!("[startup] 工具注册表启用状态加载完成 ({}ms)", t_reg.elapsed().as_millis());
    }

    // ── 12. WorkflowEvolutionTick 后台定时优化 ──
    {
        let db_arc: std::sync::Arc<axagent_harness::DatabaseConnection> =
            std::sync::Arc::new(app_state.harness.db().clone());
        let storage =
            std::sync::Arc::new(axagent_trajectory::TrajectoryStorage::new(db_arc.clone()));
        let optimizer: std::sync::Arc<dyn axagent_harness::WorkflowOptimizer> =
            std::sync::Arc::new(axagent_trajectory::WorkflowOptimizerImpl::new());
        let template_repo: std::sync::Arc<dyn axagent_harness::WorkflowTemplateRepo> =
            std::sync::Arc::new(axagent_dao::DaoWorkflowTemplateRepository { db: db_arc.clone() });

        axagent_trajectory::start_workflow_evolution_tick(
            storage,
            optimizer,
            Some(template_repo),
            axagent_trajectory::EvolutionTickConfig::default(),
        );
        tracing::info!("[startup] WorkflowEvolutionTick 已启动（默认 6h 间隔）");
    }

    tracing::info!(
        elapsed = %t0.elapsed().as_millis(),
        "[startup] 后台延迟初始化全部完成"
    );
}

// ── P0-3: DbSessionEventSink ──────────────────────────────────────────────

use async_trait::async_trait;
use sea_orm::ConnectionTrait;

struct DbSessionEventSink {
    db: sea_orm::DatabaseConnection,
}

impl DbSessionEventSink {
    fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }

    /// 查询某 session 当前最大 seq，返回 +1。
    async fn next_seq(&self, session_id: &str) -> i64 {
        let row = self.db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                format!(
                    "SELECT COALESCE(MAX(seq), 0) + 1 AS next_seq FROM session_events WHERE session_id = '{session_id}'"
                ),
            ))
            .await;
        match row.ok().flatten() {
            Some(r) => r.try_get::<i64>("", "next_seq").unwrap_or(1),
            None => 1,
        }
    }
}

#[async_trait]
impl axagent_harness::SessionEventSink for DbSessionEventSink {
    async fn emit(
        &self,
        session_id: &str,
        event_type: axagent_harness::SessionEventType,
        payload: Option<serde_json::Value>,
    ) {
        let seq = self.next_seq(session_id).await;
        let payload_str = payload
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok())
            .unwrap_or_else(|| "NULL".to_string());
        let now = chrono::Utc::now();
        let ts = now.to_rfc3339();

        let sql = format!(
            "INSERT INTO session_events (session_id, seq, event_type, payload, created_at) \
             VALUES ('{session_id}', {seq}, '{evt_type}', {payload_str}, '{ts}')",
            evt_type = event_type.as_str(),
        );

        match self.db.execute_unprepared(&sql).await {
            Ok(_) => {},
            Err(e) => {
                tracing::warn!(
                    "[session_events] emit failed session_id={session_id} type={:?}: {e}",
                    event_type
                );
            },
        }
    }

    async fn clear(&self, session_id: &str) {
        let sql = format!("DELETE FROM session_events WHERE session_id = '{session_id}'");
        let _ = self.db.execute_unprepared(&sql).await;
    }
}
