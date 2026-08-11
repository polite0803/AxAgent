// SPDX-License-Identifier: AGPL-3.0-only

//! Agent 单轮 ReAct 执行器契约(2.5 P1)。
//!
//! ## 背景
//!
//! 工作流的 Agent 节点(`rt-workflow::AgentExecutor`)原本在节点内 inline 实现
//! ReAct 循环(模型推理 + 工具调用 + 结果回填)。自由对话的
//! `runtime-core::conversation.rs` + `agent::react_engine.rs` + 工作流的
//! `agent_executor.rs` 共三套 ReAct,长期目标合并为一套。
//!
//! ## 架构约束
//!
//! `rt-workflow` 是 hybrid crate,不能依赖 `agent` crate(consumer)。
//! 因此不能直接调用 `SessionManager`。本 trait 定义在 harness(foundation),
//! 由 wiring 层(runtime/init)把 `SessionManager` 实例注入到 `WorkEngine`,
//! `AgentExecutor` 通过 trait 对象调用,实现"委托"语义。
//!
//! ## 容错策略
//!
//! `AgentExecutor` 保留原 inline ReAct 作为 fallback:
//! - 注入了 `AgentTurnRunner` → 走 trait(支持 trajectory / 权限询问 / 压缩)
//! - 未注入 → 走 inline(向后兼容,不破坏旧测试)
//!
//! ## 关联类型
//!
//! - 输入:`AgentTurnRequest` — 系统提示 + 用户输入 + 工具列表 + 模型配置
//! - 输出:`AgentTurnResult` — 最终内容 + 工具调用记录 + token 用量

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core_error::Result;
use crate::tool::ToolPermissions;
use crate::types::{ChatMessage, ChatTool, TokenUsage};
use crate::workflow_types::ToolDef;

/// Agent 单轮 ReAct 执行请求(2.5 P1)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnRequest {
    /// 执行 ID(对应工作流 execution_id),用于 trajectory 关联。
    pub execution_id: String,
    /// 节点 ID(对应 AgentNode.base.id),用于 trajectory 定位。
    pub node_id: String,
    /// 角色 ID(对应 AgentRole),用于 trajectory 分类。
    pub role_id: Option<String>,
    /// 完整的 system prompt(已模板渲染 + context_contributors 注入)。
    pub system_prompt: String,
    /// 用户/触发器输入(自然语言任务描述)。
    pub user_input: String,
    /// 历史消息(多轮对话场景;首轮为空 Vec)。
    pub history: Vec<ChatMessage>,
    /// 暴露给 LLM 的工具定义。空 Vec 表示不启用工具调用。
    pub tools: Vec<ChatTool>,
    /// 工具权限约束(None = 不施加额外约束)。
    pub tool_permissions: Option<ToolPermissions>,
    /// 模型 ID(如 "anthropic/claude-3-5-sonnet")。
    pub model: String,
    /// Provider ID(可选,用于多 provider 路由)。
    pub provider_id: Option<String>,
    /// 温度(可选,None = provider 默认)。
    pub temperature: Option<f32>,
    /// 最大 token 数(可选,None = provider 默认)。
    pub max_tokens: Option<u32>,
    /// 最大工具调用轮数(None = 由实现方决定默认值,通常 5)。
    pub max_tool_rounds: Option<u32>,
    /// 工作空间目录(用于文件类工具的相对路径解析)。
    pub workspace_dir: Option<String>,
}

/// Agent 单轮 ReAct 执行结果(2.5 P1)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnResult {
    /// 最终输出内容(LLM 最后一轮的文本)。
    pub content: String,
    /// 推理过程(thinking content,Anthropic / DeepSeek-R1 等)。
    pub thinking: Option<String>,
    /// 工具调用记录(按时间顺序)。
    pub tool_calls: Vec<AgentToolCallRecord>,
    /// token 用量统计。
    pub usage: TokenUsage,
    /// 实际执行的轮数(1 = 纯推理无工具调用)。
    pub iterations: u32,
    /// 是否因达到 max_tool_rounds 而停止。
    pub stopped_by_limit: bool,
}

