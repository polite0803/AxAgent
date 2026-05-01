#[derive(Debug, Clone)]
pub struct CommandValidationResult {
    pub is_safe: bool,
    pub sanitized: Option<String>,
    pub warnings: Vec<String>,
    pub dangerous_patterns: Vec<String>,
}

pub struct CommandValidator {
    dangerous_patterns: Vec<String>,
    max_length: usize,
}

impl Default for CommandValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandValidator {
    pub fn new() -> Self {
        Self {
            // 仅保留明确的命令注入向量，移除对合法 Shell 字符的过度封锁
            // (、)、{、}、[、]、#、~、%、^、\ 等字符在常规命令中大量出现，不应误拒
            dangerous_patterns: vec![
                "$(".to_string(), // 命令替换
                "${".to_string(), // 变量展开中的命令替换
                "`".to_string(),  // 反引号命令替换
                "\n".to_string(), // 换行符可用于命令分隔
                "\r".to_string(), // 回车符可用于命令注入
            ],
            max_length: 4096, // 4KB 限制，防止 DoS
        }
    }

    pub fn with_custom_patterns(mut self, patterns: Vec<String>) -> Self {
        self.dangerous_patterns = patterns;
        self
    }

    pub fn with_max_length(mut self, max: usize) -> Self {
        self.max_length = max;
        self
    }

    pub fn validate(&self, command: &str) -> CommandValidationResult {
        let mut warnings = Vec::new();
        let mut dangerous_patterns = Vec::new();

        if command.len() > self.max_length {
            return CommandValidationResult {
                is_safe: false,
                sanitized: None,
                warnings: vec![format!(
                    "Command exceeds maximum length of {} bytes",
                    self.max_length
                )],
                dangerous_patterns: vec![],
            };
        }

        // Phase 1: AST-level security audit via shell_parser
        if let Ok(parsed) = crate::shell_parser::parse_shell(command) {
            let audit_warnings = crate::shell_parser::audit_shell(&parsed);
            for warning in &audit_warnings {
                let severity = warning.severity();
                let desc = warning.description();
                match severity {
                    crate::shell_parser::SecuritySeverity::Critical => {
                        dangerous_patterns.push(format!("CRITICAL: {desc}"));
                        warnings.push(format!("CRITICAL: {desc}"));
                    },
                    crate::shell_parser::SecuritySeverity::High => {
                        dangerous_patterns.push(format!("HIGH: {desc}"));
                        warnings.push(format!("HIGH: {desc}"));
                    },
                    crate::shell_parser::SecuritySeverity::Medium => {
                        warnings.push(format!("MEDIUM: {desc}"));
                    },
                    crate::shell_parser::SecuritySeverity::Low => {
                        warnings.push(format!("LOW: {desc}"));
                    },
                }
            }
        }

        // Phase 2: Pattern-based validation (fallback and complementary)
        for pattern in &self.dangerous_patterns {
            if command.contains(pattern) {
                dangerous_patterns.push(pattern.clone());
                warnings.push(format!("Dangerous pattern '{}' found", pattern));
            }
        }

        if command.contains('%') && command.contains(';') {
            dangerous_patterns.push("url-encoded-injection".to_string());
            warnings.push("Potential URL-encoded command injection".to_string());
        }

        let is_safe = dangerous_patterns.is_empty();

        CommandValidationResult {
            is_safe,
            sanitized: if is_safe {
                Some(command.to_string())
            } else {
                None
            },
            warnings,
            dangerous_patterns,
        }
    }

    pub fn sanitize(&self, command: &str) -> String {
        // 仅替换明确的命令注入字符，保留合法 Shell 语法
        let mut result = command.to_string();

        for pattern in &["`", "$(", "\n", "\r"] {
            result = result.replace(pattern, " ");
        }

        result.trim().to_string()
    }
}

pub fn get_platform_blocked_commands() -> Vec<&'static str> {
    if cfg!(windows) {
        vec![
            "del /s /q C:\\",
            "rd /s /q C:\\",
            "format ",
            "diskpart",
            "net user ",
            "net localgroup ",
            "reg delete ",
            "powershell -enc",
            "cmd /c del",
            "taskkill /f",
        ]
    } else {
        vec![
            "rm -rf /",
            "mkfs",
            "dd if=",
            ":(){:|:&};",
            "chmod -R 777 /",
            "chown -R ",
        ]
    }
}

/// 安全命令前缀白名单：只允许以已知安全命令开头的命令执行
/// 白名单（allowlist）比黑名单（blocklist）更安全可靠
pub fn is_command_allowed(command: &str) -> bool {
    let allowed_prefixes: &[&str] = &[
        "ls", "cat", "head", "tail", "grep", "find", "wc", "echo", "date", "whoami", "pwd", "env",
        "printenv", "git", "npm", "node", "python", "python3", "cargo", "rustc", "gcc", "g++",
        "make", "cmake", "docker", "curl", "wget", "ssh", "scp", "tar", "zip", "unzip", "df", "du",
        "ps", "top", "kill", "ping", "mkdir", "touch", "cp", "mv", "rm", "chmod", "sort", "uniq",
        "awk", "sed", "cut", "tr", "npx", "pnpm", "yarn",
    ];
    let first_word = command.split_whitespace().next().unwrap_or("");
    allowed_prefixes.contains(&first_word)
}

pub fn validate_command(command: &str) -> Result<(), String> {
    // 第一层：命令前缀白名单（allowlist），只允许已知安全命令
    if !is_command_allowed(command) {
        return Err(format!(
            "命令 '{}' 不在允许列表中。安全策略仅允许执行常用开发工具命令。",
            command.split_whitespace().next().unwrap_or("")
        ));
    }

    let validator = CommandValidator::new();
    let result = validator.validate(command);

    if !result.is_safe {
        return Err(format!(
            "命令包含危险模式: {:?}。如果这是合法命令，请使用 shell_parser AST 审计白名单。",
            result.dangerous_patterns
        ));
    }

    for blocked in get_platform_blocked_commands() {
        if command.contains(blocked) {
            return Err(format!("命令因安全原因被封锁: {}", blocked));
        }
    }

    Ok(())
}
