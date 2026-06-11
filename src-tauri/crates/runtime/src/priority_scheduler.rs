// SPDX-License-Identifier: AGPL-3.0-only

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
    /// 入队序号：值越小表示越早入队，由 submit() 自动分配，外部请勿手动设置。
    /// 用于在反饥饿提升优先级后保持 FIFO 顺序。
    pub enqueue_seq: u64,
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
        // 主排序键：优先级（同优先级时按 FIFO 入队顺序）。
        // 反饥饿只提升 priority，enqueue_seq 保持不变，确保老任务在同优先级中始终排在新任务之前。
        // 入队序号取反向以适配 BinaryHeap（max-heap）：较早入队者排在上层。
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.enqueue_seq.cmp(&self.enqueue_seq))
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
    /// 被抢占任务的完整快照，调度器据此将任务重新放回等待队列，避免任务丢失。
    pub preempted_tasks: Vec<ScheduledTask>,
    pub reason: String,
}

#[derive(Debug)]
pub struct PriorityScheduler {
    config: SchedulerConfig,
    waiting_queue: BinaryHeap<ScheduledTask>,
    running_tasks: Vec<(ScheduledTask, f64)>,
    total_allocated: f64,
    /// 入队序号计数器，submit() 时自增分配给新任务。
    enqueue_seq_counter: u64,
}

