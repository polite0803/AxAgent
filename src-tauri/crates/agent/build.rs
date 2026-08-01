// SPDX-License-Identifier: AGPL-3.0-only
// agent crate 的测试二进制通过 dev-dependencies 链接 tauri（features = ["test"]），
// 需要嵌入 Common Controls v6 manifest，否则 Windows 上 cargo test 启动即报
// STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)。
// 参考: https://github.com/tauri-apps/tauri/issues/11028
//       https://github.com/tauri-apps/tauri/discussions/11179

fn main() {
    #[cfg(target_os = "windows")]
    {
        let is_tauri_workspace = std::env::var("__TAURI_WORKSPACE__").is_ok_and(|v| v == "true");
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        if is_tauri_workspace && target_env == "msvc" {
            let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("common-controls.manifest");
            println!("cargo:rerun-if-changed={}", manifest.display());
            // lib unit tests（#[cfg(test)] 在 lib 内）链接时只接受 rustc-link-arg，
            // rustc-link-arg-tests 仅作用于 tests/ 集成测试，对 lib unit tests 无效。
            // 该参数会传播到依赖 agent 的 crate 的 bin 链接产物（cargo 依赖传播）。
            println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
            println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        }
    }
}
