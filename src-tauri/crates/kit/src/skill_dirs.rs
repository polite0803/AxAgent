// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;
use std::sync::LazyLock;

const SKILL_DIR_PRIORITY: &[&str] = &[
    "axagent",
    "claude",
    "trae",
    "codebuddy",
    "workbuddy",
    "agents",
];

static EXTERNAL_DIRS: LazyLock<Vec<PathBuf>> = LazyLock::new(load_external_dirs_from_config);

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
    dirs_arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(expand_path)
        .filter(|p| p.is_dir())
        .collect()
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
        shellexpand::env(&tilde_expanded)
            .map(|s| s.to_string())
            .unwrap_or(tilde_expanded)
    } else {
        tilde_expanded
    };

    PathBuf::from(env_expanded)
}

static SKILL_DIRS: LazyLock<Vec<(String, PathBuf)>> = LazyLock::new(|| {
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

    for ext_dir in EXTERNAL_DIRS.iter() {
        let label = ext_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "external".to_string());
        dirs.push((label, ext_dir.clone()));
    }

    dirs
});

pub fn skill_dirs() -> Vec<(&'static str, PathBuf)> {
    SKILL_DIRS
        .iter()
        .map(|(label, dir)| (label.as_str(), dir.clone()))
        .collect()
}

pub fn all_skills_dirs() -> Vec<PathBuf> {
    SKILL_DIRS.iter().map(|(_, dir)| dir.clone()).collect()
}

pub fn external_skill_dirs() -> Vec<PathBuf> {
    EXTERNAL_DIRS.clone()
}
