// SPDX-License-Identifier: AGPL-3.0-only

//! 终端环境抽象增强 (P1-11)
//!
//! 借鉴 Hermes Agent 的终端处理：
//! - 后端配置 DTO（本地/Docker/SSH）
//! - 输出 Spill 处理（大量输出溢出）
//! - 基础设施错误分类

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// 后端配置
// ---------------------------------------------------------------------------

/// 终端后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalBackendType {
    /// 本地终端
    Local,
    /// Docker 容器
    Docker,
    /// SSH 远程
    Ssh,
    /// Podman 容器
    Podman,
    /// 混合模式（本地优先，远程备选）
    Hybrid,
}

impl TerminalBackendType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TerminalBackendType::Local => "local",
            TerminalBackendType::Docker => "docker",
            TerminalBackendType::Ssh => "ssh",
            TerminalBackendType::Podman => "podman",
            TerminalBackendType::Hybrid => "hybrid",
        }
    }

    /// 是否为远程后端
    pub fn is_remote(&self) -> bool {
        matches!(self, TerminalBackendType::Ssh)
    }

    /// 是否为容器后端
    pub fn is_container(&self) -> bool {
        matches!(self, TerminalBackendType::Docker | TerminalBackendType::Podman)
    }
}

/// 终端后端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalBackendConfig {
    /// 后端类型
    pub backend_type: TerminalBackendType,
    /// 最大会话数
    pub max_sessions: usize,
    /// 命令超时时间（秒）
    pub command_timeout_secs: u64,
    /// 输出截断长度（字符数）
    pub output_truncate_chars: usize,
    /// 是否启用输出 spill
    pub enable_output_spill: bool,
    /// 环境变量白名单（仅用于本地）
    pub env_whitelist: Vec<String>,
    /// 资源限制
    pub resource_limits: ResourceLimits,
}

impl Default for TerminalBackendConfig {
    fn default() -> Self {
        Self {
            backend_type: TerminalBackendType::Local,
            max_sessions: 10,
            command_timeout_secs: 300,
            output_truncate_chars: 100_000,
            enable_output_spill: true,
            env_whitelist: vec![
                "PATH".to_string(),
                "HOME".to_string(),
                "TMPDIR".to_string(),
                "LANG".to_string(),
                "SHELL".to_string(),
                "USER".to_string(),
            ],
            resource_limits: ResourceLimits::default(),
        }
    }
}

/// 资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// 最大内存（MB）
    pub max_memory_mb: u64,
    /// 最大 CPU 时间（秒）
    pub max_cpu_time_secs: u64,
    /// 最大文件描述符数
    pub max_file_descriptors: u64,
    /// 最大输出大小（MB）
    pub max_output_mb: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 256,
            max_cpu_time_secs: 60,
            max_file_descriptors: 1024,
            max_output_mb: 10,
        }
    }
}

/// Docker 特定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerBackendConfig {
    /// Docker socket 路径
    pub socket_path: String,
    /// 默认镜像
    pub default_image: String,
    /// 工作目录
    pub working_dir: String,
    /// 挂载卷
    pub volumes: Vec<VolumeMount>,
    /// 环境变量
    pub environment: HashMap<String, String>,
}

impl Default for DockerBackendConfig {
    fn default() -> Self {
        Self {
            socket_path: "unix:///var/run/docker.sock".to_string(),
            default_image: "alpine:latest".to_string(),
            working_dir: "/workspace".to_string(),
            volumes: Vec::new(),
            environment: HashMap::new(),
        }
    }
}

/// 卷挂载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

/// SSH 特定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshBackendConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: SshAuthMethod,
    pub default_shell: String,
    pub working_dir: Option<String>,
}

impl Default for SshBackendConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_method: SshAuthMethod::Agent,
            default_shell: "/bin/bash".to_string(),
            working_dir: None,
        }
    }
}

/// SSH 认证方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SshAuthMethod {
    /// SSH Agent
    Agent,
    /// 密钥文件
    KeyFile,
    /// 密码
    Password,
    /// 密钥内容
    KeyContent,
}

// ---------------------------------------------------------------------------
// 输出 Spill 处理
// ---------------------------------------------------------------------------

/// 输出 Spill 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSpillConfig {
    /// 是否启用 spill
    pub enabled: bool,
    /// 触发 spill 的阈值（字符数）
    pub threshold_chars: usize,
    /// spill 后保留的头部字符数
    pub keep_head_chars: usize,
    /// spill 后保留的尾部字符数
    pub keep_tail_chars: usize,
    /// 是否生成中间摘要
    pub generate_summary: bool,
    /// 摘要最大长度
    pub summary_max_chars: usize,
}

impl Default for OutputSpillConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_chars: 100_000,
            keep_head_chars: 5_000,
            keep_tail_chars: 5_000,
            generate_summary: true,
            summary_max_chars: 2_000,
        }
    }
}

