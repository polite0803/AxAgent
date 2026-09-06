// SPDX-License-Identifier: AGPL-3.0-only

//! 共享字符串常量
//!
//! 消除代码中的魔法字符串。按语义分组为子模块。
//! 非用户可见的内部标识符，不需要 i18n 翻译。

/// 消息/对话角色
pub mod role {
    pub const USER: &str = "user";
    pub const SYSTEM: &str = "system";
    pub const ASSISTANT: &str = "assistant";
    pub const TOOL: &str = "tool";
}

/// 状态标记
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

/// JSON/API 字段名 — 前后端协议字段
pub mod api_field {
    pub const CONVERSATION_ID: &str = "conversationId";
    pub const MESSAGE_ID: &str = "messageId";
    pub const STREAMING_MESSAGE_ID: &str = "streamingMessageId";
    pub const ASSISTANT_MESSAGE_ID: &str = "assistantMessageId";
    pub const TEMPLATE_ID: &str = "templateId";
    pub const TEMPLATE_NAME: &str = "templateName";
    pub const SERVER_ID: &str = "serverId";
    pub const PROVIDER_ID: &str = "providerId";
    pub const SKILL_ID: &str = "skill_id";
    pub const ERROR: &str = "error";
    pub const CODE: &str = "code";
    pub const MESSAGE: &str = "message";
    pub const TYPE: &str = "type";
    pub const NAME: &str = "name";
    pub const ID: &str = "id";
    pub const INPUT: &str = "input";
    pub const OUTPUT: &str = "output";
    pub const CONTENT: &str = "content";
    pub const TEXT: &str = "text";
    pub const REASON: &str = "reason";
    pub const SUCCESS: &str = "success";
    pub const HIT: &str = "hit";
    pub const RESULT: &str = "result";
    pub const SOURCE: &str = "source";
    pub const TARGET: &str = "target";
    pub const POSITION: &str = "position";
    pub const TOOL_USE: &str = "tool_use";
    pub const TOOL_RESULT: &str = "tool_result";
    pub const TOOL_USE_ID: &str = "tool_use_id";
    pub const TOOL_NAME: &str = "tool_name";
    pub const IS_ERROR: &str = "is_error";
    pub const IMAGE_URL: &str = "image_url";
    pub const FUNCTION: &str = "function";
    pub const WEB_SEARCH: &str = "web_search";
    pub const DOCUMENT_ID: &str = "documentId";
    pub const CONVERSATION_ID_SHORT: &str = "conversation_id";
    pub const MESSAGE_COUNT: &str = "message_count";
    pub const RESPONSE_STYLE: &str = "response_style";
    pub const CONTENT_ADJUSTMENTS: &str = "content_adjustments";
    pub const SKILL_SUGGESTIONS: &str = "skill_suggestions";
    pub const IMPROVED: &str = "improved";
    pub const IS_RUNNING: &str = "is_running";
    pub const STATS: &str = "stats";
    pub const ENABLED: &str = "enabled";
    pub const SKIPPED: &str = "skipped";
}

/// 前端事件名 — emit 给前端的 event name
pub mod event_name {
    pub const AGENT_ERROR: &str = "agent-error";
    pub const AGENT_MESSAGE_ID: &str = "agent-message-id";
    pub const AGENT_MCP_LOAD_FAILED: &str = "agent-mcp-load-failed";
    pub const AGENT_STREAM: &str = "agent-stream";
    pub const AGENT_STATUS: &str = "agent-status";
    pub const CHAT_STREAM_ERROR: &str = "chat-stream-error";
    pub const SKILL_FILE_CHANGED: &str = "skill:file-changed";
    /// 认知编排路由观测事件（T6）：三层路由决策 / 执行分派 / 结束失败三时点 emit，
    /// 前端 cognitiveRouteStore 订阅后即时渲染，不再依赖同步返回值单通道。
    pub const COGNITIVE_ROUTE_EVENT: &str = "cognitive-route-event";
}

/// 传输类型
pub mod transport {
    pub const STREAMABLE_HTTP: &str = "streamableHttp";
    pub const STDIO: &str = "stdio";
    pub const SSE: &str = "sse";
    pub const BUILTIN: &str = "builtin";
    pub const HTTP: &str = "http";
}

