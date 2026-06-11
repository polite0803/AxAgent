// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 智能体角色定义，决定可调用工具范围
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AgentRole {
    /// 执行者 — 所有读写工具
    #[default]
    Executor,
    /// 规划者 — 只读 + 搜索工具
    Planner,
    /// 研究者 — 只读 + 网络搜索 + 知识库
    Researcher,
    /// 代码审查者 — 只读文件 + lint 工具
    CodeReviewer,
    /// 安全守卫 — 仅验证工具
    SafetyGuard,
    /// 自定义角色
    Custom {
        name: String,
        allowed_categories: Vec<String>,
    },
}

impl AgentRole {
    /// 返回角色的默认工具类别
    pub fn default_tool_categories(&self) -> Vec<&'static str> {
        match self {
            AgentRole::Executor => vec![
                "FileRead",
                "FileWrite",
                "Shell",
                "Network",
                "System",
                "Agent",
                "Vcs",
                "Automation",
                "Communication",
                "AiMedia",
                "Integration",
                "Storage",
                "Knowledge",
                "Browser",
                "Desktop",
            ],
            AgentRole::Planner => vec!["FileRead", "Network", "Knowledge"],
            AgentRole::Researcher => vec!["FileRead", "Network", "Knowledge", "Browser"],
            AgentRole::CodeReviewer => vec!["FileRead", "Vcs", "Knowledge"],
            AgentRole::SafetyGuard => vec!["FileRead", "Knowledge"],
            AgentRole::Custom {
                allowed_categories: _,
                ..
            } => {
                vec![]
            },
        }
    }

    pub fn allow_all_tools(&self) -> bool {
        matches!(self, AgentRole::Executor)
    }

    pub fn as_str(&self) -> &str {
        match self {
            AgentRole::Executor => "executor",
            AgentRole::Planner => "planner",
            AgentRole::Researcher => "researcher",
            AgentRole::CodeReviewer => "code_reviewer",
            AgentRole::SafetyGuard => "safety_guard",
            AgentRole::Custom { name, .. } => name.as_str(),
        }
    }
}

impl std::str::FromStr for AgentRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "executor" => Ok(AgentRole::Executor),
            "planner" => Ok(AgentRole::Planner),
            "researcher" => Ok(AgentRole::Researcher),
            "code_reviewer" => Ok(AgentRole::CodeReviewer),
            "safety_guard" => Ok(AgentRole::SafetyGuard),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DebugMode {
    #[default]
    Off,
    Basic,
    Verbose,
}

impl DebugMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            DebugMode::Off => "off",
            DebugMode::Basic => "basic",
            DebugMode::Verbose => "verbose",
        }
    }
}

impl std::str::FromStr for DebugMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "basic" => Ok(DebugMode::Basic),
            "verbose" => Ok(DebugMode::Verbose),
            "off" => Ok(DebugMode::Off),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub react: ReActConfig,
    pub task_decomposition: TaskDecompositionConfig,
    pub error_recovery: ErrorRecoveryConfig,
    pub reflection: ReflectionConfig,
    pub debug_mode: DebugMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReActConfig {
    pub max_iterations: usize,
    pub max_depth: usize,
    pub verification_enabled: bool,
    pub stream_thoughts: bool,
    pub timeout_secs: u64,
}

impl Default for ReActConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            max_depth: 10,
            verification_enabled: true,
            stream_thoughts: true,
            timeout_secs: 300,
        }
    }
}

impl ReActConfig {
    pub fn new(max_iterations: usize, max_depth: usize) -> Self {
        Self {
            max_iterations,
            max_depth,
            verification_enabled: true,
            stream_thoughts: true,
            timeout_secs: 300,
        }
    }

    pub fn with_verification(mut self, enabled: bool) -> Self {
        self.verification_enabled = enabled;
        self
    }

    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.stream_thoughts = enabled;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDecompositionConfig {
    pub threshold: usize,
    pub parallel_execution: bool,
    pub max_subtasks: usize,
}

impl Default for TaskDecompositionConfig {
    fn default() -> Self {
        Self {
            threshold: 3,
            parallel_execution: true,
            max_subtasks: 20,
        }
    }
}

impl TaskDecompositionConfig {
    pub fn new(threshold: usize) -> Self {
        Self {
            threshold,
            parallel_execution: true,
            max_subtasks: 20,
        }
    }

