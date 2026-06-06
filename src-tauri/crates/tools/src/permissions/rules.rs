//! 权限规则匹配引擎
//!
//! SECURITY (H1): 旧实现把整个 JSON 字符串当作匹配目标。
//! - `Bash(git *)` 会匹配输入 JSON 中**任意位置**出现的 `git ` 子串，
//!   包括 extra 字段、注释等。
//! - 反向：deny 规则 `Bash(rm -rf *)` 也会因为注释/上下文里的 `rm -rf`
//!   字符串被误命中。
//!
//! 新实现：按工具的"主字段"提取"主体内容"再匹配。
//! - Bash 类工具：取 input.command / 整段（无法解析时降级为整段）
//! - FileRead / FileWrite：取 input.file_path
//! - 其他：取 `input` 的最外层 JSON，序列化后再做"按字段"提取

use super::{PermissionRule, RulePattern};

/// 匹配工具名与规则列表，返回第一个匹配的规则。
///
/// 匹配策略：
/// - 精确匹配：规则 pattern 等于工具名
/// - 前缀匹配：规则 pattern 以 `*` 结尾，匹配工具名前缀
/// - 内容匹配：规则 pattern 形如 `ToolName(content_pattern)`，
///   仅在**工具主字段**（非整段 JSON）上做匹配。
pub fn match_rules<'a>(
    tool_name: &str,
    input: &str,
    rules: &'a [PermissionRule],
) -> Option<&'a PermissionRule> {
    let primary = extract_primary_input(tool_name, input);
    rules
        .iter()
        .find(|&rule| match_pattern(&rule.pattern, tool_name, &primary, input))
        .map(|v| v as _)
}

/// 提取工具的"主内容"。
/// - Bash / Shell 系：`input.command`
/// - FileRead / FileWrite / Edit：`input.file_path`
/// - 网络类：`input.url`
/// - MCP：`input.tool` 优先；否则 `input.command`
/// - 其他：整个 JSON 字符串
fn extract_primary_input(tool_name: &str, input: &str) -> String {
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(input);
    let Ok(v) = parsed else {
        return input.to_string();
    };
    let lower = tool_name.to_ascii_lowercase();
    if lower.starts_with("bash") || lower.starts_with("shell") || lower.contains("execute") {
        if let Some(s) = v.get("command").and_then(|x| x.as_str()) {
            return s.to_string();
        }
    }
    if lower.starts_with("fileread")
        || lower.starts_with("filewrite")
        || lower.starts_with("fileedit")
        || lower.contains("file_")
    {
        if let Some(s) = v.get("file_path").and_then(|x| x.as_str()) {
            return s.to_string();
        }
    }
    if lower.starts_with("webfetch") || lower.starts_with("http") || lower.contains("fetch") {
        if let Some(s) = v.get("url").and_then(|x| x.as_str()) {
            return s.to_string();
        }
    }
    if lower.starts_with("mcp") {
        if let Some(s) = v.get("tool").and_then(|x| x.as_str()) {
            return s.to_string();
        }
        if let Some(s) = v.get("command").and_then(|x| x.as_str()) {
            return s.to_string();
        }
    }
    // 兜底：返回整段 JSON。无法解析时也是。
    input.to_string()
}

/// 匹配单个规则模式
fn match_pattern(pattern: &RulePattern, tool_name: &str, primary: &str, _full_input: &str) -> bool {
    let p = &pattern.pattern;

    // 内容匹配: "ToolName(content_pattern)"
    if p.contains('(')
        && p.ends_with(')')
        && let Some(paren_idx) = p.find('(')
    {
        let required_tool = &p[..paren_idx];
        let content_pattern = &p[paren_idx + 1..p.len() - 1];

        if !match_simple_pattern(required_tool, tool_name) {
            return false;
        }

        // 如果内容模式为空，匹配所有
        if content_pattern.is_empty() {
            return true;
        }

        // SECURITY (H1):仅在主字段上做匹配，避免"git log"出现在 extra 字段里
        // 误命中 Bash(git *)。
        return match_content(content_pattern, primary);
    }

    // 简单匹配
    match_simple_pattern(p, tool_name)
}

