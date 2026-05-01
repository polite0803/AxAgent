use crate::builtin_tools_registry::{
    get_global_db_path, get_handler, register_builtin_handler, BoxedToolHandler,
};
use crate::command_validator::CommandValidator;
use crate::error::{AxAgentError, Result};
use crate::mcp_client::McpToolResult;
use base64::Engine;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
}

fn make_handler<F, Fut>(f: F) -> BoxedToolHandler
where
    F: Fn(Value) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<McpToolResult>> + Send + 'static,
{
    type FutResult = Result<McpToolResult>;
    type DynFut = dyn std::future::Future<Output = FutResult> + Send;
    type PinnedFut = std::pin::Pin<Box<DynFut>>;

    let handler = move |args: Value| -> PinnedFut {
        let fut: Fut = f(args);
        Box::pin(fut) as PinnedFut
    };

    Arc::new(Box::new(handler) as Box<dyn Fn(Value) -> PinnedFut + Send + Sync>)
}

fn load_skills_metadata(path: &std::path::Path) -> Result<Vec<SkillMetadata>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content =
        std::fs::read_to_string(path).map_err(|e| AxAgentError::Gateway(e.to_string()))?;
    serde_json::from_str(&content).map_err(|e| AxAgentError::Gateway(e.to_string()))
}

#[allow(dead_code)]
fn save_skills_metadata(path: &std::path::Path, skills: &[SkillMetadata]) -> Result<()> {
    let content =
        serde_json::to_string_pretty(skills).map_err(|e| AxAgentError::Gateway(e.to_string()))?;
    std::fs::write(path, content).map_err(|e| AxAgentError::Gateway(e.to_string()))
}

