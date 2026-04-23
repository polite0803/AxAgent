use sea_orm::DatabaseConnection;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

use std::path::PathBuf;

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
    pub agent_permission_senders: Arc<Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>>,
    pub agent_ask_senders: Arc<Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>>,
    pub agent_always_allowed: Arc<Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>>,
    pub agent_prompters: Arc<Mutex<std::collections::HashMap<String, axagent_agent::ChannelPermissionPrompter>>>,
    pub agent_session_manager: axagent_agent::SessionManager,
    pub agent_cancel_tokens: Arc<Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>>,
    pub agent_paused: Arc<Mutex<std::collections::HashSet<String>>>,
    pub running_agents: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    pub workflow_engine: Arc<axagent_runtime::workflow_engine::WorkflowEngine>,
    pub shared_memory: Arc<std::sync::RwLock<axagent_runtime::shared_memory::SharedMemory>>,
    pub sub_agent_registry: Arc<std::sync::RwLock<axagent_trajectory::SubAgentRegistry>>,
    pub memory_service: Arc<std::sync::RwLock<axagent_trajectory::MemoryService>>,
    pub nudge_service: Arc<tokio::sync::Mutex<axagent_trajectory::NudgeService>>,
    pub closed_loop_service: Arc<axagent_trajectory::ClosedLoopService>,
    pub trajectory_storage: Arc<axagent_trajectory::TrajectoryStorage>,
    pub insight_system: Arc<std::sync::RwLock<axagent_trajectory::LearningInsightSystem>>,
    pub realtime_learning: Arc<tokio::sync::Mutex<axagent_trajectory::RealTimeLearning>>,
    pub pattern_learner: Arc<std::sync::RwLock<axagent_trajectory::PatternLearner>>,
    pub cross_session_learner: Arc<std::sync::RwLock<axagent_trajectory::CrossSessionLearner>>,
    pub rl_engine: Arc<std::sync::RwLock<axagent_trajectory::RLEngine>>,
    pub batch_processor: Arc<axagent_trajectory::BatchProcessor>,
    pub skill_evolution_engine: Arc<tokio::sync::Mutex<axagent_trajectory::SkillEvolutionEngine>>,
    pub skill_proposal_service: Arc<std::sync::RwLock<axagent_trajectory::SkillProposalService>>,
    pub auto_memory_extractor: Arc<std::sync::RwLock<axagent_trajectory::AutoMemoryExtractor>>,
    pub parallel_execution_service: Arc<tokio::sync::RwLock<axagent_trajectory::ParallelExecutionService>>,
    pub scheduled_task_service: Arc<tokio::sync::RwLock<axagent_trajectory::ScheduledTaskService>>,
    pub platform_integration_service: Arc<tokio::sync::RwLock<axagent_trajectory::PlatformIntegrationService>>,
    pub user_profile: Arc<std::sync::RwLock<axagent_trajectory::UserProfile>>,
    pub local_tool_registry: Arc<tokio::sync::Mutex<axagent_agent::LocalToolRegistry>>,
}