/// Agent 工具调用记录(trajectory 入口)。
///
/// 注意:与 `tool_metrics::ToolCallRecord` 不同 — 后者用于指标统计(轻量),
/// 本类型用于 trajectory 完整记录(含 input/output)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCallRecord {
    /// 工具调用 ID(LLM 生成)。
    pub call_id: String,
    /// 工具名称。
    pub tool_name: String,
    /// 工具输入参数(JSON 字符串)。
    pub input: String,
    /// 工具执行结果(JSON 字符串)。
    pub output: String,
    /// 是否执行出错。
    pub is_error: bool,
    /// 耗时(毫秒)。
    pub elapsed_ms: u64,
}

/// Agent 单轮 ReAct 执行器 trait(2.5 P1)。
///
/// 由 `agent::SessionManager`(或 wiring 层适配器)实现,
/// 通过 `WorkEngine::set_agent_turn_runner` 注入到 `rt-workflow`。
///
/// ## 实现要点
///
/// - **异步**:所有方法都是 `async fn`,实现方用 `#[async_trait]`
/// - **零阻塞**:长时间运行的工具调用应在内部 spawn,不要长时间持锁
/// - **错误隔离**:实现方内部错误应转为 `crate::core_error::Error`,不传播 panic
/// - **trajectory 记录**:实现方应在每轮工具调用后写 trajectory(通过 trajectory_service)
/// - **权限询问**:遇到 `tool_permissions.ask` 类工具时,通过 `PermissionPrompter` 询问用户
/// - **压缩**:实现方按 `auto_compaction_threshold` 自动压缩上下文
#[async_trait]
pub trait AgentTurnRunner: Send + Sync {
    /// 执行一轮 Agent ReAct 循环。
    ///
    /// 输入:`AgentTurnRequest`(含 system_prompt / user_input / tools / model 等)
    /// 输出:`AgentTurnResult`(含最终内容 / 工具调用记录 / usage 等)
    async fn run_turn(&self, request: AgentTurnRequest) -> Result<AgentTurnResult>;

    /// 探测实现方是否可用(用于 fallback 决策)。
    ///
    /// 默认返回 `true`。实现方可以在未配置 API key / 模型不可用时返回 `false`,
    /// 让 `AgentExecutor` 回退到 inline 模式。
    fn is_available(&self) -> bool {
        true
    }
}

/// 把 `Vec<ToolDef>` 转换为 `Vec<ChatTool>`(便捷工具函数)。
///
/// `ToolDef` 是工作流模板里的工具定义(只有 name/description/parameters),
/// `ChatTool` 是 LLM 请求里的工具定义(完整 OpenAI tool schema)。
/// 转换在 harness 层完成,避免每个 AgentTurnRunner 实现都重复一遍。
pub fn tool_defs_to_chat_tools(defs: &[ToolDef]) -> Vec<ChatTool> {
    defs.iter()
        .map(|d| ChatTool {
            r#type: "function".to_string(),
            function: crate::types::ChatToolFunction {
                name: d.name.clone(),
                description: d.description.clone(),
                parameters: d.parameters.as_ref().and_then(|p| serde_json::to_value(p).ok()),
            },
        })
        .collect()
}

