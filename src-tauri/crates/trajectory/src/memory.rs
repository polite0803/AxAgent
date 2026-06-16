// SPDX-License-Identifier: AGPL-3.0-only

#![allow(dead_code)]

//! Memory service module - unified wrapper around TrajectoryStorage
//!
//! This module provides a unified memory system that integrates:
//! - TrajectoryStorage: existing storage for trajectories, skills, patterns
//! - Entity/Relationship storage: knowledge graph entities
//! - Working memory: tiered context for prompts (ShortTerm/Working/LongTerm/Core)
//! - Closed-loop learning: nudges and proactive suggestions
//! - Memory decay, dedup, and merge

pub use crate::memory_providers::{
    closed_loop::ClosedLoopService,
    entity::{Entity, EntityType, Relationship, RelationshipType},
    service::{MemoryEntry, MemoryNature, MemoryProvenance, MemoryService, MemoryTier},
};

use crate::TrajectoryStorage;
use std::sync::Arc;

pub(crate) struct MemoryRegistry {
    pub storage: Arc<TrajectoryStorage>,
    pub memory_service: Arc<MemoryService>,
    pub closed_loop: ClosedLoopService,
}

impl MemoryRegistry {
    pub(crate) fn new(storage: Arc<TrajectoryStorage>) -> anyhow::Result<Self> {
        let memory_service = Arc::new(MemoryService::new(storage.clone())?);
        let closed_loop =
            ClosedLoopService::new(storage.clone()).with_memory_service(memory_service.clone());

        Ok(Self {
            storage,
            memory_service,
            closed_loop,
        })
    }

    pub(crate) fn initialize(&self) -> anyhow::Result<()> {
        self.memory_service.initialize()
    }
}
