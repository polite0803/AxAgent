// SPDX-License-Identifier: AGPL-3.0-only

//! Permission enforcement layer that gates tool execution based on `PermissionPolicy`.

use crate::runtime_types::permissions::{PermissionMode, PermissionOutcome, PermissionPolicy};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome")]
pub enum EnforcementResult {
    Allowed,
    AllowedWithAudit { outside_workspace: bool, sensitive_path: bool },
    Denied { tool: String, active_mode: String, required_mode: String, reason: String },
}

pub trait PermissionChecker: Send + Sync {
    fn check(&self, tool_name: &str, input: &str) -> EnforcementResult;
    fn is_allowed(&self, tool_name: &str, input: &str) -> bool;
    fn check_file_write(&self, path: &str, workspace_root: &str) -> EnforcementResult;
    fn check_bash(&self, command: &str) -> EnforcementResult;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermissionEnforcer {
    policy: PermissionPolicy,
}

impl PermissionEnforcer {
    pub fn new(policy: PermissionPolicy) -> Self {
        Self { policy }
    }

    pub fn check(&self, tool_name: &str, input: &str) -> EnforcementResult {
        if self.policy.active_mode() == PermissionMode::Prompt {
            return EnforcementResult::Allowed;
        }
        let outcome = self.policy.authorize(tool_name, input, None);
        match outcome {
            PermissionOutcome::Allow => EnforcementResult::Allowed,
            PermissionOutcome::Deny { reason } => {
                let active_mode = self.policy.active_mode();
                let required_mode = self.policy.required_mode_for(tool_name);
                EnforcementResult::Denied {
                    tool: tool_name.to_owned(),
                    active_mode: active_mode.as_str().to_owned(),
                    required_mode: required_mode.as_str().to_owned(),
                    reason,
                }
            },
        }
    }

    pub fn is_allowed(&self, tool_name: &str, input: &str) -> bool {
        matches!(self.check(tool_name, input), EnforcementResult::Allowed)
    }

    pub fn check_with_required_mode(
        &self,
        tool_name: &str,
        input: &str,
        required_mode: PermissionMode,
    ) -> EnforcementResult {
        if self.policy.active_mode() == PermissionMode::Prompt {
            return EnforcementResult::Allowed;
        }
        let active_mode = self.policy.active_mode();
        if active_mode >= required_mode {
            return EnforcementResult::Allowed;
        }
        EnforcementResult::Denied {
            tool: tool_name.to_owned(),
            active_mode: active_mode.as_str().to_owned(),
            required_mode: required_mode.as_str().to_owned(),
            reason: format!(
                "'{tool_name}' with input '{input}' requires '{}' permission, but current mode is '{}'",
                required_mode.as_str(),
                active_mode.as_str()
            ),
        }
    }

    pub fn active_mode(&self) -> PermissionMode {
        self.policy.active_mode()
    }

    pub fn check_file_write(&self, path: &str, workspace_root: &str) -> EnforcementResult {
        let mode = self.policy.active_mode();
        match mode {
            PermissionMode::ReadOnly => EnforcementResult::Denied {
                tool: "write_file".to_owned(),
                active_mode: mode.as_str().to_owned(),
                required_mode: PermissionMode::WorkspaceWrite.as_str().to_owned(),
                reason: format!("file writes are not allowed in '{}' mode", mode.as_str()),
            },
            PermissionMode::WorkspaceWrite => {
                if is_within_workspace(path, workspace_root) {
                    EnforcementResult::Allowed
                } else {
                    EnforcementResult::Denied {
                        tool: "write_file".to_owned(),
                        active_mode: mode.as_str().to_owned(),
                        required_mode: PermissionMode::DangerFullAccess.as_str().to_owned(),
                        reason: format!(
                            "path '{}' is outside workspace root '{}'",
                            path, workspace_root
                        ),
                    }
                }
            },
            PermissionMode::Allow => EnforcementResult::Allowed,
            PermissionMode::DangerFullAccess => {
                let outside = !is_within_workspace(path, workspace_root);
                let sensitive = is_sensitive_path(path);
                EnforcementResult::AllowedWithAudit {
                    outside_workspace: outside,
                    sensitive_path: sensitive,
                }
            },
            PermissionMode::Prompt => EnforcementResult::Denied {
                tool: "write_file".to_owned(),
                active_mode: mode.as_str().to_owned(),
                required_mode: PermissionMode::WorkspaceWrite.as_str().to_owned(),
                reason: "file write requires confirmation in prompt mode".to_owned(),
            },
        }
    }

