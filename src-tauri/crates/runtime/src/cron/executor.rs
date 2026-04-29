use std::sync::Arc;

use super::job_store::CronJob;

pub struct CronExecutor {
    on_execute: Option<Arc<dyn Fn(&CronJob) + Send + Sync>>,
}

impl CronExecutor {
    pub fn new() -> Self {
        Self { on_execute: None }
    }

    pub fn set_handler<F>(&mut self, handler: F)
    where
        F: Fn(&CronJob) + Send + Sync + 'static,
    {
        self.on_execute = Some(Arc::new(handler));
    }

    pub async fn execute(&self, job: CronJob) {
        tracing::info!(
            "Cron: executing job '{}' with prompt: {}",
            job.name,
            &job.prompt[..std::cmp::min(job.prompt.len(), 100)]
        );

        if let Some(ref handler) = self.on_execute {
            handler(&job);
        } else {
            tracing::warn!(
                "Cron: no handler set for job '{}'. Prompt: {}",
                job.name,
                job.prompt
            );
        }
    }
}

impl Default for CronExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[tokio::test]
    async fn test_executor_calls_handler() {
        let mut executor = CronExecutor::new();
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        executor.set_handler(move |job| {
            assert_eq!(job.name, "test");
            let mut c = called_clone.lock().unwrap();
            *c = true;
        });

        let job = CronJob::new("test", "* * * * *", "test prompt");
        executor.execute(job).await;

        assert!(*called.lock().unwrap());
    }
}
