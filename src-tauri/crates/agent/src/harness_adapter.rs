// SPDX-License-Identifier: AGPL-3.0-only

//! Harness Agent trait 适配器 — 包装 ReActEngine 实现 harness Agent。

use async_trait::async_trait;
use axagent_harness::agent::{Agent, AgentCapability, AgentExecuteRequest, AgentPlan, AgentResult, PlanStep};
use std::time::Instant;

pub struct HarnessAgentAdapter {
    name: String,
    caps: Vec<AgentCapability>,
    engine: tokio::sync::Mutex<crate::react_engine::ReActEngine>,
}

impl HarnessAgentAdapter {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            caps: vec![
                AgentCapability { name: "reasoning".into(), description: "ReAct 推理循环".into() },
                AgentCapability { name: "tool_use".into(), description: "使用注册工具执行操作".into() },
            ],
            engine: tokio::sync::Mutex::new(crate::react_engine::ReActEngine::new()),
        }
    }

    pub fn from_engine(name: &str, engine: crate::react_engine::ReActEngine) -> Self {
        Self { name: name.to_string(), caps: vec![], engine: tokio::sync::Mutex::new(engine) }
    }
}

#[async_trait]
impl Agent for HarnessAgentAdapter {
    fn name(&self) -> &str { &self.name }
    fn capabilities(&self) -> Vec<AgentCapability> { self.caps.clone() }

    async fn execute(&self, req: AgentExecuteRequest) -> Result<AgentResult, String> {
        let start = Instant::now();
        let mut engine = self.engine.lock().await;
        let result = engine.run(&req.goal).await;
        Ok(AgentResult {
            success: result.success,
            summary: result.final_response,
            output: Some(serde_json::json!({"iterations": result.iterations})),
            duration_secs: start.elapsed().as_secs_f64(),
        })
    }

    async fn plan(&self, goal: &str) -> Result<AgentPlan, String> {
        Ok(AgentPlan { steps: vec![
            PlanStep { order: 1, description: format!("分析目标：{goal}"), tool: None, estimated_secs: Some(5.0) },
            PlanStep { order: 2, description: "执行推理循环".into(), tool: None, estimated_secs: Some(60.0) },
            PlanStep { order: 3, description: "生成最终结果".into(), tool: None, estimated_secs: Some(5.0) },
        ]})
    }
}
