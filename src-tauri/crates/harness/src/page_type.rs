// SPDX-License-Identifier: AGPL-3.0-only

//! PageType enum — pure DTO migrated from dao.
//! Classifies a graph page into semantic categories.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PageType {
    Entity,
    Concept,
    SourceSummary,
    Comparison,
    Index,
    Overview,
    Note,
    Unknown,
}

impl FromStr for PageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "entity" => Ok(Self::Entity),
            "concept" => Ok(Self::Concept),
            "source_summary" | "source-summary" => Ok(Self::SourceSummary),
            "comparison" => Ok(Self::Comparison),
            "index" => Ok(Self::Index),
            "overview" => Ok(Self::Overview),
            "note" => Ok(Self::Note),
            _ => Ok(Self::Unknown),
        }
    }
}

impl PageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Concept => "concept",
            Self::SourceSummary => "source_summary",
            Self::Comparison => "comparison",
            Self::Index => "index",
            Self::Overview => "overview",
            Self::Note => "note",
            Self::Unknown => "unknown",
        }
    }
}
