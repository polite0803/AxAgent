use sea_orm::*;

use crate::entity::{mcp_servers, tool_descriptors};
use crate::error::{AxAgentError, Result};
use crate::repo::settings;
use crate::types::{CreateMcpServerInput, McpServer, ToolDescriptor};
use crate::utils::gen_id;

// ── Builtin MCP server definitions (not stored in DB, 19 remaining) ──────
//
// 以下 10 个原 MCP 内置服务器已迁移到本地 Rust 实现，不再重复注册：
//   builtin-fetch, builtin-search-file, builtin-skills, builtin-session,
//   builtin-search, builtin-filesystem, builtin-system, builtin-knowledge,
//   builtin-storage, builtin-memory
// 参见 local_tool::BUILTIN_GROUP_DEFS 和 UnifiedToolRegistry。

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
const BUILTIN_IMAGEGEN_ID: &str = "builtin-image-gen";
const BUILTIN_CHARTGEN_ID: &str = "builtin-chart-gen";
const BUILTIN_CODEEDIT_ID: &str = "builtin-code-edit";
const BUILTIN_GIT_ID: &str = "builtin-git";
const BUILTIN_CRON_ID: &str = "builtin-cron";

struct BuiltinDef {
    id: &'static str,
    name: &'static str,
    alias: &'static str,
    description: &'static str,
    default_enabled: bool,
}

const BUILTIN_DEFS: &[BuiltinDef] = &[
    // 注意：fetch, search-file, skills, session, search, filesystem, system,
    // knowledge, storage, memory 已迁移到本地 Rust 实现（local_tool.rs），
    // 此处不再作为 MCP 服务器列出。
    BuiltinDef {
        id: BUILTIN_BRAVE_SEARCH_ID,
        name: "@axagent/brave-search",
        alias: "Brave 搜索",
        description: "通过 Brave Search API 搜索网页和本地信息",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_SEQUENTIAL_THINKING_ID,
        name: "@axagent/sequential-thinking",
        alias: "深度思考",
        description: "通过多步推理分析复杂问题",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_PYTHON_ID,
        name: "@axagent/python",
        alias: "Python 执行",
        description: "在沙箱中安全执行 Python 代码",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_DIFY_KNOWLEDGE_ID,
        name: "@axagent/dify-knowledge",
        alias: "Dify 知识库",
        description: "搜索 Dify 平台知识库",
        default_enabled: false,
    },
    BuiltinDef {
        id: BUILTIN_WORKSPACE_MEMORY_ID,
        name: "@axagent/workspace-memory",
        alias: "工作区记忆",
        description: "读写工作区持久化记忆",
        default_enabled: false,
    },
    BuiltinDef {
        id: BUILTIN_FILEUTILS_ID,
        name: "@axagent/file-utils",
        alias: "文件工具",
        description: "PDF 信息、编码检测、Base64 图片",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_CACHE_ID,
        name: "@axagent/cache",
        alias: "缓存管理",
        description: "查看和清理系统缓存",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_OCR_ID,
        name: "@axagent/ocr",
        alias: "OCR 识别",
        description: "图片文字识别和语言检测",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_OBSIDIAN_ID,
        name: "@axagent/obsidian",
        alias: "Obsidian 笔记",
        description: "读取 Obsidian 笔记库文件",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_EXPORT_ID,
        name: "@axagent/export",
        alias: "文档导出",
        description: "导出为 Word 文档",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_REMOTEFILE_ID,
        name: "@axagent/remotefile",
        alias: "远程文件",
        description: "上传、列出、删除远程文件",
        default_enabled: false,
    },
    BuiltinDef {
        id: BUILTIN_AGENTCTRL_ID,
        name: "@axagent/agent-control",
        alias: "Agent 控制",
        description: "检查点、状态查询、记忆持久化",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_COMPUTER_ID,
        name: "@axagent/computer-control",
        alias: "计算机控制",
        description: "屏幕截图和鼠标键盘操作",
        default_enabled: false,
    },
    BuiltinDef {
        id: BUILTIN_BROWSER_ID,
        name: "@axagent/browser",
        alias: "浏览器控制",
        description: "浏览器导航、截图、点击、填表",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_IMAGEGEN_ID,
        name: "@axagent/image-gen",
        alias: "图片生成",
        description: "通过 AI 生成图片",
        default_enabled: false,
    },
    BuiltinDef {
        id: BUILTIN_CHARTGEN_ID,
        name: "@axagent/chart-gen",
        alias: "图表生成",
        description: "通过 AI 生成图表配置",
        default_enabled: true,
    },
    BuiltinDef {
        id: BUILTIN_CODEEDIT_ID,
        name: "@axagent/code-edit",
        alias: "代码编辑",
        description: "精确的代码编辑和替换",
        default_enabled: false,
    },
    BuiltinDef {
        id: BUILTIN_GIT_ID,
        name: "@axagent/git",
        alias: "Git 操作",
        description: "查看状态、差异、提交、日志、分支",
        default_enabled: false,
    },
    BuiltinDef {
        id: BUILTIN_CRON_ID,
        name: "@axagent/cron",
        alias: "定时任务",
        description: "添加、列表、删除定时任务",
        default_enabled: false,
    },
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
    // 内置 MCP 服务器中已有 10 个迁移到 UnifiedToolRegistry（本地 Rust 实现），
    // Git/GitHub 可通过 run_command 间接使用。仅保留 Puppeteer 作为预设 MCP。
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
        alias: None,
        description: None,
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
        alias: Some(def.alias.to_string()),
        description: Some(def.description.to_string()),
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
        alias: m.alias,
        description: m.description,
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
                alias: Set(server.alias.clone()),
                description: Set(server.description.clone()),
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
        alias: Set(input.alias),
        description: Set(input.description),
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

    let alias = input.alias.or(existing.alias);
    let description = input.description.or(existing.description);

    let mut am: mcp_servers::ActiveModel = model.into();
    am.name = Set(name);
    am.alias = Set(alias);
    am.description = Set(description);
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
        return Err(AxAgentError::Gateway("Cannot delete built-in MCP server".to_string()));
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
    // 注意：fetch, search-file, skills, session, search, filesystem, system,
    // knowledge, storage, memory 已迁移到 UnifiedToolRegistry，此处不再提供
    // MCP 描述符。
    match server_name {
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
