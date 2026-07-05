// SPDX-License-Identifier: AGPL-3.0-only

//! Memory service module - re-exports from memory_providers
//!
//! This module provides a unified re-export of types from the
//! memory_providers sub-modules (MemoryService, Entity, ClosedLoopService, etc.).

pub use crate::memory_providers::{
    closed_loop::ClosedLoopService,
    entity::{Entity, EntityType, Relationship, RelationshipType},
    service::{MemoryEntry, MemoryNature, MemoryProvenance, MemoryService, MemoryTier},
};
