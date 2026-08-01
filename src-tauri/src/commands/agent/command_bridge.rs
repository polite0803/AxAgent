// SPDX-License-Identifier: AGPL-3.0-only

//! Tauri 命令桥接器 — 将现有 Tauri 命令注册为 Agent 可调用的 Tool
//!
//! 设计原则：
//! - 只读命令直接暴露给 Agent
//! - 写入命令需要人工确认（前端通过 AgentContext 感知）
//! - 直接调用 DAO 层，避免不必要的序列化/反序列化
//! - 通过 SkillToolHandler 机制注册到 UnifiedToolRegistry

use axagent_harness::types::{ChatTool, ChatToolFunction};
use axagent_tools::registry::SkillToolHandler;
use sea_orm::DatabaseConnection;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::{Mutex as AsyncMutex, oneshot};
use tracing::{debug, instrument, warn};

/// Tauri 命令工具的元数据定义
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TauriCommandToolDef {
    /// 工具名称
    pub name: &'static str,
    /// 工具描述（给 LLM 看）
    pub description: &'static str,
    /// 输入参数的 JSON Schema
    pub input_schema: Value,
    /// 是否只读操作
    pub is_read_only: bool,
}

/// 构建可注册到 Agent 的 Tauri 命令工具列表
///
/// 使用函数而非静态变量，因为 serde_json::json! 宏在静态上下文中不可用。
pub fn build_tool_defs() -> Vec<TauriCommandToolDef> {
    vec![
        // ── 设置（只读） ──
        TauriCommandToolDef {
            name: "tauri_get_settings",
            description: "获取当前应用的完整设置，包括主题、语言、遥测级别等",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
            is_read_only: true,
        },
        // ── 设置（写入） ──
        TauriCommandToolDef {
            name: "tauri_save_settings",
            description: "保存应用设置。支持部分更新（主题模式、语言等）。此操作会立即生效。",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "theme_mode": { "type": "string", "description": "主题模式 (light/dark/system)" },
                    "theme_preset": { "type": "string", "description": "主题预设 (deep-dusk/oceanic-dark 等)" },
                    "language": { "type": "string", "description": "语言代码 (zh-CN/en-US 等)" },
                },
            }),
            is_read_only: false,
        },
        // ── 会话（只读） ──
        TauriCommandToolDef {
            name: "tauri_list_conversations",
            description: "列出所有会话，按更新时间倒序排列。返回会话 ID、标题、更新时间等",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
            is_read_only: true,
        },
        TauriCommandToolDef {
            name: "tauri_get_conversation",
            description: "获取单个会话的详细信息，包括标题、创建时间、更新时间、是否置顶等",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "conversation_id": { "type": "string", "description": "会话 ID" },
                },
                "required": ["conversation_id"],
            }),
            is_read_only: true,
        },
        // ── 知识库（只读） ──
        TauriCommandToolDef {
            name: "tauri_list_knowledge_bases",
            description: "列出所有知识库，包括名称、类型、描述等信息",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
            is_read_only: true,
        },
        // ── 记忆（只读） ──
        TauriCommandToolDef {
            name: "tauri_list_memories",
            description: "列出记忆条目，支持按重要性过滤。返回记忆内容、重要性分数等",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "min_importance": { "type": "number", "description": "最低重要性阈值 (0.0-1.0)" },
                    "limit": { "type": "integer", "description": "最大返回数量 (默认 20)" },
                },
            }),
            is_read_only: true,
        },
        // ── Agent UI 渲染（写入） ──
        TauriCommandToolDef {
            name: "tauri_render_ui",
            description: "在前端 Agent 面板中渲染一个动态 UI 组件。接收 UISchema JSON 定义，由前端 DynamicUIRenderer 渲染。支持容器、表单、表格、图表等组件类型",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "schema": { "type": "object", "description": "UISchema 定义，包含 version/id/type/props/children 等字段" },
                    "target_id": { "type": "string", "description": "目标容器 ID，用于定位渲染位置" },
                    "replace": { "type": "boolean", "description": "是否替换已存在的同名组件 (默认 true)" },
                },
                "required": ["schema"],
            }),
            is_read_only: false,
        },
        // ── Agent UI 更新（写入） ──
        TauriCommandToolDef {
            name: "tauri_update_ui",
            description: "更新已渲染的 Agent UI 组件。支持 replace/append/remove 三种操作模式",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": { "type": "string", "enum": ["replace", "append", "remove"], "description": "操作类型" },
                    "schema_id": { "type": "string", "description": "要更新的 Schema ID" },
                    "new_schema": { "type": "object", "description": "新的 UISchema (replace/append 时必填)" },
                    "path": { "type": "string", "description": "更新路径 (如 root.children[0])" },
                },
                "required": ["operation", "schema_id"],
            }),
            is_read_only: false,
        },
        // ── Agent UI 销毁（写入） ──
        TauriCommandToolDef {
            name: "tauri_remove_ui",
            description: "移除已渲染的 Agent UI 组件",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "schema_id": { "type": "string", "description": "要移除的 Schema ID" },
                },
                "required": ["schema_id"],
            }),
            is_read_only: false,
        },
    ]
}

