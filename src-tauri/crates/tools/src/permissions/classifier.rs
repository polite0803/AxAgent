//! AI 安全分类器
//!
//! 使用轻量级启发式分析工具输入，判断风险等级。

/// 分类结果
#[derive(Debug, Clone)]
pub struct ClassifierResult {
    /// 风险等级
    pub risk_level: RiskLevel,
    /// 解释
    pub reason: String,
    /// 是否建议允许
    pub suggest_allow: bool,
    /// 是否建议拒绝
    pub suggest_deny: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

/// 启发式分类器（不依赖 LLM，快速判断）
pub struct HeuristicClassifier;

impl HeuristicClassifier {
    /// 分类 Bash 命令的安全风险
    pub fn classify_bash(command: &str) -> ClassifierResult {
        let lower = command.to_lowercase();

        // 关键危险模式
        let critical_patterns = [
            "rm -rf /",
            "rm -rf /*",
            ":(){ :|:& };:", // fork bomb
            "mkfs.",
            "dd if=",
            "> /dev/sda",
            "/dev/null",
            "chmod 777 /",
            "chmod -R 777 /",
        ];

        let high_patterns = [
            "sudo ",
            "eval ",
            "curl ",
            "wget ",
            ".bash_profile",
            ".bashrc",
            "/etc/",
            "systemctl ",
            "kill -9",
            "pkill",
            "reboot",
            "shutdown",
            "docker ",
            "kubectl ",
        ];

        let medium_patterns = [
            "rm ",
            "mv ",
            "pip install",
            "npm install -g",
            "cargo install",
            "chmod ",
            "chown ",
            "git push",
            "git reset --hard",
        ];

        // Critical 检查
        for pattern in &critical_patterns {
            if lower.contains(pattern) {
                return ClassifierResult {
                    risk_level: RiskLevel::Critical,
                    reason: format!("检测到高危命令模式: {}", pattern),
                    suggest_allow: false,
                    suggest_deny: true,
                };
            }
        }

        // High 检查
        for pattern in &high_patterns {
            if lower.contains(pattern) {
                if *pattern == "curl " || *pattern == "wget " {
                    // curl/wget 如果目标明确是安全的可以放过
                    if lower.contains(" | bash") || lower.contains(" | sh") {
                        return ClassifierResult {
                            risk_level: RiskLevel::Critical,
                            reason: "检测到 curl/wget piped to shell 模式".into(),
                            suggest_allow: false,
                            suggest_deny: true,
                        };
                    }
                }
                return ClassifierResult {
                    risk_level: RiskLevel::High,
                    reason: format!("检测到高风险命令模式: {}", pattern),
                    suggest_allow: false,
                    suggest_deny: false, // 需要用户确认
                };
            }
        }

        // Medium 检查
        for pattern in &medium_patterns {
            if lower.contains(pattern) {
                return ClassifierResult {
                    risk_level: RiskLevel::Medium,
                    reason: format!("检测到中风险命令模式: {}", pattern),
                    suggest_allow: false,
                    suggest_deny: false,
                };
            }
        }

        // 安全命令白名单
        let safe_patterns = [
            "git status",
            "git diff",
            "git log",
            "git branch",
            "ls ",
            "dir",
            "cat ",
            "head ",
            "tail ",
            "wc ",
            "find ",
            "grep ",
            "echo ",
            "pwd",
            "which ",
            "type ",
            "cargo build",
            "cargo test",
            "cargo check",
            "cargo clippy",
            "npm test",
            "npm run build",
            "npm run dev",
            "python ",
            "node ",
            "rustc ",
            "go build",
            "cargo fmt",
            "rustfmt",
        ];

        for pattern in &safe_patterns {
            if lower.starts_with(pattern) || lower.contains(pattern) {
                return ClassifierResult {
                    risk_level: RiskLevel::Safe,
                    reason: format!("检测到安全命令: {}", pattern),
                    suggest_allow: true,
                    suggest_deny: false,
                };
            }
        }

        // 默认低风险
        ClassifierResult {
            risk_level: RiskLevel::Low,
            reason: "命令未匹配合规模式，请人工审核".into(),
            suggest_allow: false,
            suggest_deny: false,
        }
    }

    /// 分类文件路径的安全性
    pub fn classify_file_path(path: &str) -> ClassifierResult {
        let dangerous_paths = [
            "/etc/passwd",
            "/etc/shadow",
            "/etc/sudoers",
            "/etc/ssh/",
            "~/.ssh/",
            "C:\\Windows\\System32\\",
            "/boot/",
            "/sys/",
            "/proc/",
        ];

        for dp in &dangerous_paths {
            if path.contains(dp) {
                return ClassifierResult {
                    risk_level: RiskLevel::High,
                    reason: format!("路径包含系统文件: {}", dp),
                    suggest_allow: false,
                    suggest_deny: false,
                };
            }
        }

        ClassifierResult {
            risk_level: RiskLevel::Safe,
            reason: "路径安全".into(),
            suggest_allow: true,
            suggest_deny: false,
        }
    }
}
