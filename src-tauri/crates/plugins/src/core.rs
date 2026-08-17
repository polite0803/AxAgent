// SPDX-License-Identifier: AGPL-3.0-only
//! Plugin trait 实现 —— Builtin/Bundled/External 三个变体的 Plugin 行为。
//!
//! 类型通过 `pub use core::*` 在 lib.rs 根 re-export，
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

const EXTERNAL_MARKETPLACE: &str = "external";
const BUILTIN_MARKETPLACE: &str = "builtin";
const BUNDLED_MARKETPLACE: &str = "bundled";
const OPENCLAW_MARKETPLACE: &str = "openclaw";
const SETTINGS_FILE_NAME: &str = "settings.json";
const REGISTRY_FILE_NAME: &str = "installed.json";
pub(crate) const MANIFEST_FILE_NAME: &str = "plugin.json";
pub(crate) const MANIFEST_RELATIVE_PATH: &str = ".claude-plugin/plugin.json";
const SKILL_MD_FILE_NAME: &str = "SKILL.md";

use crate::manager::{
    PluginError, run_lifecycle_commands, validate_hook_paths, validate_lifecycle_paths,
    validate_tool_paths,
};
use crate::types::*;

pub trait Plugin {
    fn metadata(&self) -> &PluginMetadata;
    fn hooks(&self) -> &PluginHooks;
    fn lifecycle(&self) -> &PluginLifecycle;
    fn tools(&self) -> &[PluginTool];
    fn mcp_servers(&self) -> &[PluginMcpServer];
    fn skills(&self) -> &[PluginSkillEntry];
    /// 插件声明的权限集合（来自 manifest），供沙箱执行前 capability 检查使用。
    fn permissions(&self) -> &[PluginPermission];
    fn validate(&self) -> Result<(), PluginError>;
    fn initialize(&self) -> Result<(), PluginError>;
    fn shutdown(&self) -> Result<(), PluginError>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum PluginDefinition {
    Builtin(BuiltinPlugin),
    Bundled(BundledPlugin),
    External(ExternalPlugin),
    OpenClaw(OpenClawPlugin),
}

impl Plugin for BuiltinPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn hooks(&self) -> &PluginHooks {
        &self.hooks
    }

    fn lifecycle(&self) -> &PluginLifecycle {
        &self.lifecycle
    }

    fn tools(&self) -> &[PluginTool] {
        &self.tools
    }

    fn mcp_servers(&self) -> &[PluginMcpServer] {
        &self.mcp_servers
    }

    fn skills(&self) -> &[PluginSkillEntry] {
        &self.skills
    }

    fn permissions(&self) -> &[PluginPermission] {
        &self.permissions
    }

    fn validate(&self) -> Result<(), PluginError> {
        if self.metadata.name.trim().is_empty() {
            return Err(PluginError::InvalidManifest(
                "builtin plugin name cannot be empty".to_string(),
            ));
        }
        if self.metadata.version.trim().is_empty() {
            return Err(PluginError::InvalidManifest(format!(
                "builtin plugin `{}` version cannot be empty",
                self.metadata.id
            )));
        }
        Ok(())
    }

    fn initialize(&self) -> Result<(), PluginError> {
        Ok(())
    }

    fn shutdown(&self) -> Result<(), PluginError> {
        Ok(())
    }
}

impl Plugin for BundledPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn hooks(&self) -> &PluginHooks {
        &self.hooks
    }

    fn lifecycle(&self) -> &PluginLifecycle {
        &self.lifecycle
    }

    fn tools(&self) -> &[PluginTool] {
        &self.tools
    }

    fn mcp_servers(&self) -> &[PluginMcpServer] {
        &self.mcp_servers
    }

    fn skills(&self) -> &[PluginSkillEntry] {
        &self.skills
    }

    fn permissions(&self) -> &[PluginPermission] {
        &self.permissions
    }

    fn validate(&self) -> Result<(), PluginError> {
        validate_hook_paths(self.metadata.root.as_deref(), &self.hooks)?;
        validate_lifecycle_paths(self.metadata.root.as_deref(), &self.lifecycle)?;
        validate_tool_paths(self.metadata.root.as_deref(), &self.tools)
    }

    fn initialize(&self) -> Result<(), PluginError> {
        run_lifecycle_commands(self.metadata(), self.lifecycle(), "init", &self.lifecycle.init)
    }

    fn shutdown(&self) -> Result<(), PluginError> {
        run_lifecycle_commands(
            self.metadata(),
            self.lifecycle(),
            "shutdown",
            &self.lifecycle.shutdown,
        )
    }
}

