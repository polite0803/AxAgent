// SPDX-License-Identifier: AGPL-3.0-only

//! 股票命令桥接器 — 将本地股票业务 Tauri 命令注册为 Agent 可调用的 Tool
//!
//! [AxInvest 本地专属] 设计对齐上游 `commands/agent/command_bridge.rs`，但：
//! - 上游桥接器只收 `(db, app_handle)`；股票命令依赖 `Arc<AStockClient>` + `db`
//! - 只读命令直接暴露给 Agent；写命令的权限审批**复用 runtime 原生链路**
//!   （`PermissionPolicy` ask 规则 → `ChannelPermissionPrompter` emit
//!   `agent-permission-request` → 前端弹窗 → `agent_approve` 回传），
//!   本模块不自行实现门控（上游 command_bridge 的 PermissionGate 是死代码）。
//! - 直接调用 `AStockClient` 底层方法与 DAO，与 Tauri 命令共用同一份实现（零重复）
//!
//! 合并纪律：本文件为 AxInvest 本地新增，上游无此文件 → 永不冲突。
//! 上游共享文件 `command_bridge.rs` 保持不动。

use crate::commands::agent::command_bridge::TauriCommandToolDef;
use axagent_astock_data::AStockClient;
use axagent_astock_data::as_of::{self, AsOfContext};
use axagent_entities::{portfolio_holdings, price_alerts, watchlist_items};
use axagent_harness::types::{ChatTool, ChatToolFunction};
use axagent_tools::ToolError;
use axagent_tools::registry::SkillToolHandler;
use sea_orm::ActiveModelTrait;
use sea_orm::DatabaseConnection;
use sea_orm::EntityTrait;
use sea_orm::QueryOrder;
use serde_json::Value;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tracing::{debug, instrument, warn};

/// 股票写命令名单（is_read_only=false）
///
/// 供 `PermissionPolicy` ask 规则注入使用：这些工具名会触发
/// runtime 原生审批（`agent-permission-request` → 前端弹窗 → `agent_approve`）。
pub const STOCK_WRITE_TOOLS: &[&str] = &[
    "stock_add_to_watchlist",
    "stock_remove_from_watchlist",
    "stock_add_portfolio_holding",
    "stock_remove_portfolio_holding",
    "stock_create_price_alert",
    "stock_toggle_trading_enabled",
    "stock_create_stock_cron",
    "stock_delete_stock_cron",
];

