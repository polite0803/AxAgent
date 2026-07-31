// SPDX-License-Identifier: AGPL-3.0-only

//! Obsidian Vault 集成工具
//!
//! 参考 DeepTutor `deeptutor/capabilities/obsidian/` 的设计：
//! - 9 个语义化工具 `obsidian_*`（search / read / list / backlinks / links /
//!   tags / create_note / append / set_property），与 LLM 工具表对齐
//! - `FsVaultSource` 实现 `axagent_harness::vault::VaultSource` trait，
//!   所有路径操作走 `safe_join` 强制锁定在 vault 根下，防止路径穿越
//! - vault_root 通过 `ToolContext.extra["vault_root"]` 注入；
//!   ConnectedVault 类型 KB 在 wiring 层启动时设置该字段
//!
//! ## 安全铁律
//! - 所有写入操作（create_note / append_note / set_property）仅允许追加式
//!   或 frontmatter 字段级更新；不暴露任意覆盖/删除接口
//! - 路径解析强制 `canonicalize` 后必须仍位于 vault_root 之下
//! - 拒绝绝对路径、`..` 段、null 字节
//! - 创建笔记若路径已存在，返回 `NoteAlreadyExists` 错误，不覆盖

use crate::{Tool, ToolCategory, ToolContext, ToolDomain, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_harness::vault::{
    LinkHit, NoteContent, NoteHit, OBSIDIAN_TOOL_NAMES, TagCount, VaultError, VaultSource,
};
use axagent_kit::markdown_parser::{
    extract_inline_tags as kit_extract_inline_tags, extract_wikilinks as kit_extract_wikilinks,
    make_snippet as kit_make_snippet, split_frontmatter as kit_split_frontmatter,
};
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

// ── FsVaultSource ───────────────────────────────────────────────────

/// 基于 filesystem 的 `VaultSource` 实现
///
/// 在 wiring 层启动时为每个 ConnectedVault KB 构造一份，注册到
/// `VaultRegistry`；工具调用时通过 `vault_kb_id` 取回。
pub struct FsVaultSource {
    root: PathBuf,
}

impl FsVaultSource {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, VaultError> {
        let root = root.into();
        let canonical = std::fs::canonicalize(&root).map_err(VaultError::Io)?;
        if !canonical.is_dir() {
            return Err(VaultError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("vault root is not a directory: {}", canonical.display()),
            )));
        }
        Ok(Self { root: canonical })
    }

    /// 把 user 提供的 note 引用（裸名或 vault 相对路径）解析为 vault 内绝对路径
    ///
    /// 规则：
    /// - 自动补 `.md` 扩展（若原串无扩展）
    /// - 拒绝绝对路径、`..` 段
    /// - canonicalize 后必须仍位于 vault_root 下
    fn safe_join(&self, note: &str) -> Result<PathBuf, VaultError> {
        if note.is_empty() {
            return Err(VaultError::InvalidRef("note reference is empty".to_string()));
        }
        if note.contains('\0') {
            return Err(VaultError::InvalidRef("note reference contains NUL".to_string()));
        }
        let p = Path::new(note);
        if p.is_absolute() {
            return Err(VaultError::PathEscapes(note.to_string()));
        }
        // 拒绝任何 `..` 段
        if p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return Err(VaultError::PathEscapes(note.to_string()));
        }
        // 自动补 .md 扩展
        let with_ext: PathBuf = if note.ends_with(".md") {
            p.to_path_buf()
        } else {
            // 处理 "work/Plan" → "work/Plan.md"
            let mut s = note.to_string();
            s.push_str(".md");
            PathBuf::from(s)
        };

        let joined = self.root.join(&with_ext);
        // canonicalize 父目录后拼接文件名，避免文件不存在时 canonicalize 失败
        let parent = joined.parent().unwrap_or(&self.root);
        let canonical = match parent.canonicalize() {
            Ok(canonical_parent) => {
                let file_name = joined.file_name().ok_or_else(|| {
                    VaultError::InvalidRef(format!("invalid file name in note: {}", note))
                })?;
                canonical_parent.join(file_name)
            },
            Err(_) => {
                // 父目录尚不存在（如新子目录），回退到直接路径拼接；
                // 前面已校验过 .. 和绝对路径，此处安全
                if !joined.starts_with(&self.root) {
                    return Err(VaultError::PathEscapes(note.to_string()));
                }
                joined
            },
        };

        if !canonical.starts_with(&self.root) {
            return Err(VaultError::PathEscapes(note.to_string()));
        }
        Ok(canonical)
    }

    /// 把绝对路径转回 vault 相对路径（用 `/` 分隔）
    fn rel_path(&self, abs: &Path) -> String {
        abs.strip_prefix(&self.root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| abs.to_string_lossy().to_string())
    }
}

