// SPDX-License-Identifier: AGPL-3.0-only
//! CapabilityLoad — 渐进式披露 L1.5「加载层」：把能力从「可发现」推进到「可执行」。
//!
//! # 三层披露的完整闭环
//!
//! ```text
//! L0  <capability-index>  目录摘要   —— 系统提示静态注入
//! L1  CapabilityView       完整定义   —— 查看，不改任何状态
//! L1.5 CapabilityLoad      加载       —— 本工具：写会话状态 + 激活工具（有副作用）
//! L2  下轮注入             内容就位   —— Processor 读状态注入 <loaded-capabilities>
//! ```
//!
//! # 为什么它必须有副作用
//!
//! 此前 L1 只有「查看」语义：展开定义后工具仍不在 `chat_tools` 里，LLM 看得见
//! 调不动。加载动作的本质是**状态迁移** —— 写入会话状态（供下轮注入读取）
//! 并把工具定义追加进 `DynamicToolSet`（供下一次 LLM 调用发起 function call）。
//!
//! # 与 CapabilityView 的分工
//!
//! - `CapabilityView`：纯只读，返回 JSON 定义，**不改状态**（category 为 Knowledge）
//! - `CapabilityLoad`：写状态 + 激活工具，返回一行确认，**不返回正文**
//!   —— 正文由下轮的 `LoadedCapabilityContributor` 注入，避免本轮白占上下文
//!
//! # 分层合规
//! 与 `capability_view.rs` 同惯例：`CapabilityIndexer` 经 `OnceLock` + setter 注入。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolErrorKind, ToolResult};
use async_trait::async_trait;
use axagent_harness::error_codes::capability::{
    LOAD_FAILED, LOAD_NO_CONTEXT, LOAD_NO_STORE, NOT_FOUND as CAPABILITY_NOT_FOUND,
};
use axagent_harness::session_state::{NS_SKILL_LOADED, StateScope, scoped_key};
use axagent_harness::types::{ChatTool, ChatToolFunction};
use serde_json::{Value, json};
use std::sync::{Arc, OnceLock};

use super::capability_shared::capability_indexer;

static SESSION_STATE: OnceLock<Arc<dyn axagent_harness::SessionStateStore>> = OnceLock::new();

/// 注入会话状态存储（wiring 层初始化时调用一次）。
///
/// 未注入时 `CapabilityLoad` 直接返回 `CAPABILITY_LOAD_NO_STORE` 错误，
/// 而不是静默降级成「只返回正文不落状态」—— 那会让调用方误以为已生效。
pub fn set_session_state_store(store: Arc<dyn axagent_harness::SessionStateStore>) {
    let _ = SESSION_STATE.set(store);
}

/// 已加载状态的默认存活时间：30 分钟。
///
/// 与认知编排的路由短路缓存（10 分钟）同量级但更长 —— 加载是显式动作，
/// 用户中途停顿不应导致已加载的技能被回收。
const DEFAULT_TTL_MS: i64 = 30 * 60 * 1000;

pub struct CapabilityLoadTool;

#[async_trait]
impl Tool for CapabilityLoadTool {
    fn name(&self) -> &str {
        "CapabilityLoad"
    }

