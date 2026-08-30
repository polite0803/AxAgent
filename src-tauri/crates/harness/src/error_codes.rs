// SPDX-License-Identifier: AGPL-3.0-only

//! crate 层错误码定义
//!
//! 镜像 `commands/error_code.rs` 的错误码，使 crate 层可以使用相同错误码字符串。
//! 这些常量与命令层的值必须保持一致，但不依赖 commands 层。
//!
//! 命名规范: {CATEGORY}_{SHORT_NAME}

/// 会话/对话相关错误码
pub mod conversation {
    pub const NOT_WORKFLOW: &str = "CONVERSATION_NOT_WORKFLOW";
    pub const ALREADY_ARCHIVED: &str = "CONVERSATION_ALREADY_ARCHIVED";
    pub const NOT_FOUND: &str = "CONVERSATION_NOT_FOUND";
    pub const DELETE_FAILED: &str = "CONVERSATION_DELETE_FAILED";
    pub const CREATE_FAILED: &str = "CONVERSATION_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "CONVERSATION_UPDATE_FAILED";
    pub const LIST_FAILED: &str = "CONVERSATION_LIST_FAILED";
    pub const COMPRESS_FAILED: &str = "CONVERSATION_COMPRESS_FAILED";
    pub const TITLE_FAILED: &str = "CONVERSATION_TITLE_FAILED";
    pub const LOAD_MESSAGES_FAILED: &str = "CONVERSATION_LOAD_MESSAGES_FAILED";
    pub const MESSAGE_CREATE_FAILED: &str = "CONVERSATION_MESSAGE_CREATE_FAILED";
    pub const MESSAGE_DELETE_FAILED: &str = "CONVERSATION_MESSAGE_DELETE_FAILED";
    pub const TOOL_LOOP_EXCEEDED: &str = "CONVERSATION_TOOL_LOOP_EXCEEDED";
    pub const WEB_SEARCH_PARAM_MISSING: &str = "CONVERSATION_WEB_SEARCH_PARAM_MISSING";
    pub const ARCHIVE_FAILED: &str = "CONVERSATION_ARCHIVE_FAILED";
}

/// 工具执行相关错误码
pub mod tool {
    pub const NOT_FOUND: &str = "TOOL_NOT_FOUND";
    pub const PARAM_REQUIRED: &str = "TOOL_PARAM_REQUIRED";
    pub const EXECUTION_TIMEOUT: &str = "TOOL_EXECUTION_TIMEOUT";
    pub const EXECUTION_ERROR: &str = "TOOL_EXECUTION_ERROR";
    pub const STDIO_NO_COMMAND: &str = "TOOL_STDIO_NO_COMMAND";
    pub const HTTP_NO_ENDPOINT: &str = "TOOL_HTTP_NO_ENDPOINT";
    pub const SSE_NO_ENDPOINT: &str = "TOOL_SSE_NO_ENDPOINT";
    pub const TRANSPORT_UNSUPPORTED: &str = "TOOL_TRANSPORT_UNSUPPORTED";
    /// 工具重复注册（运行时动态注册时与已有工具同名）
    pub const REGISTRATION_DUPLICATE: &str = "TOOL_REGISTRATION_DUPLICATE";
}

/// MCP 服务器相关错误码
pub mod mcp {
    pub const SERVER_NOT_ENABLED: &str = "MCP_SERVER_NOT_ENABLED";
    pub const CONNECT_FAILED: &str = "MCP_CONNECT_FAILED";
    pub const TRANSPORT_UNSUPPORTED: &str = "MCP_TRANSPORT_UNSUPPORTED";
    pub const TIMEOUT: &str = "MCP_TIMEOUT";
    pub const TOOL_DISCOVERY_TIMEOUT: &str = "MCP_TOOL_DISCOVERY_TIMEOUT";
    pub const SERVER_CREATE_FAILED: &str = "MCP_SERVER_CREATE_FAILED";
    pub const SERVER_UPDATE_FAILED: &str = "MCP_SERVER_UPDATE_FAILED";
    pub const SERVER_DELETE_FAILED: &str = "MCP_SERVER_DELETE_FAILED";
    pub const SERVER_LIST_FAILED: &str = "MCP_SERVER_LIST_FAILED";
    pub const SERVER_TEST_FAILED: &str = "MCP_SERVER_TEST_FAILED";
    pub const SERVER_CONFIG_FAILED: &str = "MCP_SERVER_CONFIG_FAILED";
}