    pub fn with_parallel(mut self, enabled: bool) -> Self {
        self.parallel_execution = enabled;
        self
    }

    pub fn with_max_subtasks(mut self, max: usize) -> Self {
        self.max_subtasks = max;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorRecoveryConfig {
    pub enabled: bool,
    pub max_attempts: usize,
    pub base_delay_ms: u64,
    pub exponential_backoff: bool,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            base_delay_ms: 1000,
            exponential_backoff: true,
        }
    }
}

impl ErrorRecoveryConfig {
    pub fn new(max_attempts: usize) -> Self {
        Self {
            enabled: true,
            max_attempts,
            base_delay_ms: 1000,
            exponential_backoff: true,
        }
    }

    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.base_delay_ms = delay_ms;
        self
    }

    pub fn with_backoff(mut self, enabled: bool) -> Self {
        self.exponential_backoff = enabled;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectionConfig {
    pub enabled: bool,
    pub store_insights: bool,
    pub min_quality_threshold: u8,
}

impl Default for ReflectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            store_insights: true,
            min_quality_threshold: 5,
        }
    }
}

impl ReflectionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_insights(mut self, enabled: bool) -> Self {
        self.store_insights = enabled;
        self
    }

    pub fn with_threshold(mut self, threshold: u8) -> Self {
        self.min_quality_threshold = threshold.clamp(1, 10);
        self
    }
}

impl AgentConfig {
    pub fn new() -> Self {
        Self {
            react: ReActConfig::default(),
            task_decomposition: TaskDecompositionConfig::default(),
            error_recovery: ErrorRecoveryConfig::default(),
            reflection: ReflectionConfig::default(),
            debug_mode: DebugMode::Off,
        }
    }

    pub fn with_react(mut self, config: ReActConfig) -> Self {
        self.react = config;
        self
    }

    pub fn with_task_decomposition(mut self, config: TaskDecompositionConfig) -> Self {
        self.task_decomposition = config;
        self
    }

    pub fn with_error_recovery(mut self, config: ErrorRecoveryConfig) -> Self {
        self.error_recovery = config;
        self
    }

    pub fn with_reflection(mut self, config: ReflectionConfig) -> Self {
        self.reflection = config;
        self
    }

    pub fn with_debug_mode(mut self, mode: DebugMode) -> Self {
        self.debug_mode = mode;
        self
    }

    pub fn should_verify(&self) -> bool {
        self.react.verification_enabled
    }

    pub fn should_retry(&self) -> bool {
        self.error_recovery.enabled && self.error_recovery.max_attempts > 0
    }

    pub fn should_reflect(&self) -> bool {
        self.reflection.enabled
    }

    pub fn max_iterations(&self) -> usize {
        self.react.max_iterations
    }

    pub fn max_depth(&self) -> usize {
        self.react.max_depth
    }

    pub fn max_retry_attempts(&self) -> usize {
        self.error_recovery.max_attempts
    }

    pub fn retry_delay_ms(&self) -> u64 {
        self.error_recovery.base_delay_ms
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    pub config: AgentConfig,
    pub saved_at: chrono::DateTime<chrono::Utc>,
    pub description: Option<String>,
}

impl ConfigSnapshot {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            saved_at: chrono::Utc::now(),
            description: None,
        }
    }

    pub fn with_description(mut self, desc: String) -> Self {
        self.description = Some(desc);
        self
    }
}