/// 构建可注册到 Agent 的股票命令工具列表
///
/// 命名空间 `stock_` 前缀，与上游 `tauri_` 前缀区分，避免工具名冲突。
pub fn build_stock_tool_defs() -> Vec<TauriCommandToolDef> {
    vec![
        // ── 行情（只读） ──
        TauriCommandToolDef {
            name: "stock_get_quote",
            description: "获取指定股票的实时行情，包括最新价、涨跌幅、成交量、成交额、换手率、市盈率等。支持 as_of_date 参数查询历史时点行情",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "股票代码，如 600519、000001、00700.HK、AAPL.US" },
                    "as_of_date": { "type": "string", "description": "可选，历史时点日期 (YYYY-MM-DD)，查询该日收盘行情" },
                },
                "required": ["stock_code"],
            }),
            is_read_only: true,
        },
        TauriCommandToolDef {
            name: "stock_get_hot_stocks",
            description: "获取当日热门股票列表（人气榜），返回代码、名称、涨跌幅、成交额等",
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
            is_read_only: true,
        },
        TauriCommandToolDef {
            name: "stock_get_industry_ranking",
            description: "获取行业板块排名，按主力资金净流入排序，返回行业名称、涨跌幅、资金流向等",
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
            is_read_only: true,
        },
        TauriCommandToolDef {
            name: "stock_get_market_dragon_tiger",
            description: "获取龙虎榜数据（游资/机构席位动向），返回上榜股票、营业部买卖金额等",
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
            is_read_only: true,
        },
        TauriCommandToolDef {
            name: "stock_get_north_bound_flow",
            description: "获取北向资金（沪深股通）成交额数据。注意：北向净流入自 2024-08 起停止披露，返回字段为成交额（百万）而非净流入",
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
            is_read_only: true,
        },
        TauriCommandToolDef {
            name: "stock_get_index_quotes",
            description: "获取主要指数实时行情（上证指数、深证成指、创业板指等），返回点位、涨跌幅",
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
            is_read_only: true,
        },
        // ── 搜索（只读） ──
        TauriCommandToolDef {
            name: "stock_search",
            description: "按关键词搜索股票，返回匹配的股票代码与名称。支持按市场过滤（A/HK/US）",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "keyword": { "type": "string", "description": "股票名称或代码关键词，如 '茅台'、'600519'" },
                    "market": { "type": "string", "description": "可选，市场过滤：A=沪深A股, HK=港股, US=美股；缺省返回全部" },
                },
                "required": ["keyword"],
            }),
            is_read_only: true,
        },
        TauriCommandToolDef {
            name: "stock_search_news",
            description: "搜索与关键词相关的财经新闻，返回新闻标题、摘要、来源、发布时间",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "keyword": { "type": "string", "description": "搜索关键词，如 '贵州茅台'、'光伏'" },
                    "limit": { "type": "integer", "description": "可选，返回条数上限 (默认 20，最大 100)" },
                },
                "required": ["keyword"],
            }),
            is_read_only: true,
        },
        // ── 自选股（只读） ──
        TauriCommandToolDef {
            name: "stock_list_watchlist",
            description: "列出当前自选股列表，返回代码、名称、加入时间等",
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
            is_read_only: true,
        },
        // ── 写操作（is_read_only=false；审批走 runtime 原生链路） ──
        TauriCommandToolDef {
            name: "stock_add_to_watchlist",
            description: "将股票加入自选股。此操作会修改自选股列表，需要用户确认",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "股票代码，如 600519" },
                    "stock_name": { "type": "string", "description": "股票名称，如 贵州茅台" },
                    "notes": { "type": "string", "description": "可选备注/分组信息" },
                },
                "required": ["stock_code", "stock_name"],
            }),
            is_read_only: false,
        },
        TauriCommandToolDef {
            name: "stock_remove_from_watchlist",
            description: "将股票从自选股中移除。此操作会修改自选股列表，需要用户确认",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "自选股条目 ID（来自 stock_list_watchlist）" },
                },
                "required": ["id"],
            }),
            is_read_only: false,
        },
        TauriCommandToolDef {
            name: "stock_add_portfolio_holding",
            description: "添加一笔股票持仓（成本与数量）。此操作会修改持仓数据，需要用户确认",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "股票代码，如 600519" },
                    "stock_name": { "type": "string", "description": "股票名称，如 贵州茅台" },
                    "shares": { "type": "number", "description": "持仓股数" },
                    "avg_cost": { "type": "number", "description": "平均成本价" },
                },
                "required": ["stock_code", "stock_name", "shares", "avg_cost"],
            }),
            is_read_only: false,
        },
        TauriCommandToolDef {
            name: "stock_remove_portfolio_holding",
            description: "移除一笔股票持仓。此操作会修改持仓数据，需要用户确认",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "持仓条目 ID" },
                },
                "required": ["id"],
            }),
            is_read_only: false,
        },
        TauriCommandToolDef {
            name: "stock_create_price_alert",
            description: "创建价格提醒：当股票价格触及目标价时触发提醒。此操作会创建提醒规则，需要用户确认",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "股票代码，如 600519" },
                    "stock_name": { "type": "string", "description": "股票名称" },
                    "condition": { "type": "string", "description": "条件，如 'above'（高于）/ 'below'（低于）" },
                    "target_price": { "type": "number", "description": "目标价格" },
                },
                "required": ["stock_code", "stock_name", "condition", "target_price"],
            }),
            is_read_only: false,
        },
        TauriCommandToolDef {
            name: "stock_toggle_trading_enabled",
            description: "启用或停用交易功能开关。此操作影响交易系统可用性，需要用户确认",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "enabled": { "type": "boolean", "description": "true=启用, false=停用" },
                },
                "required": ["enabled"],
            }),
            is_read_only: false,
        },
        TauriCommandToolDef {
            name: "stock_create_stock_cron",
            description: "创建股票的定时分析任务（cron 表达式）。此操作会创建定时任务，需要用户确认",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "stock_code": { "type": "string", "description": "股票代码" },
                    "stock_name": { "type": "string", "description": "股票名称" },
                    "cron_expression": { "type": "string", "description": "cron 表达式，如 '0 18 * * *'（每天 18:00）" },
                },
                "required": ["stock_code", "stock_name", "cron_expression"],
            }),
            is_read_only: false,
        },
        TauriCommandToolDef {
            name: "stock_delete_stock_cron",
            description: "删除股票的定时分析任务。此操作会删除定时任务，需要用户确认",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "定时任务 ID" },
                },
                "required": ["id"],
            }),
            is_read_only: false,
        },
        // ── 动态 UI 渲染（P5）：把股票数据渲染为前端组件 ──
        TauriCommandToolDef {
            name: "stock_render_ui",
            description: "将股票数据渲染为前端 UI 组件，结果直接显示在股票工作区页面内（Agent 分析区）。\
                接收 UISchema JSON：{version, id, type, props, children?}。\
                ⚠️ target_id 必须传 \"stock-workspace\"，否则渲染会落到全局侧边栏而非股票页面。\
                推荐形态：\
                1) 行情卡片: type=Card, props={title:'贵州茅台 600519'}, children=[\
                   {type:'List', props:{items:[{label:'最新价',value:'1350.6'},{label:'涨跌幅',value:'-0.82%'},{label:'成交额',value:'73.7亿'}]}}]\
                2) 数据表格: type=Table, props={columns:[{title:'代码',dataIndex:'code'},{title:'名称',dataIndex:'name'},{title:'涨跌幅',dataIndex:'changePct'}],\
                   dataSource:[{code:'600519',name:'贵州茅台',changePct:-0.82}]}\
                3) 分析结论: type=Markdown, props={content:'## 结论\\n分析结果文本'}\
                组件类型: Container/Row/Column/Grid/Card/Tabs/Accordion/Table/Chart/List/Dashboard/Markdown/Form。\
                在股票分析、行情查询、回测结果等场景，主动调用本工具把结果渲染成可视化组件",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "schema": {
                        "type": "object",
                        "description": "UISchema 定义，必含 version/id/type，可选 props/children",
                    },
                    "target_id": { "type": "string", "description": "渲染目标容器 ID，必须传 \"stock-workspace\" 才显示在股票工作区" },
                    "replace": { "type": "boolean", "description": "可选，是否替换同名组件 (默认 true)" },
                },
                "required": ["schema"],
            }),
            is_read_only: true,
        },
    ]
}

