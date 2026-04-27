//! Scheduled Tasks Module - Timed and automated task execution
//!
//! This module provides infrastructure for scheduling and automating tasks:
//! - Cron-style scheduled tasks
//! - One-time delayed tasks
//! - Recurring task management
//! - Task persistence and recovery

use anyhow::Result;
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task_type: TaskType,
    pub workflow_id: Option<String>,
    pub schedule_config: ScheduleConfig,
    pub next_run_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_result: Option<TaskRunResult>,
    pub status: ScheduledTaskStatus,
    pub config: TaskConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ScheduledTask {
    pub fn new(
        name: String,
        description: String,
        task_type: TaskType,
        next_run_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            task_type,
            workflow_id: None,
            schedule_config: ScheduleConfig::default(),
            next_run_at,
            last_run_at: None,
            last_result: None,
            status: ScheduledTaskStatus::Active,
            config: TaskConfig::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_schedule_config(mut self, config: ScheduleConfig) -> Self {
        self.schedule_config = config;
        self
    }

    pub fn with_workflow_id(mut self, workflow_id: String) -> Self {
        self.workflow_id = Some(workflow_id);
        self
    }

    pub fn calculate_next_run(&mut self) {
        let holidays = ChineseHolidays::new();
        let now = Utc::now();
        let naive_now = now.naive_utc();

        match self.schedule_config.schedule_type {
            ScheduleType::Interval => {
                if let Some(interval) = self.schedule_config.interval_seconds {
                    self.next_run_at = self.last_run_at.unwrap_or(now)
                        + chrono::Duration::seconds(interval as i64);
                }
            }
            ScheduleType::Daily
            | ScheduleType::Weekly
            | ScheduleType::Monthly
            | ScheduleType::Advanced => {
                let mut next = naive_now + chrono::Duration::days(1);
                let mut attempts = 0;

                while attempts < 366 {
                    attempts += 1;

                    if self.schedule_config.exclude_holidays && holidays.is_holiday(&next.date()) {
                        next += chrono::Duration::days(1);
                        continue;
                    }

                    if holidays
                        .is_excluded_date(&next.date(), &self.schedule_config.exclude_custom_dates)
                    {
                        next += chrono::Duration::days(1);
                        continue;
                    }

                    let weekday = Weekday::fromchrono(&next);

                    let weekday_match = self.schedule_config.schedule_type != ScheduleType::Daily
                        && self.schedule_config.schedule_type != ScheduleType::Monthly
                        && self.schedule_config.weekdays.is_empty()
                        || self.schedule_config.weekdays.contains(&weekday);

                    let month_day_match =
                        if self.schedule_config.schedule_type == ScheduleType::Monthly {
                            if let Some(day) = self.schedule_config.month_day {
                                next.day() == day
                            } else {
                                true
                            }
                        } else {
                            true
                        };

                    if !weekday_match || !month_day_match {
                        next += chrono::Duration::days(1);
                        continue;
                    }

                    if let Some(first_range) = self.schedule_config.time_ranges.first() {
                        next = next
                            .date()
                            .and_hms_opt(first_range.start_hour, first_range.start_minute, 0)
                            .unwrap();
                        self.next_run_at = DateTime::from_naive_utc_and_offset(next, Utc);
                    }
                    break;
                }
            }
        }
    }

    pub fn update_last_run(&mut self, result: TaskRunResult) {
        self.last_run_at = Some(Utc::now());
        self.last_result = Some(result);
        self.calculate_next_run();
        self.updated_at = Utc::now();
    }

    pub fn is_due(&self) -> bool {
        Utc::now() >= self.next_run_at
    }

    pub fn pause(&mut self) {
        self.status = ScheduledTaskStatus::Paused;
        self.updated_at = Utc::now();
    }

    pub fn resume(&mut self) {
        self.status = ScheduledTaskStatus::Active;
        self.calculate_next_run();
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    DailySummary,
    Backup,
    Cleanup,
    Custom,
    HealthCheck,
    DataSync,
    Workflow,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::DailySummary => "daily_summary",
            TaskType::Backup => "backup",
            TaskType::Cleanup => "cleanup",
            TaskType::Custom => "custom",
            TaskType::HealthCheck => "health_check",
            TaskType::DataSync => "data_sync",
            TaskType::Workflow => "workflow",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScheduledTaskStatus {
    Active,
    Paused,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub timeout_seconds: u64,
    pub retry_on_failure: bool,
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
    pub notification_enabled: bool,
    pub run_on_startup: bool,
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 3600,
            retry_on_failure: true,
            max_retries: 3,
            retry_delay_seconds: 300,
            notification_enabled: true,
            run_on_startup: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub executed_at: DateTime<Utc>,
}

impl TaskRunResult {
    pub fn success(output: String, duration_ms: u64) -> Self {
        Self {
            success: true,
            output: Some(output),
            error: None,
            duration_ms,
            executed_at: Utc::now(),
        }
    }

    pub fn failure(error: String, duration_ms: u64) -> Self {
        Self {
            success: false,
            output: None,
            error: Some(error),
            duration_ms,
            executed_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    pub name: String,
    pub task_type: TaskType,
    pub prompt_template: String,
    pub parameters: HashMap<String, String>,
}

impl TaskDefinition {
    pub fn new(name: String, task_type: TaskType, prompt_template: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            task_type,
            prompt_template,
            parameters: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: String, value: String) -> Self {
        self.parameters.insert(key, value);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskTemplate {
    DailySummary,
    WeeklySummary,
    ConversationStats,
    KnowledgeGraphUpdate,
    InsightReport,
    CustomReport,
    WorkflowCodeReview,
    WorkflowBugFix,
    WorkflowDocGen,
    WorkflowTestGen,
    WorkflowRefactor,
    WorkflowExplore,
    WorkflowPerformance,
    WorkflowKnowledgeExtract,
    WorkflowKnowledgeToCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplateInfo {
    pub template_type: TaskTemplate,
    pub name: String,
    pub description: String,
    pub schedule_config: ScheduleConfig,
    pub workflow_id: Option<String>,
}

impl TaskTemplate {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskTemplate::DailySummary => "daily_summary",
            TaskTemplate::WeeklySummary => "weekly_summary",
            TaskTemplate::ConversationStats => "conversation_stats",
            TaskTemplate::KnowledgeGraphUpdate => "knowledge_graph_update",
            TaskTemplate::InsightReport => "insight_report",
            TaskTemplate::CustomReport => "custom_report",
            TaskTemplate::WorkflowCodeReview => "workflow_code_review",
            TaskTemplate::WorkflowBugFix => "workflow_bug_fix",
            TaskTemplate::WorkflowDocGen => "workflow_doc_gen",
            TaskTemplate::WorkflowTestGen => "workflow_test_gen",
            TaskTemplate::WorkflowRefactor => "workflow_refactor",
            TaskTemplate::WorkflowExplore => "workflow_explore",
            TaskTemplate::WorkflowPerformance => "workflow_performance",
            TaskTemplate::WorkflowKnowledgeExtract => "workflow_knowledge_extract",
            TaskTemplate::WorkflowKnowledgeToCode => "workflow_knowledge_to_code",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TaskTemplate::DailySummary => "Daily Summary Report",
            TaskTemplate::WeeklySummary => "Weekly Summary Report",
            TaskTemplate::ConversationStats => "Conversation Statistics",
            TaskTemplate::KnowledgeGraphUpdate => "Knowledge Graph Update",
            TaskTemplate::InsightReport => "Insight Report",
            TaskTemplate::CustomReport => "Custom Report",
            TaskTemplate::WorkflowCodeReview => "Code Review Workflow",
            TaskTemplate::WorkflowBugFix => "Bug Fix Workflow",
            TaskTemplate::WorkflowDocGen => "Documentation Workflow",
            TaskTemplate::WorkflowTestGen => "Test Generation Workflow",
            TaskTemplate::WorkflowRefactor => "Refactoring Workflow",
            TaskTemplate::WorkflowExplore => "Code Exploration Workflow",
            TaskTemplate::WorkflowPerformance => "Performance Optimization Workflow",
            TaskTemplate::WorkflowKnowledgeExtract => "Knowledge Extraction Workflow",
            TaskTemplate::WorkflowKnowledgeToCode => "Knowledge to Code Workflow",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TaskTemplate::DailySummary => {
                "Generate a daily summary of conversations and activities"
            }
            TaskTemplate::WeeklySummary => "Generate a weekly summary report",
            TaskTemplate::ConversationStats => "Analyze conversation patterns and statistics",
            TaskTemplate::KnowledgeGraphUpdate => "Update and optimize the knowledge graph",
            TaskTemplate::InsightReport => "Generate insights from recent activities",
            TaskTemplate::CustomReport => "Create a custom report based on your needs",
            TaskTemplate::WorkflowCodeReview => "Automated code review with multi-agent analysis",
            TaskTemplate::WorkflowBugFix => "Systematic bug diagnosis and fix workflow",
            TaskTemplate::WorkflowDocGen => "Generate comprehensive documentation",
            TaskTemplate::WorkflowTestGen => "Generate complete test suites",
            TaskTemplate::WorkflowRefactor => "Safe refactoring with behavior preservation",
            TaskTemplate::WorkflowExplore => "Explore and understand codebase",
            TaskTemplate::WorkflowPerformance => "Performance analysis and optimization",
            TaskTemplate::WorkflowKnowledgeExtract => "Extract domain knowledge from code",
            TaskTemplate::WorkflowKnowledgeToCode => "Generate code from knowledge base",
        }
    }

    pub fn workflow_id(&self) -> Option<&'static str> {
        match self {
            TaskTemplate::WorkflowCodeReview => Some("code-review"),
            TaskTemplate::WorkflowBugFix => Some("bug-fix"),
            TaskTemplate::WorkflowDocGen => Some("doc-gen"),
            TaskTemplate::WorkflowTestGen => Some("test-gen"),
            TaskTemplate::WorkflowRefactor => Some("refactor"),
            TaskTemplate::WorkflowExplore => Some("explore"),
            TaskTemplate::WorkflowPerformance => Some("performance"),
            TaskTemplate::WorkflowKnowledgeExtract => Some("knowledge-extract"),
            TaskTemplate::WorkflowKnowledgeToCode => Some("knowledge-to-code"),
            _ => None,
        }
    }

    pub fn is_workflow(&self) -> bool {
        self.workflow_id().is_some()
    }

    pub fn default_schedule(&self) -> ScheduleConfig {
        match self {
            TaskTemplate::DailySummary => ScheduleConfig {
                schedule_type: ScheduleType::Daily,
                weekdays: vec![
                    Weekday::Monday,
                    Weekday::Tuesday,
                    Weekday::Wednesday,
                    Weekday::Thursday,
                    Weekday::Friday,
                ],
                time_ranges: vec![TimeRange::new(18, 0, 18, 30)],
                interval_seconds: None,
                exclude_holidays: true,
                exclude_custom_dates: vec![],
                month_day: None,
            },
            TaskTemplate::WeeklySummary => ScheduleConfig {
                schedule_type: ScheduleType::Weekly,
                weekdays: vec![Weekday::Friday],
                time_ranges: vec![TimeRange::new(17, 0, 18, 0)],
                interval_seconds: None,
                exclude_holidays: true,
                exclude_custom_dates: vec![],
                month_day: None,
            },
            TaskTemplate::ConversationStats => ScheduleConfig {
                schedule_type: ScheduleType::Daily,
                weekdays: vec![
                    Weekday::Monday,
                    Weekday::Tuesday,
                    Weekday::Wednesday,
                    Weekday::Thursday,
                    Weekday::Friday,
                ],
                time_ranges: vec![TimeRange::new(8, 0, 9, 0)],
                interval_seconds: None,
                exclude_holidays: true,
                exclude_custom_dates: vec![],
                month_day: None,
            },
            TaskTemplate::KnowledgeGraphUpdate => ScheduleConfig {
                schedule_type: ScheduleType::Daily,
                weekdays: vec![Weekday::Monday, Weekday::Wednesday, Weekday::Friday],
                time_ranges: vec![TimeRange::new(2, 0, 4, 0)],
                interval_seconds: None,
                exclude_holidays: false,
                exclude_custom_dates: vec![],
                month_day: None,
            },
            TaskTemplate::InsightReport => ScheduleConfig {
                schedule_type: ScheduleType::Weekly,
                weekdays: vec![Weekday::Monday],
                time_ranges: vec![TimeRange::new(9, 0, 10, 0)],
                interval_seconds: None,
                exclude_holidays: true,
                exclude_custom_dates: vec![],
                month_day: None,
            },
            TaskTemplate::CustomReport => ScheduleConfig::default(),
            TaskTemplate::WorkflowCodeReview => ScheduleConfig {
                schedule_type: ScheduleType::Weekly,
                weekdays: vec![Weekday::Monday],
                time_ranges: vec![TimeRange::new(14, 0, 17, 0)],
                interval_seconds: None,
                exclude_holidays: true,
                exclude_custom_dates: vec![],
                month_day: None,
            },
            TaskTemplate::WorkflowBugFix => ScheduleConfig {
                schedule_type: ScheduleType::Weekly,
                weekdays: vec![Weekday::Wednesday],
                time_ranges: vec![TimeRange::new(9, 0, 12, 0)],
                interval_seconds: None,
                exclude_holidays: true,
                exclude_custom_dates: vec![],
                month_day: None,
            },
            TaskTemplate::WorkflowDocGen => ScheduleConfig {
                schedule_type: ScheduleType::Weekly,
                weekdays: vec![Weekday::Friday],
                time_ranges: vec![TimeRange::new(15, 0, 17, 0)],
                interval_seconds: None,
                exclude_holidays: true,
                exclude_custom_dates: vec![],
                month_day: None,
            },
            TaskTemplate::WorkflowTestGen => ScheduleConfig {
                schedule_type: ScheduleType::Weekly,
                weekdays: vec![Weekday::Thursday],
                time_ranges: vec![TimeRange::new(14, 0, 17, 0)],
                interval_seconds: None,
                exclude_holidays: true,
                exclude_custom_dates: vec![],
                month_day: None,
            },
            TaskTemplate::WorkflowRefactor => ScheduleConfig {
                schedule_type: ScheduleType::Monthly,
                weekdays: vec![],
                time_ranges: vec![TimeRange::new(18, 0, 20, 0)],
                interval_seconds: None,
                exclude_holidays: true,
                exclude_custom_dates: vec![],
                month_day: Some(15),
            },
            TaskTemplate::WorkflowExplore => ScheduleConfig {
                schedule_type: ScheduleType::Daily,
                weekdays: vec![Weekday::Monday, Weekday::Wednesday, Weekday::Friday],
                time_ranges: vec![TimeRange::new(8, 0, 9, 0)],
                interval_seconds: None,
                exclude_holidays: false,
                exclude_custom_dates: vec![],
                month_day: None,
            },
            TaskTemplate::WorkflowPerformance => ScheduleConfig {
                schedule_type: ScheduleType::Weekly,
                weekdays: vec![Weekday::Tuesday],
                time_ranges: vec![TimeRange::new(2, 0, 5, 0)],
                interval_seconds: None,
                exclude_holidays: false,
                exclude_custom_dates: vec![],
                month_day: None,
            },
            TaskTemplate::WorkflowKnowledgeExtract => ScheduleConfig {
                schedule_type: ScheduleType::Weekly,
                weekdays: vec![Weekday::Saturday],
                time_ranges: vec![TimeRange::new(10, 0, 16, 0)],
                interval_seconds: None,
                exclude_holidays: false,
                exclude_custom_dates: vec![],
                month_day: None,
            },
            TaskTemplate::WorkflowKnowledgeToCode => ScheduleConfig {
                schedule_type: ScheduleType::Monthly,
                weekdays: vec![],
                time_ranges: vec![TimeRange::new(9, 0, 17, 0)],
                interval_seconds: None,
                exclude_holidays: true,
                exclude_custom_dates: vec![],
                month_day: Some(1),
            },
        }
    }

    pub fn all_templates() -> Vec<TaskTemplateInfo> {
        vec![
            TaskTemplateInfo {
                template_type: TaskTemplate::DailySummary,
                name: TaskTemplate::DailySummary.display_name().to_string(),
                description: TaskTemplate::DailySummary.description().to_string(),
                schedule_config: TaskTemplate::DailySummary.default_schedule(),
                workflow_id: None,
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::WeeklySummary,
                name: TaskTemplate::WeeklySummary.display_name().to_string(),
                description: TaskTemplate::WeeklySummary.description().to_string(),
                schedule_config: TaskTemplate::WeeklySummary.default_schedule(),
                workflow_id: None,
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::ConversationStats,
                name: TaskTemplate::ConversationStats.display_name().to_string(),
                description: TaskTemplate::ConversationStats.description().to_string(),
                schedule_config: TaskTemplate::ConversationStats.default_schedule(),
                workflow_id: None,
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::KnowledgeGraphUpdate,
                name: TaskTemplate::KnowledgeGraphUpdate
                    .display_name()
                    .to_string(),
                description: TaskTemplate::KnowledgeGraphUpdate.description().to_string(),
                schedule_config: TaskTemplate::KnowledgeGraphUpdate.default_schedule(),
                workflow_id: None,
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::InsightReport,
                name: TaskTemplate::InsightReport.display_name().to_string(),
                description: TaskTemplate::InsightReport.description().to_string(),
                schedule_config: TaskTemplate::InsightReport.default_schedule(),
                workflow_id: None,
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::CustomReport,
                name: TaskTemplate::CustomReport.display_name().to_string(),
                description: TaskTemplate::CustomReport.description().to_string(),
                schedule_config: TaskTemplate::CustomReport.default_schedule(),
                workflow_id: None,
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::WorkflowCodeReview,
                name: TaskTemplate::WorkflowCodeReview.display_name().to_string(),
                description: TaskTemplate::WorkflowCodeReview.description().to_string(),
                schedule_config: TaskTemplate::WorkflowCodeReview.default_schedule(),
                workflow_id: TaskTemplate::WorkflowCodeReview
                    .workflow_id()
                    .map(String::from),
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::WorkflowBugFix,
                name: TaskTemplate::WorkflowBugFix.display_name().to_string(),
                description: TaskTemplate::WorkflowBugFix.description().to_string(),
                schedule_config: TaskTemplate::WorkflowBugFix.default_schedule(),
                workflow_id: TaskTemplate::WorkflowBugFix.workflow_id().map(String::from),
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::WorkflowDocGen,
                name: TaskTemplate::WorkflowDocGen.display_name().to_string(),
                description: TaskTemplate::WorkflowDocGen.description().to_string(),
                schedule_config: TaskTemplate::WorkflowDocGen.default_schedule(),
                workflow_id: TaskTemplate::WorkflowDocGen.workflow_id().map(String::from),
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::WorkflowTestGen,
                name: TaskTemplate::WorkflowTestGen.display_name().to_string(),
                description: TaskTemplate::WorkflowTestGen.description().to_string(),
                schedule_config: TaskTemplate::WorkflowTestGen.default_schedule(),
                workflow_id: TaskTemplate::WorkflowTestGen
                    .workflow_id()
                    .map(String::from),
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::WorkflowRefactor,
                name: TaskTemplate::WorkflowRefactor.display_name().to_string(),
                description: TaskTemplate::WorkflowRefactor.description().to_string(),
                schedule_config: TaskTemplate::WorkflowRefactor.default_schedule(),
                workflow_id: TaskTemplate::WorkflowRefactor
                    .workflow_id()
                    .map(String::from),
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::WorkflowExplore,
                name: TaskTemplate::WorkflowExplore.display_name().to_string(),
                description: TaskTemplate::WorkflowExplore.description().to_string(),
                schedule_config: TaskTemplate::WorkflowExplore.default_schedule(),
                workflow_id: TaskTemplate::WorkflowExplore
                    .workflow_id()
                    .map(String::from),
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::WorkflowPerformance,
                name: TaskTemplate::WorkflowPerformance.display_name().to_string(),
                description: TaskTemplate::WorkflowPerformance.description().to_string(),
                schedule_config: TaskTemplate::WorkflowPerformance.default_schedule(),
                workflow_id: TaskTemplate::WorkflowPerformance
                    .workflow_id()
                    .map(String::from),
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::WorkflowKnowledgeExtract,
                name: TaskTemplate::WorkflowKnowledgeExtract
                    .display_name()
                    .to_string(),
                description: TaskTemplate::WorkflowKnowledgeExtract
                    .description()
                    .to_string(),
                schedule_config: TaskTemplate::WorkflowKnowledgeExtract.default_schedule(),
                workflow_id: TaskTemplate::WorkflowKnowledgeExtract
                    .workflow_id()
                    .map(String::from),
            },
            TaskTemplateInfo {
                template_type: TaskTemplate::WorkflowKnowledgeToCode,
                name: TaskTemplate::WorkflowKnowledgeToCode
                    .display_name()
                    .to_string(),
                description: TaskTemplate::WorkflowKnowledgeToCode
                    .description()
                    .to_string(),
                schedule_config: TaskTemplate::WorkflowKnowledgeToCode.default_schedule(),
                workflow_id: TaskTemplate::WorkflowKnowledgeToCode
                    .workflow_id()
                    .map(String::from),
            },
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Monday = 0,
    Tuesday = 1,
    Wednesday = 2,
    Thursday = 3,
    Friday = 4,
    Saturday = 5,
    Sunday = 6,
}

impl Weekday {
    pub fn fromchrono(dt: &chrono::NaiveDateTime) -> Self {
        match dt.weekday().num_days_from_monday() {
            0 => Weekday::Monday,
            1 => Weekday::Tuesday,
            2 => Weekday::Wednesday,
            3 => Weekday::Thursday,
            4 => Weekday::Friday,
            5 => Weekday::Saturday,
            6 => Weekday::Sunday,
            _ => Weekday::Monday,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Weekday::Monday => "monday",
            Weekday::Tuesday => "tuesday",
            Weekday::Wednesday => "wednesday",
            Weekday::Thursday => "thursday",
            Weekday::Friday => "friday",
            Weekday::Saturday => "saturday",
            Weekday::Sunday => "sunday",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleType {
    Interval,
    Daily,
    Weekly,
    Monthly,
    Advanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start_hour: u32,
    pub start_minute: u32,
    pub end_hour: u32,
    pub end_minute: u32,
}

impl TimeRange {
    pub fn new(start_hour: u32, start_minute: u32, end_hour: u32, end_minute: u32) -> Self {
        Self {
            start_hour,
            start_minute,
            end_hour,
            end_minute,
        }
    }

    pub fn to_minutes(&self) -> (u32, u32) {
        (
            self.start_hour * 60 + self.start_minute,
            self.end_hour * 60 + self.end_minute,
        )
    }

    pub fn contains_time(&self, hour: u32, minute: u32) -> bool {
        let time = hour * 60 + minute;
        let (start, end) = self.to_minutes();
        time >= start && time <= end
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub schedule_type: ScheduleType,
    pub weekdays: Vec<Weekday>,
    pub time_ranges: Vec<TimeRange>,
    pub interval_seconds: Option<u64>,
    pub exclude_holidays: bool,
    pub exclude_custom_dates: Vec<String>,
    pub month_day: Option<u32>,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            schedule_type: ScheduleType::Daily,
            weekdays: vec![
                Weekday::Monday,
                Weekday::Tuesday,
                Weekday::Wednesday,
                Weekday::Thursday,
                Weekday::Friday,
            ],
            time_ranges: vec![TimeRange::new(9, 0, 17, 0)],
            interval_seconds: None,
            exclude_holidays: false,
            exclude_custom_dates: vec![],
            month_day: None,
        }
    }
}

impl ScheduleConfig {
    pub fn interval(hours: u64) -> Self {
        Self {
            schedule_type: ScheduleType::Interval,
            weekdays: vec![],
            time_ranges: vec![],
            interval_seconds: Some(hours * 3600),
            exclude_holidays: false,
            exclude_custom_dates: vec![],
            month_day: None,
        }
    }

    pub fn daily_at(hour: u32, minute: u32) -> Self {
        Self {
            schedule_type: ScheduleType::Daily,
            weekdays: vec![
                Weekday::Monday,
                Weekday::Tuesday,
                Weekday::Wednesday,
                Weekday::Thursday,
                Weekday::Friday,
            ],
            time_ranges: vec![TimeRange::new(hour, minute, hour, minute)],
            interval_seconds: None,
            exclude_holidays: false,
            exclude_custom_dates: vec![],
            month_day: None,
        }
    }

    pub fn weekdays_only() -> Self {
        Self {
            schedule_type: ScheduleType::Weekly,
            weekdays: vec![
                Weekday::Monday,
                Weekday::Tuesday,
                Weekday::Wednesday,
                Weekday::Thursday,
                Weekday::Friday,
            ],
            time_ranges: vec![TimeRange::new(9, 0, 17, 0)],
            interval_seconds: None,
            exclude_holidays: false,
            exclude_custom_dates: vec![],
            month_day: None,
        }
    }
}

pub struct ChineseHolidays {
    holidays: std::collections::HashSet<String>,
    workdays: std::collections::HashSet<String>,
}

impl ChineseHolidays {
    pub fn new() -> Self {
        let mut holidays = std::collections::HashSet::new();
        holidays.insert("2026-01-01".to_string());
        holidays.insert("2026-01-28".to_string());
        holidays.insert("2026-01-29".to_string());
        holidays.insert("2026-01-30".to_string());
        holidays.insert("2026-01-31".to_string());
        holidays.insert("2026-02-01".to_string());
        holidays.insert("2026-02-02".to_string());
        holidays.insert("2026-02-03".to_string());
        holidays.insert("2026-02-04".to_string());
        holidays.insert("2026-02-05".to_string());
        holidays.insert("2026-02-06".to_string());
        holidays.insert("2026-02-07".to_string());
        holidays.insert("2026-02-08".to_string());
        holidays.insert("2026-04-03".to_string());
        holidays.insert("2026-04-04".to_string());
        holidays.insert("2026-04-05".to_string());
        holidays.insert("2026-04-06".to_string());
        holidays.insert("2026-05-01".to_string());
        holidays.insert("2026-05-02".to_string());
        holidays.insert("2026-05-03".to_string());
        holidays.insert("2026-05-04".to_string());
        holidays.insert("2026-05-05".to_string());
        holidays.insert("2026-06-01".to_string());
        holidays.insert("2026-06-02".to_string());
        holidays.insert("2026-06-03".to_string());
        holidays.insert("2026-06-04".to_string());
        holidays.insert("2026-06-05".to_string());
        holidays.insert("2026-06-06".to_string());
        holidays.insert("2026-09-24".to_string());
        holidays.insert("2026-09-25".to_string());
        holidays.insert("2026-09-26".to_string());
        holidays.insert("2026-09-27".to_string());
        holidays.insert("2026-09-28".to_string());
        holidays.insert("2026-10-01".to_string());
        holidays.insert("2026-10-02".to_string());
        holidays.insert("2026-10-03".to_string());
        holidays.insert("2026-10-04".to_string());
        holidays.insert("2026-10-05".to_string());
        holidays.insert("2026-10-06".to_string());
        holidays.insert("2026-10-07".to_string());
        holidays.insert("2026-10-08".to_string());
        holidays.insert("2026-10-09".to_string());
        holidays.insert("2026-10-10".to_string());

        let mut workdays = std::collections::HashSet::new();
        workdays.insert("2026-02-07".to_string());
        workdays.insert("2026-02-08".to_string());
        workdays.insert("2026-02-28".to_string());
        workdays.insert("2026-02-29".to_string());
        workdays.insert("2026-04-06".to_string());
        workdays.insert("2026-04-26".to_string());
        workdays.insert("2026-05-09".to_string());
        workdays.insert("2026-06-06".to_string());
        workdays.insert("2026-09-27".to_string());
        workdays.insert("2026-09-28".to_string());
        workdays.insert("2026-10-10".to_string());
        workdays.insert("2026-10-11".to_string());

        Self { holidays, workdays }
    }

    pub fn is_holiday(&self, date: &chrono::NaiveDate) -> bool {
        let date_str = date.format("%Y-%m-%d").to_string();
        if self.workdays.contains(&date_str) {
            return false;
        }
        self.holidays.contains(&date_str)
            || date.weekday() == chrono::Weekday::Sat
            || date.weekday() == chrono::Weekday::Sun
    }

    pub fn is_workday(&self, date: &chrono::NaiveDate) -> bool {
        !self.is_holiday(date)
    }

    pub fn is_excluded_date(&self, date: &chrono::NaiveDate, exclude_dates: &[String]) -> bool {
        let date_str = date.format("%Y-%m-%d").to_string();
        exclude_dates.iter().any(|d| d == &date_str)
    }
}

impl Default for ChineseHolidays {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ScheduledTaskService {
    tasks: Arc<RwLock<HashMap<String, ScheduledTask>>>,
    task_definitions: Arc<RwLock<HashMap<String, TaskDefinition>>>,
    execution_history: Arc<RwLock<Vec<TaskRunResult>>>,
    max_history_size: usize,
    db_path: Option<std::path::PathBuf>,
}

impl Default for ScheduledTaskService {
    fn default() -> Self {
        Self::new(100)
    }
}

impl ScheduledTaskService {
    pub fn new(max_history_size: usize) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            task_definitions: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
            max_history_size,
            db_path: None,
        }
    }

    pub fn new_with_db(max_history_size: usize, db_path: std::path::PathBuf) -> Self {
        let service = Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            task_definitions: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
            max_history_size,
            db_path: Some(db_path),
        };
        service.initialize_db();
        service.load_from_db();
        service
    }

    fn initialize_db(&self) {
        if let Some(ref db_path) = self.db_path {
            if let Some(parent) = db_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(conn) = rusqlite::Connection::open(db_path) {
                let _ = conn.execute(
                    "CREATE TABLE IF NOT EXISTS scheduled_tasks (
                        id TEXT PRIMARY KEY,
                        name TEXT NOT NULL,
                        description TEXT NOT NULL,
                        task_type TEXT NOT NULL,
                        cron_expression TEXT,
                        interval_seconds INTEGER,
                        next_run_at TEXT NOT NULL,
                        last_run_at TEXT,
                        last_result TEXT,
                        status TEXT NOT NULL,
                        config TEXT NOT NULL,
                        created_at TEXT NOT NULL,
                        updated_at TEXT NOT NULL
                    )",
                    [],
                );
            }
        }
    }

    fn load_from_db(&self) {
        if let Some(ref db_path) = self.db_path {
            if let Ok(conn) = rusqlite::Connection::open(db_path) {
                let mut tasks = self.tasks.write().unwrap();
                if let Ok(mut rows) = conn.prepare(
                    "SELECT id, name, description, task_type, cron_expression, interval_seconds,
                            next_run_at, last_run_at, last_result, status, config, created_at, updated_at
                     FROM scheduled_tasks"
                ) {
                    let task_results = rows.query_map([], |row| {
                        let task_type_str: String = row.get(3)?;
                        let status_str: String = row.get(9)?;
                        let config_str: String = row.get(10)?;
                        let last_result_str: Option<String> = row.get(8)?;

                        let task_type = match task_type_str.as_str() {
                            "daily_summary" => TaskType::DailySummary,
                            "backup" => TaskType::Backup,
                            "cleanup" => TaskType::Cleanup,
                            "custom" => TaskType::Custom,
                            "health_check" => TaskType::HealthCheck,
                            "data_sync" => TaskType::DataSync,
                            _ => TaskType::Custom,
                        };

                        let status = match status_str.as_str() {
                            "active" => ScheduledTaskStatus::Active,
                            "paused" => ScheduledTaskStatus::Paused,
                            "disabled" => ScheduledTaskStatus::Disabled,
                            _ => ScheduledTaskStatus::Active,
                        };

                        let config: TaskConfig = serde_json::from_str(&config_str)
                            .unwrap_or_default();

                        let last_result: Option<TaskRunResult> = last_result_str
                            .and_then(|s| serde_json::from_str(&s).ok());

                        let schedule_config_str: String = row.get(4)?;
                        let schedule_config: ScheduleConfig = serde_json::from_str(&schedule_config_str)
                            .unwrap_or_default();

                        Ok(ScheduledTask {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            description: row.get(2)?,
                            task_type,
                            workflow_id: row.get(3)?,
                            schedule_config,
                            next_run_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .unwrap_or_else(|_| chrono::Utc::now()),
                            last_run_at: row.get::<_, Option<String>>(7)?
                                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                                .map(|dt| dt.with_timezone(&chrono::Utc)),
                            last_result,
                            status,
                            config,
                            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .unwrap_or_else(|_| chrono::Utc::now()),
                            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .unwrap_or_else(|_| chrono::Utc::now()),
                        })
                    });

                    if let Ok(task_iter) = task_results {
                        for task in task_iter.flatten() {
                            tasks.insert(task.id.clone(), task);
                        }
                    }
                }
            }
        }
    }

    pub async fn create_task(&self, mut task: ScheduledTask) -> Result<String> {
        task.calculate_next_run();
        let task_id = task.id.clone();
        let mut tasks = self.tasks.write().unwrap();
        tasks.insert(task_id.clone(), task);
        Ok(task_id)
    }

    pub async fn add_task(&self, task: ScheduledTask) -> Result<String> {
        let task_id = task.id.clone();
        let mut tasks = self.tasks.write().unwrap();
        tasks.insert(task_id.clone(), task);
        Ok(task_id)
    }

    pub async fn create_daily_summary_task(
        &self,
        name: String,
        description: String,
        hour: u32,
        minute: u32,
    ) -> Result<String> {
        let tomorrow = (Utc::now().date_naive() + chrono::Days::new(1))
            .and_hms_opt(hour, minute, 0)
            .unwrap();
        let next_run = DateTime::<Utc>::from_naive_utc_and_offset(tomorrow, Utc);

        let task = ScheduledTask::new(name, description, TaskType::DailySummary, next_run);

        self.create_task(task).await
    }

    pub async fn create_backup_task(
        &self,
        name: String,
        description: String,
        interval_hours: u64,
    ) -> Result<String> {
        let next_run = Utc::now() + chrono::Duration::hours(interval_hours as i64);

        let task = ScheduledTask::new(name, description, TaskType::Backup, next_run)
            .with_schedule_config(ScheduleConfig::interval(interval_hours));

        self.create_task(task).await
    }

    pub async fn create_cleanup_task(
        &self,
        name: String,
        description: String,
        interval_hours: u64,
    ) -> Result<String> {
        let next_run = Utc::now() + chrono::Duration::hours(interval_hours as i64);

        let task = ScheduledTask::new(name, description, TaskType::Cleanup, next_run)
            .with_schedule_config(ScheduleConfig::interval(interval_hours));

        self.create_task(task).await
    }

    pub async fn get_task(&self, id: &str) -> Option<ScheduledTask> {
        let tasks = self.tasks.read().unwrap();
        tasks.get(id).cloned()
    }

    pub async fn list_tasks(&self) -> Vec<ScheduledTask> {
        let tasks = self.tasks.read().unwrap();
        tasks.values().cloned().collect()
    }

    pub async fn list_due_tasks(&self) -> Vec<ScheduledTask> {
        let tasks = self.tasks.read().unwrap();
        tasks
            .values()
            .filter(|t| t.status == ScheduledTaskStatus::Active && t.is_due())
            .cloned()
            .collect()
    }

    pub async fn update_task(&self, id: &str, mut task: ScheduledTask) -> Option<()> {
        task.updated_at = Utc::now();
        let mut tasks = self.tasks.write().unwrap();
        if tasks.contains_key(id) {
            tasks.insert(id.to_string(), task);
            Some(())
        } else {
            None
        }
    }

    pub async fn delete_task(&self, id: &str) -> bool {
        let mut tasks = self.tasks.write().unwrap();
        tasks.remove(id).is_some()
    }

    pub async fn pause_task(&self, id: &str) -> Option<()> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.pause();
            Some(())
        } else {
            None
        }
    }

    pub async fn resume_task(&self, id: &str) -> Option<()> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.resume();
            Some(())
        } else {
            None
        }
    }

    pub async fn record_execution(&self, task_id: &str, result: TaskRunResult) {
        {
            let mut tasks = self.tasks.write().unwrap();
            if let Some(task) = tasks.get_mut(task_id) {
                task.update_last_run(result.clone());
            }
        }

        {
            let mut history = self.execution_history.write().unwrap();
            history.push(result);
            if history.len() > self.max_history_size {
                let drain_count = history.len() - self.max_history_size;
                history.drain(0..drain_count);
            }
        }
    }

    pub async fn get_execution_history(&self, limit: Option<usize>) -> Vec<TaskRunResult> {
        let history = self.execution_history.read().unwrap();
        let limit = limit.unwrap_or(self.max_history_size);
        history.iter().rev().take(limit).cloned().collect()
    }

    pub async fn register_task_definition(&self, definition: TaskDefinition) {
        let mut defs = self.task_definitions.write().unwrap();
        defs.insert(definition.id.clone(), definition);
    }

    pub async fn get_task_definition(&self, id: &str) -> Option<TaskDefinition> {
        let defs = self.task_definitions.read().unwrap();
        defs.get(id).cloned()
    }

    pub async fn get_task_definition_by_type(&self, task_type: TaskType) -> Option<TaskDefinition> {
        let defs = self.task_definitions.read().unwrap();
        defs.values().find(|d| d.task_type == task_type).cloned()
    }

    pub async fn list_task_definitions(&self) -> Vec<TaskDefinition> {
        let defs = self.task_definitions.read().unwrap();
        defs.values().cloned().collect()
    }

    pub async fn execute_task(&self, task_id: &str) -> Option<TaskRunResult> {
        let task = {
            let mut tasks = self.tasks.write().unwrap();
            tasks.get_mut(task_id)?.clone()
        };

        let start = std::time::Instant::now();
        let task_type = task.task_type;
        let name = task.name.clone();
        let _description = task.description.clone();

        let definition = self.get_task_definition_by_type(task_type).await;

        let result = match definition {
            Some(def) => {
                tracing::info!(
                    "[scheduled_task] Executing task '{}' (type: {:?}) with prompt: {}",
                    name,
                    task_type,
                    def.prompt_template.chars().take(100).collect::<String>()
                );
                TaskRunResult::success(
                    format!(
                        "Task '{}' executed successfully. Prompt: {}",
                        name, def.prompt_template
                    ),
                    start.elapsed().as_millis() as u64,
                )
            }
            None => {
                tracing::info!(
                    "[scheduled_task] Executing task '{}' (type: {:?}) - no definition found",
                    name,
                    task_type
                );
                TaskRunResult::success(
                    format!("Task '{}' executed (no prompt template defined)", name),
                    start.elapsed().as_millis() as u64,
                )
            }
        };

        {
            let mut tasks = self.tasks.write().unwrap();
            if let Some(t) = tasks.get_mut(task_id) {
                t.update_last_run(result.clone());
            }
        }

        {
            let mut history = self.execution_history.write().unwrap();
            history.push(result.clone());
            if history.len() > self.max_history_size {
                history.remove(0);
            }
        }

        Some(result)
    }

    pub async fn get_next_scheduled_time(&self) -> Option<DateTime<Utc>> {
        let tasks = self.tasks.read().unwrap();
        tasks
            .values()
            .filter(|t| t.status == ScheduledTaskStatus::Active)
            .map(|t| t.next_run_at)
            .min()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummaryConfig {
    pub hour: u32,
    pub minute: u32,
    pub include_trajectories: bool,
    pub include_skills: bool,
    pub include_insights: bool,
    pub include_user_profile: bool,
    pub format: SummaryFormat,
}

impl Default for DailySummaryConfig {
    fn default() -> Self {
        Self {
            hour: 9,
            minute: 0,
            include_trajectories: true,
            include_skills: true,
            include_insights: true,
            include_user_profile: true,
            format: SummaryFormat::Markdown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SummaryFormat {
    Markdown,
    Json,
    PlainText,
}
