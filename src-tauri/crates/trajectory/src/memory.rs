// SPDX-License-Identifier: AGPL-3.0-only

//! Memory service module - re-exports from memory_providers
//!
//! This module provides a unified re-export of types from the
//! memory_providers sub-modules (MemoryService, Entity, ClosedLoopService, etc.).

// 注:ClosedLoopService 当前未被 crate 内部使用,作为公共 API 保留 re-export
// 供 wiring 层(`src/init/`、`src/commands/`)按需导入。
#[allow(unused_imports)]
pub use crate::memory_providers::{
    closed_loop::ClosedLoopService,
    entity::{Entity, EntityType, Relationship, RelationshipType},
    service::{MemoryEntry, MemoryNature, MemoryProvenance, MemoryService, MemoryTier},
};
