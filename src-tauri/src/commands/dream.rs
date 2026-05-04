//! 梦境巩固 (Dream Consolidation) 命令
//!
//! 提供前端触发和查询梦境巩固的 Tauri 命令。

use crate::AppState;
use tauri::State;

/// 手动触发一次梦境巩固（忽略门控条件）
#[tauri::command]
pub async fn dream_consolidate_now(state: State<'_, AppState>) -> Result<String, String> {
    let consolidator = &state.dream_consolidator;

    let result = consolidator
        .consolidate(
            Some(&|n| tracing::info!("梦境: 提取 {} 条记忆", n)),
            Some(&|n| tracing::info!("梦境: 发现 {} 个模式", n)),
            Some(&|n| tracing::info!("梦境: 生成 {} 个建议", n)),
        )
        .await;

    if result.executed {
        Ok(format!(
            "梦境巩固完成: {} 条记忆, {} 个模式, {} 个建议, 耗时 {} 秒",
            result.memories_extracted,
            result.patterns_discovered,
            result.suggestions_generated,
            result.duration_secs
        ))
    } else {
        Err(format!(
            "梦境巩固未执行: {}",
            result.error.unwrap_or_else(|| "未知原因".to_string())
        ))
    }
}

/// 获取梦境巩固状态
#[tauri::command]
pub async fn dream_get_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let consolidator = &state.dream_consolidator;
    let config = consolidator.get_config().await;
    let status = consolidator.get_state().await;

    Ok(serde_json::json!({
        "enabled": config.enabled,
        "isRunning": status.is_running,
        "lastConsolidationAt": status.last_consolidation_at.map(|dt| dt.timestamp_millis()),
        "totalConsolidations": status.total_consolidations,
        "totalMemoriesExtracted": status.total_memories_extracted,
        "totalConsolidationSecs": status.total_consolidation_secs,
        "sessionsSinceLast": status.sessions_since_last,
    }))
}

/// 设置梦境巩固配置
#[tauri::command]
pub async fn dream_set_config(
    state: State<'_, AppState>,
    config: axagent_trajectory::DreamConsolidationConfig,
) -> Result<(), String> {
    state.dream_consolidator.update_config(config).await;
    Ok(())
}
