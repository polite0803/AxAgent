use crate::behavior_tracker::BehaviorEvent;
use crate::pattern_analyzer::PatternAnalyzer;
use crate::user_profile::{calculate_confidence, ProfileUpdate, UpdateSource, UserProfile};
use chrono::{DateTime, Duration, Utc};

pub struct PreferenceLearner {
    profile: UserProfile,
    event_buffer: Vec<BehaviorEvent>,
    analyzer: PatternAnalyzer,
    last_analysis: DateTime<Utc>,
    batch_size: usize,
}

impl PreferenceLearner {
    pub fn new(user_id: String) -> Self {
        Self {
            profile: UserProfile::with_user_id(user_id),
            event_buffer: Vec::new(),
            analyzer: PatternAnalyzer::new(),
            last_analysis: Utc::now(),
            batch_size: 50,
        }
    }

    pub fn with_profile(profile: UserProfile) -> Self {
        Self {
            profile,
            event_buffer: Vec::new(),
            analyzer: PatternAnalyzer::new(),
            last_analysis: Utc::now(),
            batch_size: 50,
        }
    }

    pub fn get_profile(&self) -> &UserProfile {
        &self.profile
    }

    pub fn get_mut_profile(&mut self) -> &mut UserProfile {
        &mut self.profile
    }

    pub fn process_event(&mut self, event: BehaviorEvent) -> Vec<ProfileUpdate> {
        self.event_buffer.push(event);
        self.profile.learning_state.increment_interactions();

        if self.event_buffer.len() >= self.batch_size {
            self.analyze_and_update()
        } else {
            Vec::new()
        }
    }

    pub fn analyze_and_update(&mut self) -> Vec<ProfileUpdate> {
        let mut updates = Vec::new();
        let events: Vec<BehaviorEvent> = self.event_buffer.drain(..).collect();

        if events.is_empty() {
            return updates;
        }

        let patterns = self.analyzer.analyze(&events);

        let old_coding = self.profile.coding_style.clone();
        let new_coding = self
            .analyzer
            .infer_coding_profile(&patterns.coding_patterns);
        if old_coding != new_coding {
            let confidence_change = new_coding.confidence - old_coding.confidence;
            updates.push(ProfileUpdate {
                field_changed: "coding_style".to_string(),
                old_value: serde_json::to_value(&old_coding).unwrap_or_default(),
                new_value: serde_json::to_value(&new_coding).unwrap_or_default(),
                confidence_change,
                source: UpdateSource::Inferred,
            });
            self.profile.coding_style = new_coding;
        }

        let old_comm = self.profile.communication.clone();
        let new_comm = self.analyzer.infer_communication_profile(&events);
        if old_comm != new_comm {
            let confidence_change = new_comm.confidence - old_comm.confidence;
            updates.push(ProfileUpdate {
                field_changed: "communication".to_string(),
                old_value: serde_json::to_value(&old_comm).unwrap_or_default(),
                new_value: serde_json::to_value(&new_comm).unwrap_or_default(),
                confidence_change,
                source: UpdateSource::Inferred,
            });
            self.profile.communication = new_comm;
        }

        let old_work = self.profile.work_habits.clone();
        let new_work = self.analyzer.infer_work_habit_profile(
            &patterns.temporal_patterns,
            &patterns.tool_preference_patterns,
        );
        if old_work != new_work {
            let confidence_change = new_work.confidence - old_work.confidence;
            updates.push(ProfileUpdate {
                field_changed: "work_habits".to_string(),
                old_value: serde_json::to_value(&old_work).unwrap_or_default(),
                new_value: serde_json::to_value(&new_work).unwrap_or_default(),
                confidence_change,
                source: UpdateSource::Inferred,
            });
            self.profile.work_habits = new_work;
        }

        self.update_domain_knowledge(&patterns.topic_patterns);

        self.last_analysis = Utc::now();
        self.profile.update_timestamp();

        updates
    }

    fn update_domain_knowledge(&mut self, topics: &[crate::pattern_analyzer::TopicPattern]) {
        for topic in topics {
            if topic.topic.starts_with("code:") {
                let lang = topic.topic.trim_start_matches("code:");
                if !self
                    .profile
                    .domain_knowledge
                    .expertise_areas
                    .iter()
                    .any(|a| a.name == lang)
                {
                    self.profile.domain_knowledge.expertise_areas.push(
                        crate::user_profile::ExpertiseArea {
                            name: lang.to_string(),
                            level: crate::user_profile::SkillLevel::Intermediate,
                            years_experience: 1,
                            last_applied: topic.recency,
                        },
                    );
                }
            } else if !topic.topic.contains(':')
                && !self
                    .profile
                    .domain_knowledge
                    .interest_topics
                    .contains(&topic.topic)
            {
                self.profile
                    .domain_knowledge
                    .interest_topics
                    .push(topic.topic.clone());
            }
        }

        self.profile.domain_knowledge.confidence =
            calculate_confidence(self.event_buffer.len() as u32, 168);
    }