/// 浏览器相关错误码
pub mod browser {
    pub const NOT_INITIALIZED: &str = "BROWSER_NOT_INITIALIZED";
    pub const ACTION_FAILED: &str = "BROWSER_ACTION_FAILED";
}

/// 存储/文件相关错误码
pub mod storage {
    pub const PATH_NOT_ABSOLUTE: &str = "STORAGE_PATH_NOT_ABSOLUTE";
    pub const CREATE_DIR_FAILED: &str = "STORAGE_CREATE_DIR_FAILED";
    pub const READ_DIR_FAILED: &str = "STORAGE_READ_DIR_FAILED";
    pub const READ_FILE_FAILED: &str = "STORAGE_READ_FILE_FAILED";
    pub const WRITE_FILE_FAILED: &str = "STORAGE_WRITE_FILE_FAILED";
    pub const FILE_TOO_LARGE: &str = "STORAGE_FILE_TOO_LARGE";
}

/// 技能相关错误码
pub mod skill {
    pub const HOME_DIR_FAILED: &str = "SKILL_HOME_DIR_FAILED";
    pub const MANIFEST_PARSE_FAILED: &str = "SKILL_MANIFEST_PARSE_FAILED";
    pub const DEPENDENCY_NOT_FOUND: &str = "SKILL_DEPENDENCY_NOT_FOUND";
    pub const SERIALIZE_FAILED: &str = "SKILL_SERIALIZE_FAILED";
    pub const CONTENT_EMPTY: &str = "SKILL_CONTENT_EMPTY";
    pub const INSTALL_FAILED: &str = "SKILL_INSTALL_FAILED";
    pub const UNINSTALL_FAILED: &str = "SKILL_UNINSTALL_FAILED";
    pub const UPDATE_FAILED: &str = "SKILL_UPDATE_FAILED";
    pub const LOAD_FAILED: &str = "SKILL_LOAD_FAILED";
    pub const SEARCH_FAILED: &str = "SKILL_SEARCH_FAILED";
    pub const GIT_CLONE_FAILED: &str = "SKILL_GIT_CLONE_FAILED";
    pub const ALREADY_EXISTS: &str = "SKILL_ALREADY_EXISTS";
    pub const NOT_FOUND: &str = "SKILL_NOT_FOUND";
    pub const GROUP_NOT_FOUND: &str = "SKILL_GROUP_NOT_FOUND";
    pub const DIR_NOT_FOUND: &str = "SKILL_DIR_NOT_FOUND";
    pub const INVALID_GITHUB_URL: &str = "SKILL_INVALID_GITHUB_URL";
    pub const SOURCE_NOT_FOUND: &str = "SKILL_SOURCE_NOT_FOUND";
}

/// 专家相关错误码
pub mod expert {
    pub const READ_DIR_FAILED: &str = "EXPERT_READ_DIR_FAILED";
    pub const READ_ENTRY_FAILED: &str = "EXPERT_READ_ENTRY_FAILED";
    pub const READ_FILE_FAILED: &str = "EXPERT_READ_FILE_FAILED";
    pub const SAVE_FAILED: &str = "EXPERT_SAVE_FAILED";
    pub const DELETE_FAILED: &str = "EXPERT_DELETE_FAILED";
    pub const UPDATE_FAILED: &str = "EXPERT_UPDATE_FAILED";
    pub const QUERY_FAILED: &str = "EXPERT_QUERY_FAILED";
    pub const LOAD_SETTINGS_FAILED: &str = "EXPERT_LOAD_SETTINGS_FAILED";
    pub const KEY_DECRYPT_FAILED: &str = "EXPERT_KEY_DECRYPT_FAILED";
    pub const NO_ACTIVE_KEY: &str = "EXPERT_NO_ACTIVE_KEY";
    pub const LLM_CALL_FAILED: &str = "EXPERT_LLM_CALL_FAILED";
    pub const JSON_PARSE_FAILED: &str = "EXPERT_JSON_PARSE_FAILED";
    pub const VENDOR_NOT_FOUND: &str = "EXPERT_VENDOR_NOT_FOUND";
    pub const PATH_NOT_DIR: &str = "EXPERT_PATH_NOT_DIR";
    pub const NOT_FOUND: &str = "EXPERT_NOT_FOUND";
}