    fn description(&self) -> &str {
        "加载指定能力，使其从「可发现」变为「可执行」（渐进式披露 L1.5 — 加载层）。\
         会写入会话状态：下一轮起该能力的完整定义注入 <loaded-capabilities>，\
         且 Tool 类能力立即可发起调用。\
         仅查看定义、不想产生副作用时用 CapabilityView。\
         参数 capability_id 取自系统提示的 <capability-index> 目录。"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "capability_id": {
                    "type": "string",
                    "description": "要加载的能力 ID（来自能力目录）"
                },
                "agent_id": {
                    "type": "string",
                    "description": "Agent 作用域（多 Agent 协作时必填，单 Agent 可省略）"
                },
                "ttl_ms": {
                    "type": "integer",
                    "description": "状态存活毫秒数，缺省 1800000（30 分钟）"
                }
            },
            "required": ["capability_id"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let capability_id = input["capability_id"].as_str().unwrap_or("").trim();
        if capability_id.is_empty() {
            return Err(ToolError::invalid_input_for("CapabilityLoad", "capability_id 为必填参数"));
        }

        let indexer = capability_indexer()
            .ok_or_else(|| not_found(format!("能力索引器尚未初始化，无法加载 {capability_id}")))?;

        let passport = indexer
            .get_passport(capability_id)
            .await
            .ok_or_else(|| not_found(format!("能力 '{capability_id}' 未在索引中")))?;

        // 与索引层同口径：不可见的能力既不进目录，也不能被加载
        if !passport.is_user_visible() {
            return Err(ToolError {
                message: format!("能力 '{capability_id}' 不对当前上下文公开"),
                kind: ToolErrorKind::PermissionDenied,
                error_code: CAPABILITY_NOT_FOUND.to_string(),
            });
        }

        // conversation_id 是状态 key 的必要维度；缺失时无法保证隔离，直接失败
        let conversation_id =
            ctx.conversation_id.clone().filter(|c| !c.trim().is_empty()).ok_or_else(|| {
                ToolError {
                    message: "缺少 conversation_id，无法写入会话状态".to_string(),
                    kind: ToolErrorKind::ExecutionFailed,
                    error_code: LOAD_NO_CONTEXT.to_string(),
                }
            })?;

        let store = SESSION_STATE.get().ok_or_else(|| ToolError {
            message: "会话状态存储未注入，CapabilityLoad 不可用".to_string(),
            kind: ToolErrorKind::ExecutionFailed,
            error_code: LOAD_NO_STORE.to_string(),
        })?;

        let ttl_ms = input["ttl_ms"].as_i64().unwrap_or(DEFAULT_TTL_MS);
        let agent_id = input["agent_id"].as_str().or(ctx.agent_id.as_deref());

        let key = scoped_key(
            StateScope::Temp,
            NS_SKILL_LOADED,
            &conversation_id,
            agent_id,
            &passport.capability_id,
        );

        let record = json!({
            "capabilityId": passport.capability_id,
            "kind": passport.kind,
            "name": passport.name,
            "agentId": agent_id,
            "loadedAtMs": axagent_harness::util_fns::now_ms(),
        });
        let value = serde_json::to_string(&record)
            .map_err(|e| load_failed(format!("加载状态序列化失败: {e}")))?;

        store
            .set(&key, &value, Some(ttl_ms))
            .await
            .map_err(|e| load_failed(format!("写入会话状态失败: {e}")))?;

        // Tool / Toolchain 类能力：把工具定义追加进动态工具集，下一次 LLM 调用即可发起
        // function call。工具本体已在 UnifiedToolRegistry 注册（只是不在 chat_tools 白名单里），
        // 因此无需额外注册 handler —— 补的是「对模型可见」，不是「可执行」。
        let mut activated: Vec<String> = Vec::new();
        if let Some(set) = &ctx.dynamic_tools {
            for chat_tool in chat_tools_for(&passport) {
                if set.add(chat_tool.clone()) {
                    activated.push(chat_tool.function.name);
                }
            }
        }

        let kind = passport.kind.as_str();
        let mut out = format!(
            "✅ 已加载能力 '{}'（{}）。下一轮起其完整定义注入 <loaded-capabilities>。",
            passport.capability_id, kind
        );
        if !activated.is_empty() {
            out.push_str(&format!("\n\n本轮已激活工具：{}（可立即调用）", activated.join(", ")));
        } else if ctx.dynamic_tools.is_none() {
            out.push_str("\n\n⚠️ 未注入动态工具集：状态已写入，但 Tool 类能力需下一轮才可见。");
        }

        Ok(ToolResult {
            content: out,
            is_error: false,
            truncated: false,
            metadata: Some(json!({
                "capabilityId": passport.capability_id,
                "kind": passport.kind,
                "stateKey": key,
                "activatedTools": activated,
                "ttlMs": ttl_ms,
                "level": "1.5",
            })),
            duration_ms: None,
            progress: Vec::new(),
        })
    }
}

