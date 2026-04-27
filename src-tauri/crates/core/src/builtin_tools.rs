use crate::builtin_tools_registry::{
    get_global_db_path, get_handler, register_builtin_handler, BoxedToolHandler,
};
use crate::error::{AxAgentError, Result};
use crate::mcp_client::McpToolResult;
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
