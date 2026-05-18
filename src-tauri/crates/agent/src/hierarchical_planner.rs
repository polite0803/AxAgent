use serde::{Deserialize, Serialize};

type TaskCallback = Box<dyn Fn(&str, &PlannedTask) + Send + Sync>;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanVersion {
    pub version: u32,
    pub plan: Plan,
    pub created_at: i64,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplanReason {
    StepFailed { task_id: String, error: String },
    NewDependencyDiscovered { task_id: String, dependency: String },
    GoalChanged { old_goal: String, new_goal: String },
    ResourceConstraint { constraint: String },
    ManualIntervention { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplanAction {
    Retry {
        task_id: String,
        modified_parameters: Option<serde_json::Value>,
    },
    Skip {
        task_id: String,
        reason: String,
    },
    Insert {
        phase_id: String,
        task: PlannedTask,
        position: usize,
    },
    Remove {
        task_id: String,
        reason: String,
    },
    Reorder {
        task_id: String,
        new_position: usize,
    },
    AddPhase {
        phase: Phase,
        position: usize,
    },
    ModifyTask {
        task_id: String,
        modifications: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanRecord {
    pub version: u32,
    pub timestamp: i64,
    pub reason: ReplanReason,
    pub actions: Vec<ReplanAction>,
    pub completed_steps: Vec<String>,
    pub failed_steps: Vec<String>,
    pub pending_steps: Vec<String>,
}

pub struct HierarchicalPlanner {
    current_plan: Option<Plan>,
    max_retries: u32,
    on_task_start: Option<TaskCallback>,
    on_task_complete: Option<TaskCallback>,
    on_task_fail: Option<TaskCallback>,
    plan_versions: Vec<PlanVersion>,
    replan_history: Vec<ReplanRecord>,
    current_version: u32,
}

impl HierarchicalPlanner {
    pub fn new() -> Self {
        Self {
            current_plan: None,
            max_retries: 3,
            on_task_start: None,
            on_task_complete: None,
            on_task_fail: None,
            plan_versions: Vec::new(),
            replan_history: Vec::new(),
            current_version: 0,
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
        let plan_clone = plan.clone();
        self.plan_versions.push(PlanVersion {
            version: 0,
            plan: plan_clone,
            created_at: now_timestamp(),
            description: "Initial plan".to_string(),
        });
        self.current_version = 0;
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

        if let Some(first_phase) = plan.phases.first_mut()
            && first_phase.dependencies.is_empty()
        {
            first_phase.status = PhaseStatus::InProgress;
            for task in &mut first_phase.tasks {
                if task.dependencies.is_empty() {
                    task.status = TaskStatus::Pending;
                } else {
                    task.status = TaskStatus::Blocked;
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
                };
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

    pub fn get_replan_history(&self) -> &[ReplanRecord] {
        &self.replan_history
    }

    pub fn get_plan_versions(&self) -> &[PlanVersion] {
        &self.plan_versions
    }

    pub fn replan(
        &mut self,
        reason: ReplanReason,
        actions: Vec<ReplanAction>,
    ) -> Result<ReplanRecord, String> {
        if self.current_plan.is_none() {
            return Err("No plan created".to_string());
        }

        let completed_steps = self.collect_completed_steps();
        let failed_steps = self.collect_failed_steps();
        let pending_steps = self.collect_pending_steps();

        let current_version = self.current_version;

        self.apply_replan_actions(&actions)?;

        if let Some(ref mut plan) = self.current_plan {
            plan.updated_at = now_timestamp();

            let plan_clone = plan.clone();
            let new_version = current_version + 1;
            self.plan_versions.push(PlanVersion {
                version: new_version,
                plan: plan_clone,
                created_at: now_timestamp(),
                description: format!("Replan: {:?}", reason),
            });
            self.current_version = new_version;

            let record = ReplanRecord {
                version: new_version,
                timestamp: now_timestamp(),
                reason: reason.clone(),
                actions,
                completed_steps,
                failed_steps,
                pending_steps,
            };

            self.replan_history.push(record.clone());

            Ok(record)
        } else {
            Err("No plan created".to_string())
        }
    }

    pub fn rollback(&mut self, target_version: u32) -> Result<&Plan, String> {
        let version_entry = self
            .plan_versions
            .iter()
            .find(|v| v.version == target_version)
            .ok_or_else(|| format!("Version {} not found", target_version))?;

        let restored_plan = version_entry.plan.clone();
        self.current_plan = Some(restored_plan);
        self.current_version = target_version;

        let plan = self.current_plan.as_mut().ok_or("No plan created")?;
        plan.updated_at = now_timestamp();

        Ok(plan)
    }

    pub fn get_completed_steps(&self) -> Vec<String> {
        self.collect_completed_steps()
    }

    pub fn get_failed_steps(&self) -> Vec<String> {
        self.collect_failed_steps()
    }

    pub fn get_pending_steps(&self) -> Vec<String> {
        self.collect_pending_steps()
    }

    fn collect_completed_steps(&self) -> Vec<String> {
        let plan = match &self.current_plan {
            Some(p) => p,
            None => return vec![],
        };

        plan.phases
            .iter()
            .flat_map(|phase| {
                phase
                    .tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Completed)
                    .map(|t| t.id.clone())
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn collect_failed_steps(&self) -> Vec<String> {
        let plan = match &self.current_plan {
            Some(p) => p,
            None => return vec![],
        };

        plan.phases
            .iter()
            .flat_map(|phase| {
                phase
                    .tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Failed)
                    .map(|t| t.id.clone())
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn collect_pending_steps(&self) -> Vec<String> {
        let plan = match &self.current_plan {
            Some(p) => p,
            None => return vec![],
        };

        plan.phases
            .iter()
            .flat_map(|phase| {
                phase
                    .tasks
                    .iter()
                    .filter(|t| {
                        t.status == TaskStatus::Pending
                            || t.status == TaskStatus::Blocked
                            || t.status == TaskStatus::InProgress
                    })
                    .map(|t| t.id.clone())
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn apply_replan_actions(&mut self, actions: &[ReplanAction]) -> Result<(), String> {
        let plan = self.current_plan.as_mut().ok_or("No plan created")?;

        for action in actions {
            match action {
                ReplanAction::Retry {
                    task_id,
                    modified_parameters,
                } => {
                    for phase in &mut plan.phases {
                        for task in &mut phase.tasks {
                            if task.id == *task_id {
                                task.status = TaskStatus::Pending;
                                task.retry_count = 0;
                                task.error = None;
                                if let Some(params) = modified_parameters {
                                    task.parameters = params.clone();
                                }
                                break;
                            }
                        }
                    }
                },
                ReplanAction::Skip { task_id, reason: _ } => {
                    for phase in &mut plan.phases {
                        for task in &mut phase.tasks {
                            if task.id == *task_id {
                                task.status = TaskStatus::Skipped;
                                break;
                            }
                        }
                    }
                },
                ReplanAction::Insert {
                    phase_id,
                    task,
                    position,
                } => {
                    for phase in &mut plan.phases {
                        if phase.id == *phase_id {
                            let pos = *position.min(&phase.tasks.len());
                            phase.tasks.insert(pos, task.clone());
                            break;
                        }
                    }
                },
                ReplanAction::Remove { task_id, reason: _ } => {
                    for phase in &mut plan.phases {
                        phase.tasks.retain(|t| t.id != *task_id);
                    }
                },
                ReplanAction::Reorder {
                    task_id,
                    new_position,
                } => {
                    for phase in &mut plan.phases {
                        if let Some(pos) = phase.tasks.iter().position(|t| t.id == *task_id) {
                            let task = phase.tasks.remove(pos);
                            let task_count = phase.tasks.len();
                            let new_pos = new_position.min(&task_count);
                            phase.tasks.insert(*new_pos, task);
                            break;
                        }
                    }
                },
                ReplanAction::AddPhase { phase, position } => {
                    let pos = *position.min(&plan.phases.len());
                    plan.phases.insert(pos, phase.clone());
                },
                ReplanAction::ModifyTask {
                    task_id,
                    modifications,
                } => {
                    for phase in &mut plan.phases {
                        for task in &mut phase.tasks {
                            if task.id == *task_id {
                                if let Some(desc) = modifications.get("description")
                                    && let Some(desc_str) = desc.as_str()
                                {
                                    task.description = desc_str.to_string();
                                }
                                if let Some(params) = modifications.get("parameters") {
                                    task.parameters = params.clone();
                                }
                                if let Some(retries) = modifications.get("max_retries")
                                    && let Some(retries_num) = retries.as_u64()
                                {
                                    task.max_retries = retries_num as u32;
                                }
                                if let Some(role) = modifications.get("assigned_role")
                                    && let Some(role_str) = role.as_str()
                                {
                                    task.assigned_role = Some(role_str.to_string());
                                }
                                if let Some(deps) = modifications.get("dependencies")
                                    && let Some(deps_arr) = deps.as_array()
                                {
                                    task.dependencies = deps_arr
                                        .iter()
                                        .filter_map(|d| d.as_str().map(String::from))
                                        .collect();
                                }
                                break;
                            }
                        }
                    }
                },
            }
        }

        Ok(())
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

    #[test]
    fn test_plan_version_tracking() {
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

        assert_eq!(planner.get_plan_versions().len(), 1);
        assert_eq!(planner.get_plan_versions()[0].version, 0);
        assert_eq!(planner.current_version, 0);
    }

    #[test]
    fn test_replan_retry_failed_step() {
        let mut planner = HierarchicalPlanner::new();
        let task = TaskBuilder::new("Flaky task", "shell")
            .with_max_retries(2)
            .build();
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
        planner.mark_task_failed(&task_id, "timeout").unwrap();
        planner.mark_task_started(&task_id).unwrap();
        planner.mark_task_failed(&task_id, "timeout again").unwrap();

        let failed = planner.get_failed_steps();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0], task_id);

        let reason = ReplanReason::StepFailed {
            task_id: task_id.clone(),
            error: "timeout again".to_string(),
        };

        let actions = vec![ReplanAction::Retry {
            task_id: task_id.clone(),
            modified_parameters: Some(serde_json::json!({"timeout_ms": 30000})),
        }];

        let record = planner.replan(reason, actions).unwrap();

        assert_eq!(record.version, 1);
        assert!(matches!(record.reason, ReplanReason::StepFailed { .. }));
        assert_eq!(record.completed_steps.len(), 0);
        assert_eq!(record.failed_steps.len(), 1);

        let plan = planner.get_plan().unwrap();
        let task = plan.phases[0]
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .unwrap();
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.retry_count, 0);
        assert_eq!(task.error, None);

        assert_eq!(planner.get_plan_versions().len(), 2);
        assert_eq!(planner.current_version, 1);
        assert_eq!(planner.get_replan_history().len(), 1);
    }

    #[test]
    fn test_replan_skip_impossible_step() {
        let task1 = TaskBuilder::new("Task 1", "shell").build();
        let task2 = TaskBuilder::new("Task 2", "shell").build();

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

        let reason = ReplanReason::ResourceConstraint {
            constraint: "External API rate limit exceeded".to_string(),
        };

        let task1_id = planner.get_plan().unwrap().phases[0].tasks[0].id.clone();
        let actions = vec![ReplanAction::Skip {
            task_id: task1_id.clone(),
            reason: "API rate limited, skipping for now".to_string(),
        }];

        planner.replan(reason, actions).unwrap();

        let plan = planner.get_plan().unwrap();
        let task = plan.phases[0]
            .tasks
            .iter()
            .find(|t| t.id == task1_id)
            .unwrap();
        assert_eq!(task.status, TaskStatus::Skipped);
    }

    #[test]
    fn test_replan_insert_new_step() {
        let task = TaskBuilder::new("Deploy app", "shell").build();

        let mut planner = HierarchicalPlanner::new();
        planner.create_plan(
            "Test plan",
            vec![Phase {
                id: "p1".to_string(),
                name: "Deploy".to_string(),
                description: "Deploy phase".to_string(),
                tasks: vec![task],
                dependencies: vec![],
                status: PhaseStatus::Pending,
            }],
        );

        let phase_id = planner.get_plan().unwrap().phases[0].id.clone();

        let new_task = TaskBuilder::new("Run migration", "shell")
            .with_dependencies(vec![])
            .build();

        let reason = ReplanReason::NewDependencyDiscovered {
            task_id: "deploy".to_string(),
            dependency: "database migration".to_string(),
        };

        let actions = vec![ReplanAction::Insert {
            phase_id: phase_id.clone(),
            task: new_task,
            position: 0,
        }];

        planner.replan(reason, actions).unwrap();

        let plan = planner.get_plan().unwrap();
        let phase = plan.phases.iter().find(|p| p.id == phase_id).unwrap();
        assert_eq!(phase.tasks.len(), 2);
        assert_eq!(phase.tasks[0].description, "Run migration");
        assert_eq!(phase.tasks[1].description, "Deploy app");
    }

    #[test]
    fn test_replan_add_phase() {
        let mut planner = HierarchicalPlanner::new();
        planner.create_plan(
            "Test plan",
            vec![Phase {
                id: "p1".to_string(),
                name: "Phase 1".to_string(),
                description: "First phase".to_string(),
                tasks: vec![],
                dependencies: vec![],
                status: PhaseStatus::Pending,
            }],
        );

        let new_phase = Phase {
            id: "p2".to_string(),
            name: "Phase 2".to_string(),
            description: "Inserted phase".to_string(),
            tasks: vec![TaskBuilder::new("New task", "shell").build()],
            dependencies: vec!["p1".to_string()],
            status: PhaseStatus::Pending,
        };

        let reason = ReplanReason::GoalChanged {
            old_goal: "Test plan".to_string(),
            new_goal: "Extended plan".to_string(),
        };

        let actions = vec![ReplanAction::AddPhase {
            phase: new_phase,
            position: 1,
        }];

        planner.replan(reason, actions).unwrap();

        let plan = planner.get_plan().unwrap();
        assert_eq!(plan.phases.len(), 2);
        assert_eq!(plan.phases[1].name, "Phase 2");
    }

    #[test]
    fn test_replan_modify_task() {
        let task = TaskBuilder::new("Task 1", "shell")
            .with_max_retries(1)
            .with_role("developer")
            .build();

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

        let task_id = planner.get_plan().unwrap().phases[0].tasks[0].id.clone();

        let reason = ReplanReason::ManualIntervention {
            reason: "Increase retries and change role".to_string(),
        };

        let modifications = serde_json::json!({
            "max_retries": 5,
            "assigned_role": "senior_developer",
            "description": "Updated task description"
        });

        let actions = vec![ReplanAction::ModifyTask {
            task_id: task_id.clone(),
            modifications,
        }];

        planner.replan(reason, actions).unwrap();

        let plan = planner.get_plan().unwrap();
        let task = plan.phases[0]
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .unwrap();
        assert_eq!(task.max_retries, 5);
        assert_eq!(task.assigned_role, Some("senior_developer".to_string()));
        assert_eq!(task.description, "Updated task description");
    }

    #[test]
    fn test_replan_remove_task() {
        let task1 = TaskBuilder::new("Task 1", "shell").build();
        let task2 = TaskBuilder::new("Task 2", "shell").build();

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

        let task1_id = planner.get_plan().unwrap().phases[0].tasks[0].id.clone();

        let reason = ReplanReason::StepFailed {
            task_id: task1_id.clone(),
            error: "Obsolete requirement".to_string(),
        };

        let actions = vec![ReplanAction::Remove {
            task_id: task1_id.clone(),
            reason: "No longer needed".to_string(),
        }];

        planner.replan(reason, actions).unwrap();

        let plan = planner.get_plan().unwrap();
        assert_eq!(plan.phases[0].tasks.len(), 1);
        assert!(plan.phases[0].tasks.iter().all(|t| t.id != task1_id));
    }

    #[test]
    fn test_replan_reorder_tasks() {
        let task1 = TaskBuilder::new("Task 1", "shell").build();
        let task2 = TaskBuilder::new("Task 2", "shell").build();

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

        let task2_id = planner.get_plan().unwrap().phases[0].tasks[1].id.clone();

        let reason = ReplanReason::NewDependencyDiscovered {
            task_id: task2_id.clone(),
            dependency: "task1 should run after task2".to_string(),
        };

        let actions = vec![ReplanAction::Reorder {
            task_id: task2_id.clone(),
            new_position: 0,
        }];

        planner.replan(reason, actions).unwrap();

        let plan = planner.get_plan().unwrap();
        assert_eq!(plan.phases[0].tasks[0].id, task2_id);
    }

    #[test]
    fn test_rollback_plan() {
        let mut planner = HierarchicalPlanner::new();
        let task = TaskBuilder::new("Task 1", "shell").build();
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

        let reason = ReplanReason::ManualIntervention {
            reason: "Test rollback".to_string(),
        };

        let task_id = planner.get_plan().unwrap().phases[0].tasks[0].id.clone();
        let actions = vec![ReplanAction::Remove {
            task_id: task_id.clone(),
            reason: "Testing".to_string(),
        }];

        planner.replan(reason, actions).unwrap();

        let plan = planner.get_plan().unwrap();
        assert_eq!(plan.phases[0].tasks.len(), 0);

        planner.rollback(0).unwrap();

        let plan = planner.get_plan().unwrap();
        assert_eq!(plan.phases[0].tasks.len(), 1);
        assert_eq!(planner.current_version, 0);
    }

    #[test]
    fn test_rollback_nonexistent_version() {
        let mut planner = HierarchicalPlanner::new();
        planner.create_plan(
            "Test plan",
            vec![Phase {
                id: "p1".to_string(),
                name: "Phase 1".to_string(),
                description: "First phase".to_string(),
                tasks: vec![],
                dependencies: vec![],
                status: PhaseStatus::Pending,
            }],
        );

        let result = planner.rollback(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_replan_history_accumulation() {
        let mut planner = HierarchicalPlanner::new();
        let task = TaskBuilder::new("Task 1", "shell").build();
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

        let task_id = planner.get_plan().unwrap().phases[0].tasks[0].id.clone();

        planner
            .replan(
                ReplanReason::StepFailed {
                    task_id: task_id.clone(),
                    error: "error1".to_string(),
                },
                vec![ReplanAction::Retry {
                    task_id: task_id.clone(),
                    modified_parameters: None,
                }],
            )
            .unwrap();

        planner
            .replan(
                ReplanReason::ManualIntervention {
                    reason: "manual".to_string(),
                },
                vec![ReplanAction::Skip {
                    task_id: task_id.clone(),
                    reason: "skip".to_string(),
                }],
            )
            .unwrap();

        assert_eq!(planner.get_replan_history().len(), 2);
        assert_eq!(planner.get_replan_history()[0].version, 1);
        assert_eq!(planner.get_replan_history()[1].version, 2);
    }

    #[test]
    fn test_get_step_status_methods() {
        let task1 = TaskBuilder::new("Task 1", "shell").build();
        let task2 = TaskBuilder::new("Task 2", "shell").build();

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

        let completed = planner.get_completed_steps();
        assert_eq!(completed.len(), 0);

        let failed = planner.get_failed_steps();
        assert_eq!(failed.len(), 0);

        let pending = planner.get_pending_steps();
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_multiple_replans_preserve_completed_work() {
        let task1 = TaskBuilder::new("Task 1", "shell").build();
        let task2 = TaskBuilder::new("Task 2", "shell").build();
        let task1_id = task1.id.clone();

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
        planner
            .mark_task_completed(&task1_id, serde_json::json!({"done": true}))
            .unwrap();

        let task2_id = planner.get_plan().unwrap().phases[0].tasks[1].id.clone();

        planner
            .replan(
                ReplanReason::StepFailed {
                    task_id: task2_id.clone(),
                    error: "failed".to_string(),
                },
                vec![ReplanAction::Retry {
                    task_id: task2_id.clone(),
                    modified_parameters: None,
                }],
            )
            .unwrap();

        let plan = planner.get_plan().unwrap();
        let t1 = plan.phases[0]
            .tasks
            .iter()
            .find(|t| t.id == task1_id)
            .unwrap();
        assert_eq!(t1.status, TaskStatus::Completed);
        assert_eq!(t1.result, Some(serde_json::json!({"done": true})));
    }
}
