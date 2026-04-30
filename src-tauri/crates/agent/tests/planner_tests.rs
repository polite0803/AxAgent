use axagent_agent::hierarchical_planner::{
    HierarchicalPlanner, PlanBuilder, PlanStatus, TaskBuilder,
};
use serde_json::json;

fn make_task(desc: &str, action: &str) -> axagent_agent::hierarchical_planner::PlannedTask {
    TaskBuilder::new(desc, action).build()
}

fn make_phase(
    name: &str,
    desc: &str,
    deps: Vec<String>,
    tasks: Vec<axagent_agent::hierarchical_planner::PlannedTask>,
) -> axagent_agent::hierarchical_planner::Phase {
    axagent_agent::hierarchical_planner::Phase {
        id: format!("id_{}", name),
        name: name.to_string(),
        description: desc.to_string(),
        tasks,
        dependencies: deps,
        status: axagent_agent::hierarchical_planner::PhaseStatus::Pending,
    }
}

#[test]
fn test_create_plan() {
    let mut planner = HierarchicalPlanner::new();
    let phase = make_phase(
        "data_collection",
        "Gather data",
        vec![],
        vec![make_task("Search for information", "web_search")],
    );
    let plan = planner.create_plan("Test Goal", vec![phase]);
    assert_eq!(plan.goal, "Test Goal");
    assert_eq!(plan.phases.len(), 1);
    assert_eq!(plan.status, PlanStatus::Draft);
}

#[test]
fn test_get_plan() {
    let mut planner = HierarchicalPlanner::new();
    let phase = make_phase("p1", "Phase 1", vec![], vec![make_task("task", "action")]);
    planner.create_plan("Goal", vec![phase]);
    let plan = planner.get_plan();
    assert!(plan.is_some());
    assert_eq!(plan.unwrap().goal, "Goal");
}

#[test]
fn test_start_execution() {
    let mut planner = HierarchicalPlanner::new();
    let phase = make_phase("p1", "Phase 1", vec![], vec![make_task("task", "action")]);
    planner.create_plan("Execute Test", vec![phase]);
    let result = planner.start_execution();
    assert!(result.is_ok());
    assert_eq!(planner.get_plan().unwrap().status, PlanStatus::Executing);
}

#[test]
fn test_pause_and_resume() {
    let mut planner = HierarchicalPlanner::new();
    let phase = make_phase("p1", "Phase 1", vec![], vec![make_task("task", "action")]);
    planner.create_plan("Pause Test", vec![phase]);
    planner.start_execution().unwrap();

    planner.pause_execution().unwrap();
    assert_eq!(planner.get_plan().unwrap().status, PlanStatus::Paused);

    planner.resume_execution().unwrap();
    assert_eq!(planner.get_plan().unwrap().status, PlanStatus::Executing);
}

#[test]
fn test_cancel_execution() {
    let mut planner = HierarchicalPlanner::new();
    let phase = make_phase("p1", "Phase 1", vec![], vec![make_task("task", "action")]);
    planner.create_plan("Cancel Test", vec![phase]);
    planner.start_execution().unwrap();

    planner.cancel_execution().unwrap();
    assert_eq!(planner.get_plan().unwrap().status, PlanStatus::Cancelled);
}

#[test]
fn test_get_next_executable_tasks() {
    let mut planner = HierarchicalPlanner::new();
    let phase = make_phase(
        "p1",
        "Phase 1",
        vec![],
        vec![make_task("Search for information", "web_search")],
    );
    planner.create_plan("Tasks Test", vec![phase]);
    planner.start_execution().unwrap();

    let tasks = planner.get_next_executable_tasks();
    assert!(!tasks.is_empty());
    assert_eq!(tasks[0].description, "Search for information");
}

#[test]
fn test_mark_task_lifecycle() {
    let mut planner = HierarchicalPlanner::new();
    let phase = make_phase(
        "p1",
        "Phase 1",
        vec![],
        vec![make_task("test task", "action_type")],
    );
    planner.create_plan("Lifecycle Test", vec![phase]);
    planner.start_execution().unwrap();

    let task_id = {
        let tasks = planner.get_next_executable_tasks();
        tasks[0].id.clone()
    };

    assert!(planner.mark_task_started(&task_id).is_ok());
    assert!(planner
        .mark_task_completed(&task_id, json!({"status": "ok"}))
        .is_ok());
}

