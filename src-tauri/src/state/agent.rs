//! Agent runtime domain state.
//!
//! Owns the agent-execution bookkeeping: the running-agent set, the
//! per-agent cancel-token map, the agent session manager, the reflector,
//! and the platform manager / bridge (which fan messages out to external
//! channels such as Telegram, Slack, etc.).

use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{Mutex, RwLock as TokioRwLock};

// AgentState 通过 Arc<AgentState> 在 AppState 中使用，Rust 编译器认为 struct 本身和 impl
// 都是"死代码"因为它们是间接引用（不通过直接类型引用）。实际上 AppState 持有 Box/Arc 指针。
// 这些字段被 Tauri 命令通过 app_state.agent_state.field 路径访问。
pub struct AgentState {
    pub agent_session_manager: Arc<axagent_agent::SessionManager>,
    pub agent_cancel_tokens: Arc<DashMap<String, Arc<AtomicBool>>>,
    pub agent_paused: Arc<Mutex<std::collections::HashSet<String>>>,
    pub running_agents: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
    pub reflector: Arc<axagent_agent::Reflector>,
    pub platform_manager: Arc<axagent_runtime::message_gateway::platform_manager::PlatformManager>,
    pub platform_bridge: Arc<axagent_runtime::message_gateway::platform_bridge::PlatformBridge>,
    pub local_tool_registry: Arc<tokio::sync::Mutex<axagent_tools::registry::UnifiedToolRegistry>>,
    pub work_engine: Arc<axagent_runtime::work_engine::WorkEngine>,
}

// #[allow(dead_code)] 原因同上：impl 块虽然内部方法都被调用，但 struct 本身通过
// Arc<AgentState> 间接引用，编译器标记整个 impl 为 dead code。
impl AgentState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        agent_session_manager: Arc<axagent_agent::SessionManager>,
        agent_cancel_tokens: Arc<DashMap<String, Arc<AtomicBool>>>,
        agent_paused: Arc<Mutex<std::collections::HashSet<String>>>,
        running_agents: Arc<TokioRwLock<std::collections::HashSet<String>>>,
        reflector: Arc<axagent_agent::Reflector>,
        platform_manager: Arc<axagent_runtime::message_gateway::platform_manager::PlatformManager>,
        platform_bridge: Arc<axagent_runtime::message_gateway::platform_bridge::PlatformBridge>,
        local_tool_registry: Arc<tokio::sync::Mutex<axagent_tools::registry::UnifiedToolRegistry>>,
        work_engine: Arc<axagent_runtime::work_engine::WorkEngine>,
    ) -> Self {
        Self {
            agent_session_manager,
            agent_cancel_tokens,
            agent_paused,
            running_agents,
            reflector,
            platform_manager,
            platform_bridge,
            local_tool_registry,
            work_engine,
        }
    }
}
