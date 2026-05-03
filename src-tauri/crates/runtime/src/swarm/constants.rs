//! Swarm 配置常量

/// 默认团队最大成员数
pub const MAX_TEAM_MEMBERS: usize = 8;

/// 队友心跳间隔（秒）
pub const HEARTBEAT_INTERVAL_SECS: u64 = 10;

/// 队友心跳超时（秒），超时视为离线
pub const HEARTBEAT_TIMEOUT_SECS: u64 = 30;

/// 任务默认超时（秒）
pub const DEFAULT_TASK_TIMEOUT_SECS: u64 = 600;

/// 重连最大尝试次数
pub const MAX_RECONNECT_ATTEMPTS: u32 = 3;

/// 重连间隔（秒）
pub const RECONNECT_INTERVAL_SECS: u64 = 5;

/// stdin/stdout JSON 行协议消息分隔符
pub const MESSAGE_DELIMITER: u8 = b'\n';
