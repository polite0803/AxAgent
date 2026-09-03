// SPDX-License-Identifier: AGPL-3.0-only

use crate::commands::proactive::ProactiveService;
use crate::semantic_cache::SemanticCache;
use crate::state::{
    AgentState, GatewayState, InfraState, LearningEngineState, MemoryState, SkillState, TaskState,
    ToolState,
};
use axagent_credential::CredentialManager;
use axagent_harness::CapabilityGapProposal;
use axagent_harness::DefaultCapabilityRouter;
use axagent_harness::PatternPromptGuard;
use axagent_harness::fleet::FleetRepository;
use axagent_plugins::PluginManager;
use axagent_runtime::dashboard_registry::DashboardRegistry;
use axagent_runtime::webhook_subscription::WebhookSubscriptionManager;
use axagent_runtime_core::prompt_cache::PromptCache;
use axagent_storage::cloud_storage::SyncEngine;
use axagent_storage::file_authorizer::FileAuthorizer;
use axagent_telemetry::TelemetryLevel;
use axagent_tools::CapabilityIndexerImpl;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use std::path::PathBuf;
use tokio::sync::RwLock as TokioRwLock;

// Tree of Thoughts constants
/// 剪枝阈值：低于此分数的节点在 BFS 遍历中被丢弃
pub const TOT_DEFAULT_PRUNING_THRESHOLD: f64 = 0.3;
/// 最大搜索深度（单位：ToT 层数）
pub const TOT_DEFAULT_MAX_DEPTH: u32 = 5;
/// 每层最大分支数
pub const TOT_DEFAULT_MAX_BRANCHES: u32 = 3;

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
            pruning_threshold: TOT_DEFAULT_PRUNING_THRESHOLD,
            max_depth: TOT_DEFAULT_MAX_DEPTH,
            max_branches: TOT_DEFAULT_MAX_BRANCHES,
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
    /// 共享的语义缓存实例。用 `Arc` 便于同一实例既供缓存管理命令使用，
    /// 又能作为 `Arc<dyn HarnessCache>` 注入主聊天路径的 `LlmCallConfig`。
    pub cache: Arc<SemanticCache>,
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
#[serde(rename_all = "camelCase")]
pub struct SharePermissions {
    pub allow_terminal_access: bool,
    pub allow_file_access: bool,
    pub allow_model_access: bool,
    pub require_approval_for_actions: bool,
    pub max_participants: u32,
}

impl Default for SharePermissions {
    fn default() -> Self {
        // SECURITY (M9): 共享会话默认拒绝 terminal / file 访问。
        // 加入会话 ≠ 拥有主用户的 shell 与文件系统。
        Self {
            allow_terminal_access: false,
            allow_file_access: false,
            allow_model_access: false,
            require_approval_for_actions: true,
            max_participants: 5,
        }
    }
}

/// 会话参与者
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareParticipant {
    pub id: String,
    pub name: String,
    pub joined_at: i64,
}

/// 共享会话存储类型
pub type SessionShareStore =
    Arc<TokioRwLock<std::collections::HashMap<String, ShareSessionRecord>>>;

