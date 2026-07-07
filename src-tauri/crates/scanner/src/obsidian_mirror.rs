// SPDX-License-Identifier: AGPL-3.0-only

//! Obsidian 回忆镜像 — 将 AxAgent 记忆条目写入 Obsidian vault 的 markdown 文件。

use std::path::PathBuf;

pub struct ObsidianMirror {
    vault_path: PathBuf,
    subdir: String,
}

impl ObsidianMirror {
    pub fn new(vault_path: impl Into<PathBuf>, subdir: Option<&str>) -> Self {
        Self { vault_path: vault_path.into(), subdir: subdir.unwrap_or("AxAgent").to_string() }
    }

    pub fn sync_entry(&self, id: &str, memory_type: &str, content: &str, tags: &[String], importance: f64, tier: &str, created_at: i64, updated_at: i64) -> Result<Option<PathBuf>, String> {
        let slug = sanitize_filename(id);
        let date = ts_to_date(created_at);
        let rel_path = format!("{}/{}-{}.md", self.subdir, date, slug);
        let full_path = self.vault_path.join(&rel_path);

        let md = build_markdown(content, memory_type, tags, importance, tier, created_at, updated_at);

        if full_path.exists() {
            if let Ok(existing) = std::fs::read_to_string(&full_path) {
                if existing == md { return Ok(None); }
            }
        }
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create_dir_all {}: {}", parent.display(), e))?;
        }
        std::fs::write(&full_path, &md).map_err(|e| format!("write {}: {}", full_path.display(), e))?;
        Ok(Some(full_path))
    }
}

fn build_markdown(content: &str, memory_type: &str, tags: &[String], importance: f64, tier: &str, created_at: i64, updated_at: i64) -> String {
    let tags_yaml = if tags.is_empty() { String::new() } else { format!("\n  - {}", tags.join("\n  - ")) };
    format!("---\ntype: memory\nmemory_type: {memory_type}\nimportance: {importance}\ntier: {tier}\ncreated_at: {}\nupdated_at: {}\ntags:{tags_yaml}\n---\n\n{content}\n", ts_to_iso(created_at), ts_to_iso(updated_at))
}

fn sanitize_filename(id: &str) -> String {
    id.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect::<String>().trim_matches('_').to_string()
}

fn ts_to_date(ts_ms: i64) -> String {
    match chrono::DateTime::from_timestamp(ts_ms / 1000, 0) {
        Some(dt) => dt.format("%Y-%m-%d").to_string(),
        None => "unknown".to_string(),
    }
}

fn ts_to_iso(ts_ms: i64) -> String {
    if ts_ms <= 0 { return "unknown".to_string(); }
    match chrono::DateTime::from_timestamp(ts_ms / 1000, 0) {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        None => "unknown".to_string(),
    }
}