/// 将工具定义转换为 ChatTool 列表
pub fn build_chat_tools() -> Vec<ChatTool> {
    build_tool_defs()
        .into_iter()
        .map(|def| ChatTool {
            r#type: "function".to_string(),
            function: ChatToolFunction {
                name: def.name.to_string(),
                description: Some(def.description.to_string()),
                parameters: Some(def.input_schema),
            },
        })
        .collect()
}

/// 为每个工具创建 SkillToolHandler
///
/// 在 handler 内部通过 block_in_place + block_on 调用异步 DAO 操作。
pub fn build_command_handlers(
    db: DatabaseConnection,
    app_handle: AppHandle,
) -> Vec<(String, SkillToolHandler)> {
    let mut handlers = Vec::new();

    for def in build_tool_defs() {
        let handler = create_handler(def.name, db.clone(), app_handle.clone());
        handlers.push((def.name.to_string(), handler));
    }

    handlers
}

/// 创建单个命令的 handler
fn create_handler(
    command_name: &str,
    db: DatabaseConnection,
    app_handle: AppHandle,
) -> SkillToolHandler {
    let name = command_name.to_string();
    Box::new(move |input: &str| {
        let input_value: Value =
            serde_json::from_str(input).unwrap_or_else(|_| serde_json::json!({}));

        execute_command(&name, &input_value, &db, &app_handle)
    })
}

/// 同步 handler 内部的执行逻辑
///
/// 安全地从同步上下文进入异步 runtime：
/// - 如果已在 tokio runtime 中，直接使用 Handle::current().block_on()
/// - 如果不在 runtime 中，创建临时 runtime 执行
fn execute_command(
    command_name: &str,
    input: &Value,
    db: &DatabaseConnection,
    app_handle: &AppHandle,
) -> Result<String, axagent_tools::ToolError> {
    let db = db.clone();
    let app = app_handle.clone();
    let name = command_name.to_string();

    // 安全地获取或创建 runtime 执行异步操作
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // 已在 tokio runtime 中，直接 block_on
            handle.block_on(async { dispatch_command(&name, input, &db, &app).await })
        },
        Err(_) => {
            // 不在 runtime 中，创建临时 runtime
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| axagent_tools::ToolError::execution_failed(command_name))?;
            runtime.block_on(async { dispatch_command(&name, input, &db, &app).await })
        },
    }
    .map_err(|_| axagent_tools::ToolError::execution_failed(command_name))
}

