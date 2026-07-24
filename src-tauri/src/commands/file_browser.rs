// SPDX-License-Identifier: AGPL-3.0-only

//! 文件浏览器命令
//!
//! 提供文件系统浏览/重命名/移动/新建/删除能力。所有命令均对输入路径做安全校验：
//! - 拒绝包含 `..` 的输入路径（防路径遍历）；
//! - `new_name` 不允许包含路径分隔符（`/` 或 `\`）或 `..`；
//! - 路径规范化使用 `Path::canonicalize`，不存在时回退到词法形式。
//!
//! 错误以 `ErrorResponse`（JSON 序列化为 String）返回，前端按 `error.${code}` 走 i18n。

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::commands::error::{ErrorCategory, ErrorResponse};
use crate::commands::error_code::common as common_err;
use crate::commands::error_code::file as file_err;
use crate::commands::error_code::security as sec_err;
use crate::commands::error_code::storage as storage_err;

/// 目录条目（与前端 DirEntry 对齐，camelCase 序列化）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<i64>,
}

/// 文件详细信息（与前端 FileInfo 对齐）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<i64>,
    pub extension: Option<String>,
}

/// 把 ErrorResponse 转成 String（用于 `Result<T, String>` 返回类型）。
fn err_to_string(e: ErrorResponse) -> String {
    e.to_string()
}

/// 校验输入路径字符串：非空且不含 `..` 段。
fn validate_path_input(path: &str) -> Result<(), ErrorResponse> {
    if path.trim().is_empty() {
        return Err(
            ErrorResponse::new(file_err::PATH_EMPTY).with_category(ErrorCategory::Validation)
        );
    }
    // 拒绝任何包含 `..` 的输入（规范化后可能越权访问父目录）
    if path.contains("..") {
        return Err(
            ErrorResponse::new(sec_err::PATH_TRAVERSAL).with_category(ErrorCategory::Validation)
        );
    }
    Ok(())
}

/// 校验新名称：非空、不含路径分隔符、不含 `..`。
fn validate_new_name(new_name: &str) -> Result<(), ErrorResponse> {
    if new_name.trim().is_empty() {
        return Err(ErrorResponse::new(common_err::INVALID_INPUT)
            .with_category(ErrorCategory::Validation)
            .with_detail("new_name is empty"));
    }
    if new_name.contains('/') || new_name.contains('\\') || new_name.contains("..") {
        return Err(ErrorResponse::new(common_err::INVALID_INPUT)
            .with_category(ErrorCategory::Validation)
            .with_detail("new_name must not contain path separators or '..'"));
    }
    Ok(())
}

/// 去掉 Windows 上 canonicalize 引入的 `\\?\` UNC 前缀，让返回给前端的路径更友好。
fn strip_unc_prefix(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy().into_owned();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{}", rest))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        p
    }
}

/// 规范化路径：canonicalize，不存在时回退到词法形式；并剥离 UNC 前缀。
fn canonicalize_path(path: &str) -> Result<PathBuf, ErrorResponse> {
    let p = Path::new(path);
    let canonical = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    Ok(strip_unc_prefix(canonical))
}

/// 把 IO 错误映射为带错误码的 ErrorResponse。
fn io_err<T, E: std::fmt::Display>(result: Result<T, E>, code: &str) -> Result<T, ErrorResponse> {
    result.map_err(|e| {
        ErrorResponse::new(code)
            .with_category(ErrorCategory::Unrecoverable)
            .with_detail(e.to_string())
    })
}

/// 读取文件元数据的修改时间，转换为 UNIX 秒（失败返回 None）。
fn modified_secs(meta: &std::fs::Metadata) -> Option<i64> {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}