#[async_trait]
impl VaultSource for FsVaultSource {
    fn vault_root(&self) -> &Path {
        &self.root
    }

    async fn search_notes(&self, query: &str, limit: usize) -> Result<Vec<NoteHit>, VaultError> {
        let q = query.to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let root = self.root.clone();
        let hits = tokio::task::spawn_blocking(move || -> Result<Vec<NoteHit>, VaultError> {
            let mut out = Vec::new();
            for entry in walkdir::WalkDir::new(&root)
                .into_iter()
                .filter_entry(|e| {
                    let name = e.file_name();
                    let s = name.to_string_lossy();
                    !(s.starts_with('.') && s != ".") // 跳过 .obsidian 等
                })
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(path) else {
                    continue;
                };
                let lower = content.to_lowercase();
                if let Some(idx) = lower.find(&q) {
                    let snippet = kit_make_snippet(&content, idx, 200);
                    out.push(NoteHit {
                        path: path
                            .strip_prefix(&root)
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                            .unwrap_or_default()
                            .trim_end_matches(".md")
                            .to_string(),
                        snippet,
                    });
                    if out.len() >= limit {
                        break;
                    }
                }
            }
            Ok(out)
        })
        .await
        .map_err(|e| VaultError::Io(std::io::Error::other(e.to_string())))??;
        Ok(hits)
    }

    async fn read_note(&self, note: &str) -> Result<NoteContent, VaultError> {
        let path = self.safe_join(note)?;
        let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                VaultError::NoteNotFound(note.to_string())
            } else {
                VaultError::Io(e)
            }
        })?;
        let (frontmatter, body) = kit_split_frontmatter(&content);
        Ok(NoteContent { path: self.rel_path(&path), frontmatter, body })
    }

    async fn list_notes(
        &self,
        folder: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, VaultError> {
        let base = if let Some(f) = folder {
            self.safe_join(f)?
        } else {
            self.root.clone()
        };
        let root = self.root.clone();
        let list = tokio::task::spawn_blocking(move || -> Result<Vec<String>, VaultError> {
            let mut out = Vec::new();
            for entry in
                walkdir::WalkDir::new(&base).max_depth(64).into_iter().filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let rel = path
                    .strip_prefix(&root)
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                out.push(rel);
                if out.len() >= limit {
                    break;
                }
            }
            out.sort();
            Ok(out)
        })
        .await
        .map_err(|e| VaultError::Io(std::io::Error::other(e.to_string())))??;
        Ok(list)
    }

    async fn backlinks(&self, note: &str, limit: usize) -> Result<Vec<LinkHit>, VaultError> {
        let target = note.trim_end_matches(".md").to_string();
        let pattern_lower = format!("[[{}", target.to_lowercase());
        let root = self.root.clone();
        let hits = tokio::task::spawn_blocking(move || -> Result<Vec<LinkHit>, VaultError> {
            let mut out = Vec::new();
            for entry in walkdir::WalkDir::new(&root)
                .into_iter()
                .filter_entry(|e| {
                    let s = e.file_name();
                    !s.to_string_lossy().starts_with('.')
                })
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(path) else {
                    continue;
                };
                let lower = content.to_lowercase();
                if let Some(idx) = lower.find(&pattern_lower) {
                    let snippet = kit_make_snippet(&content, idx, 200);
                    out.push(LinkHit {
                        path: path
                            .strip_prefix(&root)
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                            .unwrap_or_default(),
                        snippet,
                    });
                    if out.len() >= limit {
                        break;
                    }
                }
            }
            Ok(out)
        })
        .await
        .map_err(|e| VaultError::Io(std::io::Error::other(e.to_string())))??;
        Ok(hits)
    }

    async fn outgoing_links(&self, note: &str) -> Result<Vec<String>, VaultError> {
        let content = self.read_note(note).await?;
        Ok(kit_extract_wikilinks(&content.body))
    }

    async fn collect_tags(&self, limit: usize) -> Result<Vec<TagCount>, VaultError> {
        let root = self.root.clone();
        let tags = tokio::task::spawn_blocking(move || -> Result<Vec<TagCount>, VaultError> {
            let mut counter: HashMap<String, i64> = HashMap::new();
            for entry in walkdir::WalkDir::new(&root)
                .into_iter()
                .filter_entry(|e| {
                    let s = e.file_name();
                    !s.to_string_lossy().starts_with('.')
                })
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(path) else {
                    continue;
                };
                // inline #tag
                for tag in kit_extract_inline_tags(&content) {
                    *counter.entry(tag).or_insert(0) += 1;
                }
                // frontmatter tags
                let (fm, _) = kit_split_frontmatter(&content);
                if let Some(arr) = fm.get("tags").and_then(|v| v.as_array()) {
                    for t in arr {
                        if let Some(s) = t.as_str() {
                            *counter.entry(s.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
            let mut out: Vec<TagCount> =
                counter.into_iter().map(|(tag, count)| TagCount { tag, count }).collect();
            out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.tag.cmp(&b.tag)));
            out.truncate(limit);
            Ok(out)
        })
        .await
        .map_err(|e| VaultError::Io(std::io::Error::other(e.to_string())))??;
        Ok(tags)
    }

    async fn create_note(
        &self,
        path: &str,
        content: &str,
        frontmatter: Option<Value>,
    ) -> Result<String, VaultError> {
        let abs = self.safe_join(path)?;
        if abs.exists() {
            return Err(VaultError::NoteAlreadyExists(path.to_string()));
        }
        let body = match &frontmatter {
            Some(fm) if !fm.is_null() && fm.as_object().map(|o| !o.is_empty()).unwrap_or(false) => {
                let yaml =
                    serde_yaml::to_string(fm).map_err(|e| VaultError::Yaml(e.to_string()))?;
                format!("---\n{}---\n\n{}", yaml, content)
            },
            _ => content.to_string(),
        };
        if let Some(parent) = abs.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&abs, body).await?;
        Ok(self.rel_path(&abs))
    }

    async fn append_note(&self, note: &str, content: &str) -> Result<String, VaultError> {
        let abs = self.safe_join(note)?;
        if !abs.exists() {
            return Err(VaultError::NoteNotFound(note.to_string()));
        }
        let mut original = tokio::fs::read_to_string(&abs).await?;
        if !original.ends_with('\n') {
            original.push('\n');
        }
        original.push_str(content);
        tokio::fs::write(&abs, &original).await?;
        Ok(self.rel_path(&abs))
    }

    async fn set_property(
        &self,
        note: &str,
        key: &str,
        value: Value,
    ) -> Result<String, VaultError> {
        let abs = self.safe_join(note)?;
        if !abs.exists() {
            return Err(VaultError::NoteNotFound(note.to_string()));
        }
        let raw = tokio::fs::read_to_string(&abs).await?;
        let (mut fm, body) = kit_split_frontmatter(&raw);
        if let Some(obj) = fm.as_object_mut() {
            obj.insert(key.to_string(), value);
        } else {
            // 无 frontmatter 或非 object，重建为 object
            let mut map = serde_json::Map::new();
            map.insert(key.to_string(), value);
            fm = Value::Object(map);
        }
        let yaml = serde_yaml::to_string(&fm).map_err(|e| VaultError::Yaml(e.to_string()))?;
        let new_content = if raw.starts_with("---") {
            // 替换原 frontmatter
            let body_start = raw.find("\n---\n").map(|i| i + "\n---\n".len()).unwrap_or(0);
            format!("---\n{}---\n{}", yaml, &raw[body_start..])
        } else {
            format!("---\n{}---\n\n{}", yaml, body)
        };
        tokio::fs::write(&abs, &new_content).await?;
        Ok(self.rel_path(&abs))
    }
}

