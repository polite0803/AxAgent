use sea_orm::*;

use crate::entity::{mcp_servers, tool_descriptors};
use crate::error::{AxAgentError, Result};
use crate::repo::settings;
use crate::types::{CreateMcpServerInput, McpServer, ToolDescriptor};
use crate::utils::gen_id;

// ── Builtin server definitions (not stored in DB) ───────────────────────

const BUILTIN_FETCH_ID: &str = "builtin-fetch";
const BUILTIN_SEARCH_FILE_ID: &str = "builtin-search-file";
const BUILTIN_SKILLS_ID: &str = "builtin-skills";
const BUILTIN_SESSION_ID: &str = "builtin-session";
const BUILTIN_SEARCH_ID: &str = "builtin-search";
const BUILTIN_FILESYSTEM_ID: &str = "builtin-filesystem";
const BUILTIN_SYSTEM_ID: &str = "builtin-system";
const BUILTIN_KNOWLEDGE_ID: &str = "builtin-knowledge";
const BUILTIN_STORAGE_ID: &str = "builtin-storage";
const BUILTIN_BRAVE_SEARCH_ID: &str = "builtin-brave-search";
const BUILTIN_SEQUENTIAL_THINKING_ID: &str = "builtin-sequential-thinking";
const BUILTIN_PYTHON_ID: &str = "builtin-python";
const BUILTIN_DIFY_KNOWLEDGE_ID: &str = "builtin-dify-knowledge";
const BUILTIN_WORKSPACE_MEMORY_ID: &str = "builtin-workspace-memory";
const BUILTIN_FILEUTILS_ID: &str = "builtin-file-utils";
const BUILTIN_CACHE_ID: &str = "builtin-cache";
const BUILTIN_OCR_ID: &str = "builtin-ocr";
const BUILTIN_OBSIDIAN_ID: &str = "builtin-obsidian";
const BUILTIN_EXPORT_ID: &str = "builtin-export";
const BUILTIN_REMOTEFILE_ID: &str = "builtin-remotefile";
const BUILTIN_AGENTCTRL_ID: &str = "builtin-agent-control";
const BUILTIN_COMPUTER_ID: &str = "builtin-computer-control";
const BUILTIN_BROWSER_ID: &str = "builtin-browser";
const BUILTIN_MEMORY_ID: &str = "builtin-memory";
const BUILTIN_IMAGEGEN_ID: &str = "builtin-image-gen";
const BUILTIN_CHARTGEN_ID: &str = "builtin-chart-gen";
const BUILTIN_CODEEDIT_ID: &str = "builtin-code-edit";
const BUILTIN_GIT_ID: &str = "builtin-git";
const BUILTIN_CRON_ID: &str = "builtin-cron";


struct BuiltinDef {
    id: &'static str,
    name: &'static str,
    default_enabled: bool,
}

const BUILTIN_DEFS: &[BuiltinDef] = &[
    BuiltinDef {
        id: BUILTIN_FETCH_ID,
        name: "@axagent/fetch",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_SEARCH_FILE_ID,
        name: "@axagent/search-file",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_SKILLS_ID,
        name: "@axagent/skills",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_SESSION_ID,
        name: "@axagent/session",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_SEARCH_ID,
        name: "@axagent/search",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_FILESYSTEM_ID,
        name: "@axagent/filesystem",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_SYSTEM_ID,
        name: "@axagent/system",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_KNOWLEDGE_ID,
        name: "@axagent/knowledge",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_STORAGE_ID,
        name: "@axagent/storage",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_BRAVE_SEARCH_ID,
        name: "@axagent/brave-search",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_SEQUENTIAL_THINKING_ID,
        name: "@axagent/sequential-thinking",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_PYTHON_ID,
        name: "@axagent/python",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_DIFY_KNOWLEDGE_ID,
        name: "@axagent/dify-knowledge",
        default_enabled: false,
    },
    BuiltinDef {
        id: BUILTIN_WORKSPACE_MEMORY_ID,
        name: "@axagent/workspace-memory",
        default_enabled: false,
    },
    BuiltinDef {
        id: BUILTIN_FILEUTILS_ID,
        name: "@axagent/file-utils",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_CACHE_ID,
        name: "@axagent/cache",
        default_enabled: true,
    },
    BuiltinDef { id: BUILTIN_OCR_ID, name: "@axagent/ocr", default_enabled: true, },
    BuiltinDef { id: BUILTIN_OBSIDIAN_ID, name: "@axagent/obsidian", default_enabled: true, },
    BuiltinDef { id: BUILTIN_EXPORT_ID, name: "@axagent/export", default_enabled: true, },
    BuiltinDef { id: BUILTIN_REMOTEFILE_ID, name: "@axagent/remotefile", default_enabled: false, },
    BuiltinDef { id: BUILTIN_AGENTCTRL_ID, name: "@axagent/agent-control", default_enabled: true, },
    BuiltinDef { id: BUILTIN_COMPUTER_ID, name: "@axagent/computer-control", default_enabled: false, },
    BuiltinDef { id: BUILTIN_BROWSER_ID, name: "@axagent/browser", default_enabled: true, },
    BuiltinDef { id: BUILTIN_MEMORY_ID, name: "@axagent/memory", default_enabled: true, },
    BuiltinDef { id: BUILTIN_IMAGEGEN_ID, name: "@axagent/image-gen", default_enabled: false, },
    BuiltinDef { id: BUILTIN_CHARTGEN_ID, name: "@axagent/chart-gen", default_enabled: true, },
    BuiltinDef { id: BUILTIN_CODEEDIT_ID, name: "@axagent/code-edit", default_enabled: false, },
    BuiltinDef { id: BUILTIN_GIT_ID, name: "@axagent/git", default_enabled: false, },
    BuiltinDef { id: BUILTIN_CRON_ID, name: "@axagent/cron", default_enabled: false, },

];

// ── Preset MCP servers (stored in DB, auto-created on first run) ───────

/// Preset server definitions for popular MCP servers.
/// These are auto-created and enabled on first launch.
struct PresetDef {
    /// Unique ID for the preset
    id: &'static str,
    /// Display name
    name: &'static str,
    /// npx package name
    package: &'static str,
    /// Transport type (currently only stdio supported)
    transport: &'static str,
    /// Whether enabled by default
    default_enabled: bool,
}

