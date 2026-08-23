// SPDX-License-Identifier: AGPL-3.0-only

//! 会话日志不变量 —— Model-visible means logged（P2 事件化，缺陷 #3，05 项）。
//!
//! 不变量：**模型可见的任何内容（进入 LLM 请求的消息）都必须被记录**，
//! 且记录可重建成模型所见（可回放）。运行时断言校验：每一条 model-visible
//! 内容在记录后都能从日志重建（`content_hash` 与原文指纹一致、内容非空）。
//!
//! 与遥测 / 审计（trajectory 只存工具结果、audit 只存哈希、telemetry 默认关）
//! 互补：本模块是「模型可见内容」的权威日志，支持事后审计与可回放重建。
//! 经能力注册表 `session.log.invariant` 接缝注入，外部插件可替换实现
//! （如落盘持久化），与 event.dispatch / storage / sandbox 接缝同构。

use std::any::Any;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::types::{ChatContent, ChatMessage};

/// 一条模型可见内容摘要。
///
/// 记录 `role` + 原文 + 命中的工具名 + 内容指纹。`content_hash` 用于
/// 可重建校验：`assert_replayable` 时按原文重算指纹比对，防缺失 / 篡改。
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelVisibleContent {
    pub role: String,
    pub text: String,
    pub tool_names: Vec<String>,
    pub content_hash: String,
}

impl ModelVisibleContent {
    /// 从一条 ChatRequest 消息提取模型可见内容摘要。
    pub fn from_chat_message(msg: &ChatMessage) -> Self {
        let (text, mut tool_names) = match &msg.content {
            ChatContent::Text(t) => (t.clone(), Vec::new()),
            ChatContent::Multipart(parts) => (
                parts.iter().filter_map(|p| p.text.clone()).collect::<Vec<_>>().join(" "),
                Vec::new(),
            ),
        };
        if let Some(tcs) = &msg.tool_calls {
            tool_names = tcs.iter().map(|t| t.function.name.clone()).collect();
        }
        let content_hash = fingerprint(&text);
        Self { role: msg.role.clone(), text, tool_names, content_hash }
    }
}

/// 不变量违反报告。
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvariantViolation {
    pub session_id: String,
    pub detail: String,
}

impl std::fmt::Display for InvariantViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[会话日志不变量违反] session={} {}", self.session_id, self.detail)
    }
}

impl std::error::Error for InvariantViolation {}

/// 会话日志不变量契约。
///
/// 「记录 model-visible 内容」即 logged；`assert_replayable` 校验记录可重建。
/// `+ Any` supertrait 支持经能力注册表类型擦除存取。
pub trait SessionLogInvariant: Any + Send + Sync + std::fmt::Debug {
    /// 记录一条模型可见内容（同一 session 内追加）。
    fn record_model_visible(&self, session_id: &str, content: ModelVisibleContent);

    /// 运行时断言：该 session 的所有记录均可重建（内容指纹与原文一致、非空）。
    fn assert_replayable(&self, session_id: &str) -> Result<(), InvariantViolation>;
}

/// 内容指纹（SHA256 hex）。
pub fn fingerprint(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

/// 内存会话日志 —— 默认实现，按 session 聚合记录。
#[derive(Debug, Default)]
pub struct InMemorySessionLog {
    inner: RwLock<HashMap<String, Vec<ModelVisibleContent>>>,
}

impl InMemorySessionLog {
    /// 构造空日志。
    pub fn new() -> Self {
        Self { inner: RwLock::new(HashMap::new()) }
    }

    /// 某 session 已记录条数（测试 / 检视辅助）。
    pub fn count(&self, session_id: &str) -> usize {
        self.inner.read().unwrap_or_else(|e| e.into_inner()).get(session_id).map_or(0, |v| v.len())
    }
}

impl SessionLogInvariant for InMemorySessionLog {
    fn record_model_visible(&self, session_id: &str, content: ModelVisibleContent) {
        self.inner
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .entry(session_id.to_string())
            .or_default()
            .push(content);
    }

    fn assert_replayable(&self, session_id: &str) -> Result<(), InvariantViolation> {
        let entries = self.inner.read().unwrap_or_else(|e| e.into_inner());
        let Some(list) = entries.get(session_id) else {
            return Ok(());
        };
        for (i, c) in list.iter().enumerate() {
            if c.text.is_empty() && c.tool_names.is_empty() {
                return Err(InvariantViolation {
                    session_id: session_id.to_string(),
                    detail: format!("第 {i} 条 model-visible 内容为空，无法重建模型所见"),
                });
            }
            if fingerprint(&c.text) != c.content_hash {
                return Err(InvariantViolation {
                    session_id: session_id.to_string(),
                    detail: format!("第 {i} 条 model-visible 内容指纹不匹配（内容被篡改/丢失）"),
                });
            }
        }
        Ok(())
    }
}

/// 把会话 id 消毒成安全文件名（防路径穿越）。
///
/// 仅保留字母数字与 `-`/`_`/`.`，其余替换为 `_`；进一步移除前导 `.`
/// 并把 `..` 改写为 `__`，杜绝任何 `../` 逃逸到根目录之外的可能。
fn sanitize_session_id(session_id: &str) -> io::Result<String> {
    let s = session_id.trim();
    if s.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "会话 id 为空"));
    }
    let mut out: String = s
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    while out.starts_with('.') {
        out.remove(0);
    }
    out = out.replace("..", "__");
    if out.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "会话 id 非法"));
    }
    Ok(out)
}

