// SPDX-License-Identifier: AGPL-3.0-only

//! axagent-scanner — 本地消息/日历扫描器实现。
//! 提供 MemoryScanner trait 的两个实现，以及 Obsidian 回忆镜像。

pub mod file_scanner;
pub mod ical_scanner;
pub mod obsidian_mirror;

pub use file_scanner::FileScanner;
pub use ical_scanner::ICalScanner;
pub use obsidian_mirror::ObsidianMirror;
