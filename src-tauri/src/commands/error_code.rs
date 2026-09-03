// SPDX-License-Identifier: AGPL-3.0-only

//! 集中式错误码定义
//!
//! 前端根据 error code 查询 i18n 翻译
//!
//! 本文件中的常量和别名供其他命令模块引用。
//!
//! 命名规范: {CATEGORY}_{SHORT_NAME}
//! - CATEGORY: 会话(CONVERSATION), 工具(TOOL), MCP, 浏览器(BROWSER)等
//! - SHORT_NAME: 简短描述性名称,如 NOT_FOUND, TIMEOUT, FAILED等
//!
//! 以下常量为外部（前端 i18n）契约消费：每个常量对应前端 11 种语言
//! i18n 的 `error` 段 key，由 `scripts/check-errorcode-alignment.mjs` 校验对齐。
//! 即便某些常量当前未被 Rust 命令直接引用（如 `conversation::NOT_FOUND`），
//! 也不得删除——删除会破坏契约对齐并使前端翻译成为孤儿码。
//! 因此 `dead_code` 在此属外部契约导致的误报，按 P0 规范
//! 「lint 反映合理设计时为例外，需说明」条款保留该 allow。
//! 理由:这些常量是后端↔前端的错误码 API 契约。即便当前无后端命令直接引用，
//! 前端 11 语言均有 `error.CONSTANT` 翻译键,且 crates/插件可动态构建这些错误码。
//! 删除会破坏契约对齐、使前端翻译成为孤儿码(被 check-errorcode-alignment.mjs 捕获)。
//! 详见 src-tauri/src/commands/error_code.rs / crates/harness/src/error_codes.rs 的模块设计。
#![allow(dead_code)]

/// 会话/对话相关错误码
pub mod conversation {
    /// 内部服务器错误
    pub const INTERNAL: &str = "CONVERSATION_INTERNAL";
    /// 此会话不是工作流类型，请使用普通归档
    pub const NOT_WORKFLOW: &str = "CONVERSATION_NOT_WORKFLOW";
    /// 会话已归档，请勿重复操作
    pub const ALREADY_ARCHIVED: &str = "CONVERSATION_ALREADY_ARCHIVED";
    /// 会话未找到
    pub const NOT_FOUND: &str = "CONVERSATION_NOT_FOUND";
    /// 删除会话失败
    pub const DELETE_FAILED: &str = "CONVERSATION_DELETE_FAILED";
}

/// 微调训练相关错误码
pub mod fine_tune {
    /// 删除训练任务失败
    pub const DELETE_FAILED: &str = "FINE_TUNE_DELETE_FAILED";
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
    /// 工具重复注册（运行时动态注册时与已有工具同名）
    pub const REGISTRATION_DUPLICATE: &str = "TOOL_REGISTRATION_DUPLICATE";
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
    /// Agent 会话未找到
    pub const AGENT_SESSION_NOT_FOUND: &str = "MCP_AGENT_SESSION_NOT_FOUND";
    /// Agent 会话取消失败
    pub const AGENT_SESSION_CANCEL_FAILED: &str = "MCP_AGENT_SESSION_CANCEL_FAILED";
}

/// 浏览器相关错误码
pub mod browser {
    /// 浏览器客户端未初始化
    pub const NOT_INITIALIZED: &str = "BROWSER_NOT_INITIALIZED";
    /// 浏览器操作失败
    pub const ACTION_FAILED: &str = "BROWSER_ACTION_FAILED";
    /// URL格式无效
    pub const INVALID_URL: &str = "BROWSER_INVALID_URL";
    /// URL协议不允许（仅允许 http/https）
    pub const SCHEME_NOT_ALLOWED: &str = "BROWSER_SCHEME_NOT_ALLOWED";
    /// 目标地址不允许访问（内网/私有地址）
    pub const ADDRESS_NOT_ALLOWED: &str = "BROWSER_ADDRESS_NOT_ALLOWED";
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
    /// 资源未找到
    pub const NOT_FOUND: &str = "SKILL_NOT_FOUND";
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
    /// 内部服务器错误
    pub const INTERNAL: &str = "AGENT_INTERNAL";
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
    /// 轨迹未找到
    pub const TRAJECTORY_NOT_FOUND: &str = "TRAJECTORY_NOT_FOUND";
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
    /// 未知的预设
    pub const UNKNOWN_PRESET: &str = "UNKNOWN_PRESET";
}

