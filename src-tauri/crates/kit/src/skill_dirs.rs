// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;
use std::sync::RwLock;

const SKILL_DIR_PRIORITY: &[&str] =
    &["axagent", "claude", "trae", "codebuddy", "workbuddy", "agents"];

fn load_external_dirs_from_config() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let config_path = home.join(".axagent").join("config.yaml");
    if !config_path.exists() {
        return Vec::new();
    }
    let Ok(content) = std::fs::read_to_string(&config_path) else {
        return Vec::new();
    };
    let Ok(doc) = serde_yaml::from_str::<serde_json::Value>(&content) else {
        return Vec::new();
    };
    let Some(dirs_arr) = doc["skills"]["external_dirs"].as_array() else {
        return Vec::new();
    };
    dirs_arr.iter().filter_map(|v| v.as_str()).map(expand_path).filter(|p| p.is_dir()).collect()
}

fn expand_path(input: &str) -> PathBuf {
    let tilde_expanded = if let Some(rest) = input.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            format!("{}/{}", home.to_string_lossy(), rest)
        } else {
            input.to_string()
        }
    } else {
        input.to_string()
    };

    let env_expanded = if tilde_expanded.contains('$') {
        shellexpand::env(&tilde_expanded).map(|s| s.to_string()).unwrap_or(tilde_expanded)
    } else {
        tilde_expanded
    };

    PathBuf::from(env_expanded)
}

fn compute_skill_dirs(external_dirs: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut dirs: Vec<(String, PathBuf)> = SKILL_DIR_PRIORITY
        .iter()
        .map(|name| {
            let dir = if *name == "axagent" {
                home.join(".axagent").join("skills")
            } else {
                home.join(format!(".{}", name)).join("skills")
            };
            (name.to_string(), dir)
        })
        .collect();

    for ext_dir in external_dirs {
        let label = ext_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "external".to_string());
        dirs.push((label, ext_dir.clone()));
    }

    dirs
}

/// RwLock-backed skill directory registry supporting hot reload.
/// Replaces the previous `LazyLock` static with a mutable store so that
/// external directories can be re-scanned without restarting.
static SKILL_DIRS: RwLock<Option<Vec<(String, PathBuf)>>> = RwLock::new(None);
static EXTERNAL_DIRS: RwLock<Option<Vec<PathBuf>>> = RwLock::new(None);

fn init_if_needed() {
    {
        let dirs = SKILL_DIRS.read().unwrap();
        if dirs.is_some() {
            return;
        }
    }
    let mut dirs = SKILL_DIRS.write().unwrap();
    if dirs.is_some() {
        return;
    }
    let ext = load_external_dirs_from_config();
    *EXTERNAL_DIRS.write().unwrap() = Some(ext.clone());
    *dirs = Some(compute_skill_dirs(&ext));
}

/// Reload skill directories from config. Useful when new skill sources are
/// added at runtime (e.g., marketplace installs a skill into a new directory)
/// without restarting the application.
///
/// Returns the new set of (label, path) pairs.
pub fn reload_skill_dirs() -> Vec<(String, PathBuf)> {
    let ext = load_external_dirs_from_config();
    let computed = compute_skill_dirs(&ext);
    *EXTERNAL_DIRS.write().unwrap() = Some(ext);
    *SKILL_DIRS.write().unwrap() = Some(computed.clone());
    computed
}

pub fn skill_dirs() -> Vec<(String, PathBuf)> {
    init_if_needed();
    SKILL_DIRS
        .read()
        .unwrap()
        .as_ref()
        .map(|d| d.iter().map(|(label, dir)| (label.clone(), dir.clone())).collect())
        .unwrap_or_default()
}

pub fn all_skills_dirs() -> Vec<PathBuf> {
    init_if_needed();
    SKILL_DIRS
        .read()
        .unwrap()
        .as_ref()
        .map(|d| d.iter().map(|(_, dir)| dir.clone()).collect())
        .unwrap_or_default()
}

pub fn external_skill_dirs() -> Vec<PathBuf> {
    init_if_needed();
    EXTERNAL_DIRS.read().unwrap().clone().unwrap_or_default()
}
