use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellHookConfig {
    pub event: String,
    pub command: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShellHooksConfig {
    pub hooks: Vec<ShellHookConfig>,
}

impl ShellHooksConfig {
    pub fn load_from_dir(dir: &Path) -> Self {
        let mut hooks = Vec::new();
        if !dir.exists() {
            return Self { hooks };
        }
        let config_path = dir.join("hooks.json");
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<Self>(&content) {
                    return config;
                }
            }
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let event = if name.starts_with("pre_tool_call") {
                    Some("pre_tool_call")
                } else if name.starts_with("post_tool_call") {
                    Some("post_tool_call")
                } else if name.starts_with("pre_llm_call") {
                    Some("pre_llm_call")
                } else if name.starts_with("post_llm_call") {
                    Some("post_llm_call")
                } else {
                    None
                };
                if let Some(event) = event {
                    let command = path.to_string_lossy().to_string();
                    hooks.push(ShellHookConfig {
                        event: event.to_string(),
                        command,
                        enabled: true,
                    });
                }
            }
        }
        Self { hooks }
    }

    pub fn default_hooks_dir() -> PathBuf {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".axagent")
            .join("hooks")
    }

    pub fn enabled_hooks_for(&self, event: &str) -> Vec<&ShellHookConfig> {
        self.hooks
            .iter()
            .filter(|h| h.enabled && h.event == event)
            .collect()
    }
}
