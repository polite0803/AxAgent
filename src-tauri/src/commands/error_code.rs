//! 集中式错误码定义
//!
//! 前端根据 error code 查询 i18n 翻译
//!
//! 命名规范: {CATEGORY}_{SHORT_NAME}
//! - CATEGORY: 会话(CONVERSATION), 工具(TOOL), MCP, 浏览器(BROWSER)等
//! - SHORT_NAME: 简短描述性名称,如 NOT_FOUND, TIMEOUT, FAILED等

/// 会话/对话相关错误码
pub mod conversation {
    /// 此会话不是工作流类型，请使用普通归档
    pub const NOT_WORKFLOW: &str = "CONVERSATION_NOT_WORKFLOW";
    /// 会话已归档，请勿重复操作
    pub const ALREADY_ARCHIVED: &str = "CONVERSATION_ALREADY_ARCHIVED";
    /// 会话未找到
    pub const NOT_FOUND: &str = "CONVERSATION_NOT_FOUND";
    /// 删除会话失败
    pub const DELETE_FAILED: &str = "CONVERSATION_DELETE_FAILED";
}

/// 工具执行相关错误码
pub mod tool {
    /// 工具未找到
    pub const NOT_FOUND: &str = "TOOL_NOT_FOUND";
    /// 工具缺少必需参数
    pub const PARAM_REQUIRED: &str = "TOOL_PARAM_REQUIRED";
    /// 工具执行超时
    pub const EXECUTION_TIMEOUT: &str = "TOOL_EXECUTION_TIMEOUT";
    /// 工具执行错误
    pub const EXECUTION_ERROR: &str = "TOOL_EXECUTION_ERROR";
    /// stdio工具未配置命令
    pub const STDIO_NO_COMMAND: &str = "TOOL_STDIO_NO_COMMAND";
    /// HTTP工具未配置端点
    pub const HTTP_NO_ENDPOINT: &str = "TOOL_HTTP_NO_ENDPOINT";
    /// SSE工具未配置端点
    pub const SSE_NO_ENDPOINT: &str = "TOOL_SSE_NO_ENDPOINT";
    /// 不支持的传输类型
    pub const TRANSPORT_UNSUPPORTED: &str = "TOOL_TRANSPORT_UNSUPPORTED";
}

/// MCP服务器相关错误码
pub mod mcp {
    /// MCP服务器未启用
    pub const SERVER_NOT_ENABLED: &str = "MCP_SERVER_NOT_ENABLED";
    /// MCP连接失败
    pub const CONNECT_FAILED: &str = "MCP_CONNECT_FAILED";
    /// 不支持的MCP传输类型
    pub const TRANSPORT_UNSUPPORTED: &str = "MCP_TRANSPORT_UNSUPPORTED";
    /// MCP连接超时
    pub const TIMEOUT: &str = "MCP_TIMEOUT";
    /// MCP工具发现超时
    pub const TOOL_DISCOVERY_TIMEOUT: &str = "MCP_TOOL_DISCOVERY_TIMEOUT";
}

/// 浏览器相关错误码
pub mod browser {
    /// 浏览器客户端未初始化
    pub const NOT_INITIALIZED: &str = "BROWSER_NOT_INITIALIZED";
    /// 浏览器操作失败
    pub const ACTION_FAILED: &str = "BROWSER_ACTION_FAILED";
}

/// 存储/文件相关错误码
pub mod storage {
    /// 路径必须是绝对路径
    pub const PATH_NOT_ABSOLUTE: &str = "STORAGE_PATH_NOT_ABSOLUTE";
    /// 创建目录失败
    pub const CREATE_DIR_FAILED: &str = "STORAGE_CREATE_DIR_FAILED";
    /// 读取目录失败
    pub const READ_DIR_FAILED: &str = "STORAGE_READ_DIR_FAILED";
    /// 读取文件失败
    pub const READ_FILE_FAILED: &str = "STORAGE_READ_FILE_FAILED";
    /// 写入文件失败
    pub const WRITE_FILE_FAILED: &str = "STORAGE_WRITE_FILE_FAILED";
}

