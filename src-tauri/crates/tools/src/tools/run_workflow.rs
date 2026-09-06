// SPDX-License-Identifier: AGPL-3.0-only
//! RunWorkflow — agent 侧工作流执行工具（认知编排执行链的执行入口，T3）。
//!
//! # 解决什么
//!
//! 认知编排路由命中 Workflow 能力后，agent 路径此前没有任何执行入口：
//! 护照 tool_ref 全空、编排模式白名单只放行 7 个元工具、全仓不存在
//! RunWorkflow/ExecuteCapability 类工具 —— agent 只能反复 DiscoverSkills
//! 检索后声明"无法获取实时数据"。本工具补上执行闭环的最后一环：
//! `CapabilityLoad(kind=Workflow)` 激活本工具 → LLM 发起 function call →
//! 经注入的执行器闭包驱动 WorkEngine。
//!
//! # 分层合规
//!
//! tools crate 不能反向依赖主 crate（执行入口 `workflow_execute` 在主 crate），
//! 故按项目既有惯例经 `OnceLock` + setter 注入执行器闭包
//! （参考 `capability_shared::set_capability_indexer` 的接线方式）。
//! 未注入时返回 `WORKFLOW_EXECUTOR_NOT_SET` 显式失败，不静默降级。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolErrorKind, ToolResult};
use async_trait::async_trait;
use axagent_harness::constants::capability_chain::RUN_WORKFLOW_TOOL;
use axagent_harness::error_codes::workflow::{
    EXECUTE_FAILED as WORKFLOW_EXECUTE_FAILED, EXECUTOR_NOT_SET as WORKFLOW_EXECUTOR_NOT_SET,
};
use serde_json::{Value, json};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

/// 工作流执行器闭包：`(workflow_id, input, conversation_id) -> Result<Value, String>`。
///
/// 由主 crate 在 wiring 层注入，内部与 `workflow_execute` 命令等价
/// （校验模板存在 → 组装 RunOptions → 驱动 WorkEngine）。
pub type WorkflowExecutorFn = Arc<
    dyn Fn(
            String,
            Option<Value>,
            Option<String>,
        ) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>
        + Send
        + Sync,
>;

static WORKFLOW_EXECUTOR: OnceLock<WorkflowExecutorFn> = OnceLock::new();

/// 注入工作流执行器（wiring 层初始化时调用一次）。
///
/// 未注入时 `RunWorkflow` 直接返回 `WORKFLOW_EXECUTOR_NOT_SET` 错误，
/// 而不是返回"假执行"格式化字符串（batch_missing::WorkflowTool 的历史教训）。
pub fn set_workflow_executor(executor: WorkflowExecutorFn) {
    let _ = WORKFLOW_EXECUTOR.set(executor);
}

/// 执行器未注入的规范错误（独立成纯函数便于单测锁定错误码）。
fn executor_not_set_error() -> ToolError {
    ToolError {
        message: "工作流执行器未注入，RunWorkflow 不可用（wiring 缺失）".to_string(),
        kind: ToolErrorKind::ExecutionFailed,
        error_code: WORKFLOW_EXECUTOR_NOT_SET.to_string(),
    }
}

/// 执行失败的规范错误。
fn execute_failed_error(message: String) -> ToolError {
    ToolError {
        message,
        kind: ToolErrorKind::ExecutionFailed,
        error_code: WORKFLOW_EXECUTE_FAILED.to_string(),
    }
}

pub struct RunWorkflowTool;

#[async_trait]
impl Tool for RunWorkflowTool {
    fn name(&self) -> &str {
        RUN_WORKFLOW_TOOL
    }

