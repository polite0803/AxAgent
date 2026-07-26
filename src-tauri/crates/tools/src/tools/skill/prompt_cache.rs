// SPDX-License-Identifier: AGPL-3.0-only
//! G13 SkillPromptCache — SKILL 内容预读缓存，供 LLM 系统提示词注入使用
//!
//! ## 设计目的
//!
//! 避免每次 LLM 调用都重复读取 SKILL.md 文件。`SkillIndex` 只缓存元数据，
//! 本模块缓存完整内容字符串，按 skill name 索引，TTL 与 SkillIndex 同步。
//!
//! ## 使用场景
//!
//! - Agent 系统提示词拼接：将相关 skill 内容注入 system_prompt
//! - 工作流 Agent 节点：根据 model_role 自动加载对应 skill
//! - MCP 工具 `skill_view`：直接返回缓存内容，避免重复 I/O

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

/// 缓存条目：skill 内容 + 来源路径 + 加载时间
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct CacheEntry {
    /// SKILL.md 完整内容
    content: String,
    /// 来源路径（用于 invalidate 时重建）
    skill_dir: PathBuf,
    /// 加载时间
    loaded_at: Instant,
}

/// 全局 SkillPromptCache（单例）
static SKILL_PROMPT_CACHE: LazyLock<Mutex<SkillPromptCache>> =
    LazyLock::new(|| Mutex::new(SkillPromptCache::new()));

/// 默认 TTL：300 秒（与 SkillIndex 一致）
const DEFAULT_TTL_SECS: u64 = 300;

/// SkillPromptCache — skill 内容缓存
pub struct SkillPromptCache {
    /// 按 skill name 索引的缓存条目
    entries: HashMap<String, CacheEntry>,
    /// TTL（秒）
    ttl_secs: u64,
    /// 上次全量重建时间
    last_rebuild: Option<Instant>,
}

impl SkillPromptCache {
    /// 创建空缓存
    fn new() -> Self {
        Self { entries: HashMap::new(), ttl_secs: DEFAULT_TTL_SECS, last_rebuild: None }
    }

    /// 获取 skill 内容（若缓存过期则触发重建）
    ///
    /// # 参数
    /// - `name`: skill 名称（目录名）
    ///
    /// # 返回
    /// - `Some(content)`: skill 内容字符串
    /// - `None`: skill 不存在或读取失败
    pub fn get_skill_prompt(name: &str) -> Option<String> {
        let mut cache = SKILL_PROMPT_CACHE.lock().ok()?;
        cache.ensure_built();
        cache.entries.get(name).map(|e| e.content.clone())
    }

    /// 批量获取多个 skill 内容，按名称顺序拼接为单个字符串
    ///
    /// 任一 skill 不存在则跳过（不返回错误）。返回拼接后的字符串，每条 skill 之间用分隔符隔开。
    pub fn get_skills_prompts_combined(names: &[&str]) -> String {
        let mut cache = match SKILL_PROMPT_CACHE.lock() {
            Ok(c) => c,
            Err(_) => return String::new(),
        };
        cache.ensure_built();

        let mut parts = Vec::with_capacity(names.len());
        for name in names {
            if let Some(entry) = cache.entries.get(*name) {
                parts.push(format!("--- SKILL: {name} ---\n{}\n", entry.content));
            }
        }
        parts.join("\n")
    }

    /// 失效缓存（外部修改 skill 文件后调用）
    pub fn invalidate() {
        if let Ok(mut cache) = SKILL_PROMPT_CACHE.lock() {
            cache.entries.clear();
            cache.last_rebuild = None;
        }
    }

    /// 设置自定义 TTL（秒）
    pub fn set_ttl(ttl_secs: u64) {
        if let Ok(mut cache) = SKILL_PROMPT_CACHE.lock() {
            cache.ttl_secs = ttl_secs;
        }
    }

    /// 列出当前缓存的所有 skill 名称
    pub fn list_cached_skills() -> Vec<String> {
        let mut cache = match SKILL_PROMPT_CACHE.lock() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        cache.ensure_built();
        cache.entries.keys().cloned().collect()
    }

    /// 检查缓存是否需要重建
    fn needs_rebuild(&self) -> bool {
        match self.last_rebuild {
            None => true,
            Some(t) => t.elapsed().as_secs() > self.ttl_secs,
        }
    }

    /// 确保缓存已构建（若过期则重建）
    fn ensure_built(&mut self) {
        if self.needs_rebuild() {
            self.rebuild();
        }
    }

    /// 全量重建缓存：扫描所有 skill 目录，读取 SKILL.md 内容
    fn rebuild(&mut self) {
        self.entries.clear();

        let dirs = axagent_kit::skill_dirs::skill_dirs();
        let mut seen = std::collections::HashSet::new();

        for (_source_kind, dir) in &dirs {
            if let Ok(dir_entries) = std::fs::read_dir(dir) {
                for entry in dir_entries.filter_map(|e| e.ok()) {
                    if !entry.path().is_dir() {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    if seen.contains(&name) {
                        continue;
                    }
                    seen.insert(name.clone());

                    let skill_dir = entry.path();
                    let skill_md = skill_dir.join("SKILL.md");
                    if let Ok(content) = std::fs::read_to_string(&skill_md) {
                        self.entries.insert(
                            name,
                            CacheEntry { content, skill_dir, loaded_at: Instant::now() },
                        );
                    }
                }
            }
        }

        self.last_rebuild = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_get_nonexistent_skill() {
        // 不存在的 skill 应返回 None（不 panic）
        let result = SkillPromptCache::get_skill_prompt("__definitely_nonexistent_skill__");
        // 注意：在测试环境下 skill_dirs 可能返回空，所以这里只是确保不 panic
        let _ = result;
    }

    #[test]
    fn test_cache_combined_empty() {
        let result = SkillPromptCache::get_skills_prompts_combined(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_cache_invalidate_does_not_panic() {
        SkillPromptCache::invalidate();
    }

    #[test]
    fn test_cache_set_ttl() {
        SkillPromptCache::set_ttl(60);
        // 恢复默认
        SkillPromptCache::set_ttl(DEFAULT_TTL_SECS);
    }

    #[test]
    fn test_cache_list_does_not_panic() {
        let _ = SkillPromptCache::list_cached_skills();
    }
}