/// 将股票工具定义转换为 ChatTool 列表
pub fn build_stock_chat_tools() -> Vec<ChatTool> {
    build_stock_tool_defs()
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

/// 为每个股票工具创建 SkillToolHandler
///
/// handler 持有 `Arc<AStockClient>`、`DatabaseConnection` 与 `cron_job_store`，
/// 直接调用底层方法（与 Tauri 命令共用实现，零重复）。
///
/// 注意：写命令的权限审批**不在此处实现**——runtime 工具执行循环
/// （conversation.rs `authorize_with_context`）会对每个工具统一授权，
/// 写工具经 `STOCK_WRITE_TOOLS` 注入 ask 规则后自动走
/// `agent-permission-request` → 前端弹窗 → `agent_approve` 链路。
/// 审批通过后 handler 才会执行，因此写命令分支是纯业务逻辑。
pub fn build_stock_command_handlers<R: tauri::Runtime>(
    client: Arc<AStockClient>,
    db: DatabaseConnection,
    cron_job_store: Arc<axagent_runtime_core::CronJobStore>,
    app_handle: AppHandle<R>,
) -> Vec<(String, SkillToolHandler)> {
    let mut handlers = Vec::new();

    for def in build_stock_tool_defs() {
        let handler = create_stock_handler(
            def.name,
            client.clone(),
            db.clone(),
            cron_job_store.clone(),
            app_handle.clone(),
        );
        handlers.push((def.name.to_string(), handler));
    }

    handlers
}

/// 创建单个股票命令的 handler
fn create_stock_handler<R: tauri::Runtime>(
    command_name: &str,
    client: Arc<AStockClient>,
    db: DatabaseConnection,
    cron_job_store: Arc<axagent_runtime_core::CronJobStore>,
    app_handle: AppHandle<R>,
) -> SkillToolHandler {
    let name = command_name.to_string();
    Box::new(move |input: &str| {
        let input_value: Value =
            serde_json::from_str(input).unwrap_or_else(|_| serde_json::json!({}));

        execute_stock_command(&name, &input_value, &client, &db, &cron_job_store, &app_handle)
    })
}

/// 同步 handler 内部的执行逻辑
///
/// 安全地从同步上下文进入异步 runtime（与上游 execute_command 同模式）：
/// - 已在 tokio runtime 中 → Handle::current().block_on()
/// - 不在 runtime 中 → 创建临时 current_thread runtime
fn execute_stock_command<R: tauri::Runtime>(
    command_name: &str,
    input: &Value,
    client: &Arc<AStockClient>,
    db: &DatabaseConnection,
    cron_job_store: &Arc<axagent_runtime_core::CronJobStore>,
    app_handle: &AppHandle<R>,
) -> Result<String, ToolError> {
    let client = client.clone();
    let db = db.clone();
    let cron_job_store = cron_job_store.clone();
    let app_handle = app_handle.clone();
    let name = command_name.to_string();

    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(async {
            dispatch_stock_command(&name, input, &client, &db, &cron_job_store, &app_handle).await
        }),
        Err(_) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| ToolError::execution_failed(command_name))?;
            runtime.block_on(async {
                dispatch_stock_command(&name, input, &client, &db, &cron_job_store, &app_handle)
                    .await
            })
        },
    }
    .map_err(|e| ToolError::execution_failed_for(command_name, e))
}