    pub fn apply_explicit_update(
        &mut self,
        field: &str,
        value: serde_json::Value,
    ) -> ProfileUpdate {
        let old_value = match field {
            "naming_convention" => {
                serde_json::to_value(&self.profile.coding_style.naming_convention)
                    .unwrap_or_default()
            }
            "indentation_style" => {
                serde_json::to_value(&self.profile.coding_style.indentation_style)
                    .unwrap_or_default()
            }
            "comment_style" => {
                serde_json::to_value(&self.profile.coding_style.comment_style).unwrap_or_default()
            }
            "detail_level" => {
                serde_json::to_value(&self.profile.communication.detail_level).unwrap_or_default()
            }
            "tone" => serde_json::to_value(&self.profile.communication.tone).unwrap_or_default(),
            "language" => {
                serde_json::to_value(&self.profile.communication.language).unwrap_or_default()
            }
            _ => serde_json::Value::Null,
        };

        match field {
            "naming_convention" => {
                if let Ok(nc) =
                    serde_json::from_value::<crate::user_profile::NamingConvention>(value.clone())
                {
                    self.profile.coding_style.naming_convention = nc;
                }
            }
            "indentation_style" => {
                if let Ok(is) =
                    serde_json::from_value::<crate::user_profile::IndentationStyle>(value.clone())
                {
                    self.profile.coding_style.indentation_style = is;
                }
            }
            "comment_style" => {
                if let Ok(cs) =
                    serde_json::from_value::<crate::user_profile::CommentStyle>(value.clone())
                {
                    self.profile.coding_style.comment_style = cs;
                }
            }
            "detail_level" => {
                if let Ok(dl) =
                    serde_json::from_value::<crate::user_profile::DetailLevel>(value.clone())
                {
                    self.profile.communication.detail_level = dl;
                }
            }
            "tone" => {
                if let Ok(t) = serde_json::from_value::<crate::user_profile::Tone>(value.clone()) {
                    self.profile.communication.tone = t;
                }
            }
            "language" => {
                if let Ok(lang) = serde_json::from_value::<String>(value.clone()) {
                    self.profile.communication.language = lang;
                }
            }
            _ => {}
        }

        self.profile
            .learning_state
            .add_explicit_setting(field.to_string());
        self.profile.update_timestamp();

        ProfileUpdate {
            field_changed: field.to_string(),
            old_value,
            new_value: value,
            confidence_change: 0.1,
            source: UpdateSource::Explicit,
        }
    }

    pub fn calculate_stability(&self) -> f32 {
        let interactions = self.profile.learning_state.total_interactions;
        let time_span = Utc::now().signed_duration_since(self.profile.learning_state.last_updated);
        let time_hours = time_span.num_hours() as f32;

        let interaction_factor = (interactions as f32 / 100.0).min(1.0);
        let time_factor = if time_hours > 24.0 {
            1.0
        } else {
            time_hours / 24.0
        };

        (interaction_factor * 0.7 + time_factor * 0.3).min(1.0)
    }

    pub fn needs_refresh(&self) -> bool {
        let time_since_analysis = Utc::now().signed_duration_since(self.last_analysis);
        time_since_analysis > Duration::hours(1) || self.event_buffer.len() >= self.batch_size
    }

    pub fn merge_profile(&mut self, other: UserProfile) {
        if other.learning_state.total_interactions > self.profile.learning_state.total_interactions
        {
            self.profile.coding_style = other.coding_style;
            self.profile.communication = other.communication;
            self.profile.work_habits = other.work_habits;
            self.profile.domain_knowledge = other.domain_knowledge;
        }
        self.profile.learning_state.stability_score = self.calculate_stability();
    }
}

pub struct LearningMetrics {
    pub total_events_processed: u64,
    pub updates_generated: u64,
    pub avg_confidence_gain: f32,
    pub last_learning_time: Option<DateTime<Utc>>,
}

impl Default for LearningMetrics {
    fn default() -> Self {
        Self {
            total_events_processed: 0,
            updates_generated: 0,
            avg_confidence_gain: 0.0,
            last_learning_time: None,
        }
    }
}