#[test]
fn test_mark_task_failed() {
    let mut planner = HierarchicalPlanner::new();
    let task = TaskBuilder::new("failable task", "action")
        .with_max_retries(1)
        .build();
    let phase = make_phase("p1", "Phase 1", vec![], vec![task]);
    planner.create_plan("Fail Test", vec![phase]);
    planner.start_execution().unwrap();

    let task_id = {
        let tasks = planner.get_next_executable_tasks();
        tasks[0].id.clone()
    };

    planner.mark_task_started(&task_id).unwrap();
    assert!(planner
        .mark_task_failed(&task_id, "simulated error")
        .is_ok());
}

#[test]
fn test_get_progress() {
    let mut planner = HierarchicalPlanner::new();
    let phase = make_phase(
        "p1",
        "Phase 1",
        vec![],
        vec![make_task("task1", "action"), make_task("task2", "action")],
    );
    planner.create_plan("Progress Test", vec![phase]);

    let progress = planner.get_progress();
    assert_eq!(progress.total_tasks, 2);
    assert_eq!(progress.completed_tasks, 0);
}

#[test]
fn test_plan_builder() {
    let mut planner = HierarchicalPlanner::new();
    let plan = PlanBuilder::new("Build Test")
        .add_phase(
            "First Phase",
            "Description of first phase",
            vec![],
            vec![TaskBuilder::new("Do something", "action_type")
                .with_max_retries(3)
                .build()],
        )
        .build(&mut planner);

    assert_eq!(plan.goal, "Build Test");
    assert_eq!(plan.phases.len(), 1);
    assert_eq!(plan.phases[0].tasks.len(), 1);
    assert_eq!(plan.phases[0].tasks[0].max_retries, 3);
}

#[test]
fn test_task_builder_with_dependencies() {
    let task = TaskBuilder::new("Dependent task", "compute")
        .with_dependencies(vec!["task_1".to_string(), "task_2".to_string()])
        .with_role("analyzer")
        .build();

    assert_eq!(task.description, "Dependent task");
    assert_eq!(task.dependencies.len(), 2);
    assert_eq!(task.assigned_role, Some("analyzer".to_string()));
}

#[test]
fn test_planner_with_max_retries() {
    let planner = HierarchicalPlanner::new().with_max_retries(5);
    let mut planner = planner;
    let phase = make_phase("p1", "Phase 1", vec![], vec![make_task("task", "action")]);
    planner.create_plan("Retry Test", vec![phase]);
    let plan = planner.get_plan().unwrap();
    assert!(plan.phases[0].tasks[0].max_retries > 0);
}

#[test]
fn test_multiple_phases_with_dependencies() {
    let mut planner = HierarchicalPlanner::new();
    let plan = PlanBuilder::new("Multi Phase")
        .add_phase(
            "Phase 1",
            "First phase",
            vec![],
            vec![TaskBuilder::new("Task 1", "action").build()],
        )
        .add_phase(
            "Phase 2",
            "Second phase",
            vec!["id_Phase 1".to_string()],
            vec![TaskBuilder::new("Task 2", "action").build()],
        )
        .build(&mut planner);

    assert_eq!(plan.phases.len(), 2);
    assert!(plan.phases[0].dependencies.is_empty());
    assert!(!plan.phases[1].dependencies.is_empty());
}

#[test]
fn test_plan_status_transitions() {
    let mut planner = HierarchicalPlanner::new();
    let phase = make_phase("p1", "Phase 1", vec![], vec![make_task("task", "action")]);
    planner.create_plan("Status Test", vec![phase]);

    assert_eq!(planner.get_plan().unwrap().status, PlanStatus::Draft);
    planner.start_execution().unwrap();
    assert_eq!(planner.get_plan().unwrap().status, PlanStatus::Executing);
    planner.pause_execution().unwrap();
    assert_eq!(planner.get_plan().unwrap().status, PlanStatus::Paused);
    planner.resume_execution().unwrap();
    assert_eq!(planner.get_plan().unwrap().status, PlanStatus::Executing);
    planner.cancel_execution().unwrap();
    assert_eq!(planner.get_plan().unwrap().status, PlanStatus::Cancelled);
}

#[test]
fn test_task_builder_with_parameters() {
    let task = TaskBuilder::new("param task", "search")
        .with_parameters(json!({"query": "rust programming", "limit": 10}))
        .build();

    assert_eq!(task.description, "param task");
    assert_eq!(task.action_type, "search");
}
