use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub prompt: String,
    pub platform: Option<String>,
    pub enabled_toolsets: Option<Vec<String>>,
    pub enabled: bool,
    pub last_run_at: Option<i64>,
    pub next_run_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl CronJob {
    pub fn new(name: &str, schedule: &str, prompt: &str) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            schedule: schedule.to_string(),
            prompt: prompt.to_string(),
            platform: None,
            enabled_toolsets: None,
            enabled: true,
            last_run_at: None,
            next_run_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_platform(mut self, platform: &str) -> Self {
        self.platform = Some(platform.to_string());
        self
    }

    pub fn with_toolsets(mut self, toolsets: Vec<String>) -> Self {
        self.enabled_toolsets = Some(toolsets);
        self
    }
}

pub struct CronJobStore {
    jobs: std::sync::Arc<tokio::sync::RwLock<Vec<CronJob>>>,
}

impl CronJobStore {
    pub fn new() -> Self {
        Self {
            jobs: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    pub async fn add(&self, job: CronJob) -> String {
        let id = job.id.clone();
        let mut jobs = self.jobs.write().await;
        jobs.push(job);
        id
    }

    pub async fn remove(&self, id: &str) -> bool {
        let mut jobs = self.jobs.write().await;
        let len = jobs.len();
        jobs.retain(|j| j.id != id);
        jobs.len() < len
    }

    pub async fn get(&self, id: &str) -> Option<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.iter().find(|j| j.id == id).cloned()
    }

    pub async fn update(&self, id: &str, updater: impl FnOnce(&mut CronJob)) -> bool {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.iter_mut().find(|j| j.id == id) {
            updater(job);
            job.updated_at = chrono::Utc::now().timestamp_millis();
            true
        } else {
            false
        }
    }

    pub async fn list(&self) -> Vec<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.clone()
    }

    pub async fn list_enabled(&self) -> Vec<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.iter().filter(|j| j.enabled).cloned().collect()
    }

    pub async fn set_enabled(&self, id: &str, enabled: bool) -> bool {
        self.update(id, |job| {
            job.enabled = enabled;
        })
        .await
    }

    pub async fn record_run(&self, id: &str) -> bool {
        let now = chrono::Utc::now().timestamp_millis();
        self.update(id, |job| {
            job.last_run_at = Some(now);
            job.updated_at = now;
        })
        .await
    }
}

impl Default for CronJobStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cron_job_store_crud() {
        let store = CronJobStore::new();
        let job = CronJob::new("test", "0 9 * * *", "daily summary");
        let id = store.add(job).await;

        let found = store.get(&id).await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test");

        store.update(&id, |j| j.name = "updated".to_string()).await;
        let found = store.get(&id).await;
        assert_eq!(found.unwrap().name, "updated");

        assert!(store.remove(&id).await);
        assert!(store.get(&id).await.is_none());
    }

    #[tokio::test]
    async fn test_cron_job_with_options() {
        let job = CronJob::new("notify", "0 9 * * *", "morning check")
            .with_platform("telegram")
            .with_toolsets(vec!["gmail".to_string(), "calendar".to_string()]);

        assert_eq!(job.platform, Some("telegram".to_string()));
        assert_eq!(job.enabled_toolsets.as_ref().unwrap().len(), 2);
        assert!(job.enabled);
    }

    #[tokio::test]
    async fn test_cron_job_store_clear_enabled() {
        let store = CronJobStore::new();
        store.add(CronJob::new("j1", "* * * * *", "p1")).await;
        store.add(CronJob::new("j2", "* * * * *", "p2")).await;

        let enabled = store.list_enabled().await;
        assert_eq!(enabled.len(), 2);

        let ids = store.list().await;
        store.set_enabled(&ids[0].id, false).await;

        let enabled = store.list_enabled().await;
        assert_eq!(enabled.len(), 1);
    }

    #[tokio::test]
    async fn test_record_run() {
        let store = CronJobStore::new();
        let id = store.add(CronJob::new("runner", "* * * * *", "run me")).await;

        assert!(store.record_run(&id).await);

        let job = store.get(&id).await.unwrap();
        assert!(job.last_run_at.is_some());
    }
}
