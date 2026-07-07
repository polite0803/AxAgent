// SPDX-License-Identifier: AGPL-3.0-only

//! Agent 契约（统一 agent 接口）

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecuteRequest {
    pub goal: String,
    pub context: Option<String>,
    pub max_steps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub output: String,
    pub success: bool,
    pub steps_taken: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlan {
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub description: String,
    pub agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug)]
pub struct NoopAgent;

#[async_trait]
impl Agent for NoopAgent {
    fn name(&self) -> &str {
        "noop"
    }
    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![]
    }
    async fn execute(&self, _req: AgentExecuteRequest) -> Result<AgentResult, String> {
        Err("NoopAgent cannot execute".to_string())
    }
    async fn plan(&self, _goal: &str) -> Result<AgentPlan, String> {
        Err("NoopAgent cannot plan".to_string())
    }
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
