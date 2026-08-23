// SPDX-License-Identifier: AGPL-3.0-only

//! 技能市场目录命令
//!
//! 桥接前端与 `axagent-dao::MarketplaceCatalogServiceImpl`。
//! 单用户桌面场景下「市场」= 本地工作流模板 + 本地技能目录的统一浏览。
//! 评分/评论 CRUD 走 `MarketplaceService` trait（`get_marketplace_item_stats` 复用其 `get_stats`）。

use axagent_agent_macro::agent_command;
use axagent_dao::marketplace_service::{MarketplaceCatalogServiceImpl, MarketplaceServiceImpl};
use axagent_harness::marketplace::{CatalogItem, CatalogPage, CatalogQuery, MarketplaceStats};
use axagent_harness::{MarketplaceCatalogService, MarketplaceService};
use tauri::State;

use crate::app_state::AppState;
use crate::commands::error::{ErrorCategory, ErrorResponse};
use crate::commands::error_code::common as common_err;
use crate::commands::error_code::marketplace as mkt_err;

/// 把 `ErrorResponse` 序列化为 String（前端 `JSON.parse(e.message)` 解析）。
fn err_to_string(e: ErrorResponse) -> String {
    e.to_string()
}

/// 浏览市场目录：合并本地工作流模板与本地技能。
#[agent_command(domain = marketplace, safety = Safe, call_mode = StateInput, description = "浏览市场目录")]
#[tauri::command]
pub async fn list_marketplace_catalog(
    state: State<'_, AppState>,
    query: Option<CatalogQuery>,
) -> Result<CatalogPage, String> {
    let db = state.harness.db();
    MarketplaceCatalogServiceImpl.list_catalog(db, query.unwrap_or_default()).await.map_err(|e| {
        err_to_string(
            ErrorResponse::new(common_err::INTERNAL)
                .with_category(ErrorCategory::Unrecoverable)
                .with_detail(e),
        )
    })
}

/// 按 ID + 类型查单个目录条目。
#[agent_command(domain = marketplace, safety = Safe, call_mode = StateInput, description = "获取市场目录条目")]
#[tauri::command]
pub async fn get_marketplace_item(
    state: State<'_, AppState>,
    item_id: String,
    item_type: String,
) -> Result<Option<CatalogItem>, String> {
    let db = state.harness.db();
    MarketplaceCatalogServiceImpl.get_catalog_item(db, &item_id, &item_type).await.map_err(|e| {
        err_to_string(
            ErrorResponse::new(common_err::INTERNAL)
                .with_category(ErrorCategory::Unrecoverable)
                .with_detail(e),
        )
    })
}

/// 安装模板：在 `workflow_marketplace` 表 upsert 记录并置 `is_public = true`。
#[agent_command(domain = marketplace, safety = Caution, call_mode = StateInput, description = "安装市场模板")]
#[tauri::command]
pub async fn install_marketplace_template(
    state: State<'_, AppState>,
    template_id: String,
) -> Result<(), String> {
    let db = state.harness.db();
    MarketplaceCatalogServiceImpl.install_template(db, &template_id).await.map_err(|e| {
        err_to_string(
            ErrorResponse::new(mkt_err::INSTALL_FAILED)
                .with_category(ErrorCategory::Unrecoverable)
                .with_detail(e),
        )
    })
}

/// 卸载模板：置 `workflow_marketplace.is_public = false`。
#[agent_command(domain = marketplace, safety = Caution, call_mode = StateInput, description = "卸载市场模板")]
#[tauri::command]
pub async fn uninstall_marketplace_template(
    state: State<'_, AppState>,
    template_id: String,
) -> Result<(), String> {
    let db = state.harness.db();
    MarketplaceCatalogServiceImpl.uninstall_template(db, &template_id).await.map_err(|e| {
        err_to_string(
            ErrorResponse::new(mkt_err::UNINSTALL_FAILED)
                .with_category(ErrorCategory::Unrecoverable)
                .with_detail(e),
        )
    })
}

/// 发布模板：更新 `workflow_marketplace` 表的 `category` 和 `tags` 字段。
#[agent_command(domain = marketplace, safety = Caution, call_mode = StateInput, description = "发布市场模板")]
#[tauri::command]
pub async fn publish_marketplace_template(
    state: State<'_, AppState>,
    template_id: String,
    category: Option<String>,
    tags: Vec<String>,
) -> Result<(), String> {
    let db = state.harness.db();
    MarketplaceCatalogServiceImpl.publish_template(db, &template_id, category, tags).await.map_err(
        |e| {
            err_to_string(
                ErrorResponse::new(mkt_err::PUBLISH_FAILED)
                    .with_category(ErrorCategory::Unrecoverable)
                    .with_detail(e),
            )
        },
    )
}

/// 获取目录条目的评分统计（评论数 + 平均分）。
///
/// 复用 `MarketplaceService::get_stats` 实现，目录条目 ID 即 `marketplace_id`。
#[agent_command(domain = marketplace, safety = Safe, call_mode = StateInput, description = "获取条目评分统计")]
#[tauri::command]
pub async fn get_marketplace_item_stats(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<MarketplaceStats, String> {
    let db = state.harness.db();
    MarketplaceServiceImpl.get_stats(db, &item_id).await.map_err(|e| {
        err_to_string(
            ErrorResponse::new(common_err::INTERNAL)
                .with_category(ErrorCategory::Unrecoverable)
                .with_detail(e),
        )
    })
}