/// 技能(Skill)相关错误码
pub mod skill {
    /// 无法确定用户主目录
    pub const HOME_DIR_FAILED: &str = "SKILL_HOME_DIR_FAILED";
    /// 解析skill-manifest.json失败
    pub const MANIFEST_PARSE_FAILED: &str = "SKILL_MANIFEST_PARSE_FAILED";
    /// 技能依赖未找到
    pub const DEPENDENCY_NOT_FOUND: &str = "SKILL_DEPENDENCY_NOT_FOUND";
    /// 技能序列化失败
    pub const SERIALIZE_FAILED: &str = "SKILL_SERIALIZE_FAILED";
    /// 技能保存成功（成功消息）
    pub const SAVED: &str = "SKILL_SAVED";
    /// 技能内容为空
    pub const CONTENT_EMPTY: &str = "SKILL_CONTENT_EMPTY";
    /// 未配置默认模型提供商
    pub const MODEL_PROVIDER_NOT_CONFIGURED: &str = "SKILL_MODEL_PROVIDER_NOT_CONFIGURED";
    /// 未配置默认模型
    pub const MODEL_NOT_CONFIGURED: &str = "SKILL_MODEL_NOT_CONFIGURED";
    /// 输出格式不正确
    pub const OUTPUT_FORMAT_ERROR: &str = "SKILL_OUTPUT_FORMAT_ERROR";
}

/// 专家(Expert)相关错误码
pub mod expert {
    /// 读取目录失败
    pub const READ_DIR_FAILED: &str = "EXPERT_READ_DIR_FAILED";
    /// 读取目录条目失败
    pub const READ_ENTRY_FAILED: &str = "EXPERT_READ_ENTRY_FAILED";
    /// 读取文件失败
    pub const READ_FILE_FAILED: &str = "EXPERT_READ_FILE_FAILED";
    /// 保存失败
    pub const SAVE_FAILED: &str = "EXPERT_SAVE_FAILED";
    /// 删除失败
    pub const DELETE_FAILED: &str = "EXPERT_DELETE_FAILED";
    /// 更新失败
    pub const UPDATE_FAILED: &str = "EXPERT_UPDATE_FAILED";
    /// 查询失败
    pub const QUERY_FAILED: &str = "EXPERT_QUERY_FAILED";
    /// 加载设置失败
    pub const LOAD_SETTINGS_FAILED: &str = "EXPERT_LOAD_SETTINGS_FAILED";
    /// 密钥解密失败
    pub const KEY_DECRYPT_FAILED: &str = "EXPERT_KEY_DECRYPT_FAILED";
    /// 无活跃密钥
    pub const NO_ACTIVE_KEY: &str = "EXPERT_NO_ACTIVE_KEY";
    /// LLM调用失败
    pub const LLM_CALL_FAILED: &str = "EXPERT_LLM_CALL_FAILED";
    /// JSON解析失败
    pub const JSON_PARSE_FAILED: &str = "EXPERT_JSON_PARSE_FAILED";
    /// 未找到供应商适配器
    pub const VENDOR_NOT_FOUND: &str = "EXPERT_VENDOR_NOT_FOUND";
}

/// Agent相关错误码
pub mod agent {
    /// Agent已在运行
    pub const RUNNING: &str = "AGENT_RUNNING";
    /// Agent未运行
    pub const NOT_RUNNING: &str = "AGENT_NOT_RUNNING";
    /// Agent未暂停
    pub const NOT_PAUSED: &str = "AGENT_NOT_PAUSED";
    /// 工作流未找到
    pub const WORKFLOW_NOT_FOUND: &str = "AGENT_WORKFLOW_NOT_FOUND";
    /// Agent未找到
    pub const NOT_FOUND: &str = "AGENT_NOT_FOUND";
}