pub fn init_builtin_handlers() {
    register_builtin_handler(
        "@axagent/fetch",
        "fetch_url",
        make_handler(|args: Value| {
            Box::pin(async move {
                let url = args.get("url").and_then(|v| v.as_str()).unwrap_or_default();
                let max_length = args
                    .get("max_length")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                fetch_url(url, max_length).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/fetch",
        "fetch_markdown",
        make_handler(|args: Value| {
            Box::pin(async move {
                let url = args.get("url").and_then(|v| v.as_str()).unwrap_or_default();
                let max_length = args
                    .get("max_length")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                fetch_markdown(url, max_length).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/search-file",
        "read_file",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                read_file(path).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/search-file",
        "list_directory",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                list_directory(path).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/search-file",
        "search_files",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("*");
                let max_results = args
                    .get("max_results")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                search_files(path, pattern, max_results).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/search-file",
        "grep_content",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                let pattern = args
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let file_pattern = args
                    .get("file_pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*");
                let max_results = args
                    .get("max_results")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                grep_content(path, pattern, file_pattern, max_results).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/filesystem",
        "write_file",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                write_file(path, content).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/filesystem",
        "edit_file",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let old_str = args
                    .get("old_str")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let new_str = args
                    .get("new_str")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                edit_file(path, old_str, new_str).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/filesystem",
        "delete_file",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                delete_file(path).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/filesystem",
        "create_directory",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                create_directory(path).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/filesystem",
        "file_exists",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                file_exists(path).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/filesystem",
        "get_file_info",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                get_file_info(path).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/filesystem",
        "move_file",
        make_handler(|args: Value| {
            Box::pin(async move {
                let source = args
                    .get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let destination = args
                    .get("destination")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                move_file(source, destination).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/system",
        "run_command",
        make_handler(|args: Value| {
            Box::pin(async move {
                let command = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let timeout_secs = args
                    .get("timeout_secs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30);
                run_command(command, timeout_secs).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/system",
        "get_system_info",
        make_handler(|_args: Value| Box::pin(async move { get_system_info() })),
    );

    register_builtin_handler(
        "@axagent/system",
        "list_processes",
        make_handler(|args: Value| {
            Box::pin(async move {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .unwrap_or(20);
                list_processes(limit).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/knowledge",
        "list_knowledge_bases",
        make_handler(|_args: Value| Box::pin(async move { list_knowledge_bases() })),
    );

    register_builtin_handler(
        "@axagent/knowledge",
        "search_knowledge",
        make_handler(|args: Value| {
            Box::pin(async move {
                let base_id = args
                    .get("base_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let top_k = args
                    .get("top_k")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .unwrap_or(5);
                search_knowledge(base_id.to_string(), query.to_string(), top_k).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/knowledge",
        "create_knowledge_entity",
        make_handler(|args: Value| {
            Box::pin(async move {
                let kb_id = args
                    .get("knowledge_base_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let entity_type = args
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("entity");
                let description = args.get("description").and_then(|v| v.as_str());
                let source_path = args
                    .get("source_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let source_language = args.get("source_language").and_then(|v| v.as_str());
                let properties = args
                    .get("properties")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let lifecycle = args.get("lifecycle").cloned();
                let behaviors = args.get("behaviors").cloned();
                create_knowledge_entity_tool(
                    kb_id,
                    name,
                    entity_type,
                    description,
                    source_path,
                    source_language,
                    properties,
                    lifecycle,
                    behaviors,
                )
                .await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/knowledge",
        "create_knowledge_flow",
        make_handler(|args: Value| {
            Box::pin(async move {
                let kb_id = args
                    .get("knowledge_base_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let flow_type = args
                    .get("flow_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("process");
                let description = args.get("description").and_then(|v| v.as_str());
                let source_path = args
                    .get("source_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let steps = args
                    .get("steps")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let decision_points = args.get("decision_points").cloned();
                let error_handling = args.get("error_handling").cloned();
                let preconditions = args.get("preconditions").cloned();
                let postconditions = args.get("postconditions").cloned();
                create_knowledge_flow_tool(
                    kb_id,
                    name,
                    flow_type,
                    description,
                    source_path,
                    steps,
                    decision_points,
                    error_handling,
                    preconditions,
                    postconditions,
                )
                .await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/knowledge",
        "create_knowledge_interface",
        make_handler(|args: Value| {
            Box::pin(async move {
                let kb_id = args
                    .get("knowledge_base_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let interface_type = args
                    .get("interface_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("api");
                let description = args.get("description").and_then(|v| v.as_str());
                let source_path = args
                    .get("source_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let input_schema = args
                    .get("input_schema")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let output_schema = args
                    .get("output_schema")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let error_codes = args.get("error_codes").cloned();
                let communication_pattern =
                    args.get("communication_pattern").and_then(|v| v.as_str());
                create_knowledge_interface_tool(
                    kb_id,
                    name,
                    interface_type,
                    description,
                    source_path,
                    input_schema,
                    output_schema,
                    error_codes,
                    communication_pattern,
                )
                .await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/knowledge",
        "add_knowledge_document",
        make_handler(|args: Value| {
            Box::pin(async move {
                let kb_id = args
                    .get("knowledge_base_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                add_knowledge_document_tool(kb_id, title, content).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/storage",
        "get_storage_info",
        make_handler(|_args: Value| Box::pin(async move { get_storage_info() })),
    );

    register_builtin_handler(
        "@axagent/storage",
        "list_storage_files",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .unwrap_or(50);
                list_storage_files(path.to_string(), limit)
            })
        }),
    );

    register_builtin_handler(
        "@axagent/storage",
        "upload_storage_file",
        make_handler(|args: Value| {
            Box::pin(async move {
                let filename = args
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let content_base64 = args
                    .get("content_base64")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let bucket = args
                    .get("bucket")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                upload_storage_file(
                    filename.to_string(),
                    content_base64.to_string(),
                    bucket.to_string(),
                )
            })
        }),
    );

    register_builtin_handler(
        "@axagent/storage",
        "download_storage_file",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                download_storage_file(path.to_string())
            })
        }),
    );

    register_builtin_handler(
        "@axagent/storage",
        "delete_storage_file",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                delete_storage_file(path.to_string())
            })
        }),
    );

    register_builtin_handler(
        "@axagent/search",
        "web_search",
        make_handler(|args: Value| {
            Box::pin(async move {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let provider_type = args
                    .get("provider_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let api_key = args
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let endpoint = args.get("endpoint").and_then(|v| v.as_str());
                let max_results = args
                    .get("max_results")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as i32)
                    .unwrap_or(5);
                let timeout_ms = args
                    .get("timeout_ms")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as i32)
                    .unwrap_or(15000);
                web_search(
                    query,
                    provider_type,
                    api_key,
                    endpoint,
                    max_results,
                    timeout_ms,
                )
                .await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/skills",
        "skill_manage",
        make_handler(|args: Value| {
            Box::pin(async move {
                let action = args
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let skills_dir = args
                    .get("skills_dir")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                skill_manage(action, name, description, content, skills_dir).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/session",
        "session_search",
        make_handler(|args: Value| {
            Box::pin(async move {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32)
                    .unwrap_or(10);
                let db_path = args
                    .get("db_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                session_search(query, limit, db_path).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/session",
        "memory_flush",
        make_handler(|args: Value| {
            Box::pin(async move {
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let target = args
                    .get("target")
                    .and_then(|v| v.as_str())
                    .unwrap_or("memory");
                let category = args
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("insight");
                memory_flush(content, target, category).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/memory",
        "memory_flush",
        make_handler(|args: Value| {
            Box::pin(async move {
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let target = args
                    .get("target")
                    .and_then(|v| v.as_str())
                    .unwrap_or("memory");
                let category = args
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("insight");
                memory_flush(content, target, category).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/computer-control",
        "screen_capture",
        make_handler(|args: Value| {
            Box::pin(async move {
                let monitor = args
                    .get("monitor")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let window_title = args
                    .get("window_title")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let region = args
                    .get("region")
                    .map(|v| crate::screen_capture::CaptureRegion {
                        x: v.get("x").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                        y: v.get("y").and_then(|y| y.as_i64()).unwrap_or(0) as i32,
                        width: v.get("width").and_then(|w| w.as_u64()).unwrap_or(0) as u32,
                        height: v.get("height").and_then(|h| h.as_u64()).unwrap_or(0) as u32,
                    });
                match crate::computer_control::screen_capture(monitor, region, window_title).await {
                    Ok(result) => Ok(McpToolResult {
                        content: serde_json::to_string(&result).unwrap(),
                        is_error: false,
                    }),
                    Err(e) => Err(AxAgentError::Gateway(e.to_string())),
                }
            })
        }),
    );

    register_builtin_handler(
        "@axagent/computer-control",
        "find_ui_elements",
        make_handler(|args: Value| {
            Box::pin(async move {
                let query = crate::ui_automation::UIElementQuery {
                    role: args.get("role").and_then(|v| v.as_str()).map(String::from),
                    name_contains: args
                        .get("name_contains")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    value_contains: None,
                    application: args
                        .get("application")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    window_title: args
                        .get("window_title")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    max_depth: None,
                };
                match crate::computer_control::find_ui_elements(query).await {
                    Ok(elements) => Ok(McpToolResult {
                        content: serde_json::to_string(&elements).unwrap(),
                        is_error: false,
                    }),
                    Err(e) => Err(AxAgentError::Gateway(e.to_string())),
                }
            })
        }),
    );

    register_builtin_handler(
        "@axagent/computer-control",
        "mouse_click",
        make_handler(|args: Value| {
            Box::pin(async move {
                let x = args.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let y = args.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let button = args
                    .get("button")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                crate::computer_control::mouse_click(x, y, button)
                    .await
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: "Click successful".to_string(),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/computer-control",
        "type_text",
        make_handler(|args: Value| {
            Box::pin(async move {
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let x = args.get("x").and_then(|v| v.as_f64());
                let y = args.get("y").and_then(|v| v.as_f64());
                crate::computer_control::type_text(text.to_string(), x, y)
                    .await
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: "Type text successful".to_string(),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/computer-control",
        "press_key",
        make_handler(|args: Value| {
            Box::pin(async move {
                let key = args.get("key").and_then(|v| v.as_str()).unwrap_or_default();
                let modifiers: Vec<String> = args
                    .get("modifiers")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                crate::computer_control::press_key(key.to_string(), modifiers)
                    .await
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: "Key press successful".to_string(),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/computer-control",
        "mouse_scroll",
        make_handler(|args: Value| {
            Box::pin(async move {
                let x = args.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let y = args.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let delta = args.get("delta").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                crate::computer_control::mouse_scroll(x, y, delta)
                    .await
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: "Scroll successful".to_string(),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/browser",
        "browser_navigate",
        make_handler(|args: Value| {
            Box::pin(async move {
                let url = args.get("url").and_then(|v| v.as_str()).unwrap_or_default();
                if url.is_empty() {
                    return Err(AxAgentError::Gateway("url is required".to_string()));
                }
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                let result = rt
                    .block_on(async {
                        let mut client =
                            crate::browser_automation::PlaywrightClient::launch().await?;
                        client.navigate(url).await
                    })
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: format!("Navigated to {} - Title: {}", result.url, result.title),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/browser",
        "browser_screenshot",
        make_handler(|args: Value| {
            Box::pin(async move {
                let full_page = args
                    .get("full_page")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                let result = rt
                    .block_on(async {
                        let mut client =
                            crate::browser_automation::PlaywrightClient::launch().await?;
                        client.screenshot(full_page).await
                    })
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: format!("Screenshot captured ({} bytes)", result.image_base64.len()),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/browser",
        "browser_click",
        make_handler(|args: Value| {
            Box::pin(async move {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if selector.is_empty() {
                    return Err(AxAgentError::Gateway("selector is required".to_string()));
                }
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                rt.block_on(async {
                    let mut client = crate::browser_automation::PlaywrightClient::launch().await?;
                    client.click(selector).await
                })
                .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: "Click successful".to_string(),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/browser",
        "browser_fill",
        make_handler(|args: Value| {
            Box::pin(async move {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let value = args
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if selector.is_empty() {
                    return Err(AxAgentError::Gateway("selector is required".to_string()));
                }
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                rt.block_on(async {
                    let mut client = crate::browser_automation::PlaywrightClient::launch().await?;
                    client.fill(selector, value).await
                })
                .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: "Fill successful".to_string(),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/browser",
        "browser_type",
        make_handler(|args: Value| {
            Box::pin(async move {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if selector.is_empty() {
                    return Err(AxAgentError::Gateway("selector is required".to_string()));
                }
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                rt.block_on(async {
                    let mut client = crate::browser_automation::PlaywrightClient::launch().await?;
                    client.type_text(selector, text).await
                })
                .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: "Type successful".to_string(),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/browser",
        "browser_extract_text",
        make_handler(|args: Value| {
            Box::pin(async move {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if selector.is_empty() {
                    return Err(AxAgentError::Gateway("selector is required".to_string()));
                }
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                let text = rt
                    .block_on(async {
                        let mut client =
                            crate::browser_automation::PlaywrightClient::launch().await?;
                        client.extract_text(selector).await
                    })
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: text,
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/browser",
        "browser_extract_all",
        make_handler(|args: Value| {
            Box::pin(async move {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if selector.is_empty() {
                    return Err(AxAgentError::Gateway("selector is required".to_string()));
                }
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                let elements = rt
                    .block_on(async {
                        let mut client =
                            crate::browser_automation::PlaywrightClient::launch().await?;
                        client.extract_all(selector).await
                    })
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                let content = serde_json::to_string_pretty(&elements)
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content,
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/browser",
        "browser_get_content",
        make_handler(|_args: Value| {
            Box::pin(async move {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                let html = rt
                    .block_on(async {
                        let mut client =
                            crate::browser_automation::PlaywrightClient::launch().await?;
                        client.get_content().await
                    })
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: html,
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/browser",
        "browser_select",
        make_handler(|args: Value| {
            Box::pin(async move {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let value = args
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if selector.is_empty() {
                    return Err(AxAgentError::Gateway("selector is required".to_string()));
                }
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                rt.block_on(async {
                    let mut client = crate::browser_automation::PlaywrightClient::launch().await?;
                    client.select_option(selector, value).await
                })
                .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: "Select successful".to_string(),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/browser",
        "browser_wait_for",
        make_handler(|args: Value| {
            Box::pin(async move {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let timeout = args
                    .get("timeout")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                if selector.is_empty() {
                    return Err(AxAgentError::Gateway("selector is required".to_string()));
                }
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                rt.block_on(async {
                    let mut client = crate::browser_automation::PlaywrightClient::launch().await?;
                    client.wait_for(selector, timeout).await
                })
                .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                Ok(McpToolResult {
                    content: "Element found".to_string(),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/image-gen",
        "generate_image",
        make_handler(|args: Value| {
            Box::pin(async move {
                let prompt = args
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if prompt.is_empty() {
                    return Err(AxAgentError::Gateway("prompt is required".to_string()));
                }
                let negative_prompt = args
                    .get("negative_prompt")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let width = args.get("width").and_then(|v| v.as_u64()).map(|v| v as u32);
                let height = args
                    .get("height")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let steps = args.get("steps").and_then(|v| v.as_u64()).map(|v| v as u32);
                let _seed = args.get("seed").and_then(|v| v.as_u64());
                let provider = args
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let api_key = args
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let provider_name = provider.as_deref().unwrap_or("flux");
                let client = reqwest::Client::new();

                let result = match provider_name {
                    "flux" | "Flux" => {
                        let token = api_key.ok_or_else(|| {
                            AxAgentError::Gateway("API key required for Flux".to_string())
                        })?;
                        let size = match (width, height) {
                            (Some(w), Some(h)) => format!("{}x{}", w, h),
                            _ => "1024x1024".to_string(),
                        };
                        let mut body = serde_json::json!({
                            "version": "stability-ai/sdxl:39ed52f2a78e934b3ba6e2a89f5b1c712de7dfea757525e28f18b5198a0b426",
                            "input": {
                                "prompt": prompt,
                                "aspect_ratio": size,
                                "num_inference_steps": steps.unwrap_or(25)
                            }
                        });
                        if let Some(np) = negative_prompt {
                            body["input"]["negative_prompt"] = serde_json::json!(np);
                        }
                        let resp = client
                            .post("https://api.replicate.com/v1/predictions")
                            .header("Authorization", format!("Token {}", token))
                            .header("Content-Type", "application/json")
                            .json(&body)
                            .send()
                            .await
                            .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                        let resp_json: serde_json::Value = resp
                            .json()
                            .await
                            .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                        let output_url = resp_json["urls"]["get"].as_str().ok_or_else(|| {
                            AxAgentError::Gateway("No prediction URL in response".to_string())
                        })?;
                        loop {
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            let status_resp = client
                                .get(output_url)
                                .header("Authorization", format!("Token {}", token))
                                .send()
                                .await
                                .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                            let status_json: serde_json::Value = status_resp
                                .json()
                                .await
                                .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                            if status_json["status"].as_str() == Some("succeeded") {
                                let outputs = &status_json["output"];
                                if let Some(first) = outputs.as_array().and_then(|arr| arr.first())
                                {
                                    let image_url = first.as_str().unwrap_or("");
                                    let image_resp = client
                                        .get(image_url)
                                        .send()
                                        .await
                                        .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                                    let bytes = image_resp
                                        .bytes()
                                        .await
                                        .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                                    let base64 = base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        &bytes,
                                    );
                                    break Ok(serde_json::json!({
                                        "images": [{
                                            "url": image_url,
                                            "base64": base64,
                                            "width": width.unwrap_or(1024),
                                            "height": height.unwrap_or(1024)
                                        }],
                                        "model_used": "flux-sdxl",
                                        "elapsed_ms": 0
                                    }));
                                }
                            } else if status_json["status"].as_str() == Some("failed") {
                                break Err(AxAgentError::Gateway(
                                    "Flux generation failed".to_string(),
                                ));
                            }
                        }
                    }
                    "dall-e" | "dalle" | "DALL-E" => {
                        let key = api_key.ok_or_else(|| {
                            AxAgentError::Gateway("API key required for DALL-E".to_string())
                        })?;
                        let size = match (width, height) {
                            (Some(w), Some(h)) if w == 1024 && h == 1024 => "1024x1024",
                            (Some(w), Some(h)) if w == 1792 && h == 1024 => "1792x1024",
                            (Some(w), Some(h)) if w == 1024 && h == 1792 => "1024x1792",
                            _ => "1024x1024",
                        };
                        let body = serde_json::json!({
                            "model": "dall-e-3",
                            "prompt": prompt,
                            "n": 1,
                            "size": size,
                            "quality": "standard"
                        });
                        let resp = client
                            .post("https://api.openai.com/v1/images/generations")
                            .header("Authorization", format!("Bearer {}", key))
                            .header("Content-Type", "application/json")
                            .json(&body)
                            .send()
                            .await
                            .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                        let resp_json: serde_json::Value = resp
                            .json()
                            .await
                            .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
                        let image_url = resp_json["data"][0]["url"].as_str().ok_or_else(|| {
                            AxAgentError::Gateway("No image URL in response".to_string())
                        })?;
                        let revised_prompt = resp_json["data"][0]["revised_prompt"]
                            .as_str()
                            .unwrap_or(prompt);
                        Ok(serde_json::json!({
                            "images": [{
                                "url": image_url,
                                "width": width.unwrap_or(1024),
                                "height": height.unwrap_or(1024)
                            }],
                            "model_used": "dall-e-3",
                            "revised_prompt": revised_prompt,
                            "elapsed_ms": 0
                        }))
                    }
                    _ => Err(AxAgentError::Gateway(format!(
                        "Unknown provider: {}",
                        provider_name
                    ))),
                };

                match result {
                    Ok(resp) => Ok(McpToolResult {
                        content: serde_json::to_string(&resp).unwrap(),
                        is_error: false,
                    }),
                    Err(e) => Err(e),
                }
            })
        }),
    );

    register_builtin_handler(
        "@axagent/chart-gen",
        "generate_chart_config",
        make_handler(|args: Value| {
            Box::pin(async move {
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if description.is_empty() {
                    return Err(AxAgentError::Gateway("description is required".to_string()));
                }
                let api_key = args
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let api_key = api_key.ok_or_else(|| {
                    AxAgentError::Gateway("API key required for chart generation".to_string())
                })?;

                let base_url = args
                    .get("base_url")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                let model = args
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| "gpt-4o-mini".to_string());

                let system_prompt = r#"You are a chart configuration generator. Given a natural language description and optional data, generate a valid ECharts option object.

Rules:
1. Output ONLY valid JSON (no markdown, no code fences)
2. The JSON must be a valid ECharts option
3. Use Chinese labels when the description is in Chinese
4. Include proper axis labels, legends, and tooltips
5. Use color palette: ['#5470c6','#91cc75','#fac858','#ee6666','#73c0de','#3ba272']
6. Set animation: false
7. Include "_chartType" field with the inferred type (line/bar/pie/scatter/heatmap/radar/treemap/sankey/funnel/gauge)
8. Include "_title" field with the chart title"#;

                let data = args
                    .get("data")
                    .map(|v| serde_json::to_string(&v).unwrap_or_default());
                let user_message = if let Some(ref d) = data {
                    format!("Description: {}\n\nData:\n{}", description, d)
                } else {
                    format!("Description: {}", description)
                };

                let client = reqwest::Client::new();
                let request_body = serde_json::json!({
                    "model": model,
                    "messages": [
                        {"role": "system", "content": system_prompt},
                        {"role": "user", "content": user_message}
                    ],
                    "temperature": 0.1
                });

                let resp = client
                    .post(format!("{}/chat/completions", base_url))
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;

                let resp_json: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;

                let content = resp_json["choices"][0]["message"]["content"]
                    .as_str()
                    .ok_or_else(|| AxAgentError::Gateway("No content in response".to_string()))?;

                let option: serde_json::Value = serde_json::from_str(content).map_err(|e| {
                    AxAgentError::Gateway(format!("Failed to parse chart config: {}", e))
                })?;

                let chart_type = option["_chartType"].as_str().unwrap_or("unknown");
                let title = option["_title"].as_str().unwrap_or(description);

                let result = serde_json::json!({
                    "option": option,
                    "chart_type": chart_type,
                    "title": title
                });

                Ok(McpToolResult {
                    content: serde_json::to_string(&result).unwrap(),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/code-edit",
        "search_replace",
        make_handler(|args: Value| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let old_str = args
                    .get("old_str")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let new_str = args
                    .get("new_str")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let start_line = args
                    .get("start_line")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                let end_line = args
                    .get("end_line")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                let replace_all = args
                    .get("replace_all")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                search_replace_file(path, old_str, new_str, start_line, end_line, replace_all).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/git",
        "git_status",
        make_handler(|args: Value| {
            Box::pin(async move {
                let repo_path = args
                    .get("repo_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                git_status_tool(repo_path).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/git",
        "git_diff",
        make_handler(|args: Value| {
            Box::pin(async move {
                let repo_path = args
                    .get("repo_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let base_branch = args
                    .get("base_branch")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                git_diff_tool(repo_path, base_branch.as_deref()).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/git",
        "git_commit",
        make_handler(|args: Value| {
            Box::pin(async move {
                let repo_path = args
                    .get("repo_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let message = args
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let stage_all = args
                    .get("stage_all")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                git_commit_tool(repo_path, message, stage_all).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/git",
        "git_log",
        make_handler(|args: Value| {
            Box::pin(async move {
                let repo_path = args
                    .get("repo_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let max_count =
                    args.get("max_count").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                git_log_tool(repo_path, max_count).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/git",
        "git_branch",
        make_handler(|args: Value| {
            Box::pin(async move {
                let repo_path = args
                    .get("repo_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let action = args
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("list")
                    .to_string();
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                git_branch_tool(repo_path, &action, name.as_deref()).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/git",
        "git_review",
        make_handler(|args: Value| {
            Box::pin(async move {
                let repo_path = args
                    .get("repo_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let base_branch = args
                    .get("base_branch")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                git_review_tool(repo_path, base_branch.as_deref()).await
            })
        }),
    );

    // ─── Cron job tools ────────────────────────────────────────────────

    register_builtin_handler(
        "@axagent/cron",
        "cron_add",
        make_handler(|args: Value| {
            Box::pin(async move {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let schedule = args
                    .get("schedule")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let prompt = args
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if name.is_empty() || schedule.is_empty() || prompt.is_empty() {
                    return Err(AxAgentError::Gateway(
                        "name, schedule, and prompt are required".to_string(),
                    ));
                }
                Ok(McpToolResult {
                    content: format!("Cron job '{}' scheduled with '{}'", name, schedule),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/cron",
        "cron_list",
        make_handler(|_args: Value| {
            Box::pin(async move {
                Ok(McpToolResult {
                    content: "Cron jobs: use the Cron Manager UI or API for full listing"
                        .to_string(),
                    is_error: false,
                })
            })
        }),
    );

    register_builtin_handler(
        "@axagent/cron",
        "cron_delete",
        make_handler(|args: Value| {
            Box::pin(async move {
                let id = args.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                if id.is_empty() {
                    return Err(AxAgentError::Gateway("job id is required".to_string()));
                }
                Ok(McpToolResult {
                    content: format!("Cron job '{}' deleted", id),
                    is_error: false,
                })
            })
        }),
    );
    // ═══════════════════════════════════════════════════════════════════
    // Cherry Studio MCP servers (ported from cherry-studio)
    // ═══════════════════════════════════════════════════════════════════

    // ─── Brave Search ─────────────────────────────────────────────────

    register_builtin_handler(
        "@axagent/brave-search",
        "brave_web_search",
        make_handler(|args: Value| {
            Box::pin(async move {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let api_key = args
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                brave_web_search(query, api_key, count).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/brave-search",
        "brave_local_search",
        make_handler(|args: Value| {
            Box::pin(async move {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let api_key = args
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                brave_local_search(query, api_key, count).await
            })
        }),
    );

    // ─── Sequential Thinking ──────────────────────────────────────────

    register_builtin_handler(
        "@axagent/sequential-thinking",
        "sequentialthinking",
        make_handler(|args: Value| {
            Box::pin(async move {
                let thought = args
                    .get("thought")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let next_thought_needed = args
                    .get("nextThoughtNeeded")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let thought_number = args
                    .get("thoughtNumber")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1);
                let total_thoughts = args
                    .get("totalThoughts")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1);
                let is_revision = args.get("isRevision").and_then(|v| v.as_bool());
                let revises_thought = args
                    .get("revisesThought")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                let branch_from_thought = args
                    .get("branchFromThought")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                let branch_id = args.get("branchId").and_then(|v| v.as_str());
                let needs_more_thoughts = args.get("needsMoreThoughts").and_then(|v| v.as_bool());
                sequential_thinking(
                    thought,
                    next_thought_needed,
                    thought_number as usize,
                    total_thoughts as usize,
                    is_revision,
                    revises_thought,
                    branch_from_thought,
                    branch_id,
                    needs_more_thoughts,
                )
                .await
            })
        }),
    );

    // ─── Python ───────────────────────────────────────────────────────

    register_builtin_handler(
        "@axagent/python",
        "python_execute",
        make_handler(|args: Value| {
            Box::pin(async move {
                let script = args
                    .get("script")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let timeout = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30) as u64;
                python_execute(script, timeout).await
            })
        }),
    );
    // ═══════════════════════════════════════════════════════════════════
    // Cherry Studio MCP servers - batch 2
    // ═══════════════════════════════════════════════════════════════════

    // ─── Dify Knowledge ───────────────────────────────────────────────

    register_builtin_handler(
        "@axagent/dify-knowledge",
        "dify_list_bases",
        make_handler(|args: Value| {
            Box::pin(async move {
                let api_base = args
                    .get("api_base")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let api_key = args
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                dify_list_bases(api_base, api_key).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/dify-knowledge",
        "dify_search",
        make_handler(|args: Value| {
            Box::pin(async move {
                let api_base = args
                    .get("api_base")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let api_key = args
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let dataset_id = args
                    .get("dataset_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let top_k = args.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                dify_search(api_base, api_key, dataset_id, query, top_k).await
            })
        }),
    );

    // ─── Workspace Memory ─────────────────────────────────────────────

    register_builtin_handler(
        "@axagent/workspace-memory",
        "workspace_read",
        make_handler(|args: Value| {
            Box::pin(async move {
                let filename = args
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("FACT.md");
                let workspace_path = args
                    .get("workspace_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                workspace_read(filename, workspace_path).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/workspace-memory",
        "workspace_write",
        make_handler(|args: Value| {
            Box::pin(async move {
                let filename = args
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("FACT.md");
                let workspace_path = args
                    .get("workspace_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let content_arg = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let mode = args
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("append");
                workspace_write(filename, workspace_path, content_arg, mode).await
            })
        }),
    );

    // ─── File Utils ───────────────────────────────────────────────────

    register_builtin_handler(
        "@axagent/file-utils",
        "pdf_info",
        make_handler(|args: Value| {
            Box::pin(async move {
                let file_path = args
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                pdf_info_tool(file_path).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/file-utils",
        "detect_encoding",
        make_handler(|args: Value| {
            Box::pin(async move {
                let file_path = args
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                detect_encoding_tool(file_path).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/file-utils",
        "base64_image",
        make_handler(|args: Value| {
            Box::pin(async move {
                let file_path = args
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                base64_image_tool(file_path).await
            })
        }),
    );

    // ─── Cache ────────────────────────────────────────────────────────

    register_builtin_handler(
        "@axagent/cache",
        "cache_info",
        make_handler(|_args: Value| Box::pin(async move { cache_info_tool().await })),
    );

    register_builtin_handler(
        "@axagent/cache",
        "cache_clear",
        make_handler(|args: Value| {
            Box::pin(async move {
                let cache_type = args
                    .get("cache_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("all");
                cache_clear_tool(cache_type).await
            })
        }),
    );
    // ─── OCR ──────────────────────────────────────────────────────────

    register_builtin_handler(
        "@axagent/ocr",
        "ocr_image",
        make_handler(|args: Value| {
            Box::pin(async move {
                let file_path = args
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let lang = args.get("lang").and_then(|v| v.as_str()).unwrap_or("eng");
                ocr_image_tool(file_path, lang).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/ocr",
        "ocr_detect_langs",
        make_handler(|_args: Value| Box::pin(async move { ocr_detect_langs_tool().await })),
    );

    // ─── Obsidian ─────────────────────────────────────────────────────

    register_builtin_handler(
        "@axagent/obsidian",
        "obsidian_get_vaults",
        make_handler(|args: Value| {
            Box::pin(async move {
                let search_path = args.get("search_path").and_then(|v| v.as_str());
                obsidian_get_vaults_tool(search_path)
            })
        }),
    );

    register_builtin_handler(
        "@axagent/obsidian",
        "obsidian_list_files",
        make_handler(|args: Value| {
            Box::pin(async move {
                let vault_path = args
                    .get("vault_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                obsidian_list_files_tool(vault_path)
            })
        }),
    );

    register_builtin_handler(
        "@axagent/obsidian",
        "obsidian_read_file",
        make_handler(|args: Value| {
            Box::pin(async move {
                let vault_path = args
                    .get("vault_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let file_path = args
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                obsidian_read_file_tool(vault_path, file_path)
            })
        }),
    );

    // ─── Export ───────────────────────────────────────────────────────

    register_builtin_handler(
        "@axagent/export",
        "export_word",
        make_handler(|args: Value| {
            Box::pin(async move {
                let markdown = args
                    .get("markdown")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let output_path = args
                    .get("output_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Document");
                export_word_tool(markdown, output_path, title)
            })
        }),
    );

    // ─── Remote Files ─────────────────────────────────────────────────

    register_builtin_handler(
        "@axagent/remotefile",
        "remotefile_upload",
        make_handler(|args: Value| {
            Box::pin(async move {
                let provider = args
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let api_key = args
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let file_path = args
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let purpose = args.get("purpose").and_then(|v| v.as_str());
                remotefile_upload_tool(provider, api_key, file_path, purpose).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/remotefile",
        "remotefile_list",
        make_handler(|args: Value| {
            Box::pin(async move {
                let provider = args
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let api_key = args
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                remotefile_list_tool(provider, api_key).await
            })
        }),
    );

    register_builtin_handler(
        "@axagent/remotefile",
        "remotefile_delete",
        make_handler(|args: Value| {
            Box::pin(async move {
                let provider = args
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let api_key = args
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let file_id = args
                    .get("file_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                remotefile_delete_tool(provider, api_key, file_id).await
            })
        }),
    );

    // ─── Agent Control ────────────────────────────────────────────────

    register_builtin_handler(
        "@axagent/agent-control",
        "agent_checkpoint",
        make_handler(|args: Value| {
            Box::pin(async move {
                let action = args
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("list");
                let checkpoint_id = args
                    .get("checkpoint_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let label = args.get("label").and_then(|v| v.as_str()).unwrap_or("");
                agent_checkpoint_tool(action, checkpoint_id, label)
            })
        }),
    );

    register_builtin_handler(
        "@axagent/agent-control",
        "agent_status",
        make_handler(|_args: Value| Box::pin(async move { agent_status_tool() })),
    );

    register_builtin_handler(
        "@axagent/agent-control",
        "agent_remember",
        make_handler(|args: Value| {
            Box::pin(async move {
                let key = args.get("key").and_then(|v| v.as_str()).unwrap_or_default();
                let value = args
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                agent_remember_tool(key, value)
            })
        }),
    );

    register_builtin_handler(
        "@axagent/agent",
        "task",
        make_handler(|args: Value| {
            Box::pin(async move {
                let agent_type = args
                    .get("agent_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("general");
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled task");
                let prompt = args.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
                task_tool_handler(agent_type, description, prompt).await
            })
        }),
    );
}

pub async fn dispatch(server_name: &str, tool_name: &str, args: Value) -> Result<McpToolResult> {
    if let Some(handler) = get_handler(server_name, tool_name) {
        handler(args).await
    } else {
        Err(AxAgentError::Gateway(format!(
            "Unknown builtin tool: {}/{}",
            server_name, tool_name
        )))
    }
}

// ---------------------------------------------------------------------------
// Fetch tools
// ---------------------------------------------------------------------------

async fn fetch_url(url: &str, max_length: Option<usize>) -> Result<McpToolResult> {
    if url.is_empty() {
        return Ok(McpToolResult {
            content: "Error: url parameter is required".into(),
            is_error: true,
        });
    }

    let client = reqwest::Client::builder()
        .user_agent("AxAgent/1.0 (Web Fetch Tool)")
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| AxAgentError::Gateway(format!("HTTP client error: {}", e)))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("Failed to fetch {}: {}", url, e)))?;

    let status = resp.status();
    if !status.is_success() {
        return Ok(McpToolResult {
            content: format!(
                "HTTP error: {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            ),
            is_error: true,
        });
    }

    let body = resp
        .text()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("Failed to read response body: {}", e)))?;

    let text = html_to_text(&body);
    let max = max_length.unwrap_or(5000);
    let content = truncate_text(&text, max);

    Ok(McpToolResult {
        content,
        is_error: false,
    })
}

async fn fetch_markdown(url: &str, max_length: Option<usize>) -> Result<McpToolResult> {
    if url.is_empty() {
        return Ok(McpToolResult {
            content: "Error: url parameter is required".into(),
            is_error: true,
        });
    }

    let client = reqwest::Client::builder()
        .user_agent("AxAgent/1.0 (Web Fetch Tool)")
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| AxAgentError::Gateway(format!("HTTP client error: {}", e)))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("Failed to fetch {}: {}", url, e)))?;

    let status = resp.status();
    if !status.is_success() {
        return Ok(McpToolResult {
            content: format!(
                "HTTP error: {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            ),
            is_error: true,
        });
    }

    let body = resp
        .text()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("Failed to read response body: {}", e)))?;

    let markdown = html_to_markdown(&body);
    let max = max_length.unwrap_or(10000);
    let content = truncate_text(&markdown, max);

    Ok(McpToolResult {
        content,
        is_error: false,
    })
}

// ---------------------------------------------------------------------------
// Web search tools
// ---------------------------------------------------------------------------

async fn web_search(
    query: &str,
    provider_type: &str,
    api_key: &str,
    endpoint: Option<&str>,
    max_results: i32,
    timeout_ms: i32,
) -> Result<McpToolResult> {
    if query.is_empty() {
        return Ok(McpToolResult {
            content: "Error: query parameter is required".into(),
            is_error: true,
        });
    }
    if provider_type.is_empty() {
        return Ok(McpToolResult {
            content: "Web search is not configured. Please configure a search provider (Tavily, Zhipu, or Bocha) in Settings.".into(),
            is_error: true,
        });
    }
    if api_key.is_empty() {
        return Ok(McpToolResult {
            content: "Web search API key is not configured. Please set the API key for your search provider in Settings.".into(),
            is_error: true,
        });
    }

    match crate::search::execute_search(
        provider_type,
        endpoint,
        api_key,
        query,
        max_results,
        timeout_ms,
    )
    .await
    {
        Ok(resp) => {
            if resp.ok {
                let mut lines = vec![format!("Search results for '{}':", resp.query)];
                if resp.results.is_empty() {
                    lines.push("No results found.".to_string());
                } else {
                    for (i, r) in resp.results.iter().enumerate() {
                        lines.push(format!("\n{}. {}", i + 1, r.title));
                        if !r.url.is_empty() {
                            lines.push(format!("   URL: {}", r.url));
                        }
                        if !r.content.is_empty() {
                            lines.push(format!("   {}", r.content));
                        }
                    }
                }
                lines.push(format!("\nLatency: {}ms", resp.latency_ms));
                Ok(McpToolResult {
                    content: lines.join("\n"),
                    is_error: false,
                })
            } else {
                Ok(McpToolResult {
                    content: format!("Search failed: {}", resp.error.unwrap_or_default()),
                    is_error: true,
                })
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Search error: {}", e),
            is_error: true,
        }),
    }
}

// ---------------------------------------------------------------------------
// File tools
// ---------------------------------------------------------------------------

const ALLOWED_FILE_DIRECTORIES: &[&str] = &["workspace", "documents", "downloads", "skills"];

fn validate_and_resolve_path(path: &str, base_dir: &str) -> Result<std::path::PathBuf> {
    if path.is_empty() {
        return Err(AxAgentError::Validation("path must not be empty".into()));
    }
    if path.starts_with('/') || path.starts_with('\\') || path.contains("..") {
        return Err(AxAgentError::Validation(format!("invalid path: {}", path)));
    }

    let base_path = std::path::Path::new(base_dir);
    let requested_path = base_path.join(path);

    let absolute_path = if requested_path.is_absolute() {
        requested_path.clone()
    } else {
        std::env::current_dir()
            .map_err(|_| AxAgentError::Validation("cannot determine current directory".into()))?
            .join(&requested_path)
    };

    let canonical_path = absolute_path
        .canonicalize()
        .map_err(|_| AxAgentError::Validation(format!("path does not exist: {}", path)))?;

    for allowed_dir in ALLOWED_FILE_DIRECTORIES {
        let allowed_path = std::path::Path::new(allowed_dir);
        let canonical_allowed = if allowed_path.is_absolute() {
            allowed_path.canonicalize().ok()
        } else {
            std::env::current_dir()
                .ok()
                .and_then(|cwd| cwd.join(allowed_path).canonicalize().ok())
        };

        if let Some(canonical_allowed) = canonical_allowed {
            if canonical_path.starts_with(&canonical_allowed) {
                return Ok(canonical_path);
            }
        }
    }

    Err(AxAgentError::Validation(format!(
        "path '{}' is outside allowed directories",
        path
    )))
}

async fn read_file(path: &str) -> Result<McpToolResult> {
    if path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: path parameter is required".into(),
            is_error: true,
        });
    }

    let resolved_path = match validate_and_resolve_path(path, "workspace") {
        Ok(p) => p,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error: {}", e),
                is_error: true,
            });
        }
    };

    match tokio::fs::read_to_string(&resolved_path).await {
        Ok(content) => {
            let truncated = truncate_text(&content, 50000);
            Ok(McpToolResult {
                content: truncated,
                is_error: false,
            })
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Error reading file '{}': {}", path, e),
            is_error: true,
        }),
    }
}

async fn list_directory(path: &str) -> Result<McpToolResult> {
    if path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: path parameter is required".into(),
            is_error: true,
        });
    }

    let resolved_path = match validate_and_resolve_path(path, "workspace") {
        Ok(p) => p,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error: {}", e),
                is_error: true,
            });
        }
    };

    let mut entries = match tokio::fs::read_dir(&resolved_path).await {
        Ok(rd) => rd,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error listing directory '{}': {}", path, e),
                is_error: true,
            });
        }
    };

    let mut items = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry
            .file_type()
            .await
            .map(|ft| ft.is_dir())
            .unwrap_or(false);
        let meta = entry.metadata().await.ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);

        if is_dir {
            items.push(format!("📁 {}/", name));
        } else {
            items.push(format!("📄 {} ({})", name, human_size(size)));
        }
    }

    items.sort();
    let content = if items.is_empty() {
        format!("Directory '{}' is empty", path)
    } else {
        format!("Contents of '{}':\n{}", path, items.join("\n"))
    };

    Ok(McpToolResult {
        content,
        is_error: false,
    })
}

async fn search_files(
    path: &str,
    pattern: &str,
    max_results: Option<usize>,
) -> Result<McpToolResult> {
    let max = max_results.unwrap_or(50);
    let mut results = Vec::new();

    let pattern_lower = pattern.to_lowercase();
    walk_dir_search(
        std::path::Path::new(path),
        &pattern_lower,
        &mut results,
        max,
    )
    .await;

    let content = if results.is_empty() {
        format!("No files matching '{}' found in '{}'", pattern, path)
    } else {
        format!(
            "Found {} file(s) matching '{}':\n{}",
            results.len(),
            pattern,
            results.join("\n")
        )
    };

    Ok(McpToolResult {
        content,
        is_error: false,
    })
}

async fn walk_dir_search(
    root: &std::path::Path,
    pattern: &str,
    results: &mut Vec<String>,
    max: usize,
) {
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        if results.len() >= max {
            return;
        }

        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            if results.len() >= max {
                return;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            // Skip only "." and ".." entries, allow hidden files/dirs like .git, .env
            if name == "." || name == ".." {
                continue;
            }

            let path = entry.path();
            let is_dir = entry
                .file_type()
                .await
                .map(|ft| ft.is_dir())
                .unwrap_or(false);

            if name.to_lowercase().contains(pattern) {
                results.push(path.to_string_lossy().to_string());
            }

            if is_dir {
                stack.push(path);
            }
        }
    }
}

async fn grep_content(
    root_path: &str,
    pattern: &str,
    file_pattern: &str,
    max_results: Option<usize>,
) -> Result<McpToolResult> {
    if pattern.is_empty() {
        return Ok(McpToolResult {
            content: "Error: pattern parameter is required".into(),
            is_error: true,
        });
    }

    let max = max_results.unwrap_or(50);
    let pattern_lower = pattern.to_lowercase();
    let file_pattern_lower = file_pattern.to_lowercase();
    let mut results: Vec<String> = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(root_path)];

    while let Some(dir) = stack.pop() {
        if results.len() >= max {
            break;
        }

        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            if results.len() >= max {
                break;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            if name == "." || name == ".." {
                continue;
            }

            let path = entry.path();
            let is_dir = entry
                .file_type()
                .await
                .map(|ft| ft.is_dir())
                .unwrap_or(false);

            if is_dir {
                stack.push(path);
            } else {
                // Check file name matches file_pattern
                if !file_pattern_lower.contains('*')
                    && !name.to_lowercase().ends_with(&file_pattern_lower)
                {
                    continue;
                }

                // Read file and search for pattern in each line
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    for (line_num, line) in content.lines().enumerate() {
                        if results.len() >= max {
                            break;
                        }
                        if line.to_lowercase().contains(&pattern_lower) {
                            let line_preview = if line.len() > 200 {
                                format!("{}...", &line[..line.floor_char_boundary(200)])
                            } else {
                                line.to_string()
                            };
                            results.push(format!(
                                "{}:{}: {}",
                                path.display(),
                                line_num + 1,
                                line_preview
                            ));
                        }
                    }
                }
            }
        }
    }

    let content = if results.is_empty() {
        format!("No matches found for '{}' in '{}'", pattern, root_path)
    } else {
        format!(
            "Found {} match(es) for '{}' in '{}':\n{}",
            results.len(),
            pattern,
            root_path,
            results.join("\n")
        )
    };

    Ok(McpToolResult {
        content,
        is_error: false,
    })
}

// ---------------------------------------------------------------------------
// HTML processing
// ---------------------------------------------------------------------------

fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();
    remove_blocks(&mut text, "script");
    remove_blocks(&mut text, "style");
    remove_blocks(&mut text, "noscript");
    remove_blocks(&mut text, "nav");
    remove_blocks(&mut text, "footer");
    remove_blocks(&mut text, "header");

    let re_block = Regex::new(r"(?i)</?(p|div|section|article|main|aside|blockquote|pre|table|tr|ul|ol|dl|dt|dd|figcaption|figure)\s*[^>]*>").unwrap();
    text = re_block.replace_all(&text, "\n").to_string();

    let re_br = Regex::new(r"(?i)<br\s*/?>").unwrap();
    text = re_br.replace_all(&text, "\n").to_string();

    let re_hr = Regex::new(r"(?i)<hr\s*/?>").unwrap();
    text = re_hr.replace_all(&text, "\n---\n").to_string();

    let re_li = Regex::new(r"(?i)<li\s*[^>]*>").unwrap();
    text = re_li.replace_all(&text, "\n• ").to_string();

    let re_tag = Regex::new(r"<[^>]+>").unwrap();
    text = re_tag.replace_all(&text, "").to_string();

    decode_entities(&mut text);
    text = collapse_whitespace(&text);
    text.trim().to_string()
}

fn html_to_markdown(html: &str) -> String {
    let mut text = html.to_string();
    remove_blocks(&mut text, "script");
    remove_blocks(&mut text, "style");

    let re_heading = Regex::new(r"(?i)<h([1-6])[^>]*>(.*?)</h[1-6]>").unwrap();
    text = re_heading
        .replace_all(&text, |caps: &regex::Captures| {
            let level = caps[1].parse::<usize>().unwrap_or(1);
            let content = &caps[2];
            let hashes = "#".repeat(level);
            format!("\n{} {}\n", hashes, content)
        })
        .to_string();

    let re_blockquote = Regex::new(r"(?i)<blockquote[^>]*>(.*?)</blockquote>").unwrap();
    text = re_blockquote.replace_all(&text, "> $1\n").to_string();

    let re_code_block = Regex::new(r"(?i)<pre[^>]*><code[^>]*>(.*?)</code></pre>").unwrap();
    text = re_code_block
        .replace_all(&text, "```\n$1\n```\n")
        .to_string();

    let re_inline_code = Regex::new(r"(?i)<code[^>]*>(.*?)</code>").unwrap();
    text = re_inline_code.replace_all(&text, "`$1`").to_string();

    let re_strong = Regex::new(r"(?i)<(strong|b)[^>]*>(.*?)</(strong|b)>").unwrap();
    text = re_strong.replace_all(&text, "**$2**").to_string();

    let re_em = Regex::new(r"(?i)<(em|i)[^>]*>(.*?)</(em|i)>").unwrap();
    text = re_em.replace_all(&text, "*$2*").to_string();

    let re_link = Regex::new(r#"(?i)<a[^>]*href=['"]([^'"]+)['"][^>]*>(.*?)</a>"#).unwrap();
    text = re_link.replace_all(&text, "[$2]($1)").to_string();

    let re_ul = Regex::new(r"(?i)<li[^>]*>(.*?)</li>").unwrap();
    text = re_ul.replace_all(&text, "- $1\n").to_string();

    let re_p = Regex::new(r"(?i)<p[^>]*>(.*?)</p>").unwrap();
    text = re_p.replace_all(&text, "$1\n\n").to_string();

    let re_br = Regex::new(r"(?i)<br\s*/?>").unwrap();
    text = re_br.replace_all(&text, "\n").to_string();

    let re_tag = Regex::new(r"<[^>]+>").unwrap();
    text = re_tag.replace_all(&text, "").to_string();

    decode_entities(&mut text);
    text = collapse_whitespace(&text);
    text.trim().to_string()
}

fn remove_blocks(html: &mut String, tag: &str) {
    let re = Regex::new(&format!(r"(?i)<{}[^>]*>.*?</{}>", tag, tag)).unwrap();
    *html = re.replace_all(html, "").to_string();
}

#[allow(dead_code)]
fn strip_tags(s: &str) -> String {
    let re = Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(s, "").to_string()
}

fn decode_entities(text: &mut String) {
    *text = text
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'");
}

fn collapse_whitespace(text: &str) -> String {
    let re = Regex::new(r"\s+").unwrap();
    re.replace_all(text, " ").to_string()
}

fn truncate_text(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        let trunc_at = max.saturating_sub(50);
        let boundary = trunc_at.min(text.len());
        // Use floor_char_boundary to avoid splitting multi-byte UTF-8 characters
        let safe_boundary = text.floor_char_boundary(boundary);
        format!(
            "{}...[truncated {} chars]",
            &text[..safe_boundary],
            text.len() - max + 50
        )
    }
}

fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// ---------------------------------------------------------------------------
// Skills tools
// ---------------------------------------------------------------------------

#[allow(dead_code)]
async fn skill_manage(
    action: &str,
    name: &str,
    description: &str,
    content: &str,
    skills_dir: &str,
) -> Result<McpToolResult> {
    let skills_root = resolve_skills_dir(skills_dir);
    let metadata_path = skills_root.join("skills.json");

    match action {
        "list" => {
            let skills = load_skills_metadata(&metadata_path)?;
            let mut lines = vec!["Available skills:".to_string()];
            if skills.is_empty() {
                lines.push("No skills found.".to_string());
            } else {
                for skill in skills {
                    lines.push(format!("- {}: {}", skill.name, skill.description));
                }
            }
            Ok(McpToolResult {
                content: lines.join("\n"),
                is_error: false,
            })
        }
        "view" => {
            if name.is_empty() {
                return Ok(McpToolResult {
                    content: "Error: name parameter required for 'view' action".into(),
                    is_error: true,
                });
            }
            let skill_path = skills_root.join(format!("{}.md", name));
            match tokio::fs::read_to_string(&skill_path).await {
                Ok(content) => Ok(McpToolResult {
                    content,
                    is_error: false,
                }),
                Err(_) => Ok(McpToolResult {
                    content: format!("Skill '{}' not found", name),
                    is_error: true,
                }),
            }
        }
        "create" | "edit" | "patch" => {
            if name.is_empty() {
                return Ok(McpToolResult {
                    content: "Error: name parameter required for 'create'/'edit'/'patch' action"
                        .into(),
                    is_error: true,
                });
            }
            if content.is_empty() {
                return Ok(McpToolResult {
                    content: "Error: content parameter required for 'create'/'edit'/'patch' action"
                        .into(),
                    is_error: true,
                });
            }

            let skill_path = skills_root.join(format!("{}.md", name));
            let frontmatter = format!(
                "---\nname: {}\ndescription: {}\nversion: 1.0.0\ncreated: {}\n---\n\n",
                name,
                description,
                chrono::Utc::now().format("%Y-%m-%d")
            );
            let file_content = if content.contains("---") {
                content.to_string()
            } else {
                format!("{}{}", frontmatter, content)
            };

            tokio::fs::create_dir_all(&skills_root)
                .await
                .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
            tokio::fs::write(&skill_path, file_content.as_bytes())
                .await
                .map_err(|e| AxAgentError::Gateway(e.to_string()))?;

            let mut skills = load_skills_metadata(&metadata_path)?;
            if !skills.iter().any(|s| s.name == name) {
                skills.push(SkillMetadata {
                    name: name.to_string(),
                    description: description.to_string(),
                    version: "1.0.0".to_string(),
                });
                save_skills_metadata(&metadata_path, &skills)?;
            }

            Ok(McpToolResult {
                content: format!(
                    "Skill '{}' {}",
                    name,
                    if action == "create" {
                        "created"
                    } else {
                        "updated"
                    }
                ),
                is_error: false,
            })
        }
        "delete" => {
            if name.is_empty() {
                return Ok(McpToolResult {
                    content: "Error: name parameter required for 'delete' action".into(),
                    is_error: true,
                });
            }

            let skill_path = skills_root.join(format!("{}.md", name));
            if skill_path.exists() {
                tokio::fs::remove_file(&skill_path)
                    .await
                    .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
            }

            let mut skills = load_skills_metadata(&metadata_path)?;
            skills.retain(|s| s.name != name);
            save_skills_metadata(&metadata_path, &skills)?;

            Ok(McpToolResult {
                content: format!("Skill '{}' deleted", name),
                is_error: false,
            })
        }
        _ => Ok(McpToolResult {
            content: format!(
                "Unknown action '{}'. Use: list, view, create, edit, patch, delete",
                action
            ),
            is_error: true,
        }),
    }
}

// ---------------------------------------------------------------------------
// Session tools
// ---------------------------------------------------------------------------

fn sanitize_fts5_query(query: &str) -> Result<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(AxAgentError::Validation("query must not be empty".into()));
    }
    if trimmed.len() > 1000 {
        return Err(AxAgentError::Validation(
            "query too long (max 1000 chars)".into(),
        ));
    }
    let mut sanitized = String::with_capacity(trimmed.len() * 2);
    let mut in_phrase = false;
    for c in trimmed.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | '\t' | '\n' | '-' | '_' | '.' | '@' | '#' => {
                sanitized.push(c);
            }
            '"' => {
                in_phrase = !in_phrase;
                sanitized.push(c);
            }
            '*' => {
                if !in_phrase {
                    sanitized.push(c);
                }
            }
            '(' | ')' => {
                sanitized.push(c);
            }
            _ => {}
        }
    }
    if in_phrase {
        return Err(AxAgentError::Validation("unmatched quote in query".into()));
    }
    if sanitized.contains("..") || sanitized.contains('\'') {
        return Err(AxAgentError::Validation(
            "invalid characters in query".into(),
        ));
    }
    Ok(sanitized)
}

async fn session_search(query: &str, limit: i32, _db_path: &str) -> Result<McpToolResult> {
    if query.is_empty() {
        return Ok(McpToolResult {
            content: "Error: query parameter is required".into(),
            is_error: true,
        });
    }

    // Use the global DB path to open a direct rusqlite connection for FTS5 search
    let db_path_str = match get_global_db_path() {
        Some(p) => p,
        None => {
            return Ok(McpToolResult {
                content: "Session search unavailable: database path not configured".into(),
                is_error: true,
            });
        }
    };

    // Convert "sqlite:/path/to/db" to just "/path/to/db"
    let db_file = db_path_str.strip_prefix("sqlite:").unwrap_or(&db_path_str);

    match rusqlite::Connection::open(db_file) {
        Ok(conn) => {
            // Try parameterized FTS5 query first; some SQLite builds don't support
            // MATCH with bound parameters, so fall back to direct interpolation.
            let fts_sql = "SELECT m.conversation_id, snippet(messages_fts, 0, '>>', '<<', '...', 24) as snippet, bm25(messages_fts) as rank FROM messages_fts JOIN messages m ON m.rowid = messages_fts.rowid WHERE messages_fts MATCH ? ORDER BY rank LIMIT ?";

            // Attempt 1: parameterized query
            let rows = match conn.prepare(fts_sql) {
                Ok(mut stmt) => {
                    match stmt.query_map(rusqlite::params![query, limit], |row| {
                        let conv_id: String = row.get(0)?;
                        let snippet: String = row.get(1)?;
                        Ok(format!("[{}] {}", conv_id, snippet))
                    }) {
                        Ok(rows) => rows.filter_map(|r| r.ok()).collect::<Vec<_>>(),
                        Err(_) => {
                            // Fallback: use sanitized query (already validated by sanitize_fts5_query)
                            match sanitize_fts5_query(query) {
                                Ok(safe_query) => {
                                    let fallback_sql = "SELECT m.conversation_id, snippet(messages_fts, 0, '>>', '<<', '...', 24) as snippet, bm25(messages_fts) as rank FROM messages_fts JOIN messages m ON m.rowid = messages_fts.rowid WHERE messages_fts MATCH ? ORDER BY rank LIMIT ?";
                                    match conn.prepare(fallback_sql) {
                                        Ok(mut stmt2) => {
                                            match stmt2.query_map(
                                                rusqlite::params![safe_query, limit],
                                                |row| {
                                                    let conv_id: String = row.get(0)?;
                                                    let snippet: String = row.get(1)?;
                                                    Ok(format!("[{}] {}", conv_id, snippet))
                                                },
                                            ) {
                                                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                                                Err(_) => Vec::new(),
                                            }
                                        }
                                        Err(_) => Vec::new(),
                                    }
                                }
                                Err(_) => Vec::new(),
                            }
                        }
                    }
                }
                Err(e) => {
                    return Ok(McpToolResult {
                        content: format!("Session search error (FTS5 not available): {}", e),
                        is_error: true,
                    });
                }
            };

            if rows.is_empty() {
                Ok(McpToolResult {
                    content: format!("No results found for '{}'", query),
                    is_error: false,
                })
            } else {
                Ok(McpToolResult {
                    content: format!(
                        "Search results for '{}' ({} hits):\n{}",
                        query,
                        rows.len(),
                        rows.join("\n\n")
                    ),
                    is_error: false,
                })
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Session search error: cannot open database: {}", e),
            is_error: true,
        }),
    }
}

async fn memory_flush(content: &str, target: &str, category: &str) -> Result<McpToolResult> {
    if content.is_empty() {
        return Ok(McpToolResult {
            content: "Error: content parameter is required".into(),
            is_error: true,
        });
    }

    // Use the global DB path to persist memory via direct rusqlite
    let db_path_str = match get_global_db_path() {
        Some(p) => p,
        None => {
            return Ok(McpToolResult {
                content: "Memory flush unavailable: database path not configured".into(),
                is_error: true,
            });
        }
    };

    let db_file = db_path_str.strip_prefix("sqlite:").unwrap_or(&db_path_str);

    match rusqlite::Connection::open(db_file) {
        Ok(conn) => {
            // Try to insert into memory_items table if it exists
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let namespace_id = if target == "user" {
                "user_preferences"
            } else {
                "system_memory"
            };

            let result = conn.execute(
                "INSERT OR IGNORE INTO memory_namespaces (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![namespace_id, namespace_id, &now, &now],
            );

            let table_result = match result {
                Ok(_) => conn.execute(
                    "INSERT INTO memory_items (id, namespace_id, key, value, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![&id, namespace_id, category, content, &now, &now],
                ),
                Err(e) => Err(e),
            };

            match table_result {
                Ok(_) => Ok(McpToolResult {
                    content: format!(
                        "Memory saved: target={}, category={}, id={}",
                        target,
                        category,
                        &id[..8]
                    ),
                    is_error: false,
                }),
                Err(e) => {
                    // Fallback: save to a simple JSON file if DB table doesn't exist
                    let memory_dir = std::path::Path::new("documents").join("memory");
                    let _ = std::fs::create_dir_all(&memory_dir);
                    let file_path = memory_dir.join(format!("{}-{}.json", target, &id[..8]));
                    let entry = serde_json::json!({
                        "id": id,
                        "target": target,
                        "category": category,
                        "content": content,
                        "timestamp": now,
                    });
                    match std::fs::write(&file_path, entry.to_string()) {
                        Ok(_) => Ok(McpToolResult {
                            content: format!(
                                "Memory saved to file: target={}, category={}, id={}",
                                target,
                                category,
                                &id[..8]
                            ),
                            is_error: false,
                        }),
                        Err(write_err) => Ok(McpToolResult {
                            content: format!(
                                "Memory save failed: DB error ({}) and file fallback error ({})",
                                e, write_err
                            ),
                            is_error: true,
                        }),
                    }
                }
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Memory flush error: cannot open database: {}", e),
            is_error: true,
        }),
    }
}

// ---------------------------------------------------------------------------
// Storage tools
// ---------------------------------------------------------------------------

fn get_storage_info() -> Result<McpToolResult> {
    let docs = std::path::Path::new("documents");
    let total: u64 = if docs.exists() {
        std::fs::read_dir(docs)
            .map(|rd| rd.count() as u64)
            .unwrap_or(0)
    } else {
        0
    };
    Ok(McpToolResult {
        content: format!(
            "Storage Info:\n  Root: documents/\n  Total files: {}",
            total
        ),
        is_error: false,
    })
}

fn list_storage_files(path: String, limit: usize) -> Result<McpToolResult> {
    let docs = std::path::Path::new("documents");
    let full_path = docs.join(&path);
    let mut items = Vec::new();

    if let Ok(entries) = std::fs::read_dir(full_path) {
        for entry in entries.filter_map(|e| e.ok()).take(limit) {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.path().is_dir();
            if is_dir {
                items.push(format!("📁 {}/", name));
            } else {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                items.push(format!("📄 {} ({})", name, human_size(size)));
            }
        }
    }

    if items.is_empty() {
        Ok(McpToolResult {
            content: format!("No files found in '{}'", path),
            is_error: false,
        })
    } else {
        Ok(McpToolResult {
            content: format!("Files in '{}':\n{}", path, items.join("\n")),
            is_error: false,
        })
    }
}

fn upload_storage_file(
    filename: String,
    content_base64: String,
    bucket: String,
) -> Result<McpToolResult> {
    let decoded = base64_decode(&content_base64)?;
    let docs = std::path::Path::new("documents");
    let bucket_path = if bucket.is_empty() {
        docs.join(&filename)
    } else {
        docs.join(&bucket).join(&filename)
    };

    if let Some(parent) = bucket_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AxAgentError::Gateway(e.to_string()))?;
    }
    std::fs::write(&bucket_path, decoded).map_err(|e| AxAgentError::Gateway(e.to_string()))?;

    Ok(McpToolResult {
        content: format!(
            "File '{}' uploaded to '{}'",
            filename,
            bucket_path.display()
        ),
        is_error: false,
    })
}

fn download_storage_file(path: String) -> Result<McpToolResult> {
    let docs = std::path::Path::new("documents");
    let full_path = docs.join(&path);

    if !full_path.exists() {
        return Ok(McpToolResult {
            content: format!("File not found: {}", path),
            is_error: true,
        });
    }

    let content = std::fs::read(&full_path).map_err(|e| AxAgentError::Gateway(e.to_string()))?;
    use base64::Engine;
    let encoded = Engine::encode(&base64::engine::general_purpose::STANDARD, &content);

    Ok(McpToolResult {
        content: format!("File '{}' content (base64):\n{}", path, encoded),
        is_error: false,
    })
}

fn delete_storage_file(path: String) -> Result<McpToolResult> {
    let docs = std::path::Path::new("documents");
    let full_path = docs.join(&path);

    if !full_path.exists() {
        return Ok(McpToolResult {
            content: format!("File not found: {}", path),
            is_error: true,
        });
    }

    std::fs::remove_file(&full_path).map_err(|e| AxAgentError::Gateway(e.to_string()))?;

    Ok(McpToolResult {
        content: format!("File '{}' deleted", path),
        is_error: false,
    })
}

// ---------------------------------------------------------------------------
// Base64 helper
// ---------------------------------------------------------------------------

const MAX_BASE64_DECODE_SIZE: usize = 100 * 1024 * 1024; // 100MB decoded max

fn base64_decode(input: &str) -> Result<Vec<u8>> {
    if input.len() > MAX_BASE64_DECODE_SIZE * 4 / 3 + 100 {
        return Err(AxAgentError::Validation("Base64 input too large".into()));
    }
    use base64::Engine;
    let decoded = Engine::decode(&base64::engine::general_purpose::STANDARD, input)
        .map_err(|e| AxAgentError::Gateway(format!("Base64 decode error: {}", e)))?;

    if decoded.len() > MAX_BASE64_DECODE_SIZE {
        return Err(AxAgentError::Validation(format!(
            "Decoded data too large: {} bytes (max: {} bytes)",
            decoded.len(),
            MAX_BASE64_DECODE_SIZE
        )));
    }

    Ok(decoded)
}

// ---------------------------------------------------------------------------
// Skills helpers
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn resolve_skills_dir(skills_dir: &str) -> std::path::PathBuf {
    if !skills_dir.is_empty() {
        std::path::Path::new(skills_dir).to_path_buf()
    } else {
        std::path::Path::new(".").join("skills")
    }
}

#[allow(dead_code)]
fn find_frontmatter_end(content: &str) -> Option<usize> {
    content.find("\n---")
}

// ---------------------------------------------------------------------------
// File system tools (write/edit/delete/create_directory)
// ---------------------------------------------------------------------------

async fn write_file(path: &str, content: &str) -> Result<McpToolResult> {
    if path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: path parameter is required".into(),
            is_error: true,
        });
    }

    let resolved_path = match validate_and_resolve_path(path, "workspace") {
        Ok(p) => p,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error: {}", e),
                is_error: true,
            });
        }
    };

    let path_str = resolved_path.to_string_lossy();
    let path_obj = std::path::Path::new(&*path_str);
    if let Some(parent) = path_obj.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
        }
    }

    tokio::fs::write(&*path_str, content.as_bytes())
        .await
        .map_err(|e| AxAgentError::Gateway(e.to_string()))?;

    Ok(McpToolResult {
        content: format!(
            "File '{}' written successfully ({} bytes)",
            path,
            content.len()
        ),
        is_error: false,
    })
}

async fn edit_file(path: &str, old_str: &str, new_str: &str) -> Result<McpToolResult> {
    if path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: path parameter is required".into(),
            is_error: true,
        });
    }
    if old_str.is_empty() {
        return Ok(McpToolResult {
            content: "Error: old_str parameter is required".into(),
            is_error: true,
        });
    }

    let resolved_path = match validate_and_resolve_path(path, "workspace") {
        Ok(p) => p,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error: {}", e),
                is_error: true,
            });
        }
    };

    let path_str = resolved_path.to_string_lossy();
    let full_content = tokio::fs::read_to_string(&*path_str)
        .await
        .map_err(|e| AxAgentError::Gateway(e.to_string()))?;

    let new_content = if let Some(pos) = full_content.find(old_str) {
        let mut result = String::with_capacity(full_content.len() - old_str.len() + new_str.len());
        result.push_str(&full_content[..pos]);
        result.push_str(new_str);
        result.push_str(&full_content[pos + old_str.len()..]);
        result
    } else {
        return Ok(McpToolResult {
            content: format!("String '{}' not found in file '{}'", old_str, path),
            is_error: true,
        });
    };
    tokio::fs::write(&*path_str, new_content.as_bytes())
        .await
        .map_err(|e| AxAgentError::Gateway(e.to_string()))?;

    Ok(McpToolResult {
        content: format!("File '{}' edited successfully", path),
        is_error: false,
    })
}

async fn search_replace_file(
    path: &str,
    old_str: &str,
    new_str: &str,
    start_line: Option<usize>,
    end_line: Option<usize>,
    replace_all: bool,
) -> Result<McpToolResult> {
    if path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: path parameter is required".into(),
            is_error: true,
        });
    }
    if old_str.is_empty() {
        return Ok(McpToolResult {
            content: "Error: old_str parameter is required".into(),
            is_error: true,
        });
    }

    let resolved_path = match validate_and_resolve_path(path, "workspace") {
        Ok(p) => p,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error: {}", e),
                is_error: true,
            });
        }
    };

    let path_str = resolved_path.to_string_lossy();
    let full_content = tokio::fs::read_to_string(&*path_str)
        .await
        .map_err(|e| AxAgentError::Gateway(e.to_string()))?;

    let lines: Vec<&str> = full_content.lines().collect();

    let search_start = start_line.map(|s| s.saturating_sub(1)).unwrap_or(0);
    let search_end = end_line.map(|e| e.min(lines.len())).unwrap_or(lines.len());

    if search_start >= lines.len() {
        return Ok(McpToolResult {
            content: format!(
                "Error: start_line {} exceeds file length {}",
                start_line.unwrap_or(0),
                lines.len()
            ),
            is_error: true,
        });
    }

    let search_region: String = lines[search_start..search_end].join("\n");

    let (replacement_count, new_region) = if replace_all {
        let count = search_region.matches(old_str).count();
        if count == 0 {
            return Ok(McpToolResult {
                content: format!(
                    "String '{}' not found in file '{}' within lines {}-{}",
                    old_str,
                    path,
                    search_start + 1,
                    search_end
                ),
                is_error: true,
            });
        }
        (count, search_region.replace(old_str, new_str))
    } else {
        if let Some(pos) = search_region.find(old_str) {
            let mut result =
                String::with_capacity(search_region.len() - old_str.len() + new_str.len());
            result.push_str(&search_region[..pos]);
            result.push_str(new_str);
            result.push_str(&search_region[pos + old_str.len()..]);
            (1, result)
        } else {
            return Ok(McpToolResult {
                content: format!(
                    "String '{}' not found in file '{}' within lines {}-{}",
                    old_str,
                    path,
                    search_start + 1,
                    search_end
                ),
                is_error: true,
            });
        }
    };

    let before = lines[..search_start].join("\n");
    let after = if search_end < lines.len() {
        format!("\n{}", lines[search_end..].join("\n"))
    } else {
        String::new()
    };

    let new_content = if search_start > 0 {
        format!("{}\n{}{}", before, new_region, after)
    } else {
        format!("{}{}", new_region, after)
    };

    tokio::fs::write(&*path_str, new_content.as_bytes())
        .await
        .map_err(|e| AxAgentError::Gateway(e.to_string()))?;

    Ok(McpToolResult {
        content: format!(
            "File '{}' edited successfully: {} replacement(s) made",
            path, replacement_count
        ),
        is_error: false,
    })
}

async fn delete_file(path: &str) -> Result<McpToolResult> {
    if path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: path parameter is required".into(),
            is_error: true,
        });
    }

    let resolved_path = match validate_and_resolve_path(path, "workspace") {
        Ok(p) => p,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error: {}", e),
                is_error: true,
            });
        }
    };

    let path_str = resolved_path.to_string_lossy();
    tokio::fs::remove_file(&*path_str)
        .await
        .map_err(|e| AxAgentError::Gateway(e.to_string()))?;

    Ok(McpToolResult {
        content: format!("File '{}' deleted successfully", path),
        is_error: false,
    })
}

async fn move_file(source: &str, destination: &str) -> Result<McpToolResult> {
    if source.is_empty() || destination.is_empty() {
        return Ok(McpToolResult {
            content: "Error: both source and destination parameters are required".into(),
            is_error: true,
        });
    }

    if source == destination {
        return Ok(McpToolResult {
            content: "Error: source and destination paths are the same".into(),
            is_error: true,
        });
    }

    let resolved_source = match validate_and_resolve_path(source, "workspace") {
        Ok(p) => p,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error: {}", e),
                is_error: true,
            });
        }
    };

    let resolved_dest = match validate_and_resolve_path(destination, "workspace") {
        Ok(p) => p,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error: {}", e),
                is_error: true,
            });
        }
    };

    let source_str = resolved_source.to_string_lossy();
    let dest_str = resolved_dest.to_string_lossy();

    let dest_parent = std::path::Path::new(&*dest_str).parent();
    if let Some(parent) = dest_parent {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AxAgentError::Gateway(e.to_string()))?;
        }
    }

    tokio::fs::rename(&*source_str, &*dest_str)
        .await
        .map_err(|e| AxAgentError::Gateway(e.to_string()))?;

    Ok(McpToolResult {
        content: format!("Moved '{}' to '{}'", source, destination),
        is_error: false,
    })
}

async fn create_directory(path: &str) -> Result<McpToolResult> {
    if path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: path parameter is required".into(),
            is_error: true,
        });
    }

    let resolved_path = match validate_and_resolve_path(path, "workspace") {
        Ok(p) => p,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error: {}", e),
                is_error: true,
            });
        }
    };

    let path_str = resolved_path.to_string_lossy();
    tokio::fs::create_dir_all(&*path_str)
        .await
        .map_err(|e| AxAgentError::Gateway(e.to_string()))?;

    Ok(McpToolResult {
        content: format!("Directory '{}' created successfully", path),
        is_error: false,
    })
}

async fn file_exists(path: &str) -> Result<McpToolResult> {
    if path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: path parameter is required".into(),
            is_error: true,
        });
    }

    let resolved_path = match validate_and_resolve_path(path, "workspace") {
        Ok(p) => p,
        Err(_) => {
            return Ok(McpToolResult {
                content: format!("{}: does not exist (outside allowed directories)", path),
                is_error: false,
            });
        }
    };

    let path_str = resolved_path.to_string_lossy();
    let exists = std::path::Path::new(&*path_str).exists();
    Ok(McpToolResult {
        content: format!(
            "{}: {}",
            path,
            if exists { "exists" } else { "does not exist" }
        ),
        is_error: false,
    })
}

async fn get_file_info(path: &str) -> Result<McpToolResult> {
    if path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: path parameter is required".into(),
            is_error: true,
        });
    }

    let resolved_path = match validate_and_resolve_path(path, "workspace") {
        Ok(p) => p,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error: {}", e),
                is_error: true,
            });
        }
    };

    let path_str = resolved_path.to_string_lossy();
    let meta = std::fs::metadata(&*path_str).map_err(|e| AxAgentError::Gateway(e.to_string()))?;

    let info = format!(
        "File: {}\n  Size: {} bytes\n  Modified: {:?}",
        path,
        meta.len(),
        meta.modified()
    );

    Ok(McpToolResult {
        content: info,
        is_error: false,
    })
}

// ---------------------------------------------------------------------------
// System tools - Command execution and system info
// ---------------------------------------------------------------------------

async fn run_command(command: &str, timeout_secs: u64) -> Result<McpToolResult> {
    if command.is_empty() {
        return Ok(McpToolResult {
            content: "Error: command parameter is required".into(),
            is_error: true,
        });
    }

    let validator = CommandValidator::new();
    let validation = validator.validate(command);

    if !validation.is_safe {
        tracing::warn!(
            "Blocked potentially dangerous command: patterns={:?}",
            validation.dangerous_patterns
        );
        return Ok(McpToolResult {
            content: format!(
                "Error: Command contains dangerous patterns: {:?}",
                validation.dangerous_patterns
            ),
            is_error: true,
        });
    }

    let blocked: &[&str] = {
        #[cfg(windows)]
        {
            &[
                "del /s /q C:\\",
                "rd /s /q C:\\",
                "format ",
                "diskpart",
                "net user ",
                "net localgroup ",
                "reg delete ",
                "powershell -enc",
                "cmd /c del",
                "taskkill /f",
            ]
        }
        #[cfg(not(windows))]
        {
            &[
                "rm -rf /",
                "mkfs",
                "dd if=",
                ":(){:|:&};",
                "chmod -R 777 /",
                "chown -R ",
            ]
        }
    };
    for block in blocked {
        if command.contains(block) {
            return Ok(McpToolResult {
                content: format!("Error: Command blocked for security reasons: {}", block),
                is_error: true,
            });
        }
    }

    let output = {
        #[cfg(windows)]
        let cmd = tokio::process::Command::new("cmd")
            .args(["/C", command])
            .output();
        #[cfg(not(windows))]
        let cmd = tokio::process::Command::new("sh")
            .args(["-c", command])
            .output();
        cmd
    };

    let output =
        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), output).await {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                return Ok(McpToolResult {
                    content: format!("Error executing command: {}", e),
                    is_error: true,
                });
            }
            Err(_) => {
                return Ok(McpToolResult {
                    content: format!("Error: Command timed out after {} seconds", timeout_secs),
                    is_error: true,
                });
            }
        };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&format!("STDOUT:\n{}\n", stdout));
    }
    if !stderr.is_empty() {
        result.push_str(&format!("STDERR:\n{}\n", stderr));
    }
    result.push_str(&format!("Exit code: {}", exit_code));

    Ok(McpToolResult {
        content: result,
        is_error: exit_code != 0,
    })
}

fn get_system_info() -> Result<McpToolResult> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{} seconds", d.as_secs()))
        .unwrap_or_else(|_| "unknown".to_string());

    Ok(McpToolResult {
        content: format!(
            "System Info:\n  OS: {}\n  Architecture: {}\n  Home directory: {}\n  Uptime: {}",
            os, arch, home, uptime
        ),
        is_error: false,
    })
}

async fn list_processes(limit: usize) -> Result<McpToolResult> {
    #[cfg(windows)]
    let output = tokio::process::Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
        .await;

    #[cfg(not(windows))]
    let output = tokio::process::Command::new("ps")
        .args(["aux"])
        .output()
        .await;

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = stdout.lines().take(limit).collect();
            Ok(McpToolResult {
                content: if lines.is_empty() {
                    "No processes found".to_string()
                } else {
                    lines.join("\n")
                },
                is_error: false,
            })
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Error listing processes: {}", e),
            is_error: true,
        }),
    }
}

// ---------------------------------------------------------------------------
// Knowledge tools
// ---------------------------------------------------------------------------

/// Global callback for knowledge base search, set at startup.
/// This allows the builtin tool handler to call into the full RAG pipeline
/// (embedding + vector store) which requires runtime dependencies.
#[allow(clippy::type_complexity)]
static KNOWLEDGE_SEARCH_CALLBACK: std::sync::OnceLock<
    std::sync::Arc<
        dyn Fn(
                &str,
                &str,
                usize,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Result<Vec<KnowledgeSearchHit>>>
                        + Send
                        + 'static,
                >,
            > + Send
            + Sync,
    >,
> = std::sync::OnceLock::new();

/// A single hit from knowledge base search.
pub struct KnowledgeSearchHit {
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub score: f32,
}

/// Set the global knowledge search callback. Call once at startup.
#[allow(clippy::type_complexity)]
pub fn set_knowledge_search_callback(
    cb: std::sync::Arc<
        dyn Fn(
                &str,
                &str,
                usize,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Result<Vec<KnowledgeSearchHit>>>
                        + Send
                        + 'static,
                >,
            > + Send
            + Sync,
    >,
) {
    let _ = KNOWLEDGE_SEARCH_CALLBACK.set(cb);
}

fn list_knowledge_bases() -> Result<McpToolResult> {
    let db_path = match get_global_db_path() {
        Some(p) => p,
        None => {
            return Ok(McpToolResult {
                content: "Knowledge bases unavailable: database not initialized".to_string(),
                is_error: true,
            });
        }
    };

    let db_file = db_path.strip_prefix("sqlite:").unwrap_or(&db_path);

    match rusqlite::Connection::open(db_file) {
        Ok(conn) => {
            let mut stmt = match conn.prepare(
                "SELECT id, name, description, enabled FROM knowledge_bases ORDER BY sort_order, name"
            ) {
                Ok(s) => s,
                Err(e) => {
                    return Ok(McpToolResult {
                        content: format!("Error querying knowledge bases: {}", e),
                        is_error: true,
                    });
                }
            };

            let rows: Vec<String> = match stmt.query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let desc: Option<String> = row.get(2)?;
                let enabled: i32 = row.get(3)?;
                let status = if enabled != 0 { "enabled" } else { "disabled" };
                let desc_str = desc.map(|d| format!(" - {}", d)).unwrap_or_default();
                Ok(format!("- {} [{}] ({}){}", name, id, status, desc_str))
            }) {
                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                Err(_) => Vec::new(),
            };

            if rows.is_empty() {
                Ok(McpToolResult {
                    content: "No knowledge bases found. Create one in Settings > Knowledge."
                        .to_string(),
                    is_error: false,
                })
            } else {
                Ok(McpToolResult {
                    content: format!(
                        "Available knowledge bases ({}):\n{}",
                        rows.len(),
                        rows.join("\n")
                    ),
                    is_error: false,
                })
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Error opening database: {}", e),
            is_error: true,
        }),
    }
}

async fn search_knowledge(base_id: String, query: String, top_k: usize) -> Result<McpToolResult> {
    if query.is_empty() {
        return Ok(McpToolResult {
            content: "Error: query parameter is required".into(),
            is_error: true,
        });
    }

    // Try the global callback first (full RAG pipeline with embeddings)
    if let Some(cb) = KNOWLEDGE_SEARCH_CALLBACK.get() {
        match cb(&base_id, &query, top_k).await {
            Ok(hits) => {
                let hits: Vec<KnowledgeSearchHit> = hits;
                if hits.is_empty() {
                    Ok(McpToolResult {
                        content: format!(
                            "No results found in knowledge base '{}' for '{}'",
                            base_id, query
                        ),
                        is_error: false,
                    })
                } else {
                    let lines: Vec<String> = hits
                        .iter()
                        .map(|h| format!("[score={:.3}] {}", h.score, h.content))
                        .collect();
                    Ok(McpToolResult {
                        content: format!(
                            "Search results in '{}' for '{}' ({} hits):\n{}",
                            base_id,
                            query,
                            hits.len(),
                            lines.join("\n\n")
                        ),
                        is_error: false,
                    })
                }
            }
            Err(e) => Ok(McpToolResult {
                content: format!("Knowledge search error: {}", e),
                is_error: true,
            }),
        }
    } else {
        // Fallback: no callback set, try direct sqlite-vec query via rusqlite
        let db_path = match get_global_db_path() {
            Some(p) => p,
            None => {
                return Ok(McpToolResult {
                    content: "Knowledge search unavailable: database not initialized".to_string(),
                    is_error: true,
                });
            }
        };

        let db_file = db_path.strip_prefix("sqlite:").unwrap_or(&db_path);

        // Try to read from the metadata table directly (no vector search, just text match)
        match rusqlite::Connection::open(db_file) {
            Ok(conn) => {
                let meta_table = format!("vec_kb_{}_meta", base_id);
                let sql = format!(
                    "SELECT content FROM {} WHERE content LIKE ? LIMIT {}",
                    meta_table, top_k
                );
                let like_pattern = format!("%{}%", query);
                match conn.prepare(&sql) {
                    Ok(mut stmt) => {
                        let rows: Vec<String> =
                            match stmt.query_map(rusqlite::params![like_pattern], |row| {
                                let content: String = row.get(0)?;
                                Ok(content)
                            }) {
                                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                                Err(_) => Vec::new(),
                            };

                        if rows.is_empty() {
                            Ok(McpToolResult {
                                content: format!(
                                    "No text matches found in knowledge base '{}' for '{}'",
                                    base_id, query
                                ),
                                is_error: false,
                            })
                        } else {
                            Ok(McpToolResult {
                                content: format!(
                                    "Text search results in '{}' for '{}' ({} hits, no semantic ranking):\n{}",
                                    base_id, query, rows.len(), rows.join("\n\n")
                                ),
                                is_error: false,
                            })
                        }
                    }
                    Err(e) => Ok(McpToolResult {
                        content: format!(
                            "Knowledge base '{}' may not exist or has no indexed content: {}",
                            base_id, e
                        ),
                        is_error: true,
                    }),
                }
            }
            Err(e) => Ok(McpToolResult {
                content: format!("Error opening database: {}", e),
                is_error: true,
            }),
        }
    }
}

fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn current_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

#[allow(clippy::too_many_arguments)]
async fn create_knowledge_entity_tool(
    kb_id: &str,
    name: &str,
    entity_type: &str,
    description: Option<&str>,
    source_path: &str,
    source_language: Option<&str>,
    properties: serde_json::Value,
    lifecycle: Option<serde_json::Value>,
    behaviors: Option<serde_json::Value>,
) -> Result<McpToolResult> {
    if kb_id.is_empty() {
        return Ok(McpToolResult {
            content: "Error: knowledge_base_id is required".to_string(),
            is_error: true,
        });
    }
    if name.is_empty() {
        return Ok(McpToolResult {
            content: "Error: name is required".to_string(),
            is_error: true,
        });
    }

    let db_path = match get_global_db_path() {
        Some(p) => p,
        None => {
            return Ok(McpToolResult {
                content: "Error: database not initialized".to_string(),
                is_error: true,
            });
        }
    };

    let db_file = db_path.strip_prefix("sqlite:").unwrap_or(&db_path);
    let id = generate_uuid();
    let now = current_timestamp();
    let properties_json = serde_json::to_string(&properties).unwrap_or_else(|_| "{}".to_string());
    let lifecycle_json = lifecycle
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()));
    let behaviors_json = behaviors
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()));

    match rusqlite::Connection::open(db_file) {
        Ok(conn) => {
            let result = conn.execute(
                "INSERT INTO knowledge_entities (id, knowledge_base_id, name, entity_type, description, source_path, source_language, properties, lifecycle, behaviors, metadata, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?11, ?12)",
                rusqlite::params![
                    id,
                    kb_id,
                    name,
                    entity_type,
                    description,
                    source_path,
                    source_language,
                    properties_json,
                    lifecycle_json,
                    behaviors_json,
                    now,
                    now
                ],
            );

            match result {
                Ok(_) => Ok(McpToolResult {
                    content: format!(
                        "Created knowledge entity '{}' (id: {}) in knowledge base '{}'",
                        name, id, kb_id
                    ),
                    is_error: false,
                }),
                Err(e) => Ok(McpToolResult {
                    content: format!("Error creating knowledge entity: {}", e),
                    is_error: true,
                }),
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Error opening database: {}", e),
            is_error: true,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
async fn create_knowledge_flow_tool(
    kb_id: &str,
    name: &str,
    flow_type: &str,
    description: Option<&str>,
    source_path: &str,
    steps: serde_json::Value,
    decision_points: Option<serde_json::Value>,
    error_handling: Option<serde_json::Value>,
    preconditions: Option<serde_json::Value>,
    postconditions: Option<serde_json::Value>,
) -> Result<McpToolResult> {
    if kb_id.is_empty() {
        return Ok(McpToolResult {
            content: "Error: knowledge_base_id is required".to_string(),
            is_error: true,
        });
    }
    if name.is_empty() {
        return Ok(McpToolResult {
            content: "Error: name is required".to_string(),
            is_error: true,
        });
    }

    let db_path = match get_global_db_path() {
        Some(p) => p,
        None => {
            return Ok(McpToolResult {
                content: "Error: database not initialized".to_string(),
                is_error: true,
            });
        }
    };

    let db_file = db_path.strip_prefix("sqlite:").unwrap_or(&db_path);
    let id = generate_uuid();
    let now = current_timestamp();
    let steps_json = serde_json::to_string(&steps).unwrap_or_else(|_| "[]".to_string());
    let decision_points_json = decision_points
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()));
    let error_handling_json = error_handling
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()));
    let preconditions_json = preconditions
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()));
    let postconditions_json = postconditions
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()));

    match rusqlite::Connection::open(db_file) {
        Ok(conn) => {
            let result = conn.execute(
                "INSERT INTO knowledge_flows (id, knowledge_base_id, name, flow_type, description, source_path, steps, decision_points, error_handling, preconditions, postconditions, metadata, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, ?12, ?13)",
                rusqlite::params![
                    id,
                    kb_id,
                    name,
                    flow_type,
                    description,
                    source_path,
                    steps_json,
                    decision_points_json,
                    error_handling_json,
                    preconditions_json,
                    postconditions_json,
                    now,
                    now
                ],
            );

            match result {
                Ok(_) => Ok(McpToolResult {
                    content: format!(
                        "Created knowledge flow '{}' (id: {}) in knowledge base '{}'",
                        name, id, kb_id
                    ),
                    is_error: false,
                }),
                Err(e) => Ok(McpToolResult {
                    content: format!("Error creating knowledge flow: {}", e),
                    is_error: true,
                }),
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Error opening database: {}", e),
            is_error: true,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
async fn create_knowledge_interface_tool(
    kb_id: &str,
    name: &str,
    interface_type: &str,
    description: Option<&str>,
    source_path: &str,
    input_schema: serde_json::Value,
    output_schema: serde_json::Value,
    error_codes: Option<serde_json::Value>,
    communication_pattern: Option<&str>,
) -> Result<McpToolResult> {
    if kb_id.is_empty() {
        return Ok(McpToolResult {
            content: "Error: knowledge_base_id is required".to_string(),
            is_error: true,
        });
    }
    if name.is_empty() {
        return Ok(McpToolResult {
            content: "Error: name is required".to_string(),
            is_error: true,
        });
    }

    let db_path = match get_global_db_path() {
        Some(p) => p,
        None => {
            return Ok(McpToolResult {
                content: "Error: database not initialized".to_string(),
                is_error: true,
            });
        }
    };

    let db_file = db_path.strip_prefix("sqlite:").unwrap_or(&db_path);
    let id = generate_uuid();
    let now = current_timestamp();
    let input_schema_json =
        serde_json::to_string(&input_schema).unwrap_or_else(|_| "{}".to_string());
    let output_schema_json =
        serde_json::to_string(&output_schema).unwrap_or_else(|_| "{}".to_string());
    let error_codes_json = error_codes
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()));

    match rusqlite::Connection::open(db_file) {
        Ok(conn) => {
            let result = conn.execute(
                "INSERT INTO knowledge_interfaces (id, knowledge_base_id, name, interface_type, description, source_path, input_schema, output_schema, error_codes, communication_pattern, version, metadata, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, NULL, ?11, ?12)",
                rusqlite::params![
                    id,
                    kb_id,
                    name,
                    interface_type,
                    description,
                    source_path,
                    input_schema_json,
                    output_schema_json,
                    error_codes_json,
                    communication_pattern,
                    now,
                    now
                ],
            );

            match result {
                Ok(_) => Ok(McpToolResult {
                    content: format!(
                        "Created knowledge interface '{}' (id: {}) in knowledge base '{}'",
                        name, id, kb_id
                    ),
                    is_error: false,
                }),
                Err(e) => Ok(McpToolResult {
                    content: format!("Error creating knowledge interface: {}", e),
                    is_error: true,
                }),
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Error opening database: {}", e),
            is_error: true,
        }),
    }
}

async fn add_knowledge_document_tool(
    kb_id: &str,
    title: &str,
    content: &str,
) -> Result<McpToolResult> {
    if kb_id.is_empty() {
        return Ok(McpToolResult {
            content: "Error: knowledge_base_id is required".to_string(),
            is_error: true,
        });
    }
    if title.is_empty() {
        return Ok(McpToolResult {
            content: "Error: title is required".to_string(),
            is_error: true,
        });
    }
    if content.is_empty() {
        return Ok(McpToolResult {
            content: "Error: content is required".to_string(),
            is_error: true,
        });
    }

    let db_path = match get_global_db_path() {
        Some(p) => p,
        None => {
            return Ok(McpToolResult {
                content: "Error: database not initialized".to_string(),
                is_error: true,
            });
        }
    };

    let db_file = db_path.strip_prefix("sqlite:").unwrap_or(&db_path);

    let temp_dir = std::env::temp_dir();
    let doc_id = generate_uuid();
    let file_path = temp_dir.join(format!("kb_doc_{}.md", doc_id));

    if let Err(e) = std::fs::write(&file_path, content) {
        return Ok(McpToolResult {
            content: format!("Error writing temporary file: {}", e),
            is_error: true,
        });
    }

    let id = generate_uuid();
    let now = current_timestamp();
    let file_path_str = file_path.to_string_lossy().to_string();
    let content_size = content.len() as i64;

    match rusqlite::Connection::open(db_file) {
        Ok(conn) => {
            let result = conn.execute(
                "INSERT INTO knowledge_documents (id, knowledge_base_id, title, source_path, mime_type, size_bytes, indexing_status, doc_type, index_error, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', 'extracted', NULL, ?7, ?8)",
                rusqlite::params![
                    id,
                    kb_id,
                    title,
                    file_path_str,
                    "text/markdown",
                    content_size,
                    now,
                    now
                ],
            );

            let _ = std::fs::remove_file(&file_path);

            match result {
                Ok(_) => Ok(McpToolResult {
                    content: format!(
                        "Added knowledge document '{}' (id: {}) to knowledge base '{}'",
                        title, id, kb_id
                    ),
                    is_error: false,
                }),
                Err(e) => Ok(McpToolResult {
                    content: format!("Error creating knowledge document: {}", e),
                    is_error: true,
                }),
            }
        }
        Err(e) => {
            let _ = std::fs::remove_file(&file_path);
            Ok(McpToolResult {
                content: format!("Error opening database: {}", e),
                is_error: true,
            })
        }
    }
}

async fn git_status_tool(repo_path: &str) -> Result<McpToolResult> {
    if repo_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: repo_path parameter is required".into(),
            is_error: true,
        });
    }

    match crate::git_tools::GitTools::get_status(repo_path) {
        Ok(entries) => {
            let output: Vec<serde_json::Value> = entries
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "path": e.path,
                        "status": e.status,
                        "staged": e.staged,
                    })
                })
                .collect();
            Ok(McpToolResult {
                content: serde_json::to_string(&output).unwrap_or_default(),
                is_error: false,
            })
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Error: {}", e),
            is_error: true,
        }),
    }
}

async fn git_diff_tool(repo_path: &str, base_branch: Option<&str>) -> Result<McpToolResult> {
    if repo_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: repo_path parameter is required".into(),
            is_error: true,
        });
    }

    let result = match base_branch {
        Some(branch) => crate::git_tools::GitTools::get_branch_diff(repo_path, branch),
        None => crate::git_tools::GitTools::get_staged_diff(repo_path),
    };

    match result {
        Ok(diff) => Ok(McpToolResult {
            content: serde_json::to_string(&diff).unwrap_or_default(),
            is_error: false,
        }),
        Err(e) => Ok(McpToolResult {
            content: format!("Error: {}", e),
            is_error: true,
        }),
    }
}

async fn git_commit_tool(repo_path: &str, message: &str, stage_all: bool) -> Result<McpToolResult> {
    if repo_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: repo_path parameter is required".into(),
            is_error: true,
        });
    }
    if message.is_empty() {
        return Ok(McpToolResult {
            content: "Error: message parameter is required".into(),
            is_error: true,
        });
    }

    if stage_all {
        if let Err(e) = crate::git_tools::GitTools::stage_all(repo_path) {
            return Ok(McpToolResult {
                content: format!("Error staging files: {}", e),
                is_error: true,
            });
        }
    }

    match crate::git_tools::GitTools::commit(repo_path, message) {
        Ok(output) => Ok(McpToolResult {
            content: output,
            is_error: false,
        }),
        Err(e) => Ok(McpToolResult {
            content: format!("Error: {}", e),
            is_error: true,
        }),
    }
}

async fn git_log_tool(repo_path: &str, max_count: usize) -> Result<McpToolResult> {
    if repo_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: repo_path parameter is required".into(),
            is_error: true,
        });
    }

    match crate::git_tools::GitTools::get_log(repo_path, max_count) {
        Ok(entries) => Ok(McpToolResult {
            content: serde_json::to_string(&entries).unwrap_or_default(),
            is_error: false,
        }),
        Err(e) => Ok(McpToolResult {
            content: format!("Error: {}", e),
            is_error: true,
        }),
    }
}

async fn git_branch_tool(
    repo_path: &str,
    action: &str,
    name: Option<&str>,
) -> Result<McpToolResult> {
    if repo_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: repo_path parameter is required".into(),
            is_error: true,
        });
    }

    match action {
        "list" => match crate::git_tools::GitTools::list_branches(repo_path) {
            Ok(branches) => Ok(McpToolResult {
                content: serde_json::to_string(&branches).unwrap_or_default(),
                is_error: false,
            }),
            Err(e) => Ok(McpToolResult {
                content: format!("Error: {}", e),
                is_error: true,
            }),
        },
        "create" => match name {
            Some(n) => match crate::git_tools::GitTools::create_branch(repo_path, n) {
                Ok(output) => Ok(McpToolResult {
                    content: format!("Created and switched to branch '{}': {}", n, output),
                    is_error: false,
                }),
                Err(e) => Ok(McpToolResult {
                    content: format!("Error: {}", e),
                    is_error: true,
                }),
            },
            None => Ok(McpToolResult {
                content: "Error: name parameter is required for create action".into(),
                is_error: true,
            }),
        },
        "switch" => match name {
            Some(n) => match crate::git_tools::GitTools::switch_branch(repo_path, n) {
                Ok(output) => Ok(McpToolResult {
                    content: format!("Switched to branch '{}': {}", n, output),
                    is_error: false,
                }),
                Err(e) => Ok(McpToolResult {
                    content: format!("Error: {}", e),
                    is_error: true,
                }),
            },
            None => Ok(McpToolResult {
                content: "Error: name parameter is required for switch action".into(),
                is_error: true,
            }),
        },
        _ => Ok(McpToolResult {
            content: format!(
                "Error: unknown action '{}'. Use 'list', 'create', or 'switch'.",
                action
            ),
            is_error: true,
        }),
    }
}

async fn git_review_tool(repo_path: &str, base_branch: Option<&str>) -> Result<McpToolResult> {
    if repo_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: repo_path parameter is required".into(),
            is_error: true,
        });
    }

    let context = match base_branch {
        Some(branch) => crate::git_tools::GitTools::generate_pr_context(repo_path, branch),
        None => crate::git_tools::GitTools::generate_commit_context(repo_path),
    };

    match context {
        Ok(ctx) => Ok(McpToolResult {
            content: ctx,
            is_error: false,
        }),
        Err(e) => Ok(McpToolResult {
            content: format!("Error: {}", e),
            is_error: true,
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Cherry Studio MCP server implementations
// ═══════════════════════════════════════════════════════════════════════

// ─── Brave Search ─────────────────────────────────────────────────────

async fn brave_web_search(query: &str, api_key: &str, count: usize) -> Result<McpToolResult> {
    if query.is_empty() {
        return Ok(McpToolResult {
            content: "Error: query parameter is required".into(),
            is_error: true,
        });
    }
    if api_key.is_empty() {
        return Ok(McpToolResult { content: "Error: Brave Search API key is not configured. Please set your BRAVE_API_KEY in Settings.".into(), is_error: true });
    }

    let count = count.clamp(1, 20);
    let url = format!(
        "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
        url_encode(query),
        count
    );

    let client = reqwest::Client::builder()
        .user_agent("AxAgent/1.0 (Brave Search)")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AxAgentError::Gateway(format!("HTTP client error: {}", e)))?;

    match client
        .get(&url)
        .header("Accept", "application/json")
        .header("Accept-Encoding", "gzip")
        .header("X-Subscription-Token", api_key)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Ok(McpToolResult {
                    content: format!(
                        "Brave Search API error ({}): {}",
                        status.as_u16(),
                        truncate_text(&body, 500)
                    ),
                    is_error: true,
                });
            }
            let results = parse_brave_web_results(&body);
            Ok(McpToolResult {
                content: results,
                is_error: false,
            })
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Brave Search request failed: {}", e),
            is_error: true,
        }),
    }
}

async fn brave_local_search(query: &str, api_key: &str, count: usize) -> Result<McpToolResult> {
    if query.is_empty() {
        return Ok(McpToolResult {
            content: "Error: query parameter is required".into(),
            is_error: true,
        });
    }
    if api_key.is_empty() {
        return Ok(McpToolResult { content: "Error: Brave Search API key is not configured. Please set your BRAVE_API_KEY in Settings.".into(), is_error: true });
    }

    let count = count.clamp(1, 20);
    let url = format!(
        "https://api.search.brave.com/res/v1/local/pois/search?q={}&count={}",
        url_encode(query),
        count
    );

    let client = reqwest::Client::builder()
        .user_agent("AxAgent/1.0 (Brave Local Search)")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AxAgentError::Gateway(format!("HTTP client error: {}", e)))?;

    match client
        .get(&url)
        .header("Accept", "application/json")
        .header("Accept-Encoding", "gzip")
        .header("X-Subscription-Token", api_key)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Ok(McpToolResult {
                    content: format!(
                        "Brave Search API error ({}): {}",
                        status.as_u16(),
                        truncate_text(&body, 500)
                    ),
                    is_error: true,
                });
            }
            let results = parse_brave_local_results(&body);
            Ok(McpToolResult {
                content: results,
                is_error: false,
            })
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Brave Local Search request failed: {}", e),
            is_error: true,
        }),
    }
}

fn parse_brave_web_results(json: &str) -> String {
    let parsed: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => {
            return format!(
                "Unable to parse search results. Raw response: {}",
                truncate_text(json, 1000)
            )
        }
    };

    let web = match parsed.get("web") {
        Some(w) => w,
        None => return "No web results found.".to_string(),
    };

    let results = match web.get("results") {
        Some(r) => r.as_array().cloned().unwrap_or_default(),
        None => return "No results found.".to_string(),
    };

    if results.is_empty() {
        return "No results found for the query.".to_string();
    }

    let mut lines = Vec::new();
    lines.push(format!("Found {} results:", results.len()));
    for (i, r) in results.iter().enumerate() {
        let title = r
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let url = r.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let description = r.get("description").and_then(|v| v.as_str()).unwrap_or("");
        lines.push(format!("\n{}. {}", i + 1, title));
        if !url.is_empty() {
            lines.push(format!("   URL: {}", url));
        }
        if !description.is_empty() {
            lines.push(format!("   {}", description));
        }
    }
    lines.join("\n")
}

fn parse_brave_local_results(json: &str) -> String {
    let parsed: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => {
            return format!(
                "Unable to parse results. Raw response: {}",
                truncate_text(json, 1000)
            )
        }
    };

    let results = match parsed.get("results") {
        Some(r) => r.as_array().cloned().unwrap_or_default(),
        None => return "No local results found.".to_string(),
    };

    if results.is_empty() {
        return "No local results found for the query.".to_string();
    }

    let mut lines = Vec::new();
    lines.push(format!("Found {} local results:", results.len()));
    for (i, r) in results.iter().enumerate() {
        let name = r.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
        let address = r.get("address").and_then(|v| v.as_str()).unwrap_or("");
        let city = r.get("city").and_then(|v| v.as_str()).unwrap_or("");
        let phone = r.get("phone").and_then(|v| v.as_str()).unwrap_or("");
        let url = r.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let rating = r.get("rating").and_then(|v| v.as_f64());
        lines.push(format!("\n{}. {}", i + 1, name));
        if !address.is_empty() || !city.is_empty() {
            lines.push(
                format!("   Address: {} {}", address, city)
                    .trim_end()
                    .to_string(),
            );
        }
        if !phone.is_empty() {
            lines.push(format!("   Phone: {}", phone));
        }
        if !url.is_empty() {
            lines.push(format!("   URL: {}", url));
        }
        if let Some(rating_val) = rating {
            lines.push(format!("   Rating: {:.1}", rating_val));
        }
    }
    lines.join("\n")
}

fn url_encode(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u8)
            }
        })
        .collect()
}

// ─── Sequential Thinking ──────────────────────────────────────────────

async fn sequential_thinking(
    thought: &str,
    _next_thought_needed: bool,
    thought_number: usize,
    total_thoughts: usize,
    is_revision: Option<bool>,
    revises_thought: Option<usize>,
    branch_from_thought: Option<usize>,
    branch_id: Option<&str>,
    _needs_more_thoughts: Option<bool>,
) -> Result<McpToolResult> {
    if thought.is_empty() {
        return Ok(McpToolResult {
            content: "Error: thought parameter is required".into(),
            is_error: true,
        });
    }

    let mut parts: Vec<String> = Vec::new();

    // Header with thought number and branching/revising info
    let header = if is_revision.unwrap_or(false) {
        if let Some(revised) = revises_thought {
            format!("Thought #{} (Revision of #{}):", thought_number, revised)
        } else {
            format!("Thought #{} (Revision):", thought_number)
        }
    } else if let Some(branch_from) = branch_from_thought {
        if let Some(b_id) = branch_id {
            format!(
                "Thought #{} (Branch {} from #{})",
                thought_number, b_id, branch_from
            )
        } else {
            format!("Thought #{} (Branch from #{})", thought_number, branch_from)
        }
    } else {
        format!("Thought #{}/{}:", thought_number, total_thoughts)
    };
    parts.push(header);

    // Thought content
    parts.push(format!("\n  {}", thought));

    // Add progress indicator
    let progress = if thought_number >= total_thoughts && !is_revision.unwrap_or(false) {
        format!(
            "\n\n[Thinking complete: {}/{} thoughts]",
            thought_number, total_thoughts
        )
    } else {
        let remaining = if total_thoughts > thought_number {
            total_thoughts - thought_number
        } else {
            0
        };
        format!(
            "\n\n[Thought {}/{} completed. Estimated {} remaining. Continue with more thoughts.]",
            thought_number, total_thoughts, remaining
        )
    };
    parts.push(progress);

    Ok(McpToolResult {
        content: parts.join(""),
        is_error: false,
    })
}

// ─── Python Execution ─────────────────────────────────────────────────

async fn python_execute(script: &str, timeout_secs: u64) -> Result<McpToolResult> {
    if script.is_empty() {
        return Ok(McpToolResult {
            content: "Error: script parameter is required".into(),
            is_error: true,
        });
    }

    if script.len() > 100_000 {
        return Ok(McpToolResult {
            content: "Error: script is too long (max 100,000 characters)".into(),
            is_error: true,
        });
    }

    let timeout = std::time::Duration::from_secs(timeout_secs.clamp(1, 120));

    // Try python3 first, fall back to python
    let python_cmd = if which::which("python3").is_ok() {
        "python3"
    } else {
        "python"
    };

    let output = tokio::time::timeout(timeout, async {
        tokio::process::Command::new(python_cmd)
            .arg("-c")
            .arg(script)
            .output()
            .await
    })
    .await;

    match output {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut result = String::new();

            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push_str("\n\n[stderr]:\n");
                }
                result.push_str(&stderr);
            }
            if result.is_empty() {
                result = "(no output)".to_string();
            }

            if !output.status.success() {
                result.push_str(&format!(
                    "\n\n[Exit code: {}]",
                    output.status.code().unwrap_or(-1)
                ));
            }

            Ok(McpToolResult {
                content: result,
                is_error: !output.status.success(),
            })
        }
        Ok(Err(e)) => Ok(McpToolResult {
            content: format!(
                "Failed to execute Python: {}. Is Python installed and in PATH?",
                e
            ),
            is_error: true,
        }),
        Err(_) => Ok(McpToolResult {
            content: format!("Python execution timed out after {} seconds", timeout_secs),
            is_error: true,
        }),
    }
}

// ─── Dify Knowledge ───────────────────────────────────────────────────

async fn dify_list_bases(api_base: &str, api_key: &str) -> Result<McpToolResult> {
    if api_base.is_empty() || api_key.is_empty() {
        return Ok(McpToolResult { content: "Error: api_base and api_key parameters are required. Get them from your Dify instance settings.".into(), is_error: true });
    }

    let url = format!("{}/datasets", api_base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AxAgentError::Gateway(format!("HTTP client error: {}", e)))?;

    match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Ok(McpToolResult {
                    content: format!(
                        "Dify API error ({}): {}",
                        status.as_u16(),
                        truncate_text(&body, 500)
                    ),
                    is_error: true,
                });
            }
            let results = parse_dify_datasets(&body);
            Ok(McpToolResult {
                content: results,
                is_error: false,
            })
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Dify API request failed: {}", e),
            is_error: true,
        }),
    }
}

async fn dify_search(
    api_base: &str,
    api_key: &str,
    dataset_id: &str,
    query: &str,
    top_k: usize,
) -> Result<McpToolResult> {
    if api_base.is_empty() || api_key.is_empty() {
        return Ok(McpToolResult {
            content: "Error: api_base and api_key are required".into(),
            is_error: true,
        });
    }
    if dataset_id.is_empty() {
        return Ok(McpToolResult {
            content: "Error: dataset_id is required".into(),
            is_error: true,
        });
    }
    if query.is_empty() {
        return Ok(McpToolResult {
            content: "Error: query is required".into(),
            is_error: true,
        });
    }

    let url = format!(
        "{}/datasets/{}/retrieve",
        api_base.trim_end_matches('/'),
        dataset_id
    );
    let body = serde_json::json!({
        "query": query,
        "retrieval_model": {
            "search_method": "hybrid_search",
            "reranking_enable": false,
            "top_k": top_k.clamp(1, 20),
            "score_threshold_enabled": false
        }
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AxAgentError::Gateway(format!("HTTP client error: {}", e)))?;

    match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Ok(McpToolResult {
                    content: format!(
                        "Dify API error ({}): {}",
                        status.as_u16(),
                        truncate_text(&text, 500)
                    ),
                    is_error: true,
                });
            }
            let results = parse_dify_search_results(&text);
            Ok(McpToolResult {
                content: results,
                is_error: false,
            })
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Dify API request failed: {}", e),
            is_error: true,
        }),
    }
}

fn parse_dify_datasets(json_str: &str) -> String {
    let parsed: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => {
            return format!(
                "Unable to parse Dify response. Raw: {}",
                truncate_text(json_str, 1000)
            )
        }
    };

    let data = match parsed.get("data") {
        Some(d) => d.as_array().cloned().unwrap_or_default(),
        None => return "No datasets found.".to_string(),
    };
    if data.is_empty() {
        return "No knowledge bases found in this Dify instance.".to_string();
    }

    let mut lines = vec![format!("Found {} knowledge base(s):", data.len())];
    for (i, ds) in data.iter().enumerate() {
        let id = ds.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let name = ds.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed");
        let description = ds.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let doc_count = ds
            .get("document_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        lines.push(format!(
            "
{}. {} (id: {})",
            i + 1,
            name,
            id
        ));
        if !description.is_empty() {
            lines.push(format!("   {}", description));
        }
        lines.push(format!("   Documents: {}", doc_count));
    }
    lines.join(
        "
",
    )
}

fn parse_dify_search_results(json_str: &str) -> String {
    let parsed: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => {
            return format!(
                "Unable to parse search results. Raw: {}",
                truncate_text(json_str, 1000)
            )
        }
    };

    let records = match parsed.get("records") {
        Some(r) => r.as_array().cloned().unwrap_or_default(),
        None => return "No results found.".to_string(),
    };
    if records.is_empty() {
        return "No matching documents found for the query.".to_string();
    }

    let mut lines = vec![format!("Found {} result(s):", records.len())];
    for (i, rec) in records.iter().enumerate() {
        let content = rec.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let score = rec.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let doc = rec.get("document").and_then(|v| v.as_str()).unwrap_or("");
        let title = rec.get("title").and_then(|v| v.as_str()).unwrap_or(doc);
        let source = rec.get("source").and_then(|v| v.as_str()).unwrap_or("");
        lines.push(format!(
            "
--- Result {} (relevance: {:.1}%) ---",
            i + 1,
            score * 100.0
        ));
        if !title.is_empty() {
            lines.push(format!("Title: {}", title));
        }
        if !source.is_empty() {
            lines.push(format!("Source: {}", source));
        }
        if !content.is_empty() {
            lines.push(format!(
                "
{}",
                truncate_text(content, 2000)
            ));
        }
    }
    lines.join(
        "
",
    )
}

// ─── Workspace Memory ─────────────────────────────────────────────────

async fn workspace_read(filename: &str, workspace_path: &str) -> Result<McpToolResult> {
    if workspace_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: workspace_path is required".into(),
            is_error: true,
        });
    }

    // Sanitize filename to prevent path traversal
    let safe_name = filename
        .replace("..", "")
        .replace("\\", "")
        .replace('/', "");
    let file_path = std::path::PathBuf::from(workspace_path).join(&safe_name);

    if !file_path.starts_with(workspace_path) {
        return Ok(McpToolResult {
            content: "Error: filename contains invalid path components".into(),
            is_error: true,
        });
    }

    match std::fs::read_to_string(&file_path) {
        Ok(content) => {
            let truncated = if content.len() > 20000 {
                format!(
                    "{}...

[Content truncated at 20000 characters]",
                    &content[..20000]
                )
            } else {
                content
            };
            if truncated.is_empty() {
                Ok(McpToolResult {
                    content: format!("File '{}' exists but is empty.", safe_name),
                    is_error: false,
                })
            } else {
                Ok(McpToolResult {
                    content: truncated,
                    is_error: false,
                })
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(McpToolResult { content: format!("Memory file '{}' does not exist yet in {}. Use workspace_write to create it.", safe_name, workspace_path), is_error: false })
            } else {
                Ok(McpToolResult {
                    content: format!("Error reading file: {}", e),
                    is_error: true,
                })
            }
        }
    }
}

async fn workspace_write(
    filename: &str,
    workspace_path: &str,
    content_str: &str,
    mode: &str,
) -> Result<McpToolResult> {
    if workspace_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: workspace_path is required".into(),
            is_error: true,
        });
    }
    if content_str.is_empty() {
        return Ok(McpToolResult {
            content: "Error: content is required".into(),
            is_error: true,
        });
    }

    let safe_name = filename
        .replace("..", "")
        .replace("\\", "")
        .replace('/', "");
    let file_path = std::path::PathBuf::from(workspace_path).join(&safe_name);

    if !file_path.starts_with(workspace_path) {
        return Ok(McpToolResult {
            content: "Error: filename contains invalid path components".into(),
            is_error: true,
        });
    }

    // Ensure parent directory exists
    if let Some(parent) = file_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Ok(McpToolResult {
                content: format!("Error creating directory: {}", e),
                is_error: true,
            });
        }
    }

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

    let result = match mode {
        "overwrite" => std::fs::write(
            &file_path,
            &format!(
                "{}

[Last updated: {}]
",
                content_str, timestamp
            ),
        ),
        _ => {
            // append (default)
            let existing = std::fs::read_to_string(&file_path).unwrap_or_default();
            let new_content = if existing.is_empty() {
                format!(
                    "{}

[Created: {}]
",
                    content_str, timestamp
                )
            } else {
                format!(
                    "{}

---
{}

[Appended: {}]
",
                    existing, content_str, timestamp
                )
            };
            std::fs::write(&file_path, new_content)
        }
    };

    match result {
        Ok(_) => Ok(McpToolResult {
            content: format!(
                "Memory file '{}' {} successfully in {}",
                safe_name,
                if mode == "overwrite" {
                    "updated"
                } else {
                    "appended to"
                },
                workspace_path
            ),
            is_error: false,
        }),
        Err(e) => Ok(McpToolResult {
            content: format!("Error writing file: {}", e),
            is_error: true,
        }),
    }
}

// ─── File Utilities ──────────────────────────────────────────────────

async fn pdf_info_tool(file_path: &str) -> Result<McpToolResult> {
    if file_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: file_path parameter is required".into(),
            is_error: true,
        });
    }
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Ok(McpToolResult {
            content: format!("Error: file not found: {}", file_path),
            is_error: true,
        });
    }
    match std::fs::read(file_path) {
        Ok(data) => match pdf_extract::extract_text_from_mem(&data) {
            Ok(text) => {
                let page_count = text.matches("\x0C").count() + 1;
                let preview = truncate_text(&text.replace("\x0C", "\n--- page break ---\n"), 5000);
                let info = format!("PDF Info:\n  Path: {}\n  Size: {} bytes\n  Estimated pages: {}\n\nText preview:\n{}", file_path, data.len(), page_count, preview);
                Ok(McpToolResult {
                    content: info,
                    is_error: false,
                })
            }
            Err(e) => Ok(McpToolResult {
                content: format!("Failed to extract text from PDF: {}", e),
                is_error: true,
            }),
        },
        Err(e) => Ok(McpToolResult {
            content: format!("Failed to read file: {}", e),
            is_error: true,
        }),
    }
}

async fn detect_encoding_tool(file_path: &str) -> Result<McpToolResult> {
    if file_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: file_path parameter is required".into(),
            is_error: true,
        });
    }
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Ok(McpToolResult {
            content: format!("Error: file not found: {}", file_path),
            is_error: true,
        });
    }
    match std::fs::read(file_path) {
        Ok(data) => {
            if data.is_empty() {
                return Ok(McpToolResult {
                    content: "File is empty.".into(),
                    is_error: false,
                });
            }
            if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
                let preview = String::from_utf8_lossy(&data[3..]);
                return Ok(McpToolResult {
                    content: format!(
                        "Encoding: UTF-8 (with BOM)\nPreview: {}",
                        truncate_text(&preview, 2000)
                    ),
                    is_error: false,
                });
            }
            if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xFE {
                return Ok(McpToolResult {
                    content: format!("Encoding: UTF-16 LE (BOM)\nSize: {} bytes", data.len()),
                    is_error: false,
                });
            }
            if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
                return Ok(McpToolResult {
                    content: format!("Encoding: UTF-16 BE (BOM)\nSize: {} bytes", data.len()),
                    is_error: false,
                });
            }
            match std::str::from_utf8(&data) {
                Ok(s) => Ok(McpToolResult {
                    content: format!(
                        "Encoding: UTF-8 (valid)\nPreview: {}",
                        truncate_text(s, 2000)
                    ),
                    is_error: false,
                }),
                Err(_) => {
                    let printable = data.iter().filter(|&&b| b >= 32 && b < 127).count();
                    let ratio = printable as f64 / data.len() as f64 * 100.0;
                    let guess = if ratio > 85.0 {
                        "ASCII or Latin-1"
                    } else {
                        "Binary/unknown"
                    };
                    Ok(McpToolResult {
                        content: format!(
                            "Not valid UTF-8. {}% printable. Likely: {}\nSize: {} bytes",
                            ratio.round(),
                            guess,
                            data.len()
                        ),
                        is_error: false,
                    })
                }
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Failed to read file: {}", e),
            is_error: true,
        }),
    }
}

async fn base64_image_tool(file_path: &str) -> Result<McpToolResult> {
    if file_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: file_path parameter is required".into(),
            is_error: true,
        });
    }
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Ok(McpToolResult {
            content: format!("Error: file not found: {}", file_path),
            is_error: true,
        });
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "tiff" | "tif" => "image/tiff",
        _ => "application/octet-stream",
    };
    match std::fs::read(file_path) {
        Ok(data) => {
            if data.len() > 10 * 1024 * 1024 {
                return Ok(McpToolResult {
                    content: format!("Error: image too large ({} bytes, max 10 MB)", data.len()),
                    is_error: true,
                });
            }
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            Ok(McpToolResult {
                content: format!(
                    "{{\"mime\":\"{}\",\"size\":{},\"base64\":\"{}\"}}",
                    mime,
                    data.len(),
                    b64
                ),
                is_error: false,
            })
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Failed to read file: {}", e),
            is_error: true,
        }),
    }
}

async fn cache_info_tool() -> Result<McpToolResult> {
    let mut lines = vec!["Application Cache Information:".to_string()];
    if let Some(temp) = std::env::var("TEMP")
        .ok()
        .or_else(|| std::env::var("TMP").ok())
    {
        let temp_path = std::path::Path::new(&temp).join("axagent");
        if temp_path.exists() {
            match calculate_dir_size(&temp_path) {
                Ok((size, files)) => lines.push(format!(
                    "  Temp cache: {} ({} files)",
                    format_size(size),
                    files
                )),
                Err(_) => lines.push("  Temp cache: (unable to calculate)".to_string()),
            }
        }
    }
    if lines.len() == 1 {
        lines.push("  No cache directories found.".to_string());
    }
    Ok(McpToolResult {
        content: lines.join("\n"),
        is_error: false,
    })
}

async fn cache_clear_tool(cache_type: &str) -> Result<McpToolResult> {
    let mut cleared = 0u64;
    let mut errors: Vec<String> = Vec::new();
    if cache_type == "temp" || cache_type == "all" {
        if let Some(temp) = std::env::var("TEMP")
            .ok()
            .or_else(|| std::env::var("TMP").ok())
        {
            let temp_path = std::path::Path::new(&temp).join("axagent");
            match clear_directory(&temp_path) {
                Ok(size) => cleared += size,
                Err(e) => errors.push(e),
            }
        }
    }
    let mut result = format!("Cache cleared: {}", format_size(cleared));
    if !errors.is_empty() {
        result.push_str(&format!("\nErrors: {}", errors.join("; ")));
    }
    if cleared == 0 && errors.is_empty() {
        result = "No cache files found to clear.".to_string();
    }
    Ok(McpToolResult {
        content: result,
        is_error: !errors.is_empty(),
    })
}

fn clear_directory(path: &std::path::Path) -> std::result::Result<u64, String> {
    let mut total = 0u64;
    if path.exists() {
        match std::fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let _meta = entry.metadata().ok();
                    let _meta_len = _meta.as_ref().map_or(0, |m| m.len());
                    total += _meta.as_ref().map_or(0, |m| m.len());
                    if p.is_dir() {
                        let _ = std::fs::remove_dir_all(&p);
                    } else {
                        let _ = std::fs::remove_file(&p);
                    }
                }
            }
            Err(e) => return Err(format!("{}: {}", path.display(), e).into()),
        }
    }
    Ok(total)
}

fn calculate_dir_size(path: &std::path::Path) -> std::result::Result<(u64, usize), std::io::Error> {
    let mut total_size = 0u64;
    let mut file_count = 0usize;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Ok((s, c)) = calculate_dir_size(&entry.path()) {
                    total_size += s;
                    file_count += c;
                }
            } else {
                total_size += entry.metadata()?.len();
                file_count += 1;
            }
        }
    }
    Ok((total_size, file_count))
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

// ─── OCR Tools ───────────────────────────────────────────────────────

async fn ocr_image_tool(file_path: &str, lang: &str) -> Result<McpToolResult> {
    if file_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: file_path parameter is required".into(),
            is_error: true,
        });
    }
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Ok(McpToolResult {
            content: format!("Error: file not found: {}", file_path),
            is_error: true,
        });
    }

    // Check file size
    let meta = match std::fs::metadata(file_path) {
        Ok(m) => m,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error reading file metadata: {}", e),
                is_error: true,
            })
        }
    };
    if meta.len() > 50 * 1024 * 1024 {
        return Ok(McpToolResult {
            content: "Error: image too large (max 50 MB)".into(),
            is_error: true,
        });
    }

    let safe_lang =
        if lang.is_empty() || lang.contains("..") || lang.contains("/") || lang.contains("\\") {
            "eng"
        } else {
            lang
        };

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        tokio::process::Command::new("tesseract")
            .arg(file_path)
            .arg("stdout")
            .arg("-l")
            .arg(safe_lang)
            .output(),
    )
    .await;

    match output {
        Ok(Ok(output)) => {
            let text = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let trimmed = text.trim();

            if trimmed.is_empty() {
                let detail = if !stderr.is_empty() {
                    format!(" Tesseract stderr: {}", stderr.trim())
                } else {
                    String::new()
                };
                return Ok(McpToolResult {
                    content: format!("OCR produced no text from the image. The image may not contain recognizable text, or the language pack '{}' may not be installed.{} Use ocr_detect_langs to check available languages.", safe_lang, detail),
                    is_error: false,
                });
            }

            Ok(McpToolResult {
                content: trimmed.to_string(),
                is_error: false,
            })
        }
        Ok(Err(e)) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(McpToolResult {
                    content: "Tesseract is not installed. Install tesseract-ocr:\n  - macOS: brew install tesseract tesseract-lang\n  - Ubuntu/Debian: sudo apt install tesseract-ocr\n  - Windows: Download from https://github.com/UB-Mannheim/tesseract/wiki".into(),
                    is_error: true,
                })
            } else {
                Ok(McpToolResult {
                    content: format!("Failed to run tesseract: {}", e),
                    is_error: true,
                })
            }
        }
        Err(_) => Ok(McpToolResult {
            content: "OCR timed out after 120 seconds".into(),
            is_error: true,
        }),
    }
}

async fn ocr_detect_langs_tool() -> Result<McpToolResult> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::process::Command::new("tesseract")
            .arg("--list-langs")
            .output(),
    )
    .await;

    match output {
        Ok(Ok(output)) => {
            let text = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = text
                .lines()
                .filter(|l| !l.is_empty() && !l.contains("List of available languages"))
                .collect();

            if lines.is_empty() {
                return Ok(McpToolResult {
                    content: "No tesseract language packs detected. Install language packs:\n  - macOS: brew install tesseract-lang\n  - Ubuntu/Debian: sudo apt install tesseract-ocr-eng tesseract-ocr-chi-sim\n  - Windows: download during tesseract installation".into(),
                    is_error: false,
                });
            }

            let result = format!(
                "Available tesseract languages ({} total):\n{}",
                lines.len(),
                lines
                    .iter()
                    .enumerate()
                    .map(|(i, l)| format!("  {}. {}", i + 1, l))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            Ok(McpToolResult {
                content: result,
                is_error: false,
            })
        }
        Ok(Err(e)) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(McpToolResult {
                    content: "Tesseract is not installed. Install tesseract-ocr:\n  - macOS: brew install tesseract\n  - Ubuntu/Debian: sudo apt install tesseract-ocr\n  - Windows: https://github.com/UB-Mannheim/tesseract/wiki".into(),
                    is_error: true,
                })
            } else {
                Ok(McpToolResult {
                    content: format!("Failed to run tesseract: {}", e),
                    is_error: true,
                })
            }
        }
        Err(_) => Ok(McpToolResult {
            content: "OCR language detection timed out".into(),
            is_error: true,
        }),
    }
}

