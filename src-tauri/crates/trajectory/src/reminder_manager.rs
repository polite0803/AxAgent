use crate::proactive_assistant::{
    ProactiveAssistant, RecurrenceFrequency, Reminder, ReminderRecurrence,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderSchedule {
    pub reminder_id: String,
    pub next_trigger: DateTime<Utc>,
    pub recurrence: Option<ReminderRecurrence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderNotification {
    pub notification_id: String,
    pub reminder: Reminder,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderManagerConfig {
    pub enabled: bool,
    pub max_active_reminders: usize,
    pub snooze_duration_minutes: i64,
    pub auto_cleanup_completed: bool,
    pub cleanup_after_days: i64,
}

impl Default for ReminderManagerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_active_reminders: 50,
            snooze_duration_minutes: 15,
            auto_cleanup_completed: true,
            cleanup_after_days: 7,
        }
    }
}

pub struct ReminderManager {
    config: ReminderManagerConfig,
    reminders: HashMap<String, Reminder>,
    schedules: HashMap<String, ReminderSchedule>,
    notifications: Vec<ReminderNotification>,
    completed_history: Vec<Reminder>,
}

impl Default for ReminderManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ReminderManager {
    pub fn new() -> Self {
        Self {
            config: ReminderManagerConfig::default(),
            reminders: HashMap::new(),
            schedules: HashMap::new(),
            notifications: Vec::new(),
            completed_history: Vec::new(),
        }
    }

    pub fn with_config(config: ReminderManagerConfig) -> Self {
        Self {
            config,
            reminders: HashMap::new(),
            schedules: HashMap::new(),
            notifications: Vec::new(),
            completed_history: Vec::new(),
        }
    }

