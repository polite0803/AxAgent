// SPDX-License-Identifier: AGPL-3.0-only
//! 插件管理器：安装/卸载/启用/禁用/查询 + 清单加载与校验。
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
use tracing;

use axagent_harness::{NpmRegistryService, parse_npm_package_spec};

const EXTERNAL_MARKETPLACE: &str = "external";
const BUILTIN_MARKETPLACE: &str = "builtin";
const BUNDLED_MARKETPLACE: &str = "bundled";
const SETTINGS_FILE_NAME: &str = "settings.json";
const REGISTRY_FILE_NAME: &str = "installed.json";
const MANIFEST_FILE_NAME: &str = "plugin.json";
const MANIFEST_RELATIVE_PATH: &str = ".claude-plugin/plugin.json";
const SKILL_MD_FILE_NAME: &str = "SKILL.md";

use crate::core::*;
use crate::mcp_launcher::McpLauncher;
use crate::sandbox::{SandboxConfig, apply_env_to_command, check_subprocess_permission};
use crate::skill_installer::SkillInstaller;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct PluginManagerConfig {
    pub config_home: PathBuf,
    pub enabled_plugins: BTreeMap<String, bool>,
    pub external_dirs: Vec<PathBuf>,
    pub install_root: Option<PathBuf>,
    pub registry_path: Option<PathBuf>,
    pub bundled_root: Option<PathBuf>,
}

impl PluginManagerConfig {
    #[must_use]
    pub fn new(config_home: impl Into<PathBuf>) -> Self {
        Self {
            config_home: config_home.into(),
            enabled_plugins: BTreeMap::new(),
            external_dirs: Vec::new(),
            install_root: None,
            registry_path: None,
            bundled_root: None,
        }
    }
}

