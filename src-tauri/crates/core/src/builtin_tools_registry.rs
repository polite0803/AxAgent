use crate::error::{AxAgentError, Result};
use crate::mcp_client::McpToolResult;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::LazyLock;

pub type BoxedToolHandlerInner =
    dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<McpToolResult>> + Send>> + Send + Sync;
pub type BoxedToolHandler = Arc<BoxedToolHandlerInner>;

pub struct BuiltinToolDefinition {
    pub tool_name: String,
    pub description: String,
    pub input_schema: Value,
}

pub struct BuiltinServerDefinition {
    pub server_id: String,
    pub server_name: String,
    pub tools: Vec<BuiltinToolDefinition>,
}

pub struct BuiltinDynamicTool {
    pub server_id: String,
    pub server_name: String,
    pub tool_name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Clone)]
pub struct FlatBuiltinTool {
    pub server_id: String,
    pub server_name: String,
    pub tool_name: String,
    pub description: String,
    pub input_schema: Value,
    pub env_json: Option<String>,
    pub timeout_secs: Option<i32>,
}

static BUILTIN_HANDLERS: LazyLock<std::sync::RwLock<HashMap<(String, String), BoxedToolHandler>>> =
    LazyLock::new(|| std::sync::RwLock::new(HashMap::new()));

/// Global database path for builtin tools that need DB access (session_search, memory_flush).
/// Set once at startup via `set_global_db_path()`.
static GLOBAL_DB_PATH: LazyLock<std::sync::RwLock<Option<String>>> =
    LazyLock::new(|| std::sync::RwLock::new(None));

/// Set the global database path for builtin tools. Called once at startup.
pub fn set_global_db_path(path: &str) {
    let mut db_path = GLOBAL_DB_PATH.write().unwrap();
    *db_path = Some(path.to_string());
}

/// Get the global database path for builtin tools.
pub fn get_global_db_path() -> Option<String> {
    let db_path = GLOBAL_DB_PATH.read().unwrap();
    db_path.clone()
}

/// Type alias for a closure that creates and runs a sub-agent session.
/// Takes (provider_id, parent_conversation_id, user_input, agent_type, task_description)
/// and returns (child_conversation_id, result_text).
pub type SubAgentRunner = Arc<
    dyn Fn(
            String, // provider_id
            String, // parent_conversation_id
            String, // user_input
            String, // agent_type
            String, // task_description
        )
            -> Pin<Box<dyn Future<Output = std::result::Result<(String, String), String>> + Send>>
        + Send
        + Sync,
>;

/// Global sub-agent runner. Set once at startup by agent.rs.
static GLOBAL_SUB_AGENT_RUNNER: LazyLock<std::sync::RwLock<Option<SubAgentRunner>>> =
    LazyLock::new(|| std::sync::RwLock::new(None));

/// Set the global sub-agent runner. Called once at startup.
pub fn set_global_sub_agent_runner(runner: SubAgentRunner) {
    let mut r = GLOBAL_SUB_AGENT_RUNNER.write().unwrap();
    *r = Some(runner);
}

/// Get the global sub-agent runner.
pub fn get_global_sub_agent_runner() -> Option<SubAgentRunner> {
    let r = GLOBAL_SUB_AGENT_RUNNER.read().unwrap();
    r.clone()
}

/// Global current conversation ID for tools that need parent context.
static GLOBAL_CURRENT_CONVERSATION_ID: LazyLock<std::sync::RwLock<Option<String>>> =
    LazyLock::new(|| std::sync::RwLock::new(None));

/// Set the current conversation ID, called before each agent turn.
pub fn set_current_conversation_id(id: &str) {
    let mut cid = GLOBAL_CURRENT_CONVERSATION_ID.write().unwrap();
    *cid = Some(id.to_string());
}

/// Get the current conversation ID.
pub fn get_current_conversation_id() -> Option<String> {
    let cid = GLOBAL_CURRENT_CONVERSATION_ID.read().unwrap();
    cid.clone()
}

/// Stores pending sub-agent card data. Key is parent_conversation_id.
/// Value is (child_conversation_id, agent_type, description).
pub type PendingSubAgentCard = (String, String, String); // (child_id, agent_type, description)

static PENDING_SUB_AGENT_CARDS: LazyLock<std::sync::RwLock<HashMap<String, PendingSubAgentCard>>> =
    LazyLock::new(|| std::sync::RwLock::new(HashMap::new()));

/// Store a pending sub-agent card. Called by the task tool handler.
pub fn store_pending_sub_agent_card(
    parent_id: &str,
    child_id: &str,
    agent_type: &str,
    description: &str,
) {
    let mut m = PENDING_SUB_AGENT_CARDS.write().unwrap();
    m.insert(
        parent_id.to_string(),
        (
            child_id.to_string(),
            agent_type.to_string(),
            description.to_string(),
        ),
    );
}

/// Take and remove a pending sub-agent card for the given parent conversation.
pub fn take_pending_sub_agent_card(parent_id: &str) -> Option<PendingSubAgentCard> {
    let mut m = PENDING_SUB_AGENT_CARDS.write().unwrap();
    m.remove(parent_id)
}

pub fn register_builtin_handler(server_name: &str, tool_name: &str, handler: BoxedToolHandler) {
    let mut handlers = BUILTIN_HANDLERS.write().unwrap();
    handlers.insert((server_name.to_string(), tool_name.to_string()), handler);
}

pub fn get_handler(server_name: &str, tool_name: &str) -> Option<BoxedToolHandler> {
    let handlers = BUILTIN_HANDLERS.read().unwrap();
    handlers
        .get(&(server_name.to_string(), tool_name.to_string()))
        .cloned()
}

pub fn list_all_builtin_handlers() -> Vec<(String, String)> {
    let handlers = BUILTIN_HANDLERS.read().unwrap();
    handlers.keys().cloned().collect()
}