const PRESET_DEFS: &[PresetDef] = &[
    // NOTE: Filesystem, Terminal, and Memory are now handled by LocalToolRegistry
    // as builtin tools (@axagent/filesystem, @axagent/system, @axagent/memory).
    // Git and GitHub can be accessed via run_command (git/gh CLI).
    // Only keep Browser (puppeteer) as it has no local equivalent.
    PresetDef {
        id: "preset-puppeteer",
        name: "Browser",
        package: "@modelcontextprotocol/server-puppeteer",
        transport: "stdio",
        default_enabled: true,
    },
];

fn make_preset_server(def: &PresetDef) -> McpServer {
    McpServer {
        id: def.id.to_string(),
        name: def.name.to_string(),
        transport: def.transport.to_string(),
        command: Some("npx".to_string()),
        args_json: Some(serde_json::json!(["-y", def.package]).to_string()),
        endpoint: None,
        env_json: None,
        enabled: def.default_enabled,
        permission_policy: "ask".to_string(),
        source: "preset".to_string(),
        discover_timeout_secs: Some(60),
        execute_timeout_secs: Some(30),
        headers_json: None,
        icon_type: Some("emoji".to_string()),
        icon_value: Some(
            match def.id {
                "preset-filesystem" => "📁",
                "preset-bash" => "💻",
                "preset-git" => "🔀",
                "preset-github" => "🐙",
                "preset-memory" => "🧠",
                "preset-puppeteer" => "🌐",
                _ => "🔧",
            }
            .to_string(),
        ),
    }
}

fn builtin_setting_key(name: &str) -> String {
    format!("builtin_mcp:{name}:enabled")
}

fn make_builtin_server(def: &BuiltinDef, enabled: bool) -> McpServer {
    McpServer {
        id: def.id.to_string(),
        name: def.name.to_string(),
        transport: "builtin".to_string(),
        command: None,
        args_json: None,
        endpoint: None,
        env_json: None,
        enabled,
        permission_policy: "auto".to_string(),
        source: "builtin".to_string(),
        discover_timeout_secs: None,
        execute_timeout_secs: None,
        headers_json: None,
        icon_type: None,
        icon_value: None,
    }
}

async fn get_builtin_enabled(db: &DatabaseConnection, name: &str, default: bool) -> bool {
    match settings::get_setting(db, &builtin_setting_key(name)).await {
        Ok(Some(v)) => v == "true",
        _ => default,
    }
}

/// Return all builtin servers with their persisted enabled state.
pub async fn list_builtin_servers(db: &DatabaseConnection) -> Vec<McpServer> {
    let mut out = Vec::with_capacity(BUILTIN_DEFS.len());
    for def in BUILTIN_DEFS {
        let enabled = get_builtin_enabled(db, def.name, def.default_enabled).await;
        out.push(make_builtin_server(def, enabled));
    }
    out
}

/// Check whether a server ID belongs to a builtin.
pub fn is_builtin_id(id: &str) -> bool {
    BUILTIN_DEFS.iter().any(|d| d.id == id)
}

/// Toggle enabled state for a builtin server (persists to settings table).
pub async fn set_builtin_enabled(
    db: &DatabaseConnection,
    id: &str,
    enabled: bool,
) -> Result<McpServer> {
    let def = BUILTIN_DEFS
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| AxAgentError::NotFound(format!("Builtin server {id}")))?;
    settings::set_setting(
        db,
        &builtin_setting_key(def.name),
        if enabled { "true" } else { "false" },
    )
    .await?;
    Ok(make_builtin_server(def, enabled))
}

/// Get a single builtin server by ID.
pub async fn get_builtin_server(db: &DatabaseConnection, id: &str) -> Result<McpServer> {
    let def = BUILTIN_DEFS
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| AxAgentError::NotFound(format!("Builtin server {id}")))?;
    let enabled = get_builtin_enabled(db, def.name, def.default_enabled).await;
    Ok(make_builtin_server(def, enabled))
}

// ── DB-backed custom servers ────────────────────────────────────────────

fn model_to_mcp_server(m: mcp_servers::Model) -> McpServer {
    McpServer {
        id: m.id,
        name: m.name,
        transport: m.transport,
        command: m.command,
        args_json: m.args_json,
        endpoint: m.endpoint,
        env_json: m.env_json,
        enabled: m.enabled != 0,
        permission_policy: m.permission_policy,
        source: m.source,
        discover_timeout_secs: m.discover_timeout_secs,
        execute_timeout_secs: m.execute_timeout_secs,
        headers_json: m.headers_json,
        icon_type: m.icon_type,
        icon_value: m.icon_value,
    }
}