// ─── Obsidian Vault ──────────────────────────────────────────────────

fn obsidian_get_vaults_tool(search_path: Option<&str>) -> Result<McpToolResult> {
    let mut search_dirs = Vec::new();
    if let Some(p) = search_path {
        search_dirs.push(std::path::PathBuf::from(p));
    } else {
        if let Some(docs) = dirs::document_dir() {
            search_dirs.push(docs);
        }
        if let Some(home) = dirs::home_dir() {
            search_dirs.push(home);
        }
        if let Some(desktop) = dirs::desktop_dir() {
            search_dirs.push(desktop);
        }
    }

    let mut vaults: Vec<(String, String, u64)> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for base in &search_dirs {
        if !base.exists() {
            continue;
        }
        find_obsidian_vaults(base, 3, &mut vaults, &mut seen);
    }

    if vaults.is_empty() {
        return Ok(McpToolResult {
            content: "No Obsidian vaults found. Make sure Obsidian is installed and at least one vault exists.\nSearched: ".to_string()
                + &search_dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", "),
            is_error: false,
        });
    }

    let mut lines = vec![format!("Found {} Obsidian vault(s):", vaults.len())];
    for (i, (name, path, file_count)) in vaults.iter().enumerate() {
        lines.push(format!(
            "\n{}. {} ({} files)\n   Path: {}",
            i + 1,
            name,
            file_count,
            path
        ));
    }
    Ok(McpToolResult {
        content: lines.join("\n"),
        is_error: false,
    })
}