/// 列出指定目录下的文件和文件夹，按名称排序（目录优先）。
#[tauri::command]
pub async fn list_directory(path: String) -> Result<Vec<DirEntry>, String> {
    validate_path_input(&path).map_err(err_to_string)?;
    let abs = canonicalize_path(&path).map_err(err_to_string)?;

    let read = match std::fs::read_dir(&abs) {
        Ok(r) => r,
        Err(e) => {
            return Err(err_to_string(
                ErrorResponse::new(storage_err::READ_DIR_FAILED)
                    .with_category(ErrorCategory::Unrecoverable)
                    .with_detail(format!("{}: {}", abs.display(), e)),
            ));
        },
    };

    let mut entries: Vec<DirEntry> = Vec::new();
    for entry in read {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                return Err(err_to_string(
                    ErrorResponse::new(storage_err::READ_DIR_FAILED)
                        .with_category(ErrorCategory::Unrecoverable)
                        .with_detail(e.to_string()),
                ));
            },
        };
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue, // 跳过无法读取元数据的条目
        };
        let name = entry.file_name().to_string_lossy().to_string();
        let entry_path = entry.path().to_string_lossy().to_string();
        entries.push(DirEntry {
            name,
            path: entry_path,
            is_dir: meta.is_dir(),
            size: if meta.is_dir() {
                None
            } else {
                Some(meta.len())
            },
            modified: modified_secs(&meta),
        });
    }

    // 排序：目录优先，再按名称不区分大小写
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(entries)
}

/// 重命名文件或文件夹（仅修改最后一段名称，不允许跨目录移动）。
#[tauri::command]
pub async fn rename_entry(old_path: String, new_name: String) -> Result<(), String> {
    validate_path_input(&old_path).map_err(err_to_string)?;
    validate_new_name(&new_name).map_err(err_to_string)?;

    let abs = canonicalize_path(&old_path).map_err(err_to_string)?;
    let parent = abs.parent().ok_or_else(|| {
        err_to_string(
            ErrorResponse::new(common_err::INVALID_INPUT)
                .with_category(ErrorCategory::Validation)
                .with_detail("path has no parent directory"),
        )
    })?;
    let new_path = parent.join(&new_name);

    if new_path.exists() {
        return Err(err_to_string(
            ErrorResponse::new(common_err::INVALID_INPUT)
                .with_category(ErrorCategory::Validation)
                .with_detail(format!("target already exists: {}", new_path.display())),
        ));
    }

    io_err(std::fs::rename(&abs, &new_path), storage_err::WRITE_FILE_FAILED)
        .map_err(err_to_string)?;
    Ok(())
}

/// 移动文件/文件夹到目标目录。
#[tauri::command]
pub async fn move_entry(src_path: String, dst_dir: String) -> Result<(), String> {
    validate_path_input(&src_path).map_err(err_to_string)?;
    validate_path_input(&dst_dir).map_err(err_to_string)?;

    let src = canonicalize_path(&src_path).map_err(err_to_string)?;
    let dst = canonicalize_path(&dst_dir).map_err(err_to_string)?;

    if !dst.is_dir() {
        return Err(err_to_string(
            ErrorResponse::new(common_err::INVALID_INPUT)
                .with_category(ErrorCategory::Validation)
                .with_detail(format!("dst_dir is not a directory: {}", dst.display())),
        ));
    }

    let file_name = src.file_name().ok_or_else(|| {
        err_to_string(
            ErrorResponse::new(common_err::INVALID_INPUT)
                .with_category(ErrorCategory::Validation)
                .with_detail("src_path has no file name component"),
        )
    })?;
    let target = dst.join(file_name);
    if target.exists() {
        return Err(err_to_string(
            ErrorResponse::new(common_err::INVALID_INPUT)
                .with_category(ErrorCategory::Validation)
                .with_detail(format!("target already exists: {}", target.display())),
        ));
    }

    io_err(std::fs::rename(&src, &target), storage_err::WRITE_FILE_FAILED)
        .map_err(err_to_string)?;
    Ok(())
}

/// 创建目录（含父目录）。
#[tauri::command]
pub async fn create_directory(path: String) -> Result<(), String> {
    validate_path_input(&path).map_err(err_to_string)?;
    let abs = canonicalize_path(&path).map_err(err_to_string)?;
    io_err(std::fs::create_dir_all(&abs), storage_err::CREATE_DIR_FAILED).map_err(err_to_string)?;
    Ok(())
}

/// 删除文件或目录（recursive=true 时递归删除目录）。
#[tauri::command]
pub async fn delete_entry(path: String, recursive: bool) -> Result<(), String> {
    validate_path_input(&path).map_err(err_to_string)?;
    let abs = canonicalize_path(&path).map_err(err_to_string)?;

    if !abs.exists() {
        return Err(err_to_string(
            ErrorResponse::new(file_err::FILE_NOT_FOUND)
                .with_category(ErrorCategory::Validation)
                .with_detail(format!("path not found: {}", abs.display())),
        ));
    }

    if abs.is_dir() {
        if recursive {
            io_err(std::fs::remove_dir_all(&abs), storage_err::WRITE_FILE_FAILED)
                .map_err(err_to_string)?;
        } else {
            io_err(std::fs::remove_dir(&abs), storage_err::WRITE_FILE_FAILED)
                .map_err(err_to_string)?;
        }
    } else {
        io_err(std::fs::remove_file(&abs), storage_err::WRITE_FILE_FAILED)
            .map_err(err_to_string)?;
    }
    Ok(())
}

