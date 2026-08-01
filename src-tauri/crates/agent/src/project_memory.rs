// SPDX-License-Identifier: AGPL-3.0-only

//! 3.2 P2:长期记忆文件级四类分目录
//!
//! 对齐 Claude Code MEMORY.md 模型。在 `.axagent/memory/` 下按四类分目录:
//! - `user/`       — 用户偏好/信息(技术栈、沟通风格、工作习惯)
//! - `feedback/`   — 用户反馈(显式喜好/排斥、纠正记录)
//! - `project/`    — 项目相关(架构决策、约定、命令)
//! - `reference/`  — 参考资料(外部链接、文档索引)
//!
//! `.axagent/MEMORY.md` 是索引文件(200 行硬限制),始终加载,
//! 索引四类主题文件的相对路径与一句话摘要。
//!
//! ## 检索策略
//! 启动时加载 MEMORY.md 索引(始终加载);文件级检索时扫描 `.axagent/memory/`
//! 目录下所有 .md 文件,按关键词匹配度选最相关 N 个,与现有 DB 检索并存。

use axagent_harness::constants::memory as mem_const;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 记忆四类分目录枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryCategory {
    /// 用户偏好/信息(技术栈、沟通风格、工作习惯)
    User,
    /// 用户反馈(显式喜好/排斥、纠正记录)
    Feedback,
    /// 项目相关(架构决策、约定、命令)
    Project,
    /// 参考资料(外部链接、文档索引)
    Reference,
}

impl MemoryCategory {
    /// 目录名
    pub fn dir_name(self) -> &'static str {
        match self {
            Self::User => mem_const::USER_DIR,
            Self::Feedback => mem_const::FEEDBACK_DIR,
            Self::Project => mem_const::PROJECT_DIR,
            Self::Reference => mem_const::REFERENCE_DIR,
        }
    }

    /// 从目录名解析
    pub fn from_dir_name(name: &str) -> Option<Self> {
        match name {
            mem_const::USER_DIR => Some(Self::User),
            mem_const::FEEDBACK_DIR => Some(Self::Feedback),
            mem_const::PROJECT_DIR => Some(Self::Project),
            mem_const::REFERENCE_DIR => Some(Self::Reference),
            _ => None,
        }
    }

    /// 全部四类
    pub const ALL: [Self; 4] = [Self::User, Self::Feedback, Self::Project, Self::Reference];
}

/// 单个主题文件的索引条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFileEntry {
    /// 相对于 `.axagent/memory/` 的路径(如 `user/preferences.md`)
    pub relative_path: String,
    /// 类别
    pub category: MemoryCategory,
    /// 一句话摘要(单行)
    pub summary: String,
    /// 标签(用于检索)
    pub tags: Vec<String>,
}

/// MEMORY.md 索引文件内容
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryIndex {
    pub entries: Vec<MemoryFileEntry>,
}

impl MemoryIndex {
    /// 解析 MEMORY.md 索引文件
    ///
    /// 格式:
    /// ```markdown
    /// # Memory Index
    ///
    /// - [user/preferences.md] 用户偏好:Rust + Tauri 技术栈 #rust #tauri
    /// - [project/architecture.md] 项目架构:Cargo workspace 分层 #architecture
    /// ```
    pub fn parse(content: &str) -> Self {
        let mut entries = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            let Some(after_dash) = trimmed.strip_prefix("- [") else {
                continue;
            };
            let Some(close_idx) = after_dash.find(']') else {
                continue;
            };
            let relative_path = after_dash[..close_idx].trim().to_string();
            if relative_path.is_empty() {
                continue;
            }
            let rest = after_dash[close_idx + 1..].trim();
            // 提取 tags(以 # 开头的单词)
            let mut summary_parts = Vec::new();
            let mut tags = Vec::new();
            for word in rest.split_whitespace() {
                if let Some(tag) = word.strip_prefix('#') {
                    if !tag.is_empty() {
                        tags.push(tag.to_string());
                    }
                } else {
                    summary_parts.push(word);
                }
            }
            let summary = summary_parts.join(" ");
            // 从路径推断类别
            let category = relative_path
                .split('/')
                .next()
                .and_then(MemoryCategory::from_dir_name)
                .unwrap_or(MemoryCategory::Reference);
            entries.push(MemoryFileEntry { relative_path, category, summary, tags });
        }
        Self { entries }
    }

    /// 渲染为 MEMORY.md 内容(强制 200 行硬限制)
    pub fn render(&self) -> String {
        let mut md = String::from("# Memory Index\n\n");
        for entry in &self.entries {
            let tags_str = if entry.tags.is_empty() {
                String::new()
            } else {
                format!(
                    " {}",
                    entry.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" ")
                )
            };
            md.push_str(&format!("- [{}] {}{}\n", entry.relative_path, entry.summary, tags_str));
        }
        // 200 行硬限制:超出则截断
        let lines: Vec<&str> = md.lines().collect();
        if lines.len() > mem_const::MEMORY_INDEX_MAX_LINES {
            let truncated: Vec<&str> =
                lines.into_iter().take(mem_const::MEMORY_INDEX_MAX_LINES).collect();
            truncated.join("\n") + "\n"
        } else {
            md
        }
    }

    /// 添加条目(若同路径已存在则覆盖)
    pub fn upsert(&mut self, entry: MemoryFileEntry) {
        if let Some(existing) =
            self.entries.iter_mut().find(|e| e.relative_path == entry.relative_path)
        {
            *existing = entry;
        } else {
            self.entries.push(entry);
        }
    }

    /// 按相对路径删除条目
    pub fn remove(&mut self, relative_path: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.relative_path != relative_path);
        self.entries.len() < before
    }
}

