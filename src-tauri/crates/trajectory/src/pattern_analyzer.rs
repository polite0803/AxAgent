use crate::behavior_tracker::{BehaviorEvent, BehaviorEventType};
use crate::user_profile::{
    CodingStyleProfile, CommentStyle, CommunicationProfile, DetailLevel, IndentationStyle,
    NamingConvention, TimeRange, Tone, ToolUsagePattern,
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ExtractedPatterns {
    pub coding_patterns: Vec<CodingPatternMatch>,
    pub temporal_patterns: Vec<TemporalPattern>,
    pub tool_preference_patterns: Vec<ToolPreferencePattern>,
    pub topic_patterns: Vec<TopicPattern>,
}

#[derive(Debug, Clone)]
pub struct CodingPatternMatch {
    pub pattern_type: PatternType,
    pub value: String,
    pub confidence: f32,
    pub occurrences: u32,
}

#[derive(Debug, Clone)]
pub enum PatternType {
    Naming,
    Indentation,
    Comment,
    ModuleStructure,
    ErrorHandling,
}

#[derive(Debug, Clone)]
pub struct TemporalPattern {
    pub pattern_type: TemporalPatternType,
    pub time_range: TimeRange,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub enum TemporalPatternType {
    PeakHours,
    LowActivityHours,
    PreferredDays,
}

#[derive(Debug, Clone)]
pub struct ToolPreferencePattern {
    pub tool_name: String,
    pub usage_frequency: f32,
    pub avg_duration_ms: u64,
    pub success_rate: f32,
}

#[derive(Debug, Clone)]
pub struct TopicPattern {
    pub topic: String,
    pub frequency: u32,
    pub recency: DateTime<Utc>,
}

pub struct PatternAnalyzer {
    min_confidence_threshold: f32,
}

impl PatternAnalyzer {
    pub fn new() -> Self {
        Self {
            min_confidence_threshold: 0.5,
        }
    }

    pub fn analyze(&self, events: &[BehaviorEvent]) -> ExtractedPatterns {
        let coding_patterns = self.extract_coding_patterns(events);
        let temporal_patterns = self.extract_temporal_patterns(events);
        let tool_preference_patterns = self.extract_tool_preference_patterns(events);
        let topic_patterns = self.extract_topic_patterns(events);

        let min_threshold = self.min_confidence_threshold;

        ExtractedPatterns {
            coding_patterns: coding_patterns
                .into_iter()
                .filter(|p| p.confidence >= min_threshold)
                .collect(),
            temporal_patterns: temporal_patterns
                .into_iter()
                .filter(|p| p.confidence >= min_threshold)
                .collect(),
            tool_preference_patterns: tool_preference_patterns
                .into_iter()
                .filter(|p| p.success_rate >= min_threshold)
                .collect(),
            topic_patterns: topic_patterns
                .into_iter()
                .filter(|p| p.frequency as f32 >= (min_threshold * 10.0))
                .collect(),
        }
    }

    fn extract_coding_patterns(&self, events: &[BehaviorEvent]) -> Vec<CodingPatternMatch> {
        let mut patterns = Vec::new();
        let mut naming_counts: HashMap<String, u32> = HashMap::new();
        let mut indentation_counts: HashMap<String, u32> = HashMap::new();
        let mut comment_counts: HashMap<String, u32> = HashMap::new();

        for event in events {
            match &event.event_type {
                BehaviorEventType::CodeGeneration {
                    language,
                    line_count,
                    ..
                } => {
                    let naming_key = format!("lang:{}", language);
                    *naming_counts.entry(naming_key).or_insert(0) += 1;

                    if *line_count > 100 {
                        *indentation_counts
                            .entry("spacious".to_string())
                            .or_insert(0) += 1;
                    } else {
                        *indentation_counts.entry("compact".to_string()).or_insert(0) += 1;
                    }
                },
                BehaviorEventType::FileEdited {
                    edit_type,
                    lines_changed,
                    ..
                } => {
                    if *lines_changed > 50 {
                        *comment_counts.entry("extensive".to_string()).or_insert(0) += 1;
                    } else {
                        *comment_counts.entry("minimal".to_string()).or_insert(0) += 1;
                    }

                    if *edit_type == "refactor" {
                        *naming_counts.entry("refactoring".to_string()).or_insert(0) += 1;
                    }
                },
                _ => {},
            }
        }

        for (naming, count) in naming_counts {
            if count >= 3 {
                patterns.push(CodingPatternMatch {
                    pattern_type: PatternType::Naming,
                    value: naming,
                    confidence: (count as f32 / 10.0).min(1.0),
                    occurrences: count,
                });
            }
        }

        for (indentation, count) in indentation_counts {
            if count >= 2 {
                patterns.push(CodingPatternMatch {
                    pattern_type: PatternType::Indentation,
                    value: indentation,
                    confidence: (count as f32 / 5.0).min(1.0),
                    occurrences: count,
                });
            }
        }

        for (comment, count) in comment_counts {
            if count >= 2 {
                patterns.push(CodingPatternMatch {
                    pattern_type: PatternType::Comment,
                    value: comment,
                    confidence: (count as f32 / 5.0).min(1.0),
                    occurrences: count,
                });
            }
        }

        patterns
    }

    fn extract_temporal_patterns(&self, events: &[BehaviorEvent]) -> Vec<TemporalPattern> {
        let mut patterns = Vec::new();
        let mut hour_counts: HashMap<u8, u32> = HashMap::new();
        let mut day_counts: HashMap<u8, u32> = HashMap::new();

        for event in events {
            if let Some(hour) = event.context.time_of_day {
                *hour_counts.entry(hour).or_insert(0) += 1;
            }
            if let Some(day) = event.context.day_of_week {
                *day_counts.entry(day).or_insert(0) += 1;
            }
        }

        let mut hour_vec: Vec<_> = hour_counts.iter().collect();
        hour_vec.sort_by(|a, b| b.1.cmp(a.1));

        let mut peak_count: u32 = 0;
        if let Some((&peak_hour, &count)) = hour_vec.first() {
            peak_count = count;
            if count >= 5 {
                patterns.push(TemporalPattern {
                    pattern_type: TemporalPatternType::PeakHours,
                    time_range: TimeRange {
                        start_hour: peak_hour,
                        end_hour: (peak_hour + 2).min(23),
                        timezone: "UTC".to_string(),
                    },
                    confidence: (peak_count as f32 / 20.0).min(1.0),
                });
            }
        }

        if hour_vec.len() > 1 {
            if let Some((&low_hour, &low_count)) = hour_vec.last() {
                if low_count >= 3 && peak_count > 0 && low_count < peak_count / 2 {
                    patterns.push(TemporalPattern {
                        pattern_type: TemporalPatternType::LowActivityHours,
                        time_range: TimeRange {
                            start_hour: low_hour,
                            end_hour: (low_hour + 2).min(23),
                            timezone: "UTC".to_string(),
                        },
                        confidence: 0.5,
                    });
                }
            }
        }

        let mut day_vec: Vec<_> = day_counts.iter().collect();
        day_vec.sort_by(|a, b| b.1.cmp(a.1));

        if let Some((&preferred_day, &day_count)) = day_vec.first() {
            if day_count >= 10
                && day_count as f32 > day_vec.iter().map(|(_, c)| *c).sum::<u32>() as f32 * 0.4
            {
                patterns.push(TemporalPattern {
                    pattern_type: TemporalPatternType::PreferredDays,
                    time_range: TimeRange {
                        start_hour: preferred_day * 24,
                        end_hour: preferred_day * 24 + 23,
                        timezone: "UTC".to_string(),
                    },
                    confidence: (day_count as f32 / 30.0).min(1.0),
                });
            }
        }

        patterns
    }

    fn extract_tool_preference_patterns(
        &self,
        events: &[BehaviorEvent],
    ) -> Vec<ToolPreferencePattern> {
        let mut tool_stats: HashMap<String, ToolStats> = HashMap::new();

        for event in events {
            if let BehaviorEventType::ToolUsage {
                tool_name,
                success,
                duration_ms,
            } = &event.event_type
            {
                let stats = tool_stats
                    .entry(tool_name.clone())
                    .or_insert_with(|| ToolStats {
                        usage_count: 0,
                        success_count: 0,
                        total_duration_ms: 0,
                    });
                stats.usage_count += 1;
                if *success {
                    stats.success_count += 1;
                }
                stats.total_duration_ms += duration_ms;
            }
        }

        let total_events = events.len() as f32;
        let mut patterns = Vec::new();

        for (tool_name, stats) in tool_stats {
            if stats.usage_count >= 3 {
                patterns.push(ToolPreferencePattern {
                    tool_name,
                    usage_frequency: stats.usage_count as f32 / total_events,
                    avg_duration_ms: stats.total_duration_ms / stats.usage_count as u64,
                    success_rate: stats.success_count as f32 / stats.usage_count as f32,
                });
            }
        }

        patterns.sort_by(|a, b| b.usage_frequency.partial_cmp(&a.usage_frequency).unwrap());
        patterns.truncate(10);

        patterns
    }

    fn extract_topic_patterns(&self, events: &[BehaviorEvent]) -> Vec<TopicPattern> {
        let mut topic_counts: HashMap<String, TopicInfo> = HashMap::new();

        for event in events {
            let topic = match &event.event_type {
                BehaviorEventType::CodeGeneration { language, .. } => {
                    format!("code:{}", language)
                },
                BehaviorEventType::SearchQuery { query_type, .. } => {
                    format!("search:{}", query_type)
                },
                BehaviorEventType::ConversationStart { intent, .. } => {
                    intent.clone().unwrap_or_else(|| "general".to_string())
                },
                BehaviorEventType::ArtifactCreation { artifact_type, .. } => {
                    format!("artifact:{}", artifact_type)
                },
                _ => return Vec::new(),
            };

            let info = topic_counts.entry(topic).or_insert_with(|| TopicInfo {
                count: 0,
                last_seen: event.timestamp,
            });
            info.count += 1;
            if event.timestamp > info.last_seen {
                info.last_seen = event.timestamp;
            }
        }

        topic_counts
            .into_iter()
            .map(|(topic, info)| TopicPattern {
                topic,
                frequency: info.count,
                recency: info.last_seen,
            })
            .filter(|p| p.frequency >= 2)
            .collect()
    }

    pub fn infer_coding_profile(&self, patterns: &[CodingPatternMatch]) -> CodingStyleProfile {
        let mut profile = CodingStyleProfile::default();

        for pattern in patterns {
            match pattern.pattern_type {
                PatternType::Naming => {
                    if pattern.value.contains("camel") {
                        profile.naming_convention = NamingConvention::CamelCase;
                    } else if pattern.value.contains("snake") {
                        profile.naming_convention = NamingConvention::SnakeCase;
                    } else if pattern.value.contains("pascal") {
                        profile.naming_convention = NamingConvention::PascalCase;
                    } else if pattern.value.contains("kebab") {
                        profile.naming_convention = NamingConvention::KebabCase;
                    }
                },
                PatternType::Indentation => {
                    if pattern.value == "spacious" {
                        profile.indentation_style = IndentationStyle::FourSpaces;
                    } else {
                        profile.indentation_style = IndentationStyle::TwoSpaces;
                    }
                },
                PatternType::Comment => {
                    if pattern.value == "extensive" {
                        profile.comment_style = CommentStyle::Extensive;
                    } else if pattern.value == "minimal" {
                        profile.comment_style = CommentStyle::Minimal;
                    } else {
                        profile.comment_style = CommentStyle::Moderate;
                    }
                },
                _ => {},
            }
        }

        profile.confidence =
            patterns.iter().map(|p| p.confidence).sum::<f32>() / patterns.len().max(1) as f32;

        profile
    }

    pub fn infer_communication_profile(&self, events: &[BehaviorEvent]) -> CommunicationProfile {
        let mut profile = CommunicationProfile::default();

        let mut feedback_positive = 0;
        let mut feedback_negative = 0;

        for event in events {
            if let BehaviorEventType::FeedbackGiven { feedback_type, .. } = &event.event_type {
                match feedback_type {
                    crate::behavior_tracker::UserFeedbackType::Positive => feedback_positive += 1,
                    crate::behavior_tracker::UserFeedbackType::Negative => feedback_negative += 1,
                    _ => {},
                }
            }

            if let BehaviorEventType::SearchQuery { result_count, .. } = &event.event_type {
                if *result_count > 5 {
                    profile.detail_level = DetailLevel::Comprehensive;
                } else if *result_count > 2 {
                    profile.detail_level = DetailLevel::Moderate;
                } else {
                    profile.detail_level = DetailLevel::Minimal;
                }
            }
        }

        if feedback_negative > feedback_positive * 2 {
            profile.tone = Tone::Casual;
        } else if feedback_positive > feedback_negative * 2 {
            profile.tone = Tone::Formal;
        } else {
            profile.tone = Tone::Neutral;
        }

        let total_events = events.len() as f32;
        profile.confidence = if total_events > 10.0 {
            0.7
        } else {
            total_events / 15.0
        };

        profile
    }

    pub fn infer_work_habit_profile(
        &self,
        patterns: &[TemporalPattern],
        tool_patterns: &[ToolPreferencePattern],
    ) -> crate::user_profile::WorkHabitProfile {
        let mut profile = crate::user_profile::WorkHabitProfile::default();

        for pattern in patterns {
            match pattern.pattern_type {
                TemporalPatternType::PeakHours => {
                    profile.active_hours = pattern.time_range.clone();
                },
                TemporalPatternType::PreferredDays => {},
                _ => {},
            }
        }

        for tool_pattern in tool_patterns.iter().take(5) {
            profile.tool_usage_patterns.push(ToolUsagePattern {
                tool_name: tool_pattern.tool_name.clone(),
                usage_count: (tool_pattern.usage_frequency * 100.0) as u32,
                avg_duration_ms: tool_pattern.avg_duration_ms,
                last_used: Utc::now(),
            });
        }

        let total_tool_prefs = tool_patterns.len();
        profile.confidence = if total_tool_prefs > 0 {
            (total_tool_prefs as f32 / 10.0).min(0.8)
        } else {
            0.0
        };

        profile
    }
}

impl Default for PatternAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

struct ToolStats {
    usage_count: u32,
    success_count: u32,
    total_duration_ms: u64,
}

struct TopicInfo {
    count: u32,
    last_seen: DateTime<Utc>,
}
