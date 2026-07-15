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

    tauri_build::build()
}