#[derive(Debug)]
pub struct PluginManager {
    config: PluginManagerConfig,
    mcp_launcher: McpLauncher,
    skill_installer: SkillInstaller,
    npm_registry: Option<Arc<dyn NpmRegistryService>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallOutcome {
    pub plugin_id: String,
    pub version: String,
    pub install_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateOutcome {
    pub plugin_id: String,
    pub old_version: String,
    pub new_version: String,
    pub install_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginManifestValidationError {
    EmptyField { field: &'static str },
    EmptyEntryField { kind: &'static str, field: &'static str, name: Option<String> },
    InvalidPermission { permission: String },
    DuplicatePermission { permission: String },
    DuplicateEntry { kind: &'static str, name: String },
    MissingPath { kind: &'static str, path: PathBuf },
    PathIsDirectory { kind: &'static str, path: PathBuf },
    InvalidToolInputSchema { tool_name: String },
    InvalidToolRequiredPermission { tool_name: String, permission: String },
    UnsupportedManifestContract { detail: String },
    DependencyNotSatisfied { plugin_name: String, min_version: Option<String> },
    IntegrityCheckFailed { algorithm: String, expected: String, actual: String },
}

impl Display for PluginManifestValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField { field } => {
                write!(f, "plugin manifest {field} cannot be empty")
            },
            Self::EmptyEntryField { kind, field, name } => match name {
                Some(name) if !name.is_empty() => {
                    write!(f, "plugin {kind} `{name}` {field} cannot be empty")
                },
                _ => write!(f, "plugin {kind} {field} cannot be empty"),
            },
            Self::InvalidPermission { permission } => {
                write!(
                    f,
                    "plugin manifest permission `{permission}` must be one of read, write, execute, file_system_read, file_system_write, network_access, subprocess_execution, clipboard_access, or notification_access"
                )
            },
            Self::DuplicatePermission { permission } => {
                write!(f, "plugin manifest permission `{permission}` is duplicated")
            },
            Self::DuplicateEntry { kind, name } => {
                write!(f, "plugin {kind} `{name}` is duplicated")
            },
            Self::MissingPath { kind, path } => {
                write!(f, "{kind} path `{}` does not exist", path.display())
            },
            Self::PathIsDirectory { kind, path } => {
                write!(f, "{kind} path `{}` must point to a file", path.display())
            },
            Self::InvalidToolInputSchema { tool_name } => {
                write!(f, "plugin tool `{tool_name}` inputSchema must be a JSON object")
            },
            Self::InvalidToolRequiredPermission { tool_name, permission } => write!(
                f,
                "plugin tool `{tool_name}` requiredPermission `{permission}` must be read-only, workspace-write, or danger-full-access"
            ),
            Self::UnsupportedManifestContract { detail } => f.write_str(detail),
            Self::DependencyNotSatisfied { plugin_name, min_version } => match min_version {
                Some(ver) => write!(
                    f,
                    "plugin dependency `{plugin_name}` (min version {ver}) is not satisfied"
                ),
                None => write!(f, "plugin dependency `{plugin_name}` is not installed"),
            },
            Self::IntegrityCheckFailed { algorithm, expected, actual } => {
                write!(
                    f,
                    "plugin integrity check failed ({algorithm}): expected {expected}, got {actual}"
                )
            },
        }
    }
}

#[derive(Debug)]
pub enum PluginError {
    Io(std::io::Error),
    Json(serde_json::Error),
    ManifestValidation(Vec<PluginManifestValidationError>),
    LoadFailures(Vec<PluginLoadFailure>),
    InvalidManifest(String),
    NotFound(String),
    CommandFailed(String),
    /// 沙箱权限拦截：插件执行前 capability 检查未通过（路径越权 / 缺少
    /// `subprocess_execution` 权限等）。与 `CommandFailed` 区分以便上层
    /// 做权限引导而非通用错误处理。
    PermissionDenied(String),
    /// 插件已安装冲突 — install 时检测到 plugin_id 已存在,
    /// 调用方应提示用户使用 update 而非 install。
    AlreadyExists(String),
}

impl Display for PluginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::ManifestValidation(errors) => {
                for (index, error) in errors.iter().enumerate() {
                    if index > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{error}")?;
                }
                Ok(())
            },
            Self::LoadFailures(failures) => {
                for (index, failure) in failures.iter().enumerate() {
                    if index > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{failure}")?;
                }
                Ok(())
            },
            Self::InvalidManifest(message)
            | Self::NotFound(message)
            | Self::CommandFailed(message)
            | Self::PermissionDenied(message)
            | Self::AlreadyExists(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for PluginError {}

impl From<std::io::Error> for PluginError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for PluginError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl PluginManager {
    #[must_use]
    pub fn new(config: PluginManagerConfig) -> Self {
        let skill_installer = SkillInstaller::new(config.config_home.join("skills"));
        Self { config, mcp_launcher: McpLauncher::new(), skill_installer, npm_registry: None }
    }

    /// 注入 NPM Registry 服务（用于下载 npm 包）
    #[must_use]
    pub fn with_npm_registry(mut self, registry: Arc<dyn NpmRegistryService>) -> Self {
        self.npm_registry = Some(registry);
        self
    }

    #[must_use]
    pub fn bundled_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bundled")
    }

    #[must_use]
    pub fn install_root(&self) -> PathBuf {
        self.config
            .install_root
            .clone()
            .unwrap_or_else(|| self.config.config_home.join("plugins").join("installed"))
    }

    #[must_use]
    pub fn registry_path(&self) -> PathBuf {
        self.config
            .registry_path
            .clone()
            .unwrap_or_else(|| self.config.config_home.join("plugins").join(REGISTRY_FILE_NAME))
    }

    #[must_use]
    pub fn settings_path(&self) -> PathBuf {
        self.config.config_home.join(SETTINGS_FILE_NAME)
    }

    pub fn plugin_registry(&self) -> Result<PluginRegistry, PluginError> {
        self.plugin_registry_report()?.into_registry()
    }

    pub fn validate_dependencies(&self, manifest: &PluginManifest) -> Result<(), PluginError> {
        if manifest.dependencies.is_empty() {
            return Ok(());
        }
        let registry = self.plugin_registry()?;
        let available: BTreeMap<&str, &str> = registry
            .plugins()
            .iter()
            .map(|p| (p.metadata().name.as_str(), p.metadata().version.as_str()))
            .collect();

        let mut errors = Vec::new();
        for dep in &manifest.dependencies {
            match available.get(dep.plugin_name.as_str()) {
                None => {
                    errors.push(PluginManifestValidationError::DependencyNotSatisfied {
                        plugin_name: dep.plugin_name.clone(),
                        min_version: dep.min_version.clone(),
                    });
                },
                Some(&installed_version) => {
                    if let Some(ref min_ver) = dep.min_version
                        && !version_satisfies(installed_version, min_ver)
                    {
                        errors.push(PluginManifestValidationError::DependencyNotSatisfied {
                            plugin_name: dep.plugin_name.clone(),
                            min_version: Some(min_ver.clone()),
                        });
                    }
                },
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(PluginError::ManifestValidation(errors))
        }
    }

    pub fn verify_integrity(
        &self,
        plugin_root: &Path,
        integrity: &PluginIntegrity,
    ) -> Result<(), PluginError> {
        match integrity.algorithm.as_str() {
            "sha256" => {
                let hash = hash_plugin_directory(plugin_root)?;
                if hash.eq_ignore_ascii_case(&integrity.hash) {
                    Ok(())
                } else {
                    Err(PluginError::ManifestValidation(vec![
                        PluginManifestValidationError::IntegrityCheckFailed {
                            algorithm: integrity.algorithm.clone(),
                            expected: integrity.hash.clone(),
                            actual: hash,
                        },
                    ]))
                }
            },
            other => {
                Err(PluginError::CommandFailed(format!("unsupported integrity algorithm: {other}")))
            },
        }
    }

    pub fn plugin_registry_report(&self) -> Result<PluginRegistryReport, PluginError> {
        self.sync_bundled_plugins()?;

        let mut discovery = PluginDiscovery::default();
        discovery.plugins.extend(builtin_plugins());

        let installed = self.discover_installed_plugins_with_failures()?;
        discovery.extend(installed);

        let external =
            self.discover_external_directory_plugins_with_failures(&discovery.plugins)?;
        discovery.extend(external);

        Ok(self.build_registry_report(discovery))
    }

    pub fn list_plugins(&self) -> Result<Vec<PluginSummary>, PluginError> {
        Ok(self.plugin_registry()?.summaries())
    }

    pub fn list_installed_plugins(&self) -> Result<Vec<PluginSummary>, PluginError> {
        Ok(self.installed_plugin_registry()?.summaries())
    }

    pub fn discover_plugins(&self) -> Result<Vec<PluginDefinition>, PluginError> {
        Ok(self.plugin_registry()?.plugins.into_iter().map(|plugin| plugin.definition).collect())
    }

    pub fn aggregated_hooks(&self) -> Result<PluginHooks, PluginError> {
        self.plugin_registry()?.aggregated_hooks()
    }

    pub fn aggregated_tools(&self) -> Result<Vec<PluginTool>, PluginError> {
        self.plugin_registry()?.aggregated_tools()
    }

    pub fn validate_plugin_source(&self, source: &str) -> Result<PluginManifest, PluginError> {
        let install_source = parse_install_source(source)?;
        match install_source {
            PluginInstallSource::LocalPath { path } => load_plugin_from_directory(&path),
            _ => {
                let temp_root = self.install_root().join(".tmp");
                let staged =
                    materialize_source(&install_source, &temp_root, self.npm_registry.as_ref())?;
                let manifest = load_plugin_from_directory(&staged)?;
                // 清理临时目录
                if staged.starts_with(&temp_root) {
                    let _ = std::fs::remove_dir_all(&staged);
                }
                Ok(manifest)
            },
        }
    }

    pub fn install(&mut self, source: &str) -> Result<InstallOutcome, PluginError> {
        let install_source = parse_install_source(source)?;
        let temp_root = self.install_root().join(".tmp");
        let staged_source =
            materialize_source(&install_source, &temp_root, self.npm_registry.as_ref())?;
        let cleanup_source = matches!(install_source, PluginInstallSource::GitUrl { .. });
        let manifest = load_plugin_from_directory(&staged_source)?;

        let plugin_id = plugin_id(&manifest.name, EXTERNAL_MARKETPLACE);
        let install_path = self.install_root().join(sanitize_plugin_id(&plugin_id));

        // P3 安全:install 冲突检测 — 若 plugin_id 已存在于 registry,
        // 返回 AlreadyExists 错误,提示用户使用 update 而非 install。
        // 防止恶意插件冒充合法插件(同名覆盖)。
        {
            let registry = self.load_registry()?;
            if registry.plugins.contains_key(&plugin_id) {
                return Err(PluginError::AlreadyExists(format!(
                    "plugin `{plugin_id}` is already installed, use `update` instead of `install`"
                )));
            }
        }

        if install_path.exists() {
            fs::remove_dir_all(&install_path)?;
        }
        copy_dir_all(&staged_source, &install_path)?;
        if cleanup_source {
            let _ = fs::remove_dir_all(&staged_source);
        }

        let now = unix_time_ms();
        let record = InstalledPluginRecord {
            kind: PluginKind::External,
            id: plugin_id.clone(),
            name: manifest.name,
            version: manifest.version.clone(),
            description: manifest.description,
            install_path: install_path.clone(),
            source: install_source,
            installed_at_unix_ms: now,
            updated_at_unix_ms: now,
        };

        let mut registry = self.load_registry()?;
        registry.plugins.insert(plugin_id.clone(), record);
        self.store_registry(&registry)?;
        self.write_enabled_state(&plugin_id, Some(true))?;
        self.config.enabled_plugins.insert(plugin_id.clone(), true);

        Ok(InstallOutcome { plugin_id, version: manifest.version, install_path })
    }

    pub fn enable(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        self.ensure_known_plugin(plugin_id)?;
        self.write_enabled_state(plugin_id, Some(true))?;
        self.config.enabled_plugins.insert(plugin_id.to_string(), true);
        let registry = self.load_registry()?;
        if let Some(record) = registry.plugins.get(plugin_id) {
            let manifest = load_plugin_from_directory(&record.install_path)?;
            self.start_plugin_locked(plugin_id, record, &manifest)?;
        }
        Ok(())
    }

    fn start_plugin_locked(
        &mut self,
        plugin_id: &str,
        record: &crate::types::InstalledPluginRecord,
        manifest: &crate::types::PluginManifest,
    ) -> Result<(), PluginError> {
        if !manifest.mcp_servers.is_empty() {
            self.mcp_launcher
                .start_plugin_mcps(plugin_id, &manifest.mcp_servers, &record.install_path)
                .map_err(|e| PluginError::CommandFailed(e.to_string()))?;
        }
        if !manifest.skills.is_empty() {
            self.skill_installer
                .install_plugin_skills(plugin_id, &manifest.skills, &record.install_path)
                .map_err(|e| PluginError::CommandFailed(e.to_string()))?;
        }
        if !manifest.agents.is_empty() {
            crate::agent_provider::register_plugin_agents_sync(plugin_id, &manifest.agents);
        }
        Ok(())
    }

    pub fn disable(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        self.mcp_launcher.stop_plugin_mcps(plugin_id);
        crate::agent_provider::unregister_plugin_agents_sync(plugin_id);
        self.skill_installer.remove_plugin_skills(plugin_id).ok();
        self.ensure_known_plugin(plugin_id)?;
        self.write_enabled_state(plugin_id, Some(false))?;
        self.config.enabled_plugins.insert(plugin_id.to_string(), false);
        Ok(())
    }

    pub fn start_enabled_plugins(&mut self) -> Result<Vec<String>, PluginError> {
        let registry = self.load_registry()?;
        let mut started = Vec::new();
        let mut errors = Vec::new();

        let enabled_ids: Vec<String> = self
            .config
            .enabled_plugins
            .iter()
            .filter(|entry| *entry.1)
            .map(|(id, _)| id.clone())
            .collect();

        for plugin_id in &enabled_ids {
            let Some(record) = registry.plugins.get(plugin_id) else {
                continue;
            };
            match load_plugin_from_directory(&record.install_path) {
                Ok(manifest) => {
                    if let Err(e) = self.start_plugin_locked(plugin_id, record, &manifest) {
                        tracing::warn!("Failed to start plugin `{plugin_id}`: {e}");
                        errors.push(format!("{plugin_id}: {e}"));
                    } else {
                        started.push(plugin_id.clone());
                        tracing::info!("Started enabled plugin: {plugin_id} v{}", record.version);
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to load manifest for enabled plugin `{plugin_id}`: {e}");
                    errors.push(format!("{plugin_id}: {e}"));
                },
            }
        }

        if errors.is_empty() {
            Ok(started)
        } else {
            tracing::warn!("Some plugins failed to start: {}", errors.join("; "));
            Ok(started)
        }
    }

    pub fn stop_all_plugins(&mut self) {
        let plugin_ids: Vec<String> = self
            .config
            .enabled_plugins
            .iter()
            .filter(|entry| *entry.1)
            .map(|(id, _)| id.clone())
            .collect();
        for plugin_id in plugin_ids {
            self.mcp_launcher.stop_plugin_mcps(&plugin_id);
            crate::agent_provider::unregister_plugin_agents_sync(&plugin_id);
            self.skill_installer.remove_plugin_skills(&plugin_id).ok();
            tracing::info!("Stopped plugin: {plugin_id}");
        }
    }

    pub fn uninstall(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        let mut registry = self.load_registry()?;
        let record = registry.plugins.remove(plugin_id).ok_or_else(|| {
            PluginError::NotFound(format!("plugin `{plugin_id}` is not installed"))
        })?;
        let remaining_plugins: Vec<&InstalledPluginRecord> =
            registry.plugins.values().filter(|r| r.id != plugin_id).collect();
        let mut dependents = Vec::new();
        for other in &remaining_plugins {
            if let Ok(other_manifest) = load_plugin_from_directory(&other.install_path) {
                for dep in &other_manifest.dependencies {
                    if dep.plugin_name == record.name {
                        dependents.push(other.name.clone());
                        break;
                    }
                }
            }
        }
        if !dependents.is_empty() {
            registry.plugins.insert(plugin_id.to_string(), record);
            return Err(PluginError::CommandFailed(format!(
                "cannot uninstall plugin `{plugin_id}`: other plugins depend on it: {}",
                dependents.join(", ")
            )));
        }
        if record.kind == PluginKind::Bundled {
            registry.plugins.insert(plugin_id.to_string(), record);
            return Err(PluginError::CommandFailed(format!(
                "plugin `{plugin_id}` is bundled and managed automatically; disable it instead"
            )));
        }
        self.mcp_launcher.stop_plugin_mcps(plugin_id);
        crate::agent_provider::unregister_plugin_agents_sync(plugin_id);
        self.skill_installer.remove_plugin_skills(plugin_id).ok();
        if record.install_path.exists() {
            fs::remove_dir_all(&record.install_path)?;
        }
        self.store_registry(&registry)?;
        self.write_enabled_state(plugin_id, None)?;
        self.config.enabled_plugins.remove(plugin_id);
        Ok(())
    }

    pub fn update(&mut self, plugin_id: &str) -> Result<UpdateOutcome, PluginError> {
        let mut registry = self.load_registry()?;
        let record = registry.plugins.get(plugin_id).cloned().ok_or_else(|| {
            PluginError::NotFound(format!("plugin `{plugin_id}` is not installed"))
        })?;

        let temp_root = self.install_root().join(".tmp");
        let staged_source =
            materialize_source(&record.source, &temp_root, self.npm_registry.as_ref())?;
        let cleanup_source = matches!(record.source, PluginInstallSource::GitUrl { .. });
        let manifest = load_plugin_from_directory(&staged_source)?;
        self.validate_dependencies(&manifest)?;

        // P3 安全:升级备份机制 — 在覆盖前备份旧版本到 `.bak` 目录,
        // 保留用户对 plugin.json / SKILL.md / hooks 的本地修改。
        // 备份失败不阻断升级(降级到原行为),仅 warn 日志。
        // 备份目录命名:{install_path}.bak,仅保留最近 1 个版本。
        let backup_path = record.install_path.with_extension("bak");
        if record.install_path.exists() {
            // 先清理旧的备份目录(若存在)
            if backup_path.exists() {
                let _ = fs::remove_dir_all(&backup_path);
            }
            match copy_dir_all(&record.install_path, &backup_path) {
                Ok(()) => {
                    tracing::info!(
                        "[plugin_update] plugin `{}` 旧版本已备份到 {}",
                        plugin_id,
                        backup_path.display()
                    );
                },
                Err(e) => {
                    tracing::warn!(
                        "[plugin_update] plugin `{}` 备份失败: {} — 继续升级,旧版本将被覆盖",
                        plugin_id,
                        e
                    );
                },
            }
            fs::remove_dir_all(&record.install_path)?;
        }
        copy_dir_all(&staged_source, &record.install_path)?;
        if cleanup_source {
            let _ = fs::remove_dir_all(&staged_source);
        }

        let updated_record = InstalledPluginRecord {
            version: manifest.version.clone(),
            description: manifest.description,
            updated_at_unix_ms: unix_time_ms(),
            ..record.clone()
        };
        registry.plugins.insert(plugin_id.to_string(), updated_record);
        self.store_registry(&registry)?;

        Ok(UpdateOutcome {
            plugin_id: plugin_id.to_string(),
            old_version: record.version,
            new_version: manifest.version,
            install_path: record.install_path,
        })
    }

    fn discover_installed_plugins_with_failures(&self) -> Result<PluginDiscovery, PluginError> {
        let mut registry = self.load_registry()?;
        let mut discovery = PluginDiscovery::default();
        let mut seen_ids = BTreeSet::<String>::new();
        let mut seen_paths = BTreeSet::<PathBuf>::new();
        let mut stale_registry_ids = Vec::new();

        for install_path in discover_plugin_dirs(&self.install_root())? {
            let matched_record =
                registry.plugins.values().find(|record| record.install_path == install_path);
            let kind = matched_record.map_or(PluginKind::External, |record| record.kind);
            let source = matched_record.map_or_else(
                || install_path.display().to_string(),
                |record| describe_install_source(&record.source),
            );
            match load_plugin_definition(&install_path, kind, source.clone(), kind.marketplace()) {
                Ok(plugin) => {
                    if seen_ids.insert(plugin.metadata().id.clone()) {
                        seen_paths.insert(install_path);
                        discovery.push_plugin(plugin);
                    }
                },
                Err(error) => {
                    discovery.push_failure(PluginLoadFailure::new(
                        install_path,
                        kind,
                        source,
                        error,
                    ));
                },
            }
        }

        for record in registry.plugins.values() {
            if seen_paths.contains(&record.install_path) {
                continue;
            }
            if !record.install_path.exists() || plugin_manifest_path(&record.install_path).is_err()
            {
                stale_registry_ids.push(record.id.clone());
                continue;
            }
            let source = describe_install_source(&record.source);
            match load_plugin_definition(
                &record.install_path,
                record.kind,
                source.clone(),
                record.kind.marketplace(),
            ) {
                Ok(plugin) => {
                    if seen_ids.insert(plugin.metadata().id.clone()) {
                        seen_paths.insert(record.install_path.clone());
                        discovery.push_plugin(plugin);
                    }
                },
                Err(error) => {
                    discovery.push_failure(PluginLoadFailure::new(
                        record.install_path.clone(),
                        record.kind,
                        source,
                        error,
                    ));
                },
            }
        }

        if !stale_registry_ids.is_empty() {
            for plugin_id in stale_registry_ids {
                registry.plugins.remove(&plugin_id);
            }
            self.store_registry(&registry)?;
        }

        Ok(discovery)
    }

    fn discover_external_directory_plugins_with_failures(
        &self,
        existing_plugins: &[PluginDefinition],
    ) -> Result<PluginDiscovery, PluginError> {
        let mut discovery = PluginDiscovery::default();

        for directory in &self.config.external_dirs {
            let source = Self::derive_source_from_external_dir(directory);
            for root in discover_plugin_dirs(directory)? {
                match load_plugin_definition(
                    &root,
                    PluginKind::External,
                    source.clone(),
                    EXTERNAL_MARKETPLACE,
                ) {
                    Ok(plugin) => {
                        if existing_plugins
                            .iter()
                            .chain(discovery.plugins.iter())
                            .all(|existing| existing.metadata().id != plugin.metadata().id)
                        {
                            discovery.push_plugin(plugin);
                        }
                    },
                    Err(error) => {
                        discovery.push_failure(PluginLoadFailure::new(
                            root,
                            PluginKind::External,
                            source.clone(),
                            error,
                        ));
                    },
                }
            }
        }

        Ok(discovery)
    }

    fn derive_source_from_external_dir(directory: &Path) -> String {
        let dir_str = directory.to_string_lossy().to_lowercase();
        // 按优先级匹配，避免 .codebuddy 被 .claude 误匹配
        if dir_str.contains(".axagent") {
            "axagent".to_string()
        } else if dir_str.contains(".claude") {
            "claude".to_string()
        } else if dir_str.contains(".agents") {
            "agents".to_string()
        } else if dir_str.contains(".codebuddy") {
            "codebuddy".to_string()
        } else if dir_str.contains(".trae") {
            "trae".to_string()
        } else if dir_str.contains(".workbuddy") {
            "workbuddy".to_string()
        } else {
            // Fallback: use the parent directory name (e.g. ".custom" from "~/.custom/skills")
            directory
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        }
    }

    pub fn installed_plugin_registry_report(&self) -> Result<PluginRegistryReport, PluginError> {
        self.sync_bundled_plugins()?;
        Ok(self.build_registry_report(self.discover_installed_plugins_with_failures()?))
    }

    fn sync_bundled_plugins(&self) -> Result<(), PluginError> {
        let bundled_root = self.config.bundled_root.clone().unwrap_or_else(Self::bundled_root);
        let bundled_plugins = discover_plugin_dirs(&bundled_root)?;
        let mut registry = self.load_registry()?;
        let mut changed = false;
        let install_root = self.install_root();
        let mut active_bundled_ids = BTreeSet::new();

        for source_root in bundled_plugins {
            let manifest = load_plugin_from_directory(&source_root)?;
            let plugin_id = plugin_id(&manifest.name, BUNDLED_MARKETPLACE);
            active_bundled_ids.insert(plugin_id.clone());
            let install_path = install_root.join(sanitize_plugin_id(&plugin_id));
            let now = unix_time_ms();
            let existing_record = registry.plugins.get(&plugin_id);
            let installed_copy_is_valid =
                install_path.exists() && load_plugin_from_directory(&install_path).is_ok();
            let needs_sync = existing_record.is_none_or(|record| {
                record.kind != PluginKind::Bundled
                    || record.version != manifest.version
                    || record.name != manifest.name
                    || record.description != manifest.description
                    || record.install_path != install_path
                    || !record.install_path.exists()
                    || !installed_copy_is_valid
            });

            if !needs_sync {
                continue;
            }

            if install_path.exists() {
                fs::remove_dir_all(&install_path)?;
            }
            copy_dir_all(&source_root, &install_path)?;

            let installed_at_unix_ms =
                existing_record.map_or(now, |record| record.installed_at_unix_ms);
            registry.plugins.insert(
                plugin_id.clone(),
                InstalledPluginRecord {
                    kind: PluginKind::Bundled,
                    id: plugin_id,
                    name: manifest.name,
                    version: manifest.version,
                    description: manifest.description,
                    install_path,
                    source: PluginInstallSource::LocalPath { path: source_root },
                    installed_at_unix_ms,
                    updated_at_unix_ms: now,
                },
            );
            changed = true;
        }

        let stale_bundled_ids = registry
            .plugins
            .iter()
            .filter_map(|(plugin_id, record)| {
                (record.kind == PluginKind::Bundled && !active_bundled_ids.contains(plugin_id))
                    .then_some(plugin_id.clone())
            })
            .collect::<Vec<_>>();

        for plugin_id in stale_bundled_ids {
            if let Some(record) = registry.plugins.remove(&plugin_id) {
                if record.install_path.exists() {
                    fs::remove_dir_all(&record.install_path)?;
                }
                changed = true;
            }
        }

        if changed {
            self.store_registry(&registry)?;
        }

        Ok(())
    }

    fn is_enabled(&self, metadata: &PluginMetadata) -> bool {
        self.config.enabled_plugins.get(&metadata.id).copied().unwrap_or(match metadata.kind {
            PluginKind::External => false,
            PluginKind::Builtin | PluginKind::Bundled => metadata.default_enabled,
        })
    }

    fn ensure_known_plugin(&self, plugin_id: &str) -> Result<(), PluginError> {
        if self.plugin_registry()?.contains(plugin_id) {
            Ok(())
        } else {
            Err(PluginError::NotFound(format!(
                "plugin `{plugin_id}` is not installed or discoverable"
            )))
        }
    }

    pub(crate) fn load_registry(&self) -> Result<InstalledPluginRegistry, PluginError> {
        let path = self.registry_path();
        match fs::read_to_string(&path) {
            Ok(contents) if contents.trim().is_empty() => Ok(InstalledPluginRegistry::default()),
            Ok(contents) => Ok(serde_json::from_str(&contents)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(InstalledPluginRegistry::default())
            },
            Err(error) => Err(PluginError::Io(error)),
        }
    }

    pub(crate) fn store_registry(
        &self,
        registry: &InstalledPluginRegistry,
    ) -> Result<(), PluginError> {
        let path = self.registry_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(registry)?)?;
        Ok(())
    }

    pub(crate) fn write_enabled_state(
        &self,
        plugin_id: &str,
        enabled: Option<bool>,
    ) -> Result<(), PluginError> {
        update_settings_json(&self.settings_path(), |root| {
            let enabled_plugins = ensure_object(root, "enabledPlugins");
            match enabled {
                Some(value) => {
                    enabled_plugins.insert(plugin_id.to_string(), Value::Bool(value));
                },
                None => {
                    enabled_plugins.remove(plugin_id);
                },
            }
        })
    }

    fn installed_plugin_registry(&self) -> Result<PluginRegistry, PluginError> {
        self.installed_plugin_registry_report()?.into_registry()
    }

    fn build_registry_report(&self, discovery: PluginDiscovery) -> PluginRegistryReport {
        PluginRegistryReport::new(
            PluginRegistry::new(
                discovery
                    .plugins
                    .into_iter()
                    .map(|plugin| {
                        let enabled = self.is_enabled(plugin.metadata());
                        RegisteredPlugin::new(plugin, enabled)
                    })
                    .collect(),
            ),
            discovery.failures,
        )
    }
}

#[must_use]
pub fn builtin_plugins() -> Vec<PluginDefinition> {
    vec![PluginDefinition::Builtin(BuiltinPlugin {
        metadata: PluginMetadata {
            id: plugin_id("example-builtin", BUILTIN_MARKETPLACE),
            name: "example-builtin".to_string(),
            version: "0.1.0".to_string(),
            description: "Example built-in plugin scaffold for the Rust plugin system".to_string(),
            kind: PluginKind::Builtin,
            source: BUILTIN_MARKETPLACE.to_string(),
            default_enabled: false,
            root: None,
        },
        hooks: PluginHooks::default(),
        lifecycle: PluginLifecycle::default(),
        tools: Vec::new(),
        mcp_servers: Vec::new(),
        skills: Vec::new(),
        permissions: Vec::new(),
    })]
}

fn load_plugin_definition(
    root: &Path,
    kind: PluginKind,
    source: String,
    marketplace: &str,
) -> Result<PluginDefinition, PluginError> {
    let manifest = load_plugin_from_directory(root)?;
    let metadata = PluginMetadata {
        id: plugin_id(&manifest.name, marketplace),
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        kind,
        source,
        default_enabled: manifest.default_enabled,
        root: Some(root.to_path_buf()),
    };
    let hooks = resolve_hooks(root, &manifest.hooks);
    let lifecycle = resolve_lifecycle(root, &manifest.lifecycle);
    let tools = resolve_tools(root, &metadata.id, &metadata.name, &manifest.tools);
    let mcp_servers = manifest.mcp_servers;
    let skills = manifest.skills;
    let permissions = manifest.permissions;
    Ok(match kind {
        PluginKind::Builtin => PluginDefinition::Builtin(BuiltinPlugin {
            metadata,
            hooks,
            lifecycle,
            tools,
            mcp_servers,
            skills,
            permissions,
        }),
        PluginKind::Bundled => PluginDefinition::Bundled(BundledPlugin {
            metadata,
            hooks,
            lifecycle,
            tools,
            mcp_servers,
            skills,
            permissions,
        }),
        PluginKind::External => PluginDefinition::External(ExternalPlugin {
            metadata,
            hooks,
            lifecycle,
            tools,
            mcp_servers,
            skills,
            permissions,
        }),
    })
}

pub fn load_plugin_from_directory(root: &Path) -> Result<PluginManifest, PluginError> {
    load_manifest_from_directory(root)
}

fn load_manifest_from_directory(root: &Path) -> Result<PluginManifest, PluginError> {
    let manifest_path = plugin_manifest_path(root)?;
    if manifest_path.ends_with("SKILL.md") {
        load_manifest_from_skill_md(root, &manifest_path)
    } else {
        load_manifest_from_path(root, &manifest_path)
    }
}

// ── SKILL.md 解析 (Claude Code 兼容) ──

fn load_manifest_from_skill_md(
    root: &Path,
    manifest_path: &Path,
) -> Result<PluginManifest, PluginError> {
    let contents = fs::read_to_string(manifest_path).map_err(|error| {
        PluginError::NotFound(format!("SKILL.md not found at {}: {error}", manifest_path.display()))
    })?;

    let dir_name = root.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");

    let mut name = String::from(dir_name);
    let mut description = String::new();
    let mut version = String::from("1.0.0");
    let mut permissions: Vec<String> = Vec::new();
    let mut pre_tool_use: Vec<String> = Vec::new();
    let mut post_tool_use: Vec<String> = Vec::new();
    let mut post_tool_use_failure: Vec<String> = Vec::new();
    let mut init_commands: Vec<String> = Vec::new();
    let mut shutdown_commands: Vec<String> = Vec::new();
    let mut default_enabled = true;

    if let Some(frontmatter) = extract_yaml_frontmatter(&contents) {
        let parsed = parse_yaml_frontmatter(&frontmatter);

        if let Some(v) = parsed.get("name").and_then(|v| v.as_str()) {
            name = v.to_string();
        }
        if let Some(v) = parsed.get("description").and_then(|v| v.as_str()) {
            description = v.to_string();
        }
        if let Some(v) = parsed.get("version").and_then(|v| v.as_str()) {
            version = v.to_string();
        }
        if let Some(v) = parsed.get("default_enabled").or(parsed.get("defaultEnabled")) {
            default_enabled =
                v.as_bool().unwrap_or(v.as_str().is_none_or(|s| s.eq_ignore_ascii_case("true")));
        }

        for key in &["permissions"] {
            if let Some(items) = parsed.get(*key).and_then(|v| v.as_array()) {
                for item in items {
                    if let Some(s) = item.as_str() {
                        permissions.push(s.to_string());
                    }
                }
            }
        }

        for (target, key_names) in [
            (&mut pre_tool_use, vec!["pre_tool_use", "PreToolUse"]),
            (&mut post_tool_use, vec!["post_tool_use", "PostToolUse"]),
            (&mut post_tool_use_failure, vec!["post_tool_use_failure", "PostToolUseFailure"]),
            (&mut init_commands, vec!["init", "Init"]),
            (&mut shutdown_commands, vec!["shutdown", "Shutdown"]),
        ] {
            for key_name in key_names {
                if let Some(items) = parsed.get(key_name).and_then(|v| v.as_array()) {
                    for item in items {
                        if let Some(s) = item.as_str() {
                            target.push(s.to_string());
                        }
                    }
                }
            }
        }
    }

    let manifest = PluginManifest {
        name,
        description: if description.is_empty() {
            format!("SKILL.md 技能: {}", dir_name)
        } else {
            description
        },
        version,
        permissions: permissions.iter().filter_map(|p| PluginPermission::parse(p)).collect(),
        default_enabled,
        hooks: PluginHooks { pre_tool_use, post_tool_use, post_tool_use_failure },
        lifecycle: PluginLifecycle { init: init_commands, shutdown: shutdown_commands },
        tools: Vec::new(),
        commands: Vec::new(),
        scenarios: Vec::new(),
        mcp_servers: Vec::new(),
        skills: Vec::new(),
        agents: Vec::new(),
        dashboard_panels: Vec::new(),
        dependencies: Vec::new(),
        integrity: None,
    };
    Ok(manifest)
}

fn extract_yaml_frontmatter(contents: &str) -> Option<String> {
    let lines: Vec<&str> = contents.lines().collect();
    if lines.first().map(|l| l.trim()) != Some("---") {
        return None;
    }
    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }
    let end = end_idx?;
    Some(lines[1..end].join("\n"))
}

#[derive(Debug, Clone)]
enum YamlValue {
    String(String),
    Bool(bool),
    Array(Vec<YamlValue>),
}

impl YamlValue {
    fn as_str(&self) -> Option<&str> {
        match self {
            YamlValue::String(s) => Some(s),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            YamlValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&Vec<YamlValue>> {
        match self {
            YamlValue::Array(arr) => Some(arr),
            _ => None,
        }
    }
}

fn parse_yaml_frontmatter(frontmatter: &str) -> std::collections::HashMap<String, YamlValue> {
    let mut result = std::collections::HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_array: Vec<YamlValue> = Vec::new();

    for line in frontmatter.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if let Some(key) = current_key.take()
                && !current_array.is_empty()
            {
                result.insert(key, YamlValue::Array(std::mem::take(&mut current_array)));
            }
            continue;
        }

        if let Some(stripped) = trimmed.strip_prefix("- ") {
            if current_key.is_some() {
                let item = parse_yaml_scalar(stripped.trim());
                current_array.push(item);
            }
            continue;
        }

        if let Some(key) = current_key.take()
            && !current_array.is_empty()
        {
            result.insert(key, YamlValue::Array(std::mem::take(&mut current_array)));
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim().to_string();
            let value_part = value.trim();
            if value_part.is_empty() {
                current_key = Some(key);
                current_array.clear();
            } else {
                result.insert(key, parse_yaml_scalar(value_part));
            }
        }
    }

    if let Some(key) = current_key.take()
        && !current_array.is_empty()
    {
        result.insert(key, YamlValue::Array(current_array));
    }

    result
}

fn parse_yaml_scalar(value: &str) -> YamlValue {
    let unquoted = value.trim_matches('"').trim_matches('\'');
    if unquoted.eq_ignore_ascii_case("true") {
        YamlValue::Bool(true)
    } else if unquoted.eq_ignore_ascii_case("false") {
        YamlValue::Bool(false)
    } else {
        YamlValue::String(unquoted.to_string())
    }
}

fn load_manifest_from_path(
    root: &Path,
    manifest_path: &Path,
) -> Result<PluginManifest, PluginError> {
    let contents = fs::read_to_string(manifest_path).map_err(|error| {
        PluginError::NotFound(format!(
            "plugin manifest not found at {}: {error}",
            manifest_path.display()
        ))
    })?;
    let raw_json: Value = serde_json::from_str(&contents)?;
    let compatibility_errors = detect_claude_code_manifest_contract_gaps(&raw_json);
    if !compatibility_errors.is_empty() {
        return Err(PluginError::ManifestValidation(compatibility_errors));
    }
    let raw_manifest: RawPluginManifest = serde_json::from_value(raw_json)?;
    build_plugin_manifest(root, raw_manifest)
}

pub(crate) fn detect_claude_code_manifest_contract_gaps(
    raw_manifest: &Value,
) -> Vec<PluginManifestValidationError> {
    let Some(root) = raw_manifest.as_object() else {
        return Vec::new();
    };

    let mut errors = Vec::new();

    if root
        .get("commands")
        .and_then(Value::as_array)
        .is_some_and(|commands| commands.iter().any(Value::is_string))
    {
        errors.push(PluginManifestValidationError::UnsupportedManifestContract {
            detail: "plugin manifest field `commands` uses Claude Code-style directory globs; `claw` slash dispatch is still built-in and does not load plugin slash command markdown files.".to_string(),
        });
    }

    if let Some(hooks) = root.get("hooks").and_then(Value::as_object) {
        for hook_name in hooks.keys() {
            if !matches!(hook_name.as_str(), "PreToolUse" | "PostToolUse" | "PostToolUseFailure") {
                errors.push(PluginManifestValidationError::UnsupportedManifestContract {
                    detail: format!(
                        "plugin hook `{hook_name}` uses the Claude Code lifecycle contract; `claw` plugins currently support only PreToolUse, PostToolUse, and PostToolUseFailure."
                    ),
                });
            }
        }
    }

    errors
}

fn plugin_manifest_path(root: &Path) -> Result<PathBuf, PluginError> {
    let direct_path = root.join(MANIFEST_FILE_NAME);
    if direct_path.exists() {
        return Ok(direct_path);
    }

    let packaged_path = root.join(MANIFEST_RELATIVE_PATH);
    if packaged_path.exists() {
        return Ok(packaged_path);
    }

    // Claude Code 兼容：检查 SKILL.md
    let skill_md_path = root.join(SKILL_MD_FILE_NAME);
    if skill_md_path.exists() {
        return Ok(skill_md_path);
    }

    Err(PluginError::NotFound(format!(
        "plugin manifest not found at {} or {}",
        direct_path.display(),
        packaged_path.display()
    )))
}

fn build_plugin_manifest(
    root: &Path,
    raw: RawPluginManifest,
) -> Result<PluginManifest, PluginError> {
    let mut errors = Vec::new();

    validate_required_manifest_field("name", &raw.name, &mut errors);
    validate_required_manifest_field("version", &raw.version, &mut errors);
    validate_required_manifest_field("description", &raw.description, &mut errors);

    let permissions = build_manifest_permissions(&raw.permissions, &mut errors);
    validate_command_entries(root, raw.hooks.pre_tool_use.iter(), "hook", &mut errors);
    validate_command_entries(root, raw.hooks.post_tool_use.iter(), "hook", &mut errors);
    validate_command_entries(root, raw.hooks.post_tool_use_failure.iter(), "hook", &mut errors);
    validate_command_entries(root, raw.lifecycle.init.iter(), "lifecycle command", &mut errors);
    validate_command_entries(root, raw.lifecycle.shutdown.iter(), "lifecycle command", &mut errors);
    let tools = build_manifest_tools(root, raw.tools, &mut errors);
    let commands = build_manifest_commands(root, raw.commands, &mut errors);

    if !errors.is_empty() {
        return Err(PluginError::ManifestValidation(errors));
    }

    Ok(PluginManifest {
        name: raw.name,
        version: raw.version,
        description: raw.description,
        permissions,
        default_enabled: raw.default_enabled,
        hooks: raw.hooks,
        lifecycle: raw.lifecycle,
        tools,
        commands,
        scenarios: raw.scenarios,
        mcp_servers: raw
            .mcp_servers
            .into_iter()
            .map(|r| PluginMcpServer {
                name: r.name,
                command: r.command,
                args: r.args,
                env: r.env,
                cwd: r.cwd,
            })
            .collect(),
        skills: raw
            .skills
            .into_iter()
            .map(|r| PluginSkillEntry { name: r.name, path: r.path })
            .collect(),
        agents: raw
            .agents
            .into_iter()
            .map(|r| PluginAgentDefInternal {
                agent_type: r.agent_type,
                description: r.description,
                tools: r.tools,
                disallowed_tools: r.disallowed_tools,
                model: r.model,
                background: r.background,
                system_prompt: r.system_prompt,
            })
            .collect(),
        dashboard_panels: raw.dashboard_panels,
        dependencies: raw.dependencies,
        integrity: raw.integrity,
    })
}

fn validate_required_manifest_field(
    field: &'static str,
    value: &str,
    errors: &mut Vec<PluginManifestValidationError>,
) {
    if value.trim().is_empty() {
        errors.push(PluginManifestValidationError::EmptyField { field });
    }
}

fn build_manifest_permissions(
    permissions: &[String],
    errors: &mut Vec<PluginManifestValidationError>,
) -> Vec<PluginPermission> {
    let mut seen = BTreeSet::new();
    let mut validated = Vec::new();

    for permission in permissions {
        let permission = permission.trim();
        if permission.is_empty() {
            errors.push(PluginManifestValidationError::EmptyEntryField {
                kind: "permission",
                field: "value",
                name: None,
            });
            continue;
        }
        if !seen.insert(permission.to_string()) {
            errors.push(PluginManifestValidationError::DuplicatePermission {
                permission: permission.to_string(),
            });
            continue;
        }
        match PluginPermission::parse(permission) {
            Some(permission) => validated.push(permission),
            None => errors.push(PluginManifestValidationError::InvalidPermission {
                permission: permission.to_string(),
            }),
        }
    }

    validated
}

fn build_manifest_tools(
    root: &Path,
    tools: Vec<RawPluginToolManifest>,
    errors: &mut Vec<PluginManifestValidationError>,
) -> Vec<PluginToolManifest> {
    let mut seen = BTreeSet::new();
    let mut validated = Vec::new();

    for tool in tools {
        let name = tool.name.trim().to_string();
        if name.is_empty() {
            errors.push(PluginManifestValidationError::EmptyEntryField {
                kind: "tool",
                field: "name",
                name: None,
            });
            continue;
        }
        if !seen.insert(name.clone()) {
            errors.push(PluginManifestValidationError::DuplicateEntry { kind: "tool", name });
            continue;
        }
        if tool.description.trim().is_empty() {
            errors.push(PluginManifestValidationError::EmptyEntryField {
                kind: "tool",
                field: "description",
                name: Some(name.clone()),
            });
        }
        if tool.command.trim().is_empty() {
            errors.push(PluginManifestValidationError::EmptyEntryField {
                kind: "tool",
                field: "command",
                name: Some(name.clone()),
            });
        } else {
            validate_command_entry(root, &tool.command, "tool", errors);
        }
        if !tool.input_schema.is_object() {
            errors.push(PluginManifestValidationError::InvalidToolInputSchema {
                tool_name: name.clone(),
            });
        }
        let Some(required_permission) =
            PluginToolPermission::parse(tool.required_permission.trim())
        else {
            errors.push(PluginManifestValidationError::InvalidToolRequiredPermission {
                tool_name: name.clone(),
                permission: tool.required_permission.trim().to_string(),
            });
            continue;
        };

        validated.push(PluginToolManifest {
            name,
            description: tool.description,
            input_schema: tool.input_schema,
            command: tool.command,
            args: tool.args,
            required_permission,
        });
    }

    validated
}

fn build_manifest_commands(
    root: &Path,
    commands: Vec<PluginCommandManifest>,
    errors: &mut Vec<PluginManifestValidationError>,
) -> Vec<PluginCommandManifest> {
    let mut seen = BTreeSet::new();
    let mut validated = Vec::new();

    for command in commands {
        let name = command.name.trim().to_string();
        if name.is_empty() {
            errors.push(PluginManifestValidationError::EmptyEntryField {
                kind: "command",
                field: "name",
                name: None,
            });
            continue;
        }
        if !seen.insert(name.clone()) {
            errors.push(PluginManifestValidationError::DuplicateEntry { kind: "command", name });
            continue;
        }
        if command.description.trim().is_empty() {
            errors.push(PluginManifestValidationError::EmptyEntryField {
                kind: "command",
                field: "description",
                name: Some(name.clone()),
            });
        }
        if command.command.trim().is_empty() {
            errors.push(PluginManifestValidationError::EmptyEntryField {
                kind: "command",
                field: "command",
                name: Some(name.clone()),
            });
        } else {
            validate_command_entry(root, &command.command, "command", errors);
        }
        validated.push(command);
    }

    validated
}

fn validate_command_entries<'a>(
    root: &Path,
    entries: impl Iterator<Item = &'a String>,
    kind: &'static str,
    errors: &mut Vec<PluginManifestValidationError>,
) {
    for entry in entries {
        validate_command_entry(root, entry, kind, errors);
    }
}

fn validate_command_entry(
    root: &Path,
    entry: &str,
    kind: &'static str,
    errors: &mut Vec<PluginManifestValidationError>,
) {
    if entry.trim().is_empty() {
        errors.push(PluginManifestValidationError::EmptyEntryField {
            kind,
            field: "command",
            name: None,
        });
        return;
    }
    if is_literal_command(entry) {
        return;
    }

    let path = if Path::new(entry).is_absolute() {
        PathBuf::from(entry)
    } else {
        root.join(entry)
    };
    if !path.exists() {
        errors.push(PluginManifestValidationError::MissingPath { kind, path });
    } else if !path.is_file() {
        errors.push(PluginManifestValidationError::PathIsDirectory { kind, path });
    }
}

fn resolve_hooks(root: &Path, hooks: &PluginHooks) -> PluginHooks {
    PluginHooks {
        pre_tool_use: hooks
            .pre_tool_use
            .iter()
            .map(|entry| resolve_hook_entry(root, entry))
            .collect(),
        post_tool_use: hooks
            .post_tool_use
            .iter()
            .map(|entry| resolve_hook_entry(root, entry))
            .collect(),
        post_tool_use_failure: hooks
            .post_tool_use_failure
            .iter()
            .map(|entry| resolve_hook_entry(root, entry))
            .collect(),
    }
}

fn resolve_lifecycle(root: &Path, lifecycle: &PluginLifecycle) -> PluginLifecycle {
    PluginLifecycle {
        init: lifecycle.init.iter().map(|entry| resolve_hook_entry(root, entry)).collect(),
        shutdown: lifecycle.shutdown.iter().map(|entry| resolve_hook_entry(root, entry)).collect(),
    }
}

fn resolve_tools(
    root: &Path,
    plugin_id: &str,
    plugin_name: &str,
    tools: &[PluginToolManifest],
) -> Vec<PluginTool> {
    tools
        .iter()
        .map(|tool| {
            PluginTool::new(
                plugin_id,
                plugin_name,
                PluginToolDefinition {
                    name: tool.name.clone(),
                    description: Some(tool.description.clone()),
                    input_schema: tool.input_schema.clone(),
                },
                resolve_hook_entry(root, &tool.command),
                tool.args.clone(),
                tool.required_permission,
                Some(root.to_path_buf()),
            )
        })
        .collect()
}

pub fn validate_hook_paths(root: Option<&Path>, hooks: &PluginHooks) -> Result<(), PluginError> {
    let Some(root) = root else {
        return Ok(());
    };
    for entry in hooks
        .pre_tool_use
        .iter()
        .chain(hooks.post_tool_use.iter())
        .chain(hooks.post_tool_use_failure.iter())
    {
        validate_command_path(root, entry, "hook")?;
    }
    Ok(())
}

pub fn validate_lifecycle_paths(
    root: Option<&Path>,
    lifecycle: &PluginLifecycle,
) -> Result<(), PluginError> {
    let Some(root) = root else {
        return Ok(());
    };
    for entry in lifecycle.init.iter().chain(lifecycle.shutdown.iter()) {
        validate_command_path(root, entry, "lifecycle command")?;
    }
    Ok(())
}

pub fn validate_tool_paths(root: Option<&Path>, tools: &[PluginTool]) -> Result<(), PluginError> {
    let Some(root) = root else {
        return Ok(());
    };
    for tool in tools {
        validate_command_path(root, &tool.command, "tool")?;
    }
    Ok(())
}

fn validate_command_path(root: &Path, entry: &str, kind: &str) -> Result<(), PluginError> {
    if is_literal_command(entry) {
        return Ok(());
    }
    let path = if Path::new(entry).is_absolute() {
        PathBuf::from(entry)
    } else {
        root.join(entry)
    };
    if !path.exists() {
        return Err(PluginError::InvalidManifest(format!(
            "{kind} path `{}` does not exist",
            path.display()
        )));
    }
    if !path.is_file() {
        return Err(PluginError::InvalidManifest(format!(
            "{kind} path `{}` must point to a file",
            path.display()
        )));
    }
    Ok(())
}

fn resolve_hook_entry(root: &Path, entry: &str) -> String {
    if is_literal_command(entry) {
        entry.to_string()
    } else {
        root.join(entry).display().to_string()
    }
}

fn is_literal_command(entry: &str) -> bool {
    !entry.starts_with("./") && !entry.starts_with("../") && !Path::new(entry).is_absolute()
}

pub fn run_lifecycle_commands(
    metadata: &PluginMetadata,
    lifecycle: &PluginLifecycle,
    phase: &str,
    commands: &[String],
) -> Result<(), PluginError> {
    if lifecycle.is_empty() || commands.is_empty() {
        return Ok(());
    }

    for command in commands {
        // SECURITY: 不再通过 `cmd /C` 或 `sh -lc` 执行命令，避免 shell 解析
        // `&`、`|`、`$()`、反引号等特殊字符导致命令注入。
        // 将命令字符串按空白分割为程序 + 参数数组，直接 exec。
        // 插件 lifecycle 命令来自插件配置文件（plugin.json），虽然用户安装时会确认，
        // 但配置文件可能被篡改，因此仍需防护。
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let executable = parts[0];
        let args = &parts[1..];

        let mut process = Command::new(executable);
        process.args(args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            process.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        if let Some(root) = &metadata.root {
            process.current_dir(root);
        }
        let output = process.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(PluginError::CommandFailed(format!(
                "plugin `{}` {} failed for `{}`: {}",
                metadata.id,
                phase,
                command,
                if stderr.is_empty() {
                    format!("exit status {}", output.status)
                } else {
                    stderr
                }
            )));
        }
    }

    Ok(())
}

/// 在沙箱约束下执行 lifecycle 命令（init / shutdown）。
///
/// 与 [`run_lifecycle_commands`] 的区别：
/// 1. 执行前调用 [`check_subprocess_permission`]，未声明 `subprocess_execution`
///    权限时直接返回 [`PluginError::PermissionDenied`]，不启动子进程。
/// 2. 通过 [`apply_env_to_command`] 对子进程 ENV 做白名单过滤，屏蔽
///    API Key / Token / Secret 等敏感变量。
///
/// 原始 [`run_lifecycle_commands`] 保持不变以维持向后兼容；需要沙箱隔离的
/// 调用方（如基于 manifest 权限构建沙箱后执行 lifecycle）应使用本函数。
pub fn run_lifecycle_commands_sandboxed(
    metadata: &PluginMetadata,
    lifecycle: &PluginLifecycle,
    phase: &str,
    commands: &[String],
    sandbox: &SandboxConfig,
) -> Result<(), PluginError> {
    if lifecycle.is_empty() || commands.is_empty() {
        return Ok(());
    }

    // 沙箱检查：未声明 subprocess_execution 权限时禁止执行 lifecycle 命令
    check_subprocess_permission(sandbox)?;

    for command in commands {
        // SECURITY: 同 run_lifecycle_commands，直接 exec 避免 shell 注入
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let executable = parts[0];
        let args = &parts[1..];

        let mut process = Command::new(executable);
        process.args(args);
        // 沙箱：ENV 白名单过滤（env_clear + 回填白名单变量）
        apply_env_to_command(&mut process, sandbox);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            process.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        if let Some(root) = &metadata.root {
            process.current_dir(root);
        }
        let output = process.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(PluginError::CommandFailed(format!(
                "plugin `{}` {} failed for `{}`: {}",
                metadata.id,
                phase,
                command,
                if stderr.is_empty() {
                    format!("exit status {}", output.status)
                } else {
                    stderr
                }
            )));
        }
    }

    Ok(())
}

fn resolve_local_source(source: &str) -> Result<PathBuf, PluginError> {
    let path = PathBuf::from(source);
    if path.exists() {
        Ok(path)
    } else {
        Err(PluginError::NotFound(format!("plugin source `{source}` was not found")))
    }
}

fn looks_like_npm_spec(source: &str) -> bool {
    // 排除明显是其他格式的源
    if source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("git@")
        || source.starts_with('/')
        || source.starts_with('.')
        || source.starts_with('\\')
        || source.contains("://")
    {
        return false;
    }
    // npm 包：@scope/name 或 name@version 或裸包名
    // 裸包名不含路径分隔符 / 和 \
    if source.starts_with('@') || source.contains('@') {
        return true;
    }
    // 裸包名：不含路径分隔符且不含空格
    !source.contains('/') && !source.contains('\\') && !source.contains(' ')
}

pub(crate) fn parse_install_source(source: &str) -> Result<PluginInstallSource, PluginError> {
    // npm 包检测
    if looks_like_npm_spec(source) {
        let (name, version) = parse_npm_package_spec(source);
        return Ok(PluginInstallSource::NpmPackage {
            name: name.to_string(),
            version: version.map(|v| v.to_string()),
        });
    }
    if source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("git@")
        || Path::new(source)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("git"))
    {
        Ok(PluginInstallSource::GitUrl { url: source.to_string() })
    } else {
        Ok(PluginInstallSource::LocalPath { path: resolve_local_source(source)? })
    }
}

fn materialize_source(
    source: &PluginInstallSource,
    temp_root: &Path,
    npm_registry: Option<&Arc<dyn NpmRegistryService>>,
) -> Result<PathBuf, PluginError> {
    fs::create_dir_all(temp_root)?;
    match source {
        PluginInstallSource::LocalPath { path } => Ok(path.clone()),
        PluginInstallSource::GitUrl { url } => {
            // URL 合法性校验：阻止 `git clone` 把 `--upload-pack=...` 之类的 flag
            // 误当作 URL 参数解析（任何以 `-` 开头的 token 都不是合法 URL）。
            if url.is_empty() || url.starts_with('-') {
                return Err(PluginError::InvalidManifest(format!(
                    "git url must not be empty or start with '-': `{url}`"
                )));
            }
            static MATERIALIZE_COUNTER: AtomicU64 = AtomicU64::new(0);
            let unique = MATERIALIZE_COUNTER.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time is after UNIX epoch")
                .as_nanos();
            let destination = temp_root.join(format!("plugin-{nanos}-{unique}"));
            // `--` 终止 git 的 option 解析，后续 token 强制视为位置参数（URL/dest）
            let output = Command::new("git")
                .arg("clone")
                .arg("--depth")
                .arg("1")
                .arg("--")
                .arg(url)
                .arg(&destination)
                .output()?;
            if !output.status.success() {
                return Err(PluginError::CommandFailed(format!(
                    "git clone failed for `{url}`: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                )));
            }
            Ok(destination)
        },
        PluginInstallSource::NpmPackage { name, version } => {
            let name = name.clone();
            let version = version.clone();
            let dest = temp_root.join(format!("npm-{}", sanitize_plugin_id(&name)));
            let dest_clone = dest.clone();
            let service = npm_registry.cloned();
            match service {
                Some(registry) => {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| {
                            PluginError::CommandFailed(format!(
                                "Failed to create runtime for npm download: {e}"
                            ))
                        })?;
                    rt.block_on(async {
                        registry
                            .download_package(&name, version.as_deref(), &dest)
                            .await
                            .map_err(PluginError::CommandFailed)?;
                        Ok(dest_clone)
                    })
                },
                None => Err(PluginError::CommandFailed(
                    "NPM registry service is not configured".to_string(),
                )),
            }
        },
    }
}

fn discover_plugin_dirs(root: &Path) -> Result<Vec<PathBuf>, PluginError> {
    match fs::read_dir(root) {
        Ok(entries) => {
            let mut paths = Vec::new();
            for entry in entries {
                let path = entry?.path();
                if path.is_dir() && plugin_manifest_path(&path).is_ok() {
                    paths.push(path);
                }
            }
            paths.sort();
            Ok(paths)
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(PluginError::Io(error)),
    }
}

fn plugin_id(name: &str, marketplace: &str) -> String {
    let normalized = name.trim().to_lowercase();
    format!("{normalized}@{marketplace}")
}

fn sanitize_plugin_id(plugin_id: &str) -> String {
    plugin_id
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | '@' | ':' => '-',
            other => other,
        })
        .collect()
}

fn describe_install_source(source: &PluginInstallSource) -> String {
    match source {
        PluginInstallSource::LocalPath { path } => path.display().to_string(),
        PluginInstallSource::GitUrl { url } => url.clone(),
        PluginInstallSource::NpmPackage { name, version } => match version {
            Some(version) => format!("{name}@{version}"),
            None => name.clone(),
        },
    }
}

fn unix_time_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("time should be after epoch").as_millis()
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<(), PluginError> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn update_settings_json(
    path: &Path,
    mut update: impl FnMut(&mut Map<String, Value>),
) -> Result<(), PluginError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut root = match fs::read_to_string(path) {
        Ok(contents) if !contents.trim().is_empty() => serde_json::from_str::<Value>(&contents)?,
        Ok(_) => Value::Object(Map::new()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Value::Object(Map::new()),
        Err(error) => return Err(PluginError::Io(error)),
    };

    let object = root.as_object_mut().ok_or_else(|| {
        PluginError::InvalidManifest(format!(
            "settings file {} must contain a JSON object",
            path.display()
        ))
    })?;
    update(object);
    fs::write(path, serde_json::to_string_pretty(&root)?)?;
    Ok(())
}

fn ensure_object<'a>(root: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    if !root.get(key).is_some_and(Value::is_object) {
        root.insert(key.to_string(), Value::Object(Map::new()));
    }
    root.get_mut(key).and_then(Value::as_object_mut).expect("object should exist")
}