/// Agent 相关错误码
pub mod agent {
    pub const RUNNING: &str = "AGENT_RUNNING";
    pub const NOT_RUNNING: &str = "AGENT_NOT_RUNNING";
    pub const NOT_PAUSED: &str = "AGENT_NOT_PAUSED";
    pub const WORKFLOW_NOT_FOUND: &str = "AGENT_WORKFLOW_NOT_FOUND";
    pub const NOT_FOUND: &str = "AGENT_NOT_FOUND";
    pub const PROVIDER_LOAD_FAILED: &str = "AGENT_PROVIDER_LOAD_FAILED";
    pub const STREAM_ERROR: &str = "AGENT_STREAM_ERROR";
    pub const MAX_TURNS_EXCEEDED: &str = "AGENT_MAX_TURNS_EXCEEDED";
    pub const CANCEL_FAILED: &str = "AGENT_CANCEL_FAILED";
    pub const EXECUTION_ABORTED: &str = "AGENT_EXECUTION_ABORTED";
    pub const INVALID_STATE: &str = "AGENT_INVALID_STATE";
    pub const SKILL_MISSING: &str = "AGENT_SKILL_MISSING";
    pub const WORKSPACE_URI_INVALID: &str = "AGENT_WORKSPACE_URI_INVALID";
}

/// 后台任务相关错误码
pub mod task {
    pub const DANGEROUS_COMMAND: &str = "TASK_DANGEROUS_COMMAND";
    pub const NOT_FOUND: &str = "TASK_NOT_FOUND";
    pub const UPDATE_FAILED: &str = "TASK_UPDATE_FAILED";
    pub const START_FAILED: &str = "TASK_START_FAILED";
    pub const OUTPUT_APPEND_FAILED: &str = "TASK_OUTPUT_APPEND_FAILED";
}

/// 提供商相关错误码
pub mod provider {
    pub const MODEL_LIST_TIMEOUT: &str = "PROVIDER_MODEL_LIST_TIMEOUT";
    pub const CREATE_FAILED: &str = "PROVIDER_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "PROVIDER_UPDATE_FAILED";
    pub const DELETE_FAILED: &str = "PROVIDER_DELETE_FAILED";
    pub const KEY_ADD_FAILED: &str = "PROVIDER_KEY_ADD_FAILED";
    pub const KEY_DECRYPT_FAILED: &str = "PROVIDER_KEY_DECRYPT_FAILED";
    pub const FETCH_MODELS_FAILED: &str = "PROVIDER_FETCH_MODELS_FAILED";
    pub const TEST_FAILED: &str = "PROVIDER_TEST_FAILED";
    pub const NO_ACTIVE_KEY: &str = "PROVIDER_NO_ACTIVE_KEY";
    pub const ADAPTER_NOT_FOUND: &str = "PROVIDER_ADAPTER_NOT_FOUND";
    pub const MODEL_NOT_FOUND: &str = "PROVIDER_MODEL_NOT_FOUND";
}

/// 搜索相关错误码
pub mod search {
    pub const ENDPOINT_NOT_CONFIGURED: &str = "SEARCH_ENDPOINT_NOT_CONFIGURED";
    pub const PROVIDER_NOT_CONFIGURED: &str = "SEARCH_PROVIDER_NOT_CONFIGURED";
    pub const PROVIDER_NOT_FOUND: &str = "SEARCH_PROVIDER_NOT_FOUND";
    pub const SEARCH_FAILED: &str = "SEARCH_FAILED";
}

/// 备份相关错误码
pub mod backup {
    pub const FORMAT_UNSUPPORTED: &str = "BACKUP_FORMAT_UNSUPPORTED";
    pub const CREATE_FAILED: &str = "BACKUP_CREATE_FAILED";
    pub const RESTORE_FAILED: &str = "BACKUP_RESTORE_FAILED";
    pub const LIST_FAILED: &str = "BACKUP_LIST_FAILED";
    pub const DELETE_FAILED: &str = "BACKUP_DELETE_FAILED";
    pub const PATH_INVALID: &str = "BACKUP_PATH_INVALID";
}

