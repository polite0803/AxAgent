// SPDX-License-Identifier: AGPL-3.0-only

//! Re-export shim — 实际实现在 `axagent_harness::marketplace`。
//! 上层调用方（gateway / core）只需保持 `axagent_kit::marketplace_service::*` 路径不变。

pub use axagent_harness::marketplace::*;
