// SPDX-License-Identifier: AGPL-3.0-only

//! 信号→实盘执行桥接器
//!
//! 将 `axagent-quant` 产生的 Signal 桥接到实际交易执行。
//! pending 状态暂存内存（HashMao + Arc<RwLock>），
//! 确认后写入 DB trades 表。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use axagent_entities::trades;

use crate::AppState;
use agent_macro::agent_command;

/// 执行模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionMode {
    #[default]
    Manual,
    SemiAuto,
    FullAuto,
}

/// 待执行记录（内存态）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingExecution {
    pub id: String,
    pub stock_code: String,
    pub stock_name: String,
    pub direction: String,
    pub price: f64,
    pub quantity: i32,
    pub reason: String,
    pub risk_level: String,
    pub risk_warning: String,
    pub created_at: i64,
    pub status: String,
}

/// 风控检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskCheckResult {
    pub passed: bool,
    pub risk_level: String,
    pub reason: String,
}

/// 执行桥接器状态（注册在 AppState）
pub struct ExecutionBridgeState {
    pub mode: RwLock<ExecutionMode>,
    pub pendings: RwLock<HashMap<String, PendingExecution>>,
    pub db: Arc<DatabaseConnection>,
}

impl ExecutionBridgeState {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { mode: RwLock::new(ExecutionMode::Manual), pendings: RwLock::new(HashMap::new()), db }
    }
}

/// 提交信号到执行管道
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "提交信号到执行管道")]
#[tauri::command]
pub async fn execution_submit_signal(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    signal_code: String,
    signal_action: String,
    signal_reason: String,
    stock_name: String,
    current_price: f64,
) -> Result<String, String> {
    let direction = match signal_action.as_str() {
        "buy" | "Buy" | "strong_buy" => "buy",
        "sell" | "Sell" | "strong_sell" => "sell",
        _ => "hold",
    }
    .to_string();

    let bridge = &state.execution_bridge;
    let mode = bridge.mode.read().await.clone();

    // 风控检查
    let risk = risk_check(&bridge.pendings, &signal_code).await;

    match mode {
        ExecutionMode::FullAuto => {
            if !risk.passed {
                let _ = app.emit("execution-risk-rejected", &risk);
                return Err(format!("风控不通过: {}", risk.reason));
            }
            let trade_id = execute_trade(
                &bridge.db,
                &signal_code,
                &stock_name,
                &direction,
                current_price,
                0,
                &signal_reason,
            )
            .await?;
            let _ = app.emit(
                "execution-filled",
                &serde_json::json!({
                    "tradeId": trade_id, "stockCode": signal_code, "action": direction,
                }),
            );
            Ok(trade_id)
        },
        ExecutionMode::SemiAuto if risk.passed => {
            let trade_id = execute_trade(
                &bridge.db,
                &signal_code,
                &stock_name,
                &direction,
                current_price,
                0,
                &signal_reason,
            )
            .await?;
            let _ = app.emit(
                "execution-filled",
                &serde_json::json!({
                    "tradeId": trade_id, "stockCode": signal_code, "action": direction,
                }),
            );
            Ok(trade_id)
        },
        _ => {
            // Manual / SemiAuto risk fail → pending
            let pending_id = uuid::Uuid::new_v4().to_string();
            let pending = PendingExecution {
                id: pending_id.clone(),
                stock_code: signal_code.clone(),
                stock_name,
                direction,
                price: current_price,
                quantity: 0,
                reason: signal_reason,
                risk_level: risk.risk_level.clone(),
                risk_warning: risk.reason.clone(),
                created_at: Utc::now().timestamp_millis(),
                status: "pending".into(),
            };
            bridge.pendings.write().await.insert(pending_id.clone(), pending.clone());
            let _ = app.emit("execution-pending", &pending);
            Ok(pending_id)
        },
    }
}

/// 确认待执行
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "确认待执行交易")]
#[tauri::command]
pub async fn execution_confirm(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    pending_id: String,
    quantity: i32,
) -> Result<String, String> {
    let bridge = &state.execution_bridge;
    let pending = bridge
        .pendings
        .write()
        .await
        .remove(&pending_id)
        .ok_or_else(|| format!("待执行记录 {pending_id} 不存在或已处理"))?;

    if pending.status != "pending" {
        return Err(format!("待执行记录状态为 {}，不可确认", pending.status));
    }

    let trade_id = execute_trade(
        &bridge.db,
        &pending.stock_code,
        &pending.stock_name,
        &pending.direction,
        pending.price,
        quantity,
        &pending.reason,
    )
    .await?;

    let _ = app.emit(
        "execution-confirmed",
        &serde_json::json!({
            "pendingId": pending_id, "tradeId": trade_id, "quantity": quantity,
        }),
    );

    Ok(trade_id)
}