/// 网关相关错误码
pub mod gateway {
    pub const SSL_NO_CERT: &str = "GATEWAY_SSL_NO_CERT";
    pub const SSL_NO_KEY: &str = "GATEWAY_SSL_NO_KEY";
    pub const HTTP_UNAVAILABLE: &str = "GATEWAY_HTTP_UNAVAILABLE";
    pub const ALREADY_RUNNING: &str = "GATEWAY_ALREADY_RUNNING";
    pub const QUICK_CONNECT_INVALID: &str = "GATEWAY_QUICK_CONNECT_INVALID";
    pub const TEMPLATE_NOT_FOUND: &str = "GATEWAY_TEMPLATE_NOT_FOUND";
    pub const LINK_NOT_FOUND: &str = "GATEWAY_LINK_NOT_FOUND";
}

/// 平台集成相关错误码
pub mod platforms {
    pub const TELEGRAM_NOT_ENABLED: &str = "PLATFORM_TELEGRAM_NOT_ENABLED";
    pub const DISCORD_NOT_ENABLED: &str = "PLATFORM_DISCORD_NOT_ENABLED";
    pub const API_SERVER_NOT_ENABLED: &str = "PLATFORM_API_SERVER_NOT_ENABLED";
    pub const UNSUPPORTED_PLATFORM: &str = "PLATFORM_UNSUPPORTED";
    pub const ADAPTER_NOT_FOUND: &str = "PLATFORM_ADAPTER_NOT_FOUND";
    pub const SEND_FAILED: &str = "PLATFORM_SEND_FAILED";
}

/// 流式响应相关错误码
pub mod stream {
    pub const EMPTY_RESPONSE: &str = "STREAM_EMPTY_RESPONSE";
}

/// 工作流相关错误码
pub mod workflow {
    pub const NODE_NOT_FOUND: &str = "WORKFLOW_NODE_NOT_FOUND";
    pub const VERSION_NOT_FOUND: &str = "WORKFLOW_VERSION_NOT_FOUND";
    pub const INVALID_JSON: &str = "WORKFLOW_INVALID_JSON";
    pub const NOT_FOUND: &str = "WORKFLOW_NOT_FOUND";
    pub const PLAN_NOT_FOUND: &str = "WORKFLOW_PLAN_NOT_FOUND";
}

/// 终端相关错误码
pub mod terminal {
    pub const GIT_BRANCH_FAILED: &str = "TERMINAL_GIT_BRANCH_FAILED";
    pub const SESSION_NOT_FOUND: &str = "TERMINAL_SESSION_NOT_FOUND";
    pub const SSH_FAILED: &str = "TERMINAL_SSH_FAILED";
    pub const DOCKER_FAILED: &str = "TERMINAL_DOCKER_FAILED";
}

/// 记忆相关错误码
pub mod memory {
    pub const CREATE_FAILED: &str = "MEMORY_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "MEMORY_UPDATE_FAILED";
    pub const DELETE_FAILED: &str = "MEMORY_DELETE_FAILED";
    pub const LIST_FAILED: &str = "MEMORY_LIST_FAILED";
    pub const EXTRACT_FAILED: &str = "MEMORY_EXTRACT_FAILED";
    pub const SEARCH_FAILED: &str = "MEMORY_SEARCH_FAILED";
    pub const NOT_FOUND: &str = "MEMORY_NOT_FOUND";
    pub const EMBED_FAILED: &str = "MEMORY_EMBED_FAILED";
    pub const CONSOLIDATE_FAILED: &str = "MEMORY_CONSOLIDATE_FAILED";
    pub const NO_NAMESPACE: &str = "MEMORY_NO_NAMESPACE";
    pub const NOT_ENOUGH_MESSAGES: &str = "MEMORY_NOT_ENOUGH_MESSAGES";
    /// 记忆合并失败（条目数不足）
    pub const CONSOLIDATION_INSUFFICIENT: &str = "MEMORY_CONSOLIDATION_INSUFFICIENT";
    /// 记忆命名空间未找到
    pub const NAMESPACE_NOT_FOUND: &str = "MEMORY_NAMESPACE_NOT_FOUND";
    /// 记忆索引失败
    pub const INDEX_FAILED: &str = "MEMORY_INDEX_FAILED";
}