/// 文件名
pub mod file_name {
    pub const SKILL_MD: &str = "SKILL.md";
    pub const MANIFEST_JSON: &str = "manifest.json";
    pub const SKILL_MANIFEST_JSON: &str = "skill-manifest.json";
    pub const FRONTEND_JSON: &str = "frontend.json";
    pub const MASTER_KEY: &str = "master.key";
    pub const AXAGENT_DB: &str = "axagent.db";
    pub const SETTINGS_JSON: &str = "settings.json";
    pub const INSTALLED_JSON: &str = "installed.json";
    pub const PLUGIN_JSON: &str = "plugin.json";
    pub const CLAUDE_PLUGIN_JSON: &str = ".claude-plugin/plugin.json";
    pub const WINDOW_STATE_JSON: &str = "window-state.json";
    pub const USER_MD: &str = "USER.md";
    pub const AGENTS_MD: &str = "AGENTS.md";
    pub const CLAUDE_MD: &str = "CLAUDE.md";
    /// 3.2 P2:长期记忆索引文件路径(原 `.axagent/memory.md` 单文件已废弃)
    ///
    /// 现在是 200 行硬限制的索引文件,索引 `.axagent/memory/` 下四类分目录
    /// 的主题文件。详见 `memory::MEMORY_INDEX`。
    pub const MEMORY_MD: &str = ".axagent/MEMORY.md";
    pub const CRASH_LOG: &str = "axagent-crash.log";
    pub const STARTUP_PHASE: &str = ".startup_phase";
    pub const PRICING_TOML: &str = "pricing.toml";
    pub const SCREEN_PNG: &str = "screen.png";
}

/// 目录名/路径片段
pub mod dir_name {
    pub const AXAGENT: &str = ".axagent";
    pub const CLAUDE: &str = ".claude";
    pub const SKILLS: &str = "skills";
}

/// Unity 改造 Feature Flag 常量（与前端 FEATURE_FLAGS 双写对齐）
pub mod unity_feature_flag {
    /// P0: 任务形态分类器（原则三标尺：上下文保留成本 × 安全隔离需求）
    pub const UNITY_P0_TASK_SHAPE: &str = "UNITY_P0_TASK_SHAPE";
}

/// 3.1 P2:投机执行影子目录常量
///
/// 投机执行(CoW 覆盖文件系统轻量方案):等待用户确认时,后台投机执行
/// 工具调用,写入 `.axagent/shadow/{session_id}/` 影子目录。用户确认后
/// diff 应用到真实目录;用户拒绝时删除影子目录回滚。
///
/// 与 FUSE/驱动级 CoW 相比,此方案无需系统级支持,跨平台兼容。
pub mod shadow {
    /// 影子目录根目录(相对于项目根)
    pub const SHADOW_DIR: &str = ".axagent/shadow";
    /// 单个文件 diff 最大大小(字节,1MB)
    pub const DIFF_FILE_SIZE_LIMIT: usize = 1024 * 1024;
    /// 影子目录最大文件数(防止失控)
    pub const SHADOW_MAX_FILES: usize = 1000;
}

/// 3.2 P2:长期记忆文件级四类分目录常量
///
/// 对齐 Claude Code MEMORY.md 模型,在 `.axagent/memory/` 下按四类分目录:
/// - `user/`       — 用户偏好/信息(技术栈、沟通风格、工作习惯)
/// - `feedback/`   — 用户反馈(显式喜好/排斥、纠正记录)
/// - `project/`    — 项目相关(架构决策、约定、命令)
/// - `reference/`  — 参考资料(外部链接、文档索引)
///
/// `.axagent/MEMORY.md` 为索引文件(200 行硬限制),始终加载,
/// 索引四类主题文件的相对路径与一句话摘要。
pub mod memory {
    /// 记忆根目录(相对于项目根)
    pub const MEMORY_DIR: &str = ".axagent/memory";
    /// 索引文件路径(相对于项目根,始终加载,200 行硬限制)
    pub const MEMORY_INDEX: &str = ".axagent/MEMORY.md";
    /// 索引文件最大行数(硬限制)
    pub const MEMORY_INDEX_MAX_LINES: usize = 200;
    /// 文件级检索时选取的最相关文件数上限
    pub const MEMORY_RELEVANT_FILES_LIMIT: usize = 5;
    /// 单个记忆主题文件大小上限(字节,256KB)
    pub const MEMORY_FILE_SIZE_LIMIT: usize = 256 * 1024;

