//! Adversarial Debate - Dual agent iterative improvement
//!
//! Features:
//! - Pro/Con debate rounds
//! - Argument strength scoring
//! - Refutation tracking
//! - Convergence detection

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateTopic {
    pub id: String,
    pub question: String,
    pub context: String,
    pub debate_type: DebateType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebateType {
    ProCon,
    Analysis,
    Solution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateRound {
    pub round_number: usize,
    pub side: DebateSide,
    pub argument: String,
    pub strength: f64,
    pub rebuttals: Vec<Rebuttal>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rebuttal {
    pub target_round: usize,
    pub counter_argument: String,
    pub strength: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebateSide {
    Pro,
    Con,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateResult {
    pub topic_id: String,
    pub winner: DebateSide,
    pub final_arguments: Vec<DebateRound>,
    pub pro_score: f64,
    pub con_score: f64,
    pub consensus_reached: bool,
    pub rounds_count: usize,
}

pub struct Debate {
    topic: DebateTopic,
    rounds: Vec<DebateRound>,
    pro_score: f64,
    con_score: f64,
}

pub struct DebateManager {
    debates: HashMap<String, Debate>,
    max_rounds: usize,
    strength_threshold: f64,
}

impl Default for DebateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DebateManager {
    pub fn new() -> Self {
        Self {
            debates: HashMap::new(),
            max_rounds: 5,
            strength_threshold: 0.7,
        }
    }

    pub fn with_max_rounds(mut self, rounds: usize) -> Self {
        self.max_rounds = rounds;
        self
    }

    pub fn with_strength_threshold(mut self, threshold: f64) -> Self {
        self.strength_threshold = threshold;
        self
    }

    pub fn start_debate(&mut self, topic: DebateTopic) -> String {
        let debate_id = format!("debate_{}", uuid_simple());

        let debate = Debate {
            topic,
            rounds: Vec::new(),
            pro_score: 0.0,
            con_score: 0.0,
        };

        self.debates.insert(debate_id.clone(), debate);
        debate_id
    }

    pub fn add_argument(
        &mut self,
        debate_id: &str,
        side: DebateSide,
        argument: &str,
        strength: f64,
    ) -> Result<usize, DebateError> {
        let debate = self
            .debates
            .get_mut(debate_id)
            .ok_or_else(|| DebateError::DebateNotFound(debate_id.to_string()))?;

        if debate.rounds.len() >= self.max_rounds {
            return Err(DebateError::MaxRoundsReached);
        }

        let round_number = debate.rounds.len();
        let current_side = side;
        let current_strength = strength;

        let rebuttal = {
            let rounds_ref = &debate.rounds;
            Self::generate_rebuttal_static(rounds_ref, current_side, round_number)
        };

        let round = DebateRound {
            round_number,
            side: current_side,
            argument: argument.to_string(),
            strength: current_strength,
            rebuttals: rebuttal.into_iter().collect(),
            timestamp: current_timestamp(),
        };

        match side {
            DebateSide::Pro => debate.pro_score += strength,
            DebateSide::Con => debate.con_score += strength,
            DebateSide::Neutral => {}
        }

        debate.rounds.push(round);

        Ok(round_number)
    }

    fn generate_rebuttal_static(
        rounds: &[DebateRound],
        side: DebateSide,
        current_round: usize,
    ) -> Vec<Rebuttal> {
        let mut rebuttals = Vec::new();

        let opposing_side = match side {
            DebateSide::Pro => DebateSide::Con,
            DebateSide::Con => DebateSide::Pro,
            DebateSide::Neutral => return rebuttals,
        };

        if current_round >= rounds.len() {
            return rebuttals;
        }

        let current_strength = rounds[current_round].strength;

        for (i, round) in rounds.iter().enumerate() {
            if round.side == opposing_side {
                rebuttals.push(Rebuttal {
                    target_round: i,
                    counter_argument: format!(
                        "Rebutting: {} (strength: {:.2})",
                        round.argument.chars().take(50).collect::<String>(),
                        round.strength
                    ),
                    strength: current_strength * 0.8,
                });
            }
        }

        rebuttals
    }

    pub fn get_result(&self, debate_id: &str) -> Result<DebateResult, DebateError> {
        let debate = self
            .debates
            .get(debate_id)
            .ok_or_else(|| DebateError::DebateNotFound(debate_id.to_string()))?;

        let winner = if debate.pro_score > debate.con_score {
            DebateSide::Pro
        } else if debate.con_score > debate.pro_score {
            DebateSide::Con
        } else {
            DebateSide::Neutral
        };

        let score_diff = (debate.pro_score - debate.con_score).abs();
        let consensus_reached = debate.rounds.len() >= self.max_rounds / 2 && score_diff < 0.1;

        Ok(DebateResult {
            topic_id: debate.topic.id.clone(),
            winner,
            final_arguments: debate.rounds.clone(),
            pro_score: debate.pro_score,
            con_score: debate.con_score,
            consensus_reached,
            rounds_count: debate.rounds.len(),
        })
    }

    pub fn evaluate_strength(argument: &str, _context: &str) -> f64 {
        let mut score: f64 = 0.5;

        let has_evidence = argument.contains("data") || argument.contains("research");
        let has_reasoning = argument.contains("because") || argument.contains("therefore");
        let has_counter = argument.contains("however") || argument.contains("although");

        if has_evidence {
            score += 0.15;
        }
        if has_reasoning {
            score += 0.15;
        }
        if has_counter {
            score += 0.1;
        }

        score.min(1.0)
    }

    pub fn list_debates(&self) -> Vec<&Debate> {
        self.debates.values().collect()
    }

    pub fn end_debate(&mut self, debate_id: &str) -> Option<DebateResult> {
        let result = self.get_result(debate_id).ok()?;
        self.debates.remove(debate_id);
        Some(result)
    }
}

#[derive(Debug, Clone)]
pub enum DebateError {
    DebateNotFound(String),
    MaxRoundsReached,
    InvalidSide,
}

impl std::fmt::Display for DebateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DebateNotFound(id) => write!(f, "Debate not found: {}", id),
            Self::MaxRoundsReached => write!(f, "Maximum debate rounds reached"),
            Self::InvalidSide => write!(f, "Invalid debate side"),
        }
    }
}

impl std::error::Error for DebateError {}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn uuid_simple() -> String {
    let now = current_timestamp();
    format!("{:x}", now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debate_creation() {
        let mut manager = DebateManager::new();

        let topic = DebateTopic {
            id: "topic_1".to_string(),
            question: "Should we implement feature X?".to_string(),
            context: "Feature X requires significant resources".to_string(),
            debate_type: DebateType::ProCon,
        };

        let debate_id = manager.start_debate(topic);
        assert!(!debate_id.is_empty());
    }

    #[test]
    fn test_debate_arguments() {
        let mut manager = DebateManager::new();

        let topic = DebateTopic {
            id: "topic_1".to_string(),
            question: "Is this a good approach?".to_string(),
            context: "Context here".to_string(),
            debate_type: DebateType::ProCon,
        };

        let debate_id = manager.start_debate(topic);

        manager
            .add_argument(
                &debate_id,
                DebateSide::Pro,
                "This is good because data",
                0.8,
            )
            .unwrap();

        let result = manager.get_result(&debate_id).unwrap();
        assert_eq!(result.rounds_count, 1);
        assert_eq!(result.pro_score, 0.8);
    }

    #[test]
    fn test_debate_winner() {
        let mut manager = DebateManager::new();

        let topic = DebateTopic {
            id: "topic_1".to_string(),
            question: "Which is better?".to_string(),
            context: "Comparing A and B".to_string(),
            debate_type: DebateType::ProCon,
        };

        let debate_id = manager.start_debate(topic);

        manager
            .add_argument(
                &debate_id,
                DebateSide::Pro,
                "A is better due to performance",
                0.9,
            )
            .unwrap();
        manager
            .add_argument(&debate_id, DebateSide::Con, "B has lower cost", 0.7)
            .unwrap();

        let result = manager.get_result(&debate_id).unwrap();
        assert_eq!(result.winner, DebateSide::Pro);
    }

    #[test]
    fn test_strength_evaluation() {
        let score =
            DebateManager::evaluate_strength("This is good because research shows it", "context");
        assert!(score > 0.5);
    }
}
