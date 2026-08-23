// SPDX-License-Identifier: AGPL-3.0-only

//! 命令注册入口 — 由 build.rs 自动生成，请勿手动编辑。
//!
//! 新增命令：在 src/commands/ 下的 .rs 文件中添加 #[tauri::command] 注解即可，
//! 构建脚本会自动扫描并注册。

include!(concat!(env!("OUT_DIR"), "/register_commands.rs"));
