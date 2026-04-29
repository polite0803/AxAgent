use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{Datelike, Timelike};

use super::job_store::{CronJobStore};
use super::executor::CronExecutor;

pub struct CronScheduler {
    store: Arc<CronJobStore>,
    executor: Arc<CronExecutor>,
    running: Arc<RwLock<bool>>,
    handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl CronScheduler {
    pub fn new(store: Arc<CronJobStore>, executor: Arc<CronExecutor>) -> Self {
        Self {
            store,
            executor,
            running: Arc::new(RwLock::new(false)),
            handle: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let store = self.store.clone();
        let executor = self.executor.clone();
        let running_flag = self.running.clone();

        let handle = tokio::spawn(async move {
            let mut last_check = chrono::Utc::now();

            loop {
                if !*running_flag.read().await {
                    break;
                }

                let now = chrono::Utc::now();
                let jobs = store.list_enabled().await;

                for job in &jobs {
                    if let Some(next_run) = job.next_run_at {
                        let next = chrono::DateTime::from_timestamp_millis(next_run)
                            .unwrap_or(chrono::Utc::now());
                        if now < next {
                            continue;
                        }
                    }

                    if should_run_now(&job.schedule, &last_check, &now) {
                        tracing::info!("Cron: running job '{}' ({})", job.name, job.id);
                        executor.execute(job.clone()).await;
                        store.record_run(&job.id).await;
                    }
                }

                last_check = now;
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });

        let mut h = self.handle.write().await;
        *h = Some(handle);
    }

    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        drop(running);

        if let Some(handle) = self.handle.write().await.take() {
            handle.abort();
            let _ = handle.await;
        }
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

fn should_run_now(
    schedule: &str,
    last_check: &chrono::DateTime<chrono::Utc>,
    now: &chrono::DateTime<chrono::Utc>,
) -> bool {
    let _ = last_check;
    if schedule.contains('*') || schedule.contains('/') || schedule.contains(',') || schedule.contains('-') {
        let parts: Vec<&str> = schedule.split_whitespace().collect();
        if parts.len() == 5 {
            let minute_match = match_cron_field(parts[0], now.minute() as i64, 0, 59);
            let hour_match = match_cron_field(parts[1], now.hour() as i64, 0, 23);
            let day_match = match_cron_field(parts[2], now.day() as i64, 1, 31);
            let month_match = match_cron_field(parts[3], now.month() as i64, 1, 12);
            let weekday_match = match_cron_field(parts[4], now.weekday().num_days_from_sunday() as i64, 0, 6);

            return minute_match && hour_match && day_match && month_match && weekday_match;
        }
    }

    false
}

fn match_cron_field(field: &str, current: i64, _min: i64, _max: i64) -> bool {
    if field == "*" {
        return true;
    }

    if let Some(step) = field.strip_prefix("*/") {
        if let Ok(interval) = step.parse::<i64>() {
            return current % interval == 0;
        }
    }

    if field.contains(',') {
        return field.split(',').any(|p| match_cron_field(p, current, _min, _max));
    }

    if field.contains('-') {
        let parts: Vec<&str> = field.split('-').collect();
        if parts.len() == 2 {
            if let (Ok(lo), Ok(hi)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
                return current >= lo && current <= hi;
            }
        }
    }

    if let Ok(exact) = field.parse::<i64>() {
        return current == exact;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_cron_field_wildcard() {
        assert!(match_cron_field("*", 30, 0, 59));
    }

    #[test]
    fn test_match_cron_field_exact() {
        assert!(match_cron_field("30", 30, 0, 59));
        assert!(!match_cron_field("30", 31, 0, 59));
    }

    #[test]
    fn test_match_cron_field_range() {
        assert!(match_cron_field("10-20", 15, 0, 59));
        assert!(!match_cron_field("10-20", 25, 0, 59));
    }
   
    #[test]
    fn test_match_cron_field_list() {
        assert!(match_cron_field("0,30", 30, 0, 59));
        assert!(!match_cron_field("0,30", 15, 0, 59));
    }

    #[test]
    fn test_match_cron_field_step() {
        assert!(match_cron_field("*/15", 30, 0, 59));
        assert!(!match_cron_field("*/15", 31, 0, 59));
    }
}