/// 提供商相关错误码
pub mod provider {
    /// 获取模型列表超时，请检查网络连接和API地址
    pub const MODEL_LIST_TIMEOUT: &str = "PROVIDER_MODEL_LIST_TIMEOUT";
    /// 未找到提供商适配器
    pub const ADAPTER_NOT_FOUND: &str = "PROVIDER_ADAPTER_NOT_FOUND";
}

/// 本地模型（llama.cpp）服务管理错误码
pub mod local_model {
    /// 供应商不存在
    pub const PROVIDER_NOT_FOUND: &str = "LOCAL_MODEL_PROVIDER_NOT_FOUND";
    /// 状态探测失败
    pub const STATUS_FAILED: &str = "LOCAL_MODEL_STATUS_FAILED";
    /// 服务未在运行
    pub const NOT_RUNNING: &str = "LOCAL_MODEL_NOT_RUNNING";
    /// 启动失败（含启动超时）
    pub const START_FAILED: &str = "LOCAL_MODEL_START_FAILED";
    /// 停止失败
    pub const STOP_FAILED: &str = "LOCAL_MODEL_STOP_FAILED";
    /// 启动配置无效（可执行文件/模型文件不存在）
    pub const INVALID_CONFIG: &str = "LOCAL_MODEL_INVALID_CONFIG";
    /// 嵌入连通性测试失败
    pub const EMBED_TEST_FAILED: &str = "LOCAL_MODEL_EMBED_TEST_FAILED";
    /// 删除本地模型文件失败
    pub const DELETE_FAILED: &str = "LOCAL_MODEL_DELETE_FAILED";
    /// 日志文件不存在
    pub const LOG_NOT_FOUND: &str = "LOCAL_MODEL_LOG_NOT_FOUND";
    /// 日志读取失败
    pub const LOG_READ_FAILED: &str = "LOCAL_MODEL_LOG_READ_FAILED";
    /// llama-server 可执行文件未找到
    pub const SERVER_NOT_FOUND: &str = "LOCAL_MODEL_SERVER_NOT_FOUND";
    /// llama.cpp 安装失败
    pub const INSTALL_FAILED: &str = "LOCAL_MODEL_INSTALL_FAILED";
    /// llama.cpp 安装进行中
    pub const INSTALL_IN_PROGRESS: &str = "LOCAL_MODEL_INSTALL_IN_PROGRESS";
    /// 下载失败
    pub const DOWNLOAD_FAILED: &str = "LOCAL_MODEL_DOWNLOAD_FAILED";
    /// 端口被占用
    pub const PORT_IN_USE: &str = "LOCAL_MODEL_PORT_IN_USE";
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
    /// 备份创建失败
    pub const CREATE_FAILED: &str = "BACKUP_CREATE_FAILED";
    /// 备份恢复失败
    pub const RESTORE_FAILED: &str = "BACKUP_RESTORE_FAILED";
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
pub mod skill_op_err {
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

/// 工作流相关错误码
pub mod workflow {
    /// 工作流未找到
    pub const NOT_FOUND: &str = "WORKFLOW_NOT_FOUND";
    /// 工作流计划未找到
    pub const PLAN_NOT_FOUND: &str = "WORKFLOW_PLAN_NOT_FOUND";
    /// JSON格式无效
    pub const INVALID_JSON: &str = "WORKFLOW_INVALID_JSON";
    /// 系统模板受保护（认知编排器等 SystemOnly 模板禁止用户 CRUD）
    pub const SYSTEM_TEMPLATE_PROTECTED: &str = "WORKFLOW_SYSTEM_TEMPLATE_PROTECTED";
    /// 运行时工具状态无效（update_workflow_tool_status 收到非 pending/active/disabled 值）
    pub const TOOL_INVALID_STATUS: &str = "WORKFLOW_TOOL_INVALID_STATUS";
    /// LLM 工具生成器未配置（无可用 provider，无法执行发现闭环）
    pub const TOOL_PROVIDER_NOT_CONFIGURED: &str = "LLM_TOOL_PROVIDER_NOT_CONFIGURED";
    /// 生成工具未通过沙箱编译验证（拒绝落地）
    pub const TOOL_SANDBOX_REJECTED: &str = "WORKFLOW_TOOL_SANDBOX_REJECTED";
}

/// 股票工作流相关错误码
pub mod stock_workflow {
    /// 内部错误
    pub const INTERNAL: &str = "STOCK_WORKFLOW_INTERNAL";
}

/// 股票分析种子数据相关错误码
pub mod stock_setup {
    /// 内部错误
    pub const INTERNAL: &str = "STOCK_SETUP_INTERNAL";
}

/// OPC 需求发现工作流种子数据相关错误码
pub mod opc_setup {
    /// 内部错误
    pub const INTERNAL: &str = "OPC_SETUP_INTERNAL";
}

/// 工作流反思 / 进化 / 优化相关错误码(阶段 5 wiring 层)
pub mod workflow_reflection {
    /// 反思执行失败(内部异常,通常底层 trait 返回 Err)
    pub const REFLECT_FAILED: &str = "WORKFLOW_REFLECTION_REFLECT_FAILED";
    /// 反思记录缺失或无效(字段不完整 / 反序列化失败)
    pub const RECORD_INVALID: &str = "WORKFLOW_REFLECTION_RECORD_INVALID";
    /// 工作流模板无效(节点缺失 / 结构错误)
    pub const TEMPLATE_INVALID: &str = "WORKFLOW_REFLECTION_TEMPLATE_INVALID";
    /// 进化执行失败(代数超限 / 种群为空 / 变异异常)
    pub const EVOLVE_FAILED: &str = "WORKFLOW_REFLECTION_EVOLVE_FAILED";
    /// 优化建议生成失败
    pub const SUGGEST_FAILED: &str = "WORKFLOW_REFLECTION_SUGGEST_FAILED";
    /// 优化建议应用失败(模板修改冲突)
    pub const APPLY_FAILED: &str = "WORKFLOW_REFLECTION_APPLY_FAILED";
    /// 能力接缝未注册(反思器 / 进化器 / 优化器未在启动时注册进能力注册表)
    pub const SEAM_NOT_READY: &str = "WORKFLOW_REFLECTION_SEAM_NOT_READY";
}

/// 平台集成相关错误码
pub mod platform {
    /// Telegram集成未启用
    pub const TELEGRAM_NOT_ENABLED: &str = "PLATFORM_TELEGRAM_NOT_ENABLED";
    /// Discord集成未启用
    pub const DISCORD_NOT_ENABLED: &str = "PLATFORM_DISCORD_NOT_ENABLED";
    /// API服务器未启用
    pub const API_SERVER_NOT_ENABLED: &str = "PLATFORM_API_SERVER_NOT_ENABLED";
    /// Webhook订阅管理器未配置
    pub const WEBHOOK_NOT_CONFIGURED: &str = "PLATFORM_WEBHOOK_NOT_CONFIGURED";
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

/// 语音会话相关错误码（realtime WebSocket 通道）
///
/// 这些码通过 `RealtimeServerMessage::Error { code, params, .. }` 回传前端，
/// 前端按 `t("error.${code}", params)` 翻译。与 locales/*.json 的 `error` 段对齐。
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

/// 通用错误码
pub mod common {
    /// 内部服务器错误
    pub const INTERNAL: &str = "COMMON_INTERNAL";
    /// 无效的输入参数
    pub const INVALID_INPUT: &str = "COMMON_INVALID_INPUT";
    /// 超时
    pub const TIMEOUT: &str = "COMMON_TIMEOUT";
}

/// 市场目录相关错误码
pub mod marketplace {
    /// 目录项未找到
    pub const NOT_FOUND: &str = "MARKETPLACE_NOT_FOUND";
    /// 安装失败
    pub const INSTALL_FAILED: &str = "MARKETPLACE_INSTALL_FAILED";
    /// 卸载失败
    pub const UNINSTALL_FAILED: &str = "MARKETPLACE_UNINSTALL_FAILED";
    /// 发布失败
    pub const PUBLISH_FAILED: &str = "MARKETPLACE_PUBLISH_FAILED";
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

// 记忆（Memory）相关错误码 - 已迁移至 axagent_harness::error_codes::memory
// pub mod memory { ... } // 已删除，使用 axagent_harness::error_codes::memory

// 知识库（RAG）相关错误码 - 已迁移至 axagent_harness::error_codes::knowledge
// pub mod knowledge { ... } // 已删除，使用 axagent_harness::error_codes::knowledge

/// 知识源（Knowledge Source）相关错误码（commands/knowledge_source.rs）
pub mod knowledge_source {
    /// URL 必须以 http:// 或 https:// 开头
    pub const URL_SCHEME_INVALID: &str = "KNOWLEDGE_SOURCE_URL_SCHEME_INVALID";
    /// 禁止访问内网或私有地址
    pub const URL_BLOCKED: &str = "KNOWLEDGE_SOURCE_URL_BLOCKED";
    /// HTTP 请求失败
    pub const HTTP_FETCH_FAILED: &str = "KNOWLEDGE_SOURCE_HTTP_FETCH_FAILED";
    /// HTTP 状态码异常
    pub const HTTP_STATUS_ERROR: &str = "KNOWLEDGE_SOURCE_HTTP_STATUS_ERROR";
    /// 读取响应体失败
    pub const HTTP_READ_FAILED: &str = "KNOWLEDGE_SOURCE_HTTP_READ_FAILED";
    /// 目标内容为二进制类型，无法提取文本
    pub const CONTENT_BINARY: &str = "KNOWLEDGE_SOURCE_CONTENT_BINARY";
    /// 没有可用的 Wiki 知识库
    pub const WIKI_NOT_AVAILABLE: &str = "KNOWLEDGE_SOURCE_WIKI_NOT_AVAILABLE";
    /// 知识源不存在
    pub const NOT_FOUND: &str = "KNOWLEDGE_SOURCE_NOT_FOUND";
    /// 暂不支持的知识源类型
    pub const TYPE_UNSUPPORTED: &str = "KNOWLEDGE_SOURCE_TYPE_UNSUPPORTED";
    /// 仓库地址无法解析为 GitHub 仓库
    pub const REPO_PARSE_FAILED: &str = "KNOWLEDGE_SOURCE_REPO_PARSE_FAILED";
    /// GitHub API 请求/解析失败
    pub const GITHUB_API_FAILED: &str = "KNOWLEDGE_SOURCE_GITHUB_API_FAILED";
    /// 仓库中未找到 Markdown 文档
    pub const GITHUB_NO_DOCS: &str = "KNOWLEDGE_SOURCE_GITHUB_NO_DOCS";
    /// RSS 请求失败
    pub const RSS_FETCH_FAILED: &str = "KNOWLEDGE_SOURCE_RSS_FETCH_FAILED";
    /// RSS 读取失败
    pub const RSS_READ_FAILED: &str = "KNOWLEDGE_SOURCE_RSS_READ_FAILED";
    /// RSS 解析失败
    pub const RSS_PARSE_FAILED: &str = "KNOWLEDGE_SOURCE_RSS_PARSE_FAILED";
    /// sitemap 请求失败
    pub const SITEMAP_FETCH_FAILED: &str = "KNOWLEDGE_SOURCE_SITEMAP_FETCH_FAILED";
    /// sitemap 读取失败
    pub const SITEMAP_READ_FAILED: &str = "KNOWLEDGE_SOURCE_SITEMAP_READ_FAILED";
    /// sitemap 中未解析到任何 URL
    pub const SITEMAP_EMPTY: &str = "KNOWLEDGE_SOURCE_SITEMAP_EMPTY";
    /// cron 表达式必须是 5 个字段
    pub const CRON_INVALID: &str = "KNOWLEDGE_SOURCE_CRON_INVALID";
}

// 向量存储相关错误码 - 已迁移至 axagent_harness::error_codes::vector
// pub mod vector { ... } // 已删除，使用 axagent_harness::error_codes::vector

// Wiki 相关错误码 - 已迁移至 axagent_harness::error_codes::wiki
// pub mod wiki_ext { ... } // 已删除，使用 axagent_harness::error_codes::wiki

// Re-export harness 中的错误码模块，便于命令层统一引用
pub use axagent_harness::error_codes::knowledge;
pub use axagent_harness::error_codes::memory;
pub use axagent_harness::error_codes::vector;
pub use axagent_harness::error_codes::wiki;

// ── 别名模块：统一子命令内部使用的简写名称 ──
// 以下 re-export 被具体命令文件通过 `use error_code::<alias>` 直接引用，
// 属活跃契约别名，删除会导致编译失败（非死代码）：
//   - conv_err   ← conversations/compress.rs、conversations/messages/compress.rs
//   - skill_err  ← skills/analysis.rs
// 其余命令文件改用 `use error_code::<module> as <module>_err` 本地别名，
// 故对应的 re-export 已删除（P0：删除死代码而非 allow）。
pub use conversation as conv_err;
pub use skill as skill_err;

/// Fleet（多办公室 AI 团队）相关错误码
pub mod fleet {
    /// 舰队未找到
    pub const NOT_FOUND: &str = "FLEET_NOT_FOUND";
    /// 舰队名称为空
    pub const NAME_REQUIRED: &str = "FLEET_NAME_REQUIRED";
    /// 舰队无任何成员
    pub const NO_MEMBERS: &str = "FLEET_NO_MEMBERS";
    /// 舰队所有成员均不可用（已暂停/错误/离线）
    pub const ALL_MEMBERS_UNAVAILABLE: &str = "FLEET_ALL_MEMBERS_UNAVAILABLE";
    /// 成员未找到
    pub const MEMBER_NOT_FOUND: &str = "FLEET_MEMBER_NOT_FOUND";
    /// 没有可用的模型提供商
    pub const NO_PROVIDER: &str = "FLEET_NO_PROVIDER";
    /// LLM 意图路由调用失败
    pub const ROUTING_FAILED: &str = "FLEET_ROUTING_FAILED";
    /// Agent 执行失败
    pub const EXECUTION_FAILED: &str = "FLEET_EXECUTION_FAILED";
    /// 路由目标 slug 不在成员列表中
    pub const TARGET_NOT_IN_FLEET: &str = "FLEET_TARGET_NOT_IN_FLEET";
    /// 同名 slug 的成员已存在（slug 是路由键，必须唯一）
    pub const SLUG_EXISTS: &str = "FLEET_SLUG_EXISTS";
}

/// 设备同步（Device Sync）相关错误码
pub mod device_sync {
    // ─── 设备管理 ────────────────────────────────────────────────────
    /// 设备未找到
    pub const DEVICE_NOT_FOUND: &str = "DEVICE_SYNC_DEVICE_NOT_FOUND";
    /// 设备已配对
    pub const DEVICE_ALREADY_PAIRED: &str = "DEVICE_SYNC_DEVICE_ALREADY_PAIRED";
    /// 设备未配对
    pub const DEVICE_NOT_PAIRED: &str = "DEVICE_SYNC_DEVICE_NOT_PAIRED";
    /// 设备已禁用
    pub const DEVICE_DISABLED: &str = "DEVICE_SYNC_DEVICE_DISABLED";
    /// 配对码无效
    pub const INVALID_PAIRING_CODE: &str = "DEVICE_SYNC_INVALID_PAIRING_CODE";
    /// 配对码已过期
    pub const PAIRING_CODE_EXPIRED: &str = "DEVICE_SYNC_PAIRING_CODE_EXPIRED";
    /// 信任级别不足
    pub const INSUFFICIENT_TRUST_LEVEL: &str = "DEVICE_SYNC_INSUFFICIENT_TRUST_LEVEL";
    /// 设备权限不存在
    pub const PERMISSIONS_NOT_FOUND: &str = "DEVICE_SYNC_PERMISSIONS_NOT_FOUND";
    /// 设备未初始化
    pub const DEVICE_NOT_INITIALIZED: &str = "DEVICE_SYNC_DEVICE_NOT_INITIALIZED";

