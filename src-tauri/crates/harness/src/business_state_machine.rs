// SPDX-License-Identifier: AGPL-3.0-only

//! 执行层：业务状态机（Business State Machine）
//!
//! 本模块定义工作流的"刚性轨道"——业务状态机（FSM），
//! 与运行时 Typestate 分离，实现"刚性协议 + 柔性节点"架构。
//!
//! # 核心理念
//! - **刚性轨道**：状态转移路径由业务规则硬编码，绝对不走样
//! - **柔性执行**：状态内部的具体执行由 Agent 动态调度
//! - **编译时保证**：非法状态转移在编译期被阻止
//!
//! # 架构定位
//! - 定义在 harness 层（foundation），纯数据定义
//! - 运行时状态机执行逻辑由 rt-workflow 实现
//! - 与 NodeStatus（运行时状态）正交：FSM 管业务流程，NodeStatus 管节点执行

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ── 业务状态定义 ──

/// 业务状态 ID（稳定标识，用于序列化和跨进程传递）
pub type BusinessStateId = String;

/// 业务状态机定义
///
/// # 使用示例
/// ```ignore
/// let fsm = BusinessStateMachine::new("order_processing")
///     .with_state(BusinessState::new("created")
///         .with_label("已创建")
///         .with_node_ref("node_create_order"))
///     .with_state(BusinessState::new("approved")
///         .with_label("已审批")
///         .with_node_ref("node_approve"))
///     .with_state(BusinessState::new("completed")
///         .with_label("已完成")
///         .as_terminal())
///     .with_transition(StateTransition::new("created", "approved")
///         .with_guard(|ctx| ctx.has_role("manager")))
///     .with_transition(StateTransition::new("approved", "completed"));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BusinessStateMachine {
    /// 状态机 ID
    pub id: String,
    /// 状态机名称
    pub name: String,
    /// 状态列表
    pub states: Vec<BusinessState>,
    /// 转移规则列表
    pub transitions: Vec<StateTransition>,
    /// 初始状态 ID
    #[serde(alias = "initial_state_id")]
    pub initial_state_id: BusinessStateId,
    /// 版本号
    #[serde(default = "default_fsm_version")]
    pub version: u32,
}

fn default_fsm_version() -> u32 {
    1
}

impl BusinessStateMachine {
    /// 创建新的业务状态机
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            states: Vec::new(),
            transitions: Vec::new(),
            initial_state_id: String::new(),
            version: default_fsm_version(),
        }
    }

    /// 添加状态
    pub fn with_state(mut self, state: BusinessState) -> Self {
        if self.states.is_empty() {
            self.initial_state_id = state.id.clone();
        }
        self.states.push(state);
        self
    }

    /// 添加转移规则
    pub fn with_transition(mut self, transition: StateTransition) -> Self {
        self.transitions.push(transition);
        self
    }

    /// 设置初始状态
    pub fn with_initial_state(mut self, state_id: impl Into<String>) -> Self {
        self.initial_state_id = state_id.into();
        self
    }

    /// 查找状态
    pub fn find_state(&self, id: &str) -> Option<&BusinessState> {
        self.states.iter().find(|s| s.id == id)
    }

    /// 获取合法的后续状态
    pub fn next_states(&self, current_id: &str) -> Vec<&StateTransition> {
        self.transitions.iter().filter(|t| t.from == current_id).collect()
    }

    /// 校验转移是否合法（不检查守卫条件）
    pub fn is_valid_transition(&self, from: &str, to: &str) -> bool {
        self.transitions.iter().any(|t| t.from == from && t.to == to)
    }

    /// 校验状态机完整性
    pub fn validate(&self) -> Result<(), FsmValidationError> {
        // 1. 初始状态必须存在
        if !self.states.iter().any(|s| s.id == self.initial_state_id) {
            return Err(FsmValidationError::InvalidInitialState(self.initial_state_id.clone()));
        }

        // 2. 所有转移的 from/to 状态必须存在
        for transition in &self.transitions {
            if !self.states.iter().any(|s| s.id == transition.from) {
                return Err(FsmValidationError::InvalidTransitionFrom(transition.from.clone()));
            }
            if !self.states.iter().any(|s| s.id == transition.to) {
                return Err(FsmValidationError::InvalidTransitionTo(transition.to.clone()));
            }
        }

        // 3. 至少有一个终态
        if !self.states.iter().any(|s| s.is_terminal) {
            return Err(FsmValidationError::NoTerminalState);
        }

        Ok(())
    }
}