    /// 用户偏好/信息子目录
    pub const USER_DIR: &str = "user";
    /// 用户反馈子目录
    pub const FEEDBACK_DIR: &str = "feedback";
    /// 项目相关子目录
    pub const PROJECT_DIR: &str = "project";
    /// 参考资料子目录
    pub const REFERENCE_DIR: &str = "reference";

    /// 四类分目录列表(供扫描器遍历)
    pub const ALL_DIRS: &[&str] = &[USER_DIR, FEEDBACK_DIR, PROJECT_DIR, REFERENCE_DIR];
}

/// 提供商类型/注册表 key
pub mod provider {
    pub const OPENAI: &str = "openai";
    pub const OPENAI_RESPONSES: &str = "openai_responses";
    pub const ANTHROPIC: &str = "anthropic";
    pub const GEMINI: &str = "gemini";
    pub const OLLAMA: &str = "ollama";
    pub const OPENCLAW: &str = "openclaw";
    pub const HERMES: &str = "hermes";
}

/// 默认 API 端点
pub mod default_url {
    pub const OPENAI_BASE: &str = "https://api.openai.com/v1";
    pub const ANTHROPIC_BASE: &str = "https://api.anthropic.com/v1";
    pub const GEMINI_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
    pub const OLLAMA_HOST: &str = "http://localhost:11434";
    pub const OPENCLAW_HOST: &str = "http://localhost:8100";
    pub const HERMES_HOST: &str = "http://localhost:8642";
    pub const REPLICATE_API: &str = "https://api.replicate.com/v1/predictions";
    pub const SKILLS_HUB_API: &str = "https://api.agentskills.io";
    pub const HONCHO_API: &str = "https://api.honcho.ai";
    pub const MEM0_API: &str = "https://api.mem0.ai";
    pub const NPM_REGISTRY: &str = "https://registry.npmjs.org";
    pub const OTEL_ENDPOINT: &str = "http://localhost:4318";
}

/// 平台名称
pub mod platform {
    pub const TELEGRAM: &str = "telegram";
    pub const DISCORD: &str = "discord";
    pub const SLACK: &str = "slack";
    pub const WECHAT: &str = "wechat";
    pub const FEISHU: &str = "feishu";
    pub const DINGTALK: &str = "dingtalk";
    pub const WHATSAPP: &str = "whatsapp";
    pub const QQ: &str = "qq";
}

/// 默认语言
pub mod locale {
    pub const ZH_CN: &str = "zh-CN";
    pub const EN_US: &str = "en-US";
}

/// Android 平台消息
pub mod android_msg {
    pub const BROWSER_NOT_AVAILABLE: &str = "Browser automation is not available on Android";
    pub const COMPUTER_CONTROL_NOT_AVAILABLE: &str = "Computer control is not available on Android";
    pub const UI_AUTOMATION_NOT_SUPPORTED: &str = "UI automation is not supported on Android";
    pub const GIT_TOOLS_NOT_AVAILABLE: &str = "Git tools are not available on Android";
    pub const SANDBOX_NOT_AVAILABLE: &str = "Sandbox execution is not available on Android";
}

/// 窗口配置默认值
pub mod window {
    pub const MIN_WIDTH: f64 = 640.0;
    pub const MIN_HEIGHT: f64 = 480.0;
    pub const MARGIN: f64 = 100.0;
}

/// 超时/间隔默认值（秒）
pub mod timing {
    pub const AUTO_BACKUP_INTERVAL_HOURS: u64 = 24;
    pub const MEMORY_MAINTENANCE_SECS: u64 = 7200;
    pub const INSIGHT_INTERVAL_SECS: u64 = 600;
    pub const PATTERN_LEARN_SECS: u64 = 900;
    pub const CROSS_SESSION_LEARN_SECS: u64 = 1800;
    pub const RL_INTERVAL_SECS: u64 = 1200;
    pub const BATCH_PROCESS_SECS: u64 = 3600;
    pub const USER_PROFILE_PERSIST_SECS: u64 = 600;
    pub const SKILL_EVOLUTION_SECS: u64 = 2700;
    pub const MEMORY_DECAY_SECS: u64 = 3600;
    pub const AUTO_TOOL_OBSERVE_SECS: u64 = 3600;
    pub const TEXT_GRAD_ANALYSIS_SECS: u64 = 7200;
    pub const CRON_POLL_SECS: u64 = 30;
    pub const TRAJECTORY_CLEANUP_SECS: u64 = 86400;
    pub const SHUTDOWN_TIMEOUT_SECS: u64 = 5;
}

