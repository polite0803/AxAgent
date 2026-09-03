// SPDX-License-Identifier: AGPL-3.0-only

#![allow(dead_code)]

//! ApplyPatchTool — 原子性多文件补丁应用工具。
//!
//! 接受 git-style unified diff 作为输入，解析后对涉及文件做快照、
//! 逐文件应用 hunks，任一 hunk 匹配失败则全部文件回滚到补丁前状态。
//!
//! 不依赖外部 git/patch 命令，纯 Rust 实现行级 diff 解析 + 应用。

use crate::{Tool, ToolCategory, ToolContext, ToolDomain, ToolError, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

// ── Unified Diff 数据结构 ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct FilePatch {
    /// 目标文件相对路径（去掉 a//b/ 前缀）
    path: String,
    old_path: String,
    is_new: bool,
    is_deleted: bool,
    hunks: Vec<Hunk>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Hunk {
    old_start: usize,
    old_count: usize,
    /// 目标侧起始行号（当前解析后未直接消费，保留供未来校验）
    new_start: usize,
    /// 目标侧行数（当前解析后未直接消费，保留供未来校验）
    new_count: usize,
    lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum DiffLine {
    Context(String),
    Old(String),
    New(String),
}

// ── Diff 解析 ──────────────────────────────────────────────────────────────

/// 解析 unified diff 文本为 FilePatch 列表。
fn parse_unified_diff(diff_text: &str) -> Result<Vec<FilePatch>, String> {
    let mut patches = Vec::new();
    let lines: Vec<&str> = diff_text.lines().collect();
    let mut idx = 0usize;

    while idx < lines.len() {
        // 找 `--- a/xxx`
        let Some(marker) = lines.get(idx) else { break };
        if !marker.starts_with("--- ") {
            idx += 1;
            continue;
        }

        let old_path_raw = marker.trim_start_matches("--- ").to_string();
        idx += 1;

        let Some(new_marker) = lines.get(idx) else {
            return Err("unexpected EOF after --- header".to_string());
        };
        if !new_marker.starts_with("+++ ") {
            return Err(format!("expected +++ header at line {}, got: {new_marker}", idx + 1));
        }
        let new_path_raw = new_marker.trim_start_matches("+++ ").to_string();
        idx += 1;

        let old_path = strip_ab_prefix(&old_path_raw);
        let new_path = strip_ab_prefix(&new_path_raw);
        let is_new = old_path == "/dev/null";
        let is_deleted = new_path == "/dev/null";
        let target_path = if is_deleted {
            old_path.clone()
        } else {
            new_path.clone()
        };

        let mut hunks = Vec::new();
        while idx < lines.len() && lines[idx].starts_with("@@") {
            let hunk = parse_hunk(&lines, &mut idx)?;
            hunks.push(hunk);
        }

        if hunks.is_empty() {
            return Err(format!("no hunks found for file {target_path}"));
        }

        patches.push(FilePatch { path: target_path, old_path, is_new, is_deleted, hunks });
    }

    if patches.is_empty() {
        return Err("no file patches found in diff".to_string());
    }

    Ok(patches)
}

fn strip_ab_prefix(raw: &str) -> String {
    if raw == "/dev/null" {
        return raw.to_string();
    }
    if let Some(s) = raw.strip_prefix("a/") {
        return s.to_string();
    }
    if let Some(s) = raw.strip_prefix("b/") {
        return s.to_string();
    }
    raw.to_string()
}

fn parse_hunk(lines: &[&str], i: &mut usize) -> Result<Hunk, String> {
    let header = lines[*i];
    let rest = header.trim_start_matches("@@").trim_end_matches("@@").trim();
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(format!("malformed hunk header: {header}"));
    }

    let (old_start, old_count) = parse_range(parts[0].trim_start_matches('-'));
    let (new_start, new_count) = parse_range(parts[1].trim_start_matches('+'));

    *i += 1;
    let mut diff_lines = Vec::new();

    while *i < lines.len() {
        let line = lines[*i];
        if line.starts_with("@@") || line.starts_with("--- ") {
            break;
        }
        let kind = match line.chars().next() {
            Some(' ') => DiffLine::Context(line[1..].to_string()),
            Some('-') => DiffLine::Old(line[1..].to_string()),
            Some('+') => DiffLine::New(line[1..].to_string()),
            Some('\0') | None => break,
            _ => {
                return Err(format!(
                    "unexpected hunk line prefix at position {}: {line:?}",
                    *i + 1
                ));
            },
        };
        diff_lines.push(kind);
        *i += 1;
    }

    Ok(Hunk { old_start, old_count, new_start, new_count, lines: diff_lines })
}

fn parse_range(spec: &str) -> (usize, usize) {
    if let Some((start, count)) = spec.split_once(',') {
        (start.parse().unwrap_or(1), count.parse().unwrap_or(1))
    } else {
        (spec.parse().unwrap_or(1), 1)
    }
}

// ── Hunk 应用 ──────────────────────────────────────────────────────────────

fn apply_hunks(original: &str, hunks: &[Hunk]) -> Result<String, String> {
    let old_lines: Vec<&str> = original.lines().collect();
    let mut result: Vec<String> = Vec::new();
    let mut cursor = 0usize;

    for hunk in hunks {
        let old_start = hunk.old_start.saturating_sub(1);

        // 推进 cursor 到 hunk 的起始位置（复制之间的上下文）
        while cursor < old_lines.len() && cursor < old_start {
            result.push(old_lines[cursor].to_string());
            cursor += 1;
        }

        // 应用 hunk 内的 diff 行
        for line in &hunk.lines {
            match line {
                DiffLine::Context(content) => {
                    // 验证上下文匹配
                    if cursor >= old_lines.len() {
                        return Err(format!(
                            "context mismatch at position {cursor}: expected {content:?} (EOF reached)"
                        ));
                    }
                    if old_lines[cursor] != content {
                        return Err(format!(
                            "context mismatch at position {cursor}: expected {content:?}, found {:?}",
                            old_lines[cursor]
                        ));
                    }
                    result.push(content.clone());
                    cursor += 1;
                },
                DiffLine::Old(content) => {
                    // 验证被删除的行匹配
                    if cursor >= old_lines.len() {
                        return Err(format!(
                            "deletion mismatch at position {cursor}: expected {content:?} (EOF reached)"
                        ));
                    }
                    if old_lines[cursor] != content {
                        return Err(format!(
                            "deletion mismatch at position {cursor}: expected {content:?}, found {:?}",
                            old_lines[cursor]
                        ));
                    }
                    cursor += 1; // Old 行被跳过，不推入 result
                },
                DiffLine::New(content) => {
                    // 新增行：直接推入 result，不推进 cursor
                    result.push(content.clone());
                },
            }
        }
    }

    // 复制剩余行
    while cursor < old_lines.len() {
        result.push(old_lines[cursor].to_string());
        cursor += 1;
    }

    Ok(result.join("\n"))
}

// ── ApplyPatchTool ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApplyPatchInput {
    patch: String,
    #[serde(default)]
    cwd: Option<String>,
}