// ── AppState 字段迁移计划 ──────────────────────────────────────────
//
// Harness 架构目标已完成 Step 14：原 `sea_db` / `master_key` / `db_path` 三个
// `RuntimeHarness` 镜像字段已全部移除，所有调用方统一走 `state.harness.xxx()`。
//
// 仍由 `AppState` 持有的非镜像字段（路由层薄壳）：
//   - `gateway` / `close_to_tray` / `app_data_dir` / `task handles`
//   - `vector_store` / `indexing_semaphore` / `stream_cancel_flags`
//   - `agent_*`（权限 / 取消 / 会话 / 反思等 agent 运行时状态）
//   - `memory_service` / `shared_memory` / `sub_agent_registry` / `trajectory_*`
//   - 其它领域服务（pattern_learner / rl_engine / cron_job_store / ...）
pub struct AppState {
    pub gateway: Arc<Mutex<Option<axagent_gateway::server::GatewayServer>>>,
    pub close_to_tray: Arc<AtomicBool>,
    pub app_data_dir: PathBuf,
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
    pub vector_store: Arc<axagent_search::vector_store::VectorStore>,
    pub indexing_semaphore: Arc<tokio::sync::Semaphore>,
    pub stream_cancel_flags: Arc<DashMap<String, Arc<AtomicBool>>>,
    pub agent_permission_senders:
        Arc<Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>>,
    pub agent_ask_senders:
        Arc<Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>>,
    pub agent_always_allowed:
        Arc<Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>>,
    pub agent_prompters:
        Arc<Mutex<std::collections::HashMap<String, axagent_agent::ChannelPermissionPrompter>>>,
    /// 计划确认闸门（P0-2）的挂起审批槽：conversationId → 批准信号发送端。
    /// `agent_query` 在闸门触发时插入 sender 并 await；`agent_approve_plan` 取出并发送。
    pub agent_plan_approvals:
        Arc<Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<bool>>>>,
    /// 能力补齐/进化改进提议的挂起审批槽：proposalId → 同意信号发送端（阻塞式，保留供超时兼容）。
    pub evolution_consent_senders:
        Arc<Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<bool>>>>,
    /// 待处理的能力缺口提议（非阻塞式，静默存储，用户手动处理）。
    /// 认知编排器触发能力补齐时不再即时弹窗，而是将提议存入此处，
    /// 前端通过通知徽章提示，用户在能力管理面板中审核处理。
    pub pending_capability_gaps:
        Arc<tokio::sync::Mutex<std::collections::HashMap<String, CapabilityGapProposal>>>,
    pub agent_session_manager: Arc<axagent_agent::SessionManager>,
    pub agent_cancel_tokens: Arc<DashMap<String, Arc<AtomicBool>>>,
    pub agent_paused: Arc<Mutex<std::collections::HashSet<String>>>,
    /// P0-3 暂停桥接：conversationId → 共享 PauseState。
    /// `agent_pause`/`agent_resume` 通过它唤醒/挂起 runtime 循环（wait_while_paused）。
    pub agent_pause_states: Arc<DashMap<String, Arc<axagent_runtime_core::PauseState>>>,
    pub running_agents: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    /// 前端 SteerInput 指令队列。conversationId → Vec<instruction>
    pub steer_queue: Arc<tokio::sync::Mutex<std::collections::HashMap<String, Vec<String>>>>,
    pub reflector: Arc<axagent_agent::Reflector>,
    /// P2 集成: MCP stdio server 实例。wiring 层在 create_app_state 构造并注入。
    /// 可通过 xagent-mcp-serve binary 或直接启动 stdio transport 对外暴露。
    pub mcp_agent_server: Arc<axagent_mcp::McpAgentServer>,
    // 以下字段从 parking_lot::RwLock 改为 tokio::sync::RwLock
    // 原因：parking_lot::RwLock 的 guard 是 !Send，在异步上下文中跨 await 持有会导致未定义行为
    // 且 parking_lot::RwLock 在 panic 时会毒化，后续所有 .unwrap() 都会崩溃
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
    /// 技能学习管理器 — 编排技能创建/改进/审查/审批全流程
    pub skill_learning_manager: Arc<TokioRwLock<axagent_trajectory::SkillLearningManager>>,
    pub auto_memory_extractor: Arc<TokioRwLock<axagent_trajectory::AutoMemoryExtractor>>,
    pub parallel_execution_service:
        Arc<tokio::sync::RwLock<axagent_trajectory::ParallelExecutionService>>,
    pub cron_job_store: Arc<axagent_runtime_core::CronJobStore>,
    pub cron_scheduler: Arc<tokio::sync::RwLock<Option<Arc<axagent_runtime::cron::CronScheduler>>>>,
    pub platform_manager: Arc<axagent_runtime::message_gateway::platform_manager::PlatformManager>,
    pub platform_bridge: Arc<axagent_runtime::message_gateway::platform_bridge::PlatformBridge>,
    pub user_profile: Arc<TokioRwLock<axagent_trajectory::UserProfile>>,
    pub local_tool_registry: Arc<tokio::sync::Mutex<axagent_tools::registry::UnifiedToolRegistry>>,
    /// 进化产物运行时执行统计（阶段四后置闭环）：
    /// `conversation_id → tool_id → 真实成败计数`（D2 会话隔离，避免跨会话污染决策）。
    /// 与 `GeneratedToolAdapter` 注入的 `EvolutionFeedbackSink` 共享同一 Arc，
    /// 供进化决策按会话查询融合真实执行反馈。
    pub evolution_execution_stats: Arc<
        tokio::sync::Mutex<
            HashMap<
                String,
                HashMap<String, axagent_harness::workflow_evolution::ToolExecutionStats>,
            >,
        >,
    >,
    pub work_engine: Arc<axagent_runtime::work_engine::WorkEngine>,
    /// 夜间长时任务的成本门控状态（max_budget / spent / tripped）。
    /// 内存态，重启后由运维经 `set_budget` 重新配置；供 Scheduler gate 判定。
    pub scheduler_budget: Arc<tokio::sync::RwLock<crate::scheduler::gate::BudgetState>>,
    // 工作流反思器 / 进化器 / 优化器已不作为 AppState 字段持有 —— 命令层经
    // workflow.reflector / workflow.evolver / workflow.optimizer 能力接缝获取
    // （`axagent_harness::get_capability_registry().get_*()`），与 WorkEngine 同源。
    pub skill_decomposer: Arc<tokio::sync::RwLock<axagent_trajectory::SkillDecomposer>>,
    pub proactive_service: Arc<tokio::sync::RwLock<ProactiveService>>,
    pub dashboard_registry: Option<Arc<DashboardRegistry>>,
    pub webhook_subscription_manager: Option<Arc<WebhookSubscriptionManager>>,
    /// Webhook 事件派发器（P0 修复：用于在工具执行 / Agent 结束时触发 webhook）
    pub webhook_event_emitter: Option<Arc<dyn axagent_harness::WebhookEventSink>>,
    pub semantic_cache: Arc<tokio::sync::Mutex<SemanticCacheState>>,
    pub prompt_cache: Arc<PromptCache>,
    /// Fleet 持久化仓库
    pub fleet_repository: Arc<dyn FleetRepository>,
    /// Fleet 意图分类 LLM（wiring 层注入真实实现，供 fleet_dispatch 路由与 LlmDispatcher 复用）
    pub fleet_intent_llm: Arc<dyn axagent_harness::fleet::FleetIntentLlm>,
    /// Harness 容器（统一管理核心基础设施注入）
    pub harness: axagent_runtime::harness::RuntimeHarness,
    // Tree of Thoughts state
    pub tot_sessions: Arc<tokio::sync::Mutex<HashMap<String, TotSession>>>,
    // Replanning state
    pub planner_sessions: Arc<tokio::sync::Mutex<HashMap<String, PlannerSession>>>,
    // Browser client: use tokio::sync::Mutex to replace global static mut to avoid data race
    #[cfg(not(target_os = "android"))]
    pub browser_client:
        Arc<tokio::sync::Mutex<Option<axagent_kit::browser_automation::PlaywrightClient>>>,
    #[cfg(target_os = "android")]
    pub browser_client: Arc<tokio::sync::Mutex<Option<()>>>,
    pub dream_consolidator: Arc<axagent_trajectory::DreamConsolidator>,
    /// Smart Router：ML 成本感知路由器（启发式 + 历史统计 + 成本预算）
    pub cost_aware_router: Arc<crate::smart_router::CostAwareRouter>,
    /// Orchestrator 流式报告器（多 Agent 实时协作的 chunk 推送）
    pub stream_reporter:
        Arc<TokioRwLock<Option<Arc<dyn axagent_harness::streaming::AgentStreamReporter>>>>,
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
    /// 设备同步状态（多设备配对、同步引擎）
    pub device_sync_state: Arc<tokio::sync::RwLock<crate::commands::device_sync::DeviceSyncState>>,
    pub plugin_manager: Arc<tokio::sync::RwLock<PluginManager>>,
    pub file_authorizer: Arc<FileAuthorizer>,
    pub credential_manager: Arc<CredentialManager>,
    pub database_query_service: Arc<dyn axagent_harness::DatabaseQueryService>,
    pub session_share_manager: SessionShareStore,
    /// PTY 伪终端管理器，管理所有终端会话（仅桌面端可用）
    #[cfg(not(mobile))]
    pub pty_manager: Arc<axagent_runtime::pty::PtyManager>,
    /// 2.7 P1:遥测级别共享句柄 — 启动时从 `AppSettings.telemetry_level`
    /// 读取初值,`save_settings` 命令检测到变更后更新此句柄;运行中的
    /// `FilteringSink` 通过 `level_handle()` 引用同一 `Arc` 实现热更新。
    pub telemetry_level_handle: Arc<parking_lot::RwLock<TelemetryLevel>>,
    /// 2.7 P1:生产实例化的遥测 sink 链根节点。
    ///
    /// 由 wiring 层(`init/state.rs`)在构造 AppState 时实例化:
    /// 1. `JsonlTelemetrySink::new(app_dir/telemetry.jsonl)` — 落盘 sink
    /// 2. `FilteringSink::new_with_handle(inner, telemetry_level_handle)` — 按级别过滤
    ///
    /// 同一份 `telemetry_level_handle` 同时挂载到本字段与 `FilteringSink.level`,
    /// 因此 `save_settings` 更新 handle 时 sink 链立即响应,无需重建。
    ///
    /// 消费方:agent / streaming / providers 等通过
    /// `SessionTracer::new(sid, sink.clone())` 创建会话级 tracer,
    /// 所有 `record()` 调用都会经过 `FilteringSink` 过滤后落盘。
    pub telemetry_sink: Arc<dyn axagent_telemetry::TelemetrySink>,

