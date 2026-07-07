// SPDX-License-Identifier: AGPL-3.0-only

//! 记忆外溢 HTTP handlers。
//! 路由全部挂在 `protected`（已走 auth_middleware）。

use axagent_harness::memory::{
    MemoryActionResultDto, MemoryAddRequest, MemoryFeedbackRequest, MemoryGroupedDto,
    MemorySearchItem, MemorySearchRequest, MemoryTreeItem, MemoryUpdateRequest,
};
use axum::Json;
use axum::extract::{Path, State};
use crate::server::GatewayAppState;

pub async fn add_memory(State(state): State<GatewayAppState>, Json(req): Json<MemoryAddRequest>) -> Json<MemoryActionResultDto> {
    match state.memory_store.add_memory(req).await {
        Ok(r) => Json(r), Err(e) => Json(MemoryActionResultDto { success: false, message: e }),
    }
}

pub async fn search_memory(State(state): State<GatewayAppState>, Json(req): Json<MemorySearchRequest>) -> Json<Vec<MemorySearchItem>> {
    match state.memory_store.search(req).await {
        Ok(r) => Json(r), Err(e) => { tracing::error!("memory search failed: {e}"); Json(Vec::new()) }
    }
}

pub async fn memory_tree(State(state): State<GatewayAppState>) -> Json<Vec<MemoryTreeItem>> {
    match state.memory_store.tree().await {
        Ok(r) => Json(r), Err(e) => { tracing::error!("memory tree failed: {e}"); Json(Vec::new()) }
    }
}

pub async fn memory_working(State(state): State<GatewayAppState>) -> Json<Vec<MemoryTreeItem>> {
    match state.memory_store.working().await {
        Ok(r) => Json(r), Err(e) => { tracing::error!("memory working failed: {e}"); Json(Vec::new()) }
    }
}

pub async fn memory_grouped(State(state): State<GatewayAppState>) -> Json<MemoryGroupedDto> {
    match state.memory_store.grouped().await {
        Ok(r) => Json(r), Err(e) => { tracing::error!("memory grouped failed: {e}"); Json(MemoryGroupedDto::default()) }
    }
}

pub async fn memory_feedback(State(state): State<GatewayAppState>, Path(id): Path<String>, Json(mut req): Json<MemoryFeedbackRequest>) -> Json<MemoryActionResultDto> {
    req.id = id;
    match state.memory_store.update_importance(req).await {
        Ok(r) => Json(r), Err(e) => Json(MemoryActionResultDto { success: false, message: e }),
    }
}

pub async fn delete_memory_handler(State(state): State<GatewayAppState>, Path(id): Path<String>) -> Json<MemoryActionResultDto> {
    match state.memory_store.delete_memory(&id).await {
        Ok(r) => Json(r), Err(e) => Json(MemoryActionResultDto { success: false, message: e }),
    }
}

pub async fn update_memory_handler(State(state): State<GatewayAppState>, Path(id): Path<String>, Json(req): Json<MemoryUpdateRequest>) -> Json<MemoryActionResultDto> {
    match state.memory_store.update_memory(&id, req).await {
        Ok(r) => Json(r), Err(e) => Json(MemoryActionResultDto { success: false, message: e }),
    }
}
