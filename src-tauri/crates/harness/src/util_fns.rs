// SPDX-License-Identifier: AGPL-3.0-only

//! 通用工具函数
//!
//! 零业务逻辑的纯工具函数，供各 crate 共享使用。

/// 生成 UUID v4 字符串 ID
pub fn gen_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 获取当前 Unix 时间戳（秒）
pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

/// 获取当前时间的 RFC3339 格式字符串
pub fn current_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}