/// 后台任务相关错误码
pub mod task {
    /// 命令包含危险字符，已拒绝
    pub const DANGEROUS_COMMAND: &str = "TASK_DANGEROUS_COMMAND";
    /// 任务未找到
    pub const NOT_FOUND: &str = "TASK_NOT_FOUND";
    /// 更新任务状态失败
    pub const UPDATE_FAILED: &str = "TASK_UPDATE_FAILED";
    /// 启动任务失败
    pub const START_FAILED: &str = "TASK_START_FAILED";
    /// 追加输出失败
    pub const OUTPUT_APPEND_FAILED: &str = "TASK_OUTPUT_APPEND_FAILED";
}

/// 初始化向导相关错误码
pub mod onboarding {
    /// Ollama本地提供者已启用，请到设置中拉取模型列表
    pub const OLLAMA_NOT_CONFIGURED: &str = "ONBOARDING_OLLAMA_NOT_CONFIGURED";
    /// OpenAI提供者已启用，请添加API Key
    pub const OPENAI_NOT_CONFIGURED: &str = "ONBOARDING_OPENAI_NOT_CONFIGURED";
    /// 请在设置中添加模型供应商
    pub const NO_PROVIDER: &str = "ONBOARDING_NO_PROVIDER";
    /// 添加Key失败
    pub const API_KEY_FAILED: &str = "ONBOARDING_API_KEY_FAILED";
}

/// 提供商相关错误码
pub mod provider {
    /// 获取模型列表超时，请检查网络连接和API地址
    pub const MODEL_LIST_TIMEOUT: &str = "PROVIDER_MODEL_LIST_TIMEOUT";
}

/// 搜索相关错误码
pub mod search {
    /// 未配置端点
    pub const ENDPOINT_NOT_CONFIGURED: &str = "SEARCH_ENDPOINT_NOT_CONFIGURED";
    /// 搜索提供商未配置
    pub const PROVIDER_NOT_CONFIGURED: &str = "SEARCH_PROVIDER_NOT_CONFIGURED";
}

/// 备份相关错误码
pub mod backup {
    /// 不支持的备份格式，仅支持sqlite和json格式
    pub const FORMAT_UNSUPPORTED: &str = "BACKUP_FORMAT_UNSUPPORTED";
}

/// 流式响应相关错误码
pub mod stream {
    /// 提供商返回空响应
    pub const EMPTY_RESPONSE: &str = "STREAM_EMPTY_RESPONSE";
}

/// Agent状态消息码
pub mod agent_status {
    /// 正在初始化
    pub const INITIALIZING: &str = "AGENT_STATUS_INITIALIZING";
    /// 正在调用模型
    pub const CALLING_MODEL: &str = "AGENT_STATUS_CALLING_MODEL";
    /// 已应用引导指令
    pub const STEER_APPLIED: &str = "AGENT_STATUS_STEER_APPLIED";
}

/// 思考块标记
pub mod thinking {
    /// 思考块开始标记
    pub const BLOCK_START: &str = "THINKING_BLOCK_START";
    /// 思考块结束标记
    pub const BLOCK_END: &str = "THINKING_BLOCK_END";
}

/// 标题生成相关错误码
pub mod title {
    /// 没有可用于生成标题的消息
    pub const NO_MESSAGES: &str = "TITLE_NO_MESSAGES";
}

/// 会话压缩相关错误码
pub mod session {
    /// 没有可压缩的消息
    pub const NO_MESSAGES: &str = "SESSION_NO_MESSAGES_TO_COMPRESS";
}

/// Agent指令相关错误码
pub mod steer {
    /// 指令太长
    pub const INSTRUCTION_TOO_LONG: &str = "STEER_INSTRUCTION_TOO_LONG";
}

/// 存储路径相关错误码
pub mod storage_path {
    /// 新目录与当前目录相同
    pub const SAME_AS_CURRENT: &str = "STORAGE_PATH_SAME_AS_CURRENT";
}