pub struct ConfigManager {
    config: Arc<RwLock<AgentConfig>>,
    snapshots: Arc<RwLock<Vec<ConfigSnapshot>>>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(AgentConfig::default())),
            snapshots: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn get_config(&self) -> AgentConfig {
        self.config.read().await.clone()
    }

    pub async fn update_config(&self, new_config: AgentConfig) {
        let old_config = self.config.write().await.clone();
        *self.config.write().await = new_config;

        let snapshot = ConfigSnapshot::new(old_config)
            .with_description("Auto-saved before update".to_string());
        self.snapshots.write().await.push(snapshot);
    }

    pub async fn update_react_config(&self, react: ReActConfig) {
        self.config.write().await.react = react;
    }

    pub async fn update_task_decomposition_config(&self, td: TaskDecompositionConfig) {
        self.config.write().await.task_decomposition = td;
    }

    pub async fn update_error_recovery_config(&self, er: ErrorRecoveryConfig) {
        self.config.write().await.error_recovery = er;
    }

    pub async fn update_reflection_config(&self, refl: ReflectionConfig) {
        self.config.write().await.reflection = refl;
    }

    pub async fn update_debug_mode(&self, mode: DebugMode) {
        self.config.write().await.debug_mode = mode;
    }

    pub async fn reset_to_defaults(&self) {
        let old_config = self.config.write().await.clone();
        *self.config.write().await = AgentConfig::default();

        let snapshot =
            ConfigSnapshot::new(old_config).with_description("Auto-saved before reset".to_string());
        self.snapshots.write().await.push(snapshot);
    }

    pub async fn save_snapshot(&self, description: String) -> usize {
        let config = self.config.read().await.clone();
        let snapshot = ConfigSnapshot::new(config).with_description(description);
        let id = self.snapshots.read().await.len();
        self.snapshots.write().await.push(snapshot);
        id
    }

    pub async fn get_snapshots(&self) -> Vec<ConfigSnapshot> {
        self.snapshots.read().await.clone()
    }

    pub async fn restore_snapshot(&self, index: usize) -> bool {
        let snapshots = self.snapshots.read().await;
        if index >= snapshots.len() {
            return false;
        }

        let snapshot = snapshots[index].config.clone();
        drop(snapshots);

        let old_config = self.config.write().await.clone();
        *self.config.write().await = snapshot;

        let new_snapshot = ConfigSnapshot::new(old_config)
            .with_description(format!("Auto-saved before restore #{}", index));
        self.snapshots.write().await.push(new_snapshot);

        true
    }

    pub async fn export_config(&self) -> String {
        let config = self.config.read().await;
        serde_json::to_string_pretty(&*config).unwrap_or_default()
    }

    pub async fn import_config(&self, json: &str) -> Result<(), String> {
        let new_config: AgentConfig =
            serde_json::from_str(json).map_err(|e| format!("Failed to parse config: {}", e))?;

        self.update_config(new_config).await;
        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ConfigManager {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            snapshots: Arc::clone(&self.snapshots),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_config() {
        let config = AgentConfig::default();
        assert_eq!(config.react.max_iterations, 50);
        assert_eq!(config.react.max_depth, 10);
        assert!(config.react.verification_enabled);
        assert!(config.error_recovery.enabled);
        assert_eq!(config.error_recovery.max_attempts, 3);
    }

    #[tokio::test]
    async fn test_config_update() {
        let manager = ConfigManager::new();

        let new_config = AgentConfig::new()
            .with_react(ReActConfig::new(100, 20))
            .with_error_recovery(ErrorRecoveryConfig::new(5));

        manager.update_config(new_config.clone()).await;

        let retrieved = manager.get_config().await;
        assert_eq!(retrieved.react.max_iterations, 100);
        assert_eq!(retrieved.error_recovery.max_attempts, 5);
    }

    #[tokio::test]
    async fn test_snapshot_and_restore() {
        let manager = ConfigManager::new();

        manager.save_snapshot("Test snapshot".to_string()).await;

        let new_config = AgentConfig::new().with_react(ReActConfig::new(999, 999));
        manager.update_config(new_config).await;

        let restored = manager.restore_snapshot(0).await;
        assert!(restored);

        let config = manager.get_config().await;
        assert_eq!(config.react.max_iterations, 50);
    }

    #[tokio::test]
    async fn test_export_import() {
        let manager = ConfigManager::new();

        let exported = manager.export_config().await;
        assert!(!exported.is_empty());

        manager.import_config(&exported).await.unwrap();

        let re_exported = manager.export_config().await;
        assert_eq!(exported, re_exported);
    }
}
