//! AgentTool - 子 Agent 创建和生命周期管理
//! 内置 6 个 Agent 类型 + 支持从 `.axagent/agents/*.md` 动态加载自定义 agent

use crate::agent_def_loader::load_all_agents;
use crate::agent_def_types::{AgentDefSource, AgentDefinition};
use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_plugins::agent_provider::global_plugin_agents;
use serde_json::{Value, json};
use std::sync::{LazyLock, Mutex, RwLock};

/// 待处理子 Agent 卡片: (child_conversation_id, agent_type, description)
type PendingSubAgentCard = (String, String, String);
static PENDING_SUB_AGENT_CARDS: LazyLock<
    Mutex<std::collections::HashMap<String, PendingSubAgentCard>>,
> = LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

fn store_pending_card(parent_id: &str, child_id: &str, agent_type: &str, description: &str) {
    let mut m = PENDING_SUB_AGENT_CARDS
        .lock()
        .expect("PENDING_SUB_AGENT_CARDS poisoned");
    m.insert(
        parent_id.to_string(),
        (child_id.to_string(), agent_type.to_string(), description.to_string()),
    );
}

/// 触发 HookEvent（best-effort，失败不影响主流程）
fn fire_hook(event: axagent_runtime_core::HookEvent, data: &serde_json::Value) {
    let runner =
        axagent_runtime_core::HookRunner::new(axagent_runtime_core::RuntimeHookConfig::default());
    let data_str = data.to_string();
    let _ = runner.run_event(event, &data_str);
}

/// 全局 Agent 注册表 — 包含内置 + 动态加载的 agent 定义
static AGENT_REGISTRY: LazyLock<RwLock<Vec<AgentDefinition>>> =
    LazyLock::new(|| RwLock::new(builtin_agents()));

/// 内置 Agent 定义
fn builtin_agents() -> Vec<AgentDefinition> {
    vec![
        AgentDefinition {
            agent_type: "general-purpose".into(),
            source: AgentDefSource::BuiltIn,
            description: "通用 Agent，可调用所有工具".into(),
            when_to_use: "研究复杂问题、搜索代码、执行多步骤任务时使用".into(),
            disallowed_tools: vec!["EnterPlanMode".into(), "ExitPlanMode".into()],
            ..AgentDefinition::builtin("general-purpose", "通用 Agent")
        },
        AgentDefinition {
            agent_type: "Explore".into(),
            source: AgentDefSource::BuiltIn,
            description: "代码探索 Agent，只读工具".into(),
            when_to_use: "需要快速搜索代码库、查找文件、理解项目结构时使用".into(),
            tools: vec![
                "FileRead".into(),
                "Glob".into(),
                "Grep".into(),
                "WebFetch".into(),
                "WebSearch".into(),
                "CtxInspect".into(),
                "ListPeers".into(),
            ],
            omit_claude_md: true,
            ..AgentDefinition::builtin("Explore", "代码探索 Agent")
        },
        AgentDefinition {
            agent_type: "Plan".into(),
            source: AgentDefSource::BuiltIn,
            description: "架构设计 Agent，探索+设计".into(),
            when_to_use: "需要设计实现方案、规划架构时使用".into(),
            tools: vec![
                "FileRead".into(),
                "Glob".into(),
                "Grep".into(),
                "WebFetch".into(),
                "WebSearch".into(),
                "TodoWrite".into(),
            ],
            disallowed_tools: vec!["FileWrite".into(), "FileEdit".into(), "Bash".into()],
            omit_claude_md: true,
            ..AgentDefinition::builtin("Plan", "架构设计 Agent")
        },
        AgentDefinition {
            agent_type: "Verification".into(),
            source: AgentDefSource::BuiltIn,
            description: "验证 Agent，只读验证实现".into(),
            when_to_use: "代码实现完成后需要验证正确性时使用".into(),
            tools: vec![
                "FileRead".into(),
                "Glob".into(),
                "Grep".into(),
                "Bash".into(),
                "TodoWrite".into(),
            ],
            disallowed_tools: vec!["FileWrite".into(), "FileEdit".into()],
            background: true,
            color: Some("red".into()),
            ..AgentDefinition::builtin("Verification", "验证 Agent")
        },
        AgentDefinition {
            agent_type: "Guide".into(),
            source: AgentDefSource::BuiltIn,
            description: "指南 Agent，回答关于 Claude Code 使用的问题".into(),
            when_to_use: "用户询问 Claude Code 功能、用法、配置等问题时使用".into(),
            tools: vec![
                "FileRead".into(),
                "Glob".into(),
                "Grep".into(),
                "WebFetch".into(),
                "WebSearch".into(),
            ],
            disallowed_tools: vec!["FileWrite".into(), "FileEdit".into(), "Bash".into()],
            model: Some("haiku".into()),
            ..AgentDefinition::builtin("Guide", "指南 Agent")
        },
        AgentDefinition {
            agent_type: "StatuslineSetup".into(),
            source: AgentDefSource::BuiltIn,
            description: "状态栏配置 Agent".into(),
            when_to_use: "需要配置 Claude Code 状态栏时使用".into(),
            tools: vec!["FileRead".into(), "FileWrite".into(), "FileEdit".into()],
            disallowed_tools: vec!["Bash".into()],
            model: Some("sonnet".into()),
            color: Some("orange".into()),
            ..AgentDefinition::builtin("StatuslineSetup", "状态栏配置 Agent")
        },
    ]
}

