// SPDX-License-Identifier: AGPL-3.0-only

//! 3.1 P2:投机执行影子文件系统(CoW 覆盖文件系统轻量方案)
//!
//! 等待用户确认时,后台投机执行工具调用,写入 `.axagent/shadow/{session_id}/`
//! 影子目录。用户确认后 diff 应用到真实目录;用户拒绝时删除影子目录回滚。
//!
//! ## 方案对比
//! - FUSE/驱动级 CoW:系统级支持,Windows 实现成本高
//! - 影子目录 + diff 应用(本方案):跨平台兼容,无需系统级支持
//!
//! ## 使用流程
//! 1. `execute()` 进入 `WaitingForConfirmation` 时,创建 `ShadowFs` 实例
//! 2. 后台投机执行工具调用,通过 `write_shadow_file` / `delete_shadow_file` 写入影子目录
//! 3. 用户确认后调用 `compute_diff` + `apply_diff` 应用到真实目录
//! 4. 用户拒绝时调用 `rollback` 删除影子目录
//!
//! ## 安全性
//! - 相对路径验证:禁止绝对路径、`..` 路径遍历
//! - 文件大小限制:单文件 1MB
//! - 文件数限制:单会话 1000 个文件

use axagent_harness::constants::shadow as shadow_const;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 文件操作类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOp {
    /// 新建文件(真实目录中不存在)
    Create,
    /// 修改文件(真实目录中存在,内容不同)
    Modify,
    /// 删除文件(影子目录中标记删除)
    Delete,
}

/// 单个文件的 diff 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowDiff {
    /// 相对于项目根的路径(使用 `/` 分隔符)
    pub relative_path: String,
    /// 操作类型
    pub op: FileOp,
    /// 影子目录中的内容(Create/Modify 时为 Some,Delete 时为 None)
    pub shadow_content: Option<String>,
    /// 真实目录中的原内容(Create 时为 None,Modify/Delete 时为 Some)
    pub original_content: Option<String>,
}

/// 投机执行影子文件系统
///
/// 管理 `.axagent/shadow/{session_id}/` 影子目录,支持写入投机文件、
/// 计算 diff、应用 diff 到真实目录、回滚(删除影子目录)。
pub struct ShadowFs {
    /// 项目根目录(真实工作区)
    pub project_root: PathBuf,
    /// 会话 ID(用作影子目录子目录名,隔离不同会话)
    pub session_id: String,
}

impl ShadowFs {
    /// 创建新的影子文件系统实例
    pub fn new(project_root: impl Into<PathBuf>, session_id: impl Into<String>) -> Self {
        Self { project_root: project_root.into(), session_id: session_id.into() }
    }

    /// 影子目录绝对路径(`.axagent/shadow/{session_id}`)
    pub fn shadow_dir(&self) -> PathBuf {
        self.project_root.join(shadow_const::SHADOW_DIR).join(&self.session_id)
    }

    /// 创建影子目录(如果不存在)
    pub async fn ensure_shadow_dir(&self) -> Result<(), String> {
        let dir = self.shadow_dir();
        tokio::fs::create_dir_all(&dir).await.map_err(|e| e.to_string())
    }

