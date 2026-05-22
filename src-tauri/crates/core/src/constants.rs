//! 共享字符串常量
//! 消除代码中的魔法字符串。按语义分组为子模块。

pub mod role {
    pub const USER: &str = "user";
    pub const SYSTEM: &str = "system";
    pub const ASSISTANT: &str = "assistant";
    pub const TOOL: &str = "tool";
}

pub mod status {
    pub const RUNNING: &str = "running";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
    pub const PENDING: &str = "pending";
    pub const PROCESSING: &str = "processing";
    pub const SKIPPED: &str = "skipped";
    pub const ARCHIVED: &str = "archived";
    pub const PAUSED: &str = "paused";
    pub const CANCELLED: &str = "cancelled";
    pub const PARTIAL: &str = "partial";
    pub const ERROR: &str = "error";
    pub const READY: &str = "ready";
    pub const INDEXING: &str = "indexing";
}

pub mod api_field {
    pub const CONVERSATION_ID: &str = "conversationId";
    pub const ERROR: &str = "error";
    pub const CODE: &str = "code";
    pub const MESSAGE: &str = "message";
    pub const TYPE: &str = "type";
    pub const NAME: &str = "name";
    pub const ID: &str = "id";
    pub const CONTENT: &str = "content";
    pub const REASON: &str = "reason";
    pub const SUCCESS: &str = "success";
    pub const SOURCE: &str = "source";
    pub const TARGET: &str = "target";
    pub const TOOL_USE: &str = "tool_use";
    pub const TOOL_RESULT: &str = "tool_result";
    pub const IS_ERROR: &str = "is_error";
    pub const FUNCTION: &str = "function";
    pub const TEXT: &str = "text";
    pub const WEB_SEARCH: &str = "web_search";
}

pub mod event_name {
    pub const AGENT_ERROR: &str = "agent-error";
    pub const AGENT_STREAM: &str = "agent-stream";
    pub const AGENT_STATUS: &str = "agent-status";
}

pub mod transport {
    pub const STREAMABLE_HTTP: &str = "streamableHttp";
    pub const STDIO: &str = "stdio";
    pub const SSE: &str = "sse";
    pub const BUILTIN: &str = "builtin";
    pub const HTTP: &str = "http";
}

pub mod file_name {
    pub const SKILL_MD: &str = "SKILL.md";
    pub const MANIFEST_JSON: &str = "manifest.json";
    pub const SKILL_MANIFEST_JSON: &str = "skill-manifest.json";
    pub const MASTER_KEY: &str = "master.key";
    pub const AXAGENT_DB: &str = "axagent.db";
    pub const SETTINGS_JSON: &str = "settings.json";
}

pub mod provider {
    pub const OPENAI: &str = "openai";
    pub const ANTHROPIC: &str = "anthropic";
    pub const GEMINI: &str = "gemini";
    pub const OLLAMA: &str = "ollama";
}

pub mod default_url {
    pub const OPENAI_BASE: &str = "https://api.openai.com/v1";
    pub const ANTHROPIC_BASE: &str = "https://api.anthropic.com/v1";
    pub const GEMINI_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
    pub const OLLAMA_HOST: &str = "http://localhost:11434";
    pub const OPENCLAW_HOST: &str = "http://localhost:8100";
    pub const HERMES_HOST: &str = "http://localhost:8642";
    pub const REPLICATE_API: &str = "https://api.replicate.com/v1/predictions";
    pub const SKILLS_HUB_API: &str = "https://api.agentskills.io";
}

pub mod platform {
    pub const TELEGRAM: &str = "telegram";
    pub const DISCORD: &str = "discord";
    pub const SLACK: &str = "slack";
    pub const WECHAT: &str = "wechat";
    pub const FEISHU: &str = "feishu";
}

pub mod locale {
    pub const ZH_CN: &str = "zh-CN";
    pub const EN_US: &str = "en-US";
}

pub mod android_msg {
    pub const BROWSER_NOT_AVAILABLE: &str = "Browser automation is not available on Android";
    pub const COMPUTER_CONTROL_NOT_AVAILABLE: &str = "Computer control is not available on Android";
    pub const UI_AUTOMATION_NOT_SUPPORTED: &str = "UI automation is not supported on Android";
    pub const GIT_TOOLS_NOT_AVAILABLE: &str = "Git tools are not available on Android";
    pub const SANDBOX_NOT_AVAILABLE: &str = "Sandbox execution is not available on Android";
}

pub mod window {
    pub const MIN_WIDTH: f64 = 640.0;
    pub const MIN_HEIGHT: f64 = 480.0;
}
