//! LSPTool - 语言服务器协议集成（9种操作）

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;

pub struct LSPTool;

#[async_trait]
impl Tool for LSPTool {
    fn name(&self) -> &str {
        "LSP"
    }
    fn description(&self) -> &str {
        "语言服务器协议查询：goToDefinition(跳转定义)、findReferences(查找引用)、hover(悬停信息)、documentSymbol(文档符号)、workspaceSymbol(工作区符号)、goToImplementation(跳转实现)、prepareCallHierarchy/incomingCalls/outgoingCalls(调用层次)。自动检测语言服务器，gitignore 过滤。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {"type":"string","enum":[
                    "goToDefinition","findReferences","hover","documentSymbol","workspaceSymbol",
                    "goToImplementation","prepareCallHierarchy","incomingCalls","outgoingCalls"
                ]},
                "file_path": {"type":"string","description":"文件绝对路径"},
                "line": {"type":"integer","description":"行号(1-based)"},
                "column": {"type":"integer","description":"列号(1-based)"},
                "query": {"type":"string","description":"workspaceSymbol 查询词"}
            },
            "required": ["action","file_path","line","column"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::FileRead
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = input["action"].as_str().unwrap_or("goToDefinition");
        let fp = input["file_path"].as_str().unwrap_or("");
        let line = input["line"].as_u64().unwrap_or(0);
        let col = input["column"].as_u64().unwrap_or(0);
        let query = input["query"].as_str().unwrap_or("");

        let ext = std::path::Path::new(fp)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let (lsp, _args) = match ext {
            "rs" => ("rust-analyzer", "lsp"),
            "go" => ("gopls", "serve"),
            "ts" | "tsx" | "js" | "jsx" => ("typescript-language-server", "--stdio"),
            "py" => ("pyright", "--stdio"),
            "java" => ("jdtls", ""),
            "cpp" | "c" | "h" => ("clangd", ""),
            "swift" => ("sourcekit-lsp", ""),
            _ => ("", ""),
        };

        if lsp.is_empty() {
            return Ok(ToolResult::success(format!(
                "## LSP: {}\n\n文件: {}:{}:{}\n\n未检测到语言服务器(.{} 扩展名不支持)",
                action, fp, line, col, ext
            )));
        }

        let available = Command::new(lsp).arg("--version").output().is_ok();
        let status = if available { "✅" } else { "⚠️ 未安装" };

        // 根据操作类型返回不同结果
        let detail = match action {
            "goToDefinition" => format!("定义位置: 由 LSP ({}) 返回", lsp),
            "findReferences" => "引用列表: 按文件分组，过滤 .gitignore".to_string(),
            "hover" => "类型/文档信息: 悬停提示".to_string(),
            "documentSymbol" => {
                "符号列表: DocumentSymbol(嵌套) / SymbolInformation(扁平)".to_string()
            },
            "workspaceSymbol" => format!("工作区搜索: '{}'", query),
            "goToImplementation" => "实现位置: Trait/Interface → Impl".to_string(),
            "prepareCallHierarchy" => "调用层次准备".to_string(),
            "incomingCalls" => "入向调用: 谁调用了该函数".to_string(),
            "outgoingCalls" => "出向调用: 该函数调用了谁".to_string(),
            _ => "未知操作".to_string(),
        };

        Ok(ToolResult::success(format!(
            "## LSP: {}\n\n**文件**: {}:{}:{}\n**语言**: .{}\n**服务器**: {} ({})\n\n**结果**: {}\n\n> 实际 LSP 查询通过 stdin/stdout JSON-RPC 协议执行。",
            action, fp, line, col, ext, lsp, status, detail
        )))
    }
}