pub fn get_all_builtin_server_definitions() -> Vec<BuiltinServerDefinition> {
    vec![
        BuiltinServerDefinition {
            server_id: "builtin-fetch".to_string(),
            server_name: "@axagent/fetch".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "fetch_url".to_string(),
                    description: "Fetch the content of a URL and return it as plain text. Use this when you need to read the content of a webpage.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "The URL to fetch"
                            },
                            "max_length": {
                                "type": "integer",
                                "description": "Maximum number of characters to return",
                                "default": 5000
                            }
                        },
                        "required": ["url"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "fetch_markdown".to_string(),
                    description: "Fetch a URL and convert it to Markdown format. Best for documentation, articles, and technical content.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "The URL to fetch"
                            },
                            "max_length": {
                                "type": "integer",
                                "description": "Maximum number of characters to return",
                                "default": 10000
                            }
                        },
                        "required": ["url"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-search-file".to_string(),
            server_name: "@axagent/search-file".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "read_file".to_string(),
                    description: "Read the entire content of a file from the filesystem. Use this to view source code, configuration files, or any text content.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Absolute or relative path to the file"
                            }
                        },
                        "required": ["path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "list_directory".to_string(),
                    description: "List all files and directories in a given path.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Directory path to list",
                                "default": "."
                            }
                        }
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "search_files".to_string(),
                    description: "Search for files by name pattern in a directory tree.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Root path to search",
                                "default": "."
                            },
                            "pattern": {
                                "type": "string",
                                "description": "Glob pattern to match file names",
                                "default": "*"
                            },
                            "max_results": {
                                "type": "integer",
                                "description": "Maximum number of results",
                                "default": 50
                            }
                        }
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "grep_content".to_string(),
                    description: "Search for a text pattern within file contents across a directory tree. Returns matching lines with file paths and line numbers. Use this to find specific code, text, or patterns inside files.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Root directory to search in",
                                "default": "."
                            },
                            "pattern": {
                                "type": "string",
                                "description": "Text pattern to search for in file contents"
                            },
                            "file_pattern": {
                                "type": "string",
                                "description": "Glob pattern to filter files (e.g. '*.rs', '*.ts')",
                                "default": "*"
                            },
                            "max_results": {
                                "type": "integer",
                                "description": "Maximum number of matching lines to return",
                                "default": 50
                            }
                        },
                        "required": ["pattern"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-filesystem".to_string(),
            server_name: "@axagent/filesystem".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "write_file".to_string(),
                    description: "Write content to a file, creating it if it doesn't exist or overwriting if it does.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path to write to"
                            },
                            "content": {
                                "type": "string",
                                "description": "Content to write to the file"
                            }
                        },
                        "required": ["path", "content"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "edit_file".to_string(),
                    description: "Edit a file by replacing the first occurrence of old_str with new_str.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path to edit"
                            },
                            "old_str": {
                                "type": "string",
                                "description": "String to find and replace"
                            },
                            "new_str": {
                                "type": "string",
                                "description": "Replacement string"
                            }
                        },
                        "required": ["path", "old_str", "new_str"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "delete_file".to_string(),
                    description: "Delete a file from the filesystem.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path to delete"
                            }
                        },
                        "required": ["path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "create_directory".to_string(),
                    description: "Create a directory and all parent directories if they don't exist.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Directory path to create"
                            }
                        },
                        "required": ["path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "file_exists".to_string(),
                    description: "Check if a file or directory exists at the given path.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to check"
                            }
                        },
                        "required": ["path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "get_file_info".to_string(),
                    description: "Get information about a file including size, permissions, and modification time.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path to get info about"
                            }
                        },
                        "required": ["path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "move_file".to_string(),
                    description: "Move or rename a file or directory. The source path is renamed to the destination path. This can move files across directories or simply rename them.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "source": {
                                "type": "string",
                                "description": "Source file or directory path"
                            },
                            "destination": {
                                "type": "string",
                                "description": "Destination file or directory path"
                            }
                        },
                        "required": ["source", "destination"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-system".to_string(),
            server_name: "@axagent/system".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "run_command".to_string(),
                    description: "Execute a shell command and return its output.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "Shell command to execute"
                            },
                            "timeout_secs": {
                                "type": "integer",
                                "description": "Timeout in seconds for command execution",
                                "default": 30
                            }
                        },
                        "required": ["command"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "get_system_info".to_string(),
                    description: "Get information about the system including OS, architecture, home directory, and uptime.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "list_processes".to_string(),
                    description: "List running processes on the system.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "limit": {
                                "type": "integer",
                                "description": "Maximum number of processes to return",
                                "default": 20
                            }
                        }
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-knowledge".to_string(),
            server_name: "@axagent/knowledge".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "list_knowledge_bases".to_string(),
                    description: "List all available knowledge bases for the knowledge retrieval system.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "search_knowledge".to_string(),
                    description: "Search a knowledge base for relevant documents using vector similarity.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "base_id": {
                                "type": "string",
                                "description": "Knowledge base ID to search in"
                            },
                            "query": {
                                "type": "string",
                                "description": "Search query string"
                            },
                            "top_k": {
                                "type": "integer",
                                "description": "Number of top results to return",
                                "default": 5
                            }
                        },
                        "required": ["base_id", "query"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "create_knowledge_entity".to_string(),
                    description: "Create a knowledge graph entity (service, component, module) in a knowledge base. Entities are the fundamental units of the knowledge graph. Use this to document system components, services, or modules discovered during analysis.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "knowledge_base_id": {"type": "string", "description": "Knowledge base ID"},
                            "name": {"type": "string", "description": "Entity name"},
                            "entity_type": {"type": "string", "description": "Type: service, component, module, class, function, etc."},
                            "description": {"type": "string", "description": "Description of the entity"},
                            "source_path": {"type": "string", "description": "Source file path"},
                            "source_language": {"type": "string", "description": "Programming language"},
                            "properties": {"type": "object", "description": "Custom properties"},
                            "lifecycle": {"type": "object", "description": "Lifecycle stages"},
                            "behaviors": {"type": "object", "description": "Behavior descriptions"}
                        },
                        "required": ["knowledge_base_id", "name", "entity_type"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "create_knowledge_flow".to_string(),
                    description: "Create a knowledge graph flow (process, pipeline, workflow) in a knowledge base. Flows describe how data, control, or work moves through the system. Use this to document business processes, data pipelines, or request workflows.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "knowledge_base_id": {"type": "string", "description": "Knowledge base ID"},
                            "name": {"type": "string", "description": "Flow name"},
                            "flow_type": {"type": "string", "description": "Type: process, pipeline, workflow, data_flow, control_flow"},
                            "description": {"type": "string", "description": "Description of the flow"},
                            "source_path": {"type": "string", "description": "Source file that implements this flow"},
                            "steps": {"type": "object", "description": "Flow steps as JSON array"},
                            "decision_points": {"type": "object", "description": "Decision points in the flow"},
                            "error_handling": {"type": "object", "description": "Error handling patterns"},
                            "preconditions": {"type": "object", "description": "Pre-conditions"},
                            "postconditions": {"type": "object", "description": "Post-conditions"}
                        },
                        "required": ["knowledge_base_id", "name", "flow_type"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "create_knowledge_interface".to_string(),
                    description: "Create a knowledge graph interface (API, protocol, contract) in a knowledge base. Interfaces define how components communicate. Use this to document REST APIs, gRPC services, message formats, or inter-component contracts.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "knowledge_base_id": {"type": "string", "description": "Knowledge base ID"},
                            "name": {"type": "string", "description": "Interface name"},
                            "interface_type": {"type": "string", "description": "Type: api, rpc, message, contract, event"},
                            "description": {"type": "string", "description": "Description of the interface"},
                            "source_path": {"type": "string", "description": "Source file that defines this interface"},
                            "input_schema": {"type": "object", "description": "Input schema"},
                            "output_schema": {"type": "object", "description": "Output schema"},
                            "error_codes": {"type": "object", "description": "Error codes and messages"},
                            "communication_pattern": {"type": "string", "description": "Pattern: sync, async, pubsub, polling, streaming"}
                        },
                        "required": ["knowledge_base_id", "name", "interface_type"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "add_knowledge_document".to_string(),
                    description: "Add a document to a knowledge base for indexing and later retrieval. Documents are chunked, embedded, and stored for semantic search. Use this to add documentation, notes, or reference material to the knowledge base.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "knowledge_base_id": {"type": "string", "description": "Knowledge base ID"},
                            "title": {"type": "string", "description": "Document title"},
                            "content": {"type": "string", "description": "Document content (markdown supported)"}
                        },
                        "required": ["knowledge_base_id", "title", "content"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-storage".to_string(),
            server_name: "@axagent/storage".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "get_storage_info".to_string(),
                    description: "Get information about AxAgent's documents storage including total and used space.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "list_storage_files".to_string(),
                    description: "List files in AxAgent's documents storage.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Subdirectory path within documents root",
                                "default": ""
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Maximum number of files to return",
                                "default": 50
                            }
                        }
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "upload_storage_file".to_string(),
                    description: "Upload a file to AxAgent's documents storage. Content should be base64 encoded.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "filename": {
                                "type": "string",
                                "description": "Name of the file to create"
                            },
                            "content_base64": {
                                "type": "string",
                                "description": "File content encoded as base64"
                            },
                            "bucket": {
                                "type": "string",
                                "description": "Storage subdirectory: images, files, or backups",
                                "default": ""
                            }
                        },
                        "required": ["filename", "content_base64"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "download_storage_file".to_string(),
                    description: "Download a file from AxAgent's documents storage. Returns content as base64.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the file within documents storage"
                            }
                        },
                        "required": ["path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "delete_storage_file".to_string(),
                    description: "Delete a file from AxAgent's documents storage.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the file within documents storage"
                            }
                        },
                        "required": ["path"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-computer-control".to_string(),
            server_name: "@axagent/computer-control".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "screen_capture".to_string(),
                    description: "Capture a screenshot of the screen, a specific region, or a window. Returns a base64-encoded PNG image.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "monitor": {
                                "type": "integer",
                                "description": "Monitor index (default: 0)"
                            },
                            "region": {
                                "type": "object",
                                "description": "Capture region (optional)",
                                "properties": {
                                    "x": { "type": "integer" },
                                    "y": { "type": "integer" },
                                    "width": { "type": "integer" },
                                    "height": { "type": "integer" }
                                }
                            },
                            "window_title": {
                                "type": "string",
                                "description": "Capture specific window by title (optional)"
                            }
                        }
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "find_ui_elements".to_string(),
                    description: "Find accessible UI elements on screen using accessibility APIs. Returns element role, name, bounds, and interactivity.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "role": {
                                "type": "string",
                                "description": "Element role filter (button, text, link, input, etc.)"
                            },
                            "name_contains": {
                                "type": "string",
                                "description": "Filter by element name"
                            },
                            "application": {
                                "type": "string",
                                "description": "Filter by application name"
                            },
                            "window_title": {
                                "type": "string",
                                "description": "Filter by window title"
                            }
                        }
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "mouse_click".to_string(),
                    description: "Click at specified screen coordinates.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "x": { "type": "number", "description": "X coordinate" },
                            "y": { "type": "number", "description": "Y coordinate" },
                            "button": {
                                "type": "string",
                                "enum": ["left", "right", "middle"],
                                "description": "Mouse button (default: left)"
                            }
                        },
                        "required": ["x", "y"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "type_text".to_string(),
                    description: "Type text at the current cursor position or at specified coordinates.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "text": { "type": "string", "description": "Text to type" },
                            "x": { "type": "number", "description": "Click at X before typing (optional)" },
                            "y": { "type": "number", "description": "Click at Y before typing (optional)" }
                        },
                        "required": ["text"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "press_key".to_string(),
                    description: "Press a keyboard key with optional modifiers (Ctrl, Alt, Shift).".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "key": { "type": "string", "description": "Key to press (e.g., 'Enter', 'Tab', 'a', 'F1')" },
                            "modifiers": {
                                "type": "array",
                                "items": { "type": "string", "enum": ["alt", "control", "shift", "super"] },
                                "description": "Key modifiers"
                            }
                        },
                        "required": ["key"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "mouse_scroll".to_string(),
                    description: "Scroll at specified screen coordinates.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "x": { "type": "number", "description": "X coordinate" },
                            "y": { "type": "number", "description": "Y coordinate" },
                            "delta": { "type": "integer", "description": "Scroll amount (positive=up, negative=down)" }
                        },
                        "required": ["x", "y", "delta"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-browser".to_string(),
            server_name: "@axagent/browser".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "browser_navigate".to_string(),
                    description: "Navigate to a URL in the browser. Returns current URL and page title.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "URL to navigate to"
                            }
                        },
                        "required": ["url"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "browser_screenshot".to_string(),
                    description: "Take a screenshot of the current browser page. Returns base64-encoded PNG.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "full_page": {
                                "type": "boolean",
                                "description": "Capture full page (default: false)"
                            }
                        }
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "browser_click".to_string(),
                    description: "Click an element identified by CSS selector.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "selector": {
                                "type": "string",
                                "description": "CSS selector for the element"
                            }
                        },
                        "required": ["selector"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "browser_fill".to_string(),
                    description: "Fill an input field with text.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "selector": {
                                "type": "string",
                                "description": "CSS selector for the input field"
                            },
                            "value": {
                                "type": "string",
                                "description": "Text to fill"
                            }
                        },
                        "required": ["selector", "value"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "browser_type".to_string(),
                    description: "Type text into an element character by character.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "selector": {
                                "type": "string",
                                "description": "CSS selector for the element"
                            },
                            "text": {
                                "type": "string",
                                "description": "Text to type"
                            }
                        },
                        "required": ["selector", "text"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "browser_extract_text".to_string(),
                    description: "Extract text content from an element.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "selector": {
                                "type": "string",
                                "description": "CSS selector for the element"
                            }
                        },
                        "required": ["selector"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "browser_extract_all".to_string(),
                    description: "Extract all elements matching a selector with their attributes.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "selector": {
                                "type": "string",
                                "description": "CSS selector"
                            }
                        },
                        "required": ["selector"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "browser_get_content".to_string(),
                    description: "Get the full HTML content of the current page.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "browser_select".to_string(),
                    description: "Select an option in a dropdown/select element.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "selector": {
                                "type": "string",
                                "description": "CSS selector for the select element"
                            },
                            "value": {
                                "type": "string",
                                "description": "Option value to select"
                            }
                        },
                        "required": ["selector", "value"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "browser_wait_for".to_string(),
                    description: "Wait for an element to appear on the page.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "selector": {
                                "type": "string",
                                "description": "CSS selector to wait for"
                            },
                            "timeout": {
                                "type": "integer",
                                "description": "Timeout in milliseconds (default: 10000)"
                            }
                        },
                        "required": ["selector"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-brave-search".to_string(),
            server_name: "@axagent/brave-search".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "brave_web_search".to_string(),
                    description: "Search the web using the Brave Search API. Returns web pages, articles, and information matching your query. Use for general web searches, finding documentation, or researching topics.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search query string"
                            },
                            "count": {
                                "type": "integer",
                                "description": "Number of results to return (default: 10, max: 20)",
                                "default": 10
                            }
                        },
                        "required": ["query"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "brave_local_search".to_string(),
                    description: "Search for local businesses and places using the Brave Search API. Use for finding restaurants, stores, services, or any physical locations.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search query for local places"
                            },
                            "count": {
                                "type": "integer",
                                "description": "Number of results to return (default: 5)",
                                "default": 5
                            }
                        },
                        "required": ["query"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-sequential-thinking".to_string(),
            server_name: "@axagent/sequential-thinking".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "sequentialthinking".to_string(),
                    description: "A detailed tool for dynamic and reflective problem-solving through thoughts. This tool helps analyze problems through a flexible thinking process that can adapt and evolve. Each thought can build on, question, or revise previous insights as understanding deepens. Use this tool for complex problems requiring step-by-step reasoning, breaking down tasks, and structured analysis.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "thought": {
                                "type": "string",
                                "description": "Your current thinking step"
                            },
                            "nextThoughtNeeded": {
                                "type": "boolean",
                                "description": "Whether another thought step is needed"
                            },
                            "thoughtNumber": {
                                "type": "integer",
                                "description": "Current thought number",
                                "minimum": 1
                            },
                            "totalThoughts": {
                                "type": "integer",
                                "description": "Estimated total thoughts needed",
                                "minimum": 1
                            },
                            "isRevision": {
                                "type": "boolean",
                                "description": "Whether this revises a previous thought"
                            },
                            "revisesThought": {
                                "type": "integer",
                                "description": "Which thought number is being revised",
                                "minimum": 1
                            },
                            "branchFromThought": {
                                "type": "integer",
                                "description": "Branching point thought number",
                                "minimum": 1
                            },
                            "branchId": {
                                "type": "string",
                                "description": "Branch identifier"
                            },
                            "needsMoreThoughts": {
                                "type": "boolean",
                                "description": "Whether more thoughts are needed"
                            }
                        },
                        "required": ["thought", "nextThoughtNeeded", "thoughtNumber", "totalThoughts"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-python".to_string(),
            server_name: "@axagent/python".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "python_execute".to_string(),
                    description: "Execute a Python script in a sandboxed environment. Use this for computations, data processing, or running Python code. Returns combined stdout and stderr output.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "script": {
                                "type": "string",
                                "description": "Python script to execute"
                            },
                            "timeout": {
                                "type": "integer",
                                "description": "Timeout in seconds (default: 30, max: 120)",
                                "default": 30,
                                "minimum": 1,
                                "maximum": 120
                            }
                        },
                        "required": ["script"]
                    }),
                },
            ],
        },

        BuiltinServerDefinition {
            server_id: "builtin-dify-knowledge".to_string(),
            server_name: "@axagent/dify-knowledge".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "dify_list_bases".to_string(),
                    description: "List all available knowledge bases from a Dify instance. Use this to discover what datasets are available for searching.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "api_base": {
                                "type": "string",
                                "description": "Dify API base URL (e.g. https://api.dify.ai/v1)"
                            },
                            "api_key": {
                                "type": "string",
                                "description": "Dify API key (dataset API permission required)"
                            }
                        },
                        "required": ["api_base", "api_key"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "dify_search".to_string(),
                    description: "Search a Dify knowledge base for relevant documents using semantic search. Returns chunk content, relevance scores, and source document info.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "api_base": {
                                "type": "string",
                                "description": "Dify API base URL"
                            },
                            "api_key": {
                                "type": "string",
                                "description": "Dify API key"
                            },
                            "dataset_id": {
                                "type": "string",
                                "description": "Knowledge base (dataset) ID to search"
                            },
                            "query": {
                                "type": "string",
                                "description": "Search query string"
                            },
                            "top_k": {
                                "type": "integer",
                                "description": "Number of results to return",
                                "default": 5
                            }
                        },
                        "required": ["api_base", "api_key", "dataset_id", "query"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-workspace-memory".to_string(),
            server_name: "@axagent/workspace-memory".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "workspace_read".to_string(),
                    description: "Read a memory file from the agent workspace. This provides access to persistent workspace-level memory (e.g. FACT.md for facts, SUMMARY.md for summaries, journal entries). Use this to recall previously stored information about the workspace.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "filename": {
                                "type": "string",
                                "description": "Memory filename to read (default: FACT.md). Other options: SUMMARY.md, journal.md, decisions.md",
                                "default": "FACT.md"
                            },
                            "workspace_path": {
                                "type": "string",
                                "description": "Absolute path to the workspace directory"
                            }
                        },
                        "required": ["workspace_path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "workspace_write".to_string(),
                    description: "Write or append content to a memory file in the agent workspace. Use to persist important facts, decisions, preferences, or project context across agent sessions. Use mode='append' to add new information while preserving existing content (recommended). Use mode='overwrite' to replace the entire file.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "filename": {
                                "type": "string",
                                "description": "Memory filename (default: FACT.md). Use FACT.md for general facts, decisions.md for decisions, journal.md for dated entries.",
                                "default": "FACT.md"
                            },
                            "workspace_path": {
                                "type": "string",
                                "description": "Absolute path to the workspace directory"
                            },
                            "content": {
                                "type": "string",
                                "description": "Content to write or append to the memory file"
                            },
                            "mode": {
                                "type": "string",
                                "enum": ["overwrite", "append"],
                                "description": "Write mode: 'append' adds to end of file, 'overwrite' replaces entire file",
                                "default": "append"
                            }
                        },
                        "required": ["workspace_path", "content"]
                    }),
                },
            ],
        },


        BuiltinServerDefinition {
            server_id: "builtin-file-utils".to_string(),
            server_name: "@axagent/file-utils".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "pdf_info".to_string(),
                    description: "Extract metadata and text content from a PDF file. Returns page count and a text content preview. Use this to inspect PDF documents before processing them.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "file_path": {"type": "string", "description": "Absolute path to the PDF file"}
                        },
                        "required": ["file_path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "detect_encoding".to_string(),
                    description: "Detect the text encoding of a file by checking BOM markers and validating UTF-8. Returns the detected encoding and a text preview. Useful when reading files of unknown encoding.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "file_path": {"type": "string", "description": "Absolute path to the file to analyze"}
                        },
                        "required": ["file_path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "base64_image".to_string(),
                    description: "Read an image file and return it as base64-encoded data with MIME type detection. Supports PNG, JPEG, GIF, WebP, BMP, SVG, ICO, and TIFF. Use this when you need to pass an image to a vision model.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "file_path": {"type": "string", "description": "Absolute path to the image file"}
                        },
                        "required": ["file_path"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-cache".to_string(),
            server_name: "@axagent/cache".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "cache_info".to_string(),
                    description: "Get detailed information about application caches. Reports total cache size, file counts, and cache directory locations. Use to check disk usage of cached data.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "cache_clear".to_string(),
                    description: "Clear application caches to free disk space. Use 'all' to clear everything, or 'temp' to clear only temporary files.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "cache_type": {
                                "type": "string",
                                "enum": ["all", "temp"],
                                "description": "Cache type to clear (default: all)",
                                "default": "all"
                            }
                        }
                    }),
                },
            ],
        },


        BuiltinServerDefinition {
            server_id: "builtin-ocr".to_string(),
            server_name: "@axagent/ocr".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "ocr_image".to_string(),
                    description: "Extract text from an image file using Tesseract OCR. Supports PNG, JPEG, TIFF, BMP, and other formats supported by tesseract. Requires tesseract-ocr to be installed on the system. Use this to read text from screenshots, scanned documents, or any image containing text.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Absolute path to the image file to OCR"
                            },
                            "lang": {
                                "type": "string",
                                "description": "Tesseract language code (default: eng, chi_sim for Chinese, jpn for Japanese). Use ocr_detect_langs to see available languages."
                            }
                        },
                        "required": ["file_path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "ocr_detect_langs".to_string(),
                    description: "List all available language packs installed for Tesseract OCR. Returns language codes (e.g., eng, chi_sim, fra) suitable for use with the ocr_image tool.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
            ],
        },


        BuiltinServerDefinition {
            server_id: "builtin-obsidian".to_string(),
            server_name: "@axagent/obsidian".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "obsidian_get_vaults".to_string(),
                    description: "Find all Obsidian vaults on this system. Searches common locations (Documents, home directory) for .obsidian folders and returns vault paths and names.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "search_path": {"type": "string", "description": "Optional custom search path"}
                        }
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "obsidian_list_files".to_string(),
                    description: "List all Markdown (.md) files in an Obsidian vault directory. Returns file names, relative paths, and sizes.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "vault_path": {"type": "string", "description": "Absolute path to vault root"}
                        },
                        "required": ["vault_path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "obsidian_read_file".to_string(),
                    description: "Read a markdown file from an Obsidian vault. Handles Obsidian Wikilinks ([[links]]) and returns clean markdown content.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "vault_path": {"type": "string", "description": "Vault root path"},
                            "file_path": {"type": "string", "description": "Relative path to the file within the vault"}
                        },
                        "required": ["vault_path", "file_path"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-export".to_string(),
            server_name: "@axagent/export".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "export_word".to_string(),
                    description: "Export markdown content as a Microsoft Word (.docx) document. Creates a properly formatted document with headings, paragraphs, and text styling. Use this when the user asks to save content as a Word file.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "markdown": {"type": "string", "description": "Markdown content to export"},
                            "output_path": {"type": "string", "description": "Output file path (.docx)"},
                            "title": {"type": "string", "description": "Optional document title"}
                        },
                        "required": ["markdown", "output_path"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-remotefile".to_string(),
            server_name: "@axagent/remotefile".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "remotefile_upload".to_string(),
                    description: "Upload a local file to an AI provider's file service (Gemini Files API, OpenAI Files API, or Mistral Files API). The uploaded file can then be referenced by the AI model. Returns the remote file ID and metadata.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "provider": {"type": "string", "enum": ["gemini", "openai", "mistral"], "description": "AI provider"},
                            "api_key": {"type": "string", "description": "API key for the provider"},
                            "file_path": {"type": "string", "description": "Local file path to upload"},
                            "purpose": {"type": "string", "description": "File purpose for OpenAI (e.g., 'assistants', 'fine-tune')"}
                        },
                        "required": ["provider", "api_key", "file_path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "remotefile_list".to_string(),
                    description: "List all files stored in a remote AI provider's file service. Returns file IDs, names, sizes, and upload dates.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "provider": {"type": "string", "enum": ["gemini", "openai", "mistral"], "description": "AI provider"},
                            "api_key": {"type": "string", "description": "API key for the provider"}
                        },
                        "required": ["provider", "api_key"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "remotefile_delete".to_string(),
                    description: "Delete a file from a remote AI provider's file service using its file ID.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "provider": {"type": "string", "enum": ["gemini", "openai", "mistral"], "description": "AI provider"},
                            "api_key": {"type": "string", "description": "API key for the provider"},
                            "file_id": {"type": "string", "description": "Remote file ID to delete"}
                        },
                        "required": ["provider", "api_key", "file_id"]
                    }),
                },
            ],
        },


        BuiltinServerDefinition {
            server_id: "builtin-agent-control".to_string(),
            server_name: "@axagent/agent-control".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "agent_checkpoint".to_string(),
                    description: "Create and manage checkpoints during agent execution. Use 'save' to snapshot current progress during complex tasks, 'list' to see available checkpoints, and 'restore' to resume from a saved checkpoint if interrupted or to explore alternative approaches.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "action": {
                                "type": "string",
                                "enum": ["save", "list", "restore"],
                                "description": "Action: save a new checkpoint, list existing checkpoints, or restore from a previous one"
                            },
                            "checkpoint_id": {
                                "type": "string",
                                "description": "Checkpoint ID (required for restore action)"
                            },
                            "label": {
                                "type": "string",
                                "description": "Human-readable label describing the checkpoint state"
                            }
                        },
                        "required": ["action"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "agent_status".to_string(),
                    description: "Get the current execution status of the agent. Reports running tasks, completed tool calls, encountered errors, session duration, and resource usage. Use this to self-monitor progress during long-running operations.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "agent_remember".to_string(),
                    description: "Store a key-value pair in the agent's session-level memory. This persists across tool calls within the same session. Use for tracking task context, user preferences, important findings, decisions made, or work-in-progress. Retrieve with memory_flush or by reading back with the same key.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "key": {
                                "type": "string",
                                "description": "A descriptive key for the memory item (e.g., 'user_preference', 'task_context', 'findings', 'current_step')"
                            },
                            "value": {
                                "type": "string",
                                "description": "The value to store. Can be a plain string or a JSON-encoded object."
                            }
                        },
                        "required": ["key", "value"]
                    }),
                },
            ],
        },


        BuiltinServerDefinition {
            server_id: "builtin-memory".to_string(),
            server_name: "@axagent/memory".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "memory_flush".to_string(),
                    description: "Persist an important insight to long-term memory. Use when: task completed and you learned something worth remembering, discovered a user preference, solved a non-trivial error, or identified a reusable pattern.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "content": {"type": "string", "description": "The insight, decision, or observation to persist."},
                            "target": {"type": "string", "enum": ["memory", "user"], "description": "'memory' for system-level insights, 'user' for user preferences.", "default": "memory"},
                            "category": {"type": "string", "enum": ["insight", "decision", "error_solution", "preference", "pattern", "workflow"], "description": "Category of the memory item.", "default": "insight"}
                        },
                        "required": ["content"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-image-gen".to_string(),
            server_name: "@axagent/image-gen".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "generate_image".to_string(),
                    description: "Generate an image from a text prompt using AI. Supports Flux and DALL-E providers. Use this when the user asks to create or visualize an image based on a description.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "prompt": {"type": "string", "description": "Text description of the image to generate"},
                            "negative_prompt": {"type": "string", "description": "What to avoid in the image"},
                            "provider": {"type": "string", "enum": ["flux", "dall-e"], "description": "Image generation provider (default: flux)"},
                            "width": {"type": "integer", "description": "Image width"},
                            "height": {"type": "integer", "description": "Image height"},
                            "steps": {"type": "integer", "description": "Number of diffusion steps (Flux only)"},
                            "seed": {"type": "integer", "description": "Random seed for reproducibility"},
                            "api_key": {"type": "string", "description": "API key for the provider"}
                        },
                        "required": ["prompt"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-chart-gen".to_string(),
            server_name: "@axagent/chart-gen".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "generate_chart_config".to_string(),
                    description: "Generate an ECharts configuration from a natural language description. Supports line, bar, pie, scatter, heatmap, radar, treemap, sankey, funnel, and gauge charts. Use this to create data visualizations from descriptions.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "description": {"type": "string", "description": "Natural language description of the chart"},
                            "data": {"type": "object", "description": "Optional data to visualize"},
                            "chart_type": {"type": "string", "enum": ["line", "bar", "pie", "scatter", "heatmap", "radar", "treemap", "sankey", "funnel", "gauge"], "description": "Desired chart type"},
                            "title": {"type": "string", "description": "Chart title"},
                            "api_key": {"type": "string", "description": "API key for the LLM"},
                            "base_url": {"type": "string", "description": "Base URL for the LLM API"},
                            "model": {"type": "string", "description": "Model to use"}
                        },
                        "required": ["description"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-code-edit".to_string(),
            server_name: "@axagent/code-edit".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "search_replace".to_string(),
                    description: "Perform precise search-and-replace edits on text files. Supports optional line range constraints and replace-all mode. Use this for targeted code modifications instead of rewriting entire files.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Absolute path to the file to edit"},
                            "old_str": {"type": "string", "description": "The exact text to search for"},
                            "new_str": {"type": "string", "description": "The text to replace old_str with"},
                            "start_line": {"type": "integer", "description": "Starting line number (1-based)"},
                            "end_line": {"type": "integer", "description": "Ending line number (1-based, inclusive)"},
                            "replace_all": {"type": "boolean", "description": "Replace all occurrences", "default": false}
                        },
                        "required": ["path", "old_str", "new_str"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-git".to_string(),
            server_name: "@axagent/git".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "git_status".to_string(),
                    description: "Get the current git status of a repository. Shows staged, unstaged, and untracked files.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "repo_path": {"type": "string", "description": "Absolute path to the git repository"}
                        },
                        "required": ["repo_path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "git_diff".to_string(),
                    description: "Get a summary of staged or branch changes. Returns files changed, insertions, deletions, and hunk details.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "repo_path": {"type": "string", "description": "Absolute path to the git repository"},
                            "base_branch": {"type": "string", "description": "Show diff between this branch and HEAD"}
                        },
                        "required": ["repo_path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "git_commit".to_string(),
                    description: "Stage all changes and commit them with the given message. Use after reviewing changes to create a commit.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "repo_path": {"type": "string", "description": "Absolute path to the git repository"},
                            "message": {"type": "string", "description": "Commit message"},
                            "stage_all": {"type": "boolean", "description": "Stage all changes before committing", "default": true}
                        },
                        "required": ["repo_path", "message"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "git_log".to_string(),
                    description: "Get recent git commit history. Returns commit hash, author, date, and subject.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "repo_path": {"type": "string", "description": "Absolute path to the git repository"},
                            "max_count": {"type": "integer", "description": "Maximum number of commits", "default": 10}
                        },
                        "required": ["repo_path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "git_branch".to_string(),
                    description: "List all branches in the repository, or create/switch branches.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "repo_path": {"type": "string", "description": "Absolute path to the git repository"},
                            "action": {"type": "string", "enum": ["list", "create", "switch"], "description": "Action to perform", "default": "list"},
                            "name": {"type": "string", "description": "Branch name (for create and switch)"}
                        },
                        "required": ["repo_path"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "git_review".to_string(),
                    description: "Generate a context summary of staged changes for code review. Returns diff summary, file list, and change statistics.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "repo_path": {"type": "string", "description": "Absolute path to the git repository"},
                            "base_branch": {"type": "string", "description": "Review changes between this branch and HEAD"}
                        },
                        "required": ["repo_path"]
                    }),
                },
            ],
        },
        BuiltinServerDefinition {
            server_id: "builtin-cron".to_string(),
            server_name: "@axagent/cron".to_string(),
            tools: vec![
                BuiltinToolDefinition {
                    tool_name: "cron_add".to_string(),
                    description: "Schedule a new recurring cron job. Define the task name, cron schedule expression, and the prompt that will be executed periodically.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "name": {"type": "string", "description": "Job name"},
                            "schedule": {"type": "string", "description": "Cron expression (e.g., '0 9 * * *' for daily at 9am)"},
                            "prompt": {"type": "string", "description": "The prompt/task to execute"}
                        },
                        "required": ["name", "schedule", "prompt"]
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "cron_list".to_string(),
                    description: "List all scheduled cron jobs. Returns job IDs, names, schedules, and next execution times.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                BuiltinToolDefinition {
                    tool_name: "cron_delete".to_string(),
                    description: "Delete a scheduled cron job by its ID. The job will be permanently removed and will no longer execute.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "id": {"type": "string", "description": "Job ID to delete"}
                        },
                        "required": ["id"]
                    }),
                },
            ],
        },

    ]
}