/// 命令分发 — 根据命令名调用对应的 DAO 操作
#[instrument(skip(db, app_handle), fields(command = %command_name))]
async fn dispatch_command(
    command_name: &str,
    input: &Value,
    db: &DatabaseConnection,
    app_handle: &AppHandle,
) -> Result<String, String> {
    debug!("Executing Tauri command: {}", command_name);

    match command_name {
        "tauri_get_settings" => {
            let settings = axagent_dao::repo::settings::get_settings(db).await.map_err(|e| {
                warn!("Failed to get settings: {}", e);
                format!("获取设置失败: {}", e)
            })?;
            serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())
        },
        "tauri_save_settings" => {
            let mut settings =
                axagent_dao::repo::settings::get_settings(db).await.map_err(|e| {
                    warn!("Failed to get settings for save: {}", e);
                    format!("获取设置失败: {}", e)
                })?;
            apply_settings_patch(&mut settings, input);
            axagent_dao::repo::settings::save_settings(db, &settings).await.map_err(|e| {
                warn!("Failed to save settings: {}", e);
                format!("保存设置失败: {}", e)
            })?;
            debug!("Settings saved successfully");
            Ok(serde_json::json!({ "success": true }).to_string())
        },
        "tauri_list_conversations" => {
            let convs =
                axagent_dao::repo::conversation::list_conversations(db).await.map_err(|e| {
                    warn!("Failed to list conversations: {}", e);
                    format!("列出会话失败: {}", e)
                })?;
            let summaries: Vec<_> = convs
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "title": c.title,
                        "updated_at": c.updated_at,
                        "is_pinned": c.is_pinned,
                    })
                })
                .collect();
            serde_json::to_string_pretty(&summaries).map_err(|e| e.to_string())
        },
        "tauri_get_conversation" => {
            let conv_id = input["conversation_id"]
                .as_str()
                .ok_or_else(|| "缺少 conversation_id 参数".to_string())?;
            let conv = axagent_dao::repo::conversation::get_conversation(db, conv_id)
                .await
                .map_err(|e| {
                    warn!("Failed to get conversation {}: {}", conv_id, e);
                    format!("获取会话失败: {}", e)
                })?;
            serde_json::to_string_pretty(&conv).map_err(|e| e.to_string())
        },
        "tauri_list_knowledge_bases" => {
            let kbs =
                axagent_dao::repo::knowledge::list_knowledge_bases(db).await.map_err(|e| {
                    warn!("Failed to list knowledge bases: {}", e);
                    format!("列出知识库失败: {}", e)
                })?;
            let summaries: Vec<_> = kbs
                .iter()
                .map(|kb| {
                    serde_json::json!({
                        "id": kb.id,
                        "name": kb.name,
                        "kind": kb.kind,
                        "enabled": kb.enabled,
                    })
                })
                .collect();
            serde_json::to_string_pretty(&summaries).map_err(|e| e.to_string())
        },
        "tauri_list_memories" => {
            let min_importance = input["min_importance"].as_f64();
            let limit = input["limit"].as_u64().map(|v| v as u32);
            let memories =
                axagent_dao::repo::memory::list_high_importance_items(db, min_importance, limit)
                    .await
                    .map_err(|e| {
                        warn!("Failed to list memories: {}", e);
                        format!("列出记忆失败: {}", e)
                    })?;
            let summaries: Vec<_> = memories
                .iter()
                .map(|m| {
                    let preview = if m.content.chars().count() > 100 {
                        format!("{}...", m.content.chars().take(100).collect::<String>())
                    } else {
                        m.content.clone()
                    };
                    serde_json::json!({
                        "id": m.id,
                        "title": m.title,
                        "importance": m.importance,
                        "content_preview": preview,
                        "tags": m.tags,
                    })
                })
                .collect();
            serde_json::to_string_pretty(&summaries).map_err(|e| e.to_string())
        },
        // ── Agent UI 渲染 ──
        "tauri_render_ui" => {
            let schema =
                input["schema"].as_object().ok_or_else(|| "缺少 schema 参数".to_string())?;
            let target_id = input["target_id"].as_str().map(|s| s.to_string());
            let replace = input["replace"].as_bool().unwrap_or(true);
            let schema_id = schema.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");

            let payload = serde_json::json!({
                "schema": schema,
                "targetId": target_id,
                "replace": replace,
            });

            app_handle.emit("agent-render-ui", &payload).map_err(|e| {
                warn!("Failed to emit agent-render-ui event: {}", e);
                format!("派发 UI 渲染事件失败: {}", e)
            })?;

            debug!("UI rendered: schemaId={}, replace={}", schema_id, replace);

            Ok(serde_json::json!({
                "success": true,
                "action": "render",
                "schemaId": schema_id,
            })
            .to_string())
        },
        "tauri_update_ui" => {
            let operation =
                input["operation"].as_str().ok_or_else(|| "缺少 operation 参数".to_string())?;
            let schema_id =
                input["schema_id"].as_str().ok_or_else(|| "缺少 schema_id 参数".to_string())?;
            let new_schema = input["new_schema"].as_object();
            let path = input["path"].as_str().map(|s| s.to_string());

            let payload = serde_json::json!({
                "operation": operation,
                "schemaId": schema_id,
                "newSchema": new_schema,
                "path": path,
            });

            app_handle.emit("agent-update-ui", &payload).map_err(|e| {
                warn!("Failed to emit agent-update-ui event: {}", e);
                format!("派发 UI 更新事件失败: {}", e)
            })?;

            debug!("UI updated: schemaId={}, operation={}", schema_id, operation);

            Ok(serde_json::json!({
                "success": true,
                "action": "update",
                "schemaId": schema_id,
                "operation": operation,
            })
            .to_string())
        },
        "tauri_remove_ui" => {
            let schema_id =
                input["schema_id"].as_str().ok_or_else(|| "缺少 schema_id 参数".to_string())?;

            let payload = serde_json::json!({
                "schemaId": schema_id,
            });

            app_handle.emit("agent-remove-ui", &payload).map_err(|e| {
                warn!("Failed to emit agent-remove-ui event: {}", e);
                format!("派发 UI 移除事件失败: {}", e)
            })?;

            debug!("UI removed: schemaId={}", schema_id);

            Ok(serde_json::json!({
                "success": true,
                "action": "remove",
                "schemaId": schema_id,
            })
            .to_string())
        },
        unknown => {
            warn!("Unknown Tauri command: {}", unknown);
            Err(format!("未知的 Tauri 命令: {}", unknown))
        },
    }
}

