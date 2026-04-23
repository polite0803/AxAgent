//! Entity and Relationship types for Knowledge Graph
//!
//! These types are used by the memory service for entity extraction
//! and relationship tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Project,
    User,
    Concept,
    File,
    Task,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Project => write!(f, "project"),
            Self::User => write!(f, "user"),
            Self::Concept => write!(f, "concept"),
            Self::File => write!(f, "file"),
            Self::Task => write!(f, "task"),
        }
    }
}

impl From<&str> for EntityType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "project" => Self::Project,
            "user" => Self::User,
            "concept" => Self::Concept,
            "file" => Self::File,
            "task" => Self::Task,
            _ => Self::Concept,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    PartOf,
    RelatedTo,
    DependsOn,
    Owns,
    Defines,
    Implements,
    Contains,
    Calls,
    MethodOf,
    Performs,
    AssociatedWith,
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PartOf => write!(f, "part_of"),
            Self::RelatedTo => write!(f, "related_to"),
            Self::DependsOn => write!(f, "depends_on"),
            Self::Owns => write!(f, "owns"),
            Self::Defines => write!(f, "defines"),
            Self::Implements => write!(f, "implements"),
            Self::Contains => write!(f, "contains"),
            Self::Calls => write!(f, "calls"),
            Self::MethodOf => write!(f, "method_of"),
            Self::Performs => write!(f, "performs"),
            Self::AssociatedWith => write!(f, "associated_with"),
        }
    }
}

impl From<&str> for RelationshipType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().replace('_', "").as_str() {
            "partof" => Self::PartOf,
            "relatedto" => Self::RelatedTo,
            "dependson" => Self::DependsOn,
            "owns" => Self::Owns,
            "defines" => Self::Defines,
            "implements" => Self::Implements,
            "contains" => Self::Contains,
            "calls" => Self::Calls,
            "methodof" => Self::MethodOf,
            "performs" => Self::Performs,
            "associatedwith" => Self::AssociatedWith,
            _ => Self::RelatedTo,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub entity_type: EntityType,
    pub properties: HashMap<String, serde_json::Value>,
    pub aliases: Vec<String>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub mention_count: u32,
    pub confidence: f64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    #[serde(rename = "type")]
    pub relation_type: RelationshipType,
    pub properties: HashMap<String, serde_json::Value>,
    pub weight: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphQuery {
    pub entity_types: Option<Vec<EntityType>>,
    pub relationship_types: Option<Vec<RelationshipType>>,
    pub min_confidence: Option<f64>,
    pub min_weight: Option<f64>,
    pub since: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NudgeCandidate {
    pub entity: Entity,
    pub reason: String,
    pub urgency: String,
    pub suggested_action: Option<String>,
}