    // ─── 权限管理 ────────────────────────────────────────────────────
    /// 权限不足
    pub const PERMISSION_DENIED: &str = "DEVICE_SYNC_PERMISSION_DENIED";

    // ─── 同步操作 ────────────────────────────────────────────────────
    /// 同步失败
    pub const SYNC_FAILED: &str = "DEVICE_SYNC_FAILED";
    /// 同步超时
    pub const SYNC_TIMEOUT: &str = "DEVICE_SYNC_TIMEOUT";
    /// 冲突检测失败
    pub const CONFLICT_DETECTION_FAILED: &str = "DEVICE_SYNC_CONFLICT_DETECTION_FAILED";
    /// 冲突解决失败
    pub const CONFLICT_RESOLUTION_FAILED: &str = "DEVICE_SYNC_CONFLICT_RESOLUTION_FAILED";
    /// 冲突记录不存在
    pub const CONFLICT_NOT_FOUND: &str = "DEVICE_SYNC_CONFLICT_NOT_FOUND";
    /// 同步已在进行中
    pub const SYNC_ALREADY_IN_PROGRESS: &str = "DEVICE_SYNC_ALREADY_IN_PROGRESS";
    /// 无待同步变更
    pub const NO_CHANGES_TO_SYNC: &str = "DEVICE_SYNC_NO_CHANGES_TO_SYNC";

