use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    ToolCall,
    Reasoning,
    Query,
    Validation,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::ToolCall => "tool_call",
            TaskType::Reasoning => "reasoning",
            TaskType::Query => "query",
            TaskType::Validation => "validation",
        }
    }
}

impl std::str::FromStr for TaskType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tool_call" => Ok(TaskType::ToolCall),
            "reasoning" => Ok(TaskType::Reasoning),
            "query" => Ok(TaskType::Query),
            "validation" => Ok(TaskType::Validation),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: String,
    pub description: String,
    pub task_type: TaskType,
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl TaskNode {
    pub fn new(id: impl Into<String>, description: impl Into<String>, task_type: TaskType) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            task_type,
            dependencies: Vec::new(),
            status: TaskStatus::Pending,
            result: None,
            error: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            started_at: None,
            completed_at: None,
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    /// 将任务从 `Pending` 推进到 `Running`，由调度器在派发时调用。
    ///
    /// 防御性检查：仅允许在 `Pending` 状态启动；若任务已经处于 `Running` / `Completed` /
    /// `Failed` / `Skipped` 中任一状态再调用 `start()`，说明调度器出现了重入或状态机被绕过，
    /// 直接 panic 以暴露问题，而不是默默覆盖已有状态。
    pub fn start(&mut self) {
        assert!(
            self.status == TaskStatus::Pending,
            "TaskNode::start() 只能在 Pending 状态调用，当前状态: {:?} (id={})",
            self.status,
            self.id
        );
        self.status = TaskStatus::Running;
        self.started_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn complete(&mut self, result: serde_json::Value) {
        self.status = TaskStatus::Completed;
        self.result = Some(result);
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = TaskStatus::Failed;
        self.error = Some(error.into());
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn skip(&mut self) {
        self.status = TaskStatus::Skipped;
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn is_ready(&self) -> bool {
        self.status == TaskStatus::Pending
    }

    /// 任务是否处于“已结束”状态（用于聚合查询 `all_complete()` 等）。
    ///
    /// 采用宽松语义：`Completed` / `Failed` / `Skipped` 都视为结束。
    /// 注意：这不代表“成功”，仅代表生命周期已终止。若需判断“成功完成”请使用 `is_completed()`。
    pub fn is_complete(&self) -> bool {
        matches!(self.status, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Skipped)
    }

    /// 任务是否“成功完成”（严格语义）。仅 `Completed` 返回 `true`。
    ///
    /// 用于依赖图就绪判定等需要“该任务确实有可用产物”的场景。
    /// 与宽松的 `is_complete()` 区分，避免 `Skipped` / `Failed` 任务被误判为已成功完成。
    pub fn is_completed(&self) -> bool {
        matches!(self.status, TaskStatus::Completed)
    }
}

/// 任务依赖解析策略，控制下游任务何时认为“依赖已满足”。
///
/// 背景：原先 `get_ready_tasks()` 使用宽松的 `is_complete()`，把 `Skipped` / `Failed`
/// 都视为下游可继续。这在某些场景下会导致下游拿不到应有输入（缺陷 2.5）。
/// 通过 `DependencyPolicy` 显式区分三种策略，调用方按业务需要选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DependencyPolicy {
    /// 严格：仅当所有依赖都 `Completed` 时才认为下游可继续。
    /// 适用于流水线类业务，依赖任务的产物必须真实存在。
    Complete,
    /// 兼容策略（默认）：`Completed` 或 `Skipped` 视为依赖满足。
    /// 保留修复前的语义，避免破坏现有 API 与调度器预期。
    #[default]
    CompleteOrSkipped,
    /// 宽松：依赖任务进入任意“已结束”状态（`Completed` / `Failed` / `Skipped`）
    /// 都视为已解决。适用于容错场景：依赖失败也要继续推进后续任务。
    AnyResolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGraph {
    pub tasks: Vec<TaskNode>,
    pub parallel_groups: Vec<Vec<String>>,
    /// 依赖解析策略，默认 `CompleteOrSkipped`（与修复前行为一致）。
    pub dependency_policy: DependencyPolicy,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            parallel_groups: Vec::new(),
            dependency_policy: DependencyPolicy::default(),
        }
    }

    /// 设置依赖解析策略（builder 模式）。
    pub fn with_dependency_policy(mut self, policy: DependencyPolicy) -> Self {
        self.dependency_policy = policy;
        self
    }

    pub fn add_task(&mut self, task: TaskNode) {
        self.tasks.push(task);
    }

    pub fn get_task(&self, id: &str) -> Option<&TaskNode> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut TaskNode> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn get_ready_tasks(&self) -> Vec<&TaskNode> {
        // 委托给 dependencies_resolved()，由 dependency_policy 决定就绪语义
        // （修复 2.5：之前用宽松 is_complete() 会把 Skipped/Failed 当作就绪）
        self.tasks
            .iter()
            .filter(|t| t.is_ready() && self.dependencies_resolved(&t.id))
            .collect()
    }

    pub fn all_complete(&self) -> bool {
        self.tasks.iter().all(|t| t.is_complete())
    }

    pub fn has_failures(&self) -> bool {
        self.tasks.iter().any(|t| t.status == TaskStatus::Failed)
    }

    /// 计算任务图整体完成度（百分比，0.0–100.0）。
    ///
    /// 空任务图视为“未开始”，返回 `0.0`（修复 2.10：原实现返回 `100.0` 容易让上层误判
    /// “全部完成”进而跳过执行或误报进度）。
    pub fn completion_percentage(&self) -> f32 {
        if self.tasks.is_empty() {
            return 0.0;
        }
        let completed = self.tasks.iter().filter(|t| t.is_complete()).count() as f32;
        (completed / self.tasks.len() as f32) * 100.0
    }

    /// Kahn 算法 + Tarjan SCC 精确环检测（修复 1.2）。
    ///
    /// 复杂度：`O(V + E)` 构建反向邻接表后，每轮取入度为 0 的节点并仅遍历其下游，
    /// 不再 `task.dependencies.contains(task_id)` 这种 O(n) 查找，也不再对全图做线性扫描。
    ///
    /// 环检测：Kahn 终止后剩余未访问节点包含两类 —— 真正在环中的节点 + 仅在环下游的节点。
    /// 本方法用 Tarjan 强连通分量算法在剩余子图上求 SCC，规模 > 1 的 SCC（或带自环的单元 SCC）
    /// 才被认定为“环节点”，最终仅返回这部分精确的环节点集合。
    pub fn topological_sort(&self) -> Result<Vec<Vec<String>>, TopologicalSortError> {
        let mut result: Vec<Vec<String>> = Vec::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut visited: HashSet<String> = HashSet::new();
        // 反向邻接表：task_id -> 直接依赖该任务的下游任务集合
        let mut reverse_adj: HashMap<String, HashSet<String>> = HashMap::new();
        // 依赖列表：task_id -> 依赖的任务 id 列表（用于 SCC 计算）
        let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();

        for task in &self.tasks {
            in_degree.insert(task.id.clone(), task.dependencies.len());
            dependencies.insert(task.id.clone(), task.dependencies.clone());
            for dep in &task.dependencies {
                reverse_adj
                    .entry(dep.clone())
                    .or_default()
                    .insert(task.id.clone());
            }
        }

        // Kahn 主循环：每轮取所有入度为 0 且未访问的节点作为一个并行批
        while visited.len() < self.tasks.len() {
            let batch: Vec<String> = self
                .tasks
                .iter()
                .filter(|t| {
                    !visited.contains(&t.id) && in_degree.get(&t.id).copied().unwrap_or(0) == 0
                })
                .map(|t| t.id.clone())
                .collect();

            if batch.is_empty() {
                // 剩余未访问节点 → 用 Tarjan 找真正在环中的节点
                let remaining: HashSet<String> = self
                    .tasks
                    .iter()
                    .filter(|t| !visited.contains(&t.id))
                    .map(|t| t.id.clone())
                    .collect();

                let cycle_nodes = find_cycle_nodes_via_tarjan(&remaining, &dependencies);
                let mut cycle_vec: Vec<String> = cycle_nodes.into_iter().collect();
                cycle_vec.sort();
                return Err(TopologicalSortError::CircularDependency(cycle_vec));
            }

            result.push(batch.clone());
            for task_id in &batch {
                visited.insert(task_id.clone());
                // 仅遍历当前批节点的下游（哈希集合查表 O(1)）
                if let Some(downstream) = reverse_adj.get(task_id) {
                    for next in downstream {
                        if let Some(degree) = in_degree.get_mut(next) {
                            *degree -= 1;
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// 严格判定：所有依赖任务都已 `Completed`。
    ///
    /// 与 `dependencies_resolved()` 的区别：本方法使用严格 `is_completed()` 语义，
    /// 不受 `dependency_policy` 影响；适合上游“必须真实成功”才能继续的场景。
    pub fn dependencies_ready(&self, task_id: &str) -> bool {
        if let Some(task) = self.get_task(task_id) {
            task.dependencies.iter().all(|dep| self.is_completed(dep))
        } else {
            false
        }
    }

    /// 按图级 `dependency_policy` 判定指定任务的依赖是否已“解决”。
    ///
    /// 策略：
    /// - `Complete`         → 所有依赖 `Completed`
    /// - `CompleteOrSkipped`→ 所有依赖 `Completed` 或 `Skipped`（默认）
    /// - `AnyResolved`      → 所有依赖处于 `Completed` / `Failed` / `Skipped` 任意终态
    ///
    /// 若任务不存在或存在缺失依赖（依赖项未注册到图中），返回 `false`。
    pub fn dependencies_resolved(&self, task_id: &str) -> bool {
        let Some(task) = self.get_task(task_id) else {
            return false;
        };
        task.dependencies.iter().all(|dep_id| {
            self.get_task(dep_id)
                .map_or(false, |dep| match self.dependency_policy {
                    DependencyPolicy::Complete => dep.is_completed(),
                    DependencyPolicy::CompleteOrSkipped => {
                        matches!(dep.status, TaskStatus::Completed | TaskStatus::Skipped)
                    },
                    DependencyPolicy::AnyResolved => dep.is_complete(),
                })
        })
    }

    /// 严格判定任务是否“成功完成”（仅 `Completed` 状态返回 `true`）。
    /// 任务不存在时返回 `false`。
    pub fn is_completed(&self, task_id: &str) -> bool {
        self.get_task(task_id).is_some_and(|t| t.is_completed())
    }

    pub fn get_failed_task_ids(&self) -> Vec<String> {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Failed)
            .map(|t| t.id.clone())
            .collect()
    }

    pub fn get_status_summary(&self) -> TaskStatusSummary {
        let total = self.tasks.len();
        let pending = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .count();
        let running = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .count();
        let completed = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let failed = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Failed)
            .count();
        let skipped = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Skipped)
            .count();

        TaskStatusSummary {
            total,
            pending,
            running,
            completed,
            failed,
            skipped,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskStatusSummary {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TopologicalSortError {
    #[error("Circular dependency detected involving tasks: {0:?}")]
    CircularDependency(Vec<String>),
}

/// 在 Kahn 终止后的剩余子图上跑 Tarjan 强连通分量算法，识别真正处于“环”中的节点。
///
/// 规则：
/// - 规模 > 1 的 SCC → 整 SCC 内全部节点都是环节点
/// - 规模 = 1 但存在自环（自己依赖自己）→ 该节点是环节点
/// - 规模 = 1 且无自环 → 仅是“环的下游”，不属于环
///
/// 依赖图按“任务 A 依赖任务 B”建立，DFS 时沿依赖边（A → B）走，使“回到自己”意味着 A 经过
/// 若干依赖最终能回到 A，即存在环。
fn find_cycle_nodes_via_tarjan(
    remaining: &HashSet<String>,
    dependencies: &HashMap<String, Vec<String>>,
) -> HashSet<String> {
    let mut index_counter: usize = 0;
    let mut index_map: HashMap<String, usize> = HashMap::new();
    let mut lowlinks: HashMap<String, usize> = HashMap::new();
    let mut on_stack: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = Vec::new();
    let mut sccs: Vec<Vec<String>> = Vec::new();

    // 稳定遍历顺序：先按 id 排好序，避免 SCC 顺序抖动
    let mut remaining_sorted: Vec<&String> = remaining.iter().collect();
    remaining_sorted.sort();

    for node in remaining_sorted {
        if !index_map.contains_key(node) {
            tarjan_strongconnect(
                node,
                remaining,
                dependencies,
                &mut index_map,
                &mut lowlinks,
                &mut on_stack,
                &mut stack,
                &mut index_counter,
                &mut sccs,
            );
        }
    }

    let mut cycle_nodes: HashSet<String> = HashSet::new();
    for scc in sccs {
        if scc.len() > 1 {
            for n in scc {
                cycle_nodes.insert(n);
            }
        } else if scc.len() == 1 {
            let n = &scc[0];
            if let Some(deps) = dependencies.get(n) {
                if deps.iter().any(|d| d == n) {
                    // 单元 SCC + 自环
                    cycle_nodes.insert(n.clone());
                }
            }
        }
    }
    cycle_nodes
}

/// Tarjan 强连通分量的递归 DFS（节点规模受任务图限制，不会出现栈溢出）。
#[allow(clippy::too_many_arguments)]
fn tarjan_strongconnect(
    v: &str,
    remaining: &HashSet<String>,
    dependencies: &HashMap<String, Vec<String>>,
    index_map: &mut HashMap<String, usize>,
    lowlinks: &mut HashMap<String, usize>,
    on_stack: &mut HashSet<String>,
    stack: &mut Vec<String>,
    index_counter: &mut usize,
    sccs: &mut Vec<Vec<String>>,
) {
    index_map.insert(v.to_string(), *index_counter);
    lowlinks.insert(v.to_string(), *index_counter);
    *index_counter += 1;
    stack.push(v.to_string());
    on_stack.insert(v.to_string());

    if let Some(deps) = dependencies.get(v) {
        for w in deps {
            if !remaining.contains(w) {
                continue;
            }
            if !index_map.contains_key(w) {
                tarjan_strongconnect(
                    w,
                    remaining,
                    dependencies,
                    index_map,
                    lowlinks,
                    on_stack,
                    stack,
                    index_counter,
                    sccs,
                );
                let low_w = lowlinks[w];
                let low_v = lowlinks[v];
                lowlinks.insert(v.to_string(), low_v.min(low_w));
            } else if on_stack.contains(w) {
                let idx_w = index_map[w];
                let low_v = lowlinks[v];
                lowlinks.insert(v.to_string(), low_v.min(idx_w));
            }
        }
    }

    if lowlinks[v] == index_map[v] {
        let mut scc: Vec<String> = Vec::new();
        loop {
            let w = stack.pop().expect("Tarjan 栈不应为空");
            on_stack.remove(&w);
            scc.push(w.clone());
            if w == v {
                break;
            }
        }
        sccs.push(scc);
    }
}

impl Default for TaskGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_as_str() {
        assert_eq!(TaskType::ToolCall.as_str(), "tool_call");
        assert_eq!(TaskType::Reasoning.as_str(), "reasoning");
        assert_eq!(TaskType::Query.as_str(), "query");
        assert_eq!(TaskType::Validation.as_str(), "validation");
    }

    #[test]
    fn test_task_type_from_str() {
        assert_eq!("tool_call".parse::<TaskType>().ok(), Some(TaskType::ToolCall));
        assert_eq!("reasoning".parse::<TaskType>().ok(), Some(TaskType::Reasoning));
        assert_eq!("query".parse::<TaskType>().ok(), Some(TaskType::Query));
        assert_eq!("validation".parse::<TaskType>().ok(), Some(TaskType::Validation));
        assert_eq!("unknown".parse::<TaskType>().ok(), None);
    }

    #[test]
    fn test_task_status_as_str() {
        assert_eq!(TaskStatus::Pending.as_str(), "pending");
        assert_eq!(TaskStatus::Running.as_str(), "running");
        assert_eq!(TaskStatus::Completed.as_str(), "completed");
        assert_eq!(TaskStatus::Failed.as_str(), "failed");
        assert_eq!(TaskStatus::Skipped.as_str(), "skipped");
    }

    #[test]
    fn test_task_node_new() {
        let node = TaskNode::new("t1", "Test task", TaskType::ToolCall);
        assert_eq!(node.id, "t1");
        assert_eq!(node.description, "Test task");
        assert_eq!(node.task_type, TaskType::ToolCall);
        assert!(node.dependencies.is_empty());
        assert_eq!(node.status, TaskStatus::Pending);
        assert!(node.result.is_none());
        assert!(node.error.is_none());
    }

    #[test]
    fn test_task_node_with_dependencies() {
        let node = TaskNode::new("t2", "Task 2", TaskType::Reasoning)
            .with_dependencies(vec!["t1".to_string()]);
        assert_eq!(node.dependencies.len(), 1);
        assert_eq!(node.dependencies[0], "t1");
    }

    #[test]
    fn test_task_node_start() {
        let mut node = TaskNode::new("t1", "Task", TaskType::Query);
        node.start();
        assert_eq!(node.status, TaskStatus::Running);
        assert!(node.started_at.is_some());
    }

    #[test]
    fn test_task_node_complete() {
        let mut node = TaskNode::new("t1", "Task", TaskType::Query);
        node.start();
        node.complete(serde_json::json!("result"));
        assert_eq!(node.status, TaskStatus::Completed);
        assert!(node.result.is_some());
        assert!(node.completed_at.is_some());
    }

    #[test]
    fn test_task_node_fail() {
        let mut node = TaskNode::new("t1", "Task", TaskType::Query);
        node.start();
        node.fail("something went wrong");
        assert_eq!(node.status, TaskStatus::Failed);
        assert_eq!(node.error, Some("something went wrong".to_string()));
        assert!(node.completed_at.is_some());
    }

    #[test]
    fn test_task_node_skip() {
        let mut node = TaskNode::new("t1", "Task", TaskType::Query);
        node.skip();
        assert_eq!(node.status, TaskStatus::Skipped);
        assert!(node.completed_at.is_some());
    }

    #[test]
    fn test_task_node_is_ready() {
        let node = TaskNode::new("t1", "Task", TaskType::Query);
        assert!(node.is_ready());
        let mut node2 = TaskNode::new("t2", "Task", TaskType::Query);
        node2.start();
        assert!(!node2.is_ready());
    }

    #[test]
    fn test_task_node_is_complete() {
        let mut node = TaskNode::new("t1", "Task", TaskType::Query);
        assert!(!node.is_complete());
        node.complete(serde_json::json!(42));
        assert!(node.is_complete());
    }

    #[test]
    fn test_task_graph_new() {
        let graph = TaskGraph::new();
        assert!(graph.tasks.is_empty());
        assert!(graph.parallel_groups.is_empty());
    }

    #[test]
    fn test_task_graph_add_and_get_task() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall));
        graph.add_task(TaskNode::new("t2", "Task 2", TaskType::Reasoning));
        assert_eq!(graph.tasks.len(), 2);
        assert!(graph.get_task("t1").is_some());
        assert!(graph.get_task("t2").is_some());
        assert!(graph.get_task("t3").is_none());
    }

    #[test]
    fn test_task_graph_get_task_mut() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall));
        if let Some(task) = graph.get_task_mut("t1") {
            task.start();
        }
        assert_eq!(graph.get_task("t1").unwrap().status, TaskStatus::Running);
    }

    #[test]
    fn test_task_graph_get_ready_tasks() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall));
        graph.add_task(
            TaskNode::new("t2", "Task 2", TaskType::Reasoning)
                .with_dependencies(vec!["t1".to_string()]),
        );
        let ready = graph.get_ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "t1");
    }

    #[test]
    fn test_task_graph_get_ready_tasks_with_completed_deps() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.complete(serde_json::json!("done"));
        graph.add_task(t1);
        graph.add_task(
            TaskNode::new("t2", "Task 2", TaskType::Reasoning)
                .with_dependencies(vec!["t1".to_string()]),
        );
        let ready = graph.get_ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "t2");
    }

    #[test]
    fn test_task_graph_all_complete() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.complete(serde_json::json!("done"));
        graph.add_task(t1);
        assert!(graph.all_complete());
    }

    #[test]
    fn test_task_graph_not_all_complete() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall));
        assert!(!graph.all_complete());
    }

    #[test]
    fn test_task_graph_has_failures() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.fail("error");
        graph.add_task(t1);
        assert!(graph.has_failures());
    }

    #[test]
    fn test_task_graph_no_failures() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall));
        assert!(!graph.has_failures());
    }

    #[test]
    fn test_task_graph_completion_percentage_empty() {
        // 空任务图视为未开始，返回 0.0（修复 2.10：原实现错误返回 100.0）
        let graph = TaskGraph::new();
        assert_eq!(graph.completion_percentage(), 0.0);
    }

    #[test]
    fn test_task_graph_completion_percentage() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.complete(serde_json::json!("done"));
        graph.add_task(t1);
        graph.add_task(TaskNode::new("t2", "Task 2", TaskType::Reasoning));
        assert_eq!(graph.completion_percentage(), 50.0);
    }

    #[test]
    fn test_task_graph_topological_sort_simple() {
        let mut graph = TaskGraph::new();
        graph.add_task(TaskNode::new("t1", "Task 1", TaskType::ToolCall));
        graph.add_task(
            TaskNode::new("t2", "Task 2", TaskType::Reasoning)
                .with_dependencies(vec!["t1".to_string()]),
        );
        let result = graph.topological_sort().unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].contains(&"t1".to_string()));
        assert!(result[1].contains(&"t2".to_string()));
    }

    #[test]
    fn test_task_graph_topological_sort_circular() {
        let mut graph = TaskGraph::new();
        graph.add_task(
            TaskNode::new("t1", "Task 1", TaskType::ToolCall)
                .with_dependencies(vec!["t2".to_string()]),
        );
        graph.add_task(
            TaskNode::new("t2", "Task 2", TaskType::Reasoning)
                .with_dependencies(vec!["t1".to_string()]),
        );
        let result = graph.topological_sort();
        assert!(matches!(result, Err(TopologicalSortError::CircularDependency(_))));
    }

    #[test]
    fn test_task_graph_dependencies_ready() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.complete(serde_json::json!("done"));
        graph.add_task(t1);
        graph.add_task(
            TaskNode::new("t2", "Task 2", TaskType::Reasoning)
                .with_dependencies(vec!["t1".to_string()]),
        );
        assert!(graph.dependencies_ready("t2"));
        assert!(!graph.dependencies_ready("nonexistent"));
    }

    #[test]
    fn test_task_graph_is_completed() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.complete(serde_json::json!("done"));
        graph.add_task(t1);
        assert!(graph.is_completed("t1"));
        assert!(!graph.is_completed("nonexistent"));
    }

    #[test]
    fn test_task_graph_get_failed_task_ids() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.fail("error");
        graph.add_task(t1);
        graph.add_task(TaskNode::new("t2", "Task 2", TaskType::Reasoning));
        let failed = graph.get_failed_task_ids();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0], "t1");
    }

    #[test]
    fn test_task_graph_get_status_summary() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.complete(serde_json::json!("done"));
        let mut t2 = TaskNode::new("t2", "Task 2", TaskType::Reasoning);
        t2.fail("error");
        graph.add_task(t1);
        graph.add_task(t2);
        graph.add_task(TaskNode::new("t3", "Task 3", TaskType::Query));
        let summary = graph.get_status_summary();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.completed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.pending, 1);
    }

    #[test]
    fn test_topological_sort_error_display() {
        let err =
            TopologicalSortError::CircularDependency(vec!["t1".to_string(), "t2".to_string()]);
        let msg = format!("{}", err);
        assert!(msg.contains("Circular dependency"));
    }

    // ===== 修复 2.5：DependencyPolicy 与严格 is_completed 行为测试 =====

    /// 严格 is_completed 仅 Completed 视为成功，Skipped/Failed 不算。
    #[test]
    fn test_task_node_is_completed_strict() {
        let mut node = TaskNode::new("t1", "Task", TaskType::Query);
        // Pending → false
        assert!(!node.is_completed());
        // Skipped → false（严格）
        node.skip();
        assert!(!node.is_completed());
        assert!(node.is_complete(), "is_complete 宽松语义应包含 Skipped");

        // Failed → false（严格）
        let mut node2 = TaskNode::new("t2", "Task", TaskType::Query);
        node2.start();
        node2.fail("err");
        assert!(!node2.is_completed());
        assert!(node2.is_complete(), "is_complete 宽松语义应包含 Failed");

        // Completed → true
        let mut node3 = TaskNode::new("t3", "Task", TaskType::Query);
        node3.start();
        node3.complete(serde_json::json!("ok"));
        assert!(node3.is_completed());
        assert!(node3.is_complete());
    }

    /// start() 二次调用必须 panic（修复 2.4：防御性重入检测）。
    #[test]
    #[should_panic(expected = "TaskNode::start() 只能在 Pending 状态调用")]
    fn test_task_node_start_rejects_reentrant() {
        let mut node = TaskNode::new("t1", "Task", TaskType::Query);
        node.start();
        // 再次 start() 应 panic
        node.start();
    }

    /// TaskGraph::is_completed 严格语义：Skipped 任务返回 false。
    #[test]
    fn test_task_graph_is_completed_strict() {
        let mut graph = TaskGraph::new();
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.skip();
        graph.add_task(t1);
        assert!(!graph.is_completed("t1"), "严格语义下 Skipped 不算完成");
        // 宽松语义仍可由 TaskNode::is_complete() 体现
        assert!(graph.get_task("t1").unwrap().is_complete());
    }

    /// DependencyPolicy::Complete：仅 Completed 视为依赖满足。
    #[test]
    fn test_dependency_policy_complete_strict() {
        let mut graph = TaskGraph::new().with_dependency_policy(DependencyPolicy::Complete);
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.skip();
        graph.add_task(t1);
        graph.add_task(
            TaskNode::new("t2", "Task 2", TaskType::Reasoning)
                .with_dependencies(vec!["t1".to_string()]),
        );
        // 严格策略下 t1 是 Skipped，不算依赖满足
        assert!(!graph.dependencies_resolved("t2"));
        let ready = graph.get_ready_tasks();
        assert!(ready.is_empty());
    }

    /// DependencyPolicy::CompleteOrSkipped（默认）：Skipped 也算依赖满足。
    #[test]
    fn test_dependency_policy_complete_or_skipped_default() {
        let mut graph = TaskGraph::new();
        // 默认策略
        assert_eq!(graph.dependency_policy, DependencyPolicy::CompleteOrSkipped);

        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.skip();
        graph.add_task(t1);
        graph.add_task(
            TaskNode::new("t2", "Task 2", TaskType::Reasoning)
                .with_dependencies(vec!["t1".to_string()]),
        );
        assert!(graph.dependencies_resolved("t2"));
        let ready = graph.get_ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "t2");
    }

    /// DependencyPolicy::AnyResolved：Failed 任务也算依赖已解决。
    #[test]
    fn test_dependency_policy_any_resolved_includes_failed() {
        let mut graph = TaskGraph::new().with_dependency_policy(DependencyPolicy::AnyResolved);
        let mut t1 = TaskNode::new("t1", "Task 1", TaskType::ToolCall);
        t1.start();
        t1.fail("oops");
        graph.add_task(t1);
        graph.add_task(
            TaskNode::new("t2", "Task 2", TaskType::Reasoning)
                .with_dependencies(vec!["t1".to_string()]),
        );
        assert!(graph.dependencies_resolved("t2"));
    }

    /// with_dependency_policy builder 与缺省任务的依赖解析。
    #[test]
    fn test_dependency_policy_missing_dependency() {
        let mut graph = TaskGraph::new();
        graph.add_task(
            TaskNode::new("t1", "Task 1", TaskType::ToolCall)
                .with_dependencies(vec!["nonexistent".to_string()]),
        );
        // 依赖项不在图中 → 视为未解决
        assert!(!graph.dependencies_resolved("t1"));
        assert!(!graph.dependencies_ready("t1"));
    }

    // ===== 修复 1.2：精确环检测回归测试 =====

    /// 环节点必须被精确识别，下游孤立节点不应被误判为环节点。
    ///
    /// 场景：t1 ↔ t2 形成互依赖环，t3 仅依赖 t1。Kahn 终止时 t1/t2/t3 都未访问，
    /// 但精确环检测应只把 t1/t2 报告为环节点。
    #[test]
    fn test_topological_sort_cycle_excludes_downstream() {
        let mut graph = TaskGraph::new();
        graph.add_task(
            TaskNode::new("t1", "Task 1", TaskType::ToolCall)
                .with_dependencies(vec!["t2".to_string()]),
        );
        graph.add_task(
            TaskNode::new("t2", "Task 2", TaskType::Reasoning)
                .with_dependencies(vec!["t1".to_string()]),
        );
        graph.add_task(
            TaskNode::new("t3", "Task 3", TaskType::Query)
                .with_dependencies(vec!["t1".to_string()]),
        );
        let err = graph.topological_sort().unwrap_err();
        match err {
            TopologicalSortError::CircularDependency(nodes) => {
                assert!(nodes.contains(&"t1".to_string()));
                assert!(nodes.contains(&"t2".to_string()));
                assert!(
                    !nodes.contains(&"t3".to_string()),
                    "t3 仅是环的下游，不应被报告为环节点，实际: {nodes:?}"
                );
            },
        }
    }

    /// 自环应被检测出来。
    #[test]
    fn test_topological_sort_self_loop() {
        let mut graph = TaskGraph::new();
        graph.add_task(
            TaskNode::new("t1", "Task 1", TaskType::ToolCall)
                .with_dependencies(vec!["t1".to_string()]),
        );
        let err = graph.topological_sort().unwrap_err();
        assert!(matches!(err, TopologicalSortError::CircularDependency(_)));
    }

    /// 大型无环图（性能回归）：100 个节点链式依赖，O(V+E) 必须能跑完。
    #[test]
    fn test_topological_sort_large_chain() {
        let mut graph = TaskGraph::new();
        for i in 0..100 {
            let mut node = TaskNode::new(format!("t{i}"), "Task", TaskType::Query);
            if i > 0 {
                node = node.with_dependencies(vec![format!("t{}", i - 1)]);
            }
            graph.add_task(node);
        }
        let result = graph.topological_sort().unwrap();
        assert_eq!(result.len(), 100);
        assert_eq!(result[0], vec!["t0".to_string()]);
        assert_eq!(result[99], vec!["t99".to_string()]);
    }
}
