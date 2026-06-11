// SPDX-License-Identifier: AGPL-3.0-only

use crate::{PermissionResult, Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

/// SECURITY (C4): 工作区边界与"危险路径"的真白名单。
const DEVICE_FILE_BLACKLIST: &[&str] = &[
    "/dev/zero",
    "/dev/random",
    "/dev/urandom",
    "/dev/full",
    "/dev/tty",
    "/dev/stdin",
    "/dev/stdout",
    "/dev/stderr",
    "/dev/kmem",
    "/dev/mem",
    "/proc/self/mem",
    "/proc/kcore",
    "/sys/",
    "/proc/",
];

/// 用户级"硬禁区"：即使在 allow_write 模式下也不允许读取。
const ALWAYS_FORBIDDEN_PREFIXES: &[&str] = &[
    "/etc/shadow",
    "/etc/sudoers",
    "/etc/sudoers.d/",
    "/root/.ssh/",
    "/home/*/.ssh/",
    "/Users/*/.ssh/",
    "/var/lib/",
];

const LARGE_FILE_THRESHOLD_MB: u64 = 50;

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "FileRead"
    }
    fn description(&self) -> &str {
        "读取文件内容。支持文本文件（可指定行范围）、图片、PDF。支持偏移量和行数限制。文件路径必须是绝对路径。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "要读取的文件绝对路径"
                },
                "offset": {
                    "type": "integer",
                    "description": "从第几行开始读取（0 表示从开头）"
                },
                "limit": {
                    "type": "integer",
                    "description": "最多读取多少行（默认 2000）"
                }
            },
            "required": ["file_path"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ToolError> {
        let path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileRead", "缺少 file_path 参数"))?;

        if !Path::new(path).is_absolute() {
            return Err(ToolError::invalid_input_for("FileRead", "file_path 必须是绝对路径"));
        }
        if path.contains('\0') {
            return Err(ToolError::invalid_input_for("FileRead", "file_path 含 NUL"));
        }

        for dev in DEVICE_FILE_BLACKLIST {
            if path.starts_with(dev) {
                return Err(ToolError::permission_denied(
                    "FileRead",
                    &format!("禁止读取设备文件: {}", dev),
                ));
            }
        }
        for bad in ALWAYS_FORBIDDEN_PREFIXES {
            // 简单 glob: 仅支持 * 通配
            if matches_glob(bad, path) {
                return Err(ToolError::permission_denied(
                    "FileRead",
                    &format!("禁止读取敏感路径: {}", bad),
                ));
            }
        }
        Ok(())
    }

    fn check_permissions(&self, input: &Value, ctx: &ToolContext) -> PermissionResult {
        let path = match input["file_path"].as_str() {
            Some(p) => p,
            None => return PermissionResult::Ask("缺少 file_path".to_string()),
        };
        let working_dir = std::path::Path::new(&ctx.working_dir);

        // 工作区外 → 询问
        if !is_within_workspace(Path::new(path), working_dir) {
            return PermissionResult::Ask(format!(
                "目标文件 {} 不在工作区 {} 内，确认读取？",
                path,
                working_dir.display()
            ));
        }

        // 设备/敏感路径 → 拒绝
        for dev in DEVICE_FILE_BLACKLIST {
            if path.starts_with(dev) {
                return PermissionResult::Deny(format!("设备文件 {} 不可读", dev));
            }
        }
        for bad in ALWAYS_FORBIDDEN_PREFIXES {
            if matches_glob(bad, path) {
                return PermissionResult::Deny(format!("敏感路径 {} 不可读", bad));
            }
        }

        // 解析符号链接后再次确认仍在工作区
        if let Ok(canonical) = std::fs::canonicalize(path)
            && !is_within_workspace(&canonical, working_dir)
        {
            return PermissionResult::Deny(format!(
                "符号链接 {} 指向工作区外 {}",
                path,
                canonical.display()
            ));
        }

        PermissionResult::Allow
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        // SECURITY: 二次强制检查，避免 check_permissions 被旁路
        match self.check_permissions(&input, ctx) {
            PermissionResult::Allow => {},
            PermissionResult::Deny(reason) => {
                return Err(ToolError::permission_denied("FileRead", &reason));
            },
            PermissionResult::Ask(_) => {
                return Err(ToolError::permission_denied(
                    "FileRead",
                    "需要用户确认 (未配置自动批准)",
                ));
            },
        }

        let file_path = input["file_path"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_input_for("FileRead", "缺少 file_path 参数"))?;
        let offset: usize = input.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let limit: usize = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(2000) as usize;

        let path = Path::new(file_path);

        if !path.exists() {
            return Err(ToolError::invalid_input(format!("文件不存在: {}", file_path)));
        }

        if !path.is_file() {
            return Err(ToolError::invalid_input(format!("不是文件: {}", file_path)));
        }

        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| ToolError::execution_failed(format!("无法获取文件信息: {}", e)))?;
        if metadata.len() > LARGE_FILE_THRESHOLD_MB * 1024 * 1024 {
            return Err(ToolError::invalid_input(format!(
                "文件过大 ({} MB)，最大允许 {} MB。请使用 offset/limit 分段读取。",
                metadata.len() / 1024 / 1024,
                LARGE_FILE_THRESHOLD_MB
            )));
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" => read_image(file_path).await,
            "pdf" => read_pdf(file_path).await,
            "ipynb" => read_notebook(file_path).await,
            _ => read_text(file_path, offset, limit).await,
        }
    }
}