/// ZIP安全相关错误码
pub mod security {
    /// 检测到路径遍历
    pub const PATH_TRAVERSAL: &str = "SECURITY_PATH_TRAVERSAL";
    /// 访问被拒绝，文件在技能目录外
    pub const ACCESS_DENIED: &str = "SECURITY_ACCESS_DENIED";
}

/// 技能操作相关错误码
pub mod skill_operation {
    /// 仅支持GitHub源技能的回滚
    pub const ROLLBACK_NOT_SUPPORTED: &str = "SKILL_OP_ROLLBACK_NOT_SUPPORTED";
    /// 无效的source_ref格式
    pub const INVALID_FORMAT: &str = "SKILL_OP_INVALID_SOURCE_REF";
}

/// 终端相关错误码
pub mod terminal {
    /// 获取git分支失败
    pub const GIT_BRANCH_FAILED: &str = "TERMINAL_GIT_BRANCH_FAILED";
}

/// 平台集成相关错误码
pub mod platform {
    /// Telegram集成未启用
    pub const TELEGRAM_NOT_ENABLED: &str = "PLATFORM_TELEGRAM_NOT_ENABLED";
    /// Discord集成未启用
    pub const DISCORD_NOT_ENABLED: &str = "PLATFORM_DISCORD_NOT_ENABLED";
    /// API服务器未启用
    pub const API_SERVER_NOT_ENABLED: &str = "PLATFORM_API_SERVER_NOT_ENABLED";
}

/// 代理测试相关错误码
pub mod proxy {
    /// 无法使用内部/私有地址测试代理
    pub const ADDRESS_NOT_ALLOWED: &str = "PROXY_ADDRESS_NOT_ALLOWED";
}

/// 仪表盘相关错误码
pub mod dashboard {
    /// 目录不包含manifest.json
    pub const NO_MANIFEST: &str = "DASHBOARD_NO_MANIFEST";
    /// 复制manifest失败
    pub const COPY_MANIFEST_FAILED: &str = "DASHBOARD_COPY_MANIFEST_FAILED";
}

/// 文件操作相关错误码
pub mod file {
    /// 路径为空
    pub const PATH_EMPTY: &str = "FILE_PATH_EMPTY";
    /// 文件未找到
    pub const FILE_NOT_FOUND: &str = "FILE_NOT_FOUND";
    /// 文件和父目录都不存在
    pub const FILE_AND_PARENT_NOT_EXIST: &str = "FILE_AND_PARENT_NOT_EXIST";
}

/// 网关相关错误码
pub mod gateway {
    /// SSL已启用但未配置证书文件
    pub const SSL_NO_CERT: &str = "GATEWAY_SSL_NO_CERT";
    /// SSL已启用但未配置私钥文件
    pub const SSL_NO_KEY: &str = "GATEWAY_SSL_NO_KEY";
    /// HTTP在强制SSL时不可用
    pub const HTTP_UNAVAILABLE: &str = "GATEWAY_HTTP_UNAVAILABLE";
    /// 网关已在运行
    pub const ALREADY_RUNNING: &str = "GATEWAY_ALREADY_RUNNING";
}

// ── 别名模块：统一子命令内部使用的简写名称 ──
pub use agent as agent_err;
pub use backup as backup_err;
pub use browser as browser_err;
pub use conversation as conv_err;
pub use dashboard as dashboard_err;
pub use expert as expert_err;
pub use file as file_err;
pub use gateway as gateway_err;
pub use platform as platform_err;
pub use provider as provider_err;
pub use proxy as proxy_err;
pub use search as search_err;
pub use session as session_err;
pub use skill as skill_err;
pub use skill_operation as skill_op_err;
pub use steer as steer_err;
pub use storage as storage_err;
pub use storage_path as storage_path_err;
pub use terminal as terminal_err;
pub use thinking as thinking_err;
pub use title as title_err;
pub use tool as tool_err;