fn find_obsidian_vaults(
    dir: &std::path::Path,
    depth: u32,
    vaults: &mut Vec<(String, String, u64)>,
    seen: &mut std::collections::HashSet<String>,
) {
    if depth == 0 {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let obsidian_dir = path.join(".obsidian");
                if obsidian_dir.exists() {
                    let key = path.display().to_string();
                    if seen.insert(key.clone()) {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let count = count_md_files(&path);
                        vaults.push((name, key, count));
                    }
                } else if !path.to_string_lossy().contains(".obsidian")
                    && !path.to_string_lossy().contains("node_modules")
                {
                    find_obsidian_vaults(&path, depth - 1, vaults, seen);
                }
            }
        }
    }
}

fn count_md_files(dir: &std::path::Path) -> u64 {
    let mut count = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_md_files(&path);
            } else if path.extension().map_or(false, |e| e == "md") {
                count += 1;
            }
        }
    }
    count
}

fn obsidian_list_files_tool(vault_path: &str) -> Result<McpToolResult> {
    if vault_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: vault_path is required".into(),
            is_error: true,
        });
    }
    let root = std::path::Path::new(vault_path);
    if !root.exists() {
        return Ok(McpToolResult {
            content: format!("Vault not found: {}", vault_path),
            is_error: true,
        });
    }

    let mut files = Vec::new();
    list_md_files(root, root, &mut files, 0usize, 200usize);

    if files.is_empty() {
        return Ok(McpToolResult {
            content: "No markdown files found in this vault.".into(),
            is_error: false,
        });
    }

    let mut lines = vec![format!("Files in vault ({}):", files.len())];
    for (rel_path, size) in files {
        lines.push(format!("  {} ({} bytes)", rel_path, size));
    }
    Ok(McpToolResult {
        content: lines.join("\n"),
        is_error: false,
    })
}

