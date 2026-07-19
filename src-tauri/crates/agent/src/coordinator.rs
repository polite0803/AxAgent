// SPDX-License-Identifier: AGPL-3.0-only

use crate::event_bus::{AgentEventBus, AgentEventType, UnifiedAgentEvent};
use crate::reasoning_router::{self, ReasoningEngine, TaskFeatures};
use crate::steer_manager::SteerManager;
use crate::tree_of_thoughts::{LlmReasoningProvider as ToTReasoningProvider, TreeOfThoughtsEngine};
use async_trait::async_trait;
use axagent_harness::{SharedCacheService, SharedHookService};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use thiserror::Error;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Phase 4: 推理引擎选择来源常量
// ---------------------------------------------------------------------------

/// 默认（未显式配置，按 ReactEngine 兜底）
const ENGINE_SOURCE_DEFAULT: u32 = 0;
/// 通过 with_reasoning_router() 启用自动选择
const ENGINE_SOURCE_AUTO: u32 = 1;
/// 通过 with_reasoning_engine() 手动指定
const ENGINE_SOURCE_MANUAL: u32 = 2;

// 3.5 P2:删除 WorkerDefinition / WorkerMessage / WorkerStatus / WorkerResult 死代码。
// 子 Agent 概念的权威定义在 `axagent_trajectory::SubAgent`(持久化 + MessageBus),
// swarm 多 Agent 系统的工作者引导在 `axagent_runtime::worker_boot::WorkerStatus`。
// 历史上 coordinator.rs 内嵌的 Worker* 类型从未被 AgentCoordinator 引用,
// 保留只会造成三套概念混淆,违反 AGENTS.md 第 12 条「禁止重复定义」铁律。

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Initializing,
    Running,
    WaitingForConfirmation,
    Paused,
    Completed,
    Failed(String),
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Idle => write!(f, "Idle"),
            AgentStatus::Initializing => write!(f, "Initializing"),
            AgentStatus::Running => write!(f, "Running"),
            AgentStatus::WaitingForConfirmation => write!(f, "WaitingForConfirmation"),
            AgentStatus::Paused => write!(f, "Paused"),
            AgentStatus::Completed => write!(f, "Completed"),
            AgentStatus::Failed(msg) => write!(f, "Failed({})", msg),
        }
    }
}

/// 将 coordinator 本地 `AgentStatus` 转为 harness 层权威 `SessionStatus`。
///
/// 注意：`Failed(String)` 的错误详情会被丢弃（harness 枚举不携带载荷），
/// 调用方如需保留错误信息应单独传递。
impl From<AgentStatus> for axagent_harness::types::SessionStatus {
    fn from(status: AgentStatus) -> Self {
        match status {
            AgentStatus::Idle => Self::Idle,
            AgentStatus::Initializing => Self::Initializing,
            AgentStatus::Running => Self::Running,
            AgentStatus::WaitingForConfirmation => Self::WaitingApproval,
            AgentStatus::Paused => Self::Paused,
            AgentStatus::Completed => Self::Completed,
            AgentStatus::Failed(_) => Self::Failed,
        }
    }
}

// ---------------------------------------------------------------------------
// 状态机判别值（用于 lock-free 原子状态机）
// ---------------------------------------------------------------------------
//
// 真实状态机由 `AtomicU8` 驱动；`Arc<RwLock<AgentStatus>>` 仍保留以返回
// `Failed(String)` 等携带详情的状态给调用方。状态判别值映射：
//
// 0 = Idle
// 1 = Initializing
// 2 = Running
// 3 = WaitingForConfirmation
// 4 = Paused
// 5 = Completed
// 6 = Failed
//
// 所有比较/转换统一通过 `compare_exchange(SeqCst)` 完成，避免
// `RwLock` 写锁释放后并发 cancel/get_status 读到错乱中间态。

const STATE_IDLE: u8 = 0;
const STATE_INITIALIZING: u8 = 1;
const STATE_RUNNING: u8 = 2;
const STATE_WAITING_FOR_CONFIRMATION: u8 = 3;
const STATE_PAUSED: u8 = 4;
const STATE_COMPLETED: u8 = 5;
const STATE_FAILED: u8 = 6;