    /// 3.3 P2:持久化重试调度器(可选,None = 未启用)。
    ///
    /// 由 wiring 层在 `create_app_state` 中根据 `UnifiedConfig.persistent_runner.enabled`
    /// 决定是否构造。`enabled: false`(默认)时不构造,零开销。
    ///
    /// 启用后,`start_background_services` 会调用 `spawn_daemon` 启动后台守护循环,
    /// 每 60 秒检查 pending session 并调度执行。
    ///
    /// **注意**:当前 executor 闭包为占位实现(返回 `Err("not implemented")`),
    /// 真正的 SessionManager 适配器需后续实现。启用配置后守护线程会运行,
    /// 但实际执行会失败并记录 warn 日志。
    pub persistent_runner: Option<Arc<axagent_runtime::persistent_runner::PersistentRunner>>,

    /// 统一事件总线(跨 crate 事件流标准入口)。
    ///
    /// 由 wiring 层(src/init/state.rs)在构造 AppState 时实例化,
    /// 同一份 `Arc<dyn EventBus>` 注入到 agent / rt-workflow / orchestrator 三方,
    /// 供跨 crate 事件订阅者消费。未注入时三方保持原有行为。
    pub event_bus: Arc<dyn axagent_harness::EventBus>,

    /// 能力发现路由器（全链路编排：检索→过滤→排序→补全）
    pub capability_router: Arc<DefaultCapabilityRouter>,
    /// 能力索引器（用于注册/删除/查询能力护照）
    pub capability_indexer: Arc<CapabilityIndexerImpl>,
    /// 会话状态存储（CapabilityLoad 写入 / 下轮注入器读取的解耦点）
    pub session_state_store: Arc<dyn axagent_harness::SessionStateStore>,
    /// 认知编排器（三层路由树协调器，全局用户消息唯一入口）
    pub cognitive_router: Arc<dyn axagent_harness::CognitiveRouter>,
    /// 动态防护规则管理器（运行时注入，GuardRule/ExemptAuthorize 动态注入的入口）
    pub prompt_guard: Arc<PatternPromptGuard>,
    /// P3: 任务形态 LLM 兜底分类器（wiring 层注入，规则置信度不足时调用）
    pub task_shape_llm_classifier: Arc<dyn axagent_harness::TaskShapeLlmClassifier>,
    /// P3: ApprovalGate 审批 oneshot 通道（key = approval_id, value = Sender<bool>）
    pub task_shape_approval_senders:
        Arc<tokio::sync::Mutex<HashMap<String, tokio::sync::oneshot::Sender<bool>>>>,