// ── 工具函数 ────────────────────────────────────────────────────────

// ── Vault Registry（kb_id → Arc<dyn VaultSource>）──────────────────

/// 全局 vault 注册表
///
/// 在 wiring 层启动时调用 `register(kb_id, root)` 注册每个 ConnectedVault KB。
/// 工具通过 `ToolContext.extra["vault_kb_id"]` 取 kb_id，再查表拿 VaultSource。
fn vault_registry() -> &'static RwLock<HashMap<String, Arc<dyn VaultSource>>> {
    static REGISTRY: OnceLock<RwLock<HashMap<String, Arc<dyn VaultSource>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 注册一个 vault（wiring 层启动时调用）
pub fn register_vault(kb_id: &str, root: PathBuf) -> Result<(), VaultError> {
    let source: Arc<dyn VaultSource> = Arc::new(FsVaultSource::new(root)?);
    vault_registry().write().insert(kb_id.to_string(), source);
    Ok(())
}

/// 注销 vault（删除 ConnectedVault KB 时调用）
pub fn unregister_vault(kb_id: &str) {
    vault_registry().write().remove(kb_id);
}

/// 取已注册的 vault（工具内部使用）
fn get_vault(kb_id: &str) -> Result<Arc<dyn VaultSource>, VaultError> {
    vault_registry().read().get(kb_id).cloned().ok_or(VaultError::NotBound)
}