/// Spill 处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpillResult {
    /// 是否执行了 spill
    pub was_spilled: bool,
    /// 原始输出长度
    pub original_length: usize,
    /// 截断后长度
    pub truncated_length: usize,
    /// 头部保留
    pub head: String,
    /// 尾部保留
    pub tail: String,
    /// 中间摘要（如果生成）
    pub summary: Option<String>,
    /// 跳过的字符数
    pub skipped_chars: usize,
}

/// 输出截断处理器
pub struct OutputTruncator;

impl OutputTruncator {
    /// 处理输出，可能触发 spill
    pub fn process(output: &str, config: &OutputSpillConfig) -> SpillResult {
        let original_length = output.len();

        if !config.enabled || original_length <= config.threshold_chars {
            return SpillResult {
                was_spilled: false,
                original_length,
                truncated_length: original_length,
                head: output.to_string(),
                tail: String::new(),
                summary: None,
                skipped_chars: 0,
            };
        }

        // 执行 spill
        let head_end = config.keep_head_chars.min(original_length);
        let tail_start = original_length.saturating_sub(config.keep_tail_chars);

        let head = if head_end > 0 {
            &output[..head_end]
        } else {
            ""
        };
        let tail = if tail_start < original_length {
            &output[tail_start..]
        } else {
            ""
        };

        let skipped_chars = original_length.saturating_sub(head_end).saturating_sub(tail.len());

        // 生成摘要
        let summary = if config.generate_summary && skipped_chars > 0 {
            let s = format!("\n\n... (已省略 {} 个字符) ...\n", skipped_chars);
            Some(s.chars().take(config.summary_max_chars).collect())
        } else {
            None
        };

        let truncated_length = head.len() + tail.len() + summary.as_ref().map_or(0, String::len);

        SpillResult {
            was_spilled: true,
            original_length,
            truncated_length,
            head: head.to_string(),
            tail: tail.to_string(),
            summary,
            skipped_chars,
        }
    }
}

// ---------------------------------------------------------------------------
// 基础设施错误分类
// ---------------------------------------------------------------------------

/// 基础设施错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfrastructureErrorType {
    /// Docker 守护进程不可用
    DockerDaemonUnreachable,
    /// Docker 镜像拉取失败
    DockerImagePullFailed,
    /// Docker 容器启动失败
    DockerContainerStartFailed,
    /// SSH 连接失败
    SshConnectionFailed,
    /// SSH 认证失败
    SshAuthenticationFailed,
    /// SSH 命令执行失败
    SshCommandFailed,
    /// 本地终端创建失败
    LocalTerminalCreationFailed,
    /// 资源不足
    InsufficientResources,
    /// 权限不足
    PermissionDenied,
    /// 命令超时
    CommandTimeout,
    /// 输出溢出
    OutputOverflow,
    /// 会话断开
    SessionDisconnected,
    /// 未知基础设施错误
    Unknown,
}

impl InfrastructureErrorType {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            InfrastructureErrorType::DockerDaemonUnreachable
                | InfrastructureErrorType::DockerImagePullFailed
                | InfrastructureErrorType::SshConnectionFailed
                | InfrastructureErrorType::CommandTimeout
                | InfrastructureErrorType::SessionDisconnected
        )
    }

    pub fn requires_config_fix(&self) -> bool {
        matches!(
            self,
            InfrastructureErrorType::SshAuthenticationFailed
                | InfrastructureErrorType::PermissionDenied
                | InfrastructureErrorType::InsufficientResources
        )
    }

    pub fn description(&self) -> &'static str {
        match self {
            InfrastructureErrorType::DockerDaemonUnreachable => {
                "Docker 守护进程不可用，请确认 Docker 服务正在运行"
            },
            InfrastructureErrorType::DockerImagePullFailed => {
                "Docker 镜像拉取失败，请检查网络和镜像名称"
            },
            InfrastructureErrorType::DockerContainerStartFailed => {
                "Docker 容器启动失败，请检查资源和配置"
            },
            InfrastructureErrorType::SshConnectionFailed => "SSH 连接失败，请检查主机和网络",
            InfrastructureErrorType::SshAuthenticationFailed => "SSH 认证失败，请检查用户名和密钥",
            InfrastructureErrorType::SshCommandFailed => "SSH 命令执行失败",
            InfrastructureErrorType::LocalTerminalCreationFailed => "本地终端创建失败",
            InfrastructureErrorType::InsufficientResources => "系统资源不足（内存/CPU/磁盘）",
            InfrastructureErrorType::PermissionDenied => "权限不足，无法执行操作",
            InfrastructureErrorType::CommandTimeout => "命令执行超时",
            InfrastructureErrorType::OutputOverflow => "输出超过限制，已自动截断",
            InfrastructureErrorType::SessionDisconnected => "终端会话已断开",
            InfrastructureErrorType::Unknown => "未知基础设施错误",
        }
    }
}

/// 基础设施错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructureError {
    pub error_type: InfrastructureErrorType,
    pub message: String,
    pub backend_type: TerminalBackendType,
    pub command: Option<String>,
    pub exit_code: Option<i32>,
    pub suggestion: String,
    pub retryable: bool,
    pub requires_config_fix: bool,
}