/// 业务状态定义
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BusinessState {
    /// 状态 ID（唯一标识）
    pub id: BusinessStateId,
    /// 状态显示名称
    pub label: String,
    /// 绑定的工作流节点 ID（可选，节点执行时使用）
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "node_ref")]
    pub node_ref: Option<String>,
    /// 是否为终态（不可再转移）
    #[serde(default, alias = "is_terminal")]
    pub is_terminal: bool,
    /// 状态描述
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 进入该状态时可调用的工具白名单（空=不限制）
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "allowed_tools")]
    pub allowed_tools: Option<Vec<String>>,
    /// 状态优先级（用于冲突解决）
    #[serde(default)]
    pub priority: u32,
    /// 自定义属性（业务扩展）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl BusinessState {
    /// 创建新状态
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: String::new(),
            node_ref: None,
            is_terminal: false,
            description: None,
            allowed_tools: None,
            priority: 0,
            metadata: None,
        }
    }

    /// 设置显示名称
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// 绑定工作流节点
    pub fn with_node_ref(mut self, node_id: impl Into<String>) -> Self {
        self.node_ref = Some(node_id.into());
        self
    }

    /// 标记为终态
    pub fn as_terminal(mut self) -> Self {
        self.is_terminal = true;
        self
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// 设置工具白名单
    pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = Some(tools);
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// 设置元数据
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// 状态转移规则
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StateTransition {
    /// 源状态 ID
    pub from: BusinessStateId,
    /// 目标状态 ID
    pub to: BusinessStateId,
    /// 转移 ID（唯一标识）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 转移条件描述（人类可读）
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "guard_description")]
    pub guard_description: Option<String>,
    /// 守卫条件表达式（Rhai 脚本，返回 true 才能转移）
    ///
    /// # 示例
    /// ```text
    /// // 订单金额大于 1000 才能进入审批
    /// variables.amount > 1000 && user_role == "manager"
    ///
    /// // 必须由特定角色触发
    /// user_role == "admin"
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "guard_expr")]
    pub guard_expr: Option<String>,
    /// 是否需要审批
    #[serde(default, alias = "requires_approval")]
    pub requires_approval: bool,
    /// 转移优先级（用于冲突解决）
    #[serde(default)]
    pub priority: u32,
    /// 转移触发事件类型（可选）
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "trigger_event")]
    pub trigger_event: Option<String>,
}

impl StateTransition {
    /// 创建新转移
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            id: None,
            guard_description: None,
            guard_expr: None,
            requires_approval: false,
            priority: 0,
            trigger_event: None,
        }
    }

    /// 设置转移 ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// 设置守卫条件描述
    pub fn with_guard_description(mut self, desc: impl Into<String>) -> Self {
        self.guard_description = Some(desc.into());
        self
    }

    /// 设置守卫条件表达式（Rhai 脚本）
    pub fn with_guard_expr(mut self, expr: impl Into<String>) -> Self {
        self.guard_expr = Some(expr.into());
        self
    }

    /// 设置需要审批
    pub fn requires_approval(mut self, required: bool) -> Self {
        self.requires_approval = required;
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// 设置触发事件
    pub fn with_trigger_event(mut self, event: impl Into<String>) -> Self {
        self.trigger_event = Some(event.into());
        self
    }

    /// 检查是否有守卫条件
    pub fn has_guard(&self) -> bool {
        self.guard_expr.is_some()
    }
}

// ── FSM 运行时上下文 ──