/// 从 ToolContext.extra 取 vault_kb_id
fn vault_kb_from_ctx(ctx: &ToolContext) -> Result<String, VaultError> {
    ctx.extra.get("vault_kb_id").cloned().ok_or(VaultError::NotBound)
}

// ── 9 个语义化工具 ──────────────────────────────────────────────────

macro_rules! vault_tool_impl {
    ($struct_name:ident, $tool_name:literal, $desc:literal, $schema:expr, $read_only:expr, $handler:expr) => {
        pub struct $struct_name;

        #[async_trait]
        impl Tool for $struct_name {
            fn name(&self) -> &str {
                $tool_name
            }
            fn description(&self) -> &str {
                $desc
            }
            fn input_schema(&self) -> Value {
                $schema
            }
            fn category(&self) -> ToolCategory {
                ToolCategory::Integration
            }
            fn domain(&self) -> ToolDomain {
                ToolDomain::General
            }
            fn is_concurrency_safe(&self) -> bool {
                true
            }
            fn is_read_only(&self) -> bool {
                $read_only
            }

            async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
                let kb_id = vault_kb_from_ctx(ctx).map_err(|e| {
                    ToolError::execution_failed(format!(
                        "Obsidian vault not bound: {}. 请先在 KB 设置中连接 Obsidian Vault。",
                        e
                    ))
                })?;
                let vault = get_vault(&kb_id).map_err(|e| {
                    ToolError::execution_failed(format!("Vault source not registered: {}", e))
                })?;
                // 闭包接收 Arc<dyn VaultSource>，move 进 async block 避免 &dyn 生命周期冲突
                ($handler)(vault, input).await
            }
        }
    };
}

fn json_input_err(msg: impl Into<String>) -> ToolError {
    ToolError::invalid_input(msg.into())
}

fn missing_field(field: &str) -> ToolError {
    json_input_err(format!("missing required field: {}", field))
}

fn get_str(input: &Value, field: &str) -> Result<String, ToolError> {
    input
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| missing_field(field))
}

fn get_usize(input: &Value, field: &str, default: usize) -> usize {
    input.get(field).and_then(|v| v.as_u64()).map(|n| n as usize).unwrap_or(default)
}

