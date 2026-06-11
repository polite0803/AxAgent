// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub allowed_paths: Vec<PathBuf>,
    pub denied_paths: Vec<PathBuf>,
    pub allowed_commands: Vec<String>,
    pub denied_commands: Vec<String>,
    pub network_enabled: bool,
    pub max_memory_mb: Option<u32>,
    pub max_cpu_time_secs: Option<u32>,
    pub max_output_bytes: Option<u64>,
    pub env_whitelist: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            allowed_paths: Vec::new(),
            denied_paths: Vec::new(),
            allowed_commands: Vec::new(),
            denied_commands: Vec::new(),
            network_enabled: false,
            max_memory_mb: Some(512),
            max_cpu_time_secs: Some(60),
            max_output_bytes: Some(1024 * 1024),
            env_whitelist: vec!["PATH".to_string(), "HOME".to_string(), "TEMP".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxViolation {
    pub violation_type: SandboxViolationType,
    pub resource: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SandboxViolationType {
    PathAccessDenied,
    CommandDenied,
    NetworkDenied,
    MemoryLimitExceeded,
    CpuTimeExceeded,
    OutputLimitExceeded,
    EnvVarDenied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    pub allowed: bool,
    pub violations: Vec<SandboxViolation>,
}

pub struct SecuritySandbox {
    config: SandboxConfig,
    platform: SandboxPlatform,
}

#[derive(Debug, Clone, Copy)]
pub enum SandboxPlatform {
    Linux,
    Windows,
    MacOS,
    Unknown,
}

impl SecuritySandbox {
    pub fn new(config: SandboxConfig) -> Self {
        let platform = if cfg!(target_os = "linux") {
            SandboxPlatform::Linux
        } else if cfg!(target_os = "windows") {
            SandboxPlatform::Windows
        } else if cfg!(target_os = "macos") {
            SandboxPlatform::MacOS
        } else {
            SandboxPlatform::Unknown
        };

        Self { config, platform }
    }

    pub fn with_default_config() -> Self {
        Self::new(SandboxConfig::default())
    }

    pub fn check_path_access(&self, path: &std::path::Path) -> SandboxResult {
        let mut violations = Vec::new();

        for denied in &self.config.denied_paths {
            if path.starts_with(denied) {
                violations.push(SandboxViolation {
                    violation_type: SandboxViolationType::PathAccessDenied,
                    resource: path.display().to_string(),
                    message: format!("Path '{}' is in denied list", path.display()),
                });
                return SandboxResult {
                    allowed: false,
                    violations,
                };
            }
        }

        if !self.config.allowed_paths.is_empty() {
            let is_allowed = self
                .config
                .allowed_paths
                .iter()
                .any(|allowed| path.starts_with(allowed));
            if !is_allowed {
                violations.push(SandboxViolation {
                    violation_type: SandboxViolationType::PathAccessDenied,
                    resource: path.display().to_string(),
                    message: format!("Path '{}' is not in allowed list", path.display()),
                });
                return SandboxResult {
                    allowed: false,
                    violations,
                };
            }
        }

        SandboxResult {
            allowed: true,
            violations,
        }
    }

    pub fn check_command(&self, command: &str) -> SandboxResult {
        let base_cmd = command.split_whitespace().next().unwrap_or(command);

        for denied in &self.config.denied_commands {
            if base_cmd == *denied {
                return SandboxResult {
                    allowed: false,
                    violations: vec![SandboxViolation {
                        violation_type: SandboxViolationType::CommandDenied,
                        resource: command.to_string(),
                        message: format!("Command '{}' is denied", base_cmd),
                    }],
                };
            }
        }

        if !self.config.allowed_commands.is_empty()
            && !self.config.allowed_commands.contains(&base_cmd.to_string())
        {
            return SandboxResult {
                allowed: false,
                violations: vec![SandboxViolation {
                    violation_type: SandboxViolationType::CommandDenied,
                    resource: command.to_string(),
                    message: format!("Command '{}' is not in allowed list", base_cmd),
                }],
            };
        }

        SandboxResult {
            allowed: true,
            violations: Vec::new(),
        }
    }

    pub fn check_network(&self) -> SandboxResult {
        if !self.config.network_enabled {
            SandboxResult {
                allowed: false,
                violations: vec![SandboxViolation {
                    violation_type: SandboxViolationType::NetworkDenied,
                    resource: "network".to_string(),
                    message: "Network access is disabled in sandbox".to_string(),
                }],
            }
        } else {
            SandboxResult {
                allowed: true,
                violations: Vec::new(),
            }
        }
    }

    pub fn check_env_var(&self, var_name: &str) -> SandboxResult {
        if !self.config.env_whitelist.is_empty()
            && !self.config.env_whitelist.contains(&var_name.to_string())
        {
            return SandboxResult {
                allowed: false,
                violations: vec![SandboxViolation {
                    violation_type: SandboxViolationType::EnvVarDenied,
                    resource: var_name.to_string(),
                    message: format!("Environment variable '{}' is not whitelisted", var_name),
                }],
            };
        }
        SandboxResult {
            allowed: true,
            violations: Vec::new(),
        }
    }

    /// 验证当前进程环境变量是否符合白名单
    /// 返回被拒绝的环境变量列表
    pub fn validate_environment(&self) -> Vec<String> {
        let mut denied = Vec::new();
        for (key, _value) in std::env::vars() {
            if !self
                .config
                .env_whitelist
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(&key))
            {
                denied.push(key);
            }
        }
        if !denied.is_empty() {
            tracing::warn!("Non-whitelisted environment variables detected: {:?}", denied);
        }
        denied
    }

    pub fn get_platform_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        match self.platform {
            SandboxPlatform::Linux => {
                recommendations
                    .push("Consider using seccomp-bpf for syscall filtering".to_string());
                recommendations
                    .push("Consider using namespaces for filesystem isolation".to_string());
                recommendations.push("Consider using cgroups for resource limits".to_string());
            },
            SandboxPlatform::Windows => {
                recommendations
                    .push("Consider using AppContainer for process isolation".to_string());
                recommendations.push("Consider using Job Objects for resource limits".to_string());
            },
            SandboxPlatform::MacOS => {
                recommendations
                    .push("Consider using sandbox-exec for process isolation".to_string());
                recommendations
                    .push("Consider using Seatbelt profiles for access control".to_string());
            },
            SandboxPlatform::Unknown => {
                recommendations.push("Platform-specific sandboxing not available".to_string());
            },
        }

        recommendations
    }

    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_access_allowed() {
        let config = SandboxConfig {
            allowed_paths: vec![PathBuf::from("/workspace")],
            ..Default::default()
        };
        let sandbox = SecuritySandbox::new(config);
        let result = sandbox.check_path_access(PathBuf::from("/workspace/file.txt").as_path());
        assert!(result.allowed);
    }

    #[test]
    fn test_path_access_denied() {
        let config = SandboxConfig {
            allowed_paths: vec![PathBuf::from("/workspace")],
            ..Default::default()
        };
        let sandbox = SecuritySandbox::new(config);
        let result = sandbox.check_path_access(PathBuf::from("/etc/passwd").as_path());
        assert!(!result.allowed);
    }

    #[test]
    fn test_command_allowed() {
        let sandbox = SecuritySandbox::with_default_config();
        let result = sandbox.check_command("ls -la");
        assert!(result.allowed);
    }

    #[test]
    fn test_command_denied() {
        let config = SandboxConfig {
            denied_commands: vec!["rm".to_string(), "sudo".to_string()],
            ..Default::default()
        };
        let sandbox = SecuritySandbox::new(config);
        let result = sandbox.check_command("rm -rf /");
        assert!(!result.allowed);
    }

    #[test]
    fn test_network_denied_by_default() {
        let sandbox = SecuritySandbox::with_default_config();
        let result = sandbox.check_network();
        assert!(!result.allowed);
    }

    #[test]
    fn test_env_var_whitelist() {
        let sandbox = SecuritySandbox::with_default_config();
        assert!(sandbox.check_env_var("PATH").allowed);
        assert!(!sandbox.check_env_var("SECRET_KEY").allowed);
    }

    #[test]
    fn env_whitelist_accepts_path() {
        let sandbox = SecuritySandbox::with_default_config();
        assert!(sandbox.check_env_var("PATH").allowed);
        assert!(sandbox.check_env_var("HOME").allowed);
        assert!(sandbox.check_env_var("TEMP").allowed);
        assert!(!sandbox.check_env_var("SECRET_KEY").allowed);
        assert!(!sandbox.check_env_var("DATABASE_URL").allowed);
    }

    #[test]
    fn validate_environment_detects_denied_vars() {
        let sandbox = SecuritySandbox::with_default_config();
        let denied = sandbox.validate_environment();
        assert!(!denied.iter().any(|v| v == "PATH"));
        assert!(!denied.iter().any(|v| v == "HOME"));
    }
}