/// 简易 glob 匹配：仅支持 `*`（匹配单个路径段）。
/// 模式与文本按 `/` 或 `\` 切段比较：每个段必须相等或为 `*`；
/// 模式可作为前缀匹配更深的路径（但不能跨段匹配，如 `/etc` 不应匹配 `/etcfoo`）。
fn matches_glob(pattern: &str, text: &str) -> bool {
    let pattern_trim = pattern.trim_end_matches(['/', '\\']);

    // 手动 split 而非依赖 Path::components()，确保 Linux CI 上也能正确识别 \ 为分隔符
    let p_segs: Vec<&str> = pattern_trim
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .collect();
    let t_segs: Vec<&str> = text.split(['/', '\\']).filter(|s| !s.is_empty()).collect();

    if p_segs.len() > t_segs.len() {
        return false;
    }
    for (i, p_seg) in p_segs.iter().enumerate() {
        if *p_seg == "*" {
            continue;
        }
        if *p_seg != t_segs[i] {
            return false;
        }
    }
    true
}

/// 判断 `path` 是否在 `workspace` 内（按组件比较，已 normalize）。
fn is_within_workspace(path: &Path, workspace: &Path) -> bool {
    let p = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let w = std::fs::canonicalize(workspace).unwrap_or_else(|_| workspace.to_path_buf());
    path_components_starts_with(&p, &w)
}

fn path_components_starts_with(p: &Path, prefix: &Path) -> bool {
    let p_comp: Vec<Component> = p.components().collect();
    let w_comp: Vec<Component> = prefix.components().collect();
    if p_comp.len() < w_comp.len() {
        return false;
    }
    p_comp.iter().take(w_comp.len()).eq(w_comp.iter())
}

async fn read_text(path: &str, offset: usize, limit: usize) -> Result<ToolResult, ToolError> {
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => {
            let bytes = tokio::fs::read(path)
                .await
                .map_err(|e| ToolError::execution_failed(format!("读取失败: {}", e)))?;

            String::from_utf8_lossy(&bytes).to_string()
        },
    };

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    if offset >= total_lines {
        return Ok(ToolResult::success(format!(
            "文件共 {} 行 (offset={offset} 超出范围)",
            total_lines
        )));
    }

    let end = std::cmp::min(offset + limit, total_lines);
    let selected: Vec<&str> = lines[offset..end].to_vec();

    let mut output = String::new();
    for (i, line) in selected.iter().enumerate() {
        let line_no = offset + i + 1;
        output.push_str(&format!("{:>6}\t{}\n", line_no, line));
    }

    if end < total_lines {
        output.push_str(&format!(
            "\n[显示 {offset}-{end} / {total_lines} 行，可增加 limit 或调整 offset 读取更多]"
        ));
    }

    if content.len() > 200_000 {
        return Ok(ToolResult::truncated(output, 200_000));
    }

    Ok(ToolResult::success(output))
}

async fn read_image(path: &str) -> Result<ToolResult, ToolError> {
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| ToolError::execution_failed(format!("读取图片失败: {}", e)))?;

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    Ok(ToolResult::success(format!(
        "[图片] {} ({:.1} KB)\ndata:image/{};base64,{}",
        path,
        bytes.len() as f64 / 1024.0,
        ext,
        b64
    )))
}

async fn read_pdf(path: &str) -> Result<ToolResult, ToolError> {
    let path_owned = path.to_string();
    let text = tokio::task::spawn_blocking(move || pdf_extract::extract_text(&path_owned))
        .await
        .map_err(|e| ToolError::execution_failed(format!("PDF 读取任务失败: {}", e)))?
        .map_err(|e| ToolError::execution_failed(format!("PDF 读取失败: {}", e)))?;

    if text.len() > 200_000 {
        Ok(ToolResult::truncated(text, 200_000))
    } else {
        Ok(ToolResult::success(text))
    }
}

async fn read_notebook(path: &str) -> Result<ToolResult, ToolError> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| ToolError::execution_failed(format!("读取 Notebook 失败: {}", e)))?;

    let nb: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| ToolError::execution_failed(format!("Notebook JSON 解析失败: {}", e)))?;

    let mut output = String::new();
    output.push_str(&format!("# Notebook: {}\n\n", path));

    if let Some(cells) = nb["cells"].as_array() {
        for (i, cell) in cells.iter().enumerate() {
            let cell_type = cell["cell_type"].as_str().unwrap_or("unknown");
            let source = cell["source"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default();
            // SECURITY: 输出阶段做 HTML/script 剥离
            let safe_source = strip_html(&source);

            output.push_str(&format!("## Cell {} [{}]\n{}\n\n", i, cell_type, safe_source));
        }
    }

    Ok(ToolResult::success(output))
}

/// 极简 HTML 剥离：去掉 `<...>` 段及危险属性。
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
            continue;
        }
        if c == '>' {
            in_tag = false;
            continue;
        }
        if !in_tag {
            out.push(c);
        }
    }
    out
}

#[allow(dead_code)]
fn _unused_pathbuf() -> PathBuf {
    PathBuf::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matches_star() {
        assert!(matches_glob("/home/*/.ssh/", "/home/alice/.ssh/id_rsa"));
        assert!(matches_glob("/etc/shadow", "/etc/shadow"));
        assert!(!matches_glob("/etc/shadow", "/etc/passwd"));
    }

    #[test]
    fn device_paths_denied() {
        for dev in &["/dev/zero", "/proc/self/mem"] {
            assert!(
                DEVICE_FILE_BLACKLIST.iter().any(|d| dev.starts_with(d)),
                "{} should be in device blacklist",
                dev
            );
        }
    }

    #[test]
    fn path_components_strict() {
        let p = std::path::PathBuf::from("/workspace/inside.txt");
        let w = std::path::PathBuf::from("/workspace");
        assert!(path_components_starts_with(&p, &w));

        let p2 = std::path::PathBuf::from("/workspace_evil/x");
        assert!(!path_components_starts_with(&p2, &w));
    }
}