/// 知识库/Wiki 相关错误码
pub mod wiki {
    pub const NO_EMBEDDING_PROVIDER: &str = "WIKI_NO_EMBEDDING_PROVIDER";
    pub const PATH_NOT_DIR: &str = "WIKI_PATH_NOT_DIR";
    /// Wiki 未找到
    pub const NOT_FOUND: &str = "WIKI_NOT_FOUND";
    /// Wiki 笔记未找到
    pub const NOTE_NOT_FOUND: &str = "WIKI_NOTE_NOT_FOUND";
    /// 创建 Wiki 笔记失败
    pub const CREATE_NOTE_FAILED: &str = "WIKI_CREATE_NOTE_FAILED";
    /// 更新 Wiki 笔记失败
    pub const UPDATE_NOTE_FAILED: &str = "WIKI_UPDATE_NOTE_FAILED";
    /// 删除 Wiki 笔记失败
    pub const DELETE_NOTE_FAILED: &str = "WIKI_DELETE_NOTE_FAILED";
    /// Wiki 索引重建失败
    pub const REBUILD_FAILED: &str = "WIKI_REBUILD_FAILED";
    /// Wiki 搜索失败
    pub const SEARCH_FAILED: &str = "WIKI_SEARCH_FAILED";
    /// 删除 Wiki 失败
    pub const DELETE_FAILED: &str = "WIKI_DELETE_FAILED";
    /// Wiki 导入失败
    pub const IMPORT_FAILED: &str = "WIKI_IMPORT_FAILED";
    /// Wiki 导出失败
    pub const EXPORT_FAILED: &str = "WIKI_EXPORT_FAILED";
}

/// 知识库（RAG）相关错误码
pub mod knowledge {
    /// 知识库未找到
    pub const NOT_FOUND: &str = "KNOWLEDGE_NOT_FOUND";
    /// 知识库文档未找到
    pub const DOCUMENT_NOT_FOUND: &str = "KNOWLEDGE_DOCUMENT_NOT_FOUND";
    /// 未配置嵌入模型提供商
    pub const NO_EMBEDDING_PROVIDER: &str = "KNOWLEDGE_NO_EMBEDDING_PROVIDER";
    /// 索引重建失败
    pub const REBUILD_FAILED: &str = "KNOWLEDGE_REBUILD_FAILED";
    /// 索引清理失败
    pub const CLEAR_FAILED: &str = "KNOWLEDGE_CLEAR_FAILED";
    /// 文档索引失败
    pub const INDEX_FAILED: &str = "KNOWLEDGE_INDEX_FAILED";
    /// 目录导入失败
    pub const IMPORT_DIR_FAILED: &str = "KNOWLEDGE_IMPORT_DIR_FAILED";
    /// 添加文档失败
    pub const ADD_DOCUMENT_FAILED: &str = "KNOWLEDGE_ADD_DOCUMENT_FAILED";
    /// 删除文档失败
    pub const DELETE_DOCUMENT_FAILED: &str = "KNOWLEDGE_DELETE_DOCUMENT_FAILED";
    /// 删除知识库失败
    pub const DELETE_FAILED: &str = "KNOWLEDGE_DELETE_FAILED";
    /// 创建知识库失败
    pub const CREATE_FAILED: &str = "KNOWLEDGE_CREATE_FAILED";
    /// 更新知识库失败
    pub const UPDATE_FAILED: &str = "KNOWLEDGE_UPDATE_FAILED";
    /// 搜索知识库失败
    pub const SEARCH_FAILED: &str = "KNOWLEDGE_SEARCH_FAILED";
    /// 向量存储操作失败
    pub const VECTOR_STORE_FAILED: &str = "KNOWLEDGE_VECTOR_STORE_FAILED";
    /// 实体抽取失败
    pub const EXTRACT_ENTITIES_FAILED: &str = "KNOWLEDGE_EXTRACT_ENTITIES_FAILED";
}

/// 向量存储相关错误码
pub mod vector {
    /// 向量存储操作失败
    pub const STORE_FAILED: &str = "VECTOR_STORE_FAILED";
    /// 向量嵌入生成失败
    pub const EMBEDDING_FAILED: &str = "VECTOR_EMBEDDING_FAILED";
    /// 向量搜索失败
    pub const SEARCH_FAILED: &str = "VECTOR_SEARCH_FAILED";
}

