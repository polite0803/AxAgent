//! 统一 Agent 定义类型
//!
//! 支持三种来源：
//! - `BuiltIn`：硬编码的内置 agent
//! - `User`：`~/.axagent/agents/*.md` 用户自定义
//! - `Project`：`<project>/.axagent/agents/*.md` 项目自定义

use serde::{Deserialize, Serialize};

/// Agent 定义来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentDefSource {
    BuiltIn,
    User,
    Project,
    Plugin,
}

/// Agent 隔离模式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationMode {
    None,
    Worktree,
    Remote,
}

/// Agent 记忆作用域
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryScope {
    /// `~/.axagent/agent-memory/<agent-type>/`
    User,
    /// `.axagent/agent-memory/<agent-type>/`
    Project,
    /// `.axagent/agent-memory-local/<agent-type>/` 不纳入 VCS
    Local,
}

/// MCP 服务端配置（agent 级别）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMcpServerSpec {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

/// 统一的 Agent 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// 唯一标识（如 `general-purpose`, `Explore`, `my-reviewer`）
    pub agent_type: String,
    /// 定义来源
    pub source: AgentDefSource,
    /// 一行描述
    pub description: String,
    /// 使用场景说明（whenToUse）
    pub when_to_use: String,
    /// 允许使用的工具列表（空 = 全部允许）
    pub tools: Vec<String>,
    /// 禁止使用的工具列表
    pub disallowed_tools: Vec<String>,
    /// 预加载的 skill 名称
    pub skills: Vec<String>,
    /// MCP 服务端配置
    pub mcp_servers: Vec<AgentMcpServerSpec>,
    /// 模型指定（None = 继承父 agent）
    pub model: Option<String>,
    /// 是否始终后台执行
    pub background: bool,
    /// 隔离模式
    pub isolation: Option<IsolationMode>,
    /// 权限模式
    pub permission_mode: Option<String>,
    /// 最大执行轮次
    pub max_turns: Option<u32>,
    /// 持久化记忆作用域
    pub memory_scope: Option<MemoryScope>,
    /// UI 颜色
    pub color: Option<String>,
    /// 自定义 hook 命令列表（每个 agent 可配置自己的 hook）
    pub hooks: Vec<String>,
    /// 是否省略 CLAUDE.md 上下文
    pub omit_claude_md: bool,
    /// 初始提示词
    pub initial_prompt: Option<String>,
    /// 系统提示词（从 Markdown 正文提取）
    pub system_prompt: Option<String>,
    /// 定义文件路径（自定义 agent）
    pub source_path: Option<String>,
}

impl AgentDefinition {
    /// 创建一个新的内置 agent 定义
    pub fn builtin(agent_type: &str, description: &str) -> Self {
        Self {
            agent_type: agent_type.to_string(),
            source: AgentDefSource::BuiltIn,
            description: description.to_string(),
            when_to_use: String::new(),
            tools: Vec::new(),
            disallowed_tools: Vec::new(),
            skills: Vec::new(),
            mcp_servers: Vec::new(),
            model: None,
            background: false,
            isolation: None,
            permission_mode: None,
            max_turns: None,
            memory_scope: None,
            color: None,
            hooks: Vec::new(),
            omit_claude_md: false,
            initial_prompt: None,
            system_prompt: None,
            source_path: None,
        }
    }

    /// 快速设置工具列表
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    /// 快速设置禁止的工具列表
    pub fn with_disallowed_tools(mut self, disallowed: Vec<String>) -> Self {
        self.disallowed_tools = disallowed;
        self
    }

    /// 快速设置模型
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = Some(model.to_string());
        self
    }

    /// 快速设置后台运行
    pub fn with_background(mut self, background: bool) -> Self {
        self.background = background;
        self
    }

    /// 快速设置描述
    pub fn with_when_to_use(mut self, when: &str) -> Self {
        self.when_to_use = when.to_string();
        self
    }

    /// 是否有完整的工具白名单（非空 = 受限）
    pub fn has_tool_allowlist(&self) -> bool {
        !self.tools.is_empty()
    }

    /// 检查指定的工具是否被允许
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        if !self.disallowed_tools.is_empty()
            && self.disallowed_tools.contains(&tool_name.to_string())
        {
            return false;
        }
        if self.tools.is_empty() {
            return true;
        }
        self.tools.contains(&tool_name.to_string())
    }
}