/// 状态机运行时上下文（用于守卫条件评估）
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsmContext {
    /// 触发转移的事件
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// 触发事件的数据
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "event_data")]
    pub event_data: Option<serde_json::Value>,
    /// 当前用户角色
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "user_role")]
    pub user_role: Option<String>,
    /// 自定义变量（用于守卫条件评估）
    #[serde(default)]
    pub variables: serde_json::Map<String, serde_json::Value>,
}

impl FsmContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置事件
    pub fn with_event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }

    /// 设置事件数据
    pub fn with_event_data(mut self, data: serde_json::Value) -> Self {
        self.event_data = Some(data);
        self
    }

    /// 设置用户角色
    pub fn with_user_role(mut self, role: impl Into<String>) -> Self {
        self.user_role = Some(role.into());
        self
    }

    /// 添加变量
    pub fn with_variable(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.variables.insert(key.into(), value);
        self
    }

    /// 检查是否有指定角色
    pub fn has_role(&self, role: &str) -> bool {
        self.user_role.as_deref() == Some(role)
    }

    /// 获取变量值
    pub fn get_variable(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }
}

// ── 状态机运行时状态 ──

/// 状态机运行时状态（持久化）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsmRuntimeState {
    /// 当前状态 ID
    #[serde(alias = "current_state_id")]
    pub current_state_id: BusinessStateId,
    /// 上一个状态 ID
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "previous_state_id")]
    pub previous_state_id: Option<BusinessStateId>,
    /// 转移历史（用于时间旅行）
    #[serde(default, alias = "transition_history")]
    pub transition_history: Vec<FsmTransitionRecord>,
    /// 状态机实例 ID
    #[serde(alias = "instance_id")]
    pub instance_id: String,
    /// 状态机定义 ID
    #[serde(alias = "fsm_id")]
    pub fsm_id: String,
    /// 创建时间戳（毫秒）
    #[serde(alias = "created_at_ms")]
    pub created_at_ms: u64,
    /// 最后更新时间戳（毫秒）
    #[serde(alias = "updated_at_ms")]
    pub updated_at_ms: u64,
    /// 是否完成
    #[serde(default, alias = "is_completed")]
    pub is_completed: bool,
}

impl FsmRuntimeState {
    pub fn new(
        instance_id: impl Into<String>,
        fsm_id: impl Into<String>,
        initial_state_id: impl Into<String>,
    ) -> Self {
        let now_ms = current_timestamp_ms();
        Self {
            current_state_id: initial_state_id.into(),
            previous_state_id: None,
            transition_history: Vec::new(),
            instance_id: instance_id.into(),
            fsm_id: fsm_id.into(),
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
            is_completed: false,
        }
    }

    /// 尝试转移到新状态
    pub fn try_transition(
        &mut self,
        target_state_id: &str,
        fsm: &BusinessStateMachine,
    ) -> Result<FsmTransitionRecord, FsmTransitionError> {
        // 1. 检查状态机是否完成
        if self.is_completed {
            return Err(FsmTransitionError::MachineCompleted);
        }

        // 2. 检查转移是否合法
        if !fsm.is_valid_transition(&self.current_state_id, target_state_id) {
            return Err(FsmTransitionError::InvalidTransition {
                from: self.current_state_id.clone(),
                to: target_state_id.to_string(),
            });
        }

        // 3. 获取目标状态
        let target_state = fsm
            .find_state(target_state_id)
            .ok_or(FsmTransitionError::StateNotFound(target_state_id.to_string()))?;

        // 4. 记录转移
        let now_ms = current_timestamp_ms();
        let record = FsmTransitionRecord {
            from: self.current_state_id.clone(),
            to: target_state_id.to_string(),
            timestamp_ms: now_ms,
        };

        self.previous_state_id = Some(self.current_state_id.clone());
        self.current_state_id = target_state_id.to_string();
        self.transition_history.push(record.clone());
        self.updated_at_ms = now_ms;

        // 5. 检查是否到达终态
        if target_state.is_terminal {
            self.is_completed = true;
        }

        Ok(record)
    }

