//! 验证 Agent — 后台异步验证代码实现与计划的一致性
//! 只使用只读工具：FileRead, Grep, Glob, Bash(只读)

// 类型引用从 axagent_runtime 按需导入

/// 验证上下文 — 包含验证所需的计划信息和变更范围
pub struct VerificationContext {
    /// 计划摘要
    pub plan_summary: String,
    /// 修改的文件列表
    pub changed_files: Vec<String>,
    /// 可选的测试命令
    pub test_command: Option<String>,
    /// 关联的会话 ID
    pub session_id: String,
}

/// 验证结果
pub struct VerificationResult {
    /// 验证是否通过
    pub passed: bool,
    /// 发现的问题列表
    pub issues: Vec<String>,
    /// 改进建议列表
    pub suggestions: Vec<String>,
    /// 验证摘要
    pub summary: String,
}

/// 验证 Agent — 独立的后台验证器，只读不写
pub struct VerificationAgent;

impl VerificationAgent {
    /// 推荐的只读工具集
    pub fn allowed_tools() -> Vec<&'static str> {
        vec!["FileRead", "Grep", "Glob", "Bash", "TodoWrite"]
    }

    /// 禁止的写入工具
    pub fn disallowed_tools() -> Vec<&'static str> {
        vec!["FileWrite", "FileEdit"]
    }

    /// 生成验证用的 system prompt
    pub fn build_system_prompt(context: &VerificationContext) -> String {
        format!(
            "你是一个代码验证专家。请验证以下实现是否与计划一致。\
             \n\n## 计划摘要\n{}\n\n## 修改的文件\n{}\n\n\
             ## 验证步骤\n\
             1. 读取每个修改的文件，检查实现是否完整\n\
             2. 检查是否有遗漏的边界情况\n\
             3. 如果有测试命令，运行测试确认通过\n\
             4. 输出验证报告（通过/失败 + 问题列表）\n\n\
             ## 规则\n\
             - 只读操作，不要修改任何文件\n\
             - 直接返回验证结果，不要继续对话",
            context.plan_summary,
            context.changed_files.join("\n"),
        )
    }

    /// 创建简单的验证结果
    pub fn quick_result(passed: bool, summary: &str) -> VerificationResult {
        VerificationResult {
            passed,
            issues: Vec::new(),
            suggestions: Vec::new(),
            summary: summary.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_context_creation() {
        let ctx = VerificationContext {
            plan_summary: "implement feature X".to_string(),
            changed_files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            test_command: Some("cargo test".to_string()),
            session_id: "session-1".to_string(),
        };
        assert_eq!(ctx.plan_summary, "implement feature X");
        assert_eq!(ctx.changed_files.len(), 2);
        assert!(ctx.test_command.is_some());
        assert_eq!(ctx.session_id, "session-1");
    }

    #[test]
    fn test_verification_context_no_test_command() {
        let ctx = VerificationContext {
            plan_summary: "refactor module".to_string(),
            changed_files: vec!["src/utils.rs".to_string()],
            test_command: None,
            session_id: "session-2".to_string(),
        };
        assert!(ctx.test_command.is_none());
    }

    #[test]
    fn test_verification_result_passed() {
        let result = VerificationResult {
            passed: true,
            issues: Vec::new(),
            suggestions: vec!["consider adding more tests".to_string()],
            summary: "all checks passed".to_string(),
        };
        assert!(result.passed);
        assert!(result.issues.is_empty());
        assert_eq!(result.suggestions.len(), 1);
        assert_eq!(result.summary, "all checks passed");
    }

    #[test]
    fn test_verification_result_failed() {
        let result = VerificationResult {
            passed: false,
            issues: vec![
                "missing error handling".to_string(),
                "unused import".to_string(),
            ],
            suggestions: Vec::new(),
            summary: "verification failed".to_string(),
        };
        assert!(!result.passed);
        assert_eq!(result.issues.len(), 2);
        assert!(result.suggestions.is_empty());
    }

    #[test]
    fn test_verification_agent_allowed_tools() {
        let tools = VerificationAgent::allowed_tools();
        assert!(tools.contains(&"FileRead"));
        assert!(tools.contains(&"Grep"));
        assert!(tools.contains(&"Glob"));
        assert!(tools.contains(&"Bash"));
        assert!(tools.contains(&"TodoWrite"));
    }

    #[test]
    fn test_verification_agent_disallowed_tools() {
        let tools = VerificationAgent::disallowed_tools();
        assert!(tools.contains(&"FileWrite"));
        assert!(tools.contains(&"FileEdit"));
    }

    #[test]
    fn test_verification_agent_allowed_tools_no_write() {
        let allowed = VerificationAgent::allowed_tools();
        let disallowed = VerificationAgent::disallowed_tools();
        for tool in &disallowed {
            assert!(!allowed.contains(tool));
        }
    }

    #[test]
    fn test_verification_agent_build_system_prompt() {
        let ctx = VerificationContext {
            plan_summary: "add login feature".to_string(),
            changed_files: vec!["src/auth.rs".to_string(), "src/routes.rs".to_string()],
            test_command: Some("cargo test auth".to_string()),
            session_id: "session-3".to_string(),
        };

        let prompt = VerificationAgent::build_system_prompt(&ctx);
        assert!(prompt.contains("add login feature"));
        assert!(prompt.contains("src/auth.rs"));
        assert!(prompt.contains("src/routes.rs"));
        assert!(prompt.contains("验证步骤"));
    }

    #[test]
    fn test_verification_agent_build_system_prompt_no_test_command() {
        let ctx = VerificationContext {
            plan_summary: "refactor code".to_string(),
            changed_files: vec!["src/main.rs".to_string()],
            test_command: None,
            session_id: "session-4".to_string(),
        };

        let prompt = VerificationAgent::build_system_prompt(&ctx);
        assert!(prompt.contains("refactor code"));
        assert!(prompt.contains("src/main.rs"));
    }

    #[test]
    fn test_verification_agent_quick_result_passed() {
        let result = VerificationAgent::quick_result(true, "everything looks good");
        assert!(result.passed);
        assert_eq!(result.summary, "everything looks good");
        assert!(result.issues.is_empty());
        assert!(result.suggestions.is_empty());
    }

    #[test]
    fn test_verification_agent_quick_result_failed() {
        let result = VerificationAgent::quick_result(false, "critical issues found");
        assert!(!result.passed);
        assert_eq!(result.summary, "critical issues found");
        assert!(result.issues.is_empty());
        assert!(result.suggestions.is_empty());
    }

    #[test]
    fn test_verification_context_empty_changed_files() {
        let ctx = VerificationContext {
            plan_summary: "no changes".to_string(),
            changed_files: Vec::new(),
            test_command: None,
            session_id: "session-5".to_string(),
        };
        assert!(ctx.changed_files.is_empty());

        let prompt = VerificationAgent::build_system_prompt(&ctx);
        assert!(prompt.contains("no changes"));
    }

    #[test]
    fn test_verification_context_multiple_changed_files() {
        let ctx = VerificationContext {
            plan_summary: "multi-file refactor".to_string(),
            changed_files: vec![
                "src/a.rs".to_string(),
                "src/b.rs".to_string(),
                "src/c.rs".to_string(),
                "src/d.rs".to_string(),
            ],
            test_command: Some("cargo test".to_string()),
            session_id: "session-6".to_string(),
        };

        let prompt = VerificationAgent::build_system_prompt(&ctx);
        assert!(prompt.contains("src/a.rs"));
        assert!(prompt.contains("src/d.rs"));
    }

    #[test]
    fn test_verification_result_with_issues_and_suggestions() {
        let result = VerificationResult {
            passed: false,
            issues: vec![
                "missing null check".to_string(),
                "potential overflow".to_string(),
            ],
            suggestions: vec![
                "add null check before dereference".to_string(),
                "use checked arithmetic".to_string(),
                "add unit tests for edge cases".to_string(),
            ],
            summary: "2 issues found".to_string(),
        };
        assert!(!result.passed);
        assert_eq!(result.issues.len(), 2);
        assert_eq!(result.suggestions.len(), 3);
    }
}