    // ── Phase 3 P1 Task 3.1: domain decomposition ───────────────────────────
    // The six sub-state structs below provide a focused, composable view of
    // `AppState`. They are constructed at start-up with `Arc`/`Mutex` clones
    // of the corresponding top-level fields above, so the legacy call-sites
    // (200+ `commands/*` files) keep working unchanged. New code can opt
    // into the grouped accessors on these sub-states.  The fields are
    // `#[allow(dead_code)]` until the migration is complete.
    pub infra: InfraState,
    /// Renamed from `gateway` to `gateway_state` to avoid colliding with
    /// the existing `pub gateway: Arc<Mutex<Option<GatewayServer>>>` field
    /// above. Existing call-sites that read the gateway server handle
    /// continue to use the `gateway` field; new code can use
    /// `app_state.gateway_state` for the grouped gateway view.
    pub gateway_state: GatewayState,
    pub task: TaskState,
    pub agent: AgentState,
    pub memory: MemoryState,
    pub skill: SkillState,
    pub learning: LearningEngineState,
    pub tool: ToolState,
    /// 记忆写审批门配置 (P0-4)
    pub memory_write_approval_config:
        Arc<tokio::sync::RwLock<axagent_harness::memory::MemoryWriteApprovalConfig>>,
    /// 待审批的记忆写入列表 (P0-4)
    pub pending_memory_writes: Arc<
        tokio::sync::RwLock<Vec<(String, axagent_harness::memory::MemoryWriteApprovalRequest)>>,
    >,
}

impl Drop for AppState {
    fn drop(&mut self) {
        self.shutdown_token.cancel();
        tracing::info!("[shutdown] CancellationToken 已通知，后台任务应停止");
    }
}
