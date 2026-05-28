use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPriority {
    Critical = 4,
    High = 3,
    Normal = 2,
    Low = 1,
    Background = 0,
}

impl PartialOrd for TaskPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TaskPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub agent_id: String,
    pub priority: TaskPriority,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub resource_weight: f64,
    pub is_preemptible: bool,
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ScheduledTask {}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.deadline.cmp(&self.deadline))
            .then_with(|| other.created_at.cmp(&self.created_at))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub max_concurrent_tasks: usize,
    pub total_resource_capacity: f64,
    pub preempt_enabled: bool,
    pub starvation_threshold_ms: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            total_resource_capacity: 1.0,
            preempt_enabled: true,
            starvation_threshold_ms: 30_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreemptionResult {
    pub preempted_task_ids: Vec<String>,
    pub reason: String,
}

pub struct PriorityScheduler {
    config: SchedulerConfig,
    waiting_queue: BinaryHeap<ScheduledTask>,
    running_tasks: Vec<(ScheduledTask, f64)>,
    total_allocated: f64,
}

impl PriorityScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            waiting_queue: BinaryHeap::new(),
            running_tasks: Vec::new(),
            total_allocated: 0.0,
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(SchedulerConfig::default())
    }

    pub fn submit(&mut self, task: ScheduledTask) {
        self.waiting_queue.push(task);
    }

    pub fn schedule(&mut self) -> Vec<ScheduledTask> {
        let mut scheduled = Vec::new();

        while let Some(t) = self.waiting_queue.peek() {
            let task = t.clone();

            if self.running_tasks.len() >= self.config.max_concurrent_tasks {
                break;
            }
            if self.total_allocated + task.resource_weight > self.config.total_resource_capacity {
                if self.config.preempt_enabled
                    && let Some(preempted) = self.try_preempt(&task)
                {
                    self.running_tasks
                        .retain(|(t, _)| !preempted.preempted_task_ids.contains(&t.id));
                    self.total_allocated = self.running_tasks.iter().map(|(_, w)| *w).sum();
                    continue;
                }
                break;
            }

            let task = self
                .waiting_queue
                .pop()
                .expect("peek guaranteed the queue is non-empty");
            self.total_allocated += task.resource_weight;
            self.running_tasks
                .push((task.clone(), task.resource_weight));
            scheduled.push(task);
        }

        self.apply_anti_starvation();

        scheduled
    }

    fn try_preempt(&mut self, incoming: &ScheduledTask) -> Option<PreemptionResult> {
        if !incoming.is_preemptible {
            return None;
        }

        let mut preemptible: Vec<&(ScheduledTask, f64)> = self
            .running_tasks
            .iter()
            .filter(|(t, _)| t.is_preemptible && t.priority < incoming.priority)
            .collect();

        if preemptible.is_empty() {
            return None;
        }

        preemptible.sort_by_key(|a| a.0.priority);

        let mut freed = 0.0;
        let mut preempted_ids = Vec::new();

        for (task, weight) in &preemptible {
            if freed + weight >= incoming.resource_weight {
                preempted_ids.push(task.id.clone());
                freed += weight;
                break;
            }
            preempted_ids.push(task.id.clone());
            freed += weight;
        }

        if freed < incoming.resource_weight {
            return None;
        }

        Some(PreemptionResult {
            preempted_task_ids: preempted_ids,
            reason: format!("Preempted for higher priority task: {}", incoming.id),
        })
    }

    fn apply_anti_starvation(&mut self) {
        let now = chrono::Utc::now();
        let threshold = chrono::Duration::milliseconds(self.config.starvation_threshold_ms as i64);

        let mut boosted = Vec::new();

        while let Some(mut task) = self.waiting_queue.pop() {
            if now - task.created_at > threshold && task.priority < TaskPriority::Normal {
                tracing::info!(
                    task_id = %task.id,
                    old_priority = ?task.priority,
                    "Boosting starved task priority"
                );
                task.priority = TaskPriority::Normal;
            }
            boosted.push(task);
        }

        for task in boosted {
            self.waiting_queue.push(task);
        }
    }

    pub fn complete(&mut self, task_id: &str) {
        if let Some(pos) = self.running_tasks.iter().position(|(t, _)| t.id == task_id) {
            let (_, weight) = self.running_tasks.remove(pos);
            self.total_allocated -= weight;
        }
    }

    pub fn running_count(&self) -> usize {
        self.running_tasks.len()
    }

    pub fn waiting_count(&self) -> usize {
        self.waiting_queue.len()
    }

    pub fn get_running_tasks(&self) -> Vec<&ScheduledTask> {
        self.running_tasks.iter().map(|(t, _)| t).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, priority: TaskPriority, weight: f64) -> ScheduledTask {
        ScheduledTask {
            id: id.to_string(),
            agent_id: format!("agent_{}", id),
            priority,
            description: format!("Task {}", id),
            created_at: chrono::Utc::now(),
            deadline: None,
            resource_weight: weight,
            is_preemptible: true,
        }
    }

    #[test]
    fn test_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    #[test]
    fn test_schedule_by_priority() {
        let mut scheduler = PriorityScheduler::with_default_config();
        scheduler.submit(make_task("low", TaskPriority::Low, 0.1));
        scheduler.submit(make_task("critical", TaskPriority::Critical, 0.1));
        scheduler.submit(make_task("normal", TaskPriority::Normal, 0.1));

        let scheduled = scheduler.schedule();
        assert_eq!(scheduled[0].id, "critical");
    }

    #[test]
    fn test_resource_capacity() {
        let mut scheduler = PriorityScheduler::with_default_config();
        scheduler.submit(make_task("1", TaskPriority::High, 0.6));
        scheduler.submit(make_task("2", TaskPriority::Normal, 0.5));

        let scheduled = scheduler.schedule();
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].id, "1");
    }

    #[test]
    fn test_preemption() {
        let mut scheduler = PriorityScheduler::with_default_config();
        scheduler.submit(make_task("low", TaskPriority::Low, 0.5));
        let _ = scheduler.schedule();

        scheduler.submit(make_task("critical", TaskPriority::Critical, 0.5));
        let scheduled = scheduler.schedule();
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].id, "critical");
    }

    #[test]
    fn test_complete_frees_resource() {
        let mut scheduler = PriorityScheduler::with_default_config();
        scheduler.submit(make_task("1", TaskPriority::High, 0.5));
        let _ = scheduler.schedule();
        assert_eq!(scheduler.running_count(), 1);

        scheduler.complete("1");
        assert_eq!(scheduler.running_count(), 0);
    }
}