/// 获取文件/目录的详细信息。
#[tauri::command]
pub async fn get_file_info(path: String) -> Result<FileInfo, String> {
    validate_path_input(&path).map_err(err_to_string)?;
    let abs = canonicalize_path(&path).map_err(err_to_string)?;

    let meta = match std::fs::metadata(&abs) {
        Ok(m) => m,
        Err(e) => {
            return Err(err_to_string(
                ErrorResponse::new(file_err::FILE_NOT_FOUND)
                    .with_category(ErrorCategory::Validation)
                    .with_detail(format!("{}: {}", abs.display(), e)),
            ));
        },
    };

    let name = abs.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let extension = abs.extension().map(|e| e.to_string_lossy().to_string());

    Ok(FileInfo {
        name,
        path: abs.to_string_lossy().to_string(),
        is_dir: meta.is_dir(),
        size: if meta.is_dir() {
            None
        } else {
            Some(meta.len())
        },
        modified: modified_secs(&meta),
        extension,
    })
}

/// 文本预览读取的最大字节数（100KB）。
const TEXT_PREVIEW_MAX_BYTES: u64 = 100 * 1024;

/// 读取文本文件内容用于预览（限制 100KB，超出截断并追加提示）。
///
/// 仅用于文本类文件预览，非 UTF-8 文件返回错误。
#[tauri::command]
pub async fn read_text_file(path: String) -> Result<String, String> {
    validate_path_input(&path).map_err(err_to_string)?;
    let abs = canonicalize_path(&path).map_err(err_to_string)?;

    let meta = std::fs::metadata(&abs).map_err(|e| {
        err_to_string(
            ErrorResponse::new(file_err::FILE_NOT_FOUND)
                .with_category(ErrorCategory::Validation)
                .with_detail(format!("{}: {}", abs.display(), e)),
        )
    })?;
    if meta.is_dir() {
        return Err(err_to_string(
            ErrorResponse::new(common_err::INVALID_INPUT)
                .with_category(ErrorCategory::Validation)
                .with_detail("path is a directory, not a file"),
        ));
    }

    let file = std::fs::File::open(&abs).map_err(|e| {
        err_to_string(
            ErrorResponse::new(storage_err::READ_FILE_FAILED)
                .with_category(ErrorCategory::Unrecoverable)
                .with_detail(format!("{}: {}", abs.display(), e)),
        )
    })?;

    // 最多读取 MAX_BYTES + 1 字节，多读 1 字节用于判断是否截断
    let mut buf = Vec::with_capacity(TEXT_PREVIEW_MAX_BYTES as usize);
    file.take(TEXT_PREVIEW_MAX_BYTES + 1).read_to_end(&mut buf).map_err(|e| {
        err_to_string(
            ErrorResponse::new(storage_err::READ_FILE_FAILED)
                .with_category(ErrorCategory::Unrecoverable)
                .with_detail(e.to_string()),
        )
    })?;

    let truncated = buf.len() as u64 > TEXT_PREVIEW_MAX_BYTES;
    if truncated {
        buf.truncate(TEXT_PREVIEW_MAX_BYTES as usize);
    }

    let mut text = String::from_utf8(buf).map_err(|e| {
        err_to_string(
            ErrorResponse::new(common_err::INVALID_INPUT)
                .with_category(ErrorCategory::Validation)
                .with_detail(format!("file is not valid UTF-8: {}", e)),
        )
    })?;
    if truncated {
        text.push_str("\n\n…（文件超过 100KB，已截断）…");
    }
    Ok(text)
}

/// 返回默认的文件浏览器根目录（documents_root）。
///
/// 供前端初始化 FileTreeView 的根路径使用。
#[tauri::command]
pub async fn get_documents_root() -> Result<String, String> {
    let root = axagent_storage::storage_paths::documents_root();
    Ok(root.to_string_lossy().to_string())
}
