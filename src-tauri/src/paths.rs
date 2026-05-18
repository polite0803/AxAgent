use std::path::PathBuf;

/// Returns the canonical AxAgent home directory and ensures it exists.
///
/// - macOS / Linux: `~/.axagent/`
/// - Windows:       `%USERPROFILE%\.axagent\`
/// - Mobile (iOS):  App's sandboxed container (data directory)
/// - Mobile (Android): App's sandboxed container (data directory)
///
/// Panics if the home directory cannot be determined.
pub fn axagent_home() -> PathBuf {
    #[cfg(mobile)]
    {
        #[cfg(target_os = "android")]
        {
            // 按优先级尝试多个路径。data_dir() 需要 JNI 上下文，
            // 在子线程中可能不可用，所以这里优先尝试多种方式。
            //
            // Android 10+:   /storage/emulated/0/Android/data/<pkg>/files/
            // Android 9-:    /sdcard/Android/data/<pkg>/files/
            // 通用回退:      /data/data/<pkg>/files/ (via dirs::data_dir)
            let candidates: Vec<(&str, fn() -> PathBuf)> = vec![
                // 1. 外部 files dir（Android/data/<pkg>/files/）——无需额外权限
                (
                    "external_files",
                    || {
                        PathBuf::from("/storage/emulated/0/Android/data/top.axagent.desktop/files/.axagent")
                    },
                ),
                (
                    "sdcard_files",
                    || {
                        PathBuf::from("/sdcard/Android/data/top.axagent.desktop/files/.axagent")
                    },
                ),
                // 2. 内部 data dir（通过 dirs crate）
                (
                    "data_dir",
                    || {
                        dirs::data_dir()
                            .unwrap_or_else(|| PathBuf::from("/data/data/top.axagent.desktop"))
                            .join(".axagent")
                    },
                ),
                // 3. 内部 cache dir
                (
                    "cache_dir",
                    || {
                        dirs::cache_dir()
                            .unwrap_or_else(|| PathBuf::from("/data/data/top.axagent.desktop/cache"))
                            .join(".axagent")
                    },
                ),
                // 4. Download 目录（最低优先级，用户可见）
                (
                    "download",
                    || {
                        PathBuf::from("/storage/emulated/0/Download/.axagent")
                    },
                ),
            ];

            for (_label, path_fn) in &candidates {
                let path = path_fn();
                match std::fs::create_dir_all(&path) {
                    Ok(()) => {
                        tracing::info!("axagent_home: using {}", path.display());
                        return path;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "axagent_home: {} not writable: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }

            // 绝望回退：当前目录
            tracing::error!("axagent_home: all paths failed, using current directory");
            let fallback = PathBuf::from("./.axagent");
            let _ = std::fs::create_dir_all(&fallback);
            return fallback;
        }

        #[cfg(not(target_os = "android"))]
        {
            let base = dirs::data_dir()
                .or_else(dirs::home_dir)
                .or_else(|| {
                    std::env::var("HOME")
                        .ok()
                        .map(PathBuf::from)
                })
                .unwrap_or_else(|| {
                    tracing::warn!("Could not determine home directory, using current dir");
                    PathBuf::from(".")
                });
            base.join(".axagent")
        }
    }
    #[cfg(not(mobile))]
    {
        #[cfg(not(windows))]
        let home = std::env::var("HOME").unwrap_or_else(|_| {
            tracing::warn!("HOME 环境变量未设置，使用当前目录作为后备");
            String::from(".")
        });
        #[cfg(windows)]
        let home = std::env::var("USERPROFILE").unwrap_or_else(|_| {
            tracing::warn!("USERPROFILE 环境变量未设置，使用当前目录作为后备");
            String::from(".")
        });

        PathBuf::from(home).join(".axagent")
    }
}
