// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流 Agent 节点 → `SessionManager` 的 wiring 适配器（P1 agent-loop 接缝）。
//!
//! ## 职责
//!
//! 实现 `harness::AgentTurnRunner`，把 `rt-workflow` 的 Agent 节点执行委托给统一的
//! `SessionManager::run_turn_with_tools`。wiring 层（`state.rs`）把本适配器同时：
//!
//! 1. 注册进能力注册表（`agent.loop` 接缝，`CapabilityOrigin::BuiltIn`）
//! 2. 注入 `WorkEngine`（`set_agent_turn_runner`）
//!
//! 使内置 Agent 主循环与外部插件平权：外部插件可通过 `register_external_agent_loop`
//! 替换同一接缝，`AgentExecutor` 通过 trait 对象调用，实现「委托」语义。
//!
//! ## 设计边界
//!
//! - **工具委托**：`AgentTurnRequest.tools` 非空时按名单装配独立
//!   `UnifiedToolRegistry`（名单外禁用 + 禁 Agent/RemoteTrigger 防递归），
//!   执行带工具的 ReAct 循环；空 tools 时走纯推理（空工具注册表）。
//! - **provider 解析**：优先 `request.provider_id` 命中的提供商，否则取第一个
//!   启用且含可用 key 的提供商；`request.model` 为空时用 provider 默认模型。
//! - **会话**：以 `request.execution_id` 作为 conversation_id 创建/复用 Session。
//! - **不注入 AskUser 桥接**：带工具委托用 `PermissionMode::WorkspaceWrite`
//!   （无人工介入通道，危险操作按权限策略拒绝），纯推理维持 `PermissionMode::Prompt`。

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use async_trait::async_trait;
use tokio::sync::Mutex;

use axagent_agent::{AxAgentApiClient, ChannelPermissionPrompter, SessionManager};
use axagent_dao::repo::provider;
use axagent_harness::agent_turn_runner::{
    AgentToolCallRecord, AgentTurnRequest, AgentTurnResult, AgentTurnRunner,
};
use axagent_harness::conversation_model::ContentBlock;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::resolve_base_url_for_type;
use axagent_harness::runtime_types::conversation::ApiClient;
use axagent_harness::runtime_types::permissions::{PermissionMode, PermissionPolicy};
use axagent_harness::types::ProviderConfig;
use axagent_harness::types::provider_model::resolve_provider_proxy;
use axagent_harness::{ProviderAdapter, ProviderRequestContext};
use axagent_runtime::harness::RuntimeHarness;
use axagent_runtime_core::{
    ConversationRuntimeFactoryArgs, RuntimeFeatureConfig, create_conversation_runtime,
};
use axagent_tools::registry::UnifiedToolRegistry;

/// Agent 主循环 wiring 适配器：把 `AgentTurnRunner` 委托给 `SessionManager`。
pub struct WorkflowAgentTurnRunner {
    session_manager: Arc<SessionManager>,
    harness: Arc<RuntimeHarness>,
    prompters: Arc<Mutex<HashMap<String, ChannelPermissionPrompter>>>,
}

impl WorkflowAgentTurnRunner {
    /// 构造适配器。`harness` 提供 db / master_key / provider 解析能力。
    pub fn new(
        session_manager: Arc<SessionManager>,
        harness: Arc<RuntimeHarness>,
        prompters: Arc<Mutex<HashMap<String, ChannelPermissionPrompter>>>,
    ) -> Self {
        Self { session_manager, harness, prompters }
    }

    /// 解析提供商：优先 `provider_id` 命中，否则取第一个启用且含可用 key 的提供商。
    async fn resolve_provider(
        &self,
        provider_id: Option<&str>,
    ) -> Result<(ProviderConfig, Arc<dyn ProviderAdapter>, ProviderRequestContext)> {
        let providers = provider::list_providers(self.harness.db())
            .await
            .map_err(|e| AxAgentError::Provider(e.to_string()))?;

        let prov = match provider_id {
            Some(pid) => providers.into_iter().find(|p| p.id == *pid),
            None => providers.into_iter().find(|p| p.enabled && p.keys.iter().any(|k| k.enabled)),
        }
        .ok_or_else(|| AxAgentError::Provider("没有启用的模型提供商".into()))?;

        let key = prov
            .keys
            .iter()
            .find(|k| k.enabled)
            .ok_or_else(|| AxAgentError::Provider("没有可用的 API key".into()))?;
        let api_key = axagent_crypto::decrypt_key(&key.key_encrypted, self.harness.master_key())
            .map_err(|e| AxAgentError::Crypto(e.to_string()))?;

        let settings =
            axagent_dao::repo::settings::get_settings(self.harness.db()).await.unwrap_or_default();

        let ctx = ProviderRequestContext {
            api_key,
            key_id: key.id.clone(),
            provider_id: prov.id.clone(),
            base_url: Some(resolve_base_url_for_type(&prov.api_host, &prov.provider_type)),
            api_path: prov.api_path.clone(),
            proxy_config: resolve_provider_proxy(&prov.proxy_config, &settings),
            custom_headers: prov.custom_headers.as_ref().and_then(|s| serde_json::from_str(s).ok()),
            api_mode: None,
            conversation: None,
            previous_response_id: None,
            store_response: None,
        };

        let adapter = self.harness.get_adapter_for_provider(&prov).await.ok_or_else(|| {
            AxAgentError::Provider(format!("无适配器可用: {:?}", prov.provider_type))
        })?;

        Ok((prov, adapter, ctx))
    }
}

