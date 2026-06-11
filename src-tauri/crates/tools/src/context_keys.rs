// SPDX-License-Identifier: AGPL-3.0-only

//! ToolContext.extra 标准 key 常量定义
//!
//! 通过 ToolContext.extra HashMap 传递配置给工具，替代硬编码和全局状态。

/// 搜索提供商类型： "ddg"|"tavily"|"brave"|"bing"|"serpapi"|"google_pse"|"zhipu"|"bocha"
pub const SEARCH_PROVIDER_TYPE: &str = "search.provider_type";

/// 搜索 API 端点 URL
pub const SEARCH_ENDPOINT: &str = "search.endpoint";

/// 已解密的搜索 API key（仅内存中传递，不序列化到日志）
pub const SEARCH_API_KEY: &str = "search.api_key";

/// 区域设置 "cn"|"us" 等
pub const SEARCH_REGION: &str = "search.region";

/// 单次搜索结果数量上限
pub const SEARCH_MAX_RESULTS: &str = "search.max_results";

/// 搜索超时毫秒数
pub const SEARCH_TIMEOUT_MS: &str = "search.timeout_ms";

/// 安全搜索级别 0|1|2
pub const SEARCH_SAFE_SEARCH: &str = "search.safe_search";

/// 会话 ID
pub const CONVERSATION_ID: &str = "core.conversation_id";

/// 数据库路径（替代 global_state 的 set_db_path）
pub const SEA_DB_PATH: &str = "core.sea_db_path";