pub fn get_dynamic_builtin_tools() -> std::collections::BTreeMap<String, BuiltinDynamicTool> {
    let mut tools = std::collections::BTreeMap::new();

    tools.insert(
        "skill_manage".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-skills".to_string(),
            server_name: "@axagent/skills".to_string(),
            tool_name: "skill_manage".to_string(),
            description: "Manage self-evolution skills. Create when: complex task succeeded (5+ tool calls), errors overcome, user-corrected approach worked, non-trivial workflow discovered, or user asks you to remember a procedure.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["create", "patch", "edit", "list", "view", "delete"],
                        "description": "Action to perform"
                    },
                    "name": {
                        "type": "string",
                        "description": "Skill name (kebab-case). Required except for 'list'."
                    },
                    "description": {
                        "type": "string",
                        "description": "Short description. Used only with action='create'."
                    },
                    "content": {
                        "type": "string",
                        "description": "Skill content in Markdown."
                    },
                    "skills_dir": {
                        "type": "string",
                        "description": "Custom skills directory path."
                    }
                },
                "required": ["action"]
            }),
        },
    );

    tools.insert(
        "session_search".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-session".to_string(),
            server_name: "@axagent/session".to_string(),
            tool_name: "session_search".to_string(),
            description: "Search past conversations using full-text search. Use to recall how similar problems were solved before.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query using FTS5 syntax"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results",
                        "default": 10
                    },
                    "db_path": {
                        "type": "string",
                        "description": "Custom database path"
                    }
                },
                "required": ["query"]
            }),
        },
    );

    tools.insert(
        "memory_flush".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-memory".to_string(),
            server_name: "@axagent/memory".to_string(),
            tool_name: "memory_flush".to_string(),
            description: "Persist an important insight to long-term memory. Use when: task completed and you learned something worth remembering, discovered a user preference, solved a non-trivial error, or identified a reusable pattern.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The insight, decision, or observation to persist."
                    },
                    "target": {
                        "type": "string",
                        "enum": ["memory", "user"],
                        "description": "'memory' for system-level insights, 'user' for user preferences.",
                        "default": "memory"
                    },
                    "category": {
                        "type": "string",
                        "enum": ["insight", "decision", "error_solution", "preference", "pattern", "workflow"],
                        "description": "Category of the memory item.",
                        "default": "insight"
                    }
                },
                "required": ["content"]
            }),
        },
    );

    tools.insert(
        "web_search".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-search".to_string(),
            server_name: "@axagent/search".to_string(),
            tool_name: "web_search".to_string(),
            description: "Search the web using a configured search provider (Tavily, Zhipu, or Bocha). Returns a list of relevant results with titles, URLs, and content snippets.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query string"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results to return",
                        "default": 5
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": "Timeout in milliseconds for the search request",
                        "default": 15000
                    }
                },
                "required": ["query"]
            }),
        },
    );

    tools.insert(
        "generate_image".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-image-gen".to_string(),
            server_name: "@axagent/image-gen".to_string(),
            tool_name: "generate_image".to_string(),
            description:
                "Generate an image from a text prompt using AI. Supports Flux and DALL-E providers."
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "prompt": {
                        "type": "string",
                        "description": "Text description of the image to generate"
                    },
                    "negative_prompt": {
                        "type": "string",
                        "description": "What to avoid in the image"
                    },
                    "provider": {
                        "type": "string",
                        "enum": ["flux", "dall-e"],
                        "description": "Image generation provider (default: flux)"
                    },
                    "width": {
                        "type": "integer",
                        "description": "Image width"
                    },
                    "height": {
                        "type": "integer",
                        "description": "Image height"
                    },
                    "steps": {
                        "type": "integer",
                        "description": "Number of diffusion steps (Flux only)"
                    },
                    "seed": {
                        "type": "integer",
                        "description": "Random seed for reproducibility"
                    },
                    "api_key": {
                        "type": "string",
                        "description": "API key for the provider"
                    }
                },
                "required": ["prompt"]
            }),
        },
    );

    tools.insert(
        "generate_chart_config".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-chart-gen".to_string(),
            server_name: "@axagent/chart-gen".to_string(),
            tool_name: "generate_chart_config".to_string(),
            description: "Generate an ECharts configuration from a natural language description.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "Natural language description of the chart"
                    },
                    "data": {
                        "type": "object",
                        "description": "Optional data to visualize"
                    },
                    "chart_type": {
                        "type": "string",
                        "enum": ["line", "bar", "pie", "scatter", "heatmap", "radar", "treemap", "sankey", "funnel", "gauge"],
                        "description": "Desired chart type"
                    },
                    "title": {
                        "type": "string",
                        "description": "Chart title"
                    },
                    "api_key": {
                        "type": "string",
                        "description": "API key for the LLM"
                    },
                    "base_url": {
                        "type": "string",
                        "description": "Base URL for the LLM API"
                    },
                    "model": {
                        "type": "string",
                        "description": "Model to use"
                    }
                },
                "required": ["description"]
            }),
        },
    );

    tools.insert(
        "search_replace".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-code-edit".to_string(),
            server_name: "@axagent/code-edit".to_string(),
            tool_name: "search_replace".to_string(),
            description: "Perform precise search-and-replace edits on text files. Finds the first occurrence of old_str in the specified file and replaces it with new_str. Supports optional line range constraints for disambiguation. Use this for targeted code modifications instead of rewriting entire files.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the file to edit"
                    },
                    "old_str": {
                        "type": "string",
                        "description": "The exact text to search for. Must match exactly including whitespace and indentation."
                    },
                    "new_str": {
                        "type": "string",
                        "description": "The text to replace old_str with"
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "Optional starting line number to constrain the search range (1-based)"
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "Optional ending line number to constrain the search range (1-based, inclusive)"
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "If true, replace all occurrences instead of just the first one",
                        "default": false
                    }
                },
                "required": ["path", "old_str", "new_str"]
            }),
        },
    );

    tools.insert(
        "git_status".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-git".to_string(),
            server_name: "@axagent/git".to_string(),
            tool_name: "git_status".to_string(),
            description: "Get the current git status of a repository. Shows staged, unstaged, and untracked files.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Absolute path to the git repository"
                    }
                },
                "required": ["repo_path"]
            }),
        },
    );

    tools.insert(
        "git_diff".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-git".to_string(),
            server_name: "@axagent/git".to_string(),
            tool_name: "git_diff".to_string(),
            description: "Get a summary of staged or branch changes. Returns files changed, insertions, deletions, and hunk details.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Absolute path to the git repository"
                    },
                    "base_branch": {
                        "type": "string",
                        "description": "If provided, show diff between this branch and HEAD. Otherwise shows staged diff."
                    }
                },
                "required": ["repo_path"]
            }),
        },
    );

    tools.insert(
        "git_commit".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-git".to_string(),
            server_name: "@axagent/git".to_string(),
            tool_name: "git_commit".to_string(),
            description: "Stage all changes and commit them with the given message. Use this after reviewing changes to create a commit.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Absolute path to the git repository"
                    },
                    "message": {
                        "type": "string",
                        "description": "Commit message"
                    },
                    "stage_all": {
                        "type": "boolean",
                        "description": "If true, stage all changes before committing (git add -A). Default: true",
                        "default": true
                    }
                },
                "required": ["repo_path", "message"]
            }),
        },
    );

    tools.insert(
        "git_log".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-git".to_string(),
            server_name: "@axagent/git".to_string(),
            tool_name: "git_log".to_string(),
            description:
                "Get recent git commit history. Returns commit hash, author, date, and subject."
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Absolute path to the git repository"
                    },
                    "max_count": {
                        "type": "integer",
                        "description": "Maximum number of commits to return",
                        "default": 10
                    }
                },
                "required": ["repo_path"]
            }),
        },
    );

    tools.insert(
        "git_branch".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-git".to_string(),
            server_name: "@axagent/git".to_string(),
            tool_name: "git_branch".to_string(),
            description: "List all branches in the repository, or create a new branch. Shows current branch, remote tracking, and last commit info.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Absolute path to the git repository"
                    },
                    "action": {
                        "type": "string",
                        "enum": ["list", "create", "switch"],
                        "description": "Action to perform: list branches, create a new branch, or switch to an existing branch",
                        "default": "list"
                    },
                    "name": {
                        "type": "string",
                        "description": "Branch name (required for create and switch actions)"
                    }
                },
                "required": ["repo_path"]
            }),
        },
    );

    tools.insert(
        "git_review".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-git".to_string(),
            server_name: "@axagent/git".to_string(),
            tool_name: "git_review".to_string(),
            description: "Generate a context summary of staged changes for code review. Returns diff summary, file list, and change statistics to help the LLM provide review feedback.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "repo_path": {
                        "type": "string",
                        "description": "Absolute path to the git repository"
                    },
                    "base_branch": {
                        "type": "string",
                        "description": "If provided, review changes between this branch and HEAD (for PR review). Otherwise reviews staged changes."
                    }
                },
                "required": ["repo_path"]
            }),
        },
    );

    tools.insert(
        "task".to_string(),
        BuiltinDynamicTool {
            server_id: "builtin-agent".to_string(),
            server_name: "@axagent/agent".to_string(),
            tool_name: "task".to_string(),
            description: "Launch a sub-agent to handle complex, multi-step tasks autonomously. Use this when you need to delegate work to a specialized agent. The sub-agent runs in its own child session.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_type": {
                        "type": "string",
                        "description": "Type of sub-agent to launch (e.g. 'explore', 'general', 'build', 'plan')"
                    },
                    "description": {
                        "type": "string",
                        "description": "Short description of the task for the sub-agent"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "Full prompt/instructions for the sub-agent to execute"
                    }
                },
                "required": ["agent_type", "description", "prompt"]
            }),
        },
    );

    tools
}

