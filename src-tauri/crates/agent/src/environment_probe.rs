// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub working_directory: PathBuf,
    pub directory_listing: Vec<FileInfo>,
    pub environment_variables: HashMap<String, String>,
    pub running_processes: Vec<ProcessInfo>,
    pub disk_usage: Option<DiskUsage>,
    pub network_status: Option<NetworkStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub size_bytes: Option<u64>,
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub extension: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub command: Option<String>,
    pub cpu_percent: Option<f64>,
    pub memory_mb: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub is_connected: bool,
    pub interfaces: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeConfig {
    pub scan_directory_depth: usize,
    pub max_files_per_directory: usize,
    pub include_hidden_files: bool,
    pub include_environment_vars: bool,
    pub include_processes: bool,
    pub include_disk_usage: bool,
    pub include_network: bool,
    pub env_var_whitelist: Vec<String>,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            scan_directory_depth: 2,
            max_files_per_directory: 100,
            include_hidden_files: false,
            include_environment_vars: true,
            include_processes: false,
            include_disk_usage: true,
            include_network: true,
            env_var_whitelist: vec![
                "PATH".to_string(),
                "HOME".to_string(),
                "USER".to_string(),
                "SHELL".to_string(),
                "LANG".to_string(),
                "PWD".to_string(),
                "EDITOR".to_string(),
                "TERM".to_string(),
            ],
        }
    }
}

pub struct EnvironmentProbe {
    config: ProbeConfig,
}

impl EnvironmentProbe {
    pub fn new(config: ProbeConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(ProbeConfig::default())
    }

    pub fn scan_directory(&self, path: &std::path::Path) -> Vec<FileInfo> {
        let mut files = Vec::new();
        self.scan_directory_recursive(path, 0, &mut files);
        files
    }

    fn scan_directory_recursive(
        &self,
        path: &std::path::Path,
        depth: usize,
        files: &mut Vec<FileInfo>,
    ) {
        if depth > self.config.scan_directory_depth {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(path) {
            for (count, entry) in entries.enumerate() {
                if count >= self.config.max_files_per_directory {
                    break;
                }

                if let Ok(entry) = entry {
                    let file_name = entry.file_name().to_string_lossy().to_string();

                    if !self.config.include_hidden_files && file_name.starts_with('.') {
                        continue;
                    }

                    let metadata = entry.metadata().ok();
                    let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    let size = metadata.as_ref().map(|m| m.len());
                    let modified = metadata
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0));

                    files.push(FileInfo {
                        name: file_name,
                        path: entry.path(),
                        is_directory: is_dir,
                        size_bytes: size,
                        modified_at: modified,
                        extension: entry
                            .path()
                            .extension()
                            .map(|e| e.to_string_lossy().to_string()),
                    });

                    if is_dir && depth < self.config.scan_directory_depth {
                        self.scan_directory_recursive(&entry.path(), depth + 1, files);
                    }
                }
            }
        }
    }

    pub fn get_environment_variables(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        for key in &self.config.env_var_whitelist {
            if let Ok(value) = std::env::var(key) {
                vars.insert(key.clone(), value);
            }
        }
        vars
    }

    pub fn get_disk_usage(&self, path: &std::path::Path) -> Option<DiskUsage> {
        let _metadata = std::fs::metadata(path).ok()?;
        Some(DiskUsage {
            total_bytes: 0,
            used_bytes: 0,
            available_bytes: 0,
        })
    }

    pub fn probe(&self, working_directory: &std::path::Path) -> EnvironmentSnapshot {
        EnvironmentSnapshot {
            timestamp: chrono::Utc::now(),
            working_directory: working_directory.to_path_buf(),
            directory_listing: self.scan_directory(working_directory),
            environment_variables: if self.config.include_environment_vars {
                self.get_environment_variables()
            } else {
                HashMap::new()
            },
            running_processes: Vec::new(),
            disk_usage: if self.config.include_disk_usage {
                self.get_disk_usage(working_directory)
            } else {
                None
            },
            network_status: if self.config.include_network {
                Some(NetworkStatus {
                    is_connected: true,
                    interfaces: Vec::new(),
                })
            } else {
                None
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_current_directory() {
        let probe = EnvironmentProbe::with_default_config();
        let _ = probe.scan_directory(std::path::Path::new("."));
    }

    #[test]
    fn test_get_environment_variables() {
        let probe = EnvironmentProbe::with_default_config();
        let _ = probe.get_environment_variables();
    }

    #[test]
    fn test_probe_creates_snapshot() {
        let probe = EnvironmentProbe::with_default_config();
        let snapshot = probe.probe(std::path::Path::new("."));
        assert!(!snapshot.working_directory.as_os_str().is_empty());
    }

    #[test]
    fn test_config_customization() {
        let config = ProbeConfig {
            scan_directory_depth: 1,
            max_files_per_directory: 10,
            include_hidden_files: true,
            ..Default::default()
        };
        let probe = EnvironmentProbe::new(config);
        assert_eq!(probe.config.scan_directory_depth, 1);
        assert!(probe.config.include_hidden_files);
    }
}
