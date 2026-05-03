//! 权限规则匹配引擎

use super::{PermissionRule, RulePattern};

/// 匹配工具名与规则列表，返回第一个匹配的规则。
///
/// 匹配策略：
/// - 精确匹配：规则 pattern 等于工具名
/// - 前缀匹配：规则 pattern 以 `*` 结尾，匹配工具名前缀
/// - 内容匹配：规则 pattern 形如 `ToolName(content_pattern)`，匹配工具名 + 输入内容
pub fn match_rules<'a>(
    tool_name: &str,
    input: &str,
    rules: &'a [PermissionRule],
) -> Option<&'a PermissionRule> {
    rules
        .iter()
        .find(|&rule| match_pattern(&rule.pattern, tool_name, input))
        .map(|v| v as _)
}

/// 匹配单个规则模式
fn match_pattern(pattern: &RulePattern, tool_name: &str, input: &str) -> bool {
    let p = &pattern.pattern;

    // 内容匹配: "ToolName(content_pattern)"
    if p.contains('(') && p.ends_with(')') {
        if let Some(paren_idx) = p.find('(') {
            let required_tool = &p[..paren_idx];
            let content_pattern = &p[paren_idx + 1..p.len() - 1];

            if !match_simple_pattern(required_tool, tool_name) {
                return false;
            }

            // 如果内容模式为空，匹配所有
            if content_pattern.is_empty() {
                return true;
            }

            // 检查输入内容
            return match_content(content_pattern, input);
        }
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
fn match_content(pattern: &str, input: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    // 简单的子串匹配
    if let Some(prefix) = pattern.strip_suffix('*') {
        return input.contains(prefix);
    }
    input.contains(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::{PermissionBehavior, PermissionRule};

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
}