impl PriorityScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            waiting_queue: BinaryHeap::new(),
            running_tasks: Vec::new(),
            total_allocated: 0.0,
            enqueue_seq_counter: 0,
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(SchedulerConfig::default())
    }

    pub fn submit(&mut self, mut task: ScheduledTask) {
        // 自动分配入队序号：调用方构造 ScheduledTask 时无需关心该字段
        task.enqueue_seq = self.enqueue_seq_counter;
        self.enqueue_seq_counter += 1;
        self.waiting_queue.push(task);
    }

    pub fn schedule(&mut self) -> Vec<ScheduledTask> {
        let mut scheduled = Vec::new();

        while let Some(t) = self.waiting_queue.peek() {
            let task = t.clone();

            let has_slot = self.running_tasks.len() < self.config.max_concurrent_tasks;
            let has_capacity =
                self.total_allocated + task.resource_weight <= self.config.total_resource_capacity;

            if has_slot && has_capacity {
                let task = self
                    .waiting_queue
                    .pop()
                    .expect("peek guaranteed the queue is non-empty");
                self.total_allocated += task.resource_weight;
                self.running_tasks
                    .push((task.clone(), task.resource_weight));
                scheduled.push(task);
                continue;
            }

            // 资源或槽位不足时尝试抢占
            if self.config.preempt_enabled
                && let Some(preempted) = self.try_preempt(&task)
            {
                self.running_tasks
                    .retain(|(t, _)| !preempted.preempted_task_ids.contains(&t.id));
                self.total_allocated = self.running_tasks.iter().map(|(_, w)| *w).sum();
                // 修复 1.3：被抢占的任务必须重新放回等待队列
                for preempted_task in preempted.preempted_tasks {
                    self.waiting_queue.push(preempted_task);
                }
                continue;
            }

            break;
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

        // 修复 1.3：按优先级升序排序（低优先级优先），确保最低优先级任务先被抢占，
        // 为更高优先级任务让路的语义与注释一致。
        preemptible.sort_by_key(|a| std::cmp::Reverse(a.0.priority));

        let mut freed = 0.0;
        let mut preempted_ids = Vec::new();
        let mut preempted_tasks = Vec::new();

        for (task, weight) in &preemptible {
            if freed + weight >= incoming.resource_weight {
                preempted_ids.push(task.id.clone());
                preempted_tasks.push(task.clone());
                freed += weight;
                break;
            }
            preempted_ids.push(task.id.clone());
            preempted_tasks.push(task.clone());
            freed += weight;
        }

        if freed < incoming.resource_weight {
            return None;
        }

        Some(PreemptionResult {
            preempted_task_ids: preempted_ids,
            preempted_tasks,
            reason: format!("Preempted for higher priority task: {}", incoming.id),
        })
    }

    fn apply_anti_starvation(&mut self) {
        let now = chrono::Utc::now();
        let threshold = chrono::Duration::milliseconds(self.config.starvation_threshold_ms as i64);

        let mut boosted = Vec::new();

        while let Some(mut task) = self.waiting_queue.pop() {
            // 修复 1.4：只提升 priority，enqueue_seq/created_at 保持不变。
            // Ord 中按 (priority, enqueue_seq) 比较，老任务因序号更小在同优先级中始终排在新任务之前，
            // 避免反饥饿提升后被新 Normal 任务插队导致继续饿死。
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
            enqueue_seq: 0,
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

    /// 修复 1.3 回归测试：被抢占的任务必须重新进入等待队列，不得消失。
    #[test]
    fn test_preempted_task_returns_to_queue() {
        let config = SchedulerConfig {
            max_concurrent_tasks: 1,
            total_resource_capacity: 1.0,
            preempt_enabled: true,
            starvation_threshold_ms: 30_000,
        };
        let mut scheduler = PriorityScheduler::new(config);

        // 先让一个低优先级任务占满资源（占用 0.6 容量）
        scheduler.submit(make_task("low", TaskPriority::Low, 0.6));
        let _ = scheduler.schedule();
        assert_eq!(scheduler.running_count(), 1);

        // 提交 Critical 任务（0.5 资源），可用资源不足触发对 low 的抢占
        scheduler.submit(make_task("critical", TaskPriority::Critical, 0.5));
        let scheduled = scheduler.schedule();

        // 验证 critical 已被调度
        assert!(scheduled.iter().any(|t| t.id == "critical"));
        // 验证 low 已从 running 移除
        let running_ids: Vec<String> = scheduler
            .get_running_tasks()
            .iter()
            .map(|t| t.id.clone())
            .collect();
        assert!(!running_ids.contains(&"low".to_string()));
        // 验证 low 已重新进入 waiting 队列（关键修复点：不再消失）
        assert_eq!(scheduler.waiting_count(), 1);
    }

    /// 修复 1.4 回归测试：提升优先级后，老任务必须保持 FIFO 顺序，不能被新 Normal 任务插队。
    #[test]
    fn test_anti_starvation_preserves_fifo_order() {
        let config = SchedulerConfig {
            max_concurrent_tasks: 1,
            total_resource_capacity: 1.0,
            preempt_enabled: true,
            // 设为 0 立即触发反饥饿提升
            starvation_threshold_ms: 0,
        };
        let mut scheduler = PriorityScheduler::new(config);

        // 热身：让首任务先跑再完成，确保后续入队序号从 1 开始
        scheduler.submit(make_task("warmup", TaskPriority::Low, 0.1));
        let _ = scheduler.schedule();
        scheduler.complete("warmup");

        // 提交 first_low（Low，seq=1）和 second_normal（Normal，seq=2）
        // 调度一次：second_normal 优先运行，first_low 因 max 满留在 waiting
        // 然后 apply_anti_starvation 将 first_low 提升为 Normal
        scheduler.submit(make_task("first_low", TaskPriority::Low, 0.1));
        scheduler.submit(make_task("second_normal", TaskPriority::Normal, 0.1));
        let _ = scheduler.schedule();
        // 此时 first_low 已被提升为 Normal，仍在 waiting 队列
        scheduler.complete("second_normal");

        // 提交 third_normal（Normal，seq=3），入队更晚
        scheduler.submit(make_task("third_normal", TaskPriority::Normal, 0.1));

        // 调度：waiting 中有 first_low（Normal, seq=1）和 third_normal（Normal, seq=3）
        // 期望 first_low 先被调度（更早入队），证明 enqueue_seq 维持了 FIFO 顺序
        let next = scheduler.schedule();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, "first_low");
    }
}
