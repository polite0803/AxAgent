// SPDX-License-Identifier: AGPL-3.0-only

//! Obsidian 集成工具
//!
//! 将 builtin_handlers 中的 obsidian_get_vaults、obsidian_list_files、
//! obsidian_read_file 迁移为 Tool trait。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

fn find_vaults(search_path: Option<&str>) -> Vec<String> {
    let home = dirs::home_dir();
    let default_search = home.as_deref().unwrap_or(Path::new("."));
    let base = search_path.map(Path::new).unwrap_or(default_search);
    let mut vaults = Vec::new();
    if let Ok(entries) = std::fs::read_dir(base) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let obsidian_dir = path.join(".obsidian");
                if obsidian_dir.exists() && obsidian_dir.is_dir() {
                    vaults.push(path.to_string_lossy().to_string());
                }
                // 递归一层
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    for sub in sub_entries.filter_map(|e| e.ok()) {
                        let sp = sub.path();
                        if sp.is_dir() && sp.join(".obsidian").exists() {
                            vaults.push(sp.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }
    vaults
}

pub struct ObsidianGetVaultsTool;

#[async_trait]
impl Tool for ObsidianGetVaultsTool {
    fn name(&self) -> &str {
        "ObsidianGetVaults"
    }
    fn description(&self) -> &str {
        "搜索并列出系统中的 Obsidian 知识库。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "search_path": { "type": "string" } } })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let search_path = input.get("search_path").and_then(|v| v.as_str());
        let vaults = find_vaults(search_path);
        if vaults.is_empty() {
            Ok(ToolResult::success("未找到 Obsidian 知识库"))
        } else {
            Ok(ToolResult::success(format!(
                "找到 {} 个 Obsidian 知识库:\n{}",
                vaults.len(),
                vaults.join("\n")
            )))
        }
    }
}

pub struct ObsidianListFilesTool;

#[async_trait]
impl Tool for ObsidianListFilesTool {
    fn name(&self) -> &str {
        "ObsidianListFiles"
    }
    fn description(&self) -> &str {
        "列出 Obsidian 知识库中的文件。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "vault_path": { "type": "string" } }, "required": ["vault_path"] })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let vault_path = input
            .get("vault_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if vault_path.is_empty() {
            return Ok(ToolResult::error("Error: vault_path 是必需的"));
        }
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(vault_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md") && !name.starts_with('.') {
                    files.push(name);
                }
            }
        }
        files.sort();
        if files.is_empty() {
            Ok(ToolResult::success("未找到 .md 文件"))
        } else {
            Ok(ToolResult::success(format!(
                "文件列表 ({}):\n{}",
                files.len(),
                files.join("\n")
            )))
        }
    }
}

pub struct ObsidianReadFileTool;

#[async_trait]
impl Tool for ObsidianReadFileTool {
    fn name(&self) -> &str {
        "ObsidianReadFile"
    }
    fn description(&self) -> &str {
        "读取 Obsidian 知识库中的文件内容。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": { "vault_path": { "type": "string" }, "file_path": { "type": "string" } }, "required": ["vault_path", "file_path"] })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let vault_path = input
            .get("vault_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let file_path = input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if vault_path.is_empty() || file_path.is_empty() {
            return Ok(ToolResult::error("Error: vault_path 和 file_path 都是必需的"));
        }
        let full_path = Path::new(vault_path).join(file_path);
        if !full_path.exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", full_path.display())));
        }
        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| ToolError::execution_failed(format!("读取文件失败: {}", e)))?;
        Ok(ToolResult::success(content))
    }
}