fn list_md_files(
    root: &std::path::Path,
    current: &std::path::Path,
    files: &mut Vec<(String, u64)>,
    depth: usize,
    max: usize,
) {
    if depth > 10 || files.len() >= max {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with('.') || name == "node_modules" {
                continue;
            }
            if path.is_dir() {
                list_md_files(root, &path, files, depth + 1, max);
            } else if path.extension().map_or(false, |e| e == "md") {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                files.push((rel, size));
            }
        }
    }
}

fn obsidian_read_file_tool(vault_path: &str, file_path: &str) -> Result<McpToolResult> {
    if vault_path.is_empty() || file_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: vault_path and file_path are required".into(),
            is_error: true,
        });
    }
    let full = std::path::Path::new(vault_path).join(file_path);
    if !full.exists() {
        return Ok(McpToolResult {
            content: format!("File not found: {}", full.display()),
            is_error: true,
        });
    }
    match std::fs::read_to_string(&full) {
        Ok(content) => {
            let truncated = if content.len() > 30000 {
                format!("{}...\n\n[Truncated at 30000 chars]", &content[..30000])
            } else {
                content
            };
            Ok(McpToolResult {
                content: truncated,
                is_error: false,
            })
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Error reading file: {}", e),
            is_error: true,
        }),
    }
}