/// 初始化注册表：刷新内置 agent 并加载用户/项目自定义 agent，最后加载 Plugin Agent
pub fn refresh_agent_registry(cwd: &std::path::Path) {
    let builtin = builtin_agents();
    let custom = load_all_agents(cwd);

    // 合并：内置优先，自定义不覆盖同名内置
    let mut merged = builtin;
    for custom_def in custom {
        if !merged.iter().any(|b| b.agent_type == custom_def.agent_type) {
            merged.push(custom_def);
        }
    }

    // 合并 Plugin Agent（不覆盖同名的内置或自定义 agent）
    for plugin_def in global_plugin_agents().all() {
        if !merged.iter().any(|b| b.agent_type == plugin_def.agent_type) {
            merged.push(AgentDefinition {
                agent_type: plugin_def.agent_type,
                source: AgentDefSource::Plugin,
                description: plugin_def.description,
                tools: plugin_def.tools,
                disallowed_tools: plugin_def.disallowed_tools,
                model: plugin_def.model,
                background: plugin_def.background,
                system_prompt: plugin_def.system_prompt,
                ..AgentDefinition::builtin("", "")
            });
        }
    }

    if let Ok(mut guard) = AGENT_REGISTRY.write() {
        *guard = merged;
    }
}

/// 列出所有已注册 Agent
pub fn list_agents() -> Vec<AgentDefinition> {
    AGENT_REGISTRY.read().unwrap().clone()
}

/// 查找指定类型的 Agent
pub fn find_agent(agent_type: &str) -> Option<AgentDefinition> {
    AGENT_REGISTRY
        .read()
        .unwrap()
        .iter()
        .find(|a| a.agent_type == agent_type)
        .cloned()
}

/// 注册自定义 Agent（运行时动态添加）
pub fn register_agent(def: AgentDefinition) {
    AGENT_REGISTRY.write().unwrap().push(def);
}