/// 嵌入相关常量
pub mod embed {
    pub const BATCH_SIZE: usize = 256;
    pub const MAX_RETRIES: u32 = 3;
    pub const RETRY_BASE_DELAY_MS: u64 = 500;
    pub const RAG_CACHE_TTL_SECS: u64 = 30;
}

/// Agent 执行默认迭代上限。各层（coordinator / agent_runtime / conversation_runtime）
/// 引用此常量以避免三层默认值不一致。coordinator 层可在 AgentConfig 中覆写。
pub const DEFAULT_MAX_ITERATIONS: usize = 50;

/// 认知编排执行链共享常量（护照投影 ↔ agent 工具 ↔ 循环终止辅助共用，禁止散落字面量）
pub mod capability_chain {
    /// RunWorkflow agent 工具的注册名。
    ///
    /// 护照 tool_ref（`workflow_types::WorkflowTemplatePassportParams`）与
    /// tools crate 的注册名（`tools::tools::run_workflow::RunWorkflowTool`）
    /// 必须共用此常量，否则「看得到调不动」。
    pub const RUN_WORKFLOW_TOOL: &str = "RunWorkflow";

    /// 检索类工具「未命中」输出的机器可读标记。
    ///
    /// DiscoverSkills / CapabilityBrowse 未命中时写入输出文本，
    /// runtime-core 的循环终止辅助凭此计数（连续 N 次未命中 → 注入停止检索提示），
    /// 避免跨 crate 文本匹配魔法字符串。
    pub const SEARCH_MISS_MARKER: &str = "[capability-search-miss]";
}

/// 智能路由层级
pub mod routing_tier {
    pub const BUDGET: &str = "budget";
    pub const BALANCED: &str = "balanced";
    pub const PREMIUM: &str = "premium";
}

/// 集合名前缀
pub mod collection_prefix {
    pub const KNOWLEDGE_BASE: &str = "kb";
    pub const MEMORY: &str = "mem";
    pub const WIKI: &str = "wiki";
}

/// 错误类别标签
pub mod error_category {
    pub const FACT: &str = "fact";
    pub const PREFERENCE: &str = "preference";
    pub const PROCEDURE: &str = "procedure";
    pub const CONTEXT: &str = "context";
}

/// 全局契约式约束文本（通用 prompt 契约的四模块模板）。
pub mod general_contract {
    pub const TEXT: &str = "\
# 交付物规范
- 明确输出格式：按任务类型决定输出结构（代码/报告/分析/计划），不允许无结构漫谈
- 输出必须覆盖任务中所有明确提出的要求点，不得遗漏
- 多步骤任务 must provide step-by-step logs: step number, operation type, status, output

# 禁区
- 不可编造或猜测数据——不确定的信息 must be marked as \"待确认\" or \"推测\"
- 不可跳过验证环节直接交付——each deliverable must be self-verified before output
- 不可引入与当前任务无关的额外变更或抽象
- 不可在未确认的情况下覆盖或删除现有功能
- 不可输出伪代码、占位符或\"略\"——must output complete executable content

# 证据规则
- 每个关键数据点/结论 must have a source (URL, file path, tool name)
- 统计数据 must include data caliber (time range, scope, unit)
- 代码变更 must map to specific requirements in the task description
- 多个来源数据冲突时，must present side-by-side with noted differences

# 自验环节（pre-output checklist）
Before output, verify each item:
- [ ] 是否覆盖了任务中的所有要求点？
- [ ] 是否有未经标注的推测或假设？
- [ ] 关键数据是否都有来源标注？
- [ ] 是否已完成所有步骤（无遗漏）？
- [ ] 输出格式是否满足任务要求？";
}
