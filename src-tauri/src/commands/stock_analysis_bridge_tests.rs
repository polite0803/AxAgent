// SPDX-License-Identifier: AGPL-3.0-only

//! [AxInvest] 股票命令桥接器集成测试（P4 验证）
//!
//! 直接驱动 `stock_analysis_bridge` 的 handler 闭包，验证：
//! - 读命令：stock_get_quote 走真实行情源（IPv4 东财）
//! - 写命令：stock_add_to_watchlist → stock_list_watchlist DB 闭环
//!
//! 注：权限审批（ask 规则 → agent-permission-request → agent_approve）
//! 由 runtime 层（conversation.rs authorize_with_context）保证，不在此层验证。
//! 本测试仅验证 handler 自身执行正确性。
//!
//! 为什么用普通 `#[test]` 而非 `#[tokio::test]`：
//! handler 的 `execute_stock_command` 在已有 runtime 内（Handle::try_current()=Some）
//! 会走 `handle.block_on` 嵌套阻塞 → panic。真实执行路径（conversation.rs
//! `execute_tool_threaded`）把 handler 放在独立线程，`try_current()` 返回 Err →
//! 走临时 current_thread runtime 分支。普通 #[test] 无 runtime 上下文，
//! 恰好复现该路径，无嵌套问题。

use crate::commands::stock_analysis_bridge::{
    build_stock_chat_tools, build_stock_command_handlers,
};
use sea_orm::ConnectionTrait;

