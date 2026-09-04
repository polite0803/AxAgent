// SPDX-License-Identifier: AGPL-3.0-only

//! Harness Agent trait 适配器 — 包装 ReActEngine 实现 harness Agent。
//!
//! 接线说明：
//! - 2026-09-03：本模块曾因 lib.rs 缺 `mod harness_adapter;` 从未编译，
//!   其间 harness `AgentResult` / `PlanStep` 字段已收敛（见 `axagent-harness::agent`），
//!   此处按现行契约适配。
//! - 2026-09-04：注入可选 SessionManager，execute 时自动创建会话并返回真实
//!   session_id；同时为运行中取消预留 Arc<AtomicBool> token 检查钩子。

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use async_trait::async_trait;
use axagent_harness::agent::{
    Agent, AgentCapability, AgentExecuteRequest, AgentPlan, AgentResult, PlanStep,
};
use uuid::Uuid;

use crate::react_engine::ReActEngine;
use crate::session_manager::SessionManager;

pub struct HarnessAgentAdapter {
    name: String,
    caps: Vec<AgentCapability>,
    engine: tokio::sync::Mutex<ReActEngine>,
    /// 可选：如果注入了 SessionManager，execute 会创建真实会话并返回 session_id。
    session_manager: Option<Arc<SessionManager>>,
    /// 可选：全局取消信号。置 true 后所有正在 run 的 execute 应尽快退出。
    /// 供 AgentSessionBroker::cancel_session 调用时唤醒。
    cancellation_flag: Option<Arc<AtomicBool>>,
}

// ReActEngine 含 trait object 字段无法自动 derive Debug，
// 而 harness `Agent` trait 要求 `fmt::Debug` —— 手动实现。
impl std::fmt::Debug for HarnessAgentAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HarnessAgentAdapter")
            .field("name", &self.name)
            .field("caps", &self.caps)
            .field("has_session_manager", &self.session_manager.is_some())
            .field("has_cancellation", &self.cancellation_flag.is_some())
            .finish()
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
            engine: tokio::sync::Mutex::new(ReActEngine::new()),
            session_manager: None,
            cancellation_flag: None,
        }
    }

    /// 注入 SessionManager —— execute 时创建真实会话并返回 session_id。
    pub fn with_session_manager(mut self, sm: Arc<SessionManager>) -> Self {
        self.session_manager = Some(sm);
        self
    }

    /// 注入全局取消信号。置 true 后 ReActEngine::run 循环应尽快退出。
    pub fn with_cancellation_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.cancellation_flag = Some(flag);
        self
    }

    /// 便捷 builder：同时注入 session_manager + 一个共享取消 flag。
    pub fn with_runtime(self, sm: Arc<SessionManager>) -> Self {
        let flag = Arc::new(AtomicBool::new(false));
        self.with_session_manager(sm).with_cancellation_flag(flag)
    }

    pub fn from_engine(name: &str, engine: ReActEngine) -> Self {
        Self {
            name: name.to_string(),
            caps: vec![],
            engine: tokio::sync::Mutex::new(engine),
            session_manager: None,
            cancellation_flag: None,
        }
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

        // 1. 如果注入了 SessionManager，先创建会话
        let (provider_id, conversation_id) = self
            .session_manager
            .as_ref()
            .map(|_sm| {
                // MCP agent_run 每次调用独立创建 conversation_id
                let provider = "default".to_string();
                let conv = format!("mcp-{}", Uuid::new_v4());
                (provider, conv)
            })
            .unwrap_or_else(|| ("default".to_string(), "standalone".to_string()));

        let session_id = if let Some(sm) = &self.session_manager {
            match sm.create_session(provider_id.clone(), conversation_id.clone()).await {
                Ok(session) => Some(session.session().session_id.clone()),
                Err(e) => {
                    tracing::warn!(
                        "[HarnessAgentAdapter] create_session failed: {e}, falling back to no-session mode"
                    );
                    None
                },
            }
        } else {
            None
        };

        // 2. 获取本次执行对应的取消 token（优先 per-session，回退全局）
        let cancel_token: Option<Arc<AtomicBool>> =
            if let (Some(sm), Some(sid)) = (&self.session_manager, &session_id) {
                // per-session token：SessionManager.create_session 时注册
                sm.get_cancel_token(sid).await
            } else if let Some(ref flag) = self.cancellation_flag {
                // 回退全局 flag（无 SessionManager 场景）
                flag.store(false, std::sync::atomic::Ordering::SeqCst);
                Some(Arc::clone(flag))
            } else {
                None
            };

        // 3. 执行推理循环（带取消检查钩子）
        let result = {
            let mut engine = self.engine.lock().await;
            if let Some(token) = cancel_token {
                engine.set_cancel_flag(token);
            }
            engine.run(&req.goal).await
        };

        // 3. 返回结果
        Ok(AgentResult {
            output: result.final_response,
            success: result.success,
            steps_taken: result.iterations as u32,
            session_id,
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