/// Ensure all preset servers exist in the database.
/// Creates any missing presets with default settings.
pub async fn ensure_preset_servers(db: &DatabaseConnection) -> Result<()> {
    for preset in PRESET_DEFS {
        // Check if this preset already exists
        let existing = mcp_servers::Entity::find_by_id(preset.id).one(db).await?;

        if existing.is_none() {
            // Create the preset server
            let server = make_preset_server(preset);
            mcp_servers::ActiveModel {
                id: Set(server.id.clone()),
                name: Set(server.name.clone()),
                transport: Set(server.transport.clone()),
                command: Set(server.command.clone()),
                args_json: Set(server.args_json.clone()),
                endpoint: Set(server.endpoint.clone()),
                env_json: Set(server.env_json.clone()),
                enabled: Set(if server.enabled { 1 } else { 0 }),
                permission_policy: Set(server.permission_policy.clone()),
                source: Set(server.source.clone()),
                discover_timeout_secs: Set(server.discover_timeout_secs),
                execute_timeout_secs: Set(server.execute_timeout_secs),
                headers_json: Set(server.headers_json.clone()),
                icon_type: Set(server.icon_type.clone()),
                icon_value: Set(server.icon_value.clone()),
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}

pub async fn list_mcp_servers(db: &DatabaseConnection) -> Result<Vec<McpServer>> {
    // Ensure presets are created
    let _ = ensure_preset_servers(db).await;

    let mut servers = list_builtin_servers(db).await;

    let custom_rows = mcp_servers::Entity::find()
        .filter(mcp_servers::Column::Source.ne("preset")) // Skip presets, they have their own section
        .order_by_asc(mcp_servers::Column::Name)
        .all(db)
        .await?;
    servers.extend(custom_rows.into_iter().map(model_to_mcp_server));

    // Add preset servers
    let preset_rows = mcp_servers::Entity::find()
        .filter(mcp_servers::Column::Source.eq("preset"))
        .order_by_asc(mcp_servers::Column::Name)
        .all(db)
        .await?;
    servers.extend(preset_rows.into_iter().map(model_to_mcp_server));

    Ok(servers)
}

pub async fn get_mcp_server(db: &DatabaseConnection, id: &str) -> Result<McpServer> {
    // Check builtins first
    if is_builtin_id(id) {
        return get_builtin_server(db, id).await;
    }

    let model = mcp_servers::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("McpServer {}", id)))?;

    Ok(model_to_mcp_server(model))
}

pub async fn create_mcp_server(
    db: &DatabaseConnection,
    input: CreateMcpServerInput,
) -> Result<McpServer> {
    let id = gen_id();

    let args_json = input
        .args
        .as_ref()
        .map(|a| serde_json::to_string(a).unwrap_or_default());
    let env_json = input
        .env
        .as_ref()
        .map(|e| serde_json::to_string(e).unwrap_or_default());

    mcp_servers::ActiveModel {
        id: Set(id.clone()),
        name: Set(input.name),
        transport: Set(input.transport),
        command: Set(input.command),
        args_json: Set(args_json),
        endpoint: Set(input.endpoint),
        env_json: Set(env_json),
        enabled: Set(if input.enabled.unwrap_or(true) { 1 } else { 0 }),
        permission_policy: Set(input.permission_policy.unwrap_or_else(|| "ask".to_string())),
        source: Set(input.source.unwrap_or_else(|| "custom".to_string())),
        discover_timeout_secs: Set(input.discover_timeout_secs),
        execute_timeout_secs: Set(input.execute_timeout_secs),
        headers_json: Set(input.headers_json),
        icon_type: Set(input.icon_type),
        icon_value: Set(input.icon_value),
    }
    .insert(db)
    .await?;

    get_mcp_server(db, &id).await
}

pub async fn update_mcp_server(
    db: &DatabaseConnection,
    id: &str,
    input: CreateMcpServerInput,
) -> Result<McpServer> {
    // Builtin servers only support toggling enabled
    if is_builtin_id(id) {
        let enabled = input.enabled.unwrap_or(true);
        return set_builtin_enabled(db, id, enabled).await;
    }

    let existing = get_mcp_server(db, id).await?;

    let name = if input.name.is_empty() {
        existing.name
    } else {
        input.name
    };
    let transport = if input.transport.is_empty() {
        existing.transport
    } else {
        input.transport
    };
    let command = input.command.or(existing.command);
    let endpoint = input.endpoint.or(existing.endpoint);
    let enabled = input.enabled.unwrap_or(existing.enabled);
    let permission_policy = input
        .permission_policy
        .unwrap_or(existing.permission_policy);

    let args_json = match input.args {
        Some(ref a) => Some(serde_json::to_string(a).unwrap_or_default()),
        None => existing.args_json,
    };
    let env_json = match input.env {
        Some(ref e) => Some(serde_json::to_string(e).unwrap_or_default()),
        None => existing.env_json,
    };
    let discover_timeout_secs = input
        .discover_timeout_secs
        .or(existing.discover_timeout_secs);
    let execute_timeout_secs = input.execute_timeout_secs.or(existing.execute_timeout_secs);
    let headers_json = input.headers_json.or(existing.headers_json);
    let icon_type = match input.icon_type {
        Some(ref v) if v.is_empty() => None,
        Some(v) => Some(v),
        None => existing.icon_type,
    };
    let icon_value = match input.icon_value {
        Some(ref v) if v.is_empty() => None,
        Some(v) => Some(v),
        None => existing.icon_value,
    };

    let model = mcp_servers::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("McpServer {}", id)))?;

    let mut am: mcp_servers::ActiveModel = model.into();
    am.name = Set(name);
    am.transport = Set(transport);
    am.command = Set(command);
    am.args_json = Set(args_json);
    am.endpoint = Set(endpoint);
    am.env_json = Set(env_json);
    am.enabled = Set(if enabled { 1 } else { 0 });
    am.permission_policy = Set(permission_policy);
    am.discover_timeout_secs = Set(discover_timeout_secs);
    am.execute_timeout_secs = Set(execute_timeout_secs);
    am.headers_json = Set(headers_json);
    am.icon_type = Set(icon_type);
    am.icon_value = Set(icon_value);
    am.update(db).await?;

    get_mcp_server(db, id).await
}

pub async fn delete_mcp_server(db: &DatabaseConnection, id: &str) -> Result<()> {
    // Prevent deletion of built-in MCP servers
    let server = get_mcp_server(db, id).await?;
    if server.source == "builtin" {
        return Err(AxAgentError::Gateway(
            "Cannot delete built-in MCP server".to_string(),
        ));
    }

    let result = mcp_servers::Entity::delete_by_id(id).exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("McpServer {}", id)));
    }
    Ok(())
}

/// Return tool descriptors for a given MCP server.
pub async fn list_tools_for_server(
    db: &DatabaseConnection,
    server_id: &str,
) -> Result<Vec<ToolDescriptor>> {
    // Builtins: resolve name from definition, no DB lookup needed
    if let Some(def) = BUILTIN_DEFS.iter().find(|d| d.id == server_id) {
        return Ok(builtin_tools(server_id, def.name));
    }
    // Custom servers: read from tool_descriptors table
    let rows = tool_descriptors::Entity::find()
        .filter(tool_descriptors::Column::ServerId.eq(server_id))
        .order_by_asc(tool_descriptors::Column::Name)
        .all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|m| ToolDescriptor {
            id: m.id,
            server_id: m.server_id,
            name: m.name,
            description: m.description,
            input_schema_json: m.input_schema_json,
        })
        .collect())
}