/// 将 input 中的字段应用到 AppSettings 上（部分更新）
///
/// 仅允许更新白名单内的字段，并对输入值做基本验证：
/// - theme_mode: 仅接受 "light" / "dark" / "system"
/// - language:  必须符合 xx-YY 格式（如 zh-CN, en-US）
/// - 数值字段:  范围检查
fn apply_settings_patch(settings: &mut axagent_harness::types::AppSettings, input: &Value) {
    // ── 字符串枚举字段 ──

    if let Some(theme_mode) = input["theme_mode"].as_str() {
        let valid = matches!(theme_mode, "light" | "dark" | "system");
        if valid {
            settings.theme_mode = theme_mode.to_string();
        } else {
            warn!("Invalid theme_mode '{}', must be light/dark/system, skipping", theme_mode);
        }
    }

    if let Some(theme_preset) = input["theme_preset"].as_str() {
        settings.theme_preset = theme_preset.to_string();
    }

    if let Some(language) = input["language"].as_str() {
        // 验证语言代码格式: xx-YY 或 xx
        let is_valid_lang =
            language.len() >= 2 && language.chars().next().is_some_and(|c| c.is_ascii_lowercase());
        if is_valid_lang {
            settings.language = language.to_string();
        } else {
            warn!(
                "Invalid language code '{}', must be a valid locale (e.g. zh-CN), skipping",
                language
            );
        }
    }

    // ── 可选的数值字段（常用设置） ──

    if let Some(primary_color) = input["primary_color"].as_str() {
        // 简单验证 hex 颜色格式
        if primary_color.starts_with('#') && (primary_color.len() == 7 || primary_color.len() == 4)
        {
            settings.primary_color = primary_color.to_string();
        } else {
            warn!(
                "Invalid primary_color '{}', must be hex color (e.g. #FF0000), skipping",
                primary_color
            );
        }
    }

    if let Some(font_size) = input["font_size"].as_u64() {
        if (10..=24).contains(&font_size) {
            settings.font_size = font_size as u8;
        } else {
            warn!("Invalid font_size {}, must be 10-24, skipping", font_size);
        }
    }

    if let Some(border_radius) = input["border_radius"].as_u64() {
        if (0..=20).contains(&border_radius) {
            settings.border_radius = border_radius as u8;
        } else {
            warn!("Invalid border_radius {}, must be 0-20, skipping", border_radius);
        }
    }

    // ── 布尔开关字段 ──

    if let Some(auto_start) = input["auto_start"].as_bool() {
        settings.auto_start = auto_start;
    }

    if let Some(show_on_start) = input["show_on_start"].as_bool() {
        settings.show_on_start = show_on_start;
    }

    if let Some(minimize_to_tray) = input["minimize_to_tray"].as_bool() {
        settings.minimize_to_tray = minimize_to_tray;
    }

    if let Some(always_on_top) = input["always_on_top"].as_bool() {
        settings.always_on_top = always_on_top;
    }

    if let Some(telemetry_level) = input["telemetry_level"].as_str() {
        let valid = matches!(telemetry_level, "off" | "minimal" | "full");
        if valid {
            settings.telemetry_level = telemetry_level.to_string();
        } else {
            warn!(
                "Invalid telemetry_level '{}', must be off/minimal/full, skipping",
                telemetry_level
            );
        }
    }
}