/// 构造测试环境：真实 AStockClient + 临时文件 SQLite（同步入口，内部一次性 runtime）
///
/// 用临时文件而非 `:memory:`：handler 执行时在临时 runtime 内走连接池，
/// `:memory:` 库每个连接相互隔离（不同连接看不到建表），文件库所有连接共享。
fn test_env() -> (
    std::sync::Arc<axagent_astock_data::AStockClient>,
    sea_orm::DatabaseConnection,
    std::sync::Arc<axagent_runtime_core::CronJobStore>,
) {
    let client = std::sync::Arc::new(axagent_astock_data::AStockClient::new());
    let rt = tokio::runtime::Runtime::new().expect("创建测试 runtime 失败");
    let (db, _tmp_path) = rt.block_on(async {
        let path =
            std::env::temp_dir().join(format!("axinvest_bridge_test_{}.db", uuid::Uuid::new_v4()));
        // 与 dao::create_pool 相同的 URL 约定：sqlite:{path}?mode=rwc
        // （Windows 路径用正斜杠，避免 URL 解析问题）
        let url = format!("sqlite:{}?mode=rwc", path.to_string_lossy().replace('\\', "/"));
        let opts = sea_orm::ConnectOptions::new(url);
        let conn = sea_orm::Database::connect(opts).await.expect("连接测试 DB 失败");
        // 建 watchlist_items 表（与本地 v200 迁移 DDL 一致）
        conn.execute_raw(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "CREATE TABLE IF NOT EXISTS watchlist_items (\
                id TEXT NOT NULL PRIMARY KEY, \
                stock_code TEXT NOT NULL, stock_name TEXT NOT NULL, \
                notes TEXT, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        ))
        .await
        .expect("建 watchlist_items 测试表失败");
        (conn, path)
    });
    let cron = std::sync::Arc::new(axagent_runtime_core::CronJobStore::new_ephemeral());
    (client, db, cron)
}

#[test]
fn test_tool_defs_unique_and_registered() {
    // 工具定义与 handler 一一对应，名称唯一
    let defs = crate::commands::stock_analysis_bridge::build_stock_tool_defs();
    let mut names: Vec<&str> = defs.iter().map(|d| d.name).collect();
    let unique_len = names.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(names.len(), unique_len, "工具名必须唯一");
    names.sort_unstable();

    let chat_tools = build_stock_chat_tools();
    assert_eq!(chat_tools.len(), defs.len(), "chat_tools 数量与工具定义一致");

    // 写命令全部声明 is_read_only=false，且均在 STOCK_WRITE_TOOLS 名单中
    let mut write_tools: Vec<&str> =
        defs.iter().filter(|d| !d.is_read_only).map(|d| d.name).collect();
    write_tools.sort_unstable();
    let mut expected: Vec<&str> =
        crate::commands::stock_analysis_bridge::STOCK_WRITE_TOOLS.to_vec();
    expected.sort_unstable();
    assert_eq!(write_tools, expected, "is_read_only=false 的工具必须全部在 STOCK_WRITE_TOOLS");

    // P5: stock_render_ui 必须存在且为只读，schema 含推荐形态说明
    let render =
        defs.iter().find(|d| d.name == "stock_render_ui").expect("stock_render_ui 工具必须存在");
    assert!(render.is_read_only, "stock_render_ui 应为只读工具（仅 emit 渲染事件）");
    assert!(render.description.contains("UISchema"), "描述应包含 UISchema 指引");
    assert!(render.input_schema["properties"]["schema"].is_object(), "schema 参数应为对象");

    println!(
        "工具注册验证通过: {} read + {} write（含 stock_render_ui）",
        names.len() - expected.len(),
        expected.len()
    );
}

/// 构造测试 AppHandle（tauri mock runtime）
fn mock_app_handle() -> tauri::AppHandle<tauri::test::MockRuntime> {
    tauri::test::mock_app().handle().clone()
}

#[test]
fn test_stock_get_quote_read_command() {
    let (client, db, cron) = test_env();
    let app_handle = mock_app_handle();
    let handlers = build_stock_command_handlers(client, db, cron, app_handle);

    // 定位 stock_get_quote handler 并调用
    let handler = handlers
        .iter()
        .find(|(name, _)| name == "stock_get_quote")
        .expect("stock_get_quote handler 必须存在");
    let input = r#"{"stock_code": "600519"}"#;
    let result = (handler.1)(input);
    match result {
        Ok(json_str) => {
            let v: serde_json::Value =
                serde_json::from_str(&json_str).expect("返回必须是合法 JSON");
            assert_eq!(v["code"], "600519", "返回的股票代码应匹配输入");
            println!("stock_get_quote 返回: {}", json_str.chars().take(200).collect::<String>());
        },
        Err(e) => {
            // 行情源可能因网络/非交易时段失败，此处不硬性断言成功，
            // 但必须返回结构化错误而非 panic。
            println!("stock_get_quote 失败（可能网络/时段）: {e}");
        },
    }
}

#[test]
fn test_watchlist_write_read_roundtrip() {
    let (client, db, cron) = test_env();
    let app_handle = mock_app_handle();
    let handlers = build_stock_command_handlers(client, db, cron, app_handle);

    let add = handlers
        .iter()
        .find(|(name, _)| name == "stock_add_to_watchlist")
        .expect("add handler 必须存在");
    let list = handlers
        .iter()
        .find(|(name, _)| name == "stock_list_watchlist")
        .expect("list handler 必须存在");

    // 添加自选股
    let add_input = r#"{"stock_code": "600519", "stock_name": "贵州茅台", "notes": "test-group"}"#;
    let add_result = (add.1)(add_input).expect("添加自选股应成功");
    let added: serde_json::Value = serde_json::from_str(&add_result).expect("合法 JSON");
    assert_eq!(added["stockCode"], "600519", "实体 serde camelCase 字段名");

    // 查询列表，应包含刚添加的
    let list_result = (list.1)("{}").expect("列表查询应成功");
    let items: Vec<serde_json::Value> = serde_json::from_str(&list_result).expect("合法 JSON 数组");
    assert_eq!(items.len(), 1, "列表应包含 1 条");
    assert_eq!(items[0]["stockCode"], "600519", "serde camelCase 字段名");

    // 移除自选股，列表应为空
    let id = items[0]["id"].as_str().unwrap().to_string();
    let remove = handlers
        .iter()
        .find(|(name, _)| name == "stock_remove_from_watchlist")
        .expect("remove handler 必须存在");
    let remove_input = format!(r#"{{"id": "{id}"}}"#);
    (remove.1)(&remove_input).expect("移除自选股应成功");

    let list_after = (list.1)("{}").expect("列表查询应成功");
    let items_after: Vec<serde_json::Value> =
        serde_json::from_str(&list_after).expect("合法 JSON 数组");
    assert_eq!(items_after.len(), 0, "移除后列表应为空");
}
