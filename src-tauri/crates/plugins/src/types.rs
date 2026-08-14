// SPDX-License-Identifier: AGPL-3.0-only
//! 插件类型定义 —— 清单、配置、权限、安装源等。
//!
//! 部分 helper 常量/import 仅在测试或可选特性中使用，因此本文件放宽 dead_code 检查。

#![allow(dead_code, unused_imports)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use axagent_harness::{NpmRegistryService, parse_npm_package_spec};

use crate::manager::PluginError;
use crate::sandbox::{SandboxConfig, apply_env_to_command, check_subprocess_permission};

const EXTERNAL_MARKETPLACE: &str = "external";
const BUILTIN_MARKETPLACE: &str = "builtin";
const BUNDLED_MARKETPLACE: &str = "bundled";
const SETTINGS_FILE_NAME: &str = "settings.json";
const REGISTRY_FILE_NAME: &str = "installed.json";
const MANIFEST_FILE_NAME: &str = "plugin.json";
const MANIFEST_RELATIVE_PATH: &str = ".claude-plugin/plugin.json";
const SKILL_MD_FILE_NAME: &str = "SKILL.md";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginKind {
    Builtin,
    Bundled,
    External,
}

impl Display for PluginKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Builtin => write!(f, "builtin"),
            Self::Bundled => write!(f, "bundled"),
            Self::External => write!(f, "external"),
        }
    }
}

