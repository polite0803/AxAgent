// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

/// 路由分类请求
#[derive(Debug, Serialize, Deserialize)]
pub struct ClassifyRouteRequest {
    pub prompt: String,
}

/// 对用户提示进行分类，返回模型路由建议。
/// 这是基于启发式的快速分类器——无需 LLM 调用。
/// 前端用于在发送前决定使用哪个模型层级。
#[tauri::command]
pub fn classify_route(request: ClassifyRouteRequest) -> crate::smart_router::RouteDecision {
    crate::smart_router::classify_and_route(&request.prompt)
}