/// 从 JSONL 文件读回全部记录；损坏行按 I/O 错误上报（供断言发现）。
fn read_records(path: &Path) -> io::Result<Vec<ModelVisibleContent>> {
    let content = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let rec = serde_json::from_str::<ModelVisibleContent>(line).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("第 {i} 行 JSON 损坏: {e}"))
        })?;
        out.push(rec);
    }
    Ok(out)
}

/// 落盘会话日志 —— 默认实现，按 session 持久化为 JSONL 文件。
///
/// 每条 model-visible 内容追加为一行 JSON；`assert_replayable` 从磁盘读回
/// 并校验内容指纹（可重建）。与内存实现相比提供更强的持久保证：进程重启
/// 后仍可回放「模型所见」。追加写入由 `Mutex` 串行化，避免并发交错。
#[derive(Debug)]
pub struct DiskSessionLog {
    root: PathBuf,
    write_lock: Mutex<()>,
}

impl DiskSessionLog {
    /// 构造落盘日志，根目录不存在时自动创建。
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;
        Ok(Self { root, write_lock: Mutex::new(()) })
    }

    /// 根目录（检视/清理辅助）。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 某 session 日志文件的路径。
    fn file_path(&self, session_id: &str) -> io::Result<PathBuf> {
        Ok(self.root.join(format!("{}.jsonl", sanitize_session_id(session_id)?)))
    }

    /// 某 session 已记录条数（测试 / 检视辅助，读盘统计）。
    pub fn count(&self, session_id: &str) -> usize {
        self.file_path(session_id).ok().and_then(|p| read_records(&p).ok()).map_or(0, |v| v.len())
    }
}

