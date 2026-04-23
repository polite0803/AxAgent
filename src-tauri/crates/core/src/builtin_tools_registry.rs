use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::LazyLock;
use crate::error::{AxAgentError, Result};
use crate::mcp_client::McpToolResult;

pub type BoxedToolHandlerInner = dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<McpToolResult>> + Send>> + Send + Sync;
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

pub fn register_builtin_handler(
    server_name: &str,
    tool_name: &str,
    handler: BoxedToolHandler,
) {
    let mut handlers = BUILTIN_HANDLERS.write().unwrap();
    handlers.insert((server_name.to_string(), tool_name.to_string()), handler);
}

pub fn get_handler(server_name: &str, tool_name: &str) -> Option<BoxedToolHandler> {
    let handlers = BUILTIN_HANDLERS.read().unwrap();
    handlers.get(&(server_name.to_string(), tool_name.to_string())).cloned()
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