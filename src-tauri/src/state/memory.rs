//! Memory / learning domain state.
//!
//! Owns the trajectory / memory subsystem: shared memory, memory service,
//! sub-agent registry, nudge / closed-loop services, the various
//! learners (pattern, cross-session, RL), batch processing, the
//! dream consolidator, the semantic cache, and the prompt cache.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::RwLock as TokioRwLock;

pub struct MemoryState {
    pub shared_memory: Arc<TokioRwLock<axagent_runtime::shared_memory::SharedMemory>>,
    pub sub_agent_registry: Arc<TokioRwLock<axagent_trajectory::SubAgentRegistry>>,
    pub memory_service: Arc<TokioRwLock<axagent_trajectory::MemoryService>>,
    pub nudge_service: Arc<tokio::sync::Mutex<axagent_trajectory::NudgeService>>,
    /// 统一显著性仲裁器 — 多主动信号源竞争-广播（GWT 骨架）。
    /// `saliency_enabled=false`（默认）时消费方不得使用，行为与无仲裁器完全一致。
    pub saliency_arbiter: Arc<tokio::sync::Mutex<axagent_trajectory::SaliencyArbiter>>,
    pub saliency_enabled: Arc<AtomicBool>,
    /// 自主觉知监控器 — 内部状态感知与帧缓冲（R2：所有输入取真实运行时数据）。
    pub awareness_monitor: Arc<tokio::sync::Mutex<axagent_trajectory::AwarenessMonitor>>,
    pub closed_loop_service: Arc<axagent_trajectory::ClosedLoopService>,
    pub trajectory_storage: Arc<axagent_trajectory::TrajectoryStorage>,
    pub insight_system: Arc<TokioRwLock<axagent_trajectory::LearningInsightSystem>>,
    pub realtime_learning: Arc<tokio::sync::Mutex<axagent_trajectory::RealTimeLearning>>,
    pub pattern_learner: Arc<TokioRwLock<axagent_trajectory::PatternLearner>>,
    pub cross_session_learner: Arc<TokioRwLock<axagent_trajectory::CrossSessionLearner>>,
    pub rl_engine: Arc<TokioRwLock<axagent_trajectory::RLEngine>>,
    pub batch_processor: Arc<axagent_trajectory::BatchProcessor>,
    pub auto_memory_extractor: Arc<TokioRwLock<axagent_trajectory::AutoMemoryExtractor>>,
    pub parallel_execution_service:
        Arc<tokio::sync::RwLock<axagent_trajectory::ParallelExecutionService>>,
    pub cron_job_store: Arc<axagent_runtime_core::CronJobStore>,
    pub user_profile: Arc<TokioRwLock<axagent_trajectory::UserProfile>>,
    pub semantic_cache: Arc<tokio::sync::Mutex<crate::app_state::SemanticCacheState>>,
    pub prompt_cache: Arc<axagent_runtime_core::prompt_cache::PromptCache>,
    pub dream_consolidator: Arc<axagent_trajectory::DreamConsolidator>,
    pub dream_data_provider: Arc<axagent_trajectory::TrajectoryDreamDataProvider>,
    pub session_share_manager: crate::app_state::SessionShareStore,
}

impl MemoryState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        shared_memory: Arc<TokioRwLock<axagent_runtime::shared_memory::SharedMemory>>,
        sub_agent_registry: Arc<TokioRwLock<axagent_trajectory::SubAgentRegistry>>,
        memory_service: Arc<TokioRwLock<axagent_trajectory::MemoryService>>,
        nudge_service: Arc<tokio::sync::Mutex<axagent_trajectory::NudgeService>>,
        closed_loop_service: Arc<axagent_trajectory::ClosedLoopService>,
        trajectory_storage: Arc<axagent_trajectory::TrajectoryStorage>,
        insight_system: Arc<TokioRwLock<axagent_trajectory::LearningInsightSystem>>,
        realtime_learning: Arc<tokio::sync::Mutex<axagent_trajectory::RealTimeLearning>>,
        pattern_learner: Arc<TokioRwLock<axagent_trajectory::PatternLearner>>,
        cross_session_learner: Arc<TokioRwLock<axagent_trajectory::CrossSessionLearner>>,
        rl_engine: Arc<TokioRwLock<axagent_trajectory::RLEngine>>,
        batch_processor: Arc<axagent_trajectory::BatchProcessor>,
        auto_memory_extractor: Arc<TokioRwLock<axagent_trajectory::AutoMemoryExtractor>>,
        parallel_execution_service: Arc<
            tokio::sync::RwLock<axagent_trajectory::ParallelExecutionService>,
        >,
        cron_job_store: Arc<axagent_runtime_core::CronJobStore>,
        user_profile: Arc<TokioRwLock<axagent_trajectory::UserProfile>>,
        semantic_cache: Arc<tokio::sync::Mutex<crate::app_state::SemanticCacheState>>,
        prompt_cache: Arc<axagent_runtime_core::prompt_cache::PromptCache>,
        dream_consolidator: Arc<axagent_trajectory::DreamConsolidator>,
        dream_data_provider: Arc<axagent_trajectory::TrajectoryDreamDataProvider>,
        session_share_manager: crate::app_state::SessionShareStore,
    ) -> Self {
        Self {
            shared_memory,
            sub_agent_registry,
            memory_service,
            nudge_service,
            saliency_arbiter: Arc::new(tokio::sync::Mutex::new(
                axagent_trajectory::SaliencyArbiter::default(),
            )),
            saliency_enabled: Arc::new(AtomicBool::new(false)),
            awareness_monitor: Arc::new(tokio::sync::Mutex::new(
                axagent_trajectory::AwarenessMonitor::default(),
            )),
            closed_loop_service,
            trajectory_storage,
            insight_system,
            realtime_learning,
            pattern_learner,
            cross_session_learner,
            rl_engine,
            batch_processor,
            auto_memory_extractor,
            parallel_execution_service,
            cron_job_store,
            user_profile,
            semantic_cache,
            prompt_cache,
            dream_consolidator,
            dream_data_provider,
            session_share_manager,
        }
    }
}
