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
            dangerous_patterns: vec![
                ";".to_string(),
                "|".to_string(),
                "&".to_string(),
                "`".to_string(),
                "$(".to_string(),
                "${".to_string(),
                "\n".to_string(),
                "\r".to_string(),
                ">".to_string(),
                "<".to_string(),
                ">>".to_string(),
                "<<".to_string(),
                "2>".to_string(),
                "2>&1".to_string(),
                "&&".to_string(),
                "||".to_string(),
                "(".to_string(),
                ")".to_string(),
                "{".to_string(),
                "}".to_string(),
                "[".to_string(),
                "]".to_string(),
                "#".to_string(),
                "~".to_string(),
                "%".to_string(),
                "^".to_string(),
                "\\".to_string(),
            ],
            max_length: 10000,
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
                warnings: vec![format!("Command exceeds maximum length of {} bytes", self.max_length)],
                dangerous_patterns: vec![],
            };
        }

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
            sanitized: if is_safe { Some(command.to_string()) } else { None },
            warnings,
            dangerous_patterns,
        }
    }

    pub fn sanitize(&self, command: &str) -> String {
        let mut result = command.to_string();

        for pattern in &[";", "|", "&", "`", "$", ">", "<", "\n", "\r"] {
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

pub fn validate_command(command: &str) -> Result<(), String> {
    let validator = CommandValidator::new();
    let result = validator.validate(command);

    if !result.is_safe {
        return Err(format!(
            "Command contains dangerous patterns: {:?}",
            result.dangerous_patterns
        ));
    }

    for blocked in get_platform_blocked_commands() {
        if command.contains(blocked) {
            return Err(format!(
                "Command blocked for security reasons: {}",
                blocked
            ));
        }
    }

    Ok(())
}
