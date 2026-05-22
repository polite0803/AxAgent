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
pub mod platform {
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
}

/// 知识库/Wiki 相关错误码
pub mod wiki {
    pub const NO_EMBEDDING_PROVIDER: &str = "WIKI_NO_EMBEDDING_PROVIDER";
    pub const PATH_NOT_DIR: &str = "WIKI_PATH_NOT_DIR";
}

/// 安全性相关错误码
pub mod security {
    pub const PATH_TRAVERSAL: &str = "SECURITY_PATH_TRAVERSAL";
    pub const ACCESS_DENIED: &str = "SECURITY_ACCESS_DENIED";
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