    /// 在影子目录中写入文件(投机执行)
    ///
    /// `relative_path` 相对于项目根,会映射到
    /// `.axagent/shadow/{session_id}/{relative_path}`
    pub async fn write_shadow_file(
        &self,
        relative_path: &str,
        content: &str,
    ) -> Result<PathBuf, String> {
        validate_relative_path(relative_path)?;
        if content.len() > shadow_const::DIFF_FILE_SIZE_LIMIT {
            return Err(format!(
                "shadow file content exceeds {}KB size limit ({} bytes)",
                shadow_const::DIFF_FILE_SIZE_LIMIT / 1024,
                content.len()
            ));
        }
        self.ensure_shadow_dir().await?;
        let shadow_path = self.shadow_dir().join(relative_path);
        if let Some(parent) = shadow_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
        }
        tokio::fs::write(&shadow_path, content).await.map_err(|e| e.to_string())?;
        Ok(shadow_path)
    }

    /// 在影子目录中标记删除文件(投机执行)
    ///
    /// 在影子目录中创建一个 `.deleted` 标记文件,表示该文件应被删除。
    /// `compute_diff` 时会识别此标记并生成 `FileOp::Delete` diff。
    pub async fn delete_shadow_file(&self, relative_path: &str) -> Result<(), String> {
        validate_relative_path(relative_path)?;
        self.ensure_shadow_dir().await?;
        // 创建 .deleted 标记文件
        let marker_path = self.shadow_dir().join(format!("{}.deleted", relative_path));
        if let Some(parent) = marker_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
        }
        tokio::fs::write(&marker_path, "").await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 计算影子目录与真实目录的 diff
    ///
    /// 遍历影子目录下所有文件,与真实目录对比生成 diff 列表。
    /// `.deleted` 标记文件会识别为 `FileOp::Delete`。
    pub async fn compute_diff(&self) -> Result<Vec<ShadowDiff>, String> {
        let mut diffs = Vec::new();
        let shadow_dir = self.shadow_dir();
        if !shadow_dir.exists() {
            return Ok(diffs);
        }
        // 遍历影子目录,收集所有文件
        let mut shadow_files: Vec<PathBuf> = Vec::new();
        collect_files_recursive(&shadow_dir, &mut shadow_files)?;
        // 文件数限制检查
        if shadow_files.len() > shadow_const::SHADOW_MAX_FILES {
            return Err(format!(
                "shadow directory contains {} files, exceeds limit {}",
                shadow_files.len(),
                shadow_const::SHADOW_MAX_FILES
            ));
        }
        for shadow_path in shadow_files {
            let relative = shadow_path.strip_prefix(&shadow_dir).map_err(|e| e.to_string())?;
            let relative_path = relative.to_string_lossy().replace('\\', "/");
            // 处理 .deleted 标记文件
            if let Some(base_path) = relative_path.strip_suffix(".deleted") {
                let real_path = self.project_root.join(base_path);
                let original_content = if real_path.exists() {
                    tokio::fs::read_to_string(&real_path).await.ok()
                } else {
                    None
                };
                // 真实目录中不存在则无需删除
                if original_content.is_some() {
                    diffs.push(ShadowDiff {
                        relative_path: base_path.to_string(),
                        op: FileOp::Delete,
                        shadow_content: None,
                        original_content,
                    });
                }
                continue;
            }
            // 普通文件:对比内容
            let shadow_content = tokio::fs::read_to_string(&shadow_path).await.ok();
            let real_path = self.project_root.join(&relative_path);
            let original_content = if real_path.exists() {
                tokio::fs::read_to_string(&real_path).await.ok()
            } else {
                None
            };
            let op = match (&shadow_content, &original_content) {
                (Some(_), None) => FileOp::Create,
                (Some(new), Some(old)) if new != old => FileOp::Modify,
                (Some(_), Some(_)) => continue, // 内容相同,无 diff
                (None, _) => continue,          // 影子目录中无内容,跳过
            };
            diffs.push(ShadowDiff { relative_path, op, shadow_content, original_content });
        }
        Ok(diffs)
    }

    /// 应用 diff 到真实目录(用户确认后调用)
    ///
    /// 返回成功应用的文件数。
    pub async fn apply_diff(&self, diffs: &[ShadowDiff]) -> Result<usize, String> {
        let mut applied = 0;
        for diff in diffs {
            let real_path = self.project_root.join(&diff.relative_path);
            match diff.op {
                FileOp::Create | FileOp::Modify => {
                    if let Some(content) = &diff.shadow_content {
                        if let Some(parent) = real_path.parent() {
                            tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
                        }
                        tokio::fs::write(&real_path, content).await.map_err(|e| e.to_string())?;
                        applied += 1;
                    }
                },
                FileOp::Delete => {
                    if real_path.exists() {
                        tokio::fs::remove_file(&real_path).await.map_err(|e| e.to_string())?;
                        applied += 1;
                    }
                },
            }
        }
        Ok(applied)
    }

    /// 回滚:删除影子目录(用户拒绝时调用)
    pub async fn rollback(&self) -> Result<(), String> {
        let dir = self.shadow_dir();
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// 清理:应用 diff 后删除影子目录(成功提交后调用)
    pub async fn cleanup(&self) -> Result<(), String> {
        // 与 rollback 相同:删除影子目录
        self.rollback().await
    }
}

/// 验证相对路径安全性(防止路径遍历攻击)
fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty() || path.starts_with('/') || path.starts_with('\\') {
        return Err(format!("invalid relative path: {}", path));
    }
    // 检查 .. 段
    for segment in path.split(['/', '\\']) {
        if segment == ".." {
            return Err(format!("path traversal not allowed: {}", path));
        }
    }
    // Windows 绝对路径检查(如 C:\)
    #[cfg(windows)]
    {
        if path.len() >= 2
            && path.as_bytes()[1] == b':'
            && (path.as_bytes()[2] == b'\\' || path.as_bytes()[2] == b'/')
        {
            return Err(format!("absolute path not allowed: {}", path));
        }
    }
    Ok(())
}