/// 文件级检索结果项
#[derive(Debug, Clone)]
pub struct MemorySearchResult {
    /// 文件绝对路径
    pub path: PathBuf,
    /// 类别
    pub category: MemoryCategory,
    /// 文件内容
    pub content: String,
    /// 关键词匹配得分(越高越相关)
    pub score: f64,
}

/// 长期记忆文件级存储(本地文件系统实现)
///
/// 管理 `.axagent/memory/{user,feedback,project,reference}/` 四类分目录,
/// 以及 `.axagent/MEMORY.md` 索引文件(200 行硬限制,始终加载)。
pub struct ProjectMemory {
    /// 项目根目录
    pub project_root: PathBuf,
}

impl ProjectMemory {
    /// 创建新的记忆存储实例
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self { project_root: project_root.into() }
    }

    /// 索引文件绝对路径(`.axagent/MEMORY.md`)
    pub fn index_path(&self) -> PathBuf {
        self.project_root.join(mem_const::MEMORY_INDEX)
    }

    /// 记忆根目录绝对路径(`.axagent/memory`)
    pub fn memory_dir(&self) -> PathBuf {
        self.project_root.join(mem_const::MEMORY_DIR)
    }

    /// 类别子目录绝对路径
    pub fn category_dir(&self, category: MemoryCategory) -> PathBuf {
        self.memory_dir().join(category.dir_name())
    }

    /// 主题文件绝对路径
    pub fn topic_file_path(&self, category: MemoryCategory, file_name: &str) -> PathBuf {
        self.category_dir(category).join(file_name)
    }

    /// 加载 MEMORY.md 索引文件(始终加载,不存在则返回空索引)
    pub async fn load_index(&self) -> MemoryIndex {
        let path = self.index_path();
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => MemoryIndex::parse(&content),
            Err(_) => MemoryIndex::default(),
        }
    }

    /// 保存索引文件(强制 200 行硬限制)
    pub async fn save_index(&self, index: &MemoryIndex) -> Result<(), String> {
        let path = self.index_path();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
        }
        let content = index.render();
        tokio::fs::write(&path, content).await.map_err(|e| e.to_string())
    }

    /// 加载某个类别的所有文件内容(返回 (绝对路径, 内容) 列表)
    pub async fn load_category(
        &self,
        category: MemoryCategory,
    ) -> Result<Vec<(PathBuf, String)>, String> {
        let dir = self.category_dir(category);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut results = Vec::new();
        let mut reader = tokio::fs::read_dir(&dir).await.map_err(|e| e.to_string())?;
        while let Some(entry) = reader.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let content = tokio::fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
            // 单文件大小限制检查
            if content.len() > mem_const::MEMORY_FILE_SIZE_LIMIT {
                tracing::warn!(
                    path = %path.display(),
                    size = content.len(),
                    limit = mem_const::MEMORY_FILE_SIZE_LIMIT,
                    "[ProjectMemory] 主题文件超过 256KB 大小限制,跳过加载"
                );
                continue;
            }
            results.push((path, content));
        }
        Ok(results)
    }

    /// 写入一个主题文件(不自动更新索引,需另行调用 save_index)
    pub async fn save_topic_file(
        &self,
        category: MemoryCategory,
        file_name: &str,
        content: &str,
    ) -> Result<PathBuf, String> {
        // 路径验证:file_name 不能包含 .. 或绝对路径
        if file_name.is_empty()
            || file_name.contains('/')
            || file_name.contains('\\')
            || file_name.contains("..")
            || file_name.starts_with('.')
        {
            return Err(format!("invalid topic file name: {}", file_name));
        }
        if !file_name.ends_with(".md") {
            return Err(format!("topic file must end with .md: {}", file_name));
        }
        // 大小限制检查
        if content.len() > mem_const::MEMORY_FILE_SIZE_LIMIT {
            return Err(format!(
                "topic file content exceeds {}KB size limit ({} bytes)",
                mem_const::MEMORY_FILE_SIZE_LIMIT / 1024,
                content.len()
            ));
        }
        let path = self.topic_file_path(category, file_name);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
        }
        tokio::fs::write(&path, content).await.map_err(|e| e.to_string())?;
        Ok(path)
    }

    /// 删除主题文件(同步删除索引条目需另行调用 save_index)
    pub async fn delete_topic_file(
        &self,
        category: MemoryCategory,
        file_name: &str,
    ) -> Result<bool, String> {
        let path = self.topic_file_path(category, file_name);
        if !path.exists() {
            return Ok(false);
        }
        tokio::fs::remove_file(&path).await.map_err(|e| e.to_string())?;
        Ok(true)
    }

    /// 扫描四类分目录,按关键词选最相关 N 个文件
    ///
    /// 检索策略:简单 TF 关键词匹配
    /// - query 按空格/标点分词
    /// - 文件内容中每个 query 词出现次数累加
    /// - 归一化 score = total_hits / (content_len + 1)
    /// - 按 score 降序排序,取前 N 个
    pub async fn scan_relevant_files(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemorySearchResult>, String> {
        let limit = if limit == 0 {
            mem_const::MEMORY_RELEVANT_FILES_LIMIT
        } else {
            limit
        };
        let keywords = tokenize_query(query);
        let mut results: Vec<MemorySearchResult> = Vec::new();
        for category in MemoryCategory::ALL {
            let files = self.load_category(category).await?;
            for (path, content) in files {
                let score = compute_relevance_score(&content, &keywords);
                results.push(MemorySearchResult { path, category, content, score });
            }
        }
        // 按得分降序排序,取前 N 个
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        Ok(results)
    }

    /// 从旧的单文件 `.axagent/memory.md` 迁移到四类分目录
    ///
    /// 如果存在旧的 `.axagent/memory.md` 但不存在新结构,则将其内容
    /// 迁移到 `.axagent/memory/project/legacy.md` 并创建索引。
    pub async fn migrate_from_legacy(&self) -> Result<bool, String> {
        let legacy_path = self.project_root.join(".axagent/memory.md");
        // 已迁移判定：新结构目录 `.axagent/memory/` 存在即视为已迁移。
        // 不用索引文件 `MEMORY.md` 判断——Windows 文件系统大小写不敏感，
        // legacy `memory.md` 与索引 `MEMORY.md` 指向同一文件，会误判已迁移。
        if self.memory_dir().exists() || !legacy_path.exists() {
            return Ok(false);
        }
        let content = tokio::fs::read_to_string(&legacy_path).await.map_err(|e| e.to_string())?;
        // 迁移到 project/legacy.md
        self.save_topic_file(MemoryCategory::Project, "legacy.md", &content).await?;
        // 创建初始索引
        let mut index = MemoryIndex::default();
        index.upsert(MemoryFileEntry {
            relative_path: "project/legacy.md".to_string(),
            category: MemoryCategory::Project,
            summary: "从旧 .axagent/memory.md 迁移的遗留记忆".to_string(),
            tags: vec!["legacy".to_string()],
        });
        self.save_index(&index).await?;
        // 删除旧文件。Windows 文件系统大小写不敏感：legacy `memory.md` 与索引
        // `MEMORY.md` 指向同一文件，删除会连刚写入的索引一起删掉，故跳过。
        // Linux/macOS 大小写敏感，二者是不同文件，可安全删除。
        #[cfg(not(windows))]
        tokio::fs::remove_file(&legacy_path).await.map_err(|e| e.to_string())?;
        tracing::info!(
            legacy = %legacy_path.display(),
            "[ProjectMemory] 旧 memory.md 已迁移至 .axagent/memory/project/legacy.md"
        );
        Ok(true)
    }

    /// v108: 自进化闭环 — 把 DB memory_items 导出到文件级 ProjectMemory。
    ///
    /// 仅导出 `tier ∈ {core, long_term}` 的记忆（短期/工作记忆不落盘文件）。
    /// - core → `user/` 目录（核心偏好，长期有效）
    /// - long_term → `project/` 目录（项目经验）
    ///
    /// 文件名基于 `title` 做 sanitize（仅保留字母数字与下划线）。
    /// 索引条目的 summary 取 `title`，tags 合并 `tags` 与 `applicability_tags`。
    /// 已存在的同名文件会被覆盖（幂等导出）。
    ///
    /// 返回成功导出的文件数量。
    pub async fn export_memory_items(
        &self,
        items: &[axagent_harness::types::MemoryItem],
    ) -> Result<usize, String> {
        if items.is_empty() {
            return Ok(0);
        }

        // 加载现有索引（保留已有条目，仅 upsert 导出的）
        let mut index = self.load_index().await;
        let mut exported = 0usize;

        for item in items {
            // tier → category 映射；short_term/working 不导出
            let category = match item.tier.as_str() {
                "core" => MemoryCategory::User,
                "long_term" => MemoryCategory::Project,
                _ => continue,
            };

            // 文件名：sanitize title，回退到 id 前 8 字符
            let base = sanitize_file_name(&item.title);
            let file_name = if base.is_empty() {
                format!("{}.md", &item.id[..item.id.len().min(8)])
            } else {
                format!("{}.md", base)
            };

            // 拼接 .md 文件内容：标题 + 元信息 + 正文
            let mut md = String::new();
            md.push_str(&format!("# {}\n\n", item.title));
            md.push_str(&format!(
                "- tier: {}\n- importance: {:.2}\n- source: {}\n- confirmed: {}\n\n",
                item.tier, item.importance, item.source, item.confirmed,
            ));
            if !item.tags.is_empty() || !item.applicability_tags.is_empty() {
                md.push_str("## Tags\n\n");
                if !item.tags.is_empty() {
                    md.push_str(&format!("- tags: {}\n", item.tags.join(", ")));
                }
                if !item.applicability_tags.is_empty() {
                    md.push_str(&format!(
                        "- applicability: {}\n",
                        item.applicability_tags.join(", ")
                    ));
                }
                md.push('\n');
            }
            md.push_str("## Content\n\n");
            md.push_str(&item.content);
            md.push('\n');

            // 写入文件
            self.save_topic_file(category, &file_name, &md).await?;

            // upsert 索引条目
            let relative_path = format!("{}/{}", category.dir_name(), file_name);
            let mut entry_tags = item.tags.clone();
            entry_tags.extend(item.applicability_tags.iter().cloned());
            entry_tags.sort();
            entry_tags.dedup();
            index.upsert(MemoryFileEntry {
                relative_path,
                category,
                summary: item.title.clone(),
                tags: entry_tags,
            });

            exported += 1;
        }

        // 保存索引（强制 200 行硬限制）
        if exported > 0 {
            self.save_index(&index).await?;
        }

        tracing::info!(
            exported,
            total = items.len(),
            "[ProjectMemory] DB memory_items 导出到文件级记忆完成"
        );
        Ok(exported)
    }
}