/// Environment variable lock for test isolation.
/// Guards against concurrent modification of `CLAW_CONFIG_HOME`.
#[cfg(test)]
pub(crate) fn env_lock() -> &'static std::sync::Mutex<()> {
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    &ENV_LOCK
}

fn version_satisfies(installed: &str, required: &str) -> bool {
    let installed_clean = strip_prerelease(installed);
    let required_clean = strip_prerelease(required);

    let installed_parts: Vec<u32> =
        installed_clean.split('.').filter_map(|s| s.parse().ok()).collect();
    let required_parts: Vec<u32> =
        required_clean.split('.').filter_map(|s| s.parse().ok()).collect();

    for i in 0..required_parts.len().max(installed_parts.len()) {
        let installed_val = installed_parts.get(i).copied().unwrap_or(0);
        let required_val = required_parts.get(i).copied().unwrap_or(0);
        if installed_val > required_val {
            return true;
        }
        if installed_val < required_val {
            return false;
        }
    }

    let installed_has_prerelease = installed.contains('-');
    let required_has_prerelease = required.contains('-');

    if installed_has_prerelease && !required_has_prerelease {
        let installed_base_eq = installed_parts == required_parts;
        return !installed_base_eq;
    }

    true
}

fn strip_prerelease(version: &str) -> &str {
    version.split('-').next().unwrap_or(version)
}

fn hash_plugin_directory(plugin_root: &Path) -> Result<String, PluginError> {
    let mut hasher = sha2::Sha256::new();
    let mut files: Vec<PathBuf> = Vec::new();
    collect_files_recursive(plugin_root, &mut files)?;
    files.sort();

    for file_path in &files {
        let data = fs::read(file_path).map_err(PluginError::Io)?;
        use sha2::Digest;
        hasher.update(
            file_path.strip_prefix(plugin_root).unwrap_or(file_path).to_string_lossy().as_bytes(),
        );
        hasher.update(&data);
    }

    use sha2::Digest;
    let result = hasher.finalize();
    let mut hash_str = String::with_capacity(result.len() * 2);
    for byte in result {
        use std::fmt::Write;
        write!(hash_str, "{byte:02x}").expect("writing to String should never fail");
    }
    Ok(hash_str)
}

fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), PluginError> {
    let entries = fs::read_dir(dir).map_err(PluginError::Io)?;
    for entry in entries {
        let entry = entry.map_err(PluginError::Io)?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {}
