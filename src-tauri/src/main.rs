// SPDX-License-Identifier: AGPL-3.0-only

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

fn main() {
    // P1-NEW-3: 全局 panic hook，记录崩溃调用栈到 tracing 和 crash 日志文件。
    // 在 set_hook 之前先保存默认 hook，以便在自定义 hook 中调用默认行为。
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };

        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let backtrace = std::backtrace::Backtrace::force_capture();
        let msg = format!("PANIC at {location}: {payload}\nBacktrace:\n{backtrace}");

        // 写入 tracing（如果 subscriber 已初始化则可见于日志）
        tracing::error!("{msg}");

        // 写入 crash 日志文件（兜底，即使 tracing subscriber 未初始化）
        let crash_log_path = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(|dir| PathBuf::from(dir).join("axagent-crash.log"))
            .unwrap_or_else(|_| PathBuf::from("axagent-crash.log"));

        let _ = std::fs::write(&crash_log_path, &msg);

        // 调用默认 hook（打印到 stderr + 触发 Windows Error Reporting）
        default_hook(panic_info);
    }));

    axagent_lib::run()
}
