//! Android 崩溃诊断工具。
//!
//! Android 没有 MessageBox 等价物，因此将崩溃信息写入文件系统。
//! 下次启动时前端读取并显示给用户。
//!
//! 这等效于 windows_utils.rs 对 Windows 的作用。

#[allow(unused_imports)]
use std::path::PathBuf;

#[cfg(target_os = "android")]
fn crash_log_path() -> Option<PathBuf> {
    let dir = crate::paths::axagent_home();
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("crash.log"))
}

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
pub(crate) fn crash_log_path() -> Option<PathBuf> {
    None
}

/// 尝试写入外部可访问的路径（用户可通过文件管理器读取）。
/// 在 Android 10+ 上，`/sdcard/Android/data/<package>/files/` 目录
/// 无需额外权限即可写入，且可通过系统文件管理器或「设置→存储」查看。
#[cfg(target_os = "android")]
fn external_diagnostic_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 尝试多个常用路径，只要一个可用即可
    let external_candidates = [
        "/storage/emulated/0/Android/data/top.axagent.desktop/files",
        "/sdcard/Android/data/top.axagent.desktop/files",
        "/storage/emulated/0/Download",
        "/sdcard/Download",
    ];

    for candidate in &external_candidates {
        let p = PathBuf::from(candidate);
        if p.exists() || std::fs::create_dir_all(&p).is_ok() {
            paths.push(p.join("axagent-crash.log"));
            break; // 找到一个可用的就够
        }
    }

    paths
}

/// 写入崩溃条目。始终记录到 tracing（logcat）并持久化到磁盘（多个位置）。
pub fn report_fatal_error(message: &str) {
    tracing::error!("FATAL_STARTUP: {}", message);

    #[cfg(target_os = "android")]
    {
        // 写入内部存储（原始 crash log）
        if let Some(path) = crash_log_path() {
            let existing = std::fs::read_to_string(&path).unwrap_or_default();
            let entry =
                format!("{} | {}\n", chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"), message);
            let _ = std::fs::write(&path, existing + entry.as_str());
        }

        // 写入外部可访问路径（用户可通过文件管理器读取）
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
        let entry = format!("[{}] FATAL: {}\n", timestamp, message);
        for ext_path in external_diagnostic_paths() {
            let existing = std::fs::read_to_string(&ext_path).unwrap_or_default();
            let _ = std::fs::write(&ext_path, existing + &entry);
        }
    }
}

/// 读取并删除崩溃日志。无先前崩溃时返回 None。
/// 在 WebView 运行后调用，以便前端展示。
pub fn consume_crash_log() -> Option<String> {
    #[cfg(target_os = "android")]
    if let Some(crash_path) = crash_log_path() {
        let phase_path = crate::paths::axagent_home().join(".startup_phase");

        let crash_contents = if crash_path.exists() {
            std::fs::read_to_string(&crash_path).ok()
        } else {
            None
        };
        let phase = std::fs::read_to_string(&phase_path).ok();

        let _ = std::fs::remove_file(&crash_path);
        let _ = std::fs::remove_file(&phase_path);

        if let Some(crash) = crash_contents {
            let mut report = crash;
            if let Some(phase_str) = phase {
                let trimmed = phase_str.trim();
                if !trimmed.is_empty() {
                    report.push_str(&format!("\nLast startup phase: {}", trimmed));
                }
            }
            let trimmed = report.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

/// 记录启动阶段标记，用于崩溃诊断。
/// 轻量级文件写入，帮助确定应用在哪个阶段崩溃。
/// 同时写入内部存储和外部可访问路径。
pub fn mark_startup_phase(phase: &str) {
    #[cfg(target_os = "android")]
    {
        // 写入内部存储
        let path = crate::paths::axagent_home().join(".startup_phase");
        let _ = std::fs::write(&path, phase);

        // 写入外部可访问路径
        let entry = format!("[STARTUP_PHASE] {}\n", phase);
        for ext_path in external_diagnostic_paths() {
            let existing = std::fs::read_to_string(&ext_path).unwrap_or_default();
            let _ = std::fs::write(&ext_path, existing + &entry);
        }
    }
    tracing::info!("STARTUP_PHASE: {}", phase);
}
