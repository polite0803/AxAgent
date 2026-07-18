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
use axagent_harness::AgentSessionRepository;
use axagent_plugins::{PluginManager, PluginManagerConfig};
use axagent_runtime_core::prompt_cache::PromptCache;
use axagent_storage::cloud_storage::{CloudStorageConfig, SyncEngine};
use tokio_util::sync::CancellationToken;

/// 构造 AppState。
///
/// 失败时返回结构化错误，由调用方决定如何处理（错误展示 / 重试 / 退出）。
/// 不再 `process::exit(1)`——harness 架构要求启动错误可被前端感知。
pub async fn create_app_state(db_result: DatabaseInitResult) -> Result<AppState, String> {
    let DatabaseInitResult { db_handle, master_key, db_path, app_dir, .. } = db_result;

    // 初始化 RLOptimizer 共享状态（优先从文件加载，自动持久化）
    crate::commands::_shared_state::init_shared_state(&app_dir);

    // db_handle 进入 harness（Step 4）；同时克隆 conn 给其它需要 DatabaseConnection 的
    // 旧式组件（vector_store / trajectory_storage / cron / semantic_cache 等）。
    // 这些组件后续在 Step 5/6 也会迁到 harness 内部。
    let sea_db = db_handle.conn.clone();

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

    // 注入 tools 扩展层的 trait 实现（MigrationRunner + PluginAgentProvider）。
    // 通过 OnceLock 全局注入，工具层不再依赖 axagent-migration / axagent-plugins。
    axagent_tools::tools::init_extensions(
        std::sync::Arc::new(axagent_migration::DefaultMigrationRunner),
        std::sync::Arc::new(axagent_plugins::agent_provider::GlobalPluginAgentProvider),
    );

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
        if let Err(e) = ms.initialize().await {
            tracing::warn!("Failed to initialize MemoryService: {}", e);
        }
        Arc::new(TokioRwLock::new(ms))
    };

    // ── 初始化 Harness 容器（统一管理核心基础设施注入） ──
    let harness =
        axagent_runtime::harness::RuntimeHarness::new(axagent_runtime::harness::HarnessDeps {
            persistence: Arc::new(db_handle) as axagent_harness::SharedPersistence,
            master_key,
            provider_registry: Arc::new(
                axagent_providers::registry::ProviderRegistry::create_default(),
            )
                as Arc<dyn axagent_harness::registry::ProviderRegistry>,
        });
    let harness_registry = harness.provider_registry().clone();

    let platform_manager =
        Arc::new(axagent_runtime::message_gateway::platform_manager::PlatformManager::new());

    let platform_bridge = harness.build_platform_bridge(platform_manager.clone());

    platform_manager.set_message_callback(platform_bridge.clone()).await;

    let sync_engine = create_sync_engine(&sea_db, &app_settings).await;

    let config_home = app_dir.clone();
    let mut plugin_config = PluginManagerConfig::new(config_home.clone());
    plugin_config.external_dirs = axagent_kit::skill_dirs::all_skills_dirs();
    let npm_registry = Arc::new(axagent_npm::NpmRegistry::new());
    let plugin_manager = Arc::new(tokio::sync::RwLock::new(
        PluginManager::new(plugin_config).with_npm_registry(npm_registry),
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
    let agent_session_repo: Arc<dyn AgentSessionRepository> =
        Arc::new(DaoAgentSessionRepository::new(Arc::new(sea_db.clone())));
    let agent_session_manager = Arc::new(axagent_agent::SessionManager::new(agent_session_repo));
    let agent_cancel_tokens: Arc<DashMap<String, Arc<AtomicBool>>> = Arc::new(DashMap::new());
    let agent_paused: Arc<Mutex<std::collections::HashSet<String>>> =
        Arc::new(Mutex::new(std::collections::HashSet::new()));
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
    let sub_agent_registry: Arc<TokioRwLock<axagent_trajectory::SubAgentRegistry>> = Arc::new(
        TokioRwLock::new(axagent_trajectory::SubAgentRegistry::new().await.unwrap_or_default()),
    );
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
            engine
                .set_sandbox(Arc::new(
                    axagent_trajectory::SkillSandboxExecutor::with_default_policy(),
                ))
                .await;
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
        if let Err(e) = auto_ms.initialize().await {
            tracing::warn!("Failed to initialize MemoryService for AutoMemory: {}", e);
        }
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
    let cron_job_store: Arc<axagent_runtime_core::CronJobStore> =
        Arc::new(axagent_runtime_core::CronJobStore::new(Arc::new(sea_db.clone())).await);
    let user_profile: Arc<TokioRwLock<axagent_trajectory::UserProfile>> =
        Arc::new(TokioRwLock::new(axagent_trajectory::UserProfile::new()));
    let local_tool_registry: Arc<tokio::sync::Mutex<axagent_tools::registry::UnifiedToolRegistry>> = {
        let mut registry = axagent_tools::registry::UnifiedToolRegistry::new();
        registry.load_enabled_state(&sea_db).await;
        // 挂载 RL 策略工具排名器，每次 get_chat_tools() 实时读取最新权重
        registry.tool_ranker = Some(crate::commands::_shared_state::SHARED_TOOL_RANKER.clone());
        Arc::new(tokio::sync::Mutex::new(registry))
    };
    // ── 阶段 5:工作流反思 / 进化 / 优化三层 trait 实现 ──
    // 同一份 Arc 实例同时挂载到 WorkEngine(用于自动触发钩子)与 AppState 字段(供命令层手动调用)。
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

    // 优化 4-b:注入 LLM 变异器(若 DB 中有启用的 provider)
    {
        if let Some(bridge) = axagent_runtime::llm_bridge::build_llm_bridge_from_db_with(
            &master_key,
            &harness_registry,
            None,
            None,
        )
        .await
        {
            let mutator = super::workflow_injections::ProviderWorkflowLlmMutator::new(bridge);
            if let Err(e) = workflow_evolver
                .set_llm_provider(std::sync::Arc::new(mutator)
                    as std::sync::Arc<dyn axagent_harness::WorkflowLlmMutator>)
                .await
            {
                tracing::warn!("[Evolver] set_llm_provider failed: {e}");
            } else {
                tracing::info!(
                    "[Evolver] LLM mutator injected (provider workflow evolution enabled)"
                );
            }
        } else {
            tracing::info!(
                "[Evolver] No enabled provider in DB, LLM mutation disabled (using MVP placeholder)"
            );
        }
    }

    // P2-8:注入带有限试运行的沙箱(静态校验 + 模拟执行,始终注入)
    // 比 ReachabilityWorkflowSandbox 更强:额外做节点级配置合理性、累积超时上限、
    // 环检测,并用 tokio::time::timeout 做硬超时保护(5 秒)。
    {
        let sandbox = super::workflow_injections::DryRunWorkflowSandbox::new();
        if let Err(e) = workflow_evolver
            .set_sandbox(std::sync::Arc::new(sandbox)
                as std::sync::Arc<dyn axagent_harness::WorkflowSandbox>)
            .await
        {
            tracing::warn!("[Evolver] set_sandbox failed: {e}");
        } else {
            tracing::info!("[Evolver] DryRun sandbox injected (static + simulate + hard timeout)");
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
            let engine = Arc::new(
                axagent_runtime::work_engine::WorkEngine::new(
                    master_key,
                    harness_registry.clone(),
                )
                // 阶段 5:注入反思 / 进化 / 优化三层实现,WorkEngine 在工作流整体完成与节点级失败时自动触发反思。
                .with_workflow_reflector(workflow_reflector.clone())
                .with_workflow_evolver(workflow_evolver.clone())
                .with_workflow_optimizer(workflow_optimizer.clone()),
            );
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
            // ApprovalNode HITL: 注入数据库连接供 ApprovalOps 回调使用
            engine.set_db(sea_db.clone());
            engine
        };
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
    let webhook_subscription_manager: Option<
        Arc<axagent_runtime::webhook_subscription::WebhookSubscriptionManager>,
    > = Some(Arc::new(axagent_runtime::webhook_subscription::WebhookSubscriptionManager::new()));
    let semantic_cache: Arc<tokio::sync::Mutex<SemanticCacheState>> = {
        let cache = match SemanticCache::new(sea_db.clone(), CacheConfig::default()).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Semantic cache init failed: {} — retrying once", e);
                match SemanticCache::new(sea_db.clone(), CacheConfig::default()).await {
                    Ok(c) => c,
                    Err(e2) => {
                        // 数据库初始化已成功，两次失败表明 CREATE TABLE 持续出错。
                        // 回退到内存 SQLite，应用正常运行但缓存不持久化。
                        tracing::error!(
                            "Semantic cache failed permanently: {} — using in-memory fallback (non-persistent cache)",
                            e2
                        );
                        let fallback_db = sea_orm::Database::connect("sqlite::memory:").await;
                        match fallback_db {
                            Ok(mem_db) => SemanticCache::new(mem_db, CacheConfig::default())
                                .await
                                .map_err(|e3| {
                                    crate::android_utils::report_fatal_error(&format!(
                                        "SemanticCache in-memory fallback failed: {}",
                                        e3,
                                    ));
                                    format!("SemanticCache in-memory fallback failed: {}", e3)
                                })?,
                            Err(e3) => {
                                let msg =
                                    format!("SemanticCache in-memory DB connect failed: {}", e3,);
                                crate::android_utils::report_fatal_error(&msg);
                                return Err(msg);
                            },
                        }
                    },
                }
            },
        };
        Arc::new(tokio::sync::Mutex::new(SemanticCacheState {
            cache,
            enabled: true,
            in_memory_entries: Vec::new(),
            similarity_threshold: 0.85,
        }))
    };
    let prompt_cache = Arc::new(PromptCache::new());
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
    // Smart Router：ML 成本感知路由器实例化（带 DB 持久化，加载历史决策与统计）
    let cost_aware_router =
        Arc::new(crate::smart_router::CostAwareRouter::with_db(Arc::new(sea_db.clone())));
    cost_aware_router.load_from_db().await.map_err(|e| format!("加载路由历史失败: {}", e))?;
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
    let skill_state = crate::state::SkillState::new(
        skill_evolution_engine.clone(),
        skill_proposal_service.clone(),
        skill_decomposer.clone(),
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
    let learning_state = LearningEngineState::new(
        text_grad_engine.clone(),
        intrinsic_motivation.clone(),
        coevolution_env.clone(),
        process_reward_model.clone(),
    );
    let tool_state = ToolState::new(auto_tool_creator.clone());

    // 注册 MemoryRepository（给 MemoryFlush 等工具使用）
    axagent_harness::repositories::set_memory_repository(Arc::new(
        axagent_dao::memory_repository::DaoMemoryRepository::new(Arc::new(sea_db.clone())),
    ));

    // 启动时加载历史反思(P0-3 修复:进程重启后历史不丢失)。
    // reflect() 落盘由 Reflector::persist_reflection 自动处理,
    // 这里只需启动时从 `app_dir/reflections.jsonl` 加载到内存即可。
    match reflector.load_persistence().await {
        Ok(n) => tracing::info!("[reflector] loaded {n} reflections from disk"),
        Err(e) => tracing::warn!(
            "[reflector] load_persistence failed: {e} (will start with empty history)"
        ),
    }

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
        agent_session_manager,
        agent_cancel_tokens,
        agent_paused,
        running_agents,
        steer_queue,
        reflector,
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
        auto_memory_extractor,
        parallel_execution_service,
        cron_job_store,
        cron_scheduler: Arc::new(tokio::sync::RwLock::new(None)),
        platform_manager,
        platform_bridge,
        user_profile,
        local_tool_registry,
        work_engine,
        workflow_reflector,
        workflow_evolver,
        workflow_optimizer,
        skill_decomposer,
        proactive_service,
        dashboard_registry,
        webhook_subscription_manager,
        semantic_cache,
        prompt_cache,
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
        plugin_manager,
        file_authorizer,
        credential_manager,
        session_share_manager,
        #[cfg(not(mobile))]
        pty_manager,
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
    })
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