    /// 获取当前状态
    pub fn current_state<'a>(&self, fsm: &'a BusinessStateMachine) -> Option<&'a BusinessState> {
        fsm.find_state(&self.current_state_id)
    }

    /// 检查是否在终态
    pub fn is_at_terminal(&self, fsm: &BusinessStateMachine) -> bool {
        self.current_state(fsm).map(|s| s.is_terminal).unwrap_or(false)
    }

    /// 获取转移历史
    pub fn transition_history(&self) -> &[FsmTransitionRecord] {
        &self.transition_history
    }
}

/// 转移记录
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsmTransitionRecord {
    pub from: BusinessStateId,
    pub to: BusinessStateId,
    #[serde(alias = "timestamp_ms")]
    pub timestamp_ms: u64,
}

// ── 错误类型 ──

/// 状态机验证错误
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FsmValidationError {
    #[serde(rename = "invalid_initial_state")]
    InvalidInitialState(BusinessStateId),
    #[serde(rename = "invalid_transition_from")]
    InvalidTransitionFrom(BusinessStateId),
    #[serde(rename = "invalid_transition_to")]
    InvalidTransitionTo(BusinessStateId),
    #[serde(rename = "no_terminal_state")]
    NoTerminalState,
}

impl std::fmt::Display for FsmValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsmValidationError::InvalidInitialState(id) => {
                write!(f, "初始状态不存在: {id}")
            },
            FsmValidationError::InvalidTransitionFrom(id) => {
                write!(f, "转移源状态不存在: {id}")
            },
            FsmValidationError::InvalidTransitionTo(id) => {
                write!(f, "转移目标状态不存在: {id}")
            },
            FsmValidationError::NoTerminalState => {
                write!(f, "状态机必须至少有一个终态")
            },
        }
    }
}

/// 状态转移错误
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FsmTransitionError {
    #[serde(rename = "machine_completed")]
    MachineCompleted,
    #[serde(rename = "invalid_transition")]
    InvalidTransition { from: BusinessStateId, to: BusinessStateId },
    #[serde(rename = "state_not_found")]
    StateNotFound(BusinessStateId),
    #[serde(rename = "guard_failed")]
    GuardFailed { transition_id: Option<String>, reason: String },
    #[serde(rename = "requires_approval")]
    RequiresApproval { transition_id: Option<String> },
}

impl std::fmt::Display for FsmTransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsmTransitionError::MachineCompleted => {
                write!(f, "状态机已完成，无法继续转移")
            },
            FsmTransitionError::InvalidTransition { from, to } => {
                write!(f, "非法状态转移: {from} → {to}")
            },
            FsmTransitionError::StateNotFound(id) => {
                write!(f, "状态不存在: {id}")
            },
            FsmTransitionError::GuardFailed { reason, .. } => {
                write!(f, "守卫条件不满足: {reason}")
            },
            FsmTransitionError::RequiresApproval { .. } => {
                write!(f, "该转移需要审批")
            },
        }
    }
}

// ── 工具函数 ──

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── 预设业务状态机模板 ──

impl BusinessStateMachine {
    /// 审批流程模板：Submitted → UnderReview → Approved/Rejected
    pub fn approval_flow() -> Self {
        Self::new("approval_flow", "审批流程")
            .with_state(
                BusinessState::new("submitted").with_label("已提交").with_node_ref("node_submit"),
            )
            .with_state(
                BusinessState::new("under_review")
                    .with_label("审核中")
                    .with_node_ref("node_review"),
            )
            .with_state(
                BusinessState::new("approved")
                    .with_label("已批准")
                    .with_node_ref("node_approve")
                    .as_terminal(),
            )
            .with_state(
                BusinessState::new("rejected")
                    .with_label("已拒绝")
                    .with_node_ref("node_reject")
                    .as_terminal(),
            )
            .with_transition(StateTransition::new("submitted", "under_review"))
            .with_transition(StateTransition::new("under_review", "approved"))
            .with_transition(StateTransition::new("under_review", "rejected"))
    }

