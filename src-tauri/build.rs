// SPDX-License-Identifier: AGPL-3.0-only

fn main() {
    // Enable cfg(mobile) when building for Android or iOS
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("android")
        || target.contains("ios")
        || cfg!(target_os = "android")
        || cfg!(target_os = "ios")
    {
        println!("cargo:rustc-cfg=mobile");
        println!("cargo:warning=Building for mobile target: {}", target);
    }

    println!("cargo::rustc-check-cfg=cfg(mobile)");

    // ── Windows: 增大主线程栈到 8MB ──
    // Tauri 2.x 在 Windows 上启动 WebView2 时调用栈很深，
    // Rust 默认 2MB 栈不够，在某些硬件上会间歇 stack overflow。
    // Linux/macOS 默认栈够大（8MB），不需要此设置。
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-arg=/STACK:8388608");
    }

    // ── Windows: Common Controls v6 manifest 统一处理 ──
    // tauri 已知问题：`cargo test` 生成的测试 exe 未声明 comctl32 v6 依赖，
    // 链接了 tauri/wry 的测试二进制启动即报 STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)。
    // 参考: https://github.com/tauri-apps/tauri/issues/11028
    //       https://github.com/tauri-apps/tauri/discussions/11179
    //
    // 方案：bin 与 lib unit tests 统一由本 build.rs 通过 rustc-link-arg 嵌入同一份
    // Common Controls v6 manifest；同时让 tauri-build 关闭默认 app manifest
    // （try_build + WindowsAttributes::new_without_app_manifest），避免 bin 重复嵌入。
    // 注意：rustc-link-arg-tests 仅作用于 tests/ 集成测试，对 lib unit tests
    // （src/lib.rs 的 #[cfg(test)]）无效，故必须用 rustc-link-arg（所有目标）。
    #[cfg(target_os = "windows")]
    {
        let is_tauri_workspace = std::env::var("__TAURI_WORKSPACE__").is_ok_and(|v| v == "true");
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        if is_tauri_workspace && target_env == "msvc" {
            let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("common-controls.manifest");
            println!("cargo:rerun-if-changed={}", manifest.display());
            println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
            println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        }
    }

    #[cfg(target_os = "windows")]
    {
        let is_tauri_workspace = std::env::var("__TAURI_WORKSPACE__").is_ok_and(|v| v == "true");
        if is_tauri_workspace {
            // 关闭 tauri-build 默认 app manifest，改由上方 rustc-link-arg 统一嵌入。
            let attrs = tauri_build::Attributes::new()
                .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
            tauri_build::try_build(attrs).unwrap_or_else(|e| {
                println!("cargo:warning=tauri-build try_build failed: {e:#}");
                // 失败时回退默认 build（保留原有行为），避免构建中断
                tauri_build::build();
            });
            return;
        }
    }

    tauri_build::build()
}
