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
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-arg=/STACK:8388608");
    }

    // ── Windows: Common Controls v6 manifest 统一处理 ──
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
            let attrs = tauri_build::Attributes::new()
                .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
            tauri_build::try_build(attrs).unwrap_or_else(|e| {
                println!("cargo:warning=tauri-build try_build failed: {e:#}");
                tauri_build::build();
            });
            return;
        }
    }

    tauri_build::build()
}