impl SessionLogInvariant for DiskSessionLog {
    fn record_model_visible(&self, session_id: &str, content: ModelVisibleContent) {
        let path = match self.file_path(session_id) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("[session.log] 非法会话 id `{session_id}`: {e}");
                return;
            },
        };
        let line = match serde_json::to_string(&content) {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("[session.log] 序列化 model-visible 内容失败: {e}");
                return;
            },
        };
        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{line}");
        }
    }

    fn assert_replayable(&self, session_id: &str) -> Result<(), InvariantViolation> {
        let path = match self.file_path(session_id) {
            Ok(p) => p,
            Err(e) => {
                return Err(InvariantViolation {
                    session_id: session_id.to_string(),
                    detail: format!("非法会话 id: {e}"),
                });
            },
        };
        if !path.exists() {
            return Ok(());
        }
        let records = read_records(&path).map_err(|e| InvariantViolation {
            session_id: session_id.to_string(),
            detail: format!("读日志失败: {e}"),
        })?;
        for (i, c) in records.iter().enumerate() {
            if c.text.is_empty() && c.tool_names.is_empty() {
                return Err(InvariantViolation {
                    session_id: session_id.to_string(),
                    detail: format!("第 {i} 条 model-visible 内容为空，无法重建模型所见"),
                });
            }
            if fingerprint(&c.text) != c.content_hash {
                return Err(InvariantViolation {
                    session_id: session_id.to_string(),
                    detail: format!("第 {i} 条内容指纹不匹配（被篡改/丢失）"),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(text: &str) -> ChatMessage {
        ChatMessage {
            role: "user".into(),
            content: ChatContent::Text(text.into()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        }
    }

    #[test]
    fn record_and_replay_passes() {
        let log = InMemorySessionLog::new();
        log.record_model_visible("s1", ModelVisibleContent::from_chat_message(&user("你好")));
        log.record_model_visible("s1", ModelVisibleContent::from_chat_message(&user("帮我写报告")));
        assert_eq!(log.count("s1"), 2);
        assert!(log.assert_replayable("s1").is_ok());
        // 未记录的 session 视为可重建（空日志）
        assert!(log.assert_replayable("s2").is_ok());
    }

    #[test]
    fn empty_content_is_violation() {
        let log = InMemorySessionLog::new();
        log.record_model_visible("s1", ModelVisibleContent::from_chat_message(&user("")));
        let err = log.assert_replayable("s1").unwrap_err();
        assert!(err.detail.contains("为空"));
    }

    #[test]
    fn tampered_hash_is_violation() {
        let log = InMemorySessionLog::new();
        let mut c = ModelVisibleContent::from_chat_message(&user("原始内容"));
        c.text = "被篡改".into(); // 内容变了但指纹未更新
        log.record_model_visible("s1", c);
        let err = log.assert_replayable("s1").unwrap_err();
        assert!(err.detail.contains("指纹不匹配"));
    }

    #[test]
    fn from_multipart_extracts_text() {
        let msg = ChatMessage {
            role: "user".into(),
            content: ChatContent::Multipart(vec![
                crate::types::ContentPart {
                    r#type: "text".into(),
                    text: Some("图一".into()),
                    image_url: None,
                },
                crate::types::ContentPart {
                    r#type: "text".into(),
                    text: Some("图二".into()),
                    image_url: None,
                },
            ]),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        };
        let c = ModelVisibleContent::from_chat_message(&msg);
        assert_eq!(c.text, "图一 图二");
        assert!(!c.text.is_empty());
        assert_eq!(c.content_hash, fingerprint(&c.text));
    }

    // ── DiskSessionLog 落盘实现 ──

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("axagent_session_log_{tag}_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn disk_record_and_replay_passes() {
        let root = temp_root("record");
        let log = DiskSessionLog::new(&root).unwrap();
        log.record_model_visible("s1", ModelVisibleContent::from_chat_message(&user("你好")));
        log.record_model_visible("s1", ModelVisibleContent::from_chat_message(&user("帮我写报告")));
        assert_eq!(log.count("s1"), 2);
        assert!(log.root().join("s1.jsonl").exists());
        assert!(log.assert_replayable("s1").is_ok());
        // 未记录的 session（无文件）视为可重建
        assert!(log.assert_replayable("s2").is_ok());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn disk_persists_across_reopen() {
        let root = temp_root("reopen");
        {
            let log = DiskSessionLog::new(&root).unwrap();
            log.record_model_visible(
                "s1",
                ModelVisibleContent::from_chat_message(&user("持久内容")),
            );
        }
        // 重建实例（模拟进程重启），从磁盘恢复可重建
        let reopened = DiskSessionLog::new(&root).unwrap();
        assert_eq!(reopened.count("s1"), 1);
        assert!(reopened.assert_replayable("s1").is_ok());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn disk_tampered_is_violation() {
        let root = temp_root("tamper");
        let log = DiskSessionLog::new(&root).unwrap();
        log.record_model_visible("s1", ModelVisibleContent::from_chat_message(&user("原始内容")));
        // 直接改文件内容，指纹不再匹配
        std::fs::write(root.join("s1.jsonl"), "{\"role\":\"user\",\"text\":\"被篡改\",\"tool_names\":[],\"content_hash\":\"deadbeef\"}\n").unwrap();
        let err = log.assert_replayable("s1").unwrap_err();
        assert!(err.detail.contains("指纹不匹配"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn disk_sanitizes_unsafe_session_id() {
        let root = temp_root("sanitize");
        let log = DiskSessionLog::new(&root).unwrap();
        // `../` 不应逃逸到根目录之外：`../etc/passwd` 应消毒为根目录内的安全文件名
        log.record_model_visible(
            "../etc/passwd",
            ModelVisibleContent::from_chat_message(&user("x")),
        );
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&root).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            names.push(name.clone());
            // 安全文件名：无路径分隔符、无前导点（防隐藏/穿越）、以 .jsonl 结尾
            assert!(name.ends_with(".jsonl"));
            assert!(!name.contains('/') && !name.contains('\\'), "含路径分隔符 => 路径穿越");
            assert!(!name.starts_with('.'), "隐藏文件/前导点 => 非法");
        }
        // 恰好一个文件，且全部落在根目录内（read_dir 已枚举）
        assert_eq!(names.len(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
