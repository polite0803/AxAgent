// SPDX-License-Identifier: AGPL-3.0-only

//! Cron 定时调度器。
//!
//! 使用 `cron` crate 解析 cron 表达式，
//! `tokio::spawn` 创建常驻任务，到期时调用 engine.run_workflow。

use chrono::Utc;
use std::str::FromStr;
use std::sync::Arc;

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

    // 尝试解析时区
    let tz: chrono::FixedOffset = if timezone.is_empty() || timezone == "UTC" {
        chrono::FixedOffset::east_opt(0).unwrap()
    } else {
        // 尝试 IANA 时区 → FixedOffset（基于当前日期估算）
        parse_timezone_to_offset(timezone)?
    };

    let wf_id = workflow_id.to_string();
    let cron_expr = cron_expr.to_string();
    let engine_lock = manager.get_engine().await;
    let engine = engine_lock.ok_or_else(|| "引擎未就绪".to_string())?;

    let handle = tokio::spawn(async move {
        let mut schedule = schedule;
        loop {
            // 计算下一次触发时间（UTC）
            let now_utc = Utc::now();
            let next_utc = match schedule.upcoming(chrono::Utc).next() {
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

            // 对齐到目标时区
            let next_local = next_utc + chrono::Duration::seconds(tz.local_minus_utc() as i64);

            let wait_dur = (next_utc - now_utc)
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(60));
            tracing::info!(
                workflow_id = %wf_id,
                cron = %cron_expr,
                next_fire = %next_local.format("%Y-%m-%d %H:%M:%S"),
                wait_secs = wait_dur.as_secs(),
                "定时任务等待触发"
            );

            tokio::time::sleep(wait_dur).await;

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
        }
    });

    Ok(handle)
}

/// 尝试将 IANA 时区名称解析为 FixedOffset（基于当前日期估算）。
fn parse_timezone_to_offset(tz_name: &str) -> Result<chrono::FixedOffset, String> {
    // 常见中文别名映射
    let tz_name = match tz_name {
        "北京时间" | "中国标准时间" | "Asia/Shanghai" | "CST" => "Asia/Shanghai",
        "东京时间" | "Asia/Tokyo" => "Asia/Tokyo",
        "纽约时间" | "America/New_York" => "America/New_York",
        "伦敦时间" | "Europe/London" => "Europe/London",
        other => other,
    };

    // 尝试从环境获取系统时区
    #[cfg(windows)]
    {
        // Windows 上用 powershell 获取时区信息
        if let Ok(output) = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "[TimeZoneInfo]::FindSystemTimeZoneById('{}').BaseUtcOffset.TotalMinutes",
                    tz_name
                ),
            ])
            .output()
        {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if let Ok(minutes) = stdout.trim().parse::<f64>() {
                    return Ok(chrono::FixedOffset::east_opt((minutes * 60.0) as i32)
                        .ok_or_else(|| format!("无效的时区偏移: {minutes} 分钟"))?);
                }
            }
        }
    }

    // 回退：尝试直接解析为 ±HH:MM 格式
    if let Ok(offset) = chrono::FixedOffset::from_str(tz_name) {
        return Ok(offset);
    }

    // 回退：常见 UTC 偏移简写
    match tz_name {
        "UTC" | "Etc/UTC" | "GMT" => Ok(chrono::FixedOffset::east_opt(0).unwrap()),
        "Asia/Shanghai" => Ok(chrono::FixedOffset::east_opt(8 * 3600).unwrap()),
        "Asia/Tokyo" => Ok(chrono::FixedOffset::east_opt(9 * 3600).unwrap()),
        "America/New_York" => Ok(chrono::FixedOffset::east_opt(-5 * 3600).unwrap()),
        "Europe/London" => Ok(chrono::FixedOffset::east_opt(0).unwrap()),
        _ => Err(format!("无法识别的时区: {tz_name}")),
    }
}