impl Plugin for ExternalPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn hooks(&self) -> &PluginHooks {
        &self.hooks
    }

    fn lifecycle(&self) -> &PluginLifecycle {
        &self.lifecycle
    }

    fn tools(&self) -> &[PluginTool] {
        &self.tools
    }

    fn mcp_servers(&self) -> &[PluginMcpServer] {
        &self.mcp_servers
    }

    fn skills(&self) -> &[PluginSkillEntry] {
        &self.skills
    }

    fn permissions(&self) -> &[PluginPermission] {
        &self.permissions
    }

    fn validate(&self) -> Result<(), PluginError> {
        validate_hook_paths(self.metadata.root.as_deref(), &self.hooks)?;
        validate_lifecycle_paths(self.metadata.root.as_deref(), &self.lifecycle)?;
        validate_tool_paths(self.metadata.root.as_deref(), &self.tools)
    }

    fn initialize(&self) -> Result<(), PluginError> {
        run_lifecycle_commands(self.metadata(), self.lifecycle(), "init", &self.lifecycle.init)
    }

    fn shutdown(&self) -> Result<(), PluginError> {
        run_lifecycle_commands(
            self.metadata(),
            self.lifecycle(),
            "shutdown",
            &self.lifecycle.shutdown,
        )
    }
}

impl Plugin for OpenClawPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn hooks(&self) -> &PluginHooks {
        &self.hooks
    }

    fn lifecycle(&self) -> &PluginLifecycle {
        &self.lifecycle
    }

    fn tools(&self) -> &[PluginTool] {
        &self.tools
    }

    fn mcp_servers(&self) -> &[PluginMcpServer] {
        &self.mcp_servers
    }

    fn skills(&self) -> &[PluginSkillEntry] {
        &self.skills
    }

    fn permissions(&self) -> &[PluginPermission] {
        &self.permissions
    }

    fn validate(&self) -> Result<(), PluginError> {
        validate_hook_paths(self.metadata.root.as_deref(), &self.hooks)?;
        validate_lifecycle_paths(self.metadata.root.as_deref(), &self.lifecycle)?;
        validate_tool_paths(self.metadata.root.as_deref(), &self.tools)
    }

    fn initialize(&self) -> Result<(), PluginError> {
        run_lifecycle_commands(self.metadata(), self.lifecycle(), "init", &self.lifecycle.init)
    }

    fn shutdown(&self) -> Result<(), PluginError> {
        run_lifecycle_commands(
            self.metadata(),
            self.lifecycle(),
            "shutdown",
            &self.lifecycle.shutdown,
        )
    }
}

impl Plugin for PluginDefinition {
    fn metadata(&self) -> &PluginMetadata {
        match self {
            Self::Builtin(plugin) => plugin.metadata(),
            Self::Bundled(plugin) => plugin.metadata(),
            Self::External(plugin) => plugin.metadata(),
            Self::OpenClaw(plugin) => plugin.metadata(),
        }
    }

    fn hooks(&self) -> &PluginHooks {
        match self {
            Self::Builtin(plugin) => plugin.hooks(),
            Self::Bundled(plugin) => plugin.hooks(),
            Self::External(plugin) => plugin.hooks(),
            Self::OpenClaw(plugin) => plugin.hooks(),
        }
    }

    fn lifecycle(&self) -> &PluginLifecycle {
        match self {
            Self::Builtin(plugin) => plugin.lifecycle(),
            Self::Bundled(plugin) => plugin.lifecycle(),
            Self::External(plugin) => plugin.lifecycle(),
            Self::OpenClaw(plugin) => plugin.lifecycle(),
        }
    }

    fn tools(&self) -> &[PluginTool] {
        match self {
            Self::Builtin(plugin) => plugin.tools(),
            Self::Bundled(plugin) => plugin.tools(),
            Self::External(plugin) => plugin.tools(),
            Self::OpenClaw(plugin) => plugin.tools(),
        }
    }

    fn mcp_servers(&self) -> &[PluginMcpServer] {
        match self {
            Self::Builtin(plugin) => plugin.mcp_servers(),
            Self::Bundled(plugin) => plugin.mcp_servers(),
            Self::External(plugin) => plugin.mcp_servers(),
            Self::OpenClaw(plugin) => plugin.mcp_servers(),
        }
    }

