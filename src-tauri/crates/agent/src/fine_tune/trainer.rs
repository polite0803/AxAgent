use crate::fine_tune::dataset::{FineTuneDataset, FineTuneError};
use crate::fine_tune::lora::{JobStatus, LoRAConfig, TrainingJob};

pub struct FineTuneTrainer {
    jobs: Vec<TrainingJob>,
    current_job: Option<String>,
}

impl FineTuneTrainer {
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            current_job: None,
        }
    }

    pub fn create_job(
        &mut self,
        dataset_id: String,
        base_model: String,
        config: LoRAConfig,
    ) -> TrainingJob {
        let job = TrainingJob::new(
            uuid::Uuid::new_v4().to_string(),
            dataset_id,
            base_model,
            config,
        );
        self.jobs.push(job.clone());
        job
    }

    pub fn get_job(&self, job_id: &str) -> Option<&TrainingJob> {
        self.jobs.iter().find(|j| j.id == job_id)
    }

    pub fn get_job_mut(&mut self, job_id: &str) -> Option<&mut TrainingJob> {
        self.jobs.iter_mut().find(|j| j.id == job_id)
    }

    pub fn list_jobs(&self) -> Vec<&TrainingJob> {
        self.jobs.iter().collect()
    }

    pub fn list_jobs_by_status(&self, status: JobStatus) -> Vec<&TrainingJob> {
        self.jobs.iter().filter(|j| j.status == status).collect()
    }

    pub fn start_training(&mut self, job_id: &str) -> Result<(), FineTuneError> {
        if let Some(job) = self.get_job_mut(job_id) {
            if job.status == JobStatus::Pending {
                job.status = JobStatus::Preparing;
                self.current_job = Some(job_id.to_string());
                Ok(())
            } else {
                Err(FineTuneError::ValidationError(format!(
                    "Cannot start job in status {:?}",
                    job.status
                )))
            }
        } else {
            Err(FineTuneError::NotFound(job_id.to_string()))
        }
    }

    pub fn pause_training(&mut self, job_id: &str) -> Result<(), FineTuneError> {
        if let Some(job) = self.get_job_mut(job_id) {
            if job.status == JobStatus::Training {
                job.status = JobStatus::Pending;
                Ok(())
            } else {
                Err(FineTuneError::ValidationError(format!(
                    "Cannot pause job in status {:?}",
                    job.status
                )))
            }
        } else {
            Err(FineTuneError::NotFound(job_id.to_string()))
        }
    }

    pub fn cancel_training(&mut self, job_id: &str) -> Result<(), FineTuneError> {
        if let Some(job) = self.get_job_mut(job_id) {
            job.cancel();
            if self.current_job.as_deref() == Some(job_id) {
                self.current_job = None;
            }
            Ok(())
        } else {
            Err(FineTuneError::NotFound(job_id.to_string()))
        }
    }

    pub fn delete_job(&mut self, job_id: &str) -> Result<TrainingJob, FineTuneError> {
        if let Some(pos) = self.jobs.iter().position(|j| j.id == job_id) {
            if self.current_job.as_deref() == Some(job_id) {
                self.current_job = None;
            }
            Ok(self.jobs.remove(pos))
        } else {
            Err(FineTuneError::NotFound(job_id.to_string()))
        }
    }

    pub fn get_current_job(&self) -> Option<&TrainingJob> {
        self.current_job.as_ref().and_then(|id| self.get_job(id))
    }

    pub fn update_progress(
        &mut self,
        job_id: &str,
        current_epoch: u32,
        current_step: u32,
        loss: f32,
    ) -> Result<(), FineTuneError> {
        if let Some(job) = self.get_job_mut(job_id) {
            job.progress.current_epoch = current_epoch;
            job.progress.current_step = current_step;
            job.progress.loss = loss;
            job.metrics.train_loss.push(loss);
            Ok(())
        } else {
            Err(FineTuneError::NotFound(job_id.to_string()))
        }
    }

    pub fn complete_job(&mut self, job_id: &str, output_path: String) -> Result<(), FineTuneError> {
        if let Some(job) = self.get_job_mut(job_id) {
            job.complete(output_path);
            self.current_job = None;
            Ok(())
        } else {
            Err(FineTuneError::NotFound(job_id.to_string()))
        }
    }

    pub fn fail_job(&mut self, job_id: &str) -> Result<(), FineTuneError> {
        if let Some(job) = self.get_job_mut(job_id) {
            job.fail();
            self.current_job = None;
            Ok(())
        } else {
            Err(FineTuneError::NotFound(job_id.to_string()))
        }
    }

    pub fn get_training_stats(&self) -> TrainingStats {
        let total = self.jobs.len();
        let completed = self
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Completed)
            .count();
        let running = self.jobs.iter().filter(|j| j.is_running()).count();
        let failed = self
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Failed)
            .count();

        TrainingStats {
            total_jobs: total,
            completed_jobs: completed,
            running_jobs: running,
            failed_jobs: failed,
        }
    }
}

impl Default for FineTuneTrainer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrainingStats {
    pub total_jobs: usize,
    pub completed_jobs: usize,
    pub running_jobs: usize,
    pub failed_jobs: usize,
}

pub struct DatasetConverter;

impl DatasetConverter {
    pub fn convert_to_alpaca(dataset: &FineTuneDataset) -> Result<String, FineTuneError> {
        let samples: Vec<serde_json::Value> = dataset
            .samples
            .iter()
            .map(|s| {
                serde_json::json!({
                    "instruction": s.input,
                    "output": s.output,
                    "system": s.system_prompt.clone().unwrap_or_default()
                })
            })
            .collect();

        serde_json::to_string_pretty(&samples)
            .map_err(|e| FineTuneError::SerializationError(e.to_string()))
    }

    pub fn convert_to_chatml(dataset: &FineTuneDataset) -> Result<String, FineTuneError> {
        let samples: Vec<String> = dataset
            .samples
            .iter()
            .map(|s| {
                let messages = serde_json::json!([
                    {"role": "system", "content": s.system_prompt.as_deref().unwrap_or("")},
                    {"role": "user", "content": &s.input},
                    {"role": "assistant", "content": &s.output}
                ]);
                serde_json::to_string(&messages).unwrap_or_default()
            })
            .collect();

        Ok(samples.join("\n"))
    }

    pub fn convert_to_jsonl(dataset: &FineTuneDataset) -> Result<String, FineTuneError> {
        let lines: Vec<String> = dataset
            .samples
            .iter()
            .map(|s| serde_json::to_string(s).unwrap_or_default())
            .collect();

        Ok(lines.join("\n"))
    }
}