/// 文件名 sanitize：仅保留字母数字与下划线，空格转下划线，转小写，截断到 50 字符。
fn sanitize_file_name(s: &str) -> String {
    let sanitized: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                // 空白字符和其他非字母数字字符统一替换为下划线
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase();
    // 合并连续下划线
    let merged = sanitized.split('_').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("_");
    merged.chars().take(50).collect()
}

/// 简单分词:按空格/标点切分,转小写,过滤空词
fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}

/// 计算关键词在内容中的匹配得分
fn compute_relevance_score(content: &str, keywords: &[String]) -> f64 {
    if keywords.is_empty() {
        return 0.0;
    }
    let lower = content.to_lowercase();
    let mut total_hits: usize = 0;
    for kw in keywords {
        if kw.is_empty() {
            continue;
        }
        total_hits += lower.matches(kw.as_str()).count();
    }
    // 归一化:用内容长度 + 1 避免除零,防止长文件占优
    let content_len = lower.len().max(1);
    (total_hits as f64) / (content_len as f64) * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_category_dir_name() {
        assert_eq!(MemoryCategory::User.dir_name(), "user");
        assert_eq!(MemoryCategory::Feedback.dir_name(), "feedback");
        assert_eq!(MemoryCategory::Project.dir_name(), "project");
        assert_eq!(MemoryCategory::Reference.dir_name(), "reference");
    }

    #[test]
    fn test_memory_category_from_dir_name() {
        assert_eq!(MemoryCategory::from_dir_name("user"), Some(MemoryCategory::User));
        assert_eq!(MemoryCategory::from_dir_name("feedback"), Some(MemoryCategory::Feedback));
        assert_eq!(MemoryCategory::from_dir_name("project"), Some(MemoryCategory::Project));
        assert_eq!(MemoryCategory::from_dir_name("reference"), Some(MemoryCategory::Reference));
        assert_eq!(MemoryCategory::from_dir_name("unknown"), None);
    }

    #[test]
    fn test_memory_category_all() {
        assert_eq!(MemoryCategory::ALL.len(), 4);
        assert!(MemoryCategory::ALL.contains(&MemoryCategory::User));
        assert!(MemoryCategory::ALL.contains(&MemoryCategory::Feedback));
        assert!(MemoryCategory::ALL.contains(&MemoryCategory::Project));
        assert!(MemoryCategory::ALL.contains(&MemoryCategory::Reference));
    }

    #[test]
    fn test_memory_index_parse_empty() {
        let idx = MemoryIndex::parse("");
        assert!(idx.entries.is_empty());
    }

    #[test]
    fn test_memory_index_parse_header_only() {
        let idx = MemoryIndex::parse("# Memory Index\n");
        assert!(idx.entries.is_empty());
    }

    #[test]
    fn test_memory_index_parse_single_entry() {
        let content =
            "# Memory Index\n\n- [user/preferences.md] 用户偏好:Rust + Tauri #rust #tauri\n";
        let idx = MemoryIndex::parse(content);
        assert_eq!(idx.entries.len(), 1);
        assert_eq!(idx.entries[0].relative_path, "user/preferences.md");
        assert_eq!(idx.entries[0].category, MemoryCategory::User);
        assert_eq!(idx.entries[0].summary, "用户偏好:Rust + Tauri");
        assert_eq!(idx.entries[0].tags, vec!["rust", "tauri"]);
    }

    #[test]
    fn test_memory_index_parse_multiple_entries() {
        let content = "\
# Memory Index

- [user/preferences.md] Rust 技术栈 #rust
- [project/architecture.md] Cargo workspace 分层 #architecture
- [feedback/code_style.md] 偏好 tabs 而非 spaces #style
- [reference/links.md] 外部参考资料 #docs
";
        let idx = MemoryIndex::parse(content);
        assert_eq!(idx.entries.len(), 4);
        assert_eq!(idx.entries[0].category, MemoryCategory::User);
        assert_eq!(idx.entries[1].category, MemoryCategory::Project);
        assert_eq!(idx.entries[2].category, MemoryCategory::Feedback);
        assert_eq!(idx.entries[3].category, MemoryCategory::Reference);
    }

    #[test]
    fn test_memory_index_parse_unknown_category_defaults_to_reference() {
        let content = "- [unknown/foo.md] 未知类别\n";
        let idx = MemoryIndex::parse(content);
        assert_eq!(idx.entries.len(), 1);
        assert_eq!(idx.entries[0].category, MemoryCategory::Reference);
    }

    #[test]
    fn test_memory_index_parse_no_tags() {
        let content = "- [project/arch.md] 项目架构\n";
        let idx = MemoryIndex::parse(content);
        assert_eq!(idx.entries.len(), 1);
        assert!(idx.entries[0].tags.is_empty());
        assert_eq!(idx.entries[0].summary, "项目架构");
    }

    #[test]
    fn test_memory_index_render_empty() {
        let idx = MemoryIndex::default();
        let md = idx.render();
        assert!(md.contains("# Memory Index"));
    }

    #[test]
    fn test_memory_index_render_with_entries() {
        let mut idx = MemoryIndex::default();
        idx.entries.push(MemoryFileEntry {
            relative_path: "user/prefs.md".to_string(),
            category: MemoryCategory::User,
            summary: "用户偏好".to_string(),
            tags: vec!["rust".to_string()],
        });
        let md = idx.render();
        assert!(md.contains("# Memory Index"));
        assert!(md.contains("- [user/prefs.md] 用户偏好 #rust"));
    }

    #[test]
    fn test_memory_index_render_respects_line_limit() {
        let mut idx = MemoryIndex::default();
        // 添加 300 个条目(每个一行,加 header 2 行 = 302 行,超过 200 限制)
        for i in 0..300 {
            idx.entries.push(MemoryFileEntry {
                relative_path: format!("project/file_{}.md", i),
                category: MemoryCategory::Project,
                summary: format!("summary {}", i),
                tags: vec![],
            });
        }
        let md = idx.render();
        let line_count = md.lines().count();
        assert!(
            line_count <= mem_const::MEMORY_INDEX_MAX_LINES,
            "expected <= {} lines, got {}",
            mem_const::MEMORY_INDEX_MAX_LINES,
            line_count
        );
    }

    #[test]
    fn test_memory_index_roundtrip() {
        let mut original = MemoryIndex::default();
        original.entries.push(MemoryFileEntry {
            relative_path: "user/prefs.md".to_string(),
            category: MemoryCategory::User,
            summary: "用户偏好".to_string(),
            tags: vec!["rust".to_string(), "tauri".to_string()],
        });
        original.entries.push(MemoryFileEntry {
            relative_path: "project/arch.md".to_string(),
            category: MemoryCategory::Project,
            summary: "项目架构".to_string(),
            tags: vec![],
        });
        let md = original.render();
        let parsed = MemoryIndex::parse(&md);
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].relative_path, "user/prefs.md");
        assert_eq!(parsed.entries[0].category, MemoryCategory::User);
        assert_eq!(parsed.entries[0].summary, "用户偏好");
        assert_eq!(parsed.entries[0].tags, vec!["rust", "tauri"]);
        assert_eq!(parsed.entries[1].relative_path, "project/arch.md");
        assert_eq!(parsed.entries[1].category, MemoryCategory::Project);
        assert!(parsed.entries[1].tags.is_empty());
    }

    #[test]
    fn test_memory_index_upsert_new() {
        let mut idx = MemoryIndex::default();
        let entry = MemoryFileEntry {
            relative_path: "user/prefs.md".to_string(),
            category: MemoryCategory::User,
            summary: "用户偏好".to_string(),
            tags: vec![],
        };
        idx.upsert(entry);
        assert_eq!(idx.entries.len(), 1);
    }

    #[test]
    fn test_memory_index_upsert_existing() {
        let mut idx = MemoryIndex::default();
        idx.entries.push(MemoryFileEntry {
            relative_path: "user/prefs.md".to_string(),
            category: MemoryCategory::User,
            summary: "旧摘要".to_string(),
            tags: vec![],
        });
        let new_entry = MemoryFileEntry {
            relative_path: "user/prefs.md".to_string(),
            category: MemoryCategory::User,
            summary: "新摘要".to_string(),
            tags: vec!["rust".to_string()],
        };
        idx.upsert(new_entry);
        assert_eq!(idx.entries.len(), 1);
        assert_eq!(idx.entries[0].summary, "新摘要");
        assert_eq!(idx.entries[0].tags, vec!["rust"]);
    }

    #[test]
    fn test_memory_index_remove_existing() {
        let mut idx = MemoryIndex::default();
        idx.entries.push(MemoryFileEntry {
            relative_path: "user/prefs.md".to_string(),
            category: MemoryCategory::User,
            summary: "用户偏好".to_string(),
            tags: vec![],
        });
        assert!(idx.remove("user/prefs.md"));
        assert!(idx.entries.is_empty());
    }

    #[test]
    fn test_memory_index_remove_nonexistent() {
        let mut idx = MemoryIndex::default();
        assert!(!idx.remove("nonexistent.md"));
    }

    #[test]
    fn test_tokenize_query_basic() {
        let tokens = tokenize_query("rust tauri react");
        assert_eq!(tokens, vec!["rust", "tauri", "react"]);
    }

    #[test]
    fn test_tokenize_query_with_punctuation() {
        let tokens = tokenize_query("rust, tauri; react!");
        assert_eq!(tokens, vec!["rust", "tauri", "react"]);
    }

    #[test]
    fn test_tokenize_query_empty() {
        let tokens = tokenize_query("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_query_lowercase() {
        let tokens = tokenize_query("Rust TAURI React");
        assert_eq!(tokens, vec!["rust", "tauri", "react"]);
    }

    #[test]
    fn test_compute_relevance_score_empty_keywords() {
        let score = compute_relevance_score("some content", &[]);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_compute_relevance_score_no_match() {
        let keywords = vec!["python".to_string()];
        let score = compute_relevance_score("rust and tauri", &keywords);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_compute_relevance_score_with_match() {
        let keywords = vec!["rust".to_string()];
        let content = "rust is great, rust is fun";
        let score = compute_relevance_score(content, &keywords);
        assert!(score > 0.0);
    }

    #[test]
    fn test_compute_relevance_score_multiple_keywords() {
        let keywords = vec!["rust".to_string(), "tauri".to_string()];
        let content = "rust and tauri together";
        let score = compute_relevance_score(content, &keywords);
        assert!(score > 0.0);
    }

    #[tokio::test]
    async fn test_project_memory_new() {
        let pm = ProjectMemory::new("/test/project");
        assert_eq!(pm.project_root, PathBuf::from("/test/project"));
    }

    #[tokio::test]
    async fn test_project_memory_index_path() {
        let pm = ProjectMemory::new("/test/project");
        assert_eq!(pm.index_path(), PathBuf::from("/test/project/.axagent/MEMORY.md"));
    }

    #[tokio::test]
    async fn test_project_memory_memory_dir() {
        let pm = ProjectMemory::new("/test/project");
        assert_eq!(pm.memory_dir(), PathBuf::from("/test/project/.axagent/memory"));
    }

    #[tokio::test]
    async fn test_project_memory_category_dir() {
        let pm = ProjectMemory::new("/test/project");
        assert_eq!(
            pm.category_dir(MemoryCategory::User),
            PathBuf::from("/test/project/.axagent/memory/user")
        );
        assert_eq!(
            pm.category_dir(MemoryCategory::Project),
            PathBuf::from("/test/project/.axagent/memory/project")
        );
    }

    #[tokio::test]
    async fn test_project_memory_load_index_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let idx = pm.load_index().await;
        assert!(idx.entries.is_empty());
    }

    #[tokio::test]
    async fn test_project_memory_save_and_load_index() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let mut idx = MemoryIndex::default();
        idx.entries.push(MemoryFileEntry {
            relative_path: "user/prefs.md".to_string(),
            category: MemoryCategory::User,
            summary: "用户偏好".to_string(),
            tags: vec!["rust".to_string()],
        });
        pm.save_index(&idx).await.unwrap();
        let loaded = pm.load_index().await;
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].relative_path, "user/prefs.md");
        assert_eq!(loaded.entries[0].category, MemoryCategory::User);
        assert_eq!(loaded.entries[0].summary, "用户偏好");
        assert_eq!(loaded.entries[0].tags, vec!["rust"]);
    }

    #[tokio::test]
    async fn test_project_memory_load_category_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let result = pm.load_category(MemoryCategory::User).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_project_memory_save_and_load_topic_file() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let path =
            pm.save_topic_file(MemoryCategory::Project, "arch.md", "项目架构内容").await.unwrap();
        assert!(path.exists());
        let files = pm.load_category(MemoryCategory::Project).await.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].1, "项目架构内容");
    }

    #[tokio::test]
    async fn test_project_memory_save_topic_file_invalid_name_with_slash() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let result = pm.save_topic_file(MemoryCategory::Project, "sub/arch.md", "content").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_project_memory_save_topic_file_invalid_name_dotdot() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let result = pm.save_topic_file(MemoryCategory::Project, "../escape.md", "content").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_project_memory_save_topic_file_invalid_name_leading_dot() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let result = pm.save_topic_file(MemoryCategory::Project, ".hidden.md", "content").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_project_memory_save_topic_file_invalid_name_not_md() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let result = pm.save_topic_file(MemoryCategory::Project, "arch.txt", "content").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_project_memory_save_topic_file_too_large() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        // 创建超过 256KB 的内容
        let huge_content = "a".repeat(mem_const::MEMORY_FILE_SIZE_LIMIT + 1);
        let result = pm.save_topic_file(MemoryCategory::Project, "huge.md", &huge_content).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_project_memory_delete_topic_file() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        pm.save_topic_file(MemoryCategory::User, "prefs.md", "content").await.unwrap();
        let deleted = pm.delete_topic_file(MemoryCategory::User, "prefs.md").await.unwrap();
        assert!(deleted);
        // 再次删除应返回 false
        let deleted_again = pm.delete_topic_file(MemoryCategory::User, "prefs.md").await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_project_memory_delete_topic_file_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let deleted = pm.delete_topic_file(MemoryCategory::User, "nonexistent.md").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_project_memory_scan_relevant_files_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let results = pm.scan_relevant_files("rust", 5).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_project_memory_scan_relevant_files_with_matches() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        pm.save_topic_file(MemoryCategory::User, "prefs.md", "rust and tauri").await.unwrap();
        pm.save_topic_file(MemoryCategory::Project, "arch.md", "python and django").await.unwrap();
        let results = pm.scan_relevant_files("rust", 5).await.unwrap();
        assert!(!results.is_empty());
        // rust 匹配 user/prefs.md
        assert_eq!(results[0].category, MemoryCategory::User);
        assert!(results[0].score > 0.0);
    }

    #[tokio::test]
    async fn test_project_memory_scan_relevant_files_respects_limit() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        // 创建 3 个文件
        for i in 0..3 {
            pm.save_topic_file(MemoryCategory::Project, &format!("file_{}.md", i), "rust content")
                .await
                .unwrap();
        }
        let results = pm.scan_relevant_files("rust", 2).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_project_memory_scan_relevant_files_default_limit() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        // 创建 7 个文件(超过默认限制 5)
        for i in 0..7 {
            pm.save_topic_file(MemoryCategory::Project, &format!("file_{}.md", i), "rust content")
                .await
                .unwrap();
        }
        let results = pm.scan_relevant_files("rust", 0).await.unwrap();
        assert_eq!(results.len(), mem_const::MEMORY_RELEVANT_FILES_LIMIT);
    }

    #[tokio::test]
    async fn test_project_memory_migrate_from_legacy_no_legacy() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let migrated = pm.migrate_from_legacy().await.unwrap();
        assert!(!migrated);
    }

    #[tokio::test]
    async fn test_project_memory_migrate_from_legacy_with_legacy() {
        let dir = tempfile::tempdir().unwrap();
        // 创建旧文件
        let axagent_dir = dir.path().join(".axagent");
        std::fs::create_dir_all(&axagent_dir).unwrap();
        std::fs::write(axagent_dir.join("memory.md"), "# Legacy Memory\n\n- old content\n")
            .unwrap();
        let pm = ProjectMemory::new(dir.path());
        let migrated = pm.migrate_from_legacy().await.unwrap();
        assert!(migrated);
        // 旧文件应已删除——Windows 大小写不敏感下 memory.md 与索引 MEMORY.md 同一文件，
        // 迁移会保留（内容已被索引覆盖），仅 Linux/macOS 真正删除。
        #[cfg(not(windows))]
        assert!(!axagent_dir.join("memory.md").exists());
        // 新文件应已创建
        assert!(pm.topic_file_path(MemoryCategory::Project, "legacy.md").exists());
        // 索引应已创建
        let idx = pm.load_index().await;
        assert_eq!(idx.entries.len(), 1);
        assert_eq!(idx.entries[0].relative_path, "project/legacy.md");
    }

    #[tokio::test]
    async fn test_project_memory_migrate_from_legacy_already_migrated() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        // 先创建新结构（save_topic_file 会建 `.axagent/memory/` 目录）
        pm.save_topic_file(MemoryCategory::Project, "existing.md", "already there").await.unwrap();
        // 创建旧文件
        let axagent_dir = dir.path().join(".axagent");
        std::fs::create_dir_all(&axagent_dir).unwrap();
        std::fs::write(axagent_dir.join("memory.md"), "legacy").unwrap();
        let migrated = pm.migrate_from_legacy().await.unwrap();
        assert!(!migrated);
        // 旧文件应保留(未迁移)
        assert!(axagent_dir.join("memory.md").exists());
    }

    // ── v108: sanitize_file_name 单元测试 ──────────────────────────

    #[test]
    fn test_sanitize_file_name_basic() {
        assert_eq!(sanitize_file_name("Rust Preferences"), "rust_preferences");
    }

    #[test]
    fn test_sanitize_file_name_special_chars() {
        // 特殊字符替换为下划线，连续下划线合并
        assert_eq!(sanitize_file_name("a@b#c$d"), "a_b_c_d");
    }

    #[test]
    fn test_sanitize_file_name_empty() {
        assert_eq!(sanitize_file_name(""), "");
        assert_eq!(sanitize_file_name("   "), "");
    }

    #[test]
    fn test_sanitize_file_name_leading_trailing_underscores() {
        // 前后下划线被 filter(|s| !s.is_empty()) 去除
        assert_eq!(sanitize_file_name("__test__"), "test");
    }

    #[test]
    fn test_sanitize_file_name_truncation() {
        let long = "a".repeat(100);
        let result = sanitize_file_name(&long);
        assert_eq!(result.len(), 50);
    }

    #[test]
    fn test_sanitize_file_name_unicode() {
        // 中文字符 is_alphanumeric() 为 true，保留原样
        assert_eq!(sanitize_file_name("用户偏好"), "用户偏好");
        // 混合：空格变为下划线
        assert_eq!(sanitize_file_name("rust 用户"), "rust_用户");
    }

    // ── v108: export_memory_items 集成测试 ──────────────────────────

    #[tokio::test]
    async fn test_export_memory_items_empty() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let exported = pm.export_memory_items(&[]).await.unwrap();
        assert_eq!(exported, 0);
    }

    #[tokio::test]
    async fn test_export_memory_items_core_tier() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let item = axagent_harness::types::MemoryItem {
            id: "test123".to_string(),
            namespace_id: "ns1".to_string(),
            title: "Rust Core Preference".to_string(),
            content: "Always use cargo clippy".to_string(),
            source: "reflector".to_string(),
            index_status: "ready".to_string(),
            index_error: None,
            updated_at: "2026-07-28T00:00:00Z".to_string(),
            tier: "core".to_string(),
            importance: 0.9,
            access_count: 5,
            last_accessed: None,
            decay_rate: 0.001,
            expires_at: None,
            memory_nature: "semantic".to_string(),
            tags: vec!["rust".to_string()],
            source_conversation_id: None,
            source_message_id: None,
            applicability_tags: vec!["rust".to_string()],
            confirmed: 1,
        };
        let exported = pm.export_memory_items(&[item]).await.unwrap();
        assert_eq!(exported, 1);
        // 文件应写入 user/ 目录（core → User）
        let user_dir = dir.path().join(".axagent/memory/user");
        assert!(user_dir.exists());
        let files: Vec<_> = std::fs::read_dir(&user_dir).unwrap().collect();
        assert_eq!(files.len(), 1);
        // 索引应更新
        let index = pm.load_index().await;
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].category, MemoryCategory::User);
    }

    #[tokio::test]
    async fn test_export_memory_items_long_term_tier() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let item = axagent_harness::types::MemoryItem {
            id: "lt456".to_string(),
            namespace_id: "ns1".to_string(),
            title: "Project Architecture".to_string(),
            content: "Cargo workspace with 32 crates".to_string(),
            source: "manual".to_string(),
            index_status: "ready".to_string(),
            index_error: None,
            updated_at: "2026-07-28T00:00:00Z".to_string(),
            tier: "long_term".to_string(),
            importance: 0.7,
            access_count: 3,
            last_accessed: None,
            decay_rate: 0.005,
            expires_at: None,
            memory_nature: "semantic".to_string(),
            tags: vec![],
            source_conversation_id: None,
            source_message_id: None,
            applicability_tags: vec![],
            confirmed: 0,
        };
        let exported = pm.export_memory_items(&[item]).await.unwrap();
        assert_eq!(exported, 1);
        // 文件应写入 project/ 目录（long_term → Project）
        let project_dir = dir.path().join(".axagent/memory/project");
        assert!(project_dir.exists());
    }

    #[tokio::test]
    async fn test_export_memory_items_skip_short_term() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let item = axagent_harness::types::MemoryItem {
            id: "st789".to_string(),
            namespace_id: "ns1".to_string(),
            title: "Temp Note".to_string(),
            content: "Temporary".to_string(),
            source: "manual".to_string(),
            index_status: "ready".to_string(),
            index_error: None,
            updated_at: "2026-07-28T00:00:00Z".to_string(),
            tier: "short_term".to_string(),
            importance: 0.3,
            access_count: 0,
            last_accessed: None,
            decay_rate: 0.1,
            expires_at: None,
            memory_nature: "episodic".to_string(),
            tags: vec![],
            source_conversation_id: None,
            source_message_id: None,
            applicability_tags: vec![],
            confirmed: 0,
        };
        let exported = pm.export_memory_items(&[item]).await.unwrap();
        assert_eq!(exported, 0);
        // 不应创建任何目录
        assert!(!dir.path().join(".axagent/memory/user").exists());
        assert!(!dir.path().join(".axagent/memory/project").exists());
    }

    #[tokio::test]
    async fn test_export_memory_items_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        let item = axagent_harness::types::MemoryItem {
            id: "idem001".to_string(),
            namespace_id: "ns1".to_string(),
            title: "Idempotent Test".to_string(),
            content: "Content v1".to_string(),
            source: "manual".to_string(),
            index_status: "ready".to_string(),
            index_error: None,
            updated_at: "2026-07-28T00:00:00Z".to_string(),
            tier: "core".to_string(),
            importance: 0.8,
            access_count: 0,
            last_accessed: None,
            decay_rate: 0.001,
            expires_at: None,
            memory_nature: "semantic".to_string(),
            tags: vec![],
            source_conversation_id: None,
            source_message_id: None,
            applicability_tags: vec![],
            confirmed: 1,
        };
        // 第一次导出
        let exported1 = pm.export_memory_items(std::slice::from_ref(&item)).await.unwrap();
        assert_eq!(exported1, 1);
        // 第二次导出（覆盖）
        let exported2 = pm.export_memory_items(std::slice::from_ref(&item)).await.unwrap();
        assert_eq!(exported2, 1);
        // 索引应只有 1 个条目（upsert 覆盖）
        let index = pm.load_index().await;
        assert_eq!(index.entries.len(), 1);
    }
}