pub fn get_all_builtin_tools_flat() -> Vec<FlatBuiltinTool> {
    let mut tools = Vec::new();

    for server in get_all_builtin_server_definitions() {
        for tool in server.tools {
            tools.push(FlatBuiltinTool {
                server_id: server.server_id.clone(),
                server_name: server.server_name.clone(),
                tool_name: tool.tool_name,
                description: tool.description,
                input_schema: tool.input_schema,
                env_json: None,
                timeout_secs: Some(30),
            });
        }
    }

    for (_name, dynamic_tool) in get_dynamic_builtin_tools() {
        let env_json = if dynamic_tool.server_id == "builtin-session"
            || dynamic_tool.server_id == "builtin-memory"
            || dynamic_tool.server_id == "builtin-search"
            || dynamic_tool.server_id == "builtin-agent"
        {
            Some(serde_json::json!({}).to_string())
        } else {
            None
        };

        tools.push(FlatBuiltinTool {
            server_id: dynamic_tool.server_id,
            server_name: dynamic_tool.server_name,
            tool_name: dynamic_tool.tool_name,
            description: dynamic_tool.description,
            input_schema: dynamic_tool.input_schema,
            env_json,
            timeout_secs: Some(10),
        });
    }

    tools
}

pub fn validate_builtin_tools() -> Result<()> {
    let registered_handlers = list_all_builtin_handlers();
    let all_tools = get_all_builtin_tools_flat();

    let mut missing_handlers = Vec::new();
    for tool in &all_tools {
        let key = (tool.server_name.clone(), tool.tool_name.clone());
        if !registered_handlers.contains(&key) {
            missing_handlers.push(format!("{}/{}", tool.server_name, tool.tool_name));
        }
    }

    if !missing_handlers.is_empty() {
        return Err(AxAgentError::Gateway(format!(
            "Tools registered in registry but missing handlers: {}",
            missing_handlers.join(", ")
        )));
    }

    Ok(())
}
