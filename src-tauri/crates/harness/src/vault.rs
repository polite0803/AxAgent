// SPDX-License-Identifier: AGPL-3.0-only

//! Obsidian Vault 集成契约层
//!
//! 参考 DeepTutor `deeptutor/capabilities/obsidian/` 的设计：
//! - vault 是用户已有的 Markdown 文件夹（含 `.obsidian` 配置目录）
//! - KB 条目可作为 "ConnectedVault" 类型指向该文件夹
//! - 不索引、不向量化，agent 通过 9 个 `obsidian_*` 工具直接读写 live 文件
//! - 路径安全：`_vault_path` 由服务端注入，模型不可见；写入仅限追加式
//!
//! 本模块只定义 trait + DTO，实现下沉到 `axagent-tools::tools::obsidian`。
//! consumer crate（agent / gateway）通过 `Arc<dyn VaultSource>` 注入。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// 知识库类型区分（与 DeepTutor `kb_types.py` 对齐）
///
/// - `Indexed`：默认，KB 内容存于 `data/knowledge_bases/`，走 RAG 索引
/// - `ConnectedVault`：指针型，指向用户已有的 Obsidian vault，agent 直接读写 live 文件
/// - `ConnectedLinked`：指针型，指向外部已建索引目录（保留枚举位，本期不实现）
/// - `ConnectedSubagent`：指针型，指向本地 CLI agent（保留枚举位，本期不实现）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KbKind {
    #[default]
    Indexed,
    ConnectedVault,
    ConnectedLinked,
    ConnectedSubagent,
}

impl KbKind {
    /// 是否为指针型 KB（数据存于外部，不索引、不向量化）
    pub fn is_connected(&self) -> bool {
        matches!(self, KbKind::ConnectedVault | KbKind::ConnectedLinked | KbKind::ConnectedSubagent)
    }

    /// 是否跳过 RAG 索引流水线
    pub fn skips_indexing(&self) -> bool {
        self.is_connected()
    }
}

/// Vault 操作错误
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Vault path escapes the bound root: {0}")]
    PathEscapes(String),
    #[error("Note '{0}' not found in the vault")]
    NoteNotFound(String),
    #[error("Note '{0}' already exists; use append instead")]
    NoteAlreadyExists(String),
    #[error("Invalid note reference: {0}")]
    InvalidRef(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML frontmatter parse error: {0}")]
    Yaml(String),
    #[error("Vault not bound on this turn")]
    NotBound,
}

/// 笔记引用（裸名或 vault 相对路径）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NoteRef {
    /// 原始引用字符串（如 "Project Plan" 或 "work/Plan.md"）
    pub raw: String,
}

/// Frontmatter + 正文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteContent {
    /// Vault 相对路径（如 "work/Plan.md"）
    pub path: String,
    /// YAML frontmatter 解析结果（空 dict 表示无 frontmatter）
    pub frontmatter: serde_json::Value,
    /// Markdown 正文（已剥离 frontmatter）
    pub body: String,
}

/// 搜索命中
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteHit {
    pub path: String,
    pub snippet: String,
}

/// 反向链接或正向链接命中
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkHit {
    pub path: String,
    pub snippet: String,
}

/// Tag 频次
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCount {
    pub tag: String,
    pub count: i64,
}

/// Vault 操作契约
///
/// 实现层：`axagent_tools::tools::obsidian::FsVaultSource`
/// 消费层：`obsidian_*` 工具 / agent capability / 未来 IM channels
///
/// 所有方法接收 `&self`，实现层内部用 `Arc<dyn VaultSource>` 共享。
/// 路径安全铁律：实现必须在 `safe_join` 内强制锁定在 vault 根下。
#[async_trait]
pub trait VaultSource: Send + Sync {
    /// Vault 根路径（已 canonicalize）
    fn vault_root(&self) -> &std::path::Path;

    /// 全 vault 子串搜索（标题 + 正文，大小写不敏感）
    async fn search_notes(&self, query: &str, limit: usize) -> Result<Vec<NoteHit>, VaultError>;

    /// 读取单条笔记（frontmatter + body）
    async fn read_note(&self, note: &str) -> Result<NoteContent, VaultError>;

    /// 列出 vault 或子文件夹下的笔记路径
    async fn list_notes(
        &self,
        folder: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, VaultError>;

    /// 反向链接：谁通过 `[[note]]` 链到了它
    async fn backlinks(&self, note: &str, limit: usize) -> Result<Vec<LinkHit>, VaultError>;

    /// 正向链接：它链到了谁
    async fn outgoing_links(&self, note: &str) -> Result<Vec<String>, VaultError>;

    /// 全 vault tag 频次统计（inline `#tag` + frontmatter `tags`）
    async fn collect_tags(&self, limit: usize) -> Result<Vec<TagCount>, VaultError>;

    /// 创建新笔记（拒绝覆盖）
    async fn create_note(
        &self,
        path: &str,
        content: &str,
        frontmatter: Option<serde_json::Value>,
    ) -> Result<String, VaultError>;

    /// 追加到末尾（不改正文）
    async fn append_note(&self, note: &str, content: &str) -> Result<String, VaultError>;

    /// 设置 frontmatter 单个 key（不动正文）
    async fn set_property(
        &self,
        note: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<String, VaultError>;
}

/// 已绑定的 vault（用于服务端注入到工具调用）
#[derive(Debug, Clone)]
pub struct VaultBinding {
    pub name: String,
    pub root: PathBuf,
}

/// 9 个 obsidian_* 工具名（与 DeepTutor 对齐）
pub const OBSIDIAN_TOOL_NAMES: &[&str] = &[
    "obsidian_search",
    "obsidian_read",
    "obsidian_list",
    "obsidian_backlinks",
    "obsidian_links",
    "obsidian_tags",
    "obsidian_create_note",
    "obsidian_append",
    "obsidian_set_property",
];