/// Multi-Agent 委派相关错误码
pub mod multi_agent {
    /// 角色未找到
    pub const ROLE_NOT_FOUND: &str = "MULTI_AGENT_ROLE_NOT_FOUND";
    /// 委派失败（LLM 调用内部异常）
    pub const DELEGATE_FAILED: &str = "MULTI_AGENT_DELEGATE_FAILED";
    /// 无效的角色名称（非 analyst/implementer/reviewer）
    pub const INVALID_ROLE: &str = "MULTI_AGENT_INVALID_ROLE";
    /// 提供商未找到
    pub const PROVIDER_NOT_FOUND: &str = "MULTI_AGENT_PROVIDER_NOT_FOUND";
}

/// 安全性相关错误码
pub mod security {
    pub const PATH_TRAVERSAL: &str = "SECURITY_PATH_TRAVERSAL";
    pub const ACCESS_DENIED: &str = "SECURITY_ACCESS_DENIED";
    pub const SSRF_BLOCKED: &str = "SECURITY_SSRF_BLOCKED";
    pub const SSRF_PRIVATE_NETWORK: &str = "SECURITY_SSRF_PRIVATE_NETWORK";
    pub const RATE_LIMIT_EXCEEDED: &str = "SECURITY_RATE_LIMIT_EXCEEDED";
    pub const CONTENT_BLOCKED: &str = "SECURITY_CONTENT_BLOCKED";
    pub const CONTENT_PII_DETECTED: &str = "SECURITY_CONTENT_PII_DETECTED";
    pub const TOOL_ACCESS_DENIED: &str = "SECURITY_TOOL_ACCESS_DENIED";
    pub const CIRCUIT_OPEN: &str = "SECURITY_CIRCUIT_OPEN";
}

/// 云存储相关错误码
pub mod cloud {
    pub const NOT_CLOUD_URI: &str = "CLOUD_NOT_CLOUD_URI";
    pub const UNKNOWN_CONFLICT: &str = "CLOUD_UNKNOWN_CONFLICT";
    pub const UNKNOWN_STRATEGY: &str = "CLOUD_UNKNOWN_STRATEGY";
    pub const UNKNOWN_STORAGE: &str = "CLOUD_UNKNOWN_STORAGE";
    pub const SYNC_FAILED: &str = "CLOUD_SYNC_FAILED";
}

/// 桌面相关错误码
pub mod desktop {
    pub const CONNECTION_TIMEOUT: &str = "DESKTOP_CONNECTION_TIMEOUT";
    pub const NATIVE_NOTIFICATION_FAILED: &str = "DESKTOP_NATIVE_NOTIFICATION_FAILED";
}

/// 语音会话相关错误码（realtime WebSocket 通道）
///
/// 通过 `RealtimeServerMessage::Error { code, params, .. }` 回传前端，
/// 前端按 `t("error.${code}", params)` 翻译。
/// 与 `commands/error_code.rs::voice` 模块及 locales/*.json 的 `error` 段对齐。
pub mod voice {
    /// 票据无效/过期/已被使用（401 鉴权失败）
    pub const TICKET_INVALID: &str = "VOICE_TICKET_INVALID";
    /// 语音模型未找到或不支持
    pub const MODEL_NOT_FOUND: &str = "VOICE_MODEL_NOT_FOUND";
    /// 语音提供商未找到
    pub const PROVIDER_NOT_FOUND: &str = "VOICE_PROVIDER_NOT_FOUND";
    /// 提供商不支持语音能力（STT/TTS）
    pub const PROVIDER_NO_SPEECH: &str = "VOICE_PROVIDER_NO_SPEECH";
    /// 网关密钥解密失败
    pub const DECRYPT_KEY_FAILED: &str = "VOICE_DECRYPT_KEY_FAILED";
    /// 语音识别（STT）失败
    pub const STT_FAILED: &str = "VOICE_STT_FAILED";
    /// 语音合成（TTS）失败
    pub const TTS_FAILED: &str = "VOICE_TTS_FAILED";
    /// 无效的语音消息格式
    pub const INVALID_MESSAGE: &str = "VOICE_INVALID_MESSAGE";
    /// 未先发送 session.create 消息
    pub const SESSION_CREATE_REQUIRED: &str = "VOICE_SESSION_CREATE_REQUIRED";
}