/// 将 `AgentStatus` 映射到原子判别值（`Failed(_)` 一律映射到 `STATE_FAILED`，详情保留在 RwLock 中）。
fn state_discriminant(status: &AgentStatus) -> u8 {
    match status {
        AgentStatus::Idle => STATE_IDLE,
        AgentStatus::Initializing => STATE_INITIALIZING,
        AgentStatus::Running => STATE_RUNNING,
        AgentStatus::WaitingForConfirmation => STATE_WAITING_FOR_CONFIRMATION,
        AgentStatus::Paused => STATE_PAUSED,
        AgentStatus::Completed => STATE_COMPLETED,
        AgentStatus::Failed(_) => STATE_FAILED,
    }
}

/// 由原子判别值构造无详情 `AgentStatus`（`Failed` 分支 detail 为空字符串，真实 detail 由 RwLock 提供）。
fn state_from_discriminant(value: u8) -> AgentStatus {
    match value {
        STATE_IDLE => AgentStatus::Idle,
        STATE_INITIALIZING => AgentStatus::Initializing,
        STATE_RUNNING => AgentStatus::Running,
        STATE_WAITING_FOR_CONFIRMATION => AgentStatus::WaitingForConfirmation,
        STATE_PAUSED => AgentStatus::Paused,
        STATE_COMPLETED => AgentStatus::Completed,
        STATE_FAILED => AgentStatus::Failed(String::new()),
        // 未知判别值兜底为 Idle，避免污染状态机
        _ => AgentStatus::Idle,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: usize,
    pub timeout_secs: Option<u64>,
    pub enable_self_verification: bool,
    pub enable_error_recovery: bool,
    pub require_plan_approval: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: axagent_harness::constants::DEFAULT_MAX_ITERATIONS,
            timeout_secs: Some(300),
            enable_self_verification: true,
            enable_error_recovery: true,
            require_plan_approval: false,
        }
    }
}

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent not initialized")]
    NotInitialized,
    #[error("Agent already running")]
    AlreadyRunning,
    #[error("Agent is in invalid state: {0}")]
    InvalidState(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    pub content: String,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorOutput {
    pub content: String,
    pub status: AgentStatus,
    pub iterations: usize,
    pub metadata: serde_json::Value,
}

impl CoordinatorOutput {
    pub fn success(content: String, iterations: usize) -> Self {
        Self {
            content,
            status: AgentStatus::Completed,
            iterations,
            metadata: serde_json::json!({}),
        }
    }

    pub fn failure(message: String, iterations: usize) -> Self {
        Self {
            content: message.clone(),
            status: AgentStatus::Failed(message),
            iterations,
            metadata: serde_json::json!({}),
        }
    }
}

#[async_trait]
pub trait AgentImpl: Send + Sync {
    async fn initialize(&mut self, config: AgentConfig) -> Result<(), AgentError>;
    async fn execute(&mut self, input: AgentInput) -> Result<CoordinatorOutput, AgentError>;
    async fn pause(&mut self) -> Result<(), AgentError>;
    async fn resume(&mut self) -> Result<(), AgentError>;
    async fn cancel(&mut self) -> Result<(), AgentError>;
    fn status(&self) -> AgentStatus;
    fn agent_type(&self) -> &'static str;
}

pub struct AgentCoordinator<T: AgentImpl> {
    /// 状态机判别值（lock-free 状态机；具体含义见 `STATE_*` 常量注释）。
    /// 状态转换通过 `compare_exchange(SeqCst)` 完成，避免 `RwLock` 写锁
    /// 释放后并发 cancel/get_status 读到错乱中间态。
    state: Arc<AtomicU8>,
    /// 完整 `AgentStatus`（含 `Failed(String)` 详情），由 atomic 状态机驱动刷新。
    status: Arc<RwLock<AgentStatus>>,
    config: Arc<RwLock<AgentConfig>>,
    implementation: Arc<tokio::sync::Mutex<T>>,
    event_bus: Arc<AgentEventBus>,
    correlation_counter: std::sync::atomic::AtomicU64,
    pub cache_service: SharedCacheService,
    pub hook_chain: SharedHookService,
    pub steer_manager: Arc<SteerManager>,
    tot_engine: Arc<tokio::sync::Mutex<Option<TreeOfThoughtsEngine>>>,
    /// Phase 4: 推理策略自动选择路由器
    reasoning_engine: Arc<RwLock<ReasoningEngine>>,
    /// 从 execute() 输入中自动提取的任务特征（用于决策追溯）
    current_task_features: Arc<RwLock<Option<TaskFeatures>>>,
    /// 推理引擎选择来源：auto（自动选择）或 manual（手动指定）
    engine_selection_source: Arc<AtomicU32>,
    /// P0-2：计划确认闸门挂起的原始输入。
    /// 仅当 `require_plan_approval` 开启且任务被判定为复杂、进入
    /// `WaitingForConfirmation` 时暂存，供 `approve_plan()` 取回后执行。
    pending_input: Arc<RwLock<Option<AgentInput>>>,
}

/// P0-2 复用入口：模块级纯函数，供生产命令层（`agent_query`）直接调用，
/// 无需持有 `AgentCoordinator` 实例。判定任务是否复杂到需要计划确认闸门。
///
/// 与 `reasoning_router` 共享同一套特征提取逻辑；只要任务被路由到
/// 非 ReAct 引擎（分支/验证类），或多步（节点数 > 1）、多工具调用
/// （轮数 > 1），即视为复杂，需要用户确认后再执行。
pub fn plan_requires_approval(content: &str) -> bool {
    let (engine, features) = reasoning_router::auto_select_engine(content);
    engine != ReasoningEngine::ReactEngine
        || features.node_count > 1
        || features.estimated_tool_rounds > 1
}

/// P0-2 复用入口：基于任务特征生成一份人类可读的计划草稿（供用户确认）。
///
/// 草稿含：任务预览、自动选择的推理引擎、以及从 `reasoning_router`
/// 提取的结构化特征。前端可据此渲染确认 UI。
pub fn build_plan_draft_content(content: &str) -> String {
    let (engine, features) = reasoning_router::auto_select_engine(content);
    let preview: String = content.chars().take(200).collect();
    let plan = serde_json::json!({
        "task_preview": preview,
        "selected_engine": engine.to_string(),
        "features": {
            "node_count": features.node_count,
            "estimated_tool_rounds": features.estimated_tool_rounds,
            "requires_verification": features.requires_verification,
            "has_branches": features.has_branches,
            "has_conditions": features.has_conditions,
        },
        "note": "任务已被判定为复杂任务，请确认后执行。",
    });
    serde_json::to_string_pretty(&plan).unwrap_or(preview)
}

impl<T: AgentImpl> AgentCoordinator<T> {
    pub fn new(
        implementation: Arc<tokio::sync::Mutex<T>>,
        event_bus: Option<Arc<AgentEventBus>>,
        cache_service: SharedCacheService,
        hook_chain: SharedHookService,
    ) -> Self {
        let event_bus =
            event_bus.unwrap_or_else(|| Arc::new(AgentEventBus::new("typed_coordinator")));

        Self {
            state: Arc::new(AtomicU8::new(STATE_IDLE)),
            status: Arc::new(RwLock::new(AgentStatus::Idle)),
            config: Arc::new(RwLock::new(AgentConfig::default())),
            implementation,
            event_bus,
            correlation_counter: std::sync::atomic::AtomicU64::new(0),
            cache_service,
            hook_chain,
            steer_manager: Arc::new(SteerManager::new()),
            tot_engine: Arc::new(tokio::sync::Mutex::new(None)),
            reasoning_engine: Arc::new(RwLock::new(ReasoningEngine::ReactEngine)),
            current_task_features: Arc::new(RwLock::new(None)),
            engine_selection_source: Arc::new(AtomicU32::new(ENGINE_SOURCE_DEFAULT)),
            pending_input: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_tot_engine(self, engine: TreeOfThoughtsEngine) -> Self {
        *self.tot_engine.blocking_lock() = Some(engine);
        self
    }

    /// 启用推理策略自动选择路由器。
    ///
    /// 每次 execute() 被调用时，自动从任务描述中提取特征并选择最优推理引擎。
    /// 如果不调用此方法，默认使用 ReactEngine。
    pub fn with_reasoning_router(self) -> Self {
        self.engine_selection_source.store(ENGINE_SOURCE_AUTO, Ordering::Release);
        self
    }

    /// 手动指定推理引擎（覆盖自动选择）。
    pub fn with_reasoning_engine(self, engine: ReasoningEngine) -> Self {
        *self.reasoning_engine.blocking_write() = engine;
        self.engine_selection_source.store(ENGINE_SOURCE_MANUAL, Ordering::Release);
        self
    }

    /// 获取当前推理引擎类型
    pub async fn current_reasoning_engine(&self) -> ReasoningEngine {
        *self.reasoning_engine.read().await
    }

    /// 获取最近一次任务特征（用于诊断）
    pub async fn current_task_features(&self) -> Option<TaskFeatures> {
        self.current_task_features.read().await.clone()
    }

    /// 使用 Tree of Thoughts 探索多路径推理。
    ///
    /// Phase 4: 在执行前检查当前推理引擎是否支持 ToT，不支持则跳过。
    pub async fn reason_with_tot(
        &self,
        _problem: &str,
        context: &str,
        provider: &Arc<dyn ToTReasoningProvider>,
    ) -> Option<Vec<String>> {
        let current_engine = *self.reasoning_engine.read().await;
        if current_engine != ReasoningEngine::TreeOfThoughts {
            tracing::debug!(
                engine = %current_engine,
                "Current engine != TreeOfThoughts, skipping ToT reasoning"
            );
            return None;
        }
        let mut engine_guard = self.tot_engine.lock().await;
        let engine = engine_guard.as_mut()?;

        let root_id = engine.root_id.clone();
        let child_ids = engine.generate_branching_options(root_id, context, provider).await.ok()?;

        let mut scored_ids = Vec::new();
        for child_id in &child_ids {
            if let Ok(score) = engine.evaluate_and_score_node(child_id, context, provider).await {
                scored_ids.push((child_id.clone(), score));
            }
        }

        engine.prune_below_threshold(0.3);

        let best_path = engine.select_best_path();
        if !best_path.is_empty() {
            tracing::info!(
                path_length = best_path.len(),
                "Tree of Thoughts selected best reasoning path"
            );
        }

        Some(best_path)
    }

    /// P0-1：可验证性硬约束（证据锚定）。
    ///
    /// `enable_self_verification` 开启时，对已完成的一轮做证据链校验：
    /// 最终输出为空即视为「无法提供证据链」，判定任务失败、拒绝交付。
    ///
    /// 说明：步骤级的事实/一致性校验由 ReAct 引擎的 `SelfVerifier` 在循环内
    /// 实时执行（其 `verification_enabled` 默认开启）；此处作为协调器层的
    /// 交付前最后一道闸门，强制「无证据即拒交付」。
    /// 更细粒度的「轨迹已验证步骤数」证据门禁需要在 ReAct 引擎向
    /// `CoordinatorOutput.metadata` 暴露 `verified_steps`，列为后续增强。
    async fn enforce_evidence_anchor(&self, output: &CoordinatorOutput) -> Option<AgentError> {
        if !self.config.read().await.enable_self_verification {
            return None;
        }
        if output.content.trim().is_empty() {
            tracing::warn!(
                "self_verification: empty final output, refusing delivery (no evidence chain)"
            );
            return Some(AgentError::ExecutionFailed(
                "验证失败：最终输出为空，无法提供证据链，拒绝交付".into(),
            ));
        }
        None
    }

    /// # 锁顺序约定
    ///
    /// 本方法获取 `self.implementation.lock()`。全局锁顺序约定：**始终先锁 tot_engine 再锁 implementation**。
    pub async fn initialize(&self, config: AgentConfig) -> Result<(), AgentError> {
        // 1. 原子守卫：仅允许从 Idle 进入 Initializing；并发进入返回 InvalidState
        if !self.try_transition(&[STATE_IDLE], STATE_INITIALIZING) {
            let current = self.current_state();
            return Err(AgentError::InvalidState(format!(
                "Cannot initialize from state {}",
                state_from_discriminant(current)
            )));
        }
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Initializing;
        }

        // 2. 调用实现；失败时复位状态为 Idle，再传播错误（避免卡在 Initializing）
        let init_result = {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.initialize(config.clone()).await
        };
        if let Err(err) = init_result {
            self.set_state(STATE_IDLE);
            {
                let mut status = self.status.write().await;
                *status = AgentStatus::Idle;
            }
            return Err(err);
        }

        // 3. 成功：原子置 Idle，刷新 config，发出 StateChanged 事件
        self.set_state(STATE_IDLE);
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Idle;
        }
        let mut cfg = self.config.write().await;
        *cfg = config;

        self.emit_event(
            AgentEventType::StateChanged,
            serde_json::json!({
                "previous": "Initializing",
                "current": "Idle"
            }),
        )
        .await;

        Ok(())
    }

    /// 将外部指令列表同步到内部 SteerManager。
    ///
    /// 调用方（commands/agent/mod.rs）在 execute 前将 AppState.steer_queue
    /// 中的指定 conversation 指令取出来注入到此方法。
    pub async fn sync_steer_queue(&self, instructions: Vec<String>) {
        if !instructions.is_empty() {
            let count = self.steer_manager.extend(instructions).await.len();
            tracing::info!("[coordinator] synced {count} steer instruction(s) from external queue");
        }
    }

    /// 对话级执行入口。
    ///
    /// # P0-2 计划确认闸门
    /// 当 `require_plan_approval` 开启且任务被判定为复杂时，不立即执行，
    /// 而是将状态机切到 `WaitingForConfirmation`，暂存原始输入，并发出
    /// `PlanReadyForApproval` 事件（含计划草稿）。调用方（命令层 / 前端）
    /// 需调 [`Self::approve_plan`] 确认后才真正执行。
    ///
    /// 该特性默认关闭（`require_plan_approval = false`），关闭时行为与
    /// 改造前完全一致——零行为变化。
    ///
    /// # 锁顺序约定
    ///
    /// 本方法获取 `self.implementation.lock()`。全局锁顺序约定：**始终先锁 tot_engine 再锁 implementation**。
    pub async fn execute(&self, input: AgentInput) -> Result<CoordinatorOutput, AgentError> {
        // 0. 从外部 steer_queue 同步指令由调用方（commands/agent/mod.rs）
        //    在 execute 前通过 self.sync_steer_queue() 完成。

        // 1. 原子守卫：仅允许从 Idle|Paused 进入 Running；并发进入时
        //    - 当前已是 Running → AlreadyRunning
        //    - 其余状态 → InvalidState
        if !self.try_transition(&[STATE_IDLE, STATE_PAUSED], STATE_RUNNING) {
            let current = self.current_state();
            if current == STATE_RUNNING {
                return Err(AgentError::AlreadyRunning);
            }
            return Err(AgentError::InvalidState(format!(
                "Cannot execute from state {}",
                state_from_discriminant(current)
            )));
        }
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Running;
        }

        // 2. P0-2：计划确认闸门——开启且任务复杂时，进入等待确认而非直接执行
        let require_approval = self.config.read().await.require_plan_approval;
        if require_approval
            && self.is_complex_task(&input).await
            && self.try_transition(&[STATE_RUNNING], STATE_WAITING_FOR_CONFIRMATION)
        {
            let draft = self.build_plan_draft(&input).await;
            {
                let mut status = self.status.write().await;
                *status = AgentStatus::WaitingForConfirmation;
            }
            // 暂存原始输入，供 approve_plan() 取回后执行
            {
                let mut pending = self.pending_input.write().await;
                *pending = Some(input);
            }
            self.emit_event(
                AgentEventType::PlanReadyForApproval,
                serde_json::json!({
                    "plan": draft,
                }),
            )
            .await;
            return Ok(CoordinatorOutput {
                content: draft,
                status: AgentStatus::WaitingForConfirmation,
                iterations: 0,
                metadata: serde_json::json!({ "awaiting_approval": true }),
            });
        }

        // 3. 常规路径：直接执行
        self.run_impl(input).await
    }

    /// 真正的执行核心，被 [`Self::execute`] 与 [`Self::approve_plan`] 共用。
    ///
    /// 假定调用方已完成状态机进入 `Running` 的守卫。本方法负责：
    /// steer 注入 → 推理引擎选择 → TurnStarted 事件 → `impl.execute` →
    /// 证据锚定闸门（P0-1）→ 终态写回。
    ///
    /// # 锁顺序约定
    ///
    /// 本方法获取 `self.implementation.lock()`。全局锁顺序约定：**始终先锁 tot_engine 再锁 implementation**。
    async fn run_impl(&self, input: AgentInput) -> Result<CoordinatorOutput, AgentError> {
        let mut input = input;
        if self.steer_manager.has_pending().await
            && let Some(steer_block) = self.steer_manager.format_steer_block().await
        {
            let mut ctx = input
                .context
                .take()
                .and_then(|v| {
                    serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(v).ok()
                })
                .unwrap_or_default();
            ctx.insert("steer".to_string(), serde_json::json!(steer_block));
            input.context = Some(serde_json::Value::Object(ctx));
            tracing::info!("Injecting steer instructions into agent turn");
        }

        // Phase 4: 推理策略自动选择路由器
        // ——如果是 auto 或 manual 模式，在每次 execute 时重新评估
        let source = self.engine_selection_source.load(Ordering::Acquire);
        let selected_engine = if source == ENGINE_SOURCE_AUTO {
            let (engine, features) = reasoning_router::auto_select_engine(&input.content);
            {
                let mut eng = self.reasoning_engine.write().await;
                *eng = engine;
            }
            {
                let mut ft = self.current_task_features.write().await;
                *ft = Some(features);
            }
            tracing::info!(
                engine = %engine,
                "Reasoning router auto-selected engine"
            );
            engine
        } else {
            *self.reasoning_engine.read().await
        };

        // For complex tasks that require multi-path reasoning, use Tree of Thoughts
        // when tot_engine is available AND router recommends it
        let _should_use_tot = selected_engine == ReasoningEngine::TreeOfThoughts
            && self.tot_engine.lock().await.is_some();

        let cache_was_valid = self.cache_service.is_cache_valid().await;
        self.emit_event(
            AgentEventType::TurnStarted,
            serde_json::json!({
                "input_preview": input.content.chars().take(100).collect::<String>(),
                "cache_valid": cache_was_valid,
                "has_pending_changes": self.cache_service.has_pending_changes().await,
                "reasoning_engine": selected_engine.to_string(),
                "engine_source": if source == ENGINE_SOURCE_AUTO { "auto" } else if source == ENGINE_SOURCE_MANUAL { "manual" } else { "default" },
            }),
        )
        .await;

        let correlation_id = self.next_correlation_id();
        let result = {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.execute(input).await
        };

        // P0-1：可验证性硬约束——启用时，最终输出为空即拒绝交付（无证据链）
        let result = match result {
            Ok(output) => {
                if let Some(err) = self.enforce_evidence_anchor(&output).await {
                    self.emit_event(
                        AgentEventType::Error,
                        serde_json::json!({
                            "correlation_id": correlation_id,
                            "verification": "failed",
                            "reason": "empty_output_no_evidence",
                        }),
                    )
                    .await;
                    Err(err)
                } else {
                    Ok(output)
                }
            },
            err => err,
        };

        // 4. 写回终态：以 atomic 为准，detail 走 RwLock
        match &result {
            Ok(output) => {
                self.set_state(state_discriminant(&output.status));
                {
                    let mut status = self.status.write().await;
                    *status = output.status.clone();
                }
                self.emit_event(
                    AgentEventType::TurnCompleted,
                    serde_json::json!({
                        "correlation_id": correlation_id,
                        "iterations": output.iterations,
                        "status": output.status.to_string(),
                        "cache_was_valid": cache_was_valid,
                    }),
                )
                .await;
            },
            Err(e) => {
                self.set_state(STATE_FAILED);
                {
                    let mut status = self.status.write().await;
                    *status = AgentStatus::Failed(e.to_string());
                }
                self.emit_event(
                    AgentEventType::Error,
                    serde_json::json!({
                        "correlation_id": correlation_id,
                        "error": e.to_string(),
                        "cache_was_valid": cache_was_valid,
                    }),
                )
                .await;
            },
        }

        result
    }

    /// P0-2：在计划确认闸门后批准执行。
    ///
    /// 仅当状态机处于 `WaitingForConfirmation` 时有效；通过后切到 `Running`
    /// 并取回挂起的输入，调用 [`Self::run_impl`] 真正执行。
    pub async fn approve_plan(&self) -> Result<CoordinatorOutput, AgentError> {
        if !self.try_transition(&[STATE_WAITING_FOR_CONFIRMATION], STATE_RUNNING) {
            let current = self.current_state();
            return Err(AgentError::InvalidState(format!(
                "Cannot approve plan from state {}",
                state_from_discriminant(current)
            )));
        }
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Running;
        }

        let input = {
            let mut pending = self.pending_input.write().await;
            pending.take()
        };
        let input = match input {
            Some(input) => input,
            None => {
                // 无挂起输入（异常路径），复位状态避免卡死
                self.set_state(STATE_IDLE);
                {
                    let mut status = self.status.write().await;
                    *status = AgentStatus::Idle;
                }
                return Err(AgentError::InvalidState(
                    "无待批准的计划输入（pending_input 为空）".into(),
                ));
            },
        };

        self.emit_event(
            AgentEventType::StateChanged,
            serde_json::json!({ "from": "WaitingForConfirmation", "to": "Running" }),
        )
        .await;

        self.run_impl(input).await
    }

    /// P0-2：判定任务是否复杂到需要计划确认闸门。
    ///
    /// 与 `reasoning_router` 共享同一套特征提取逻辑；只要任务被路由到
    /// 非 ReAct 引擎（分支/验证类），或多步（节点数 > 1）、多工具调用
    /// （轮数 > 1），即视为复杂，需要用户确认后再执行。
    async fn is_complex_task(&self, input: &AgentInput) -> bool {
        plan_requires_approval(&input.content)
    }

    /// P0-2：基于任务特征生成一份人类可读的计划草稿（供用户确认）。
    ///
    /// 草稿含：任务预览、自动选择的推理引擎、以及从 `reasoning_router`
    /// 提取的结构化特征。前端可据此渲染确认 UI。
    async fn build_plan_draft(&self, input: &AgentInput) -> String {
        build_plan_draft_content(&input.content)
    }

    pub async fn force_now(&self) {
        self.cache_service.set_force_immediate(true).await;
        self.cache_service.invalidate("--now flag: immediate invalidation").await;
    }

    pub async fn prepare_for_new_session(&self) {
        self.cache_service.invalidate_for_new_session().await;
        self.cache_service.set_force_immediate(false).await;
    }

    /// # 锁顺序约定
    ///
    /// 本方法获取 `self.implementation.lock()`。全局锁顺序约定：**始终先锁 tot_engine 再锁 implementation**。
    pub async fn pause(&self) -> Result<(), AgentError> {
        // 1. 原子检查：仅当状态为 Running 时才进入（不预占状态，避免失败后回滚）
        let current = self.current_state();
        if current != STATE_RUNNING {
            return Err(AgentError::InvalidState(format!(
                "Cannot pause from state {}",
                state_from_discriminant(current)
            )));
        }

        // 2. 调用实现，失败时原状态（Running）保持不变
        {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.pause().await?;
        }

        // 3. 成功：原子置 Paused，刷新 detail
        self.set_state(STATE_PAUSED);
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Paused;
        }

        self.emit_event(
            AgentEventType::StateChanged,
            serde_json::json!({
                "from": "Running",
                "to": "Paused"
            }),
        )
        .await;

        Ok(())
    }

    /// # 锁顺序约定
    ///
    /// 本方法获取 `self.implementation.lock()`。全局锁顺序约定：**始终先锁 tot_engine 再锁 implementation**。
    pub async fn resume(&self) -> Result<(), AgentError> {
        // 1. 原子检查：仅当状态为 Paused 时才进入
        let current = self.current_state();
        if current != STATE_PAUSED {
            return Err(AgentError::InvalidState(format!(
                "Cannot resume from state {}",
                state_from_discriminant(current)
            )));
        }

        // 2. 调用实现，失败时原状态（Paused）保持不变
        {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.resume().await?;
        }

        // 3. 成功：原子置 Running，刷新 detail
        self.set_state(STATE_RUNNING);
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Running;
        }

        self.emit_event(
            AgentEventType::StateChanged,
            serde_json::json!({
                "from": "Paused",
                "to": "Running"
            }),
        )
        .await;

        Ok(())
    }

    /// # 锁顺序约定
    ///
    /// 本方法获取 `self.implementation.lock()`。全局锁顺序约定：**始终先锁 tot_engine 再锁 implementation**。
    pub async fn cancel(&self) -> Result<(), AgentError> {
        // 0. 状态守卫：仅允许 Running / Paused / WaitingForConfirmation 进入取消流程
        let current = self.current_state();
        if !matches!(current, STATE_RUNNING | STATE_PAUSED | STATE_WAITING_FOR_CONFIRMATION) {
            return Err(AgentError::InvalidState(format!(
                "Cannot cancel agent in state: {}",
                current
            )));
        }

        // 1. 调用实现；失败时原状态保持不变，错误直接传播（避免掩盖实现层错误）
        {
            let mut impl_guard = self.implementation.lock().await;
            impl_guard.cancel().await?;
        }

        // 2. 成功：原子置 Idle，刷新 detail
        self.set_state(STATE_IDLE);
        {
            let mut status = self.status.write().await;
            *status = AgentStatus::Idle;
        }
        // P0-2：若从确认闸门取消，清理挂起的输入，避免残留
        {
            let mut pending = self.pending_input.write().await;
            *pending = None;
        }

        self.emit_event(
            AgentEventType::StateChanged,
            serde_json::json!({
                "to": "Idle"
            }),
        )
        .await;

        Ok(())
    }

    pub async fn get_status(&self) -> AgentStatus {
        // 先读 atomic 判别值，以它为准；RwLock 仅补充 detail
        let disc = self.current_state();
        match disc {
            STATE_IDLE => AgentStatus::Idle,
            STATE_RUNNING => self.status.read().await.clone(),
            STATE_PAUSED => AgentStatus::Paused,
            STATE_WAITING_FOR_CONFIRMATION => AgentStatus::WaitingForConfirmation,
            STATE_COMPLETED => AgentStatus::Completed,
            STATE_FAILED => self.status.read().await.clone(),
            STATE_INITIALIZING => AgentStatus::Initializing,
            _ => AgentStatus::Idle,
        }
    }

    pub fn event_bus(&self) -> Arc<AgentEventBus> {
        Arc::clone(&self.event_bus)
    }

    fn next_correlation_id(&self) -> u64 {
        self.correlation_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// 读取当前状态判别值（SeqCst load）。
    fn current_state(&self) -> u8 {
        self.state.load(Ordering::SeqCst)
    }

    /// 原子地、无锁地将状态从 `from` 中的任一判别值转换为 `to`。
    ///
    /// 使用 `compare_exchange` 循环避免 ABA；当前判别值不在
    /// `from` 中或被并发修改时返回 `false`，由调用方决定如何响应。
    fn try_transition(&self, from: &[u8], to: u8) -> bool {
        let mut current = self.current_state();
        loop {
            if !from.contains(&current) {
                return false;
            }
            match self.state.compare_exchange(current, to, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    /// 无条件原子地写入状态判别值（仅在锁/事件已正确同步时使用）。
    fn set_state(&self, to: u8) {
        self.state.store(to, Ordering::SeqCst);
    }

    async fn emit_event(&self, event_type: AgentEventType, payload: serde_json::Value) {
        let event = UnifiedAgentEvent::new("AgentCoordinator", event_type, payload);
        if let Err(e) = self.event_bus.emit(event) {
            tracing::warn!("Failed to emit event: {:?}", e);
        }
    }
}

impl<T: AgentImpl> std::fmt::Debug for AgentCoordinator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentCoordinator").field("event_bus", &self.event_bus.name()).finish()
    }
}
