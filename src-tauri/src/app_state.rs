use crate::commands::proactive::ProactiveService;
use crate::semantic_cache::SemanticCache;
use axagent_core::cloud_storage::SyncEngine;
use axagent_core::file_authorizer::FileAuthorizer;
use axagent_plugins::PluginManager;
use axagent_runtime::dashboard_registry::DashboardRegistry;
use axagent_runtime::webhook_subscription::WebhookSubscriptionManager;
use axagent_runtime_core::prompt_cache::PromptCache;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use std::path::PathBuf;
use tokio::sync::RwLock as TokioRwLock;

// Tree of Thoughts types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub content: String,
    pub score: Option<f64>,
    pub children: Vec<String>,
    pub depth: u32,
    pub thought_type: String,
    pub metadata: serde_json::Value,
}

impl Default for TotNode {
    fn default() -> Self {
        Self {
            id: String::new(),
            parent_id: None,
            content: String::new(),
            score: None,
            children: Vec::new(),
            depth: 0,
            thought_type: "reasoning".to_string(),
            metadata: serde_json::Value::Null,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotSession {
    pub nodes: HashMap<String, TotNode>,
    pub current_node_id: Option<String>,
    pub root_node_id: Option<String>,
    pub traversal_strategy: String,
    pub pruning_threshold: f64,
    pub max_depth: u32,
    pub max_branches: u32,
}

impl Default for TotSession {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            current_node_id: None,
            root_node_id: None,
            traversal_strategy: "bfs".to_string(),
            pruning_threshold: 0.3,
            max_depth: 5,
            max_branches: 3,
        }
    }
}

// Replanning types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerAction {
    pub id: String,
    pub timestamp: i64,
    pub action_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerVersion {
    pub id: u32,
    pub timestamp: i64,
    pub reason: String,
    pub state: serde_json::Value,
    pub action_snapshot: Vec<PlannerAction>,
    pub diff_from_previous: Option<PlannerVersionDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerVersionDiff {
    pub from_version: u32,
    pub to_version: u32,
    pub actions_added: Vec<PlannerAction>,
    pub actions_removed: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerSession {
    pub actions: Vec<PlannerAction>,
    pub versions: Vec<PlannerVersion>,
    pub current_version: u32,
}

// Semantic cache (add enabled flag)
pub struct SemanticCacheState {
    pub cache: SemanticCache,
    pub enabled: bool,
    pub in_memory_entries: Vec<InMemoryCacheEntry>,
    pub similarity_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InMemoryCacheEntry {
    pub query_hash: String,
    pub query_text: String,
    pub query_embedding: Vec<f32>,
    pub response: String,
    pub model_id: Option<String>,
    pub created_at: i64,
    pub access_count: u32,
    pub ttl_secs: u64,
}

// ─── Session Share 类型 ───

/// 共享会话的内部存储记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareSessionRecord {
    pub session_id: String,
    pub invite_code: String,
    pub conversation_id: String,
    pub permissions: SharePermissions,
    pub participants: Vec<ShareParticipant>,
    pub created_at: i64,
}

/// 共享会话权限配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharePermissions {
    pub allow_terminal_access: bool,
    pub allow_file_access: bool,
    pub allow_model_access: bool,
    pub require_approval_for_actions: bool,
    pub max_participants: u32,
}

impl Default for SharePermissions {
    fn default() -> Self {
        Self {
            allow_terminal_access: true,
            allow_file_access: true,
            allow_model_access: false,
            require_approval_for_actions: true,
            max_participants: 10,
        }
    }
}

/// 会话参与者
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareParticipant {
    pub id: String,
    pub name: String,
    pub joined_at: i64,
}

/// 共享会话存储类型
pub type SessionShareStore =
    Arc<TokioRwLock<std::collections::HashMap<String, ShareSessionRecord>>>;

pub struct AppState {
    pub sea_db: DatabaseConnection,
    pub master_key: [u8; 32],
    pub gateway: Arc<Mutex<Option<axagent_gateway::server::GatewayServer>>>,
    pub close_to_tray: Arc<AtomicBool>,
    pub app_data_dir: PathBuf,
    pub db_path: String,
    pub auto_backup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub webdav_sync_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub api_server_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub trajectory_cleanup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 集中式任务管理器（Phase C-1），逐步替代上方的独立 JoinHandle 字段
    pub task_manager: Arc<axagent_runtime::task_manager::TaskManager>,
    /// 技能文件监听器关闭信号 (OS 线程), OnceLock 允许在 &self 上初始化
    pub skill_watcher_shutdown: std::sync::OnceLock<Arc<AtomicBool>>,
    /// 优雅关闭信号，通知所有后台任务停止
    pub shutdown_token: CancellationToken,
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
    pub reflector: Arc<axagent_agent::Reflector>,
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
    pub cron_job_store: Arc<axagent_runtime_core::CronJobStore>,
    pub platform_manager: Arc<axagent_runtime::message_gateway::platform_manager::PlatformManager>,
    pub platform_bridge: Arc<axagent_runtime::message_gateway::platform_bridge::PlatformBridge>,
    pub user_profile: Arc<TokioRwLock<axagent_trajectory::UserProfile>>,
    pub local_tool_registry: Arc<tokio::sync::Mutex<axagent_tools::registry::UnifiedToolRegistry>>,
    pub work_engine: Arc<axagent_runtime::work_engine::WorkEngine>,
    pub skill_decomposer: Arc<tokio::sync::RwLock<axagent_trajectory::SkillDecomposer>>,
    pub proactive_service: Arc<tokio::sync::RwLock<ProactiveService>>,
    pub dashboard_registry: Option<Arc<DashboardRegistry>>,
    pub webhook_subscription_manager: Option<Arc<WebhookSubscriptionManager>>,
    pub semantic_cache: Arc<tokio::sync::Mutex<SemanticCacheState>>,
    pub prompt_cache: Arc<PromptCache>,
    // Tree of Thoughts state
    pub tot_sessions: Arc<tokio::sync::Mutex<HashMap<String, TotSession>>>,
    // Replanning state
    pub planner_sessions: Arc<tokio::sync::Mutex<HashMap<String, PlannerSession>>>,
    // Browser client: use tokio::sync::Mutex to replace global static mut to avoid data race
    #[cfg(not(target_os = "android"))]
    pub browser_client:
        Arc<tokio::sync::Mutex<Option<axagent_core::browser_automation::PlaywrightClient>>>,
    #[cfg(target_os = "android")]
    pub browser_client: Arc<tokio::sync::Mutex<Option<()>>>,
    pub dream_consolidator: Arc<axagent_trajectory::DreamConsolidator>,
    pub text_grad_engine: Arc<tokio::sync::Mutex<axagent_trajectory::TextGradEngine>>,
    pub auto_tool_creator: Arc<tokio::sync::Mutex<axagent_trajectory::AutoToolCreator>>,
    pub intrinsic_motivation:
        Arc<tokio::sync::Mutex<axagent_trajectory::IntrinsicMotivationEngine>>,
    pub coevolution_env: Arc<tokio::sync::Mutex<axagent_trajectory::CoevolutionEnvironment>>,
    pub constitution: Arc<axagent_trajectory::ImmutableConstitution>,
    pub process_reward_model: Arc<tokio::sync::Mutex<axagent_trajectory::ProcessRewardModel>>,
    pub dream_data_provider: Arc<axagent_trajectory::TrajectoryDreamDataProvider>,
    #[cfg(not(target_os = "android"))]
    pub sandbox_executor: Arc<axagent_trajectory::SkillSandboxExecutor>,
    #[cfg(target_os = "android")]
    pub sandbox_executor: Arc<()>,
    pub sync_engine: Option<Arc<SyncEngine>>,
    pub plugin_manager: std::sync::RwLock<PluginManager>,
    pub file_authorizer: Arc<FileAuthorizer>,
    pub session_share_manager: SessionShareStore,
}

impl Drop for AppState {
    fn drop(&mut self) {
        self.shutdown_token.cancel();
        tracing::info!("[shutdown] CancellationToken 已通知，后台任务应停止");
    }
}
