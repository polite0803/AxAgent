//! crate 层错误码定义 — 镜像 commands/error_code.rs
//! crate 层不依赖 commands 层，故独立定义相同错误码字符串。

pub mod conversation {
    pub const NOT_WORKFLOW: &str = "CONVERSATION_NOT_WORKFLOW";
    pub const ALREADY_ARCHIVED: &str = "CONVERSATION_ALREADY_ARCHIVED";
    pub const NOT_FOUND: &str = "CONVERSATION_NOT_FOUND";
    pub const DELETE_FAILED: &str = "CONVERSATION_DELETE_FAILED";
    pub const CREATE_FAILED: &str = "CONVERSATION_CREATE_FAILED";
    pub const LIST_FAILED: &str = "CONVERSATION_LIST_FAILED";
    pub const COMPRESS_FAILED: &str = "CONVERSATION_COMPRESS_FAILED";
    pub const TITLE_FAILED: &str = "CONVERSATION_TITLE_FAILED";
    pub const ARCHIVE_FAILED: &str = "CONVERSATION_ARCHIVE_FAILED";
    pub const TOOL_LOOP_EXCEEDED: &str = "CONVERSATION_TOOL_LOOP_EXCEEDED";
    pub const WEB_SEARCH_PARAM_MISSING: &str = "CONVERSATION_WEB_SEARCH_PARAM_MISSING";
    pub const MESSAGE_NOT_FOUND: &str = "CONVERSATION_MESSAGE_NOT_FOUND";
    pub const NO_USER_MESSAGE: &str = "CONVERSATION_NO_USER_MESSAGE";
    pub const SUMMARY_GENERATE_FAILED: &str = "CONVERSATION_SUMMARY_GENERATE_FAILED";
    pub const SUMMARY_NOT_FOUND: &str = "CONVERSATION_SUMMARY_NOT_FOUND";
}

pub mod tool {
    pub const NOT_FOUND: &str = "TOOL_NOT_FOUND";
    pub const PARAM_REQUIRED: &str = "TOOL_PARAM_REQUIRED";
    pub const EXECUTION_TIMEOUT: &str = "TOOL_EXECUTION_TIMEOUT";
    pub const EXECUTION_ERROR: &str = "TOOL_EXECUTION_ERROR";
}

pub mod mcp {
    pub const SERVER_NOT_ENABLED: &str = "MCP_SERVER_NOT_ENABLED";
    pub const CONNECT_FAILED: &str = "MCP_CONNECT_FAILED";
    pub const TIMEOUT: &str = "MCP_TIMEOUT";
}

pub mod browser {
    pub const NOT_INITIALIZED: &str = "BROWSER_NOT_INITIALIZED";
    pub const ACTION_FAILED: &str = "BROWSER_ACTION_FAILED";
}

pub mod storage {
    pub const PATH_NOT_ABSOLUTE: &str = "STORAGE_PATH_NOT_ABSOLUTE";
    pub const CREATE_DIR_FAILED: &str = "STORAGE_CREATE_DIR_FAILED";
    pub const READ_FILE_FAILED: &str = "STORAGE_READ_FILE_FAILED";
    pub const WRITE_FILE_FAILED: &str = "STORAGE_WRITE_FILE_FAILED";
    pub const FILE_TOO_LARGE: &str = "STORAGE_FILE_TOO_LARGE";
}

pub mod skill {
    pub const HOME_DIR_FAILED: &str = "SKILL_HOME_DIR_FAILED";
    pub const MANIFEST_PARSE_FAILED: &str = "SKILL_MANIFEST_PARSE_FAILED";
    pub const SERIALIZE_FAILED: &str = "SKILL_SERIALIZE_FAILED";
    pub const NOT_FOUND: &str = "SKILL_NOT_FOUND";
    pub const LOAD_FAILED: &str = "SKILL_LOAD_FAILED";
    pub const INSTALL_FAILED: &str = "SKILL_INSTALL_FAILED";
    pub const GIT_CLONE_FAILED: &str = "SKILL_GIT_CLONE_FAILED";
    pub const ALREADY_EXISTS: &str = "SKILL_ALREADY_EXISTS";
    pub const DIR_NOT_FOUND: &str = "SKILL_DIR_NOT_FOUND";
}

pub mod expert {
    pub const READ_DIR_FAILED: &str = "EXPERT_READ_DIR_FAILED";
    pub const SAVE_FAILED: &str = "EXPERT_SAVE_FAILED";
    pub const DELETE_FAILED: &str = "EXPERT_DELETE_FAILED";
    pub const QUERY_FAILED: &str = "EXPERT_QUERY_FAILED";
    pub const NOT_FOUND: &str = "EXPERT_NOT_FOUND";
}

pub mod agent {
    pub const RUNNING: &str = "AGENT_RUNNING";
    pub const NOT_RUNNING: &str = "AGENT_NOT_RUNNING";
    pub const NOT_FOUND: &str = "AGENT_NOT_FOUND";
    pub const INVALID_STATE: &str = "AGENT_INVALID_STATE";
    pub const PROVIDER_LOAD_FAILED: &str = "AGENT_PROVIDER_LOAD_FAILED";
    pub const STREAM_ERROR: &str = "AGENT_STREAM_ERROR";
    pub const MAX_TURNS_EXCEEDED: &str = "AGENT_MAX_TURNS_EXCEEDED";
    pub const CANCEL_FAILED: &str = "AGENT_CANCEL_FAILED";
    pub const SKILL_MISSING: &str = "AGENT_SKILL_MISSING";
}

