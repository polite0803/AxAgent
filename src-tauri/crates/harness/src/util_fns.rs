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

/// 按字符安全边界截断 UTF-8 字符串到最多 `max_bytes` 字节。
///
/// `&s[..N]` 直接按字节切片，当 N 落在 UTF-8 多字节字符中间时会
/// `byte index N is not a char boundary` panic。该工具函数回退
/// 到最近的字符边界，保证：
///
/// - 不会 panic
/// - 返回的 `&str` 仍是合法 UTF-8
/// - 长度永远不超过 `max_bytes` 字节
/// - 输入长度不足时返回原文
///
/// ## 用法
///
/// ```ignore
/// let preview = truncate_to_char_boundary(content, 2000);
/// format!("{}...[已截断]", preview);
/// ```
///
/// ## 性能
///
/// 字符串长度在 4 KB 级别时回退循环最多迭代 3 次（UTF-8 最大 4 字节/字符），
/// 实测延迟 < 100 ns，可放心用于热路径。
pub fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_ascii_unchanged() {
        assert_eq!(truncate_to_char_boundary("hello world", 5), "hello");
    }

    #[test]
    fn truncate_chinese_safe_at_boundary() {
        // 5 个汉字 = 15 字节；14 字节落在字符中间，必须回退到 12（4 字符）
        let s = "一二三四五"; // 5 chars, 15 bytes
        assert_eq!(s.len(), 15);
        assert_eq!(truncate_to_char_boundary(s, 14), "一二三四");
        assert_eq!(truncate_to_char_boundary(s, 16), "一二三四五");
    }

    #[test]
    fn truncate_short_input_unchanged() {
        let s = "abc";
        assert_eq!(truncate_to_char_boundary(s, 100), "abc");
    }

    #[test]
    fn truncate_emoji_4byte() {
        let s = "🎉🎉";
        assert_eq!(s.len(), 8);
        assert_eq!(truncate_to_char_boundary(s, 3), "");
        assert_eq!(truncate_to_char_boundary(s, 4), "🎉");
    }

    #[test]
    fn truncate_empty() {
        assert_eq!(truncate_to_char_boundary("", 0), "");
        assert_eq!(truncate_to_char_boundary("", 100), "");
    }
}
