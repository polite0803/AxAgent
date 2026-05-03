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