impl PluginKind {
    #[must_use]
    pub fn marketplace(self) -> &'static str {
        match self {
            Self::Builtin => BUILTIN_MARKETPLACE,
            Self::Bundled => BUNDLED_MARKETPLACE,
            Self::External => EXTERNAL_MARKETPLACE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub kind: PluginKind,
    pub source: String,
    pub default_enabled: bool,
    pub root: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginHooks {
    #[serde(rename = "PreToolUse", default)]
    pub pre_tool_use: Vec<String>,
    #[serde(rename = "PostToolUse", default)]
    pub post_tool_use: Vec<String>,
    #[serde(rename = "PostToolUseFailure", default)]
    pub post_tool_use_failure: Vec<String>,
}

impl PluginHooks {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pre_tool_use.is_empty()
            && self.post_tool_use.is_empty()
            && self.post_tool_use_failure.is_empty()
    }

    #[must_use]
    pub fn merged_with(&self, other: &Self) -> Self {
        let mut merged = self.clone();
        merged.pre_tool_use.extend(other.pre_tool_use.iter().cloned());
        merged.post_tool_use.extend(other.post_tool_use.iter().cloned());
        merged.post_tool_use_failure.extend(other.post_tool_use_failure.iter().cloned());
        merged
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginLifecycle {
    #[serde(rename = "Init", default)]
    pub init: Vec<String>,
    #[serde(rename = "Shutdown", default)]
    pub shutdown: Vec<String>,
}

impl PluginLifecycle {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.init.is_empty() && self.shutdown.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginDashboardPanel {
    pub id: String,
    pub title: String,
    pub component_name: String,
    pub position: String,
    pub size: String,
    #[serde(default)]
    pub props: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub permissions: Vec<PluginPermission>,
    #[serde(rename = "defaultEnabled", default)]
    pub default_enabled: bool,
    #[serde(default)]
    pub hooks: PluginHooks,
    #[serde(default)]
    pub lifecycle: PluginLifecycle,
    #[serde(default)]
    pub tools: Vec<PluginToolManifest>,
    #[serde(default)]
    pub commands: Vec<PluginCommandManifest>,
    #[serde(default)]
    pub scenarios: Vec<String>,
    pub mcp_servers: Vec<PluginMcpServer>,
    pub skills: Vec<PluginSkillEntry>,
    pub agents: Vec<PluginAgentDefInternal>,
    #[serde(default)]
    pub dashboard_panels: Vec<PluginDashboardPanel>,
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    #[serde(default)]
    pub integrity: Option<PluginIntegrity>,
    /// 插件声明的能力（P3 外部插件注册：启用时注册到能力注册表，禁用/卸载时可逆回滚）。
    #[serde(default)]
    pub capabilities: Vec<PluginCapabilityDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_name: String,
    pub min_version: Option<String>,
}

/// 插件声明的能力（P3 外部插件注册）。
///
/// 脚本插件无法跨进程提供 Rust trait 对象，因此以「声明」形式描述插件
/// 提供的能力接缝；`PluginManager` 在启用插件时将这些声明以
/// `CapabilityOrigin::ExternalPlugin` 注册进能力注册表，禁用 / 卸载时可逆回滚。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilityDecl {
    /// 能力接缝 ID（如 `"platform.adapter.telegram"`、`"tool.set.myplugin"`）。
    pub seam: String,
    /// 能力类型标识（如 `"platform_adapter"`、`"tool_set"`）。
    #[serde(default)]
    pub capability_type: String,
    /// 契约版本（默认 `"1.0"`）。
    #[serde(default = "default_capability_version")]
    pub version: String,
    /// 人类可读描述。
    #[serde(default)]
    pub description: String,
}

fn default_capability_version() -> String {
    "1.0".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginIntegrity {
    pub algorithm: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMcpServer {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginSkillEntry {
    pub name: String,
    pub path: String,
}

/// 插件内部 Agent 定义（反序列化后转换为 agent_provider::PluginAgentDef）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginAgentDefInternal {
    pub agent_type: String,
    pub description: String,
    pub tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub model: Option<String>,
    pub background: bool,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    Read,
    Write,
    Execute,
    FileSystemRead,
    FileSystemWrite,
    NetworkAccess,
    SubprocessExecution,
    ClipboardAccess,
    NotificationAccess,
}

impl PluginPermission {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Execute => "execute",
            Self::FileSystemRead => "file_system_read",
            Self::FileSystemWrite => "file_system_write",
            Self::NetworkAccess => "network_access",
            Self::SubprocessExecution => "subprocess_execution",
            Self::ClipboardAccess => "clipboard_access",
            Self::NotificationAccess => "notification_access",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "execute" => Some(Self::Execute),
            "file_system_read" => Some(Self::FileSystemRead),
            "file_system_write" => Some(Self::FileSystemWrite),
            "network_access" => Some(Self::NetworkAccess),
            "subprocess_execution" => Some(Self::SubprocessExecution),
            "clipboard_access" => Some(Self::ClipboardAccess),
            "notification_access" => Some(Self::NotificationAccess),
            _ => None,
        }
    }

    #[must_use]
    pub fn implies_read(&self) -> bool {
        matches!(
            self,
            Self::Read
                | Self::FileSystemRead
                | Self::Write
                | Self::FileSystemWrite
                | Self::Execute
                | Self::SubprocessExecution
        )
    }

    #[must_use]
    pub fn implies_write(&self) -> bool {
        matches!(
            self,
            Self::Write | Self::FileSystemWrite | Self::Execute | Self::SubprocessExecution
        )
    }

    #[must_use]
    pub fn implies_execute(&self) -> bool {
        matches!(self, Self::Execute | Self::SubprocessExecution)
    }
}

impl AsRef<str> for PluginPermission {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginToolManifest {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub required_permission: PluginToolPermission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginToolPermission {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl PluginToolPermission {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::WorkspaceWrite => "workspace-write",
            Self::DangerFullAccess => "danger-full-access",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "read-only" => Some(Self::ReadOnly),
            "workspace-write" => Some(Self::WorkspaceWrite),
            "danger-full-access" => Some(Self::DangerFullAccess),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginToolDefinition {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCommandManifest {
    pub name: String,
    pub description: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPluginMcpServer {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPluginSkillEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPluginAgentDef {
    #[serde(rename = "agentType")]
    pub agent_type: String,
    pub description: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(rename = "disallowedTools", default)]
    pub disallowed_tools: Vec<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub background: bool,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RawPluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(rename = "defaultEnabled", default)]
    pub default_enabled: bool,
    #[serde(default)]
    pub hooks: PluginHooks,
    #[serde(default)]
    pub lifecycle: PluginLifecycle,
    #[serde(default)]
    pub tools: Vec<RawPluginToolManifest>,
    #[serde(default)]
    pub commands: Vec<PluginCommandManifest>,
    #[serde(default)]
    pub scenarios: Vec<String>,
    #[serde(default, alias = "mcpServers")]
    pub mcp_servers: Vec<RawPluginMcpServer>,
    #[serde(default)]
    pub skills: Vec<RawPluginSkillEntry>,
    #[serde(default)]
    pub agents: Vec<RawPluginAgentDef>,
    #[serde(default)]
    pub dashboard_panels: Vec<PluginDashboardPanel>,
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    #[serde(default)]
    pub integrity: Option<PluginIntegrity>,
    #[serde(default)]
    pub capabilities: Vec<PluginCapabilityDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPluginToolManifest {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(rename = "requiredPermission", default = "default_tool_permission_label")]
    pub required_permission: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PluginTool {
    plugin_id: String,
    plugin_name: String,
    definition: PluginToolDefinition,
    pub command: String,
    args: Vec<String>,
    required_permission: PluginToolPermission,
    root: Option<PathBuf>,
}

impl PluginTool {
    #[must_use]
    pub fn new(
        plugin_id: impl Into<String>,
        plugin_name: impl Into<String>,
        definition: PluginToolDefinition,
        command: impl Into<String>,
        args: Vec<String>,
        required_permission: PluginToolPermission,
        root: Option<PathBuf>,
    ) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            plugin_name: plugin_name.into(),
            definition,
            command: command.into(),
            args,
            required_permission,
            root,
        }
    }

    #[must_use]
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    #[must_use]
    pub fn definition(&self) -> &PluginToolDefinition {
        &self.definition
    }

    #[must_use]
    pub fn required_permission(&self) -> &str {
        self.required_permission.as_str()
    }

    pub fn check_permission(
        &self,
        declared_permissions: &[PluginPermission],
    ) -> Result<(), PluginError> {
        match self.required_permission {
            PluginToolPermission::ReadOnly => {
                if !declared_permissions.iter().any(|p| p.implies_read()) {
                    return Err(PluginError::CommandFailed(format!(
                        "plugin tool `{}` requires read permission, but plugin `{}` only declares {:?}",
                        self.definition.name,
                        self.plugin_id,
                        declared_permissions.iter().map(|p| p.as_str()).collect::<Vec<_>>()
                    )));
                }
                Ok(())
            },
            PluginToolPermission::WorkspaceWrite => {
                if !declared_permissions.iter().any(|p| p.implies_write()) {
                    return Err(PluginError::CommandFailed(format!(
                        "plugin tool `{}` requires workspace-write permission, but plugin `{}` only declares {:?}",
                        self.definition.name,
                        self.plugin_id,
                        declared_permissions.iter().map(|p| p.as_str()).collect::<Vec<_>>()
                    )));
                }
                Ok(())
            },
            PluginToolPermission::DangerFullAccess => {
                if !declared_permissions.iter().any(|p| p.implies_execute()) {
                    return Err(PluginError::CommandFailed(format!(
                        "plugin tool `{}` requires danger-full-access permission, but plugin `{}` only declares {:?} (needs 'execute')",
                        self.definition.name,
                        self.plugin_id,
                        declared_permissions.iter().map(|p| p.as_str()).collect::<Vec<_>>()
                    )));
                }
                Ok(())
            },
        }
    }

    pub fn execute_with_permission_check(
        &self,
        input: &Value,
        declared_permissions: &[PluginPermission],
    ) -> Result<String, PluginError> {
        self.check_permission(declared_permissions)?;
        self.execute(input)
    }

    const DEFAULT_TOOL_TIMEOUT_SECS: u64 = 60;

    pub fn execute(&self, input: &Value) -> Result<String, PluginError> {
        let input_json = input.to_string();
        let mut process = Command::new(&self.command);
        process
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("CLAWD_PLUGIN_ID", &self.plugin_id)
            .env("CLAWD_PLUGIN_NAME", &self.plugin_name)
            .env("CLAWD_TOOL_NAME", &self.definition.name)
            .env("CLAWD_TOOL_INPUT", &input_json);
        // Windows: 隐藏控制台窗口（插件命令可能是 node/python 等控制台程序）
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            process.creation_flags(0x08000000);
        }
        if let Some(root) = &self.root {
            process.current_dir(root).env("CLAWD_PLUGIN_ROOT", root.display().to_string());
        }

        let mut child = process.spawn()?;
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write as _;
            stdin.write_all(input_json.as_bytes())?;
        }

        let timeout = Duration::from_secs(Self::DEFAULT_TOOL_TIMEOUT_SECS);
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let output = child.wait_with_output()?;
                    if status.success() {
                        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        return Err(PluginError::CommandFailed(format!(
                            "plugin tool `{}` from `{}` failed for `{}`: {}",
                            self.definition.name,
                            self.plugin_id,
                            self.command,
                            if stderr.is_empty() {
                                format!("exit status {}", status)
                            } else {
                                stderr
                            }
                        )));
                    }
                },
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(PluginError::CommandFailed(format!(
                            "plugin tool `{}` from `{}` timed out after {}s",
                            self.definition.name,
                            self.plugin_id,
                            timeout.as_secs()
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                },
                Err(e) => return Err(PluginError::Io(e)),
            }
        }
    }

    /// 在沙箱约束下执行工具命令。
    ///
    /// 与 [`execute`](Self::execute) 的区别：
    /// 1. 执行前调用 [`check_subprocess_permission`]，未声明 `subprocess_execution`
    ///    权限时直接返回 [`PluginError::PermissionDenied`]，不启动子进程。
    /// 2. 通过 [`apply_env_to_command`] 对子进程 ENV 做白名单过滤，屏蔽
    ///    API Key / Token / Secret 等敏感变量。
    ///
    /// 显式设置的 `CLAWD_PLUGIN_ID` 等专用变量在 `apply_env_to_command` 之后
    /// 写入，不受白名单约束。
    pub fn execute_sandboxed(
        &self,
        input: &Value,
        sandbox: &SandboxConfig,
    ) -> Result<String, PluginError> {
        // 沙箱检查：未声明 subprocess_execution 权限时禁止启动子进程
        check_subprocess_permission(sandbox)?;

        let input_json = input.to_string();
        let mut process = Command::new(&self.command);
        // 沙箱：ENV 白名单过滤（env_clear + 回填白名单变量）
        apply_env_to_command(&mut process, sandbox);
        process
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("CLAWD_PLUGIN_ID", &self.plugin_id)
            .env("CLAWD_PLUGIN_NAME", &self.plugin_name)
            .env("CLAWD_TOOL_NAME", &self.definition.name)
            .env("CLAWD_TOOL_INPUT", &input_json);
        // Windows: 隐藏控制台窗口（插件命令可能是 node/python 等控制台程序）
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            process.creation_flags(0x08000000);
        }
        if let Some(root) = &self.root {
            process.current_dir(root).env("CLAWD_PLUGIN_ROOT", root.display().to_string());
        }

        let mut child = process.spawn()?;
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write as _;
            stdin.write_all(input_json.as_bytes())?;
        }

        let timeout = Duration::from_secs(Self::DEFAULT_TOOL_TIMEOUT_SECS);
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let output = child.wait_with_output()?;
                    if status.success() {
                        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        return Err(PluginError::CommandFailed(format!(
                            "plugin tool `{}` from `{}` failed for `{}`: {}",
                            self.definition.name,
                            self.plugin_id,
                            self.command,
                            if stderr.is_empty() {
                                format!("exit status {}", status)
                            } else {
                                stderr
                            }
                        )));
                    }
                },
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(PluginError::CommandFailed(format!(
                            "plugin tool `{}` from `{}` timed out after {}s",
                            self.definition.name,
                            self.plugin_id,
                            timeout.as_secs()
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                },
                Err(e) => return Err(PluginError::Io(e)),
            }
        }
    }
}

fn default_tool_permission_label() -> String {
    "read-only".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginInstallSource {
    LocalPath { path: PathBuf },
    GitUrl { url: String },
    NpmPackage { name: String, version: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPluginRecord {
    #[serde(default = "default_plugin_kind")]
    pub kind: PluginKind,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub install_path: PathBuf,
    pub source: PluginInstallSource,
    pub installed_at_unix_ms: u128,
    pub updated_at_unix_ms: u128,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPluginRegistry {
    #[serde(default)]
    pub plugins: BTreeMap<String, InstalledPluginRecord>,
}

fn default_plugin_kind() -> PluginKind {
    PluginKind::External
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuiltinPlugin {
    pub metadata: PluginMetadata,
    pub hooks: PluginHooks,
    pub lifecycle: PluginLifecycle,
    pub tools: Vec<PluginTool>,
    pub mcp_servers: Vec<PluginMcpServer>,
    pub skills: Vec<PluginSkillEntry>,
    /// 插件声明的权限集合（来自 manifest），用于沙箱 capability 检查。
    pub permissions: Vec<PluginPermission>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BundledPlugin {
    pub metadata: PluginMetadata,
    pub hooks: PluginHooks,
    pub lifecycle: PluginLifecycle,
    pub tools: Vec<PluginTool>,
    pub mcp_servers: Vec<PluginMcpServer>,
    pub skills: Vec<PluginSkillEntry>,
    /// 插件声明的权限集合（来自 manifest），用于沙箱 capability 检查。
    pub permissions: Vec<PluginPermission>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternalPlugin {
    pub metadata: PluginMetadata,
    pub hooks: PluginHooks,
    pub lifecycle: PluginLifecycle,
    pub tools: Vec<PluginTool>,
    pub mcp_servers: Vec<PluginMcpServer>,
    pub skills: Vec<PluginSkillEntry>,
    /// 插件声明的权限集合（来自 manifest），用于沙箱 capability 检查。
    pub permissions: Vec<PluginPermission>,
}
