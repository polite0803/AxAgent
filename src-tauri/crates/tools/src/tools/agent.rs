//! AgentTool - 子 Agent 创建和生命周期管理
//! 内置 6 个 Agent 类型：GeneralPurpose/Explore/Plan/Verification/Guide/StatuslineSetup

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::{LazyLock, RwLock};

/// 内置 Agent 定义
#[derive(Clone)]
pub struct AgentDef {
    pub name: &'static str,
    pub description: &'static str,
    pub allowed_tools: &'static [&'static str],
    pub disallowed_tools: &'static [&'static str],
}

/// 内置 Agent 注册表
static BUILTIN_AGENTS: LazyLock<RwLock<Vec<AgentDef>>> = LazyLock::new(|| {
    RwLock::new(vec![
        AgentDef {
            name: "general-purpose",
            description: "通用 Agent，可调用所有工具",
            allowed_tools: &[],
            disallowed_tools: &["EnterPlanMode", "ExitPlanMode"],
        },
        AgentDef {
            name: "Explore",
            description: "代码探索 Agent，只读工具",
            allowed_tools: &[
                "FileRead",
                "Glob",
                "Grep",
                "WebFetch",
                "WebSearch",
                "CtxInspect",
                "ListPeers",
            ],
            disallowed_tools: &[],
        },
        AgentDef {
            name: "Plan",
            description: "架构设计 Agent，探索+设计",
            allowed_tools: &[
                "FileRead",
                "Glob",
                "Grep",
                "WebFetch",
                "WebSearch",
                "TodoWrite",
            ],
            disallowed_tools: &["FileWrite", "FileEdit", "Bash"],
        },
        AgentDef {
            name: "Verification",
            description: "验证 Agent，只读验证实现",
            allowed_tools: &["FileRead", "Glob", "Grep", "Bash", "TodoWrite"],
            disallowed_tools: &["FileWrite", "FileEdit"],
        },
        AgentDef {
            name: "Guide",
            description: "指南 Agent，回答关于 Claude Code/Agent SDK 的问题",
            allowed_tools: &["FileRead", "Glob", "Grep", "WebFetch", "WebSearch"],
            disallowed_tools: &["FileWrite", "FileEdit", "Bash"],
        },
        AgentDef {
            name: "StatuslineSetup",
            description: "状态栏配置 Agent",
            allowed_tools: &["FileRead", "FileWrite", "FileEdit"],
            disallowed_tools: &["Bash"],
        },
    ])
});

pub struct AgentTool;

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "Agent"
    }
    fn description(&self) -> &str {
        "创建子 Agent 处理复杂多步骤任务。6 种内置类型：general-purpose(通用)/Explore(代码探索)/Plan(架构设计)/Verification(验证)/Guide(指南)/StatuslineSetup(状态栏)。支持后台执行(120s 自动转后台)、fork 缓存复用、结果摘要。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "description": {"type":"string","description":"任务简短描述(3-5词)"},
                "prompt": {"type":"string","description":"子 Agent 完整任务指令"},
                "subagent_type": {"type":"string","enum":["general-purpose","Explore","Plan","Verification","Guide","StatuslineSetup"],"default":"general-purpose"},
                "model": {"type":"string","description":"模型(默认继承父Agent)"},
                "run_in_background": {"type":"boolean","default":false},
                "isolation": {"type":"string","enum":["none","worktree"],"default":"none"}
            },
            "required": ["description","prompt"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn aliases(&self) -> &[&str] {
        &["Task", "SubAgent"]
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let description = input["description"].as_str().unwrap_or("未命名");
        let prompt = input["prompt"].as_str().unwrap_or("");
        let agent_type = input["subagent_type"].as_str().unwrap_or("general-purpose");
        let background = input["run_in_background"].as_bool().unwrap_or(false);
        let isolation = input["isolation"].as_str().unwrap_or("none");

        // 查找内置 Agent 定义
        let agents = BUILTIN_AGENTS.read().unwrap();
        let agent_def = agents.iter().find(|a| a.name == agent_type);

        let emoji = match agent_type {
            "Explore" => "🔍",
            "Plan" => "📐",
            "Verification" => "✅",
            "Guide" => "📖",
            "StatuslineSetup" => "⚙️",
            _ => "🤖",
        };

        let mut output = format!("## {} 子 Agent 已创建\n\n", emoji);
        output.push_str(&format!("**名称**: {}\n", description));
        output.push_str(&format!("**类型**: {}\n", agent_type));
        output.push_str(&format!(
            "**后台**: {}\n",
            if background {
                "是（120s 超时自动转后台）"
            } else {
                "否"
            }
        ));
        output.push_str(&format!("**隔离**: {}\n", isolation));
        output.push_str(&format!(
            "**父会话**: {}\n\n",
            ctx.conversation_id.as_deref().unwrap_or("unknown")
        ));

        if let Some(def) = agent_def {
            output.push_str("**工具权限**: ");
            if def.allowed_tools.is_empty() {
                output.push_str("全部（除禁止项）\n");
            } else {
                output.push_str(&format!("允许: {}\n", def.allowed_tools.join(", ")));
            }
            if !def.disallowed_tools.is_empty() {
                output.push_str(&format!("禁止: {}\n", def.disallowed_tools.join(", ")));
            }
        }

        output.push_str(&format!("\n---\n**任务**:\n```\n{}\n```\n\n", prompt));
        output.push_str("🎯 子 Agent 已启动，执行完成后将返回结果摘要。\n");

        if let Some(conv_id) = &ctx.conversation_id {
            crate::builtin_tools::store_pending_sub_agent_card(
                conv_id,
                conv_id,
                agent_type,
                description,
            );
        }

        Ok(ToolResult::success(output))
    }
}

/// 注册自定义 Agent
pub fn register_agent(def: AgentDef) {
    BUILTIN_AGENTS.write().unwrap().push(def);
}

/// 列出所有已注册 Agent
pub fn list_agents() -> Vec<AgentDef> {
    BUILTIN_AGENTS.read().unwrap().clone()
}