pub mod provider {
    pub const MODEL_LIST_TIMEOUT: &str = "PROVIDER_MODEL_LIST_TIMEOUT";
    pub const ADAPTER_NOT_FOUND: &str = "PROVIDER_ADAPTER_NOT_FOUND";
    pub const NO_ACTIVE_KEY: &str = "PROVIDER_NO_ACTIVE_KEY";
    pub const CREATE_FAILED: &str = "PROVIDER_CREATE_FAILED";
    pub const DELETE_FAILED: &str = "PROVIDER_DELETE_FAILED";
    pub const KEY_DECRYPT_FAILED: &str = "PROVIDER_KEY_DECRYPT_FAILED";
    pub const FETCH_MODELS_FAILED: &str = "PROVIDER_FETCH_MODELS_FAILED";
}

pub mod search {
    pub const ENDPOINT_NOT_CONFIGURED: &str = "SEARCH_ENDPOINT_NOT_CONFIGURED";
    pub const PROVIDER_NOT_CONFIGURED: &str = "SEARCH_PROVIDER_NOT_CONFIGURED";
    pub const PROVIDER_NOT_FOUND: &str = "SEARCH_PROVIDER_NOT_FOUND";
    pub const SEARCH_FAILED: &str = "SEARCH_FAILED";
}

pub mod backup {
    pub const FORMAT_UNSUPPORTED: &str = "BACKUP_FORMAT_UNSUPPORTED";
    pub const CREATE_FAILED: &str = "BACKUP_CREATE_FAILED";
}

pub mod gateway {
    pub const SSL_NO_CERT: &str = "GATEWAY_SSL_NO_CERT";
    pub const SSL_NO_KEY: &str = "GATEWAY_SSL_NO_KEY";
    pub const ALREADY_RUNNING: &str = "GATEWAY_ALREADY_RUNNING";
    pub const QUICK_CONNECT_INVALID: &str = "GATEWAY_QUICK_CONNECT_INVALID";
    pub const TEMPLATE_NOT_FOUND: &str = "GATEWAY_TEMPLATE_NOT_FOUND";
    pub const LINK_NOT_FOUND: &str = "GATEWAY_LINK_NOT_FOUND";
}

pub mod terminal {
    pub const GIT_BRANCH_FAILED: &str = "TERMINAL_GIT_BRANCH_FAILED";
    pub const SESSION_NOT_FOUND: &str = "TERMINAL_SESSION_NOT_FOUND";
    pub const SSH_FAILED: &str = "TERMINAL_SSH_FAILED";
    pub const DOCKER_FAILED: &str = "TERMINAL_DOCKER_FAILED";
}

pub mod memory {
    pub const CREATE_FAILED: &str = "MEMORY_CREATE_FAILED";
    pub const NOT_FOUND: &str = "MEMORY_NOT_FOUND";
    pub const EMBED_FAILED: &str = "MEMORY_EMBED_FAILED";
    pub const CONSOLIDATE_FAILED: &str = "MEMORY_CONSOLIDATE_FAILED";
    pub const NO_NAMESPACE: &str = "MEMORY_NO_NAMESPACE";
}

pub mod wiki {
    pub const NO_EMBEDDING_PROVIDER: &str = "WIKI_NO_EMBEDDING_PROVIDER";
    pub const PATH_NOT_DIR: &str = "WIKI_PATH_NOT_DIR";
}

pub mod workflow {
    pub const NODE_NOT_FOUND: &str = "WORKFLOW_NODE_NOT_FOUND";
    pub const NOT_FOUND: &str = "WORKFLOW_NOT_FOUND";
    pub const PLAN_NOT_FOUND: &str = "WORKFLOW_PLAN_NOT_FOUND";
    pub const INVALID_JSON: &str = "WORKFLOW_INVALID_JSON";
}

pub mod cloud {
    pub const NOT_CLOUD_URI: &str = "CLOUD_NOT_CLOUD_URI";
    pub const UNKNOWN_CONFLICT: &str = "CLOUD_UNKNOWN_CONFLICT";
    pub const UNKNOWN_STRATEGY: &str = "CLOUD_UNKNOWN_STRATEGY";
    pub const UNKNOWN_STORAGE: &str = "CLOUD_UNKNOWN_STORAGE";
    pub const SYNC_FAILED: &str = "CLOUD_SYNC_FAILED";
}

pub mod security {
    pub const PATH_TRAVERSAL: &str = "SECURITY_PATH_TRAVERSAL";
    pub const ACCESS_DENIED: &str = "SECURITY_ACCESS_DENIED";
}

pub mod desktop {
    pub const CONNECTION_TIMEOUT: &str = "DESKTOP_CONNECTION_TIMEOUT";
    pub const NATIVE_NOTIFICATION_FAILED: &str = "DESKTOP_NATIVE_NOTIFICATION_FAILED";
}