// ──────────────────────────────────────────────────────────────
// PermissionGate — 写操作确认机制
// ──────────────────────────────────────────────────────────────

/// 待确认的写操作请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct PendingRequest {
    /// 请求 ID（UUID）
    id: String,
    /// 工具名称
    tool_name: String,
    /// 操作描述（给用户看）
    description: String,
    /// 请求时间戳
    created_at: std::time::Instant,
}

/// 待确认请求队列的类型别名
type PendingMap = HashMap<String, (PendingRequest, oneshot::Sender<bool>)>;

/// PermissionGate — 管理写操作的用户确认
///
/// 工作流程：
/// 1. 后端派发 `agent-permission-required` 事件到前端
/// 2. 前端展示确认弹窗给用户
/// 3. 用户点击"允许"或"拒绝"
/// 4. 前端通过 `agent-permission-response` 事件回传结果
/// 5. 后端接收结果，决定是否执行写操作
#[allow(dead_code)]
pub struct PermissionGate {
    /// 待确认的请求队列（request_id → (request, sender)）
    pending: Arc<AsyncMutex<PendingMap>>,
    /// AppHandle，用于派发事件
    app_handle: AppHandle,
    /// 默认超时时间
    timeout: std::time::Duration,
}

impl PermissionGate {
    #[allow(dead_code)]
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            pending: Arc::new(AsyncMutex::new(HashMap::new())),
            app_handle,
            timeout: std::time::Duration::from_secs(30),
        }
    }

    /// 请求用户确认写操作
    ///
    /// 返回：Ok(true) = 用户允许，Ok(false) = 用户拒绝，Err = 超时或其他错误
    #[allow(dead_code)]
    pub async fn request_permission(
        &self,
        tool_name: &str,
        description: &str,
    ) -> Result<bool, String> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (sender, receiver) = oneshot::channel();

        let request = PendingRequest {
            id: request_id.clone(),
            tool_name: tool_name.to_string(),
            description: description.to_string(),
            created_at: std::time::Instant::now(),
        };

        // 存入待确认队列
        self.pending.lock().await.insert(request_id.clone(), (request, sender));

        // 派发事件到前端
        let payload = serde_json::json!({
            "requestId": request_id,
            "toolName": tool_name,
            "description": description,
        });

        self.app_handle.emit("agent-permission-required", &payload).map_err(|e| {
            warn!("Failed to emit permission request: {}", e);
            format!("派发权限请求失败: {}", e)
        })?;

        debug!("Permission request sent: id={}, tool={}", request_id, tool_name);

        // 等待前端响应（带超时）
        let result = tokio::time::timeout(self.timeout, receiver)
            .await
            .map_err(|_| "权限确认超时".to_string())?
            .map_err(|_| "权限确认通道错误".to_string())?;

        // 清理队列
        self.pending.lock().await.remove(&request_id);

        if result {
            debug!("Permission granted: id={}", request_id);
        } else {
            warn!("Permission denied: id={}, tool={}", request_id, tool_name);
        }

        Ok(result)
    }

    /// 处理前端的权限响应
    #[allow(dead_code)]
    pub async fn handle_response(&self, request_id: &str, approved: bool) -> Result<(), String> {
        let mut pending = self.pending.lock().await;

        if let Some((request, sender)) = pending.remove(request_id) {
            debug!(
                "Permission response received: id={}, approved={}, tool={}",
                request_id, approved, request.tool_name
            );

            sender.send(approved).map_err(|_| "响应通道已关闭".to_string())?;
            Ok(())
        } else {
            warn!("Permission response for unknown request: {}", request_id);
            Err(format!("未知的权限请求: {}", request_id))
        }
    }

    /// 创建 handler 时使用的方法
    #[allow(dead_code)]
    pub fn create_permission_check_handler(
        &self,
        tool_name: &'static str,
        description: &'static str,
    ) -> PermissionCheckHandler {
        PermissionCheckHandler {
            gate: self.pending.clone(),
            app_handle: self.app_handle.clone(),
            tool_name,
            description,
            timeout: self.timeout,
        }
    }
}

