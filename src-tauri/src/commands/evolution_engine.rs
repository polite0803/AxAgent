// SPDX-License-Identifier: AGPL-3.0-only

//! Evolution Engine 管理命令
//!
//! 提供技能进化引擎的启停、配置管理、状态查询。
//! 使用内存状态管理，支持多个进化引擎并行。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use tauri::command;
use tokio::sync::Mutex;

// ── Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineLog {
    pub timestamp: i64,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStatus {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub running: bool,
    pub config: serde_json::Value,
    pub stats: serde_json::Value,
    pub logs: Vec<EngineLog>,
    pub last_active: Option<i64>,
}

// ── Global State ──

static ENGINES: OnceLock<Mutex<HashMap<String, EngineStatus>>> = OnceLock::new();

fn engines() -> &'static Mutex<HashMap<String, EngineStatus>> {
    ENGINES.get_or_init(|| {
        let now = chrono::Utc::now().timestamp_millis();
        let mut map = HashMap::new();

        let engine_defs: Vec<(&str, &str, &str, &str)> = vec![
            ("skill_evolution", "技能进化引擎", "自动评估并优化技能效果", "core"),
            ("auto_tool_creator", "工具自动创建", "根据使用模式自动生成新工具", "core"),
            ("text_grad", "文本梯度优化", "通过文本梯度优化提示词", "core"),
            ("constitution", "宪法训练", "对齐训练与安全约束", "safety"),
            ("intrinsic_motivation", "内在动机", "好奇心驱动探索", "learning"),
        ];

        for (name, display_name, desc, category) in engine_defs {
            map.insert(
                name.to_string(),
                EngineStatus {
                    name: name.to_string(),
                    display_name: display_name.to_string(),
                    description: desc.to_string(),
                    category: category.to_string(),
                    running: category == "core",
                    config: serde_json::json!({"enabled": true}),
                    stats: serde_json::json!({"total_runs": 0, "last_run": null}),
                    logs: vec![EngineLog {
                        timestamp: now,
                        level: "info".into(),
                        message: format!("[{}] Engine initialized", name),
                    }],
                    last_active: if category == "core" { Some(now) } else { None },
                },
            );
        }

        Mutex::new(map)
    })
}

fn timestamp_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

// ── Commands ──

#[command]
pub async fn get_all_engine_status() -> Result<HashMap<String, EngineStatus>, String> {
    let store = engines().lock().await;
    // Return a snapshot with updated "last_active" for running engines
    let now = timestamp_millis();
    let mut result: HashMap<String, EngineStatus> = HashMap::new();
    for (k, v) in store.iter() {
        let mut updated = v.clone();
        if updated.running {
            updated.last_active = Some(now);
        }
        result.insert(k.clone(), updated);
    }
    Ok(result)
}

#[command]
pub async fn start_engine(engine_name: String) -> Result<(), String> {
    let mut store = engines().lock().await;
    match store.get_mut(&engine_name) {
        Some(engine) => {
            if engine.running {
                return Err(format!("Engine '{}' is already running", engine_name));
            }
            engine.running = true;
            engine.last_active = Some(timestamp_millis());
            engine.logs.push(EngineLog {
                timestamp: timestamp_millis(),
                level: "info".into(),
                message: format!("[{}] Engine started", engine_name),
            });
            tracing::info!(target: "evolution_engine", engine = %engine_name, "Engine started");
            Ok(())
        },
        None => Err(format!("Engine '{}' not found", engine_name)),
    }
}

#[command]
pub async fn stop_engine(engine_name: String) -> Result<(), String> {
    let mut store = engines().lock().await;
    match store.get_mut(&engine_name) {
        Some(engine) => {
            if !engine.running {
                return Err(format!("Engine '{}' is not running", engine_name));
            }
            engine.running = false;
            engine.logs.push(EngineLog {
                timestamp: timestamp_millis(),
                level: "info".into(),
                message: format!("[{}] Engine stopped", engine_name),
            });
            tracing::info!(target: "evolution_engine", engine = %engine_name, "Engine stopped");
            Ok(())
        },
        None => Err(format!("Engine '{}' not found", engine_name)),
    }
}

#[command]
pub async fn update_engine_config(
    engine_name: String,
    config: serde_json::Value,
) -> Result<(), String> {
    let mut store = engines().lock().await;
    match store.get_mut(&engine_name) {
        Some(engine) => {
            // Merge config (only top-level keys)
            if let (Some(existing_obj), Some(new_obj)) =
                (engine.config.as_object_mut(), config.as_object())
            {
                for (k, v) in new_obj {
                    existing_obj.insert(k.clone(), v.clone());
                }
            } else {
                engine.config = config;
            }
            engine.logs.push(EngineLog {
                timestamp: timestamp_millis(),
                level: "info".into(),
                message: format!("[{}] Configuration updated", engine_name),
            });
            tracing::info!(target: "evolution_engine", engine = %engine_name, "Config updated");
            Ok(())
        },
        None => Err(format!("Engine '{}' not found", engine_name)),
    }
}

#[command]
pub async fn get_engine_logs(
    engine_name: String,
    limit: Option<usize>,
) -> Result<Vec<EngineLog>, String> {
    let store = engines().lock().await;
    match store.get(&engine_name) {
        Some(engine) => {
            let logs = if let Some(l) = limit {
                if engine.logs.len() > l {
                    engine.logs[engine.logs.len() - l..].to_vec()
                } else {
                    engine.logs.clone()
                }
            } else {
                engine.logs.clone()
            };
            Ok(logs)
        },
        None => Err(format!("Engine '{}' not found", engine_name)),
    }
}

// 注：trigger_skill_evolution 已删除（伪触发命令，只写日志不执行真进化）
// 前端 evolutionStore.ts 已改调 commands::evolution::skill_evolution_start（真进化命令）
