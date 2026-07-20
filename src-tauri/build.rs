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
        println!("Windows: set main thread stack to 8MB (8388608)");

        // ── 为测试二进制嵌入 Common Controls v6 manifest ──
        // 背景：cargo test --lib 生成的测试 EXE 不继承 tauri_build 为主 bin
        // 嵌入的 manifest 资源，导致 Windows 加载 comctl32.dll v5，
        // 缺少 v6 入口点，启动时 STATUS_ENTRYPOINT_NOT_FOUND (0xC0000139)。
        //
        // 分层策略（cargo 的 link-arg 无法直接区分"主 bin"和"测试 bin"）：
        //   1. compile_for_tests → cargo:rustc-link-arg-tests
        //      只覆盖 [[test]] 显式声明的 integration test 目标
        //   2. lib 单元测试 harness（rustc --test src\lib.rs）不被 rustc-link-arg-tests 覆盖，
        //      通过 src/lib.rs 中的 #[cfg(test)] #[link(name = "test-manifest")] 显式声明链接
        //   3. 主 bin 不引用 test-manifest.lib，与 tauri_build 的 resource.lib 不冲突
        //
        // 同时输出 cargo:rustc-link-search 让 #[link] 能在 OUT_DIR 中找到 test-manifest.lib。
        embed_resource::compile_for_tests("test-manifest.rc", embed_resource::NONE)
            .manifest_required()
            .expect("test manifest (Common Controls v6) is required on Windows");

        // 将 OUT_DIR 加入链接搜索路径，供 src/lib.rs 的 #[link(name = "test-manifest")] 使用
        let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
        println!("cargo:rustc-link-search=native={out_dir}");
    }

    tauri_build::build()
}