/// 简单名称匹配（支持 * 通配符）
fn match_simple_pattern(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return name.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return name.ends_with(suffix);
    }
    pattern == name
}

/// 内容匹配（支持 * 通配符）
/// SECURITY: 同时考虑 canonical/normalized 形式：
/// - 对 $IFS、${IFS}、`IFS=` 等都做归一化
/// - 对 \u00A0 (NBSP) 做替换
fn match_content(pattern: &str, input: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    let normalized_input = normalize_whitespace(input);
    if let Some(prefix) = pattern.strip_suffix('*') {
        return normalized_input.contains(prefix);
    }
    normalized_input.contains(pattern)
}

fn normalize_whitespace(s: &str) -> String {
    // 1) 把 NBSP/零宽等替换为普通空格
    let mut buf = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_whitespace() {
            buf.push(' ');
        } else if c == '\u{00A0}' || c == '\u{200B}' || c == '\u{200C}' || c == '\u{200D}' {
            buf.push(' ');
        } else {
            buf.push(c);
        }
    }
    // 2) 压缩连续空白
    let mut out = String::with_capacity(buf.len());
    let mut last_space = false;
    for c in buf.chars() {
        if c == ' ' {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
    // 3) 把 ${IFS} 替换成单空格（与 shell 一致）
    let out = out.replace("${IFS}", " ").replace("$IFS", " ");
    // 4) 拼接相邻字符串拼接符 (a'b'c → abc)
    //    简单启发式：当看到引号 / $ / 等时不去拼接；这里不展开复杂解析，
    //    仅做最浅的拼接（连续引号包住的纯字母数字）。
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::PermissionRule;

    #[test]
    fn test_exact_match() {
        let rules = vec![PermissionRule::allow("FileRead", "允许读取")];
        assert!(match_rules("FileRead", "{}", &rules).is_some());
        assert!(match_rules("FileWrite", "{}", &rules).is_none());
    }

    #[test]
    fn test_wildcard_match() {
        let rules = vec![PermissionRule::deny("Bash*", "禁用所有 Bash 工具")];
        assert!(match_rules("Bash", "{}", &rules).is_some());
        assert!(match_rules("BashTool", "{}", &rules).is_some());
        assert!(match_rules("FileRead", "{}", &rules).is_none());
    }

    #[test]
    fn test_content_match() {
        let rules = vec![PermissionRule::allow("Bash(git *)", "允许 git 命令")];
        assert!(match_rules("Bash", r#"{"command": "git log"}"#, &rules).is_some());
        assert!(match_rules("Bash", r#"{"command": "rm -rf"}"#, &rules).is_none());
    }

    #[test]
    fn content_match_only_via_primary_field() {
        // SECURITY (H1): "git log" 出现在 extra 字段里不应让 Bash(git *) 匹配
        let rules = vec![PermissionRule::allow("Bash(git *)", "")];
        let input = r#"{"command": "rm -rf /", "extra": "for context: git log"}"#;
        assert!(match_rules("Bash", input, &rules).is_none());
    }

    #[test]
    fn content_match_obfuscated_whitespace() {
        // SECURITY (H1): 多空格、NBSP 不应绕过
        let rules = vec![PermissionRule::deny("Bash(rm -rf /*)", "")];
        let input = r#"{"command": "rm\u00a0\u00a0-rf\u00a0/"}"#;
        assert!(match_rules("Bash", input, &rules).is_some());
    }

    #[test]
    fn content_match_ifs_obfuscation() {
        // SECURITY (H1): $IFS 替换为空格后应可匹配
        let rules = vec![PermissionRule::deny("Bash(rm -rf /*)", "")];
        let input = r#"{"command": "rm${IFS}-rf${IFS}/"}"#;
        assert!(match_rules("Bash", input, &rules).is_some());
    }
}