    pub fn get_config(&self) -> &ReminderManagerConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: ReminderManagerConfig) {
        self.config = config;
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    pub fn add_reminder(&mut self, reminder: Reminder) -> Result<(), ReminderError> {
        if self.reminders.len() >= self.config.max_active_reminders {
            return Err(ReminderError::LimitReached {
                max: self.config.max_active_reminders,
            });
        }

        let schedule = self.calculate_next_schedule(&reminder);
        self.reminders.insert(reminder.id.clone(), reminder.clone());
        self.schedules.insert(reminder.id.clone(), schedule);

        Ok(())
    }

    fn calculate_next_schedule(&self, reminder: &Reminder) -> ReminderSchedule {
        ReminderSchedule {
            reminder_id: reminder.id.clone(),
            next_trigger: reminder.scheduled_at,
            recurrence: reminder.recurrence.clone(),
        }
    }

    pub fn get_reminder(&self, id: &str) -> Option<&Reminder> {
        self.reminders.get(id)
    }

    pub fn get_all_reminders(&self) -> Vec<&Reminder> {
        self.reminders.values().collect()
    }

    pub fn get_active_reminders(&self) -> Vec<&Reminder> {
        self.reminders.values().filter(|r| !r.completed).collect()
    }

    pub fn get_due_reminders(&self) -> Vec<&Reminder> {
        let now = Utc::now();
        self.reminders
            .values()
            .filter(|r| !r.completed && r.scheduled_at <= now)
            .collect()
    }

    pub fn complete_reminder(&mut self, id: &str) -> Result<Reminder, ReminderError> {
        let (should_reschedule, recurrence) = {
            let schedule = self.schedules.get(id).ok_or(ReminderError::NotFound)?;
            let has_recurrence = schedule.recurrence.is_some();
            let rec = schedule.recurrence.clone();
            (has_recurrence, rec)
        };

        let reminder = self.reminders.get_mut(id).ok_or(ReminderError::NotFound)?;
        reminder.completed = true;
        let completed_reminder = reminder.clone();
        let current_scheduled_at = reminder.scheduled_at;
        let _ = reminder;

        if should_reschedule {
            let recurrence = recurrence.unwrap();
            let new_scheduled_at = self.calculate_next_occurrence(
                current_scheduled_at,
                recurrence.frequency,
                recurrence.interval,
            );

            let reminder = self.reminders.get_mut(id).unwrap();
            reminder.scheduled_at = new_scheduled_at;

            let schedule = self.schedules.get_mut(id).unwrap();
            schedule.next_trigger = new_scheduled_at;
        }

        self.completed_history.push(completed_reminder);
        Ok(self.reminders.get(id).unwrap().clone())
    }

    fn calculate_next_occurrence(
        &self,
        last: DateTime<Utc>,
        frequency: RecurrenceFrequency,
        interval: u32,
    ) -> DateTime<Utc> {
        match frequency {
            RecurrenceFrequency::Daily => last + Duration::days(interval as i64),
            RecurrenceFrequency::Weekly => last + Duration::weeks(interval as i64),
            RecurrenceFrequency::Monthly => last + Duration::days((interval as i64) * 30),
        }
    }

    pub fn snooze_reminder(
        &mut self,
        id: &str,
        duration_minutes: Option<i64>,
    ) -> Result<Reminder, ReminderError> {
        let duration = duration_minutes.unwrap_or(self.config.snooze_duration_minutes);
        let new_time = Utc::now() + Duration::minutes(duration);

        let reminder = self.reminders.get_mut(id).ok_or(ReminderError::NotFound)?;

        reminder.scheduled_at = new_time;

        if let Some(schedule) = self.schedules.get_mut(id) {
            schedule.next_trigger = new_time;
        }

        Ok(reminder.clone())
    }

    pub fn delete_reminder(&mut self, id: &str) -> Result<Reminder, ReminderError> {
        let reminder = self.reminders.remove(id).ok_or(ReminderError::NotFound)?;
        self.schedules.remove(id);
        Ok(reminder)
    }

    pub fn update_reminder(
        &mut self,
        id: &str,
        title: Option<String>,
        description: Option<String>,
        scheduled_at: Option<DateTime<Utc>>,
    ) -> Result<Reminder, ReminderError> {
        let reminder = self.reminders.get_mut(id).ok_or(ReminderError::NotFound)?;

        if let Some(t) = title {
            reminder.title = t;
        }
        if let Some(d) = description {
            reminder.description = d;
        }
        if let Some(s) = scheduled_at {
            reminder.scheduled_at = s;
            if let Some(schedule) = self.schedules.get_mut(id) {
                schedule.next_trigger = s;
            }
        }

        Ok(reminder.clone())
    }

    pub fn trigger_reminder(&mut self, id: &str) -> Result<ReminderNotification, ReminderError> {
        let reminder = self.reminders.get(id).ok_or(ReminderError::NotFound)?;

        if reminder.completed {
            return Err(ReminderError::AlreadyCompleted);
        }

        let notification = ReminderNotification {
            notification_id: format!("notif_{}", ProactiveAssistant::generate_reminder_id()),
            reminder: reminder.clone(),
            triggered_at: Utc::now(),
            acknowledged: false,
        };

        self.notifications.push(notification.clone());
        Ok(notification)
    }

    pub fn acknowledge_notification(&mut self, notification_id: &str) -> Result<(), ReminderError> {
        let notification = self
            .notifications
            .iter_mut()
            .find(|n| n.notification_id == notification_id)
            .ok_or(ReminderError::NotFound)?;

        notification.acknowledged = true;
        Ok(())
    }

    pub fn get_pending_notifications(&self) -> Vec<&ReminderNotification> {
        self.notifications
            .iter()
            .filter(|n| !n.acknowledged)
            .collect()
    }

    pub fn cleanup_completed(&mut self) {
        let cutoff = Utc::now() - Duration::days(self.config.cleanup_after_days);
        self.completed_history.retain(|r| r.scheduled_at > cutoff);
    }

    pub fn get_completed_history(&self) -> &[Reminder] {
        &self.completed_history
    }

    pub fn reschedule_all(&mut self, new_time: DateTime<Utc>) {
        for (id, reminder) in &mut self.reminders {
            reminder.scheduled_at = new_time;
            if let Some(schedule) = self.schedules.get_mut(id) {
                schedule.next_trigger = new_time;
            }
        }
    }

    pub fn snooze_all(&mut self, duration_minutes: i64) {
        let new_time = Utc::now() + Duration::minutes(duration_minutes);
        for (id, reminder) in &mut self.reminders {
            if !reminder.completed {
                reminder.scheduled_at = new_time;
                if let Some(schedule) = self.schedules.get_mut(id) {
                    schedule.next_trigger = new_time;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ReminderError {
    NotFound,
    LimitReached { max: usize },
    AlreadyCompleted,
    InvalidSchedule,
}

impl std::fmt::Display for ReminderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReminderError::NotFound => write!(f, "Reminder not found"),
            ReminderError::LimitReached { max } => write!(f, "Reminder limit reached: {}", max),
            ReminderError::AlreadyCompleted => write!(f, "Reminder already completed"),
            ReminderError::InvalidSchedule => write!(f, "Invalid reminder schedule"),
        }
    }
}

impl std::error::Error for ReminderError {}