    /// 订单流程模板：Created → Paid → Shipped → Delivered
    pub fn order_flow() -> Self {
        Self::new("order_flow", "订单流程")
            .with_state(
                BusinessState::new("created")
                    .with_label("已创建")
                    .with_node_ref("node_create_order"),
            )
            .with_state(
                BusinessState::new("paid").with_label("已支付").with_node_ref("node_payment"),
            )
            .with_state(
                BusinessState::new("shipped").with_label("已发货").with_node_ref("node_ship"),
            )
            .with_state(
                BusinessState::new("delivered")
                    .with_label("已送达")
                    .with_node_ref("node_deliver")
                    .as_terminal(),
            )
            .with_state(
                BusinessState::new("cancelled")
                    .with_label("已取消")
                    .with_node_ref("node_cancel")
                    .as_terminal(),
            )
            .with_transition(StateTransition::new("created", "paid"))
            .with_transition(StateTransition::new("created", "cancelled"))
            .with_transition(StateTransition::new("paid", "shipped"))
            .with_transition(StateTransition::new("shipped", "delivered"))
    }

    /// 数据处理流程模板：Extract → Transform → Load → Complete
    pub fn data_pipeline() -> Self {
        Self::new("data_pipeline", "数据处理流程")
            .with_state(
                BusinessState::new("extract").with_label("数据抽取").with_node_ref("node_extract"),
            )
            .with_state(
                BusinessState::new("transform")
                    .with_label("数据转换")
                    .with_node_ref("node_transform"),
            )
            .with_state(
                BusinessState::new("load").with_label("数据加载").with_node_ref("node_load"),
            )
            .with_state(
                BusinessState::new("complete")
                    .with_label("完成")
                    .with_node_ref("node_complete")
                    .as_terminal(),
            )
            .with_state(
                BusinessState::new("error")
                    .with_label("错误处理")
                    .with_node_ref("node_error")
                    .as_terminal(),
            )
            .with_transition(StateTransition::new("extract", "transform"))
            .with_transition(StateTransition::new("transform", "load"))
            .with_transition(StateTransition::new("load", "complete"))
            .with_transition(StateTransition::new("extract", "error"))
            .with_transition(StateTransition::new("transform", "error"))
            .with_transition(StateTransition::new("load", "error"))
    }
}

// ── FSM 持久化接口 ──

/// FSM 持久化错误
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FsmPersistenceError {
    #[serde(rename = "not_found")]
    NotFound(String),
    #[serde(rename = "serialization_failed")]
    SerializationFailed(String),
    #[serde(rename = "storage_error")]
    StorageError(String),
}

impl std::fmt::Display for FsmPersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsmPersistenceError::NotFound(id) => write!(f, "FSM 实例未找到: {id}"),
            FsmPersistenceError::SerializationFailed(msg) => write!(f, "序列化失败: {msg}"),
            FsmPersistenceError::StorageError(msg) => write!(f, "存储错误: {msg}"),
        }
    }
}

/// 决策日志记录（用于时间旅行还原）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsmDecisionLog {
    /// 决策 ID
    pub id: String,
    /// 实例 ID
    #[serde(alias = "instance_id")]
    pub instance_id: String,
    /// 决策时间戳
    #[serde(alias = "timestamp_ms")]
    pub timestamp_ms: u64,
    /// 决策类型
    #[serde(alias = "decision_type")]
    pub decision_type: FsmDecisionType,
    /// 决策前状态
    #[serde(alias = "from_state")]
    pub from_state: String,
    /// 决策后状态
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "to_state")]
    pub to_state: Option<String>,
    /// 决策上下文（守卫条件评估等）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<FsmContext>,
    /// 决策结果描述
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 决策类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum FsmDecisionType {
    /// 状态转移决策
    Transition,
    /// 守卫条件评估决策
    GuardEvaluation,
    /// 审批决策
    Approval,
    /// 重置决策
    Reset,
    /// 创建决策
    Create,
}

