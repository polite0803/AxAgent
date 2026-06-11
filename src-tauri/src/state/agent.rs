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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