// 1. obsidian_search
vault_tool_impl! {
    ObsidianSearchTool,
    "obsidian_search",
    "在已连接的 Obsidian Vault 中搜索笔记（标题+正文，大小写不敏感）。返回命中路径与上下文片段。",
    serde_json::json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "搜索关键词" },
            "limit": { "type": "integer", "default": 20, "minimum": 1, "maximum": 200 }
        },
        "required": ["query"]
    }),
    true,
    |vault: Arc<dyn VaultSource>, input: Value| async move {
        let query = get_str(&input, "query")?;
        let limit = get_usize(&input, "limit", 20);
        let hits = vault.search_notes(&query, limit).await
            .map_err(|e| ToolError::execution_failed(format!("search_notes failed: {}", e)))?;
        let json = serde_json::to_string_pretty(&hits)
            .map_err(|e| ToolError::execution_failed(format!("serialize failed: {}", e)))?;
        Ok(ToolResult::success(json))
    }
}

// 2. obsidian_read
vault_tool_impl! {
    ObsidianReadTool,
    "obsidian_read",
    "读取 Obsidian Vault 中指定笔记的内容（含 YAML frontmatter 与正文）。",
    serde_json::json!({
        "type": "object",
        "properties": {
            "note": { "type": "string", "description": "笔记路径或裸名（自动补 .md）" }
        },
        "required": ["note"]
    }),
    true,
    |vault: Arc<dyn VaultSource>, input: Value| async move {
        let note = get_str(&input, "note")?;
        let content = vault.read_note(&note).await
            .map_err(|e| ToolError::execution_failed(format!("read_note failed: {}", e)))?;
        let formatted = format!(
            "Path: {}\nFrontmatter: {}\n\n---\n\n{}",
            content.path,
            serde_json::to_string(&content.frontmatter).unwrap_or_default(),
            content.body
        );
        Ok(ToolResult::success(formatted))
    }
}

// 3. obsidian_list
vault_tool_impl! {
    ObsidianListTool,
    "obsidian_list",
    "列出 Obsidian Vault 或子文件夹下的笔记路径。",
    serde_json::json!({
        "type": "object",
        "properties": {
            "folder": { "type": "string", "description": "可选子文件夹路径，留空列整个 vault" },
            "limit": { "type": "integer", "default": 100, "minimum": 1, "maximum": 2000 }
        }
    }),
    true,
    |vault: Arc<dyn VaultSource>, input: Value| async move {
        let folder = input.get("folder").and_then(|v| v.as_str());
        let limit = get_usize(&input, "limit", 100);
        let notes = vault.list_notes(folder, limit).await
            .map_err(|e| ToolError::execution_failed(format!("list_notes failed: {}", e)))?;
        Ok(ToolResult::success(notes.join("\n")))
    }
}

// 4. obsidian_backlinks
vault_tool_impl! {
    ObsidianBacklinksTool,
    "obsidian_backlinks",
    "查找 Obsidian Vault 中通过 [[note]] 链向指定笔记的反向链接。",
    serde_json::json!({
        "type": "object",
        "properties": {
            "note": { "type": "string" },
            "limit": { "type": "integer", "default": 20, "minimum": 1, "maximum": 200 }
        },
        "required": ["note"]
    }),
    true,
    |vault: Arc<dyn VaultSource>, input: Value| async move {
        let note = get_str(&input, "note")?;
        let limit = get_usize(&input, "limit", 20);
        let hits = vault.backlinks(&note, limit).await
            .map_err(|e| ToolError::execution_failed(format!("backlinks failed: {}", e)))?;
        let json = serde_json::to_string_pretty(&hits)
            .map_err(|e| ToolError::execution_failed(format!("serialize failed: {}", e)))?;
        Ok(ToolResult::success(json))
    }
}

// 5. obsidian_links
vault_tool_impl! {
    ObsidianLinksTool,
    "obsidian_links",
    "查找 Obsidian 指定笔记通过 [[...]] 链向的其他笔记。",
    serde_json::json!({
        "type": "object",
        "properties": {
            "note": { "type": "string" }
        },
        "required": ["note"]
    }),
    true,
    |vault: Arc<dyn VaultSource>, input: Value| async move {
        let note = get_str(&input, "note")?;
        let links = vault.outgoing_links(&note).await
            .map_err(|e| ToolError::execution_failed(format!("outgoing_links failed: {}", e)))?;
        Ok(ToolResult::success(links.join("\n")))
    }
}