    fn skills(&self) -> &[PluginSkillEntry] {
        match self {
            Self::Builtin(plugin) => plugin.skills(),
            Self::Bundled(plugin) => plugin.skills(),
            Self::External(plugin) => plugin.skills(),
            Self::OpenClaw(plugin) => plugin.skills(),
        }
    }

    fn permissions(&self) -> &[PluginPermission] {
        match self {
            Self::Builtin(plugin) => plugin.permissions(),
            Self::Bundled(plugin) => plugin.permissions(),
            Self::External(plugin) => plugin.permissions(),
            Self::OpenClaw(plugin) => plugin.permissions(),
        }
    }

    fn validate(&self) -> Result<(), PluginError> {
        match self {
            Self::Builtin(plugin) => plugin.validate(),
            Self::Bundled(plugin) => plugin.validate(),
            Self::External(plugin) => plugin.validate(),
            Self::OpenClaw(plugin) => plugin.validate(),
        }
    }

    fn initialize(&self) -> Result<(), PluginError> {
        match self {
            Self::Builtin(plugin) => plugin.initialize(),
            Self::Bundled(plugin) => plugin.initialize(),
            Self::External(plugin) => plugin.initialize(),
            Self::OpenClaw(plugin) => plugin.initialize(),
        }
    }

    fn shutdown(&self) -> Result<(), PluginError> {
        match self {
            Self::Builtin(plugin) => plugin.shutdown(),
            Self::Bundled(plugin) => plugin.shutdown(),
            Self::External(plugin) => plugin.shutdown(),
            Self::OpenClaw(plugin) => plugin.shutdown(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisteredPlugin {
    pub(crate) definition: PluginDefinition,
    enabled: bool,
}

impl RegisteredPlugin {
    #[must_use]
    pub fn new(definition: PluginDefinition, enabled: bool) -> Self {
        Self { definition, enabled }
    }

    #[must_use]
    pub fn metadata(&self) -> &PluginMetadata {
        self.definition.metadata()
    }

    #[must_use]
    pub fn hooks(&self) -> &PluginHooks {
        self.definition.hooks()
    }

    #[must_use]
    pub fn tools(&self) -> &[PluginTool] {
        self.definition.tools()
    }

    #[must_use]
    pub fn permissions(&self) -> &[PluginPermission] {
        self.definition.permissions()
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn validate(&self) -> Result<(), PluginError> {
        self.definition.validate()
    }

    pub fn initialize(&self) -> Result<(), PluginError> {
        self.definition.initialize()
    }

    pub fn shutdown(&self) -> Result<(), PluginError> {
        self.definition.shutdown()
    }

    #[must_use]
    pub fn summary(&self) -> PluginSummary {
        PluginSummary {
            metadata: self.metadata().clone(),
            enabled: self.enabled,
            tool_names: self.tools().iter().map(|t| t.definition().name.clone()).collect(),
            mcp_server_names: self
                .definition
                .mcp_servers()
                .iter()
                .map(|m| m.name.clone())
                .collect(),
            skill_names: self.definition.skills().iter().map(|s| s.name.clone()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginSummary {
    pub metadata: PluginMetadata,
    pub enabled: bool,
    pub tool_names: Vec<String>,
    pub mcp_server_names: Vec<String>,
    pub skill_names: Vec<String>,
}

#[derive(Debug)]
pub struct PluginLoadFailure {
    pub plugin_root: PathBuf,
    pub kind: PluginKind,
    pub source: String,
    error: Box<PluginError>,
}

impl PluginLoadFailure {
    #[must_use]
    pub fn new(plugin_root: PathBuf, kind: PluginKind, source: String, error: PluginError) -> Self {
        Self { plugin_root, kind, source, error: Box::new(error) }
    }

    #[must_use]
    pub fn error(&self) -> &PluginError {
        self.error.as_ref()
    }
}

impl Display for PluginLoadFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to load {} plugin from `{}` (source: {}): {}",
            self.kind,
            self.plugin_root.display(),
            self.source,
            self.error()
        )
    }
}

#[derive(Debug)]
pub struct PluginRegistryReport {
    registry: PluginRegistry,
    failures: Vec<PluginLoadFailure>,
}

impl PluginRegistryReport {
    #[must_use]
    pub fn new(registry: PluginRegistry, failures: Vec<PluginLoadFailure>) -> Self {
        Self { registry, failures }
    }

    #[must_use]
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }

    #[must_use]
    pub fn failures(&self) -> &[PluginLoadFailure] {
        &self.failures
    }

    #[must_use]
    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    #[must_use]
    pub fn summaries(&self) -> Vec<PluginSummary> {
        self.registry.summaries()
    }

    pub fn into_registry(self) -> Result<PluginRegistry, PluginError> {
        if self.failures.is_empty() {
            Ok(self.registry)
        } else {
            Err(PluginError::LoadFailures(self.failures))
        }
    }

    /// Like `into_registry()`, but ignores load failures and returns the
    /// successfully loaded plugins. Failures are kept in `self.failures`
    /// so callers can still inspect them via `failures()`.
    pub fn into_registry_allowing_failures(self) -> PluginRegistry {
        self.registry
    }
}

#[derive(Debug, Default)]
pub(crate) struct PluginDiscovery {
    pub(crate) plugins: Vec<PluginDefinition>,
    pub(crate) failures: Vec<PluginLoadFailure>,
}

impl PluginDiscovery {
    pub(crate) fn push_plugin(&mut self, plugin: PluginDefinition) {
        self.plugins.push(plugin);
    }

    pub(crate) fn push_failure(&mut self, failure: PluginLoadFailure) {
        self.failures.push(failure);
    }

    pub(crate) fn extend(&mut self, other: Self) {
        self.plugins.extend(other.plugins);
        self.failures.extend(other.failures);
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PluginRegistry {
    pub(crate) plugins: Vec<RegisteredPlugin>,
}

impl PluginRegistry {
    #[must_use]
    pub fn new(mut plugins: Vec<RegisteredPlugin>) -> Self {
        plugins.sort_by(|left, right| left.metadata().id.cmp(&right.metadata().id));
        Self { plugins }
    }

    #[must_use]
    pub fn plugins(&self) -> &[RegisteredPlugin] {
        &self.plugins
    }

    #[must_use]
    pub fn get(&self, plugin_id: &str) -> Option<&RegisteredPlugin> {
        self.plugins.iter().find(|plugin| plugin.metadata().id == plugin_id)
    }

    #[must_use]
    pub fn contains(&self, plugin_id: &str) -> bool {
        self.get(plugin_id).is_some()
    }

    #[must_use]
    pub fn summaries(&self) -> Vec<PluginSummary> {
        self.plugins.iter().map(RegisteredPlugin::summary).collect()
    }

    pub fn aggregated_hooks(&self) -> Result<PluginHooks, PluginError> {
        self.plugins.iter().filter(|plugin| plugin.is_enabled()).try_fold(
            PluginHooks::default(),
            |acc, plugin| {
                plugin.validate()?;
                Ok(acc.merged_with(plugin.hooks()))
            },
        )
    }

    pub fn aggregated_tools(&self) -> Result<Vec<PluginTool>, PluginError> {
        let mut tools = Vec::new();
        let mut seen_names = BTreeMap::new();
        for plugin in self.plugins.iter().filter(|plugin| plugin.is_enabled()) {
            plugin.validate()?;
            for tool in plugin.tools() {
                if let Some(existing_plugin) =
                    seen_names.insert(tool.definition().name.clone(), tool.plugin_id().to_string())
                {
                    return Err(PluginError::InvalidManifest(format!(
                        "plugin tool `{}` is defined by both `{existing_plugin}` and `{}`",
                        tool.definition().name,
                        tool.plugin_id()
                    )));
                }
                tools.push(tool.clone());
            }
        }
        Ok(tools)
    }

    /// 聚合所有 enabled 插件声明的权限集合（去重）。
    ///
    /// 用于在 [`crate::hooks::HookRunner`] 等执行入口构建统一沙箱：
    /// 任一插件声明的能力在聚合后即放行（并集语义），避免单个插件的
    /// 缺失权限阻塞其他插件的合法子进程调用。
    #[must_use]
    pub fn aggregated_permissions(&self) -> Vec<PluginPermission> {
        let mut seen = BTreeSet::new();
        let mut aggregated: Vec<PluginPermission> = Vec::new();
        for plugin in self.plugins.iter().filter(|plugin| plugin.is_enabled()) {
            for permission in plugin.permissions() {
                if seen.insert(*permission) {
                    aggregated.push(*permission);
                }
            }
        }
        aggregated
    }

    pub fn initialize(&self) -> Result<(), PluginError> {
        for plugin in self.plugins.iter().filter(|plugin| plugin.is_enabled()) {
            plugin.validate()?;
            plugin.initialize()?;
        }
        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), PluginError> {
        for plugin in self.plugins.iter().rev().filter(|plugin| plugin.is_enabled()) {
            plugin.shutdown()?;
        }
        Ok(())
    }
}
