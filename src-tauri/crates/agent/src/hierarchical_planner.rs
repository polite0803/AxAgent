use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub goal: String,
    pub phases: Vec<Phase>,
    pub status: PlanStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tasks: Vec<PlannedTask>,
    pub dependencies: Vec<String>,
    pub status: PhaseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedTask {
    pub id: String,
    pub description: String,
    pub action_type: String,
    pub parameters: serde_json::Value,
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub assigned_role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    Draft,
    Executing,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
    Blocked,
}

#[allow(clippy::type_complexity)]
pub struct HierarchicalPlanner {
    current_plan: Option<Plan>,
    max_retries: u32,
    on_task_start: Option<Box<dyn Fn(&str, &PlannedTask) + Send + Sync>>,
    on_task_complete: Option<Box<dyn Fn(&str, &PlannedTask) + Send + Sync>>,
    on_task_fail: Option<Box<dyn Fn(&str, &PlannedTask) + Send + Sync>>,
}

impl HierarchicalPlanner {
    pub fn new() -> Self {
        Self {
            current_plan: None,
            max_retries: 3,
            on_task_start: None,
            on_task_complete: None,
            on_task_fail: None,
        }
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn create_plan(&mut self, goal: &str, phases: Vec<Phase>) -> &Plan {
        let plan = Plan {
            id: generate_id(),
            goal: goal.to_string(),
            phases,
            status: PlanStatus::Draft,
            created_at: now_timestamp(),
            updated_at: now_timestamp(),
        };
        self.current_plan = Some(plan);
        self.current_plan.as_ref().unwrap()
    }

    pub fn get_plan(&self) -> Option<&Plan> {
        self.current_plan.as_ref()
    }

    pub fn get_plan_mut(&mut self) -> Option<&mut Plan> {
        self.current_plan.as_mut()
    }

    pub fn start_execution(&mut self) -> Result<(), String> {
        let plan = self.current_plan.as_mut().ok_or("No plan created")?;

        if plan.phases.is_empty() {
            return Err("Cannot start execution: plan has no phases".to_string());
        }

        plan.status = PlanStatus::Executing;

        if let Some(first_phase) = plan.phases.first_mut() {
            if first_phase.dependencies.is_empty() {
                first_phase.status = PhaseStatus::InProgress;
                for task in &mut first_phase.tasks {
                    if task.dependencies.is_empty() {
                        task.status = TaskStatus::Pending;
                    } else {
                        task.status = TaskStatus::Blocked;
                    }
                }
            }
        }

        plan.updated_at = now_timestamp();
        Ok(())
    }

    pub fn pause_execution(&mut self) -> Result<(), String> {
        let plan = self.current_plan.as_mut().ok_or("No plan created")?;
        if plan.status != PlanStatus::Executing {
            return Err("Plan is not executing".to_string());
        }
        plan.status = PlanStatus::Paused;
        plan.updated_at = now_timestamp();
        Ok(())
    }

    pub fn resume_execution(&mut self) -> Result<(), String> {
        let plan = self.current_plan.as_mut().ok_or("No plan created")?;
        if plan.status != PlanStatus::Paused {
            return Err("Plan is not paused".to_string());
        }
        plan.status = PlanStatus::Executing;
        plan.updated_at = now_timestamp();
        Ok(())
    }

    pub fn cancel_execution(&mut self) -> Result<(), String> {
        let plan = self.current_plan.as_mut().ok_or("No plan created")?;
        plan.status = PlanStatus::Cancelled;
        plan.updated_at = now_timestamp();
        Ok(())
    }

    pub fn get_next_executable_tasks(&self) -> Vec<&PlannedTask> {
        let plan = match &self.current_plan {
            Some(p) => p,
            None => return vec![],
        };

        if plan.status != PlanStatus::Executing {
            return vec![];
        }

        let mut executable = vec![];
        for phase in &plan.phases {
            if phase.status != PhaseStatus::InProgress {
                continue;
            }
            for task in &phase.tasks {
                if task.status != TaskStatus::Pending {
                    continue;
                }
                let deps_met = task.dependencies.iter().all(|dep_id| {
                    phase
                        .tasks
                        .iter()
                        .any(|t| t.id == *dep_id && t.status == TaskStatus::Completed)
                });
                if deps_met {
                    executable.push(task);
                }
            }
        }
        executable
    }

    pub fn mark_task_started(&mut self, task_id: &str) -> Result<(), String> {
        let plan = self.current_plan.as_mut().ok_or("No plan created")?;

        for phase in &mut plan.phases {
            for task in &mut phase.tasks {
                if task.id == task_id {
                    task.status = TaskStatus::InProgress;
                    if let Some(ref callback) = self.on_task_start {
                        callback(&plan.id, task);
                    }
                    plan.updated_at = now_timestamp();
                    return Ok(());
                }
            }
        }

        Err(format!("Task '{}' not found", task_id))
    }

    pub fn mark_task_completed(
        &mut self,
        task_id: &str,
        result: serde_json::Value,
    ) -> Result<(), String> {
        let _plan_id = {
            let plan = self.current_plan.as_mut().ok_or("No plan created")?;
            let mut found = false;
            let plan_id = plan.id.clone();

            for phase in &mut plan.phases {
                for task in &mut phase.tasks {
                    if task.id == task_id {
                        task.status = TaskStatus::Completed;
                        task.result = Some(result.clone());
                        if let Some(ref callback) = self.on_task_complete {
                            callback(&plan_id, task);
                        }
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }

            plan_id
        };

        self.unblock_dependent_tasks(task_id)?;
        self.check_phase_completion()?;
        self.check_plan_completion()?;
        self.advance_to_next_phase()?;

        if let Some(ref mut plan) = self.current_plan {
            plan.updated_at = now_timestamp();
        }
        Ok(())
    }

    pub fn mark_task_failed(&mut self, task_id: &str, error: &str) -> Result<(), String> {
        {
            let plan = self.current_plan.as_mut().ok_or("No plan created")?;
            let mut found = false;
            let plan_id = plan.id.clone();

            for phase in &mut plan.phases {
                for task in &mut phase.tasks {
                    if task.id == task_id {
                        task.retry_count += 1;
                        task.error = Some(error.to_string());

                        if task.retry_count >= task.max_retries {
                            task.status = TaskStatus::Failed;
                            if let Some(ref callback) = self.on_task_fail {
                                callback(&plan_id, task);
                            }
                        } else {
                            task.status = TaskStatus::Pending;
                        }
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }
        }

        self.check_phase_completion()?;
        if let Some(ref mut plan) = self.current_plan {
            plan.updated_at = now_timestamp();
        }
        Ok(())
    }

    pub fn get_progress(&self) -> PlanProgress {
        let plan = match &self.current_plan {
            Some(p) => p,
            None => {
                return PlanProgress {
                    total_phases: 0,
                    completed_phases: 0,
                    total_tasks: 0,
                    completed_tasks: 0,
                    failed_tasks: 0,
                    in_progress_tasks: 0,
                    pending_tasks: 0,
                    percentage: 0.0,
                }
            },
        };

        let total_phases = plan.phases.len();
        let completed_phases = plan
            .phases
            .iter()
            .filter(|p| p.status == PhaseStatus::Completed)
            .count();

        let total_tasks: usize = plan.phases.iter().map(|p| p.tasks.len()).sum();
        let completed_tasks: usize = plan
            .phases
            .iter()
            .flat_map(|p| p.tasks.iter())
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let failed_tasks: usize = plan
            .phases
            .iter()
            .flat_map(|p| p.tasks.iter())
            .filter(|t| t.status == TaskStatus::Failed)
            .count();
        let in_progress_tasks: usize = plan
            .phases
            .iter()
            .flat_map(|p| p.tasks.iter())
            .filter(|t| t.status == TaskStatus::InProgress)
            .count();
        let pending_tasks: usize = plan
            .phases
            .iter()
            .flat_map(|p| p.tasks.iter())
            .filter(|t| t.status == TaskStatus::Pending)
            .count();

        let percentage = if total_tasks > 0 {
            (completed_tasks as f64 / total_tasks as f64) * 100.0
        } else {
            0.0
        };

        PlanProgress {
            total_phases,
            completed_phases,
            total_tasks,
            completed_tasks,
            failed_tasks,
            in_progress_tasks,
            pending_tasks,
            percentage,
        }
    }

    fn unblock_dependent_tasks(&mut self, completed_task_id: &str) -> Result<(), String> {
        {
            let plan = self.current_plan.as_mut().ok_or("No plan created")?;
            let completed_id = completed_task_id.to_string();

            for phase in &mut plan.phases {
                let phase_tasks = phase.tasks.clone();
                for task in &mut phase.tasks {
                    if task.status == TaskStatus::Blocked
                        && task.dependencies.contains(&completed_id)
                    {
                        let all_deps_met = task.dependencies.iter().all(|dep_id| {
                            phase_tasks
                                .iter()
                                .any(|t| t.id == *dep_id && t.status == TaskStatus::Completed)
                        });
                        if all_deps_met {
                            task.status = TaskStatus::Pending;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn check_phase_completion(&mut self) -> Result<(), String> {
        let plan = self.current_plan.as_mut().ok_or("No plan created")?;

        for phase in &mut plan.phases {
            if phase.status != PhaseStatus::InProgress {
                continue;
            }

            let all_completed = phase
                .tasks
                .iter()
                .all(|t| t.status == TaskStatus::Completed);
            let any_failed = phase.tasks.iter().any(|t| t.status == TaskStatus::Failed);

            if all_completed {
                phase.status = PhaseStatus::Completed;
            } else if any_failed {
                let all_done = phase
                    .tasks
                    .iter()
                    .all(|t| t.status == TaskStatus::Completed || t.status == TaskStatus::Failed);
                if all_done {
                    phase.status = PhaseStatus::Failed;
                }
            }
        }

        Ok(())
    }

    fn check_plan_completion(&mut self) -> Result<(), String> {
        let plan = self.current_plan.as_mut().ok_or("No plan created")?;

        let all_completed = plan
            .phases
            .iter()
            .all(|p| p.status == PhaseStatus::Completed);
        let any_failed = plan.phases.iter().any(|p| p.status == PhaseStatus::Failed);

        if all_completed {
            plan.status = PlanStatus::Completed;
        } else if any_failed {
            let all_done = plan.phases.iter().all(|p| {
                p.status == PhaseStatus::Completed
                    || p.status == PhaseStatus::Failed
                    || p.status == PhaseStatus::Skipped
            });
            if all_done {
                plan.status = PlanStatus::Failed;
            }
        }

        Ok(())
    }

    fn advance_to_next_phase(&mut self) -> Result<(), String> {
        let plan = self.current_plan.as_mut().ok_or("No plan created")?;

        if plan.status != PlanStatus::Executing {
            return Ok(());
        }

        let completed_phase_ids: Vec<String> = plan
            .phases
            .iter()
            .filter(|p| p.status == PhaseStatus::Completed)
            .map(|p| p.id.clone())
            .collect();

        for phase in &mut plan.phases {
            if phase.status != PhaseStatus::Pending {
                continue;
            }

            let deps_met = phase
                .dependencies
                .iter()
                .all(|dep| completed_phase_ids.contains(dep));

            if deps_met {
                phase.status = PhaseStatus::InProgress;
                for task in &mut phase.tasks {
                    if task.dependencies.is_empty() {
                        task.status = TaskStatus::Pending;
                    } else {
                        task.status = TaskStatus::Blocked;
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanProgress {
    pub total_phases: usize,
    pub completed_phases: usize,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub in_progress_tasks: usize,
    pub pending_tasks: usize,
    pub percentage: f64,
}

pub struct PlanBuilder {
    goal: String,
    phases: Vec<Phase>,
}

impl PlanBuilder {
    pub fn new(goal: &str) -> Self {
        Self {
            goal: goal.to_string(),
            phases: Vec::new(),
        }
    }

    pub fn add_phase(
        mut self,
        name: &str,
        description: &str,
        dependencies: Vec<String>,
        tasks: Vec<PlannedTask>,
    ) -> Self {
        self.phases.push(Phase {
            id: generate_id(),
            name: name.to_string(),
            description: description.to_string(),
            tasks,
            dependencies,
            status: PhaseStatus::Pending,
        });
        self
    }

    pub fn build(self, planner: &mut HierarchicalPlanner) -> &Plan {
        planner.create_plan(&self.goal, self.phases)
    }
}

pub struct TaskBuilder {
    description: String,
    action_type: String,
    parameters: serde_json::Value,
    dependencies: Vec<String>,
    max_retries: u32,
    assigned_role: Option<String>,
}

impl TaskBuilder {
    pub fn new(description: &str, action_type: &str) -> Self {
        Self {
            description: description.to_string(),
            action_type: action_type.to_string(),
            parameters: serde_json::json!({}),
            dependencies: Vec::new(),
            max_retries: 3,
            assigned_role: None,
        }
    }

    pub fn with_parameters(mut self, params: serde_json::Value) -> Self {
        self.parameters = params;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn with_role(mut self, role: &str) -> Self {
        self.assigned_role = Some(role.to_string());
        self
    }

    pub fn build(self) -> PlannedTask {
        PlannedTask {
            id: generate_id(),
            description: self.description,
            action_type: self.action_type,
            parameters: self.parameters,
            dependencies: self.dependencies,
            status: TaskStatus::Pending,
            result: None,
            error: None,
            retry_count: 0,
            max_retries: self.max_retries,
            assigned_role: self.assigned_role,
        }
    }
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

impl Default for HierarchicalPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_plan() {
        let mut planner = HierarchicalPlanner::new();
        let plan = planner.create_plan(
            "Build a REST API",
            vec![Phase {
                id: "phase-1".to_string(),
                name: "Setup".to_string(),
                description: "Initialize project".to_string(),
                tasks: vec![],
                dependencies: vec![],
                status: PhaseStatus::Pending,
            }],
        );
        assert_eq!(plan.goal, "Build a REST API");
        assert_eq!(plan.status, PlanStatus::Draft);
        assert_eq!(plan.phases.len(), 1);
    }

    #[test]
    fn test_start_execution() {
        let mut planner = HierarchicalPlanner::new();
        planner.create_plan(
            "Test plan",
            vec![Phase {
                id: "p1".to_string(),
                name: "Phase 1".to_string(),
                description: "First phase".to_string(),
                tasks: vec![TaskBuilder::new("Task 1", "shell").build()],
                dependencies: vec![],
                status: PhaseStatus::Pending,
            }],
        );

        let result = planner.start_execution();
        assert!(result.is_ok());

        let plan = planner.get_plan().unwrap();
        assert_eq!(plan.status, PlanStatus::Executing);
        assert_eq!(plan.phases[0].status, PhaseStatus::InProgress);
    }

    #[test]
    fn test_task_completion_flow() {
        let mut planner = HierarchicalPlanner::new();
        let task = TaskBuilder::new("Task 1", "shell").build();
        let task_id = task.id.clone();

        planner.create_plan(
            "Test plan",
            vec![Phase {
                id: "p1".to_string(),
                name: "Phase 1".to_string(),
                description: "First phase".to_string(),
                tasks: vec![task],
                dependencies: vec![],
                status: PhaseStatus::Pending,
            }],
        );

        planner.start_execution().unwrap();
        planner.mark_task_started(&task_id).unwrap();
        planner
            .mark_task_completed(&task_id, serde_json::json!({"output": "done"}))
            .unwrap();

        let plan = planner.get_plan().unwrap();
        assert_eq!(plan.phases[0].tasks[0].status, TaskStatus::Completed);
        assert_eq!(plan.phases[0].status, PhaseStatus::Completed);
        assert_eq!(plan.status, PlanStatus::Completed);
    }

    #[test]
    fn test_task_dependency_blocking() {
        let task1 = TaskBuilder::new("Task 1", "shell").build();
        let task1_id = task1.id.clone();
        let task2 = TaskBuilder::new("Task 2", "shell")
            .with_dependencies(vec![task1_id.clone()])
            .build();
        let task2_id = task2.id.clone();

        let mut planner = HierarchicalPlanner::new();
        planner.create_plan(
            "Test plan",
            vec![Phase {
                id: "p1".to_string(),
                name: "Phase 1".to_string(),
                description: "First phase".to_string(),
                tasks: vec![task1, task2],
                dependencies: vec![],
                status: PhaseStatus::Pending,
            }],
        );

        planner.start_execution().unwrap();

        let next = planner.get_next_executable_tasks();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, task1_id);

        planner.mark_task_started(&task1_id).unwrap();
        planner
            .mark_task_completed(&task1_id, serde_json::json!({}))
            .unwrap();

        let next = planner.get_next_executable_tasks();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, task2_id);
    }

    #[test]
    fn test_progress_tracking() {
        let mut planner = HierarchicalPlanner::new();
        let task1 = TaskBuilder::new("Task 1", "shell").build();
        let task1_id = task1.id.clone();
        let task2 = TaskBuilder::new("Task 2", "shell").build();

        planner.create_plan(
            "Test plan",
            vec![Phase {
                id: "p1".to_string(),
                name: "Phase 1".to_string(),
                description: "First phase".to_string(),
                tasks: vec![task1, task2],
                dependencies: vec![],
                status: PhaseStatus::Pending,
            }],
        );

        planner.start_execution().unwrap();
        let progress = planner.get_progress();
        assert_eq!(progress.total_tasks, 2);
        assert_eq!(progress.pending_tasks, 2);
        assert_eq!(progress.percentage, 0.0);

        planner.mark_task_started(&task1_id).unwrap();
        planner
            .mark_task_completed(&task1_id, serde_json::json!({}))
            .unwrap();

        let progress = planner.get_progress();
        assert_eq!(progress.completed_tasks, 1);
    }

    #[test]
    fn test_task_retry_on_failure() {
        let task = TaskBuilder::new("Flaky task", "shell")
            .with_max_retries(2)
            .build();
        let task_id = task.id.clone();

        let mut planner = HierarchicalPlanner::new();
        planner.create_plan(
            "Test plan",
            vec![Phase {
                id: "p1".to_string(),
                name: "Phase 1".to_string(),
                description: "First phase".to_string(),
                tasks: vec![task],
                dependencies: vec![],
                status: PhaseStatus::Pending,
            }],
        );

        planner.start_execution().unwrap();
        planner.mark_task_started(&task_id).unwrap();

        planner.mark_task_failed(&task_id, "timeout").unwrap();
        let plan = planner.get_plan().unwrap();
        assert_eq!(plan.phases[0].tasks[0].status, TaskStatus::Pending);
        assert_eq!(plan.phases[0].tasks[0].retry_count, 1);

        planner.mark_task_started(&task_id).unwrap();
        planner.mark_task_failed(&task_id, "timeout again").unwrap();
        let plan = planner.get_plan().unwrap();
        assert_eq!(plan.phases[0].tasks[0].status, TaskStatus::Failed);
    }
}