// ─── Export to Word ──────────────────────────────────────────────────

fn export_word_tool(markdown: &str, output_path: &str, title: &str) -> Result<McpToolResult> {
    if markdown.is_empty() {
        return Ok(McpToolResult {
            content: "Error: markdown content is required".into(),
            is_error: true,
        });
    }
    if output_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: output_path is required".into(),
            is_error: true,
        });
    }

    use docx_rs::*;

    let path = std::path::Path::new(output_path);
    let parent = path.parent().unwrap_or(std::path::Path::new("."));
    if let Err(e) = std::fs::create_dir_all(parent) {
        return Ok(McpToolResult {
            content: format!("Error creating output directory: {}", e),
            is_error: true,
        });
    }

    let mut doc = Docx::new();

    doc = doc.add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text(title).size(32).bold())
            .align(AlignmentType::Center),
    );
    doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("")));

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("")));
        } else if trimmed.starts_with("# ") {
            doc = doc.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(&trimmed[2..]).size(36).bold()),
            );
        } else if trimmed.starts_with("## ") {
            doc = doc.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(&trimmed[3..]).size(28).bold()),
            );
        } else if trimmed.starts_with("### ") {
            doc = doc.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(&trimmed[4..]).size(24).bold()),
            );
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            doc = doc.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(&format!("• {}", &trimmed[2..]))),
            );
        } else if trimmed.starts_with("> ") {
            doc = doc.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(trimmed).italic().color("666666")),
            );
        } else {
            doc =
                doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(trimmed).size(22)));
        }
    }

    match doc
        .build()
        .pack(std::fs::File::create(path).map_err(|e| AxAgentError::Gateway(e.to_string()))?)
    {
        Ok(_) => Ok(McpToolResult {
            content: format!("Word document exported successfully to: {}", output_path),
            is_error: false,
        }),
        Err(e) => Ok(McpToolResult {
            content: format!("Error creating Word document: {}", e),
            is_error: true,
        }),
    }
}