    // ─── 加密 ────────────────────────────────────────────────────────
    /// 加密失败
    pub const ENCRYPTION_FAILED: &str = "DEVICE_SYNC_ENCRYPTION_FAILED";
    /// 解密失败
    pub const DECRYPTION_FAILED: &str = "DEVICE_SYNC_DECRYPTION_FAILED";
    /// 加密必须提供密码
    pub const PASSWORD_REQUIRED: &str = "DEVICE_SYNC_PASSWORD_REQUIRED";
    /// 加密密码不能为空
    pub const PASSWORD_EMPTY: &str = "DEVICE_SYNC_PASSWORD_EMPTY";
    /// 解密需要提供盐值
    pub const SALT_REQUIRED: &str = "DEVICE_SYNC_SALT_REQUIRED";

    // ─── 存储 ────────────────────────────────────────────────────────
    /// 存储操作失败
    pub const STORAGE_OPERATION_FAILED: &str = "DEVICE_SYNC_STORAGE_OPERATION_FAILED";

    // ─── 策略 ────────────────────────────────────────────────────────
    /// 策略操作失败
    pub const POLICY_OPERATION_FAILED: &str = "DEVICE_SYNC_POLICY_OPERATION_FAILED";
}

/// 能力发现相关错误码
pub mod capability {
    /// 能力注册失败
    pub const REGISTER_FAILED: &str = "CAPABILITY_REGISTER_FAILED";
    /// 能力发现失败
    pub const DISCOVER_FAILED: &str = "CAPABILITY_DISCOVER_FAILED";
    /// 能力未找到
    pub const NOT_FOUND: &str = "CAPABILITY_NOT_FOUND";
    /// 能力索引失败
    pub const INDEX_FAILED: &str = "CAPABILITY_INDEX_FAILED";
    /// 嵌入生成失败
    pub const EMBEDDING_FAILED: &str = "CAPABILITY_EMBEDDING_FAILED";
    /// 向量存储操作失败
    pub const VECTOR_STORE_FAILED: &str = "CAPABILITY_VECTOR_STORE_FAILED";
    /// 无效的能力护照
    pub const INVALID_PASSPORT: &str = "CAPABILITY_INVALID_PASSPORT";
    /// 能力列表获取失败
    pub const LIST_FAILED: &str = "CAPABILITY_LIST_FAILED";
    /// 能力统计获取失败
    pub const STATS_FAILED: &str = "CAPABILITY_STATS_FAILED";
    /// 元数据恢复失败
    pub const METADATA_RESTORE_FAILED: &str = "CAPABILITY_METADATA_RESTORE_FAILED";
    /// 能力进化失败
    pub const EVOLVE_FAILED: &str = "CAPABILITY_EVOLVE_FAILED";
    /// 能力不可进化（外部插件只读能力，evolvable = none）
    pub const NOT_EVOLVABLE: &str = "CAPABILITY_NOT_EVOLVABLE";
}

/// 用户提问通道已关闭错误码
pub mod agent_input {
    /// 用户提问通道已关闭
    pub const CHANNEL_CLOSED: &str = "AGENT_INPUT_CHANNEL_CLOSED";
    /// 等待用户回复超时（5 分钟）
    pub const WAIT_REPLY_TIMEOUT: &str = "AGENT_INPUT_WAIT_REPLY_TIMEOUT";
    /// 没有可用的 provider/model：请先在设置中启用提供商
    pub const NO_PROVIDER: &str = "AGENT_INPUT_NO_PROVIDER";
}

/// Agent 角色相关错误码
pub mod agent_role {
    /// 内置角色不可删除
    pub const BUILTIN_NOT_DELETABLE: &str = "AGENT_ROLE_BUILTIN_NOT_DELETABLE";
}

/// 备份路径穿越相关错误码
pub mod backup_security {
    /// 路径穿越检测失败：cloud_key 指向了备份目录之外的位置
    pub const CLOUD_KEY_TRAVERSAL: &str = "BACKUP_CLOUD_KEY_TRAVERSAL";
}

/// 流式请求并发相关错误码
pub mod streaming {
    /// 已有正在进行的请求，请等待完成后再发送
    pub const REQUEST_IN_PROGRESS: &str = "STREAMING_REQUEST_IN_PROGRESS";
}

/// DynamicUI 相关错误码
pub mod dynamic_ui {
    /// 内置 Schema 不允许修改
    pub const BUILTIN_NOT_MODIFIABLE: &str = "DYNAMIC_UI_BUILTIN_NOT_MODIFIABLE";
    /// 内置 Schema 不允许删除
    pub const BUILTIN_NOT_DELETABLE: &str = "DYNAMIC_UI_BUILTIN_NOT_DELETABLE";
    /// DynamicUI Schema 不存在
    pub const SCHEMA_NOT_FOUND: &str = "DYNAMIC_UI_SCHEMA_NOT_FOUND";
    /// 内置 Schema 不允许回滚
    pub const BUILTIN_NOT_ROLLBACK: &str = "DYNAMIC_UI_BUILTIN_NOT_ROLLBACK";
    /// 编辑指令不能为空
    pub const EDIT_PROMPT_EMPTY: &str = "DYNAMIC_UI_EDIT_PROMPT_EMPTY";
    /// AI 返回的 Schema 缺少必要字段 (type/id)
    pub const SCHEMA_MISSING_FIELD: &str = "DYNAMIC_UI_SCHEMA_MISSING_FIELD";
    /// 生成指令不能为空
    pub const GENERATE_PROMPT_EMPTY: &str = "DYNAMIC_UI_GENERATE_PROMPT_EMPTY";
}

/// 进化引擎相关错误码
pub mod evolution_engine {
    /// 没有足够的轨迹数据用于进化
    pub const INSUFFICIENT_TRAJECTORY: &str = "EVOLUTION_INSUFFICIENT_TRAJECTORY";
    /// 初始反馈不能为空
    pub const INITIAL_FEEDBACK_EMPTY: &str = "EVOLUTION_INITIAL_FEEDBACK_EMPTY";
}

/// 自然语言转 Cron 相关错误码
pub mod nl_to_cron {
    /// 请输入定时任务描述
    pub const DESCRIPTION_EMPTY: &str = "NL_TO_CRON_DESCRIPTION_EMPTY";
}

/// 研究报告相关错误码
pub mod research {
    /// 对话消息不足，无法生成报告
    pub const NOT_ENOUGH_MESSAGES: &str = "RESEARCH_NOT_ENOUGH_MESSAGES";
    /// 研究主题不能为空
    pub const TOPIC_EMPTY: &str = "RESEARCH_TOPIC_EMPTY";
}

/// 会话分享相关错误码
pub mod session_share {
    /// 无效的邀请码
    pub const INVALID_CODE: &str = "SESSION_SHARE_INVALID_CODE";
    /// 会话已满，无法加入
    pub const ALREADY_FULL: &str = "SESSION_SHARE_ALREADY_FULL";
    /// 会话不存在
    pub const NOT_FOUND: &str = "SESSION_SHARE_NOT_FOUND";
}

/// 工作流引擎相关错误码
pub mod work_engine {
    /// execution_id 无效或工作流未注册
    pub const EXECUTION_NOT_FOUND: &str = "WORK_ENGINE_EXECUTION_NOT_FOUND";
}

/// 插件组合 Profile（缺陷 #9：agent 预设上升为可 dump/patch 的组合机制）
pub mod plugin_profile {
    /// 插件组合 Profile 不存在
    pub const NOT_FOUND: &str = "PLUGIN_PROFILE_NOT_FOUND";
    /// 插件组合 Profile 名称重复
    pub const DUPLICATE_NAME: &str = "PLUGIN_PROFILE_DUPLICATE_NAME";
    /// 插件组合 Profile 持久化 I/O 失败
    pub const IO_FAILED: &str = "PLUGIN_PROFILE_IO_FAILED";
}

/// Unity 改造相关错误码
///
/// P0 阶段：任务形态分类器（原则三标尺：上下文保留成本 × 安全隔离需求）。
/// 与前端 `error.UNITY_P0_CLASSIFIER_FAILED` 翻译键对齐。
/// 镜像 `crates/harness/src/error_codes.rs::unity`，两处定义值必须一致。
pub mod unity {
    /// 任务形态分类失败（分类器内部异常，回退到 HandleLocally 策略）
    pub const P0_CLASSIFIER_FAILED: &str = "UNITY_P0_CLASSIFIER_FAILED";
}