/// FSM 持久化接口
///
/// 定义 FSM 实例状态的持久化操作，支持从数据库加载和保存。
pub trait FsmPersistence: Send + Sync {
    /// 保存 FSM 运行时状态
    fn save_state(
        &self,
        state: &FsmRuntimeState,
        decision_logs: &[FsmDecisionLog],
    ) -> Result<(), FsmPersistenceError>;

    /// 加载 FSM 运行时状态
    fn load_state(
        &self,
        instance_id: &str,
    ) -> Result<Option<(FsmRuntimeState, Vec<FsmDecisionLog>)>, FsmPersistenceError>;

    /// 列出所有 FSM 实例
    fn list_instances(&self) -> Result<Vec<FsmRuntimeState>, FsmPersistenceError>;

    /// 删除 FSM 实例
    fn delete_instance(&self, instance_id: &str) -> Result<(), FsmPersistenceError>;
}

/// 内存持久化实现（用于测试和单机场景）
#[derive(Default)]
pub struct MemoryFsmPersistence {
    states: std::sync::RwLock<Vec<(FsmRuntimeState, Vec<FsmDecisionLog>)>>,
}

impl MemoryFsmPersistence {
    pub fn new() -> Self {
        Self { states: std::sync::RwLock::new(Vec::new()) }
    }
}

impl FsmPersistence for MemoryFsmPersistence {
    fn save_state(
        &self,
        state: &FsmRuntimeState,
        decision_logs: &[FsmDecisionLog],
    ) -> Result<(), FsmPersistenceError> {
        let mut states = self
            .states
            .write()
            .map_err(|e| FsmPersistenceError::StorageError(format!("锁获取失败: {e}")))?;

        // 查找已有实例并更新
        if let Some(pos) = states.iter().position(|(s, _)| s.instance_id == state.instance_id) {
            states[pos] = (state.clone(), decision_logs.to_vec());
        } else {
            states.push((state.clone(), decision_logs.to_vec()));
        }

        Ok(())
    }

    fn load_state(
        &self,
        instance_id: &str,
    ) -> Result<Option<(FsmRuntimeState, Vec<FsmDecisionLog>)>, FsmPersistenceError> {
        let states = self
            .states
            .read()
            .map_err(|e| FsmPersistenceError::StorageError(format!("锁获取失败: {e}")))?;

        Ok(states.iter().find(|(s, _)| s.instance_id == instance_id).cloned())
    }

    fn list_instances(&self) -> Result<Vec<FsmRuntimeState>, FsmPersistenceError> {
        let states = self
            .states
            .read()
            .map_err(|e| FsmPersistenceError::StorageError(format!("锁获取失败: {e}")))?;

        Ok(states.iter().map(|(s, _)| s.clone()).collect())
    }

    fn delete_instance(&self, instance_id: &str) -> Result<(), FsmPersistenceError> {
        let mut states = self
            .states
            .write()
            .map_err(|e| FsmPersistenceError::StorageError(format!("锁获取失败: {e}")))?;

        states.retain(|(s, _)| s.instance_id != instance_id);
        Ok(())
    }
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_fsm() {
        let fsm = BusinessStateMachine::new("test", "测试状态机");
        assert_eq!(fsm.id, "test");
        assert_eq!(fsm.name, "测试状态机");
        assert!(fsm.states.is_empty());
    }

    #[test]
    fn test_add_state() {
        let fsm = BusinessStateMachine::new("test", "test")
            .with_state(BusinessState::new("state1").with_label("状态1"));

        assert_eq!(fsm.states.len(), 1);
        assert_eq!(fsm.initial_state_id, "state1");
    }

    #[test]
    fn test_add_transition() {
        let fsm = BusinessStateMachine::new("test", "test")
            .with_state(BusinessState::new("a"))
            .with_state(BusinessState::new("b"))
            .with_transition(StateTransition::new("a", "b"));

        assert_eq!(fsm.transitions.len(), 1);
        assert!(fsm.is_valid_transition("a", "b"));
        assert!(!fsm.is_valid_transition("b", "a"));
    }

    #[test]
    fn test_fsm_validation_success() {
        let fsm = BusinessStateMachine::approval_flow();
        assert!(fsm.validate().is_ok());
    }

