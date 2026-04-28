use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub plan_id: String,
    pub phase_index: usize,
    pub completed_task_ids: Vec<String>,
    pub state: serde_json::Value,
    pub timestamp: i64,
    pub label: Option<String>,
}

pub struct CheckpointManager {
    checkpoint_dir: PathBuf,
}

impl CheckpointManager {
    pub fn new(work_dir: &str) -> Self {
        let checkpoint_dir = PathBuf::from(work_dir).join(".axagent/checkpoints");
        Self { checkpoint_dir }
    }

    pub async fn save(&self, checkpoint: &Checkpoint) -> Result<(), String> {
        tokio::fs::create_dir_all(&self.checkpoint_dir)
            .await
            .map_err(|e| format!("Failed to create checkpoint directory: {}", e))?;

        let path = self.checkpoint_dir.join(format!("{}.json", checkpoint.id));
        let content = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| format!("Failed to serialize checkpoint: {}", e))?;

        tokio::fs::write(path, content)
            .await
            .map_err(|e| format!("Failed to write checkpoint: {}", e))?;

        Ok(())
    }

    pub async fn load(&self, id: &str) -> Result<Checkpoint, String> {
        let path = self.checkpoint_dir.join(format!("{id}.json"));

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read checkpoint '{}': {}", id, e))?;

        serde_json::from_str(&content).map_err(|e| format!("Failed to parse checkpoint: {}", e))
    }

    pub async fn list(&self) -> Result<Vec<Checkpoint>, String> {
        let mut entries = tokio::fs::read_dir(&self.checkpoint_dir)
            .await
            .map_err(|e| format!("Failed to read checkpoint directory: {}", e))?;

        let mut checkpoints: Vec<Checkpoint> = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read directory entry: {}", e))?
        {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| format!("Failed to read checkpoint file: {}", e))?;
                if let Ok(cp) = serde_json::from_str(&content) {
                    checkpoints.push(cp);
                }
            }
        }

        checkpoints.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(checkpoints)
    }

    pub async fn delete(&self, id: &str) -> Result<(), String> {
        let path = self.checkpoint_dir.join(format!("{id}.json"));

        if !path.exists() {
            return Err(format!("Checkpoint '{}' not found", id));
        }

        tokio::fs::remove_file(path)
            .await
            .map_err(|e| format!("Failed to delete checkpoint: {}", e))
    }

    pub async fn list_for_plan(&self, plan_id: &str) -> Result<Vec<Checkpoint>, String> {
        let all = self.list().await?;
        Ok(all
            .into_iter()
            .filter(|cp| cp.plan_id == plan_id)
            .collect())
    }

    pub async fn get_latest_for_plan(&self, plan_id: &str) -> Result<Option<Checkpoint>, String> {
        let plan_checkpoints = self.list_for_plan(plan_id).await?;
        Ok(plan_checkpoints.into_iter().next())
    }

    pub async fn cleanup_old(&self, keep_count: usize) -> Result<usize, String> {
        let all = self.list().await?;
        if all.len() <= keep_count {
            return Ok(0);
        }

        let to_delete = &all[keep_count..];
        let mut deleted = 0;
        for cp in to_delete {
            if self.delete(&cp.id).await.is_ok() {
                deleted += 1;
            }
        }
        Ok(deleted)
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new(".")
    }
}

pub struct CheckpointBuilder {
    plan_id: String,
    phase_index: usize,
    completed_task_ids: Vec<String>,
    state: serde_json::Value,
    label: Option<String>,
}

impl CheckpointBuilder {
    pub fn new(plan_id: &str, phase_index: usize) -> Self {
        Self {
            plan_id: plan_id.to_string(),
            phase_index,
            completed_task_ids: Vec::new(),
            state: serde_json::json!({}),
            label: None,
        }
    }

    pub fn with_completed_tasks(mut self, task_ids: Vec<String>) -> Self {
        self.completed_task_ids = task_ids;
        self
    }

    pub fn with_state(mut self, state: serde_json::Value) -> Self {
        self.state = state;
        self
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn build(self) -> Checkpoint {
        use std::time::{SystemTime, UNIX_EPOCH};
        let id = format!(
            "cp-{}-{}",
            self.plan_id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        Checkpoint {
            id,
            plan_id: self.plan_id,
            phase_index: self.phase_index,
            completed_task_ids: self.completed_task_ids,
            state: self.state,
            timestamp: chrono::Utc::now().timestamp(),
            label: self.label,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_builder() {
        let cp = CheckpointBuilder::new("plan-1", 0)
            .with_completed_tasks(vec!["task-1".to_string(), "task-2".to_string()])
            .with_state(serde_json::json!({"key": "value"}))
            .with_label("After setup phase")
            .build();

        assert_eq!(cp.plan_id, "plan-1");
        assert_eq!(cp.phase_index, 0);
        assert_eq!(cp.completed_task_ids.len(), 2);
        assert_eq!(cp.label, Some("After setup phase".to_string()));
    }
}