/// 驳回待执行
#[agent_command(domain = "finance", safety = Dangerous, call_mode = StateInput, description = "驳回待执行交易")]
#[tauri::command]
pub async fn execution_reject(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    pending_id: String,
    reason: String,
) -> Result<(), String> {
    let bridge = &state.execution_bridge;
    bridge.pendings.write().await.remove(&pending_id);
    let _ = app.emit(
        "execution-rejected",
        &serde_json::json!({
            "pendingId": pending_id, "reason": reason,
        }),
    );
    Ok(())
}

/// 列出待执行
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "列出待执行记录")]
#[tauri::command]
pub async fn execution_list_pending(
    state: State<'_, AppState>,
) -> Result<Vec<PendingExecution>, String> {
    let bridge = &state.execution_bridge;
    let pendings = bridge.pendings.read().await;
    let mut list: Vec<PendingExecution> = pendings.values().cloned().collect();
    list.sort_by_key(|a| std::cmp::Reverse(a.created_at));
    Ok(list)
}

/// 设置执行模式
#[agent_command(domain = "finance", safety = Caution, call_mode = StateInput, description = "设置执行模式")]
#[tauri::command]
pub async fn execution_set_mode(state: State<'_, AppState>, mode: String) -> Result<(), String> {
    let exec_mode = match mode.as_str() {
        "manual" => ExecutionMode::Manual,
        "semi_auto" | "semi-auto" => ExecutionMode::SemiAuto,
        "full_auto" | "full-auto" | "auto" => ExecutionMode::FullAuto,
        _ => return Err(format!("未知模式: {mode} (可选: manual/semi_auto/full_auto)")),
    };
    *state.execution_bridge.mode.write().await = exec_mode;
    Ok(())
}

/// 获取当前执行模式
#[agent_command(domain = "finance", safety = Safe, call_mode = StateInput, description = "获取当前执行模式")]
#[tauri::command]
pub async fn execution_get_mode(state: State<'_, AppState>) -> Result<String, String> {
    let mode = state.execution_bridge.mode.read().await;
    Ok(match *mode {
        ExecutionMode::Manual => "manual",
        ExecutionMode::SemiAuto => "semi_auto",
        ExecutionMode::FullAuto => "full_auto",
    }
    .to_string())
}

// ── 内部辅助 ──

async fn risk_check(
    pendings: &RwLock<HashMap<String, PendingExecution>>,
    stock_code: &str,
) -> RiskCheckResult {
    let existing = pendings.read().await;
    let has_pending =
        existing.values().any(|p| p.stock_code == stock_code && p.status == "pending");
    if has_pending {
        return RiskCheckResult {
            passed: false,
            risk_level: "中".into(),
            reason: format!("股票 {stock_code} 已有待确认的执行"),
        };
    }
    RiskCheckResult { passed: true, risk_level: "低".into(), reason: String::new() }
}

async fn execute_trade(
    db: &DatabaseConnection,
    stock_code: &str,
    stock_name: &str,
    direction: &str,
    price: f64,
    quantity: i32,
    reason: &str,
) -> Result<String, String> {
    let now = Utc::now();
    let trade_id = uuid::Uuid::new_v4().to_string();

    let trade = trades::ActiveModel {
        id: Set(trade_id.clone()),
        stock_code: Set(stock_code.to_string()),
        stock_name: Set(stock_name.to_string()),
        direction: Set(direction.to_string()),
        price: Set(price),
        quantity: Set(quantity),
        trade_date: Set(now.format("%Y-%m-%d").to_string()),
        trade_time: Set(now.format("%H:%M").to_string()),
        fee: Set(None),
        realized_pnl: Set(None),
        strategy: Set(Some("quant".to_string())),
        notes: Set(Some(reason.to_string())),
        created_at: Set(now.timestamp_millis()),
    };
    trade.insert(db).await.map_err(|e| format!("写入交易记录失败: {e}"))?;

    Ok(trade_id)
}