    pub fn check_bash(&self, command: &str) -> EnforcementResult {
        let mode = self.policy.active_mode();
        match mode {
            PermissionMode::ReadOnly => {
                if is_read_only_command(command) {
                    EnforcementResult::Allowed
                } else {
                    EnforcementResult::Denied {
                        tool: "bash".to_owned(),
                        active_mode: mode.as_str().to_owned(),
                        required_mode: PermissionMode::WorkspaceWrite.as_str().to_owned(),
                        reason: format!(
                            "command may modify state; not allowed in '{}' mode",
                            mode.as_str()
                        ),
                    }
                }
            },
            PermissionMode::Prompt => EnforcementResult::Denied {
                tool: "bash".to_owned(),
                active_mode: mode.as_str().to_owned(),
                required_mode: PermissionMode::DangerFullAccess.as_str().to_owned(),
                reason: "bash requires confirmation in prompt mode".to_owned(),
            },
            _ => EnforcementResult::Allowed,
        }
    }
}

impl PermissionChecker for PermissionEnforcer {
    fn check(&self, tool_name: &str, input: &str) -> EnforcementResult {
        self.check(tool_name, input)
    }
    fn is_allowed(&self, tool_name: &str, input: &str) -> bool {
        self.is_allowed(tool_name, input)
    }
    fn check_file_write(&self, path: &str, workspace_root: &str) -> EnforcementResult {
        self.check_file_write(path, workspace_root)
    }
    fn check_bash(&self, command: &str) -> EnforcementResult {
        self.check_bash(command)
    }
}

fn is_within_workspace(path: &str, workspace_root: &str) -> bool {
    if path.is_empty() || path.contains('\0') {
        return false;
    }
    let canonical = match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let canonical_root = match std::fs::canonicalize(workspace_root) {
        Ok(p) => p,
        Err(_) => return false,
    };
    canonical.starts_with(&canonical_root) || canonical == canonical_root
}

fn is_sensitive_path(path: &str) -> bool {
    let sensitive_prefixes = [
        "/etc/",
        "/boot/",
        "/sys/",
        "/proc/",
        "/dev/",
        "C:\\Windows\\",
        "C:\\Windows\\System32\\",
        "/System/Library/",
        "/Library/System/",
    ];
    sensitive_prefixes.iter().any(|prefix| path.starts_with(prefix))
}

fn is_read_only_command(command: &str) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return false;
    }
    let mut chars = trimmed.chars();
    let mut prev = '\0';
    let mut in_single = false;
    let mut in_double = false;
    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ';' | '|' | '`' if !in_single && !in_double => return false,
            '&' if !in_single && !in_double && prev == '&' => return false,
            '$' if !in_single => {
                if chars.clone().next() == Some('(') {
                    return false;
                }
            },
            _ => {},
        }
        prev = c;
    }
    let first_token =
        trimmed.split_whitespace().next().unwrap_or("").rsplit('/').next().unwrap_or("");
    let is_whitelisted = matches!(
        first_token,
        "cat"
            | "head"
            | "tail"
            | "less"
            | "more"
            | "wc"
            | "ls"
            | "find"
            | "grep"
            | "rg"
            | "awk"
            | "sed"
            | "echo"
            | "printf"
            | "which"
            | "where"
            | "whoami"
            | "pwd"
            | "env"
            | "printenv"
            | "date"
            | "cal"
            | "df"
            | "du"
            | "free"
            | "uptime"
            | "uname"
            | "file"
            | "stat"
            | "diff"
            | "sort"
            | "uniq"
            | "tr"
            | "cut"
            | "paste"
            | "xargs"
            | "test"
            | "true"
            | "false"
            | "type"
            | "readlink"
            | "realpath"
            | "basename"
            | "dirname"
            | "sha256sum"
            | "md5sum"
            | "b3sum"
            | "xxd"
            | "hexdump"
            | "od"
            | "strings"
            | "tree"
            | "jq"
            | "yq"
    );
    is_whitelisted
        && !command.contains("-i ")
        && !command.contains("--in-place")
        && !command.contains(" > ")
        && !command.contains(" >> ")
}