    fn description(&self) -> &str {
        "执行指定的工作流（认知编排命中 Workflow 能力后的执行入口）。\
         workflow_id 取自已加载能力的 capability_id（支持 'workflow:xxx' 或裸 'xxx'）；\
         input 可为 JSON 对象（键值对注入模板变量，如 {\"stock_code\": \"301302\"}）\
         或文本（自动参数提取），也可省略由工作流内部解析。\
         执行完成前本调用会阻塞，结果即工作流最终输出。"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "workflow_id": {
                    "type": "string",
                    "description": "要执行的工作流/能力 ID（来自 capability-index 或已加载能力）"
                },
                "input": {
                    "description": "执行输入：JSON 对象（键值对）或纯文本（自动参数提取），可省略"
                }
            },
            "required": ["workflow_id"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let raw_id = input["workflow_id"].as_str().unwrap_or("").trim().to_string();
        if raw_id.is_empty() {
            return Err(ToolError::invalid_input_for(RUN_WORKFLOW_TOOL, "workflow_id 为必填参数"));
        }
        // 容忍 'workflow:xxx' 护照 ID 形态，剥前缀取模板 ID
        let workflow_id = raw_id.strip_prefix("workflow:").unwrap_or(&raw_id).to_string();

        let executor = WORKFLOW_EXECUTOR.get().ok_or_else(executor_not_set_error)?;
        // 两种入参形态兼容：
        // 1) { workflow_id, input: {...} } —— 显式 input 键
        // 2) { workflow_id, <工作流入参...> } —— CapabilityLoad 激活的扁平 schema 形态，
        //    除 workflow_id/input 外的顶层键整体作为执行输入透传
        let exec_input = match input.get("input") {
            Some(v) if !v.is_null() => Some(v.clone()),
            _ => {
                let mut rest = input.clone();
                if let Value::Object(map) = &mut rest {
                    map.remove("workflow_id");
                    map.remove("input");
                }
                (rest.as_object().is_some_and(|m| !m.is_empty())).then_some(rest)
            },
        };
        let conversation_id = ctx.conversation_id.clone().filter(|c| !c.trim().is_empty());

        let value = executor(workflow_id.clone(), exec_input, conversation_id)
            .await
            .map_err(execute_failed_error)?;

        // 输出统一为文本：字符串直用，结构化值 pretty JSON
        let content = match &value {
            Value::String(s) if !s.trim().is_empty() => s.clone(),
            Value::Null => "(工作流执行完成，无输出)".to_string(),
            other => serde_json::to_string_pretty(other)
                .unwrap_or_else(|_| "(工作流输出序列化失败)".to_string()),
        };

        Ok(ToolResult {
            content,
            is_error: false,
            truncated: false,
            metadata: Some(json!({
                "tool": RUN_WORKFLOW_TOOL,
                "workflowId": workflow_id,
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata_matches_shared_constant() {
        let tool = RunWorkflowTool;
        // 注册名与护照 tool_ref 共用常量，锁定一致性
        assert_eq!(tool.name(), RUN_WORKFLOW_TOOL);
        assert_eq!(tool.name(), "RunWorkflow");
    }

    #[test]
    fn executor_not_set_error_carries_workflow_code() {
        let err = executor_not_set_error();
        assert_eq!(err.error_code, "WORKFLOW_EXECUTOR_NOT_SET");
        assert_eq!(err.kind, ToolErrorKind::ExecutionFailed);
    }

    #[test]
    fn execute_failed_error_carries_workflow_code() {
        let err = execute_failed_error("引擎爆炸".to_string());
        assert_eq!(err.error_code, "WORKFLOW_EXECUTE_FAILED");
        assert_eq!(err.kind, ToolErrorKind::ExecutionFailed);
    }

    #[test]
    fn missing_workflow_id_is_invalid_input() {
        // 未注入执行器也会先撞参数校验 —— 校验失败不依赖 wiring
        let tool = RunWorkflowTool;
        let ctx = ToolContext::new("test-dir");
        let err = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(tool.call(json!({}), &ctx))
            .expect_err("workflow_id 缺失必须报错");
        assert!(err.message.contains("workflow_id"));
    }

    #[tokio::test]
    async fn executes_via_injected_executor_and_strips_prefix() {
        let tool = RunWorkflowTool;
        let ctx = ToolContext::new("test-dir");
        let called = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let called_clone = called.clone();
        set_workflow_executor(Arc::new(
            move |wf: String, input: Option<Value>, conv: Option<String>| {
                called_clone.lock().push((wf.clone(), input.clone(), conv.clone()));
                Box::pin(async move {
                    // 断言 'workflow:' 前缀已被剥离
                    assert_eq!(wf, "tpl_1");
                    Ok(json!({"result": format!("done:{input:?}")}))
                }) as Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>
            },
        ));

        let result = tool
            .call(json!({"workflow_id": "workflow:tpl_1", "input": {"stock_code": "301302"}}), &ctx)
            .await
            .expect("注入执行器后应执行成功");
        assert!(result.content.contains("done:"));
        assert!(!result.is_error);
        let calls = called.lock();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "tpl_1");
        assert_eq!(calls[0].1.as_ref().and_then(|v| v["stock_code"].as_str()), Some("301302"));
    }
}