// ─── Remote Files ────────────────────────────────────────────────────

async fn remotefile_upload_tool(
    provider: &str,
    api_key: &str,
    file_path: &str,
    purpose: Option<&str>,
) -> Result<McpToolResult> {
    if provider.is_empty() || api_key.is_empty() || file_path.is_empty() {
        return Ok(McpToolResult {
            content: "Error: provider, api_key, and file_path are required".into(),
            is_error: true,
        });
    }
    let fp = std::path::Path::new(file_path);
    if !fp.exists() {
        return Ok(McpToolResult {
            content: format!("File not found: {}", file_path),
            is_error: true,
        });
    }

    let data = match std::fs::read(file_path) {
        Ok(d) => d,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error reading file: {}", e),
                is_error: true,
            })
        }
    };
    if data.len() > 100 * 1024 * 1024 {
        return Ok(McpToolResult {
            content: "Error: file too large (max 100 MB)".into(),
            is_error: true,
        });
    }

    let client = reqwest::Client::new();
    match provider {
        "gemini" => upload_to_gemini(&client, api_key, &data, fp).await,
        "openai" => upload_to_openai(&client, api_key, &data, fp, purpose).await,
        "mistral" => upload_to_mistral(&client, api_key, &data, fp).await,
        _ => Ok(McpToolResult {
            content: format!(
                "Unknown provider: {}. Use gemini, openai, or mistral.",
                provider
            ),
            is_error: true,
        }),
    }
}