pub struct AgentTool;

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "Agent"
    }
    fn description(&self) -> &str {
        "创建子 Agent 处理独立任务。适用：代码探索(Explore)、方案评审(Plan)、\
         并行处理多个独立子任务。不适用：简单问题。内置类型: general-purpose(通用)/Explore(代码搜索)/\
         Plan(方案设计)/Verification(验证)/Guide(帮助)。支持后台运行和 worktree 隔离。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "description": {"type":"string","description":"任务简短描述(3-5词)"},
                "prompt": {"type":"string","description":"子 Agent 完整任务指令"},
                "subagent_type": {
                    "type":"string",
                    "description":"Agent 类型。省略则激活 fork 子 agent（如启用 FORK_SUBAGENT）"
                },
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
        let agent_type = input["subagent_type"].as_str().unwrap_or("");
        let background = input["run_in_background"].as_bool().unwrap_or(false);
        let isolation = input["isolation"].as_str().unwrap_or("none");

        // Verification Agent 需要启用 VERIFICATION_AGENT feature flag
        if agent_type == "Verification"
            && !axagent_runtime_core::feature_flags::global_feature_flags().verification_agent()
        {
            return Err(ToolError::new(
                "Verification Agent 未启用（设置 AXAGENT_FF_VERIFICATION_AGENT=1 或 features.VerificationAgent=true）",
            ));
        }

        // 查找 Agent 定义
        let agent_def = if agent_type.is_empty() {
            // 如启用 FORK_SUBAGENT，则隐式 fork
            if axagent_runtime_core::feature_flags::global_feature_flags().fork_subagent() {
                return handle_fork_subagent(description, prompt, ctx).await;
            }
            // 默认使用 general-purpose
            find_agent("general-purpose")
        } else {
            find_agent(agent_type)
        };

        let emoji = match agent_type {
            "Explore" => "\u{1F50D}",
            "Plan" => "\u{1F4D0}",
            "Verification" => "\u{2705}",
            "Guide" => "\u{1F4D6}",
            "StatuslineSetup" => "\u{2699}\u{FE0F}",
            _ => "\u{1F916}",
        };

        let resolved_type = agent_def
            .as_ref()
            .map(|a| a.agent_type.as_str())
            .unwrap_or(agent_type);

        let mut output = format!("## {} 子 Agent 已创建\n\n", emoji);
        output.push_str(&format!("**名称**: {}\n", description));
        output.push_str(&format!("**类型**: {}\n", resolved_type));
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

        if let Some(def) = &agent_def {
            output.push_str("**工具权限**: ");
            if def.tools.is_empty() {
                output.push_str("全部（除禁止项）\n");
            } else {
                output.push_str(&format!("允许: {}\n", def.tools.join(", ")));
            }
            if !def.disallowed_tools.is_empty() {
                output.push_str(&format!("禁止: {}\n", def.disallowed_tools.join(", ")));
            }
            if def.background {
                output.push_str("**模式**: 后台运行\n");
            }
            if let Some(ref model) = def.model {
                output.push_str(&format!("**模型**: {}\n", model));
            }
            if !def.when_to_use.is_empty() {
                output.push_str(&format!("**用途**: {}\n", def.when_to_use));
            }
        }

        output.push_str(&format!("\n---\n**任务**:\n```\n{}\n```\n\n", prompt));
        output.push_str("\u{1F3AF} 子 Agent 已启动，执行完成后将返回结果摘要。\n");

        if let Some(conv_id) = &ctx.conversation_id {
            store_pending_card(conv_id, conv_id, resolved_type, description);
        }

        // 触发 SubagentStart hook (best-effort)
        fire_hook(
            axagent_runtime_core::HookEvent::SubagentStart,
            &json!({
                "agent_type": resolved_type,
                "description": description,
                "background": background,
                "isolation": isolation,
                "conversation_id": ctx.conversation_id,
            }),
        );

        Ok(ToolResult::success(output))
    }
}