// 6. obsidian_tags
vault_tool_impl! {
    ObsidianTagsTool,
    "obsidian_tags",
    "统计 Obsidian Vault 内所有 tag 的频次（inline #tag + frontmatter tags）。",
    serde_json::json!({
        "type": "object",
        "properties": {
            "limit": { "type": "integer", "default": 50, "minimum": 1, "maximum": 1000 }
        }
    }),
    true,
    |vault: Arc<dyn VaultSource>, input: Value| async move {
        let limit = get_usize(&input, "limit", 50);
        let tags = vault.collect_tags(limit).await
            .map_err(|e| ToolError::execution_failed(format!("collect_tags failed: {}", e)))?;
        let json = serde_json::to_string_pretty(&tags)
            .map_err(|e| ToolError::execution_failed(format!("serialize failed: {}", e)))?;
        Ok(ToolResult::success(json))
    }
}

// 7. obsidian_create_note
vault_tool_impl! {
    ObsidianCreateNoteTool,
    "obsidian_create_note",
    "在 Obsidian Vault 中创建新笔记（拒绝覆盖已存在笔记；frontmatter 可选）。",
    serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "vault 相对路径，如 \"work/Plan\" 或 \"work/Plan.md\"" },
            "content": { "type": "string", "description": "Markdown 正文（不含 frontmatter）" },
            "frontmatter": { "type": "object", "description": "可选 YAML frontmatter 字段" }
        },
        "required": ["path", "content"]
    }),
    false,
    |vault: Arc<dyn VaultSource>, input: Value| async move {
        let path = get_str(&input, "path")?;
        let content = get_str(&input, "content")?;
        let fm = input.get("frontmatter").cloned();
        let created = vault.create_note(&path, &content, fm).await
            .map_err(|e| ToolError::execution_failed(format!("create_note failed: {}", e)))?;
        Ok(ToolResult::success(format!("Created: {}", created)))
    }
}

// 8. obsidian_append
vault_tool_impl! {
    ObsidianAppendTool,
    "obsidian_append",
    "向 Obsidian Vault 中已存在的笔记追加内容（仅追加，不修改原文）。",
    serde_json::json!({
        "type": "object",
        "properties": {
            "note": { "type": "string" },
            "content": { "type": "string", "description": "要追加的文本" }
        },
        "required": ["note", "content"]
    }),
    false,
    |vault: Arc<dyn VaultSource>, input: Value| async move {
        let note = get_str(&input, "note")?;
        let content = get_str(&input, "content")?;
        let updated = vault.append_note(&note, &content).await
            .map_err(|e| ToolError::execution_failed(format!("append_note failed: {}", e)))?;
        Ok(ToolResult::success(format!("Appended to: {}", updated)))
    }
}

// 9. obsidian_set_property
vault_tool_impl! {
    ObsidianSetPropertyTool,
    "obsidian_set_property",
    "设置 Obsidian Vault 笔记的 YAML frontmatter 单个字段（不改正文）。",
    serde_json::json!({
        "type": "object",
        "properties": {
            "note": { "type": "string" },
            "key": { "type": "string", "description": "frontmatter 字段名" },
            "value": { "description": "字段值（任意 JSON 类型）" }
        },
        "required": ["note", "key", "value"]
    }),
    false,
    |vault: Arc<dyn VaultSource>, input: Value| async move {
        let note = get_str(&input, "note")?;
        let key = get_str(&input, "key")?;
        let value = input.get("value").cloned().ok_or_else(|| missing_field("value"))?;
        let updated = vault.set_property(&note, &key, value).await
            .map_err(|e| ToolError::execution_failed(format!("set_property failed: {}", e)))?;
        Ok(ToolResult::success(format!("Updated: {}", updated)))
    }
}

// ── 公开导出 ────────────────────────────────────────────────────────