/// Save discovered tool descriptors for a server (replaces existing).
pub async fn save_tool_descriptors(
    db: &DatabaseConnection,
    server_id: &str,
    tools: Vec<crate::mcp_client::DiscoveredTool>,
) -> Result<Vec<ToolDescriptor>> {
    // Delete existing tools for this server
    tool_descriptors::Entity::delete_many()
        .filter(tool_descriptors::Column::ServerId.eq(server_id))
        .exec(db)
        .await?;

    // Insert new tools
    let mut result = Vec::with_capacity(tools.len());
    for tool in tools {
        let id = gen_id();
        let input_schema_json = tool
            .input_schema
            .as_ref()
            .map(|s| serde_json::to_string(s).unwrap_or_default());

        tool_descriptors::ActiveModel {
            id: Set(id.clone()),
            server_id: Set(server_id.to_string()),
            name: Set(tool.name.clone()),
            description: Set(tool.description.clone()),
            input_schema_json: Set(input_schema_json.clone()),
        }
        .insert(db)
        .await?;

        result.push(ToolDescriptor {
            id,
            server_id: server_id.to_string(),
            name: tool.name,
            description: tool.description,
            input_schema_json,
        });
    }
    Ok(result)
}

fn builtin_tools(server_id: &str, server_name: &str) -> Vec<ToolDescriptor> {
    match server_name {
        "@axagent/fetch" => vec![
            ToolDescriptor {
                id: format!("{server_id}-fetch-url"),
                server_id: server_id.to_string(),
                name: "fetch_url".into(),
                description: Some("Fetch a URL and return its content".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"url":{"type":"string","description":"URL to fetch"},"max_length":{"type":"integer","description":"Maximum content length"}},"required":["url"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-fetch-markdown"),
                server_id: server_id.to_string(),
                name: "fetch_markdown".into(),
                description: Some("Fetch a URL and convert the content to markdown".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"url":{"type":"string","description":"URL to fetch"}},"required":["url"]}"#.into()),
            },
        ],
        "@axagent/search-file" => vec![
            ToolDescriptor {
                id: format!("{server_id}-read-file"),
                server_id: server_id.to_string(),
                name: "read_file".into(),
                description: Some("Read the contents of a file".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string","description":"File path to read"}},"required":["path"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-list-directory"),
                server_id: server_id.to_string(),
                name: "list_directory".into(),
                description: Some("List files and directories in a given path".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string","description":"Directory path to list"}},"required":["path"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-search-files"),
                server_id: server_id.to_string(),
                name: "search_files".into(),
                description: Some("Search for files matching a pattern".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string","description":"Base directory"},"pattern":{"type":"string","description":"Search pattern"}},"required":["path","pattern"]}"#.into()),
            },
        ],
        "@axagent/skills" => vec![
            ToolDescriptor {
                id: format!("{server_id}-skill-manage"),
                server_id: server_id.to_string(),
                name: "skill_manage".into(),
                description: Some("Manage AI skills: create, patch, edit, list, view, delete skills".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"action":{"type":"string","description":"Action: list, view, create, patch, edit, delete"},"name":{"type":"string","description":"Skill name"},"description":{"type":"string","description":"Skill description"},"content":{"type":"string","description":"Skill content (markdown)"},"skills_dir":{"type":"string","description":"Skills directory path"}},"required":["action"]}"#.into()),
            },
        ],
        "@axagent/session" => vec![
            ToolDescriptor {
                id: format!("{server_id}-session-search"),
                server_id: server_id.to_string(),
                name: "session_search".into(),
                description: Some("Search past conversations using full-text search".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"query":{"type":"string","description":"Search query"},"limit":{"type":"integer","description":"Maximum results (default: 10)"},"db_path":{"type":"string","description":"Database path"}},"required":["query"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-memory-flush"),
                server_id: server_id.to_string(),
                name: "memory_flush".into(),
                description: Some("Persist an insight or memory for future sessions".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"insight":{"type":"string","description":"Insight content to persist"},"target":{"type":"string","description":"Target: skill, session, memory"},"category":{"type":"string","description":"Category: pattern, solution, preference, fact"}},"required":["insight","target","category"]}"#.into()),
            },
        ],
        "@axagent/search" => vec![
            ToolDescriptor {
                id: format!("{server_id}-web-search"),
                server_id: server_id.to_string(),
                name: "web_search".into(),
                description: Some("Search the web using Tavily, Zhipu, or Bocha search provider".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"query":{"type":"string","description":"Search query"},"provider_type":{"type":"string","description":"Provider: tavily, zhipu, bocha (default: zhipu)"},"max_results":{"type":"integer","description":"Maximum results (default: 5)"}},"required":["query"]}"#.into()),
            },
        ],
        "@axagent/filesystem" => vec![
            ToolDescriptor {
                id: format!("{server_id}-write-file"),
                server_id: server_id.to_string(),
                name: "write_file".into(),
                description: Some("Write content to a file (creates parent directories if needed)".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string","description":"File path to write"},"content":{"type":"string","description":"Content to write"}},"required":["path","content"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-edit-file"),
                server_id: server_id.to_string(),
                name: "edit_file".into(),
                description: Some("Edit a file by replacing text (first occurrence only)".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string","description":"File path to edit"},"old_str":{"type":"string","description":"Text to find and replace"},"new_str":{"type":"string","description":"Replacement text"}},"required":["path","old_str","new_str"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-delete-file"),
                server_id: server_id.to_string(),
                name: "delete_file".into(),
                description: Some("Delete a file or directory".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string","description":"File or directory path to delete"}},"required":["path"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-create-directory"),
                server_id: server_id.to_string(),
                name: "create_directory".into(),
                description: Some("Create a directory (creates parent directories recursively)".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string","description":"Directory path to create"}},"required":["path"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-file-exists"),
                server_id: server_id.to_string(),
                name: "file_exists".into(),
                description: Some("Check if a file or directory exists".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string","description":"File or directory path to check"}},"required":["path"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-get-file-info"),
                server_id: server_id.to_string(),
                name: "get_file_info".into(),
                description: Some("Get detailed information about a file or directory".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string","description":"File or directory path"}},"required":["path"]}"#.into()),
            },
        ],
        "@axagent/system" => vec![
            ToolDescriptor {
                id: format!("{server_id}-run-command"),
                server_id: server_id.to_string(),
                name: "run_command".into(),
                description: Some("Execute a shell command (blocked dangerous commands for security)".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"command":{"type":"string","description":"Shell command to execute"},"timeout_secs":{"type":"integer","description":"Timeout in seconds (default: 30)"}},"required":["command"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-get-system-info"),
                server_id: server_id.to_string(),
                name: "get_system_info".into(),
                description: Some("Get system information (OS, architecture, home dir, uptime)".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{}}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-list-processes"),
                server_id: server_id.to_string(),
                name: "list_processes".into(),
                description: Some("List running processes".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"limit":{"type":"integer","description":"Maximum number of processes to show (default: 20)"}}}"#.into()),
            },
        ],
        "@axagent/knowledge" => vec![
            ToolDescriptor {
                id: format!("{server_id}-list-knowledge-bases"),
                server_id: server_id.to_string(),
                name: "list_knowledge_bases".into(),
                description: Some("List available knowledge bases".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{}}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-search-knowledge"),
                server_id: server_id.to_string(),
                name: "search_knowledge".into(),
                description: Some("Search a knowledge base for relevant information".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"base_id":{"type":"string","description":"Knowledge base ID"},"query":{"type":"string","description":"Search query"},"top_k":{"type":"integer","description":"Number of results (default: 5)"}},"required":["query"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-create-knowledge-entity"),
                server_id: server_id.to_string(),
                name: "create_knowledge_entity".into(),
                description: Some("Create a knowledge graph entity (service, component, module, etc.)".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"knowledge_base_id":{"type":"string"},"name":{"type":"string"},"entity_type":{"type":"string"},"description":{"type":"string"},"source_path":{"type":"string"},"source_language":{"type":"string"},"properties":{"type":"object"},"lifecycle":{"type":"object"},"behaviors":{"type":"object"}},"required":["knowledge_base_id","name","entity_type"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-create-knowledge-flow"),
                server_id: server_id.to_string(),
                name: "create_knowledge_flow".into(),
                description: Some("Create a knowledge graph flow (process, pipeline, workflow)".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"knowledge_base_id":{"type":"string"},"name":{"type":"string"},"flow_type":{"type":"string"},"description":{"type":"string"},"source_path":{"type":"string"},"steps":{"type":"object"},"decision_points":{"type":"object"},"error_handling":{"type":"object"},"preconditions":{"type":"object"},"postconditions":{"type":"object"}},"required":["knowledge_base_id","name","flow_type"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-create-knowledge-interface"),
                server_id: server_id.to_string(),
                name: "create_knowledge_interface".into(),
                description: Some("Create a knowledge graph interface (API, protocol, contract)".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"knowledge_base_id":{"type":"string"},"name":{"type":"string"},"interface_type":{"type":"string"},"description":{"type":"string"},"source_path":{"type":"string"},"input_schema":{"type":"object"},"output_schema":{"type":"object"},"error_codes":{"type":"object"},"communication_pattern":{"type":"string"}},"required":["knowledge_base_id","name","interface_type"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-add-knowledge-document"),
                server_id: server_id.to_string(),
                name: "add_knowledge_document".into(),
                description: Some("Add a document to a knowledge base for indexing".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"knowledge_base_id":{"type":"string"},"title":{"type":"string"},"content":{"type":"string"}},"required":["knowledge_base_id","title","content"]}"#.into()),
            },
        ],
        "@axagent/storage" => vec![
            ToolDescriptor {
                id: format!("{server_id}-get-storage-info"),
                server_id: server_id.to_string(),
                name: "get_storage_info".into(),
                description: Some("Get AxAgent storage information (config and documents directories)".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{}}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-list-storage-files"),
                server_id: server_id.to_string(),
                name: "list_storage_files".into(),
                description: Some("List files in AxAgent storage directory".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string","description":"Subdirectory path (images, files, backups, or empty for root)"},"limit":{"type":"integer","description":"Maximum files to show (default: 50)"}}}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-upload-storage-file"),
                server_id: server_id.to_string(),
                name: "upload_storage_file".into(),
                description: Some("Upload a file to AxAgent storage (base64 encoded)".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"filename":{"type":"string","description":"File name"},"content_base64":{"type":"string","description":"File content as base64"},"bucket":{"type":"string","description":"Storage bucket: images, files, or backups"}},"required":["filename","content_base64"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-download-storage-file"),
                server_id: server_id.to_string(),
                name: "download_storage_file".into(),
                description: Some("Download a file from AxAgent storage (returns base64)".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string","description":"File path relative to documents root"}},"required":["path"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-delete-storage-file"),
                server_id: server_id.to_string(),
                name: "delete_storage_file".into(),
                description: Some("Delete a file from AxAgent storage".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string","description":"File path relative to documents root"}},"required":["path"]}"#.into()),
            },
        ],
        "@axagent/brave-search" => vec![
            ToolDescriptor {
                id: format!("{server_id}-brave-web-search"),
                server_id: server_id.to_string(),
                name: "brave_web_search".into(),
                description: Some("Search the web using Brave Search API. Returns web search results with titles, URLs, and descriptions.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"query":{"type":"string","description":"Search query string"},"count":{"type":"integer","description":"Number of results (default: 10, max: 20)"}},"required":["query"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-brave-local-search"),
                server_id: server_id.to_string(),
                name: "brave_local_search".into(),
                description: Some("Search for local businesses and places using Brave Search API.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"query":{"type":"string","description":"Search query for local places"},"count":{"type":"integer","description":"Number of results (default: 5)"}},"required":["query"]}"#.into()),
            },
        ],
        "@axagent/sequential-thinking" => vec![
            ToolDescriptor {
                id: format!("{server_id}-sequential-thinking"),
                server_id: server_id.to_string(),
                name: "sequentialthinking".into(),
                description: Some("A detailed tool for dynamic and reflective problem-solving through thoughts. This tool helps analyze problems through a flexible thinking process that can adapt and evolve. Each thought can build on, question, or revise previous insights as understanding deepens. Use this tool for complex problems requiring step-by-step reasoning.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"thought":{"type":"string","description":"Your current thinking step"},"nextThoughtNeeded":{"type":"boolean","description":"Whether another thought step is needed"},"thoughtNumber":{"type":"integer","description":"Current thought number"},"totalThoughts":{"type":"integer","description":"Estimated total thoughts needed"},"isRevision":{"type":"boolean","description":"Whether this revises a previous thought"},"revisesThought":{"type":"integer","description":"Which thought number is being revised"},"branchFromThought":{"type":"integer","description":"Branching point thought number"},"branchId":{"type":"string","description":"Branch identifier"},"needsMoreThoughts":{"type":"boolean","description":"Whether more thoughts are needed"}},"required":["thought","nextThoughtNeeded","thoughtNumber","totalThoughts"]}"#.into()),
            },
        ],
        "@axagent/python" => vec![
            ToolDescriptor {
                id: format!("{server_id}-python-execute"),
                server_id: server_id.to_string(),
                name: "python_execute".into(),
                description: Some("Execute a Python script in a sandboxed environment. Returns stdout and stderr output.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"script":{"type":"string","description":"Python script to execute"},"timeout":{"type":"integer","description":"Timeout in seconds (default: 30, max: 120)"}},"required":["script"]}"#.into()),
            },
        ],
        "@axagent/dify-knowledge" => vec![
            ToolDescriptor {
                id: format!("{server_id}-dify-list-bases"),
                server_id: server_id.to_string(),
                name: "dify_list_bases".into(),
                description: Some("List all available knowledge bases from a Dify instance.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"api_base":{"type":"string","description":"Dify API base URL (e.g. https://api.dify.ai/v1)"},"api_key":{"type":"string","description":"Dify API key"}},"required":["api_base","api_key"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-dify-search"),
                server_id: server_id.to_string(),
                name: "dify_search".into(),
                description: Some("Search a Dify knowledge base for relevant documents.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"api_base":{"type":"string","description":"Dify API base URL"},"api_key":{"type":"string","description":"Dify API key"},"dataset_id":{"type":"string","description":"Knowledge base (dataset) ID to search"},"query":{"type":"string","description":"Search query"},"top_k":{"type":"integer","description":"Number of results (default: 5)"}},"required":["api_base","api_key","dataset_id","query"]}"#.into()),
            },
        ],
        "@axagent/workspace-memory" => vec![
            ToolDescriptor {
                id: format!("{server_id}-workspace-read"),
                server_id: server_id.to_string(),
                name: "workspace_read".into(),
                description: Some("Read a memory file from the agent workspace (e.g. SUMMARY.md, FACT.md, journal entries).".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"filename":{"type":"string","description":"Memory filename to read (default: FACT.md)"},"workspace_path":{"type":"string","description":"Workspace directory path"}},"required":["workspace_path"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-workspace-write"),
                server_id: server_id.to_string(),
                name: "workspace_write".into(),
                description: Some("Write or append to a memory file in the agent workspace. Use to persist important facts, decisions, or context.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"filename":{"type":"string","description":"Memory filename (default: FACT.md)"},"workspace_path":{"type":"string","description":"Workspace directory path"},"content":{"type":"string","description":"Content to write or append"},"mode":{"type":"string","enum":["overwrite","append"],"description":"Write mode (default: append)"}},"required":["workspace_path","content"]}"#.into()),
            },
        ],

        "@axagent/file-utils" => vec![
            ToolDescriptor {
                id: format!("{server_id}-pdf-info"),
                server_id: server_id.to_string(),
                name: "pdf_info".into(),
                description: Some("Extract text and metadata from a PDF file. Returns page count and text preview.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"file_path":{"type":"string","description":"Absolute path to the PDF file"}},"required":["file_path"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-detect-encoding"),
                server_id: server_id.to_string(),
                name: "detect_encoding".into(),
                description: Some("Detect the text encoding of a file.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"file_path":{"type":"string","description":"Absolute path to the file"}},"required":["file_path"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-base64-image"),
                server_id: server_id.to_string(),
                name: "base64_image".into(),
                description: Some("Read an image file and return base64-encoded content with MIME type.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"file_path":{"type":"string","description":"Absolute path to the image file"}},"required":["file_path"]}"#.into()),
            },
        ],
        "@axagent/cache" => vec![
            ToolDescriptor {
                id: format!("{server_id}-cache-info"),
                server_id: server_id.to_string(),
                name: "cache_info".into(),
                description: Some("Get application cache size and information.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{}}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-cache-clear"),
                server_id: server_id.to_string(),
                name: "cache_clear".into(),
                description: Some("Clear application caches to free disk space.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"cache_type":{"type":"string","enum":["all","temp"],"description":"Cache type (default: all)"}}}"#.into()),
            },
        ],

        "@axagent/ocr" => vec![
            ToolDescriptor {
                id: format!("{server_id}-ocr-image"),
                server_id: server_id.to_string(),
                name: "ocr_image".into(),
                description: Some("Extract text from an image file using OCR (Optical Character Recognition). Supports PNG, JPEG, TIFF, BMP. Requires tesseract to be installed.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"file_path":{"type":"string","description":"Absolute path to the image file"},"lang":{"type":"string","description":"Language code (default: eng). Use ocr_detect_langs to list available languages."}},"required":["file_path"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-ocr-detect-langs"),
                server_id: server_id.to_string(),
                name: "ocr_detect_langs".into(),
                description: Some("List available OCR language packs installed in tesseract.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{}}"#.into()),
            },
        ],

        "@axagent/obsidian" => vec![
            ToolDescriptor {
                id: format!("{server_id}-obsidian-get-vaults"),
                server_id: server_id.to_string(),
                name: "obsidian_get_vaults".into(),
                description: Some("Find all Obsidian vaults on this system. Searches common locations including Documents, home directory, and configured paths.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"search_path":{"type":"string","description":"Optional override search path"}}}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-obsidian-list-files"),
                server_id: server_id.to_string(),
                name: "obsidian_list_files".into(),
                description: Some("List all markdown files in an Obsidian vault.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"vault_path":{"type":"string","description":"Absolute path to the Obsidian vault root"}},"required":["vault_path"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-obsidian-read-file"),
                server_id: server_id.to_string(),
                name: "obsidian_read_file".into(),
                description: Some("Read a markdown file from an Obsidian vault.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"vault_path":{"type":"string","description":"Vault root path"},"file_path":{"type":"string","description":"Relative path to the file within the vault"}},"required":["vault_path","file_path"]}"#.into()),
            },
        ],
        "@axagent/export" => vec![
            ToolDescriptor {
                id: format!("{server_id}-export-word"),
                server_id: server_id.to_string(),
                name: "export_word".into(),
                description: Some("Export markdown content as a Word (.docx) document.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"markdown":{"type":"string","description":"Markdown content to export"},"output_path":{"type":"string","description":"Output file path (e.g. /path/to/document.docx)"},"title":{"type":"string","description":"Document title"}},"required":["markdown","output_path"]}"#.into()),
            },
        ],
        "@axagent/remotefile" => vec![
            ToolDescriptor {
                id: format!("{server_id}-remotefile-upload"),
                server_id: server_id.to_string(),
                name: "remotefile_upload".into(),
                description: Some("Upload a file to a remote AI file service (Gemini, OpenAI, or Mistral).".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"provider":{"type":"string","enum":["gemini","openai","mistral"],"description":"AI provider"},"api_key":{"type":"string","description":"API key for the provider"},"file_path":{"type":"string","description":"Local file path to upload"},"purpose":{"type":"string","description":"File purpose (optional, for OpenAI)"}},"required":["provider","api_key","file_path"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-remotefile-list"),
                server_id: server_id.to_string(),
                name: "remotefile_list".into(),
                description: Some("List files stored on a remote AI file service.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"provider":{"type":"string","enum":["gemini","openai","mistral"],"description":"AI provider"},"api_key":{"type":"string","description":"API key"}},"required":["provider","api_key"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-remotefile-delete"),
                server_id: server_id.to_string(),
                name: "remotefile_delete".into(),
                description: Some("Delete a file from a remote AI file service.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"provider":{"type":"string","enum":["gemini","openai","mistral"],"description":"AI provider"},"api_key":{"type":"string","description":"API key"},"file_id":{"type":"string","description":"File ID to delete"}},"required":["provider","api_key","file_id"]}"#.into()),
            },
        ],

        "@axagent/agent-control" => vec![
            ToolDescriptor {
                id: format!("{server_id}-agent-checkpoint"),
                server_id: server_id.to_string(),
                name: "agent_checkpoint".into(),
                description: Some("Save a checkpoint of the current agent task state. Use during complex multi-step tasks to allow resuming if interrupted.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"action":{"type":"string","enum":["save","list","restore"],"description":"Action: save a new checkpoint, list existing checkpoints, or restore from a checkpoint"},"checkpoint_id":{"type":"string","description":"Checkpoint ID (required for restore)"},"label":{"type":"string","description":"Human-readable label for the checkpoint"}},"required":["action"]}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-agent-status"),
                server_id: server_id.to_string(),
                name: "agent_status".into(),
                description: Some("Report the current agent status including running tasks, tool execution history, error count, and session duration.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{}}"#.into()),
            },
            ToolDescriptor {
                id: format!("{server_id}-agent-remember"),
                server_id: server_id.to_string(),
                name: "agent_remember".into(),
                description: Some("Persist an important piece of information to the agent's session memory. Use for key findings, user preferences, decisions, or work-in-progress state that should survive across tool calls.".into()),
                input_schema_json: Some(r#"{"type":"object","properties":{"key":{"type":"string","description":"Memory key (e.g. 'user_preference', 'task_context', 'findings')"},"value":{"type":"string","description":"Value to remember"}},"required":["key","value"]}"#.into()),
            },
        ],

        "@axagent/computer-control" => vec![
            ToolDescriptor { id: format!("{server_id}-screen-capture"), server_id: server_id.to_string(), name: "screen_capture".into(), description: Some("Capture a screenshot of the screen, region, or window".into()), input_schema_json: Some(r#"{"type":"object","properties":{"monitor":{"type":"integer"},"region":{"type":"object"},"window_title":{"type":"string"}}}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-find-ui-elements"), server_id: server_id.to_string(), name: "find_ui_elements".into(), description: Some("Find accessible UI elements on screen".into()), input_schema_json: Some(r#"{"type":"object","properties":{"role":{"type":"string"},"name_contains":{"type":"string"},"application":{"type":"string"},"window_title":{"type":"string"}}}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-mouse-click"), server_id: server_id.to_string(), name: "mouse_click".into(), description: Some("Click at specified screen coordinates".into()), input_schema_json: Some(r#"{"type":"object","properties":{"x":{"type":"number"},"y":{"type":"number"},"button":{"type":"string","enum":["left","right","middle"]}},"required":["x","y"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-type-text"), server_id: server_id.to_string(), name: "type_text".into(), description: Some("Type text at the current position".into()), input_schema_json: Some(r#"{"type":"object","properties":{"text":{"type":"string"},"x":{"type":"number"},"y":{"type":"number"}},"required":["text"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-press-key"), server_id: server_id.to_string(), name: "press_key".into(), description: Some("Press a keyboard key with optional modifiers".into()), input_schema_json: Some(r#"{"type":"object","properties":{"key":{"type":"string"},"modifiers":{"type":"array","items":{"type":"string"}}},"required":["key"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-mouse-scroll"), server_id: server_id.to_string(), name: "mouse_scroll".into(), description: Some("Scroll at specified coordinates".into()), input_schema_json: Some(r#"{"type":"object","properties":{"x":{"type":"number"},"y":{"type":"number"},"delta":{"type":"integer"}},"required":["x","y","delta"]}"#.into()), },
        ],
        "@axagent/browser" => vec![
            ToolDescriptor { id: format!("{server_id}-browser-navigate"), server_id: server_id.to_string(), name: "browser_navigate".into(), description: Some("Navigate to a URL in the browser".into()), input_schema_json: Some(r#"{"type":"object","properties":{"url":{"type":"string"}},"required":["url"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-browser-screenshot"), server_id: server_id.to_string(), name: "browser_screenshot".into(), description: Some("Take a screenshot of the browser page".into()), input_schema_json: Some(r#"{"type":"object","properties":{"full_page":{"type":"boolean"}}}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-browser-click"), server_id: server_id.to_string(), name: "browser_click".into(), description: Some("Click an element by CSS selector".into()), input_schema_json: Some(r#"{"type":"object","properties":{"selector":{"type":"string"}},"required":["selector"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-browser-fill"), server_id: server_id.to_string(), name: "browser_fill".into(), description: Some("Fill an input field".into()), input_schema_json: Some(r#"{"type":"object","properties":{"selector":{"type":"string"},"value":{"type":"string"}},"required":["selector","value"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-browser-type"), server_id: server_id.to_string(), name: "browser_type".into(), description: Some("Type text into an element".into()), input_schema_json: Some(r#"{"type":"object","properties":{"selector":{"type":"string"},"text":{"type":"string"}},"required":["selector","text"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-browser-extract-text"), server_id: server_id.to_string(), name: "browser_extract_text".into(), description: Some("Extract text from an element".into()), input_schema_json: Some(r#"{"type":"object","properties":{"selector":{"type":"string"}},"required":["selector"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-browser-extract-all"), server_id: server_id.to_string(), name: "browser_extract_all".into(), description: Some("Extract all matching elements".into()), input_schema_json: Some(r#"{"type":"object","properties":{"selector":{"type":"string"}},"required":["selector"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-browser-get-content"), server_id: server_id.to_string(), name: "browser_get_content".into(), description: Some("Get full HTML content of the page".into()), input_schema_json: Some(r#"{"type":"object","properties":{}}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-browser-select"), server_id: server_id.to_string(), name: "browser_select".into(), description: Some("Select a dropdown option".into()), input_schema_json: Some(r#"{"type":"object","properties":{"selector":{"type":"string"},"value":{"type":"string"}},"required":["selector","value"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-browser-wait-for"), server_id: server_id.to_string(), name: "browser_wait_for".into(), description: Some("Wait for an element to appear".into()), input_schema_json: Some(r#"{"type":"object","properties":{"selector":{"type":"string"},"timeout":{"type":"integer"}},"required":["selector"]}"#.into()), },
        ],
        "@axagent/memory" => vec![
            ToolDescriptor { id: format!("{server_id}-memory-flush"), server_id: server_id.to_string(), name: "memory_flush".into(), description: Some("Persist an insight to long-term memory".into()), input_schema_json: Some(r#"{"type":"object","properties":{"content":{"type":"string"},"target":{"type":"string","enum":["memory","user"]},"category":{"type":"string","enum":["insight","decision","error_solution","preference","pattern","workflow"]}},"required":["content"]}"#.into()), },
        ],
        "@axagent/image-gen" => vec![
            ToolDescriptor { id: format!("{server_id}-generate-image"), server_id: server_id.to_string(), name: "generate_image".into(), description: Some("Generate an image from a text prompt".into()), input_schema_json: Some(r#"{"type":"object","properties":{"prompt":{"type":"string"},"provider":{"type":"string","enum":["flux","dall-e"]},"width":{"type":"integer"},"height":{"type":"integer"},"steps":{"type":"integer"},"seed":{"type":"integer"},"api_key":{"type":"string"}},"required":["prompt"]}"#.into()), },
        ],
        "@axagent/chart-gen" => vec![
            ToolDescriptor { id: format!("{server_id}-generate-chart-config"), server_id: server_id.to_string(), name: "generate_chart_config".into(), description: Some("Generate an ECharts config from description".into()), input_schema_json: Some(r#"{"type":"object","properties":{"description":{"type":"string"},"data":{"type":"object"},"chart_type":{"type":"string"},"title":{"type":"string"},"api_key":{"type":"string"},"base_url":{"type":"string"},"model":{"type":"string"}},"required":["description"]}"#.into()), },
        ],
        "@axagent/code-edit" => vec![
            ToolDescriptor { id: format!("{server_id}-search-replace"), server_id: server_id.to_string(), name: "search_replace".into(), description: Some("Search and replace text in a file".into()), input_schema_json: Some(r#"{"type":"object","properties":{"path":{"type":"string"},"old_str":{"type":"string"},"new_str":{"type":"string"},"start_line":{"type":"integer"},"end_line":{"type":"integer"},"replace_all":{"type":"boolean"}},"required":["path","old_str","new_str"]}"#.into()), },
        ],
        "@axagent/git" => vec![
            ToolDescriptor { id: format!("{server_id}-git-status"), server_id: server_id.to_string(), name: "git_status".into(), description: Some("Get the current git status".into()), input_schema_json: Some(r#"{"type":"object","properties":{"repo_path":{"type":"string"}},"required":["repo_path"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-git-diff"), server_id: server_id.to_string(), name: "git_diff".into(), description: Some("Get staged or branch changes summary".into()), input_schema_json: Some(r#"{"type":"object","properties":{"repo_path":{"type":"string"},"base_branch":{"type":"string"}},"required":["repo_path"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-git-commit"), server_id: server_id.to_string(), name: "git_commit".into(), description: Some("Stage all changes and commit".into()), input_schema_json: Some(r#"{"type":"object","properties":{"repo_path":{"type":"string"},"message":{"type":"string"},"stage_all":{"type":"boolean"}},"required":["repo_path","message"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-git-log"), server_id: server_id.to_string(), name: "git_log".into(), description: Some("Get recent commit history".into()), input_schema_json: Some(r#"{"type":"object","properties":{"repo_path":{"type":"string"},"max_count":{"type":"integer"}},"required":["repo_path"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-git-branch"), server_id: server_id.to_string(), name: "git_branch".into(), description: Some("List or create git branches".into()), input_schema_json: Some(r#"{"type":"object","properties":{"repo_path":{"type":"string"},"action":{"type":"string","enum":["list","create","switch"]},"name":{"type":"string"}},"required":["repo_path"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-git-review"), server_id: server_id.to_string(), name: "git_review".into(), description: Some("Generate a code review context summary".into()), input_schema_json: Some(r#"{"type":"object","properties":{"repo_path":{"type":"string"},"base_branch":{"type":"string"}},"required":["repo_path"]}"#.into()), },
        ],
        "@axagent/cron" => vec![
            ToolDescriptor { id: format!("{server_id}-cron-add"), server_id: server_id.to_string(), name: "cron_add".into(), description: Some("Schedule a new recurring cron job".into()), input_schema_json: Some(r#"{"type":"object","properties":{"name":{"type":"string"},"schedule":{"type":"string"},"prompt":{"type":"string"}},"required":["name","schedule","prompt"]}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-cron-list"), server_id: server_id.to_string(), name: "cron_list".into(), description: Some("List all scheduled cron jobs".into()), input_schema_json: Some(r#"{"type":"object","properties":{}}"#.into()), },
            ToolDescriptor { id: format!("{server_id}-cron-delete"), server_id: server_id.to_string(), name: "cron_delete".into(), description: Some("Delete a scheduled cron job".into()), input_schema_json: Some(r#"{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}"#.into()), },
        ],

        _ => vec![],
    }
}

/// Find which MCP server owns a given tool, searching across the provided server IDs.
pub async fn find_server_for_tool(
    db: &DatabaseConnection,
    tool_name: &str,
    server_ids: &[String],
) -> Result<Option<(McpServer, ToolDescriptor)>> {
    for server_id in server_ids {
        if let Ok(tools) = list_tools_for_server(db, server_id).await {
            if let Some(td) = tools.into_iter().find(|t| t.name == tool_name) {
                if let Ok(server) = get_mcp_server(db, server_id).await {
                    return Ok(Some((server, td)));
                }
            }
        }
    }
    Ok(None)
}