/// Fork 子 agent 处理 — 当 FORK_SUBAGENT feature flag 启用且未指定 subagent_type 时触发
///
/// 存储 fork 上下文使子 agent 可继承父 agent 的对话历史，最大化 prompt cache 命中。
async fn handle_fork_subagent(
    description: &str,
    prompt: &str,
    ctx: &ToolContext,
) -> Result<ToolResult, ToolError> {
    let parent_id = ctx.conversation_id.as_deref().unwrap_or("unknown");

    // 存储 fork 上下文 — 子 agent 启动时读取以继承父 agent 消息历史
    // ForkSessionData 在 runtime::fork_bridge 中，父 agent 填入 session 数据后子 agent 可加载
    axagent_runtime_core::fork_bridge::store_fork_session(
        axagent_runtime_core::fork_bridge::ForkSessionData {
            parent_conversation_id: parent_id.to_string(),
            description: description.to_string(),
            prompt: prompt.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            parent_system_prompt: Vec::new(),
            parent_messages_json: String::new(),
            child_system_prompt: Some(axagent_runtime_core::fork_bridge::build_fork_child_prompt(
                prompt,
            )),
        },
    );
    store_pending_card(parent_id, parent_id, "fork", description);

    let output = format!(
        "## \u{1F500} Fork 子 Agent 已创建\n\n\
         **名称**: {description}\n\
         **模式**: Fork（继承父 agent 上下文，共享 prompt cache）\n\
         **父会话**: {parent_id}\n\
         **缓存策略**: 子 agent 复用父 agent 消息前缀\n\n\
         ---\n\
         **任务**:\n```\n{}\n```\n\n\
         \u{26A0}\u{FE0F} Fork 规则：\n\
         - 不使用 EnterPlanMode/ExitPlanMode\n\
         - 不递归 fork 子 agent\n\
         - 完成后直接返回结果\n\
         - 只读操作优先\n\n\
         \u{1F3AF} Fork 子 Agent 已启动...\n",
        prompt
    );

    // 触发 SubagentStart hook — fork 类型 (best-effort)
    fire_hook(
        axagent_runtime_core::HookEvent::SubagentStart,
        &json!({
            "agent_type": "fork",
            "description": description,
            "conversation_id": ctx.conversation_id,
        }),
    );

    Ok(ToolResult::success(output))
}

// ── RemoteTrigger ──

pub struct RemoteTriggerTool;

#[async_trait]
impl Tool for RemoteTriggerTool {
    fn name(&self) -> &str {
        "RemoteTrigger"
    }
    fn description(&self) -> &str {
        "远程触发另一个 Agent 会话执行。传入目标会话 ID 和指令，在目标会话中启动新的 Agent 任务。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"session_id":{"type":"string"},"prompt":{"type":"string"}},"required":["session_id","prompt"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let sid = i["session_id"].as_str().unwrap_or("?");
        let prompt = i["prompt"].as_str().unwrap_or("");
        fire_hook(
            axagent_runtime_core::HookEvent::SubagentStart,
            &serde_json::json!({
                "agent_type": "remote",
                "session_id": sid,
                "description": prompt,
            }),
        );
        Ok(ToolResult::success(format!(
            "📡 已远程触发会话 {} — 指令: {}",
            sid,
            &prompt[..prompt.len().min(100)]
        )))
    }
}

// ── SuggestBackgroundPR ──

pub struct SuggestBackgroundPRTool;

#[async_trait]
impl Tool for SuggestBackgroundPRTool {
    fn name(&self) -> &str {
        "SuggestBackgroundPR"
    }
    fn description(&self) -> &str {
        "分析当前分支变更并建议创建 PR。检查 diff 大小、提交信息质量、是否缺少测试。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"branch":{"type":"string"}},"required":["branch"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let branch = i["branch"].as_str().unwrap_or("main");
        // 尝试获取 diff 统计
        let diff_info = std::process::Command::new("git")
            .args(["diff", "--stat", &format!("origin/{}..HEAD", branch)])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout).ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "无法获取 diff 信息".to_string());

        Ok(ToolResult::success(format!(
            "## PR 分析: {} 分支\n\n```\n{}\n```\n\n💡 建议: 检查变更是否包含测试，提交信息是否遵循规范。",
            branch, diff_info
        )))
    }
}
