//! Memory service module - unified wrapper around TrajectoryStorage
//!
//! This module provides a unified memory system that integrates:
//! - TrajectoryStorage: existing storage for trajectories, skills, patterns
//! - Entity/Relationship storage: knowledge graph entities
//! - Working memory: short-term context for prompts
//! - Closed-loop learning: nudges and proactive suggestions

mod closed_loop;
mod entity;
pub mod service;

pub use closed_loop::{
    AutoAction, ClosedLoopConfig, ClosedLoopService, MemoryConsolidationTask, PeriodicNudge,
    SkillCreationProposal, SkillUpgradeProposal,
};
pub use entity::{Entity, EntityType, GraphQuery, NudgeCandidate, Relationship, RelationshipType};
pub use service::{
    MemoryActionResult, MemoryConfig, MemoryEntry, MemoryService, MemoryUsage, SearchResult,
    WorkingMemory,
};

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
