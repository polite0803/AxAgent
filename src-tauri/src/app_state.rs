use crate::commands::proactive::ProactiveService;
use crate::semantic_cache::SemanticCache;
use axagent_runtime::dashboard_registry::DashboardRegistry;
use axagent_runtime::webhook_subscription::WebhookSubscriptionManager;
use sea_orm::DatabaseConnection;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

use std::path::PathBuf;
use tokio::sync::RwLock as TokioRwLock;

pub struct AppState {
    pub sea_db: DatabaseConnection,
    pub master_key: [u8; 32],
    pub gateway: Arc<Mutex<Option<axagent_gateway::server::GatewayServer>>>,
    pub close_to_tray: Arc<AtomicBool>,
    pub app_data_dir: PathBuf,
    pub db_path: String,
    pub auto_backup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub webdav_sync_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub vector_store: Arc<axagent_core::vector_store::VectorStore>,
    pub indexing_semaphore: Arc<tokio::sync::Semaphore>,
    pub stream_cancel_flags: Arc<Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>>,
    pub agent_permission_senders:
        Arc<Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>>,
    pub agent_ask_senders:
        Arc<Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>>,
    pub agent_always_allowed:
        Arc<Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>>,
    pub agent_prompters:
        Arc<Mutex<std::collections::HashMap<String, axagent_agent::ChannelPermissionPrompter>>>,
    pub agent_session_manager: Arc<axagent_agent::SessionManager>,
    pub agent_cancel_tokens: Arc<Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>>,
    pub agent_paused: Arc<Mutex<std::collections::HashSet<String>>>,
    pub running_agents: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    pub workflow_engine: Arc<axagent_runtime::workflow_engine::WorkflowEngine>,
    // 以下字段从 std::sync::RwLock 改为 tokio::sync::RwLock
    // 原因：std::sync::RwLock 的 guard 是 !Send，在异步上下文中跨 await 持有会导致未定义行为
    // 且 std::sync::RwLock 在 panic 时会毒化，后续所有 .unwrap() 都会崩溃
    pub shared_memory: Arc<TokioRwLock<axagent_runtime::shared_memory::SharedMemory>>,
    pub sub_agent_registry: Arc<TokioRwLock<axagent_trajectory::SubAgentRegistry>>,
    pub memory_service: Arc<TokioRwLock<axagent_trajectory::MemoryService>>,
    pub nudge_service: Arc<tokio::sync::Mutex<axagent_trajectory::NudgeService>>,
    pub closed_loop_service: Arc<axagent_trajectory::ClosedLoopService>,
    pub trajectory_storage: Arc<axagent_trajectory::TrajectoryStorage>,
    pub insight_system: Arc<TokioRwLock<axagent_trajectory::LearningInsightSystem>>,
    pub realtime_learning: Arc<tokio::sync::Mutex<axagent_trajectory::RealTimeLearning>>,
    pub pattern_learner: Arc<TokioRwLock<axagent_trajectory::PatternLearner>>,
    pub cross_session_learner: Arc<TokioRwLock<axagent_trajectory::CrossSessionLearner>>,
    pub rl_engine: Arc<TokioRwLock<axagent_trajectory::RLEngine>>,
    pub batch_processor: Arc<axagent_trajectory::BatchProcessor>,
    pub skill_evolution_engine: Arc<tokio::sync::Mutex<axagent_trajectory::SkillEvolutionEngine>>,
    pub skill_proposal_service: Arc<TokioRwLock<axagent_trajectory::SkillProposalService>>,
    pub auto_memory_extractor: Arc<TokioRwLock<axagent_trajectory::AutoMemoryExtractor>>,
    pub parallel_execution_service:
        Arc<tokio::sync::RwLock<axagent_trajectory::ParallelExecutionService>>,
    pub scheduled_task_service: Arc<tokio::sync::RwLock<axagent_trajectory::ScheduledTaskService>>,
    pub platform_integration_service:
        Arc<tokio::sync::RwLock<axagent_trajectory::PlatformIntegrationService>>,
    pub platform_manager: Arc<axagent_runtime::message_gateway::platform_manager::PlatformManager>,
    pub platform_bridge: Arc<axagent_runtime::message_gateway::platform_bridge::PlatformBridge>,
    pub user_profile: Arc<TokioRwLock<axagent_trajectory::UserProfile>>,
    pub local_tool_registry: Arc<tokio::sync::Mutex<axagent_agent::LocalToolRegistry>>,
    pub work_engine: Arc<tokio::sync::RwLock<axagent_runtime::work_engine::WorkEngine>>,
    pub skill_decomposer: Arc<tokio::sync::RwLock<axagent_trajectory::SkillDecomposer>>,
    pub proactive_service: Arc<tokio::sync::RwLock<ProactiveService>>,
    pub dashboard_registry: Option<Arc<DashboardRegistry>>,
    pub webhook_subscription_manager: Option<Arc<WebhookSubscriptionManager>>,
    pub semantic_cache: Arc<tokio::sync::Mutex<SemanticCache>>,
    // 浏览器客户端：使用 tokio::sync::Mutex 取代全局 static mut，避免数据竞争
    pub browser_client: Arc<tokio::sync::Mutex<Option<axagent_core::browser_automation::PlaywrightClient>>>,
}
