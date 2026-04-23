//! Memory service module - unified wrapper around TrajectoryStorage
//!
//! This module provides a unified memory system that integrates:
//! - TrajectoryStorage: existing storage for trajectories, skills, patterns
//! - Entity/Relationship storage: knowledge graph entities
//! - Working memory: short-term context for prompts
//! - Closed-loop learning: nudges and proactive suggestions

mod entity;
mod memory;
mod closed_loop;

pub use entity::{Entity, EntityType, Relationship, RelationshipType, GraphQuery, NudgeCandidate};
pub use memory::{MemoryService, MemoryConfig, MemoryEntry, WorkingMemory, MemoryUsage, SearchResult, MemoryActionResult};
pub use closed_loop::{ClosedLoopService, ClosedLoopConfig, PeriodicNudge, MemoryConsolidationTask, SkillCreationProposal};

use crate::TrajectoryStorage;
use std::sync::Arc;

pub struct MemoryRegistry {
    pub storage: Arc<TrajectoryStorage>,
    pub memory_service: MemoryService,
    pub closed_loop: ClosedLoopService,
}

impl MemoryRegistry {
    pub fn new(storage: Arc<TrajectoryStorage>) -> anyhow::Result<Self> {
        let memory_service = MemoryService::new(storage.clone())?;
        let closed_loop = ClosedLoopService::new(storage.clone());

        Ok(Self {
            storage,
            memory_service,
            closed_loop,
        })
    }

    pub fn initialize(&self) -> anyhow::Result<()> {
        self.memory_service.initialize()
    }
}