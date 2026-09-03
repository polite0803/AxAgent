// SPDX-License-Identifier: AGPL-3.0-only

//! Harness Agent trait 适配器 — 包装 ReActEngine 实现 harness Agent。
//!
//! 接线说明（2026-09-03）：本模块曾因 lib.rs 缺 `mod harness_adapter;` 从未编译，
//! 其间 harness `AgentResult` / `PlanStep` 字段已收敛（见 `axagent-harness::agent`），
//! 此处按现行契约适配。

use std::time::Instant;

use async_trait::async_trait;
use axagent_harness::agent::{
    Agent, AgentCapability, AgentExecuteRequest, AgentPlan, AgentResult, PlanStep,
};

pub struct HarnessAgentAdapter {
    name: String,
    caps: Vec<AgentCapability>,
    engine: tokio::sync::Mutex<crate::react_engine::ReActEngine>,
}

// ReActEngine 含 trait object 字段无法自动 derive Debug，
// 而 harness `Agent` trait 要求 `fmt::Debug` —— 手动实现（跳过 engine 内部）。
impl std::fmt::Debug for HarnessAgentAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HarnessAgentAdapter")
            .field("name", &self.name)
            .field("caps", &self.caps)
            .finish_non_exhaustive()
    }
}

impl HarnessAgentAdapter {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            caps: vec![
                AgentCapability {
                    name: "reasoning".into(), description: "ReAct 推理循环".into()
                },
                AgentCapability {
                    name: "tool_use".into(),
                    description: "使用注册工具执行操作".into(),
                },
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
    fn name(&self) -> &str {
        &self.name
    }
    fn capabilities(&self) -> Vec<AgentCapability> {
        self.caps.clone()
    }

    async fn execute(&self, req: AgentExecuteRequest) -> Result<AgentResult, String> {
        let _start = Instant::now();
        let mut engine = self.engine.lock().await;
        let result = engine.run(&req.goal).await;
        Ok(AgentResult {
            output: result.final_response,
            success: result.success,
            steps_taken: result.iterations as u32,
        })
    }

    async fn plan(&self, goal: &str) -> Result<AgentPlan, String> {
        Ok(AgentPlan {
            steps: vec![
                PlanStep { description: format!("分析目标：{goal}"), agent: None },
                PlanStep { description: "执行推理循环".into(), agent: None },
                PlanStep { description: "生成最终结果".into(), agent: None },
            ],
        })
    }
}
