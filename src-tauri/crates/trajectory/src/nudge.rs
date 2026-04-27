//! Nudge service module
//!
//! Replaces TypeScript `NudgeService.ts` with Rust implementation.
//! Provides real-time nudge generation, presentation, and action tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Urgency {
    Low,
    Medium,
    High,
}

impl Urgency {
    fn to_number(self) -> u32 {
        match self {
            Urgency::Low => 0,
            Urgency::Medium => 1,
            Urgency::High => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NudgeType {
    LowActivity,
    BestPractice,
    Improvement,
    Reminder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NudgeMessage {
    pub id: String,
    #[serde(rename = "nudgeType")]
    pub nudge_type: NudgeType,
    pub message: String,
    #[serde(rename = "suggestedAction")]
    pub suggested_action: Option<String>,
    pub priority: u32,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NudgeConfig {
    #[serde(rename = "maxNudgesPerSession")]
    pub max_nudges_per_session: usize,
    #[serde(rename = "minUrgencyThreshold")]
    pub min_urgency_threshold: Urgency,
    #[serde(rename = "autoAddHighConfidence")]
    pub auto_add_high_confidence: bool,
    #[serde(rename = "nudgeHistorySize")]
    pub nudge_history_size: usize,
    #[serde(rename = "reminderDecayHours")]
    pub reminder_decay_hours: i64,
    #[serde(rename = "proactiveSuggestionInterval")]
    pub proactive_suggestion_interval: i64,
    #[serde(rename = "contextualReminderWeight")]
    pub contextual_reminder_weight: f64,
}

impl Default for NudgeConfig {
    fn default() -> Self {
        Self {
            max_nudges_per_session: 3,
            min_urgency_threshold: Urgency::Medium,
            auto_add_high_confidence: true,
            nudge_history_size: 50,
            reminder_decay_hours: 24,
            proactive_suggestion_interval: 300000,
            contextual_reminder_weight: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nudge {
    pub id: String,
    #[serde(rename = "entityId")]
    pub entity_id: String,
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub reason: String,
    pub urgency: Urgency,
    #[serde(rename = "suggestedAction")]
    pub suggested_action: Option<String>,
    pub presented: bool,
    #[serde(rename = "actionTaken")]
    pub action_taken: Option<NudgeAction>,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "presentedAt")]
    pub presented_at: Option<i64>,
    #[serde(rename = "dismissedAt")]
    pub dismissed_at: Option<i64>,
    #[serde(rename = "snoozedUntil")]
    pub snoozed_until: Option<i64>,
    #[serde(rename = "recurrenceCount")]
    pub recurrence_count: u32,
    #[serde(rename = "lastRecurrenceAt")]
    pub last_recurrence_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NudgeAction {
    AddedToMemory,
    Dismissed,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NudgeSession {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub nudges: Vec<Nudge>,
    #[serde(rename = "startedAt")]
    pub started_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NudgeCandidate {
    pub entity: NudgeEntity,
    pub reason: String,
    pub urgency: Urgency,
    #[serde(rename = "suggestedAction")]
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NudgeEntity {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub confidence: f64,
}

pub struct NudgeService {
    config: NudgeConfig,
    session: Option<NudgeSession>,
    history: Vec<Nudge>,
}

impl Default for NudgeService {
    fn default() -> Self {
        Self::new()
    }
}

impl NudgeService {
    pub fn new() -> Self {
        Self {
            config: NudgeConfig::default(),
            session: None,
            history: Vec::new(),
        }
    }

    pub fn with_config(config: NudgeConfig) -> Self {
        Self {
            config,
            session: None,
            history: Vec::new(),
        }
    }

    pub fn get_config(&self) -> &NudgeConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: NudgeConfig) {
        self.config = config;
    }

    fn generate_nudge_id(&self) -> String {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let random: String = (0..7)
            .map(|_| {
                let idx = (timestamp % 36) as usize;
                let chars = b"0123456789abcdefghijklmnopqrstuvwxyz";
                chars[idx] as char
            })
            .collect();
        format!("nudge_{}_{}", timestamp, random)
    }

    fn urgency_to_number(urgency: &Urgency) -> u32 {
        urgency.to_number()
    }

    pub fn start_session(&mut self, session_id: String) -> &mut Self {
        self.session = Some(NudgeSession {
            session_id,
            nudges: Vec::new(),
            started_at: chrono::Utc::now().timestamp_millis(),
        });
        self
    }

    pub fn get_session(&self) -> Option<&NudgeSession> {
        self.session.as_ref()
    }

    pub fn generate_nudges(
        &mut self,
        context: NudgeContext,
        candidates: Vec<NudgeCandidate>,
    ) -> Vec<Nudge> {
        let session_id = context.session_id.clone();

        let existing_ids: HashSet<String> = if let Some(session) = &self.session {
            if session.session_id == session_id {
                session.nudges.iter().map(|n| n.entity_id.clone()).collect()
            } else {
                HashSet::new()
            }
        } else {
            HashSet::new()
        };

        if self.session.is_none()
            || self.session.as_ref().map(|s| &s.session_id) != Some(&session_id)
        {
            self.start_session(session_id);
        }

        let filtered_candidates: Vec<&NudgeCandidate> = candidates
            .iter()
            .filter(|c| !existing_ids.contains(&c.entity.id))
            .collect();

        let urgency_threshold = Self::urgency_to_number(&self.config.min_urgency_threshold);
        let mut new_nudges: Vec<Nudge> = Vec::new();

        for candidate in filtered_candidates {
            if new_nudges.len() >= self.config.max_nudges_per_session {
                break;
            }

            let candidate_urgency = Self::urgency_to_number(&candidate.urgency);
            if candidate_urgency < urgency_threshold {
                continue;
            }

            let nudge = Nudge {
                id: self.generate_nudge_id(),
                entity_id: candidate.entity.id.clone(),
                entity_name: candidate.entity.name.clone(),
                reason: candidate.reason.clone(),
                urgency: candidate.urgency,
                suggested_action: candidate.suggested_action.clone(),
                presented: false,
                action_taken: None,
                created_at: chrono::Utc::now().timestamp_millis(),
                presented_at: None,
                dismissed_at: None,
                snoozed_until: None,
                recurrence_count: 0,
                last_recurrence_at: None,
            };

            new_nudges.push(nudge);
        }

        if self.config.auto_add_high_confidence {
            for candidate in &candidates {
                if candidate.entity.confidence > 0.8
                    && candidate.urgency == Urgency::High
                    && !existing_ids.contains(&candidate.entity.id)
                {
                    let nudge = Nudge {
                        id: self.generate_nudge_id(),
                        entity_id: candidate.entity.id.clone(),
                        entity_name: candidate.entity.name.clone(),
                        reason: format!("Auto-added: {}", candidate.reason),
                        urgency: candidate.urgency,
                        suggested_action: Some("Auto-added to working memory".to_string()),
                        presented: false,
                        action_taken: Some(NudgeAction::AddedToMemory),
                        created_at: chrono::Utc::now().timestamp_millis(),
                        presented_at: None,
                        dismissed_at: None,
                        snoozed_until: None,
                        recurrence_count: 0,
                        last_recurrence_at: None,
                    };
                    new_nudges.push(nudge);
                }
            }
        }

        if let Some(session) = &mut self.session {
            session.nudges.extend(new_nudges.clone());
        }
        self.history.extend(new_nudges.clone());

        if self.history.len() > self.config.nudge_history_size {
            let drain_count = self.history.len() - self.config.nudge_history_size;
            self.history.drain(..drain_count);
        }

        new_nudges
    }

    pub fn get_pending_nudges(&self, session_id: &str) -> Vec<&Nudge> {
        let max_nudges = self.config.max_nudges_per_session;
        if let Some(session) = &self.session {
            if session.session_id == session_id {
                return session
                    .nudges
                    .iter()
                    .filter(|n| !n.presented)
                    .take(max_nudges)
                    .collect();
            }
        }
        Vec::new()
    }

    pub fn mark_nudge_presented(&mut self, nudge_id: &str) -> bool {
        if let Some(session) = &mut self.session {
            if let Some(nudge) = session.nudges.iter_mut().find(|n| n.id == nudge_id) {
                nudge.presented = true;
                nudge.presented_at = Some(chrono::Utc::now().timestamp_millis());
                return true;
            }
        }
        false
    }

    pub fn take_nudge_action(&mut self, nudge_id: &str, action: NudgeAction) -> bool {
        let dismiss_time = if action == NudgeAction::Dismissed {
            Some(chrono::Utc::now().timestamp_millis())
        } else {
            None
        };

        if let Some(session) = &mut self.session {
            if let Some(nudge) = session.nudges.iter_mut().find(|n| n.id == nudge_id) {
                nudge.action_taken = Some(action);
                if let Some(t) = dismiss_time {
                    nudge.dismissed_at = Some(t);
                }
                return true;
            }
        }
        false
    }

    pub fn snooze_nudge(&mut self, nudge_id: &str, until: i64) -> bool {
        if let Some(session) = &mut self.session {
            if let Some(nudge) = session.nudges.iter_mut().find(|n| n.id == nudge_id) {
                nudge.snoozed_until = Some(until);
                return true;
            }
        }
        false
    }

    pub fn get_nudge_stats(&self) -> NudgeStats {
        let total = self.history.len();
        let presented = self.history.iter().filter(|n| n.presented).count();
        let added_to_memory = self
            .history
            .iter()
            .filter(|n| n.action_taken == Some(NudgeAction::AddedToMemory))
            .count();
        let dismissed = self
            .history
            .iter()
            .filter(|n| n.action_taken == Some(NudgeAction::Dismissed))
            .count();
        let pending = self
            .history
            .iter()
            .filter(|n| n.action_taken == Some(NudgeAction::Pending))
            .count();

        NudgeStats {
            total_nudges: total,
            presented_count: presented,
            added_to_memory_count: added_to_memory,
            dismissed_count: dismissed,
            pending_count: pending,
            acceptance_rate: if presented > 0 {
                added_to_memory as f64 / presented as f64
            } else {
                0.0
            },
        }
    }

    pub fn get_recent_nudges(&self, limit: usize) -> Vec<&Nudge> {
        self.history.iter().rev().take(limit).collect()
    }

    pub fn clear_session(&mut self) {
        self.session = None;
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NudgeContext {
    #[serde(rename = "currentTask")]
    pub current_task: Option<String>,
    #[serde(rename = "recentEntities")]
    pub recent_entities: Option<Vec<String>>,
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NudgeStats {
    #[serde(rename = "totalNudges")]
    pub total_nudges: usize,
    #[serde(rename = "presentedCount")]
    pub presented_count: usize,
    #[serde(rename = "addedToMemoryCount")]
    pub added_to_memory_count: usize,
    #[serde(rename = "dismissedCount")]
    pub dismissed_count: usize,
    #[serde(rename = "pendingCount")]
    pub pending_count: usize,
    #[serde(rename = "acceptanceRate")]
    pub acceptance_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nudge_creation() {
        let mut service = NudgeService::new();
        service.start_session("test_session".to_string());

        let candidates = vec![NudgeCandidate {
            entity: NudgeEntity {
                id: "entity_1".to_string(),
                name: "Test Entity".to_string(),
                entity_type: "concept".to_string(),
                confidence: 0.7,
            },
            reason: "Test reason".to_string(),
            urgency: Urgency::High,
            suggested_action: Some("Take action".to_string()),
        }];

        let context = NudgeContext {
            current_task: Some("Testing".to_string()),
            recent_entities: None,
            session_id: "test_session".to_string(),
        };

        let nudges = service.generate_nudges(context, candidates);
        assert_eq!(nudges.len(), 1);
        assert_eq!(nudges[0].entity_name, "Test Entity");
    }

    #[test]
    fn test_nudge_presentation() {
        let mut service = NudgeService::new();
        service.start_session("test_session".to_string());

        let candidates = vec![NudgeCandidate {
            entity: NudgeEntity {
                id: "entity_1".to_string(),
                name: "Test".to_string(),
                entity_type: "concept".to_string(),
                confidence: 0.9,
            },
            reason: "High confidence".to_string(),
            urgency: Urgency::High,
            suggested_action: None,
        }];

        let context = NudgeContext {
            current_task: None,
            recent_entities: None,
            session_id: "test_session".to_string(),
        };

        service.generate_nudges(context, candidates);

        let pending = service.get_pending_nudges("test_session");
        assert!(!pending.is_empty());

        if let Some(nudge) = pending.first() {
            assert!(!nudge.presented);
        }
    }

    #[test]
    fn test_nudge_action() {
        let mut service = NudgeService::new();
        service.start_session("test_session".to_string());

        let candidates = vec![NudgeCandidate {
            entity: NudgeEntity {
                id: "entity_1".to_string(),
                name: "Test".to_string(),
                entity_type: "concept".to_string(),
                confidence: 0.7,
            },
            reason: "Test".to_string(),
            urgency: Urgency::Medium,
            suggested_action: None,
        }];

        let context = NudgeContext {
            current_task: None,
            recent_entities: None,
            session_id: "test_session".to_string(),
        };

        let nudges = service.generate_nudges(context, candidates);
        let nudge_id = &nudges[0].id;

        let result = service.take_nudge_action(nudge_id, NudgeAction::Dismissed);
        assert!(result);

        let stats = service.get_nudge_stats();
        assert_eq!(stats.dismissed_count, 1);
    }

    #[test]
    fn test_auto_add_high_confidence() {
        let mut config = NudgeConfig::default();
        config.auto_add_high_confidence = true;
        let mut service = NudgeService::with_config(config);
        service.start_session("test_session".to_string());

        let candidates = vec![NudgeCandidate {
            entity: NudgeEntity {
                id: "entity_1".to_string(),
                name: "High Confidence".to_string(),
                entity_type: "concept".to_string(),
                confidence: 0.85,
            },
            reason: "Auto-add test".to_string(),
            urgency: Urgency::High,
            suggested_action: None,
        }];

        let context = NudgeContext {
            current_task: None,
            recent_entities: None,
            session_id: "test_session".to_string(),
        };

        let nudges = service.generate_nudges(context, candidates);

        let auto_added = nudges
            .iter()
            .filter(|n| n.action_taken == Some(NudgeAction::AddedToMemory))
            .count();
        assert!(auto_added >= 1);
    }
}