/// 把 `Vec<ChatMessage>` 序列化为 trajectory 用的 JSON 字符串(便捷工具函数)。
pub fn serialize_history(history: &[ChatMessage]) -> String {
    serde_json::to_string(history).unwrap_or_else(|_| "[]".to_string())
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_turn_request_serialization_roundtrip() {
        let req = AgentTurnRequest {
            execution_id: "exec-001".to_string(),
            node_id: "agent-1".to_string(),
            role_id: Some("developer".to_string()),
            system_prompt: "You are a developer".to_string(),
            user_input: "Fix the bug in auth module".to_string(),
            history: vec![],
            tools: vec![],
            tool_permissions: None,
            model: "claude-3-5-sonnet".to_string(),
            provider_id: None,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            max_tool_rounds: Some(5),
            workspace_dir: Some("/tmp/work".to_string()),
        };
        let json = serde_json::to_string(&req).expect("测试：JSON序列化应成功");
        let parsed: AgentTurnRequest =
            serde_json::from_str(&json).expect("测试：JSON反序列化应成功");
        assert_eq!(parsed.execution_id, "exec-001");
        assert_eq!(parsed.node_id, "agent-1");
        assert_eq!(parsed.model, "claude-3-5-sonnet");
        assert_eq!(parsed.max_tool_rounds, Some(5));
    }

    #[test]
    fn agent_turn_result_serialization() {
        let result = AgentTurnResult {
            content: "Done".to_string(),
            thinking: Some("Hmm...".to_string()),
            tool_calls: vec![AgentToolCallRecord {
                call_id: "call-1".to_string(),
                tool_name: "read_file".to_string(),
                input: r#"{"path":"a.rs"}"#.to_string(),
                output: r#"{"content":"..."}"#.to_string(),
                is_error: false,
                elapsed_ms: 42,
            }],
            usage: TokenUsage::default(),
            iterations: 2,
            stopped_by_limit: false,
        };
        let json = serde_json::to_string(&result).expect("测试：JSON序列化应成功");
        let parsed: AgentTurnResult =
            serde_json::from_str(&json).expect("测试：JSON反序列化应成功");
        assert_eq!(parsed.content, "Done");
        assert_eq!(parsed.tool_calls.len(), 1);
        assert_eq!(parsed.tool_calls[0].tool_name, "read_file");
    }

    #[test]
    fn tool_defs_to_chat_tools_converts_correctly() {
        let defs = vec![ToolDef {
            name: "search".to_string(),
            description: Some("Search the web".to_string()),
            parameters: None,
        }];
        let tools = tool_defs_to_chat_tools(&defs);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name, "search");
        assert_eq!(tools[0].function.description.as_deref(), Some("Search the web"));
    }

    #[test]
    fn tool_defs_to_chat_tools_empty_input() {
        let tools = tool_defs_to_chat_tools(&[]);
        assert!(tools.is_empty());
    }

    #[test]
    fn serialize_history_handles_empty() {
        let s = serialize_history(&[]);
        assert_eq!(s, "[]");
    }

    /// Stub 实现用于验证 trait 可被 Box<dyn AgentTurnRunner> 调用
    struct StubRunner;

    #[async_trait]
    impl AgentTurnRunner for StubRunner {
        async fn run_turn(&self, _request: AgentTurnRequest) -> Result<AgentTurnResult> {
            Ok(AgentTurnResult {
                content: "stub".to_string(),
                thinking: None,
                tool_calls: vec![],
                usage: TokenUsage::default(),
                iterations: 1,
                stopped_by_limit: false,
            })
        }
    }

    #[tokio::test]
    async fn stub_runner_returns_stub_content() {
        let runner: Box<dyn AgentTurnRunner> = Box::new(StubRunner);
        let req = AgentTurnRequest {
            execution_id: "e".to_string(),
            node_id: "n".to_string(),
            role_id: None,
            system_prompt: String::new(),
            user_input: "hi".to_string(),
            history: vec![],
            tools: vec![],
            tool_permissions: None,
            model: "m".to_string(),
            provider_id: None,
            temperature: None,
            max_tokens: None,
            max_tool_rounds: None,
            workspace_dir: None,
        };
        let result = runner.run_turn(req).await.expect("测试：异步操作应成功");
        assert_eq!(result.content, "stub");
        assert_eq!(result.iterations, 1);
    }

    #[test]
    fn is_available_defaults_to_true() {
        let runner: Box<dyn AgentTurnRunner> = Box::new(StubRunner);
        assert!(runner.is_available());
    }
}
