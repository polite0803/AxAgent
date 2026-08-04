// SPDX-License-Identifier: AGPL-3.0-only

//! 设备同步模块共享工具函数

/// 从 XML 字符串中提取指定标签的文本值
///
/// # 参数
/// - `xml`: XML 字符串
/// - `tag`: 标签名（如 "D:href"）
///
/// # 返回
/// - `Some(String)`: 找到标签及其内容
/// - `None`: 未找到
pub fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);

    if let Some(start) = xml.find(&open_tag) {
        let start = start + open_tag.len();
        if let Some(end) = xml[start..].find(&close_tag) {
            return Some(xml[start..start + end].trim().to_string());
        }
    }

    None
}

/// 尝试解析 HTTP 日期格式（RFC 7231 / RFC 1123）
///
/// 支持格式：
/// - "Mon, 01 Jan 2024 00:00:00 GMT"
pub fn parse_http_date(date_str: &str) -> u64 {
    chrono::DateTime::parse_from_rfc2822(date_str)
        .map(|dt| dt.timestamp_millis() as u64)
        .unwrap_or(0)
}

/// 尝试解析 ISO 8601 日期格式
///
/// 支持格式：
/// - "2024-01-01T00:00:00Z"
/// - "2024-01-01T00:00:00.000Z"
pub fn parse_iso8601_date(date_str: &str) -> u64 {
    chrono::DateTime::parse_from_rfc3339(date_str)
        .map(|dt| dt.timestamp_millis() as u64)
        .or_else(|_| {
            // 尝试带毫秒的变体格式
            let cleaned = date_str.replace('.', "");
            chrono::DateTime::parse_from_str(&cleaned, "%Y-%m-%dT%H:%M:%S%z")
                .map(|dt| dt.timestamp_millis() as u64)
        })
        .unwrap_or(0)
}

/// 根据文件扩展名猜测 MIME 内容类型
pub fn guess_content_type(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();

    match ext.as_str() {
        "json" => "application/json",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "html" => "text/html",
        "xml" => "application/xml",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "tar" => "application/x-tar",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// Base64 编码（使用 base64 crate）
pub fn base64_encode(input: &str) -> String {
    use base64::{Engine, engine::general_purpose::STANDARD};
    STANDARD.encode(input.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_xml_value() {
        let xml = r#"<D:response><D:href>/path/to/file.txt</D:href></D:response>"#;
        assert_eq!(extract_xml_value(xml, "D:href"), Some("/path/to/file.txt".to_string()));
        assert_eq!(extract_xml_value(xml, "D:nonexistent"), None);
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode("hello"), "aGVsbG8=");
        assert_eq!(base64_encode("user:pass"), "dXNlcjpwYXNz");
        assert_eq!(base64_encode(""), "");
    }

    #[test]
    fn test_guess_content_type() {
        assert_eq!(guess_content_type("data.json"), "application/json");
        assert_eq!(guess_content_type("readme.md"), "text/markdown");
        assert_eq!(guess_content_type("image.png"), "image/png");
        assert_eq!(guess_content_type("unknown.xyz"), "application/octet-stream");
        assert_eq!(guess_content_type("no_extension"), "application/octet-stream");
    }

    #[test]
    fn test_parse_http_date() {
        let ts = parse_http_date("Mon, 01 Jan 2024 00:00:00 GMT");
        assert!(ts > 0);
        assert_eq!(parse_http_date("invalid"), 0);
    }

    #[test]
    fn test_parse_iso8601_date() {
        let ts = parse_iso8601_date("2024-01-01T00:00:00Z");
        assert!(ts > 0);

        let ts2 = parse_iso8601_date("2024-06-15T14:30:00+08:00");
        assert!(ts2 > 0);

        assert_eq!(parse_iso8601_date("invalid"), 0);
    }
}