/// 从护照派生可直接下发给模型的工具定义。
///
/// - `Tool`：护照自带的 `tool_ref.tool_name`
/// - `Toolchain`：按 `steps` 展开各步骤引用的真实工具
/// - `Workflow`：激活 `RunWorkflow`（T3 执行链闭环）—— tool_ref 指向执行入口，
///   入参 schema 在工作流自身 `input_schema` 上补 `workflow_id` 必填项，
///   描述指明 workflow_id 取护照 capability_id，避免模型空手调用
/// - 其余类型（Skill / Template / KnowledgeBase）是**指令**而非可调用函数，
///   不生成工具定义 —— 它们的正文由下轮注入块承载。
fn chat_tools_for(passport: &axagent_harness::CapabilityPassportDto) -> Vec<ChatTool> {
    if passport.kind == axagent_harness::CapabilityKind::Workflow {
        return passport
            .tool_ref
            .as_ref()
            .map(|r| {
                vec![ChatTool {
                    r#type: "function".to_string(),
                    function: ChatToolFunction {
                        name: r.tool_name.clone(),
                        description: Some(format!(
                            "{}（调用时 workflow_id 固定填 \"{}\"，工作流入参直接作为顶层参数传入）",
                            passport.description, passport.capability_id
                        )),
                        parameters: Some(run_workflow_chat_schema(passport)),
                    },
                }]
            })
            .unwrap_or_default();
    }

    let names: Vec<String> = if passport.kind == axagent_harness::CapabilityKind::Toolchain {
        // 工具链按步骤顺序展开；护照内只存 ID，工具名在引用里
        passport.steps.iter().filter_map(|s| s.strip_prefix("tool:").map(str::to_string)).collect()
    } else {
        passport.tool_ref.as_ref().map(|r| vec![r.tool_name.clone()]).unwrap_or_default()
    };

    names
        .into_iter()
        .map(|name| ChatTool {
            r#type: "function".to_string(),
            function: ChatToolFunction {
                name,
                // 描述与入参 schema 取自护照本身 —— 护照是唯一权威来源，
                // 不在此处另造一份，避免两处漂移。
                description: Some(passport.description.clone()),
                parameters: passport.input_schema.clone(),
            },
        })
        .collect()
}

/// RunWorkflow 的 chat 入参 schema：工作流自身 `input_schema` 透传 + `workflow_id` 必填。
///
/// 生成的 schema 形态为扁平调用（`{workflow_id, <工作流入参...>}`），
/// `RunWorkflowTool::call` 会把除 `workflow_id`/`input` 外的顶层键整体作为执行输入透传。
fn run_workflow_chat_schema(passport: &axagent_harness::CapabilityPassportDto) -> Value {
    let mut schema = match passport.input_schema.clone() {
        Some(Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    if !schema.contains_key("type") {
        schema.insert("type".to_string(), Value::String("object".to_string()));
    }
    let props = schema.entry("properties").or_insert_with(|| json!({}));
    if let Value::Object(props) = props {
        props.insert(
            "workflow_id".to_string(),
            json!({
                "type": "string",
                "description": format!("工作流/能力 ID，固定填 \"{}\"", passport.capability_id)
            }),
        );
    }
    let mut required: Vec<Value> =
        schema.get("required").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    if !required.iter().any(|v| v.as_str() == Some("workflow_id")) {
        required.insert(0, Value::String("workflow_id".to_string()));
    }
    schema.insert("required".to_string(), Value::Array(required));
    Value::Object(schema)
}

fn not_found(message: String) -> ToolError {
    ToolError { message, kind: ToolErrorKind::NotFound, error_code: CAPABILITY_NOT_FOUND.into() }
}

fn load_failed(message: String) -> ToolError {
    ToolError { message, kind: ToolErrorKind::ExecutionFailed, error_code: LOAD_FAILED.into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_carries_capability_error_code() {
        let err = not_found("能力不存在".to_string());
        assert_eq!(err.kind, ToolErrorKind::NotFound);
        assert_eq!(err.error_code, CAPABILITY_NOT_FOUND);
    }

    #[test]
    fn load_failed_carries_load_error_code() {
        let err = load_failed("写入失败".to_string());
        assert_eq!(err.kind, ToolErrorKind::ExecutionFailed);
        assert_eq!(err.error_code, LOAD_FAILED);
    }

    #[test]
    fn category_is_agent() {
        assert_eq!(CapabilityLoadTool.category(), ToolCategory::Agent);
    }

    #[test]
    fn toolchain_passport_expands_step_tools() {
        let p = axagent_harness::CapabilityPassportDto {
            kind: axagent_harness::CapabilityKind::Toolchain,
            steps: vec!["tool:alpha".to_string(), "tool:beta".to_string()],
            ..Default::default()
        };
        let tools = chat_tools_for(&p);
        let names: Vec<&str> = tools.iter().map(|t| t.function.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn skill_passport_yields_no_tool() {
        let p = axagent_harness::CapabilityPassportDto {
            kind: axagent_harness::CapabilityKind::Skill,
            ..Default::default()
        };
        assert!(chat_tools_for(&p).is_empty(), "Skill 是指令不是可调用函数，不应生成工具定义");
    }
}
