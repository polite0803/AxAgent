// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use std::fs;

pub fn preview_openclaw() -> Vec<MigrationItem> {
    let oc = openclaw_home();
    let home = axagent_home();
    let mut items = Vec::new();

    if oc.join("SOUL.md").exists() {
        items.push(make_item(
            oc.join("SOUL.md"),
            home.join("personalities").join("openclaw-import").join("SOUL.md"),
            "personality",
            "SOUL.md → personalities/openclaw-import/SOUL.md".to_string(),
        ));
    }

    if oc.join("MEMORY.md").exists() {
        items.push(make_item(
            oc.join("MEMORY.md"),
            home.join("memories").join("openclaw-import.md"),
            "memory",
            "MEMORY.md → memories/openclaw-import.md".to_string(),
        ));
    }

    let skill_dir = oc.join("skills");
    if skill_dir.exists()
        && let Ok(entries) = fs::read_dir(&skill_dir)
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                items.push(make_item(
                    path,
                    home.join("skills").join("openclaw-imports").join(&name),
                    "skill",
                    format!("skills/{} → skills/openclaw-imports/{}", name, name),
                ));
            }
        }
    }

    let allowlist = oc.join("allowed-commands.json");
    if allowlist.exists() {
        items.push(make_item(
            allowlist,
            home.join("allowed-commands.json"),
            "allowlist",
            "allowed-commands.json → allowed-commands.json".to_string(),
        ));
    }

    let env_file = oc.join(".env");
    if env_file.exists() {
        items.push(make_item(
            env_file,
            home.join(".env"),
            "env",
            ".env → .env (API keys, merged)".to_string(),
        ));
    }

    items
}

pub fn preview_hermes() -> Vec<MigrationItem> {
    let hm = hermes_home();
    let home = axagent_home();
    let mut items = Vec::new();

    let skill_dir = hm.join("skills");
    if skill_dir.exists()
        && let Ok(entries) = fs::read_dir(&skill_dir)
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                items.push(make_item(
                    path,
                    home.join("skills").join("hermes-imports").join(&name),
                    "skill",
                    format!("skills/{} → skills/hermes-imports/{}", name, name),
                ));
            }
        }
    }

    let mem_dir = hm.join("memories");
    if mem_dir.exists()
        && let Ok(entries) = fs::read_dir(&mem_dir)
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                let name = entry.file_name().to_string_lossy().to_string();
                items.push(make_item(
                    path,
                    home.join("memories").join(&name),
                    "memory",
                    format!("memories/{} → memories/{}", name, name),
                ));
            }
        }
    }

    let config = hm.join("config.yaml");
    if config.exists() {
        items.push(make_item(
            config,
            home.join("config.yaml"),
            "config",
            "config.yaml → config.yaml (merged)".to_string(),
        ));
    }

    let cron = hm.join("cron-tasks.json");
    if cron.exists() {
        items.push(make_item(
            cron,
            home.join("cron-tasks.json"),
            "cron",
            "cron-tasks.json → cron-tasks.json".to_string(),
        ));
    }

    let personalities_dir = hm.join("personalities");
    if personalities_dir.exists()
        && let Ok(entries) = fs::read_dir(&personalities_dir)
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                items.push(make_item(
                    path,
                    home.join("personalities").join(&name),
                    "personality",
                    format!("personalities/{} → personalities/{}", name, name),
                ));
            }
        }
    }

    items
}
