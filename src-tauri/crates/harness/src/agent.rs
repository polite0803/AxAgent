// SPDX-License-Identifier: AGPL-3.0-only

//! Agent 契约（统一 agent 接口）

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapability {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentExecuteRequest {
    pub goal: String,
    pub context: Option<String>,
    #[serde(alias = "max_steps")]
    pub max_steps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentResult {
    pub output: String,
    pub success: bool,
    #[serde(alias = "steps_taken")]
    pub steps_taken: u32,
    /// 执行过程中创建的会话 ID（如果有持久化会话）。
    /// 用于 MCP `agent_run` 返回后，调用方可通过 `agent_status` / `agent_cancel` 跟踪。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentPlan {
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub description: String,
    pub agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<AgentCapability>,
}

#[async_trait]
pub trait Agent: Send + Sync + fmt::Debug {
    fn name(&self) -> &str;
    fn capabilities(&self) -> Vec<AgentCapability>;
    async fn execute(&self, req: AgentExecuteRequest) -> Result<AgentResult, String>;
    async fn plan(&self, goal: &str) -> Result<AgentPlan, String>;
}

use std::collections::HashMap;

pub struct AgentRegistry {
    agents: HashMap<String, Box<dyn Agent>>,
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self { agents: HashMap::new() }
    }
    pub fn register(&mut self, name: &str, agent: Box<dyn Agent>) {
        self.agents.insert(name.to_string(), agent);
    }
    pub fn get(&self, name: &str) -> Option<&dyn Agent> {
        self.agents.get(name).map(|b| b.as_ref())
    }
    pub fn list(&self) -> Vec<AgentInfo> {
        self.agents
            .iter()
            .map(|(name, agent)| AgentInfo {
                name: name.clone(),
                description: String::new(),
                capabilities: agent.capabilities(),
            })
            .collect()
    }
}
