// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use std::fs;

pub fn detect_platforms() -> Vec<DetectedPlatform> {
    let mut platforms = Vec::new();

    let oc = openclaw_home();
    if oc.exists() && oc.is_dir() {
        let skill_dir = oc.join("skills");
        let skill_count = if skill_dir.exists() {
            fs::read_dir(&skill_dir)
                .map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).count())
                .unwrap_or(0)
        } else {
            0
        };
        platforms.push(DetectedPlatform {
            name: "OpenClaw".to_string(),
            base_path: oc.clone(),
            has_soul: oc.join("SOUL.md").exists(),
            has_memory: oc.join("MEMORY.md").exists(),
            has_skills: skill_dir.exists() && skill_count > 0,
            has_config: oc.join("config.yaml").exists() || oc.join("config.yml").exists(),
            has_env: oc.join(".env").exists(),
            has_cron: false,
            has_personalities: false,
            skill_count,
            memory_count: 0,
        });
    }

    let hm = hermes_home();
    if hm.exists() && hm.is_dir() {
        let skill_dir = hm.join("skills");
        let skill_count = if skill_dir.exists() {
            fs::read_dir(&skill_dir)
                .map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).count())
                .unwrap_or(0)
        } else {
            0
        };
        let mem_dir = hm.join("memories");
        let memory_count = if mem_dir.exists() {
            fs::read_dir(&mem_dir)
                .map(|d| {
                    d.filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                        .count()
                })
                .unwrap_or(0)
        } else {
            0
        };
        let personalities_dir = hm.join("personalities");
        platforms.push(DetectedPlatform {
            name: "Hermes".to_string(),
            base_path: hm.clone(),
            has_soul: false,
            has_memory: mem_dir.exists() && memory_count > 0,
            has_skills: skill_dir.exists() && skill_count > 0,
            has_config: hm.join("config.yaml").exists(),
            has_env: false,
            has_cron: hm.join("cron-tasks.json").exists(),
            has_personalities: personalities_dir.exists(),
            skill_count,
            memory_count,
        });
    }

    platforms
}