impl InfrastructureError {
    pub fn new(
        error_type: InfrastructureErrorType,
        message: &str,
        backend_type: TerminalBackendType,
    ) -> Self {
        Self {
            error_type,
            message: message.to_string(),
            backend_type,
            command: None,
            exit_code: None,
            suggestion: error_type.description().to_string(),
            retryable: error_type.is_retryable(),
            requires_config_fix: error_type.requires_config_fix(),
        }
    }

    pub fn with_command(mut self, command: &str) -> Self {
        self.command = Some(command.to_string());
        self
    }

    pub fn with_exit_code(mut self, exit_code: i32) -> Self {
        self.exit_code = Some(exit_code);
        self
    }
}

/// 基础设施错误分类器
pub struct InfrastructureErrorClassifier;

impl InfrastructureErrorClassifier {
    /// 从错误消息分类
    pub fn classify(message: &str, backend_type: TerminalBackendType) -> InfrastructureErrorType {
        let lower = message.to_lowercase();

        // Docker 相关
        if backend_type.is_container() {
            if lower.contains("connection refused") || lower.contains("cannot connect") {
                return InfrastructureErrorType::DockerDaemonUnreachable;
            }
            if lower.contains("pull") && lower.contains("fail") {
                return InfrastructureErrorType::DockerImagePullFailed;
            }
            if lower.contains("container") && (lower.contains("start") || lower.contains("create"))
            {
                return InfrastructureErrorType::DockerContainerStartFailed;
            }
        }

        // SSH 相关
        if backend_type.is_remote() {
            if lower.contains("permission denied") || lower.contains("authentication") {
                return InfrastructureErrorType::SshAuthenticationFailed;
            }
            if lower.contains("connection")
                || lower.contains("timeout")
                || lower.contains("refused")
            {
                return InfrastructureErrorType::SshConnectionFailed;
            }
            if lower.contains("ssh") {
                return InfrastructureErrorType::SshCommandFailed;
            }
        }

        // 通用
        if lower.contains("timeout") || lower.contains("timed out") {
            return InfrastructureErrorType::CommandTimeout;
        }
        if lower.contains("out of memory") || lower.contains("oom") || lower.contains("resource") {
            return InfrastructureErrorType::InsufficientResources;
        }
        if lower.contains("permission") || lower.contains("access denied") {
            return InfrastructureErrorType::PermissionDenied;
        }
        if lower.contains("overflow") || lower.contains("too large") || lower.contains("exceed") {
            return InfrastructureErrorType::OutputOverflow;
        }
        if lower.contains("disconnect") || lower.contains("broken pipe") {
            return InfrastructureErrorType::SessionDisconnected;
        }

        InfrastructureErrorType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_type() {
        assert!(TerminalBackendType::Docker.is_container());
        assert!(TerminalBackendType::Ssh.is_remote());
        assert!(!TerminalBackendType::Local.is_remote());
    }

    #[test]
    fn test_output_truncator_no_spill() {
        let config = OutputSpillConfig::default();
        let output = "hello world".to_string();
        let result = OutputTruncator::process(&output, &config);

        assert!(!result.was_spilled);
        assert_eq!(result.original_length, result.truncated_length);
    }

    #[test]
    fn test_output_truncator_with_spill() {
        let config = OutputSpillConfig {
            threshold_chars: 100,
            keep_head_chars: 20,
            keep_tail_chars: 20,
            ..Default::default()
        };

        let output = "a".repeat(500);
        let result = OutputTruncator::process(&output, &config);

        assert!(result.was_spilled);
        assert!(result.skipped_chars > 0);
        assert!(result.summary.is_some());
    }

    #[test]
    fn test_infrastructure_error_classifier() {
        let err_type = InfrastructureErrorClassifier::classify(
            "Connection refused by Docker daemon",
            TerminalBackendType::Docker,
        );
        assert_eq!(err_type, InfrastructureErrorType::DockerDaemonUnreachable);

        let err_type = InfrastructureErrorClassifier::classify(
            "Permission denied (publickey)",
            TerminalBackendType::Ssh,
        );
        assert_eq!(err_type, InfrastructureErrorType::SshAuthenticationFailed);

        let err_type = InfrastructureErrorClassifier::classify(
            "Command timed out after 30s",
            TerminalBackendType::Local,
        );
        assert_eq!(err_type, InfrastructureErrorType::CommandTimeout);
    }

    #[test]
    fn test_error_type_properties() {
        assert!(InfrastructureErrorType::CommandTimeout.is_retryable());
        assert!(!InfrastructureErrorType::PermissionDenied.is_retryable());
        assert!(InfrastructureErrorType::SshAuthenticationFailed.requires_config_fix());
    }

    #[test]
    fn test_backend_config_default() {
        let config = TerminalBackendConfig::default();
        assert_eq!(config.backend_type, TerminalBackendType::Local);
        assert!(config.enable_output_spill);
        assert!(!config.env_whitelist.is_empty());
    }
}