/// 返回 9 个 obsidian_* 工具的实例（用于注册到 ToolRegistry）
pub fn all_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(ObsidianSearchTool),
        Arc::new(ObsidianReadTool),
        Arc::new(ObsidianListTool),
        Arc::new(ObsidianBacklinksTool),
        Arc::new(ObsidianLinksTool),
        Arc::new(ObsidianTagsTool),
        Arc::new(ObsidianCreateNoteTool),
        Arc::new(ObsidianAppendTool),
        Arc::new(ObsidianSetPropertyTool),
    ]
}

/// 9 个工具的名字（用于校验、白名单过滤）
pub fn tool_names() -> &'static [&'static str] {
    OBSIDIAN_TOOL_NAMES
}

// 保持 KbKind / NoteRef / 等类型在本模块可见，便于 wiring 层直接 import
pub use axagent_harness::vault::{KbKind as ReExportedKbKind, NoteRef as ReExportedNoteRef};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_frontmatter_with_yaml() {
        let content = "---\ntitle: Test\ntags: [a, b]\n---\n\nHello world";
        let (fm, body) = split_frontmatter(content);
        assert_eq!(fm["title"], "Test");
        assert!(body.contains("Hello world"));
    }

    #[test]
    fn test_split_frontmatter_without_yaml() {
        let content = "Just plain markdown";
        let (fm, body) = split_frontmatter(content);
        assert!(fm.is_object());
        assert_eq!(body, content);
    }

    #[test]
    fn test_extract_wikilinks() {
        let body = "See [[Project Plan]] and [[Work/Meeting#section|alias]] for details.";
        let links = extract_wikilinks(body);
        assert_eq!(links, vec!["Project Plan", "Work/Meeting"]);
    }

    #[test]
    fn test_extract_inline_tags() {
        let content = "# Heading\n\ntext #foo and #bar/baz\n#tag_at_start";
        let tags = extract_inline_tags(content);
        assert!(tags.contains(&"foo".to_string()));
        assert!(tags.contains(&"bar/baz".to_string()));
        assert!(tags.contains(&"tag_at_start".to_string()));
    }

    #[tokio::test]
    async fn test_fs_vault_source_safe_join() {
        let tmp = tempfile::tempdir().unwrap();
        let vault = FsVaultSource::new(tmp.path()).unwrap();
        // 正常路径
        let p = vault.safe_join("note").unwrap();
        assert!(p.starts_with(tmp.path()));
        assert_eq!(p.extension().unwrap(), "md");

        // 路径穿越
        assert!(matches!(vault.safe_join("../../etc/passwd"), Err(VaultError::PathEscapes(_))));
        // 绝对路径
        assert!(matches!(vault.safe_join("/etc/passwd"), Err(VaultError::PathEscapes(_))));
        // 空字符串
        assert!(matches!(vault.safe_join(""), Err(VaultError::InvalidRef(_))));
    }

    #[tokio::test]
    async fn test_fs_vault_source_create_read_append() {
        let tmp = tempfile::tempdir().unwrap();
        let vault = FsVaultSource::new(tmp.path()).unwrap();
        let fm = serde_json::json!({"title": "Test Note", "tags": ["demo"]});

        // 创建
        let path = vault.create_note("work/Plan", "Initial content", Some(fm)).await.unwrap();
        assert!(path.contains("work/Plan.md"));

        // 重复创建应失败
        let err = vault.create_note("work/Plan", "again", None).await.unwrap_err();
        assert!(matches!(err, VaultError::NoteAlreadyExists(_)));

        // 读取
        let note = vault.read_note("work/Plan").await.unwrap();
        assert_eq!(note.frontmatter["title"], "Test Note");
        assert!(note.body.contains("Initial content"));

        // 追加
        vault.append_note("work/Plan", "\nAppended line").await.unwrap();
        let note2 = vault.read_note("work/Plan").await.unwrap();
        assert!(note2.body.contains("Appended line"));

        // set_property
        vault.set_property("work/Plan", "status", serde_json::json!("done")).await.unwrap();
        let note3 = vault.read_note("work/Plan").await.unwrap();
        assert_eq!(note3.frontmatter["status"], "done");
    }
}