    #[test]
    fn test_fsm_validation_no_terminal() {
        let fsm = BusinessStateMachine::new("test", "test").with_state(BusinessState::new("a"));

        let result = fsm.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsmValidationError::NoTerminalState));
    }

    #[test]
    fn test_fsm_validation_invalid_initial() {
        let fsm = BusinessStateMachine::new("test", "test")
            .with_state(BusinessState::new("a").as_terminal())
            .with_initial_state("nonexistent");

        let result = fsm.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsmValidationError::InvalidInitialState(_)));
    }

    #[test]
    fn test_runtime_state_transition() {
        let fsm = BusinessStateMachine::approval_flow();
        let mut runtime = FsmRuntimeState::new("inst-1", "approval_flow", "submitted");

        assert_eq!(runtime.current_state_id, "submitted");

        // 合法转移
        let result = runtime.try_transition("under_review", &fsm);
        assert!(result.is_ok());
        assert_eq!(runtime.current_state_id, "under_review");
        assert_eq!(runtime.transition_history.len(), 1);

        // 继续转移
        let result = runtime.try_transition("approved", &fsm);
        assert!(result.is_ok());
        assert!(runtime.is_completed);

        // 终态后无法转移
        let result = runtime.try_transition("submitted", &fsm);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsmTransitionError::MachineCompleted));
    }

    #[test]
    fn test_invalid_transition_blocked() {
        let fsm = BusinessStateMachine::approval_flow();
        let mut runtime = FsmRuntimeState::new("inst-1", "approval_flow", "submitted");

        // 非法转移
        let result = runtime.try_transition("approved", &fsm);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FsmTransitionError::InvalidTransition { from, to }
            if from == "submitted" && to == "approved"
        ));
    }

    #[test]
    fn test_find_state() {
        let fsm = BusinessStateMachine::approval_flow();

        let state = fsm.find_state("submitted").unwrap();
        assert_eq!(state.label, "已提交");
        assert!(!state.is_terminal);

        let state = fsm.find_state("approved").unwrap();
        assert!(state.is_terminal);

        assert!(fsm.find_state("nonexistent").is_none());
    }

    #[test]
    fn test_next_states() {
        let fsm = BusinessStateMachine::approval_flow();

        let next = fsm.next_states("submitted");
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].to, "under_review");

        let next = fsm.next_states("under_review");
        assert_eq!(next.len(), 2);
    }

    #[test]
    fn test_fsm_context() {
        let ctx = FsmContext::new()
            .with_event("submit")
            .with_user_role("manager")
            .with_variable("amount", serde_json::json!(100));

        assert_eq!(ctx.event, Some("submit".to_string()));
        assert!(ctx.has_role("manager"));
        assert!(!ctx.has_role("admin"));
        assert_eq!(ctx.get_variable("amount"), Some(&serde_json::json!(100)));
    }

    #[test]
    fn test_preset_machines() {
        // 验证所有预设状态机都能通过验证
        let machines = vec![
            ("审批流程", BusinessStateMachine::approval_flow()),
            ("订单流程", BusinessStateMachine::order_flow()),
            ("数据流程", BusinessStateMachine::data_pipeline()),
        ];

        for (name, fsm) in machines {
            assert!(fsm.validate().is_ok(), "{name} 状态机验证失败");
        }
    }

    #[test]
    fn test_state_context_methods() {
        let state = BusinessState::new("test")
            .with_label("测试状态")
            .with_node_ref("node-1")
            .with_allowed_tools(vec!["tool_a".to_string(), "tool_b".to_string()])
            .with_priority(10)
            .with_description("这是一个测试状态");

        assert_eq!(state.id, "test");
        assert_eq!(state.label, "测试状态");
        assert_eq!(state.node_ref, Some("node-1".to_string()));
        assert_eq!(state.allowed_tools, Some(vec!["tool_a".to_string(), "tool_b".to_string()]));
        assert_eq!(state.priority, 10);
        assert_eq!(state.description, Some("这是一个测试状态".to_string()));
    }
}