/// LLM 调用相关错误码
pub mod llm {
    /// LLM 调用失败（含重试耗尽）
    pub const CALL_FAILED: &str = "LLM_CALL_FAILED";
}

/// 能力发现相关错误码
pub mod capability {
    /// 能力嵌入向量生成失败
    pub const EMBEDDING_FAILED: &str = "CAPABILITY_EMBEDDING_FAILED";
    /// 能力护照未找到
    pub const NOT_FOUND: &str = "CAPABILITY_NOT_FOUND";
}

/// 构造携带错误码的结构化错误字符串。
///
/// BE-I1 修复：crate 层函数（如 `execute_llm`、认知 RAR 检索、能力嵌入）返回
/// `Result<_, String>` 历史上是裸字符串，前端无法按 `error.${code}` 做 i18n。
/// 本函数把错误编码为前端 `translateBackendError` 可解析的 JSON
/// `{ code, category, detail, params }`，命中 `error.${code}` 翻译键，未命中回退 detail。
pub fn error_json(code: &str, detail: impl Into<String>) -> String {
    let detail = detail.into();
    serde_json::json!({
        "code": code,
        "category": "retryable",
        "detail": detail,
        "params": { "detail": detail }
    })
    .to_string()
}

/// 论文/文献相关错误码
pub mod paper {
    /// 论文概览未找到
    pub const OVERVIEW_NOT_FOUND: &str = "PAPER_OVERVIEW_NOT_FOUND";
}

/// 阅读列表相关错误码
pub mod reading_list {
    /// 阅读列表未找到
    pub const NOT_FOUND: &str = "READING_LIST_NOT_FOUND";
    /// 阅读条目未找到
    pub const ITEM_NOT_FOUND: &str = "READING_LIST_ITEM_NOT_FOUND";
}

/// 认知路由相关错误码
pub mod cognitive {
    /// 认知路由执行失败
    pub const ROUTE_FAILED: &str = "COGNITIVE_ROUTE_FAILED";
    /// 认知路由输入为空
    pub const EMPTY_INPUT: &str = "COGNITIVE_EMPTY_INPUT";
    /// 认知路由已熔断
    pub const CIRCUIT_BROKEN: &str = "COGNITIVE_CIRCUIT_BROKEN";
    /// 认知路由未找到可用能力
    pub const NO_CANDIDATE: &str = "COGNITIVE_NO_CANDIDATE";
    /// 认知路由执行模式无效
    pub const EXECUTION_MODE_INVALID: &str = "COGNITIVE_EXECUTION_MODE_INVALID";
    /// 安全拦截：检测到注入/越狱，拒绝执行并记录安全日志
    pub const PROMPT_REJECTED: &str = "COGNITIVE_PROMPT_REJECTED";
    /// RAR 向量检索失败（BE-I1）
    pub const RAR_RETRIEVE_FAILED: &str = "COGNITIVE_RAR_RETRIEVE_FAILED";
    /// 能力补齐提议待用户确认（等待用户同意/拒绝弹窗）
    pub const GAP_PROPOSAL_PENDING: &str = "COGNITIVE_GAP_PROPOSAL_PENDING";
    /// 能力补齐提议已应用（用户同意后完成补齐，请重新发送请求）
    pub const GAP_PROPOSAL_APPLIED: &str = "COGNITIVE_GAP_PROPOSAL_APPLIED";
    /// 能力补齐提议被用户拒绝（保持原拒绝/无候选行为）
    pub const GAP_PROPOSAL_REJECTED: &str = "COGNITIVE_GAP_PROPOSAL_REJECTED";
    /// 工具链执行失败（P2：固定顺序执行器，步骤缺失/无工具引用/执行报错）
    pub const TOOLCHAIN_EXEC_FAILED: &str = "COGNITIVE_TOOLCHAIN_EXEC_FAILED";
}

/// Unity 改造相关错误码
///
/// P0 阶段：任务形态分类器（原则三标尺：上下文保留成本 × 安全隔离需求）。
/// 与前端 `error.UNITY_P0_CLASSIFIER_FAILED` 翻译键对齐。
pub mod unity {
    /// 任务形态分类失败（分类器内部异常，回退到 HandleLocally 策略）
    pub const P0_CLASSIFIER_FAILED: &str = "UNITY_P0_CLASSIFIER_FAILED";
}