/// 权限检查 handler，用于在 SkillToolHandler 中执行前置检查
#[allow(dead_code)]
pub struct PermissionCheckHandler {
    gate: Arc<AsyncMutex<PendingMap>>,
    app_handle: AppHandle,
    tool_name: &'static str,
    description: &'static str,
    timeout: std::time::Duration,
}

impl PermissionCheckHandler {
    /// 执行权限检查
    ///
    /// 返回 Ok(true) 表示用户允许继续执行，Err 表示应中止操作
    #[allow(dead_code)]
    pub async fn check(&self) -> Result<(), String> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (sender, receiver) = oneshot::channel();

        let request = PendingRequest {
            id: request_id.clone(),
            tool_name: self.tool_name.to_string(),
            description: self.description.to_string(),
            created_at: std::time::Instant::now(),
        };

        // 存入待确认队列
        self.gate.lock().await.insert(request_id.clone(), (request, sender));

        // 派发事件到前端
        let payload = serde_json::json!({
            "requestId": request_id,
            "toolName": self.tool_name,
            "description": self.description,
        });

        self.app_handle.emit("agent-permission-required", &payload).map_err(|e| {
            warn!("Failed to emit permission request: {}", e);
            format!("派发权限请求失败: {}", e)
        })?;

        debug!("Permission request sent: id={}, tool={}", request_id, self.tool_name);

        // 等待前端响应（带超时）
        let result = tokio::time::timeout(self.timeout, receiver)
            .await
            .map_err(|_| "权限确认超时".to_string())?
            .map_err(|_| "权限确认通道错误".to_string())?;

        // 清理队列
        self.gate.lock().await.remove(&request_id);

        if result {
            debug!("Permission granted: id={}", request_id);
            Ok(())
        } else {
            warn!("Permission denied: id={}, tool={}", request_id, self.tool_name);
            Err(format!("操作被用户拒绝: {}", self.description))
        }
    }

    /// 处理前端的权限响应
    #[allow(dead_code)]
    pub async fn handle_response(
        gate: &Arc<AsyncMutex<PendingMap>>,
        request_id: &str,
        approved: bool,
    ) -> Result<(), String> {
        let mut pending = gate.lock().await;

        if let Some((request, sender)) = pending.remove(request_id) {
            debug!(
                "Permission response received: id={}, approved={}, tool={}",
                request_id, approved, request.tool_name
            );

            sender.send(approved).map_err(|_| "响应通道已关闭".to_string())?;
            Ok(())
        } else {
            warn!("Permission response for unknown request: {}", request_id);
            Err(format!("未知的权限请求: {}", request_id))
        }
    }
}