pub struct ApplyPatchTool;

#[async_trait]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        "ApplyPatch"
    }

    fn description(&self) -> &str {
        "原子性地应用 git-style unified diff 到多个文件。任一文件 hunk 匹配失败则全部回滚，保证不会出现半应用状态。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "patch": {
                    "type": "string",
                    "description": "git-style unified diff 文本（含 --- / +++ / @@ 标记）"
                },
                "cwd": {
                    "type": "string",
                    "description": "可选：补丁内相对路径的解析基准目录"
                }
            },
            "required": ["patch"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::FileWrite
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let parsed: ApplyPatchInput = serde_json::from_value(input)
            .map_err(|e| ToolError::invalid_input(format!("deserialize error: {e}")))?;

        let patches = parse_unified_diff(&parsed.patch)
            .map_err(|e| ToolError::invalid_input(format!("diff parse error: {e}")))?;

        let cwd = parsed
            .cwd
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // 阶段 1：解析 + 验证 + 快照 + 计算目标内容
        // 用索引关联 snapshots 和 file_ops
        let mut snapshots: Vec<(String, String)> = Vec::new(); // (relative_path, original_content)
        let mut file_ops: Vec<(PathBuf, Option<String>)> = Vec::new(); // (abs_path, Some(new_content) | None=delete)

        for fp in &patches {
            let abs_path = cwd.join(&fp.path);

            if fp.is_deleted {
                let original = match fs::read_to_string(&abs_path) {
                    Ok(c) => c,
                    Err(e) => {
                        return Err(ToolError::new(format!(
                            "patch refers to delete {}, but read failed: {e}",
                            abs_path.display()
                        )));
                    },
                };
                snapshots.push((fp.path.clone(), original));
                file_ops.push((abs_path, None));
                continue;
            }

            let original = if fp.is_new {
                if abs_path.exists() {
                    return Err(ToolError::new(format!(
                        "patch marks {} as new, but file already exists",
                        abs_path.display()
                    )));
                }
                String::new()
            } else {
                match fs::read_to_string(&abs_path) {
                    Ok(c) => c,
                    Err(e) => {
                        return Err(ToolError::new(format!(
                            "cannot read {}: {e}",
                            abs_path.display()
                        )));
                    },
                }
            };

            let new_content = apply_hunks(&original, &fp.hunks).map_err(|e| {
                ToolError::new(format!("hunk application failed for {}: {e}", fp.path))
            })?;

            snapshots.push((fp.path.clone(), original));
            file_ops.push((abs_path, Some(new_content)));
        }

        // 阶段 2：实际写入
        let mut applied: Vec<String> = Vec::new();
        let mut failed_idx: Option<usize> = None;

        for (idx, (path, op)) in file_ops.iter().enumerate() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent).ok();
            }

            let result = match op {
                None => fs::remove_file(path),
                Some(content) => fs::write(path, content),
            };

            if result.is_err() {
                failed_idx = Some(idx);
                break;
            }
            applied.push(path.to_string_lossy().to_string());
        }

        // 阶段 3：失败回滚
        if let Some(_idx) = failed_idx {
            let mut rollback_count = 0usize;
            for (i, (path, _op)) in file_ops.iter().enumerate() {
                let (_rel, snapshot) = &snapshots[i];
                if path.exists() {
                    let _ = fs::write(path, snapshot);
                } else {
                    // 文件不存在：恢复（如果原来存在的话）
                    if !snapshot.is_empty() {
                        let _ = fs::write(path, snapshot);
                    }
                }
                rollback_count += 1;
            }
            return Ok(ToolResult::error(format!(
                "patch apply failed at index {}, rolled back {rollback_count} files",
                failed_idx.unwrap_or(0)
            )));
        }

        let result_json = serde_json::json!({
            "applied": applied,
            "count": applied.len(),
        });

        Ok(ToolResult::success(serde_json::to_string(&result_json).unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_diff() {
        let diff = "--- a/src/foo.rs\n+++ b/src/foo.rs\n@@ -1,4 +1,4 @@\n pub fn old() {}\n-pub fn bad() {}\n+pub fn good() {}\n pub fn ugly() {}\n";
        let patches = parse_unified_diff(diff).unwrap();
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].path, "src/foo.rs");
        assert_eq!(patches[0].hunks.len(), 1);
    }

    #[test]
    fn test_apply_single_hunk() {
        let original = "pub fn old() {}\npub fn bad() {}\npub fn ugly() {}\n";
        let diff = "--- a/foo\n+++ b/foo\n@@ -1,3 +1,3 @@\n pub fn old() {}\n-pub fn bad() {}\n+pub fn good() {}\n pub fn ugly() {}\n";
        let patches = parse_unified_diff(diff).unwrap();
        let result = apply_hunks(original, &patches[0].hunks).unwrap();
        assert!(result.contains("pub fn good()"));
        assert!(!result.contains("pub fn bad()"));
        assert!(result.contains("pub fn old()"));
        assert!(result.contains("pub fn ugly()"));
    }
}
