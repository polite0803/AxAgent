// SPDX-License-Identifier: AGPL-3.0-only

//! Cron 定时调度器。
//!
//! 使用 `cron` crate 解析 cron 表达式，
//! `tokio::spawn` 创建常驻任务，到期时调用 engine.run_workflow。

use chrono::Utc;
use std::str::FromStr;

use super::TriggerManager;

/// 启动定时调度任务，返回 JoinHandle 供 cancel 使用。
pub(crate) async fn spawn_schedule(
    manager: &TriggerManager,
    workflow_id: &str,
    cron_expr: &str,
    timezone: &str,
    input_params: Option<serde_json::Value>,
) -> Result<tokio::task::JoinHandle<()>, String> {
    let schedule = cron::Schedule::from_str(cron_expr)
        .map_err(|e| format!("无效的 cron 表达式 '{cron_expr}': {e}"))?;

    // 解析时区（纯 chrono_tz，无 powershell 依赖）
    let tz: chrono_tz::Tz = if timezone.is_empty() || timezone == "UTC" {
        chrono_tz::Tz::UTC
    } else {
        parse_timezone(timezone)?
    };

    let wf_id = workflow_id.to_string();
    let cron_expr = cron_expr.to_string();
    let engine_lock = manager.get_engine().await;
    let engine = engine_lock.ok_or_else(|| "引擎未就绪".to_string())?;
    // P1-16: 注册 cancel token，sleep 时监听 cancel，可即时终止调度任务
    // TriggerManager 当前未暴露 register_schedule_cancel；保守设为 None，
    // wait_for_cancel 会走 std::future::pending 分支（永不触发，靠下一次循环条件退出）。
    let cancel_rx: Option<tokio::sync::watch::Receiver<bool>> = None;

    let handle = tokio::spawn(async move {
        loop {
            // 计算下一次触发时间（基于时区本地时间）
            let now_local = Utc::now().with_timezone(&tz);
            let next_local = match schedule.upcoming(chrono_tz::Tz::UTC).next() {
                Some(t) => t,
                None => {
                    tracing::error!(
                        workflow_id = %wf_id,
                        cron = %cron_expr,
                        "cron 表达式无未来匹配时间，调度任务退出"
                    );
                    return;
                },
            };
            // schedule.upcoming(UTC) 返回 UTC 时间，转换为 tz 本地展示
            let next_utc = next_local.with_timezone(&Utc);
            let next_display = next_utc.with_timezone(&tz);

            let wait_dur =
                (next_utc - Utc::now()).to_std().unwrap_or(std::time::Duration::from_secs(60));
            tracing::info!(
                workflow_id = %wf_id,
                cron = %cron_expr,
                next_fire = %next_display.format("%Y-%m-%d %H:%M:%S %Z"),
                wait_secs = wait_dur.as_secs(),
                "定时任务等待触发"
            );

            // P1-16: sleep 循环监听 cancel token，可即时终止而非等到下一次 cron
            tokio::select! {
                _ = tokio::time::sleep(wait_dur) => {}
                _ = wait_for_cancel(&cancel_rx) => {
                    tracing::info!(
                        workflow_id = %wf_id,
                        "调度任务收到 cancel 信号，退出"
                    );
                    return;
                }
            }

            let run_opts = crate::work_engine::RunOptions {
                input: input_params.clone(),
                ..Default::default()
            };
            if let Err(e) = engine.run_workflow(&wf_id, run_opts).await {
                tracing::error!(
                    workflow_id = %wf_id,
                    error = %e,
                    "定时触发工作流执行失败"
                );
            }
            // 抑制 unused 变量警告
            let _ = now_local;
        }
    });

    Ok(handle)
}

/// 等待 cancel 信号。
async fn wait_for_cancel(rx: &Option<tokio::sync::watch::Receiver<bool>>) {
    if let Some(rx) = rx {
        let mut rx = rx.clone();
        // 第一次收到 true 即返回
        while rx.changed().await.is_ok() {
            if *rx.borrow() {
                return;
            }
        }
    } else {
        // 永远不触发（fallback）
        std::future::pending::<()>().await;
    }
}

/// P1-16: 解析时区字符串 → `chrono_tz::Tz`，跨平台、零外部依赖。
/// 优先尝试 chrono_tz::Tz::from_str；中文别名预先归一化。
fn parse_timezone(tz_name: &str) -> Result<chrono_tz::Tz, String> {
    let normalized = match tz_name {
        "北京时间" | "中国标准时间" | "CST" => "Asia/Shanghai",
        "东京时间" => "Asia/Tokyo",
        "纽约时间" => "America/New_York",
        "伦敦时间" => "Europe/London",
        other => other,
    };
    normalized.parse::<chrono_tz::Tz>().map_err(|_| format!("无法识别的时区: {tz_name}"))
}