/// 递归收集目录下所有文件
fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_op_equality() {
        assert_eq!(FileOp::Create, FileOp::Create);
        assert_eq!(FileOp::Modify, FileOp::Modify);
        assert_eq!(FileOp::Delete, FileOp::Delete);
        assert_ne!(FileOp::Create, FileOp::Modify);
        assert_ne!(FileOp::Modify, FileOp::Delete);
    }

    #[test]
    fn test_file_op_serialization() {
        let op = FileOp::Create;
        let json = serde_json::to_string(&op).unwrap();
        let deserialized: FileOp = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, FileOp::Create);
    }

    #[test]
    fn test_shadow_diff_serialization() {
        let diff = ShadowDiff {
            relative_path: "src/main.rs".to_string(),
            op: FileOp::Modify,
            shadow_content: Some("new content".to_string()),
            original_content: Some("old content".to_string()),
        };
        let json = serde_json::to_string(&diff).unwrap();
        let deserialized: ShadowDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.relative_path, "src/main.rs");
        assert_eq!(deserialized.op, FileOp::Modify);
        assert_eq!(deserialized.shadow_content, Some("new content".to_string()));
        assert_eq!(deserialized.original_content, Some("old content".to_string()));
    }

    #[test]
    fn test_validate_relative_path_valid() {
        assert!(validate_relative_path("src/main.rs").is_ok());
        assert!(validate_relative_path("src/sub/file.rs").is_ok());
        assert!(validate_relative_path("README.md").is_ok());
    }

    #[test]
    fn test_validate_relative_path_empty() {
        assert!(validate_relative_path("").is_err());
    }

    #[test]
    fn test_validate_relative_path_absolute_unix() {
        assert!(validate_relative_path("/etc/passwd").is_err());
    }

    #[test]
    fn test_validate_relative_path_absolute_windows() {
        assert!(validate_relative_path("C:\\Windows\\system32").is_err());
        assert!(validate_relative_path("C:/Windows/system32").is_err());
    }

    #[test]
    fn test_validate_relative_path_traversal() {
        assert!(validate_relative_path("../escape").is_err());
        assert!(validate_relative_path("src/../../escape").is_err());
        assert!(validate_relative_path("src/../sub/file.rs").is_ok()); // .. 在中间但无逃逸
    }

    #[test]
    fn test_validate_relative_path_backslash_start() {
        assert!(validate_relative_path("\\\\server\\share").is_err());
    }

    #[tokio::test]
    async fn test_shadow_fs_new() {
        let fs = ShadowFs::new("/project", "session-123");
        assert_eq!(fs.project_root, PathBuf::from("/project"));
        assert_eq!(fs.session_id, "session-123");
    }

    #[tokio::test]
    async fn test_shadow_fs_shadow_dir() {
        let fs = ShadowFs::new("/project", "session-123");
        assert_eq!(fs.shadow_dir(), PathBuf::from("/project/.axagent/shadow/session-123"));
    }

    #[tokio::test]
    async fn test_shadow_fs_ensure_shadow_dir() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        fs.ensure_shadow_dir().await.unwrap();
        assert!(fs.shadow_dir().exists());
    }

    #[tokio::test]
    async fn test_shadow_fs_write_shadow_file() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        let path = fs.write_shadow_file("src/main.rs", "fn main() {}").await.unwrap();
        assert!(path.exists());
        assert_eq!(path, fs.shadow_dir().join("src/main.rs"));
    }

    #[tokio::test]
    async fn test_shadow_fs_write_shadow_file_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        fs.write_shadow_file("deep/nested/path/file.rs", "content").await.unwrap();
        assert!(fs.shadow_dir().join("deep/nested/path/file.rs").exists());
    }

    #[tokio::test]
    async fn test_shadow_fs_write_shadow_file_invalid_path() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        assert!(fs.write_shadow_file("../escape", "content").await.is_err());
        assert!(fs.write_shadow_file("/absolute", "content").await.is_err());
        assert!(fs.write_shadow_file("", "content").await.is_err());
    }

    #[tokio::test]
    async fn test_shadow_fs_write_shadow_file_too_large() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        let huge_content = "a".repeat(shadow_const::DIFF_FILE_SIZE_LIMIT + 1);
        assert!(fs.write_shadow_file("big.txt", &huge_content).await.is_err());
    }

    #[tokio::test]
    async fn test_shadow_fs_delete_shadow_file() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        fs.delete_shadow_file("src/main.rs").await.unwrap();
        // 应创建 .deleted 标记文件
        assert!(fs.shadow_dir().join("src/main.rs.deleted").exists());
    }

    #[tokio::test]
    async fn test_shadow_fs_compute_diff_empty() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        let diffs = fs.compute_diff().await.unwrap();
        assert!(diffs.is_empty());
    }

    #[tokio::test]
    async fn test_shadow_fs_compute_diff_create() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        // 写入影子目录(真实目录中不存在)
        fs.write_shadow_file("new_file.rs", "new content").await.unwrap();
        let diffs = fs.compute_diff().await.unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].op, FileOp::Create);
        assert_eq!(diffs[0].relative_path, "new_file.rs");
        assert_eq!(diffs[0].shadow_content, Some("new content".to_string()));
        assert_eq!(diffs[0].original_content, None);
    }

    #[tokio::test]
    async fn test_shadow_fs_compute_diff_modify() {
        let dir = tempfile::tempdir().unwrap();
        // 在真实目录创建文件
        std::fs::write(dir.path().join("existing.rs"), "old content").unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        // 在影子目录写入修改后的内容
        fs.write_shadow_file("existing.rs", "new content").await.unwrap();
        let diffs = fs.compute_diff().await.unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].op, FileOp::Modify);
        assert_eq!(diffs[0].shadow_content, Some("new content".to_string()));
        assert_eq!(diffs[0].original_content, Some("old content".to_string()));
    }

    #[tokio::test]
    async fn test_shadow_fs_compute_diff_delete() {
        let dir = tempfile::tempdir().unwrap();
        // 在真实目录创建文件
        std::fs::write(dir.path().join("to_delete.rs"), "content").unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        // 在影子目录标记删除
        fs.delete_shadow_file("to_delete.rs").await.unwrap();
        let diffs = fs.compute_diff().await.unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].op, FileOp::Delete);
        assert_eq!(diffs[0].shadow_content, None);
        assert_eq!(diffs[0].original_content, Some("content".to_string()));
    }

    #[tokio::test]
    async fn test_shadow_fs_compute_diff_no_diff_when_same() {
        let dir = tempfile::tempdir().unwrap();
        // 在真实目录创建文件
        std::fs::write(dir.path().join("same.rs"), "same content").unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        // 在影子目录写入相同内容
        fs.write_shadow_file("same.rs", "same content").await.unwrap();
        let diffs = fs.compute_diff().await.unwrap();
        assert!(diffs.is_empty());
    }

    #[tokio::test]
    async fn test_shadow_fs_compute_diff_multiple() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("modify.rs"), "old").unwrap();
        std::fs::write(dir.path().join("delete.rs"), "to delete").unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        fs.write_shadow_file("create.rs", "new").await.unwrap();
        fs.write_shadow_file("modify.rs", "new").await.unwrap();
        fs.delete_shadow_file("delete.rs").await.unwrap();
        let diffs = fs.compute_diff().await.unwrap();
        assert_eq!(diffs.len(), 3);
    }

    #[tokio::test]
    async fn test_shadow_fs_apply_diff_create() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        let diffs = vec![ShadowDiff {
            relative_path: "new_file.rs".to_string(),
            op: FileOp::Create,
            shadow_content: Some("new content".to_string()),
            original_content: None,
        }];
        let applied = fs.apply_diff(&diffs).await.unwrap();
        assert_eq!(applied, 1);
        assert!(dir.path().join("new_file.rs").exists());
        assert_eq!(std::fs::read_to_string(dir.path().join("new_file.rs")).unwrap(), "new content");
    }

    #[tokio::test]
    async fn test_shadow_fs_apply_diff_modify() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("file.rs"), "old content").unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        let diffs = vec![ShadowDiff {
            relative_path: "file.rs".to_string(),
            op: FileOp::Modify,
            shadow_content: Some("new content".to_string()),
            original_content: Some("old content".to_string()),
        }];
        let applied = fs.apply_diff(&diffs).await.unwrap();
        assert_eq!(applied, 1);
        assert_eq!(std::fs::read_to_string(dir.path().join("file.rs")).unwrap(), "new content");
    }

    #[tokio::test]
    async fn test_shadow_fs_apply_diff_delete() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("file.rs"), "content").unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        let diffs = vec![ShadowDiff {
            relative_path: "file.rs".to_string(),
            op: FileOp::Delete,
            shadow_content: None,
            original_content: Some("content".to_string()),
        }];
        let applied = fs.apply_diff(&diffs).await.unwrap();
        assert_eq!(applied, 1);
        assert!(!dir.path().join("file.rs").exists());
    }

    #[tokio::test]
    async fn test_shadow_fs_apply_diff_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        let diffs = vec![ShadowDiff {
            relative_path: "deep/nested/file.rs".to_string(),
            op: FileOp::Create,
            shadow_content: Some("content".to_string()),
            original_content: None,
        }];
        fs.apply_diff(&diffs).await.unwrap();
        assert!(dir.path().join("deep/nested/file.rs").exists());
    }

    #[tokio::test]
    async fn test_shadow_fs_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        fs.write_shadow_file("file.rs", "content").await.unwrap();
        assert!(fs.shadow_dir().exists());
        fs.rollback().await.unwrap();
        assert!(!fs.shadow_dir().exists());
    }

    #[tokio::test]
    async fn test_shadow_fs_rollback_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        // 不存在的影子目录回滚应成功(幂等)
        fs.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn test_shadow_fs_cleanup_after_apply() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        fs.write_shadow_file("new.rs", "content").await.unwrap();
        let diffs = fs.compute_diff().await.unwrap();
        fs.apply_diff(&diffs).await.unwrap();
        // 应用后清理影子目录
        fs.cleanup().await.unwrap();
        assert!(!fs.shadow_dir().exists());
        // 真实目录中的文件应保留
        assert!(dir.path().join("new.rs").exists());
    }

    #[tokio::test]
    async fn test_shadow_fs_full_workflow_create() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        // 1. 投机执行:写入影子目录
        fs.write_shadow_file("new_file.rs", "fn main() {}").await.unwrap();
        // 2. 计算 diff
        let diffs = fs.compute_diff().await.unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].op, FileOp::Create);
        // 3. 应用 diff
        let applied = fs.apply_diff(&diffs).await.unwrap();
        assert_eq!(applied, 1);
        // 4. 清理
        fs.cleanup().await.unwrap();
        // 验证:真实目录中存在文件,影子目录已删除
        assert!(dir.path().join("new_file.rs").exists());
        assert!(!fs.shadow_dir().exists());
    }

    #[tokio::test]
    async fn test_shadow_fs_full_workflow_reject() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        // 1. 投机执行:写入影子目录
        fs.write_shadow_file("new_file.rs", "fn main() {}").await.unwrap();
        // 2. 用户拒绝:回滚
        fs.rollback().await.unwrap();
        // 验证:真实目录中不存在文件,影子目录已删除
        assert!(!dir.path().join("new_file.rs").exists());
        assert!(!fs.shadow_dir().exists());
    }

    #[tokio::test]
    async fn test_shadow_fs_compute_diff_respects_file_limit() {
        let dir = tempfile::tempdir().unwrap();
        let fs = ShadowFs::new(dir.path(), "session-1");
        // 创建超过限制的文件数
        for i in 0..(shadow_const::SHADOW_MAX_FILES + 1) {
            fs.write_shadow_file(&format!("file_{}.rs", i), "content").await.unwrap();
        }
        let result = fs.compute_diff().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_shadow_fs_multiple_sessions_isolated() {
        let dir = tempfile::tempdir().unwrap();
        let fs1 = ShadowFs::new(dir.path(), "session-1");
        let fs2 = ShadowFs::new(dir.path(), "session-2");
        // 不同会话写入不同文件
        fs1.write_shadow_file("file1.rs", "content1").await.unwrap();
        fs2.write_shadow_file("file2.rs", "content2").await.unwrap();
        // 验证影子目录隔离
        assert!(fs1.shadow_dir().join("file1.rs").exists());
        assert!(fs2.shadow_dir().join("file2.rs").exists());
        assert!(!fs1.shadow_dir().join("file2.rs").exists());
        assert!(!fs2.shadow_dir().join("file1.rs").exists());
        // 各自计算 diff 互不影响
        let diffs1 = fs1.compute_diff().await.unwrap();
        let diffs2 = fs2.compute_diff().await.unwrap();
        assert_eq!(diffs1.len(), 1);
        assert_eq!(diffs2.len(), 1);
        assert_eq!(diffs1[0].relative_path, "file1.rs");
        assert_eq!(diffs2[0].relative_path, "file2.rs");
    }
}