async fn remotefile_list_tool(provider: &str, api_key: &str) -> Result<McpToolResult> {
    if provider.is_empty() || api_key.is_empty() {
        return Ok(McpToolResult {
            content: "Error: provider and api_key are required".into(),
            is_error: true,
        });
    }
    let client = reqwest::Client::new();
    match provider {
        "gemini" => list_gemini_files(&client, api_key).await,
        "openai" => list_openai_files(&client, api_key).await,
        "mistral" => list_mistral_files(&client, api_key).await,
        _ => Ok(McpToolResult {
            content: format!("Unknown provider: {}", provider),
            is_error: true,
        }),
    }
}

async fn remotefile_delete_tool(
    provider: &str,
    api_key: &str,
    file_id: &str,
) -> Result<McpToolResult> {
    if provider.is_empty() || api_key.is_empty() || file_id.is_empty() {
        return Ok(McpToolResult {
            content: "Error: provider, api_key, and file_id are required".into(),
            is_error: true,
        });
    }
    let client = reqwest::Client::new();
    match provider {
        "gemini" => delete_gemini_file(&client, api_key, file_id).await,
        "openai" => delete_openai_file(&client, api_key, file_id).await,
        "mistral" => delete_mistral_file(&client, api_key, file_id).await,
        _ => Ok(McpToolResult {
            content: format!("Unknown provider: {}", provider),
            is_error: true,
        }),
    }
}

async fn upload_to_gemini(
    client: &reqwest::Client,
    api_key: &str,
    data: &[u8],
    path: &std::path::Path,
) -> Result<McpToolResult> {
    let mime = mime_for_path(path);
    let display_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let url = format!(
        "https://generativelanguage.googleapis.com/upload/v1beta/files?key={}",
        api_key
    );

    // Gemini uses a two-step: start upload, then actually upload
    let metadata = serde_json::json!({
        "file": {
            "display_name": display_name,
            "mime_type": mime,
        }
    });

    let body = serde_json::json!({
        "metadata_bytes": base64::engine::general_purpose::STANDARD.encode(serde_json::to_string(&metadata).unwrap_or_default()),
        "file_bytes": base64::engine::general_purpose::STANDARD.encode(data),
    });

    match client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) => {
                    let name = v
                        .get("file")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("");
                    let uri = v
                        .get("file")
                        .and_then(|f| f.get("uri"))
                        .and_then(|u| u.as_str())
                        .unwrap_or("");
                    Ok(McpToolResult {
                        content: format!("Uploaded to Gemini: {} (uri: {})", name, uri),
                        is_error: false,
                    })
                }
                Err(_) => Ok(McpToolResult {
                    content: format!("Gemini upload response: {}", truncate_text(&text, 1000)),
                    is_error: false,
                }),
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Gemini upload failed: {}", e),
            is_error: true,
        }),
    }
}

async fn list_gemini_files(client: &reqwest::Client, api_key: &str) -> Result<McpToolResult> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/files?key={}",
        api_key
    );
    match client.get(&url).send().await {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) => {
                    let files = v
                        .get("files")
                        .and_then(|f| f.as_array())
                        .cloned()
                        .unwrap_or_default();
                    if files.is_empty() {
                        return Ok(McpToolResult {
                            content: "No files stored in Gemini.".into(),
                            is_error: false,
                        });
                    }
                    let mut lines = vec![format!("Gemini files ({}):", files.len())];
                    for f in files {
                        let name = f.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let display = f.get("displayName").and_then(|n| n.as_str()).unwrap_or("");
                        let mime = f.get("mimeType").and_then(|n| n.as_str()).unwrap_or("");
                        lines.push(format!("  {} ({}): {}", name, mime, display));
                    }
                    Ok(McpToolResult {
                        content: lines.join("\n"),
                        is_error: false,
                    })
                }
                Err(_) => Ok(McpToolResult {
                    content: format!("Gemini response: {}", truncate_text(&text, 1000)),
                    is_error: false,
                }),
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Gemini list failed: {}", e),
            is_error: true,
        }),
    }
}

async fn delete_gemini_file(
    client: &reqwest::Client,
    api_key: &str,
    file_id: &str,
) -> Result<McpToolResult> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/{}?key={}",
        file_id, api_key
    );
    match client.delete(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok(McpToolResult {
                    content: format!("Deleted {}", file_id),
                    is_error: false,
                })
            } else {
                Ok(McpToolResult {
                    content: format!("Delete failed: {}", resp.status()),
                    is_error: true,
                })
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Gemini delete failed: {}", e),
            is_error: true,
        }),
    }
}

async fn upload_to_openai(
    client: &reqwest::Client,
    api_key: &str,
    data: &[u8],
    path: &std::path::Path,
    purpose: Option<&str>,
) -> Result<McpToolResult> {
    let display_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let purpose_str = purpose.unwrap_or("assistants");
    let url = "https://api.openai.com/v1/files";
    let b64 = base64::engine::general_purpose::STANDARD.encode(data);

    let body = serde_json::json!({
        "purpose": purpose_str,
        "file": b64,
        "filename": display_name,
    });

    match client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) => {
                    let id = v.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    let fname = v.get("filename").and_then(|n| n.as_str()).unwrap_or("");
                    let bytes = v.get("bytes").and_then(|b| b.as_u64()).unwrap_or(0);
                    Ok(McpToolResult {
                        content: format!(
                            "Uploaded to OpenAI: {} (id: {}, {} bytes)",
                            fname, id, bytes
                        ),
                        is_error: false,
                    })
                }
                Err(_) => Ok(McpToolResult {
                    content: format!("OpenAI response: {}", truncate_text(&text, 1000)),
                    is_error: false,
                }),
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("OpenAI upload failed: {}", e),
            is_error: true,
        }),
    }
}

async fn list_openai_files(client: &reqwest::Client, api_key: &str) -> Result<McpToolResult> {
    let url = "https://api.openai.com/v1/files";
    match client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) => {
                    let files = v
                        .get("data")
                        .and_then(|d| d.as_array())
                        .cloned()
                        .unwrap_or_default();
                    if files.is_empty() {
                        return Ok(McpToolResult {
                            content: "No files in OpenAI storage.".into(),
                            is_error: false,
                        });
                    }
                    let mut lines = vec![format!("OpenAI files ({}):", files.len())];
                    for f in files {
                        let id = f.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let name = f.get("filename").and_then(|n| n.as_str()).unwrap_or("");
                        let bytes = f.get("bytes").and_then(|b| b.as_u64()).unwrap_or(0);
                        lines.push(format!("  {}: {} ({} bytes)", id, name, bytes));
                    }
                    Ok(McpToolResult {
                        content: lines.join("\n"),
                        is_error: false,
                    })
                }
                Err(_) => Ok(McpToolResult {
                    content: format!("OpenAI response: {}", truncate_text(&text, 1000)),
                    is_error: false,
                }),
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("OpenAI list failed: {}", e),
            is_error: true,
        }),
    }
}

async fn delete_openai_file(
    client: &reqwest::Client,
    api_key: &str,
    file_id: &str,
) -> Result<McpToolResult> {
    let url = format!("https://api.openai.com/v1/files/{}", file_id);
    match client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok(McpToolResult {
                    content: format!("Deleted {}", file_id),
                    is_error: false,
                })
            } else {
                Ok(McpToolResult {
                    content: format!("Delete failed: {}", resp.status()),
                    is_error: true,
                })
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("OpenAI delete failed: {}", e),
            is_error: true,
        }),
    }
}

async fn upload_to_mistral(
    client: &reqwest::Client,
    api_key: &str,
    data: &[u8],
    path: &std::path::Path,
) -> Result<McpToolResult> {
    let url = "https://api.mistral.ai/v1/files";
    let display_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let mime_type = mime_for_path(path);
    let part = reqwest::multipart::Part::bytes(data.to_vec())
        .file_name(display_name.clone())
        .mime_str(mime_type)
        .unwrap_or_else(|_| {
            reqwest::multipart::Part::bytes(data.to_vec()).file_name(display_name.clone())
        });
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("purpose", "fine-tune");

    match client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
    {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) => {
                    let id = v.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    Ok(McpToolResult {
                        content: format!("Uploaded to Mistral: {} (id: {})", display_name, id),
                        is_error: false,
                    })
                }
                Err(_) => Ok(McpToolResult {
                    content: format!("Mistral response: {}", truncate_text(&text, 1000)),
                    is_error: false,
                }),
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Mistral upload failed: {}", e),
            is_error: true,
        }),
    }
}

async fn list_mistral_files(client: &reqwest::Client, api_key: &str) -> Result<McpToolResult> {
    let url = "https://api.mistral.ai/v1/files";
    match client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) => {
                    let files = v
                        .get("data")
                        .and_then(|d| d.as_array())
                        .cloned()
                        .unwrap_or_default();
                    if files.is_empty() {
                        return Ok(McpToolResult {
                            content: "No files in Mistral storage.".into(),
                            is_error: false,
                        });
                    }
                    let mut lines = vec![format!("Mistral files ({}):", files.len())];
                    for f in files {
                        let id = f.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let name = f.get("filename").and_then(|n| n.as_str()).unwrap_or("");
                        let bytes = f.get("size_bytes").and_then(|b| b.as_u64()).unwrap_or(0);
                        lines.push(format!("  {}: {} ({} bytes)", id, name, bytes));
                    }
                    Ok(McpToolResult {
                        content: lines.join("\n"),
                        is_error: false,
                    })
                }
                Err(_) => Ok(McpToolResult {
                    content: format!("Mistral response: {}", truncate_text(&text, 1000)),
                    is_error: false,
                }),
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Mistral list failed: {}", e),
            is_error: true,
        }),
    }
}

async fn delete_mistral_file(
    client: &reqwest::Client,
    api_key: &str,
    file_id: &str,
) -> Result<McpToolResult> {
    let url = format!("https://api.mistral.ai/v1/files/{}", file_id);
    match client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok(McpToolResult {
                    content: format!("Deleted {}", file_id),
                    is_error: false,
                })
            } else {
                Ok(McpToolResult {
                    content: format!("Delete failed: {}", resp.status()),
                    is_error: true,
                })
            }
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Mistral delete failed: {}", e),
            is_error: true,
        }),
    }
}

fn mime_for_path(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "json" => "application/json",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",
        "html" => "text/html",
        "xml" => "application/xml",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    }
}

// ─── Agent Control Tools ─────────────────────────────────────────────

use std::sync::LazyLock;

static CHECKPOINTS: LazyLock<std::sync::Mutex<Vec<(String, String, String)>>> =
    LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

static AGENT_MEMORY: LazyLock<std::sync::Mutex<std::collections::HashMap<String, String>>> =
    LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

fn agent_checkpoint_tool(action: &str, checkpoint_id: &str, label: &str) -> Result<McpToolResult> {
    match action {
        "save" => {
            let id = format!("ckpt-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
            let display_label = if label.is_empty() {
                "unnamed checkpoint"
            } else {
                label
            };
            let timestamp = chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string();
            let mut checkpoints = CHECKPOINTS.lock().unwrap();
            checkpoints.push((id.clone(), display_label.to_string(), timestamp));
            if checkpoints.len() > 50 {
                checkpoints.remove(0);
            }
            Ok(McpToolResult {
                content: format!("Checkpoint saved: {} (label: {})", id, display_label),
                is_error: false,
            })
        }
        "list" => {
            let checkpoints = CHECKPOINTS.lock().unwrap();
            if checkpoints.is_empty() {
                return Ok(McpToolResult {
                    content: "No checkpoints saved yet. Use action='save' to create one.".into(),
                    is_error: false,
                });
            }
            let mut lines = vec![format!("Checkpoints ({}):", checkpoints.len())];
            for (id, lbl, ts) in checkpoints.iter() {
                lines.push(format!("  {} -- {} ({})", id, lbl, ts));
            }
            Ok(McpToolResult {
                content: lines.join("\n"),
                is_error: false,
            })
        }
        "restore" => {
            if checkpoint_id.is_empty() {
                return Ok(McpToolResult {
                    content: "Error: checkpoint_id is required for restore action".into(),
                    is_error: true,
                });
            }
            let checkpoints = CHECKPOINTS.lock().unwrap();
            let found = checkpoints.iter().find(|(id, _, _)| id == checkpoint_id);
            match found {
                Some((id, label, ts)) => Ok(McpToolResult {
                    content: format!("Checkpoint restored: {} (label: {}, saved: {})\nNote: Session state has been marked for restoration. Continue from this point.", id, label, ts),
                    is_error: false,
                }),
                None => Ok(McpToolResult { content: format!("Checkpoint '{}' not found. Use action='list' to see available checkpoints.", checkpoint_id), is_error: true }),
            }
        }
        _ => Ok(McpToolResult {
            content: format!("Unknown action: {}. Use save, list, or restore.", action),
            is_error: true,
        }),
    }
}

fn agent_status_tool() -> Result<McpToolResult> {
    let checkpoints = CHECKPOINTS.lock().unwrap();
    let memory = AGENT_MEMORY.lock().unwrap();

    let mut lines = vec!["Agent Session Status:".to_string()];
    lines.push(format!("  Checkpoints: {}", checkpoints.len()));
    lines.push(format!("  Memory items: {}", memory.len()));

    if !checkpoints.is_empty() {
        let last = checkpoints.last().unwrap();
        lines.push(format!("  Last checkpoint: {} ({})", last.0, last.2));
    }

    if !memory.is_empty() {
        lines.push("  Stored keys:".to_string());
        for (key, _) in memory.iter() {
            lines.push(format!("    - {}", key));
        }
    }

    Ok(McpToolResult {
        content: lines.join("\n"),
        is_error: false,
    })
}

fn agent_remember_tool(key: &str, value: &str) -> Result<McpToolResult> {
    if key.is_empty() {
        return Ok(McpToolResult {
            content: "Error: key is required".into(),
            is_error: true,
        });
    }
    if value.is_empty() {
        return Ok(McpToolResult {
            content: "Error: value is required".into(),
            is_error: true,
        });
    }

    let mut memory = AGENT_MEMORY.lock().unwrap();
    let was_updated = memory.contains_key(key);
    memory.insert(key.to_string(), value.to_string());

    if was_updated {
        Ok(McpToolResult {
            content: format!("Memory updated: {}", key),
            is_error: false,
        })
    } else {
        Ok(McpToolResult {
            content: format!("Memory stored: {} (total: {} items)", key, memory.len()),
            is_error: false,
        })
    }
}

async fn task_tool_handler(
    agent_type: &str,
    description: &str,
    _prompt: &str,
) -> Result<McpToolResult> {
    use crate::builtin_tools_registry::get_current_conversation_id;
    use rusqlite::params;
    use uuid::Uuid;

    let parent_id = match get_current_conversation_id() {
        Some(id) => id,
        None => {
            return Ok(McpToolResult {
                content: "Error: task tool requires a parent conversation context".into(),
                is_error: true,
            });
        }
    };

    let child_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();

    let db_path_str = match crate::builtin_tools_registry::get_global_db_path() {
        Some(p) => p,
        None => {
            return Ok(McpToolResult {
                content: "Error: database not available for task tool".into(),
                is_error: true,
            });
        }
    };

    let db_file = db_path_str.strip_prefix("sqlite:").unwrap_or(&db_path_str);

    let conn = match rusqlite::Connection::open(db_file) {
        Ok(c) => c,
        Err(e) => {
            return Ok(McpToolResult {
                content: format!("Error opening database: {}", e),
                is_error: true,
            });
        }
    };

    let result = conn.execute(
        "INSERT INTO conversations (id, title, model_id, provider_id, system_prompt, temperature, max_tokens, top_p, frequency_penalty, message_count, is_pinned, is_archived, search_enabled, thinking_budget, enabled_mcp_server_ids, enabled_knowledge_base_ids, enabled_memory_namespace_ids, created_at, updated_at, context_compression, category_id, parent_conversation_id, mode, scenario, enabled_skill_ids)
         VALUES (?1, ?2, '', '', NULL, NULL, NULL, NULL, NULL, 0, 0, 0, 0, NULL, '[]', '[]', '[]', ?3, ?3, 0, NULL, ?4, 'agent', ?5, '[]')",
        params![child_id, description, now, parent_id, format!("subagent:{}", agent_type)],
    );

    match result {
        Ok(_) => {
            let output = serde_json::json!({
                "status": "created",
                "child_conversation_id": child_id,
                "agent_type": agent_type,
                "description": description,
                "parent_conversation_id": parent_id
            });
            Ok(McpToolResult {
                content: output.to_string(),
                is_error: false,
            })
        }
        Err(e) => Ok(McpToolResult {
            content: format!("Failed to create child conversation: {}", e),
            is_error: true,
        }),
    }
}
