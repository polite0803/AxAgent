// SPDX-License-Identifier: AGPL-3.0-only
//! 能力披露工具的共享依赖槽 —— CapabilityView / CapabilityLoad / DiscoverSkills
//! / CapabilityBrowse 共用同一份 `CapabilityIndexer`。
//!
//! 此前 view 与 load 各持一个 `OnceLock`，wiring 层要注入两次 —— 值虽相同，
//! 但结构上允许两个工具看到不同索引（将来加测试隔离或热更新时必然漂移）。
//! 收敛到单一槽位后，wiring 只需注入一次。

use axagent_harness::CapabilityIndexer;
use std::sync::{Arc, OnceLock};

static CAPABILITY_INDEXER: OnceLock<Arc<dyn CapabilityIndexer>> = OnceLock::new();

/// 注入共享索引器（wiring 层初始化时调用一次）。
pub fn set_capability_indexer(indexer: Arc<dyn CapabilityIndexer>) {
    let _ = CAPABILITY_INDEXER.set(indexer);
}

/// 取共享索引器；未注入返回 `None`。
pub fn capability_indexer() -> Option<Arc<dyn CapabilityIndexer>> {
    CAPABILITY_INDEXER.get().cloned()
}