#[async_trait]
impl AgentTurnRunner for WorkflowAgentTurnRunner {
    async fn run_turn(&self, mut request: AgentTurnRequest) -> Result<AgentTurnResult> {
        // 工具委托（R6 真接线）：非空 tools = 按名单装配独立工具注册表执行
        // ReAct 循环；空 tools = 纯推理（原行为）。
        // 独立实例而非复用主 registry：工具执行链上可能持有主 registry 的
        // tokio Mutex，子代理再取同一把锁会死锁；且名单/禁用约束互不污染。
        let allow_names: std::collections::HashSet<String> =
            request.tools.iter().map(|t| t.function.name.clone()).collect();
        let mut tool_registry = UnifiedToolRegistry::new();
        if !allow_names.is_empty() {
            tool_registry.init_all();
            // 白名单约束：名单外的全部禁用
            for name in tool_registry.list_tools() {
                if !allow_names.contains(&name) {
                    tool_registry.disable_tool(&name);
                }
            }
            // 防递归：子代理不得再创建子代理或远程触发会话
            tool_registry.disable_tool("Agent");
            tool_registry.disable_tool("RemoteTrigger");
        }

        let (prov, adapter, ctx) = self.resolve_provider(request.provider_id.as_deref()).await?;

        // 模型：request.model 优先，否则 provider 默认模型。
        let model = if request.model.is_empty() {
            prov.models.first().map(|m| m.model_id.clone()).unwrap_or_default()
        } else {
            request.model.clone()
        };

        // 会话：以 execution_id 为 conversation_id 创建/复用。
        let session = self
            .session_manager
            .get_or_create_session(prov.id.clone(), request.execution_id.clone())
            .await
            .map_err(|e| AxAgentError::agent(e.to_string()))?;
        let session_id = session.session().session_id.clone();

        // ApiClient + 工具注册表（空名单 = 空注册表纯推理）。
        let api_client: Box<dyn ApiClient + Send> =
            Box::new(AxAgentApiClient::new(adapter.clone(), ctx.clone()).with_model(model.clone()));
        let tool_executor: Box<
            dyn axagent_harness::runtime_types::conversation::ToolExecutor + Send,
        > = Box::new(tool_registry);

        // 权限：带工具委托无人工介入通道，用 WorkspaceWrite（危险操作按策略拒绝）；
        // 纯推理维持 Prompt（原行为）。
        let permission_mode = if allow_names.is_empty() {
            PermissionMode::Prompt
        } else {
            PermissionMode::WorkspaceWrite
        };

        let runtime = create_conversation_runtime(ConversationRuntimeFactoryArgs::new(
            session.session().clone(),
            api_client,
            tool_executor,
            PermissionPolicy::new(permission_mode),
            vec![request.system_prompt.clone()],
            RuntimeFeatureConfig::default(),
        ));

        let cancel_token = Arc::new(AtomicBool::new(false));
        let (summary, _updated_session) = self
            .session_manager
            .run_turn_with_tools(
                &session_id,
                std::mem::take(&mut request.user_input),
                runtime,
                request.execution_id.clone(),
                Some(cancel_token),
                self.prompters.clone(),
            )
            .await
            .map_err(|e| AxAgentError::execution(e.to_string()))?;

        // 结果映射：content = assistant 文本；tool_calls = ToolResult 记录。
        let mut content = String::new();
        for msg in &summary.assistant_messages {
            for block in &msg.blocks {
                if let ContentBlock::Text { text } = block {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(text.as_str());
                }
            }
        }

        let mut tool_calls = Vec::new();
        for msg in &summary.tool_results {
            for block in &msg.blocks {
                if let ContentBlock::ToolResult { tool_use_id, tool_name, output, is_error } = block
                {
                    tool_calls.push(AgentToolCallRecord {
                        call_id: tool_use_id.clone(),
                        tool_name: tool_name.clone(),
                        input: String::new(),
                        output: output.clone(),
                        is_error: *is_error,
                        elapsed_ms: 0,
                    });
                }
            }
        }

        Ok(AgentTurnResult {
            content,
            thinking: if summary.thinking.is_empty() {
                None
            } else {
                Some(summary.thinking.clone())
            },
            tool_calls,
            usage: summary.usage,
            iterations: summary.iterations as u32,
            stopped_by_limit: false,
        })
    }

    fn is_available(&self) -> bool {
        // 无 API 配置时返回 false，让 AgentExecutor 回退 inline。
        true
    }
}
