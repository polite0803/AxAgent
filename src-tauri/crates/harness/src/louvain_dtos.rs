// SPDX-License-Identifier: AGPL-3.0-only

//! Louvain DTOs — pure types migrated from dao.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

const LOW_COHESION_THRESHOLD: f64 = 0.15;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LouvainResult {
    pub communities: HashMap<String, i32>,
    pub cohesion_scores: HashMap<i32, f64>,
    pub community_sizes: HashMap<i32, usize>,
    pub top_nodes: HashMap<i32, String>,
    pub modularity: f64,
    pub num_communities: usize,
    pub color_palette: Vec<String>,
}

impl LouvainResult {
    pub fn default_palette() -> Vec<String> {
        vec![
            "#4C72B0".to_string(),
            "#DD8452".to_string(),
            "#55A868".to_string(),
            "#C44E52".to_string(),
            "#8172B3".to_string(),
            "#937860".to_string(),
            "#DA8BC3".to_string(),
            "#8C8C8C".to_string(),
            "#CCB974".to_string(),
            "#64B5CD".to_string(),
            "#E18B6C".to_string(),
            "#7AA153".to_string(),
        ]
    }

    pub fn get_color(&self, community_id: i32) -> String {
        let idx = community_id.rem_euclid(self.color_palette.len() as i32) as usize;
        self.color_palette[idx].clone()
    }

    pub fn is_low_cohesion(&self, community_id: i32) -> bool {
        self.cohesion_scores
            .get(&community_id)
            .map(|s| *s < LOW_COHESION_THRESHOLD)
            .unwrap_or(false)
    }

    pub fn get_community_nodes(&self, community_id: i32) -> Vec<String> {
        self.communities
            .iter()
            .filter(|(_, c)| **c == community_id)
            .map(|(n, _)| n.clone())
            .collect()
    }
}