/// 命令分发 — 根据命令名调用 AStockClient 底层方法 / DAO
#[instrument(skip(client, db, cron_job_store, app_handle), fields(command = %command_name))]
async fn dispatch_stock_command<R: tauri::Runtime>(
    command_name: &str,
    input: &Value,
    client: &Arc<AStockClient>,
    db: &DatabaseConnection,
    cron_job_store: &Arc<axagent_runtime_core::CronJobStore>,
    app_handle: &AppHandle<R>,
) -> Result<String, String> {
    debug!("Executing stock command: {}", command_name);

    match command_name {
        "stock_get_quote" => {
            let stock_code =
                input["stock_code"].as_str().ok_or_else(|| "缺少 stock_code 参数".to_string())?;
            if stock_code.trim().is_empty() {
                return Err("stock_code 不能为空".to_string());
            }
            let as_of_ctx = AsOfContext::parse_optional(input["as_of_date"].as_str())
                .map_err(|e| format!("as_of_date 解析失败: {e}"))?;
            let quote = as_of::with_optional_asof(as_of_ctx, async {
                as_of::with_degradation_log(async { client.get_quote(stock_code).await }).await
            })
            .await
            .map_err(|e| format!("获取实时行情失败: {e}"))?;
            serde_json::to_string_pretty(&quote).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_get_hot_stocks" => {
            let data =
                client.get_hot_stocks().await.map_err(|e| format!("获取热门股票失败: {e}"))?;
            serde_json::to_string_pretty(&data).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_get_industry_ranking" => {
            let data = client
                .get_industry_ranking()
                .await
                .map_err(|e| format!("获取行业排名失败: {e}"))?;
            serde_json::to_string_pretty(&data).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_get_market_dragon_tiger" => {
            let data = client
                .get_market_dragon_tiger()
                .await
                .map_err(|e| format!("获取龙虎榜失败: {e}"))?;
            serde_json::to_string_pretty(&data).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_get_north_bound_flow" => {
            let data = client
                .get_north_bound_flow()
                .await
                .map_err(|e| format!("获取北向资金失败: {e}"))?;
            serde_json::to_string_pretty(&data).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_get_index_quotes" => {
            let data =
                client.get_index_quotes().await.map_err(|e| format!("获取指数行情失败: {e}"))?;
            serde_json::to_string_pretty(&data).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_search" => {
            let keyword =
                input["keyword"].as_str().ok_or_else(|| "缺少 keyword 参数".to_string())?;
            if keyword.trim().is_empty() {
                return Err("keyword 不能为空".to_string());
            }
            let market = input["market"].as_str();
            let results =
                client.search_stock(keyword).await.map_err(|e| format!("搜索股票失败: {e}"))?;
            let filtered = match market {
                Some("A") => results
                    .into_iter()
                    .filter(|r| !r.code.ends_with(".HK") && !r.code.ends_with(".US"))
                    .collect(),
                Some("HK") => results.into_iter().filter(|r| r.code.ends_with(".HK")).collect(),
                Some("US") => results.into_iter().filter(|r| r.code.ends_with(".US")).collect(),
                _ => results,
            };
            serde_json::to_string_pretty(&filtered).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_search_news" => {
            let keyword =
                input["keyword"].as_str().ok_or_else(|| "缺少 keyword 参数".to_string())?;
            if keyword.trim().is_empty() {
                return Err("keyword 不能为空".to_string());
            }
            let limit = input["limit"].as_u64().unwrap_or(20).min(100) as u32;
            let data = client
                .search_news(keyword, limit)
                .await
                .map_err(|e| format!("搜索新闻失败: {e}"))?;
            serde_json::to_string_pretty(&data).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_list_watchlist" => {
            let items = watchlist_items::Entity::find()
                .order_by_desc(watchlist_items::Column::CreatedAt)
                .all(db)
                .await
                .map_err(|e| format!("查询自选股列表失败: {e}"))?;
            serde_json::to_string_pretty(&items).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        // ── 写操作（P2）：审批由 runtime 原生链路负责，此处仅执行 ──
        "stock_add_to_watchlist" => {
            let stock_code =
                input["stock_code"].as_str().ok_or_else(|| "缺少 stock_code 参数".to_string())?;
            let stock_name =
                input["stock_name"].as_str().ok_or_else(|| "缺少 stock_name 参数".to_string())?;
            let notes = input["notes"].as_str().map(|s| s.to_string());
            let now = chrono::Utc::now().timestamp_millis();
            let model = watchlist_items::ActiveModel {
                id: sea_orm::Set(uuid::Uuid::new_v4().to_string()),
                stock_code: sea_orm::Set(stock_code.to_string()),
                stock_name: sea_orm::Set(stock_name.to_string()),
                notes: sea_orm::Set(notes),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            };
            let saved = model.insert(db).await.map_err(|e| format!("添加自选股失败: {e}"))?;
            serde_json::to_string_pretty(&saved).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_remove_from_watchlist" => {
            let id = input["id"].as_str().ok_or_else(|| "缺少 id 参数".to_string())?;
            watchlist_items::Entity::delete_by_id(id)
                .exec(db)
                .await
                .map_err(|e| format!("移除自选股失败: {e}"))?;
            serde_json::to_string_pretty(&serde_json::json!({ "success": true, "id": id })).map_err(
                |e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                },
            )
        },
        "stock_add_portfolio_holding" => {
            let stock_code =
                input["stock_code"].as_str().ok_or_else(|| "缺少 stock_code 参数".to_string())?;
            let stock_name =
                input["stock_name"].as_str().ok_or_else(|| "缺少 stock_name 参数".to_string())?;
            let shares = input["shares"].as_f64().ok_or_else(|| "缺少 shares 参数".to_string())?;
            let avg_cost =
                input["avg_cost"].as_f64().ok_or_else(|| "缺少 avg_cost 参数".to_string())?;
            let now = chrono::Utc::now().timestamp_millis();
            let model = portfolio_holdings::ActiveModel {
                id: sea_orm::Set(uuid::Uuid::new_v4().to_string()),
                stock_code: sea_orm::Set(stock_code.to_string()),
                stock_name: sea_orm::Set(stock_name.to_string()),
                shares: sea_orm::Set(shares),
                avg_cost: sea_orm::Set(avg_cost),
                notes: sea_orm::Set(None),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            };
            let saved = model.insert(db).await.map_err(|e| format!("添加持仓失败: {e}"))?;
            serde_json::to_string_pretty(&saved).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_remove_portfolio_holding" => {
            let id = input["id"].as_str().ok_or_else(|| "缺少 id 参数".to_string())?;
            portfolio_holdings::Entity::delete_by_id(id)
                .exec(db)
                .await
                .map_err(|e| format!("移除持仓失败: {e}"))?;
            serde_json::to_string_pretty(&serde_json::json!({ "success": true, "id": id })).map_err(
                |e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                },
            )
        },
        "stock_create_price_alert" => {
            let stock_code =
                input["stock_code"].as_str().ok_or_else(|| "缺少 stock_code 参数".to_string())?;
            let stock_name =
                input["stock_name"].as_str().ok_or_else(|| "缺少 stock_name 参数".to_string())?;
            let condition =
                input["condition"].as_str().ok_or_else(|| "缺少 condition 参数".to_string())?;
            let target_price = input["target_price"]
                .as_f64()
                .ok_or_else(|| "缺少 target_price 参数".to_string())?;

            use axagent_analysis_engine::alert_mapping::{
                condition_type_for, legacy_condition_to_alert_type,
            };
            let alert_type = legacy_condition_to_alert_type(condition)
                .unwrap_or(axagent_analysis_engine::alert_mapping::alert_types::TAKE_PROFIT);
            let condition_type = condition_type_for(alert_type);

            let now = chrono::Utc::now().timestamp_millis();
            let model = price_alerts::ActiveModel {
                id: sea_orm::Set(uuid::Uuid::new_v4().to_string()),
                stock_code: sea_orm::Set(stock_code.to_string()),
                stock_name: sea_orm::Set(stock_name.to_string()),
                condition: sea_orm::Set(condition.to_string()),
                target_price: sea_orm::Set(target_price),
                alert_type: sea_orm::Set(Some(alert_type.to_string())),
                condition_type: sea_orm::Set(Some(condition_type.to_string())),
                threshold: sea_orm::Set(Some(target_price)),
                is_triggered: sea_orm::Set(0),
                triggered_at: sea_orm::Set(None),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            };
            let saved = model.insert(db).await.map_err(|e| format!("创建价格提醒失败: {e}"))?;
            serde_json::to_string_pretty(&saved).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_toggle_trading_enabled" => {
            let enabled =
                input["enabled"].as_bool().ok_or_else(|| "缺少 enabled 参数".to_string())?;
            axagent_dao::repo::settings::set_setting(db, "trading_enabled", &enabled.to_string())
                .await
                .map_err(|e| format!("切换交易功能失败: {e}"))?;
            serde_json::to_string_pretty(
                &serde_json::json!({ "success": true, "enabled": enabled }),
            )
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_create_stock_cron" => {
            let stock_code =
                input["stock_code"].as_str().ok_or_else(|| "缺少 stock_code 参数".to_string())?;
            let stock_name =
                input["stock_name"].as_str().ok_or_else(|| "缺少 stock_name 参数".to_string())?;
            let cron_expression = input["cron_expression"]
                .as_str()
                .ok_or_else(|| "缺少 cron_expression 参数".to_string())?;
            let id = format!(
                "stock-{}-{}",
                stock_code,
                uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x")
            );
            let prompt = format!("对 {} ({}) 执行完整股票分析", stock_code, stock_name);
            let desc = format!("定时分析 {}", stock_code);
            let job = axagent_runtime_core::CronJob::new(&id, cron_expression, &prompt, &desc)
                .with_workflow_id("stock-analysis".to_string())
                .with_task_type("stock-analysis");
            cron_job_store.add(job.clone()).await;
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                "id": id,
                "stock_code": stock_code,
                "cron_expression": cron_expression,
            }))
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "stock_delete_stock_cron" => {
            let id = input["id"].as_str().ok_or_else(|| "缺少 id 参数".to_string())?;
            cron_job_store.remove(id).await;
            serde_json::to_string_pretty(&serde_json::json!({ "success": true, "id": id })).map_err(
                |e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                },
            )
        },
        // ── 动态 UI 渲染（P5）：emit agent-render-ui 事件，前端 AgentUIRenderer 渲染 ──
        "stock_render_ui" => {
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
                warn!("[stock-bridge] 派发 UI 渲染事件失败: {}", e);
                format!("派发 UI 渲染事件失败: {e}")
            })?;

            debug!("[stock-bridge] UI rendered: schemaId={}, replace={}", schema_id, replace);
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                "action": "render",
                "schemaId": schema_id,
            }))
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        other => {
            warn!("Unknown stock command: {}", other);
            Err(format!("未知股票命令: {other}"))
        },
    }
}
