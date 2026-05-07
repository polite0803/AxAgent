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
        let job =
            TrainingJob::new(uuid::Uuid::new_v4().to_string(), dataset_id, base_model, config);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine_tune::dataset::{FineTuneSample, SampleMetadata};

    fn make_sample(id: &str, input: &str, output: &str) -> FineTuneSample {
        FineTuneSample {
            id: id.to_string(),
            input: input.to_string(),
            output: output.to_string(),
            system_prompt: None,
            metadata: SampleMetadata {
                source: "test".to_string(),
                category: None,
                difficulty: None,
                tags: vec![],
            },
        }
    }

    fn make_dataset() -> FineTuneDataset {
        let mut ds = FineTuneDataset::new("ds1".to_string(), "Test".to_string());
        ds.add_sample(make_sample("s1", "hello", "world"));
        ds.add_sample(make_sample("s2", "foo", "bar"));
        ds
    }

    #[test]
    fn test_fine_tune_trainer_new() {
        let trainer = FineTuneTrainer::new();
        assert!(trainer.list_jobs().is_empty());
        assert!(trainer.get_current_job().is_none());
    }

    #[test]
    fn test_fine_tune_trainer_create_job() {
        let mut trainer = FineTuneTrainer::new();
        let job = trainer.create_job("ds1".to_string(), "model1".to_string(), LoRAConfig::default());
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(trainer.list_jobs().len(), 1);
    }

    #[test]
    fn test_fine_tune_trainer_get_job() {
        let mut trainer = FineTuneTrainer::new();
        let job = trainer.create_job("ds1".to_string(), "model1".to_string(), LoRAConfig::default());
        let found = trainer.get_job(&job.id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().dataset_id, "ds1");
        assert!(trainer.get_job("nonexistent").is_none());
    }

    #[test]
    fn test_fine_tune_trainer_start_training() {
        let mut trainer = FineTuneTrainer::new();
        let job = trainer.create_job("ds1".to_string(), "model1".to_string(), LoRAConfig::default());
        let result = trainer.start_training(&job.id);
        assert!(result.is_ok());
        let found = trainer.get_job(&job.id).unwrap();
        assert_eq!(found.status, JobStatus::Preparing);
    }

    #[test]
    fn test_fine_tune_trainer_start_training_non_pending() {
        let mut trainer = FineTuneTrainer::new();
        let job = trainer.create_job("ds1".to_string(), "model1".to_string(), LoRAConfig::default());
        trainer.start_training(&job.id).unwrap();
        let result = trainer.start_training(&job.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_fine_tune_trainer_start_nonexistent() {
        let mut trainer = FineTuneTrainer::new();
        let result = trainer.start_training("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_fine_tune_trainer_cancel_training() {
        let mut trainer = FineTuneTrainer::new();
        let job = trainer.create_job("ds1".to_string(), "model1".to_string(), LoRAConfig::default());
        let result = trainer.cancel_training(&job.id);
        assert!(result.is_ok());
        assert_eq!(trainer.get_job(&job.id).unwrap().status, JobStatus::Cancelled);
    }

    #[test]
    fn test_fine_tune_trainer_complete_job() {
        let mut trainer = FineTuneTrainer::new();
        let job = trainer.create_job("ds1".to_string(), "model1".to_string(), LoRAConfig::default());
        trainer.start_training(&job.id).unwrap();
        let result = trainer.complete_job(&job.id, "/output/lora".to_string());
        assert!(result.is_ok());
        assert_eq!(trainer.get_job(&job.id).unwrap().status, JobStatus::Completed);
    }

    #[test]
    fn test_fine_tune_trainer_fail_job() {
        let mut trainer = FineTuneTrainer::new();
        let job = trainer.create_job("ds1".to_string(), "model1".to_string(), LoRAConfig::default());
        let result = trainer.fail_job(&job.id);
        assert!(result.is_ok());
        assert_eq!(trainer.get_job(&job.id).unwrap().status, JobStatus::Failed);
    }

    #[test]
    fn test_fine_tune_trainer_delete_job() {
        let mut trainer = FineTuneTrainer::new();
        let job = trainer.create_job("ds1".to_string(), "model1".to_string(), LoRAConfig::default());
        let result = trainer.delete_job(&job.id);
        assert!(result.is_ok());
        assert!(trainer.get_job(&job.id).is_none());
        assert!(trainer.list_jobs().is_empty());
    }

    #[test]
    fn test_fine_tune_trainer_update_progress() {
        let mut trainer = FineTuneTrainer::new();
        let job = trainer.create_job("ds1".to_string(), "model1".to_string(), LoRAConfig::default());
        let result = trainer.update_progress(&job.id, 1, 10, 0.5);
        assert!(result.is_ok());
        let found = trainer.get_job(&job.id).unwrap();
        assert_eq!(found.progress.current_epoch, 1);
        assert_eq!(found.progress.current_step, 10);
        assert!((found.progress.loss - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_fine_tune_trainer_list_jobs_by_status() {
        let mut trainer = FineTuneTrainer::new();
        let j1 = trainer.create_job("ds1".to_string(), "m1".to_string(), LoRAConfig::default());
        let _j2 = trainer.create_job("ds2".to_string(), "m2".to_string(), LoRAConfig::default());
        trainer.cancel_training(&j1.id).unwrap();
        let pending = trainer.list_jobs_by_status(JobStatus::Cancelled);
        assert_eq!(pending.len(), 1);
        let active = trainer.list_jobs_by_status(JobStatus::Pending);
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_fine_tune_trainer_get_training_stats() {
        let mut trainer = FineTuneTrainer::new();
        let j1 = trainer.create_job("ds1".to_string(), "m1".to_string(), LoRAConfig::default());
        let _j2 = trainer.create_job("ds2".to_string(), "m2".to_string(), LoRAConfig::default());
        trainer.cancel_training(&j1.id).unwrap();
        let stats = trainer.get_training_stats();
        assert_eq!(stats.total_jobs, 2);
        assert_eq!(stats.failed_jobs, 0);
    }

    #[test]
    fn test_dataset_converter_to_alpaca() {
        let ds = make_dataset();
        let result = DatasetConverter::convert_to_alpaca(&ds);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("instruction"));
        assert!(json.contains("output"));
    }

    #[test]
    fn test_dataset_converter_to_chatml() {
        let ds = make_dataset();
        let result = DatasetConverter::convert_to_chatml(&ds);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("system"));
        assert!(output.contains("user"));
        assert!(output.contains("assistant"));
    }

    #[test]
    fn test_dataset_converter_to_jsonl() {
        let ds = make_dataset();
        let result = DatasetConverter::convert_to_jsonl(&ds);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("hello"));
        assert!(output.contains("foo"));
    }

    #[test]
    fn test_fine_tune_trainer_pause_training() {
        let mut trainer = FineTuneTrainer::new();
        let job = trainer.create_job("ds1".to_string(), "model1".to_string(), LoRAConfig::default());
        trainer.start_training(&job.id).unwrap();
        let j = trainer.get_job_mut(&job.id).unwrap();
        j.status = JobStatus::Training;
        let result = trainer.pause_training(&job.id);
        assert!(result.is_ok());
    }
}
