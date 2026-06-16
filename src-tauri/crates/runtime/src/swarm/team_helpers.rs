// SPDX-License-Identifier: AGPL-3.0-only

//! Swarm 团队核心数据结构与逻辑
//!
//! 包含 Team（团队）、Teammate（队友）、TeamTask（团队任务）、
//! TeammateMessage（队友间消息）等核心类型，以及团队创建/解散/消息路由逻辑。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::constants::MAX_TEAM_MEMBERS;

// ── 队友状态 ──

/// 队友当前状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeammateStatus {
    /// 空闲，等待任务
    Idle,
    /// 忙碌，正在执行任务
    Busy,
    /// 离线
    Offline,
    /// 出错
    Error(String),
}

impl std::fmt::Display for TeammateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeammateStatus::Idle => write!(f, "Idle"),
            TeammateStatus::Busy => write!(f, "Busy"),
            TeammateStatus::Offline => write!(f, "Offline"),
            TeammateStatus::Error(e) => write!(f, "Error({})", e),
        }
    }
}

// ── 后端类型 ──

/// 队友进程后端类型
///
/// - `InProcess`: 同进程内运行（通过异步 task）
/// - `SubProcess`: 独立的子进程（通过 stdin/stdout JSON 行协议通信）
/// - `Tmux`: 通过 tmux 会话管理（仅 Unix）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendType {
    /// 同一进程内运行
    InProcess,
    /// 独立的子进程
    SubProcess,
    /// tmux 会话管理（仅 Unix）
    Tmux,
}

// ── 队友 ──

/// 单个队友信息
///
/// `process_pid` 记录子进程的 PID（仅在 SubProcess/Tmux 后端时有效），
/// 用于外部管理进程生命周期。实际的 `std::process::Child` 句柄由
/// `ProcessBackend` 持有。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Teammate {
    /// 队友唯一标识（格式: "name@team"）
    pub agent_id: String,
    /// 队友名称
    pub agent_name: String,
    /// 所属团队名
    pub team_name: String,
    /// 当前状态
    pub status: TeammateStatus,
    /// 后端类型
    pub backend_type: BackendType,
    /// 子进程 PID（SubProcess/Tmux 后端时有效）
    #[serde(skip)]
    pub process_pid: Option<u32>,
}

impl Teammate {
    /// 创建新的队友
    pub fn new(
        agent_id: String,
        agent_name: String,
        team_name: String,
        backend_type: BackendType,
    ) -> Self {
        Self {
            agent_id,
            agent_name,
            team_name,
            status: TeammateStatus::Idle,
            backend_type,
            process_pid: None,
        }
    }
}

// ── 团队 ──

/// Swarm 团队
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    /// 团队唯一标识
    pub id: String,
    /// 团队名称
    pub name: String,
    /// 团队成员列表
    pub members: Vec<Teammate>,
    /// 团队任务列表
    pub tasks: Vec<TeamTask>,
    /// 团队创建时间
    pub created_at: DateTime<Utc>,
}

impl Team {
    /// 创建新团队
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            members: Vec::new(),
            tasks: Vec::new(),
            created_at: Utc::now(),
        }
    }
}

// ── 团队任务 ──

/// 团队任务状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// 待分配
    Pending,
    /// 已分配，等待执行
    Assigned,
    /// 执行中
    InProgress,
    /// 已完成
    Completed,
    /// 失败
    Failed(String),
}

/// 团队任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTask {
    /// 任务唯一标识
    pub id: String,
    /// 任务描述
    pub description: String,
    /// 分配给谁（agent_id）
    pub assigned_to: Option<String>,
    /// 任务状态
    pub status: TaskStatus,
    /// 任务结果（完成后填充）
    pub result: Option<String>,
    /// 任务创建时间
    pub created_at: DateTime<Utc>,
}

impl TeamTask {
    /// 创建新任务
    pub fn new(description: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.to_string(),
            assigned_to: None,
            status: TaskStatus::Pending,
            result: None,
            created_at: Utc::now(),
        }
    }
}

// ── 队友间消息 ──

/// 队友间通信消息（通过 stdin/stdout JSON 行协议）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TeammateMessage {
    /// leader 分配任务给队友
    TaskAssign {
        /// 任务 ID
        task_id: String,
        /// 任务描述
        description: String,
        /// 发送者 agent_id
        from: String,
    },
    /// 队友返回任务执行结果
    TaskResult {
        /// 任务 ID
        task_id: String,
        /// 是否成功
        success: bool,
        /// 结果内容
        content: String,
        /// 发送者 agent_id
        from: String,
    },
    /// 队友报告进度
    ProgressReport {
        /// 关联任务 ID
        task_id: String,
        /// 进度百分比（0-100）
        progress: u8,
        /// 进度描述
        message: String,
        /// 发送者 agent_id
        from: String,
    },
    /// 心跳保活
    Heartbeat {
        /// 发送者 agent_id
        from: String,
        /// 发送者的当前状态
        status: TeammateStatus,
    },
    /// 关闭队友进程
    Shutdown {
        /// 关闭原因
        reason: String,
    },
}

// ── 公共函数 ──

/// 创建一个新团队
///
/// 仅在 SWARM_MODE feature flag 启用时才能有效运作。
/// 若 flag 未启用，仍可创建 Team 结构体但附加 swarm 功能不可用。
///
/// ## 线程安全
///
/// **本模块的公共函数不是线程安全的**。`Team` 本身不携带任何内部锁，
/// 所有 `&mut Team` 的操作假定由调用方在持有独占锁时调用。
///
/// 调用方应将 `Team` 包装在 `Arc<tokio::sync::RwLock<Team>>` 或
/// `Arc<parking_lot::RwLock<Team>>` 中，并通过 `.write().await` / `.write()`
/// 获取可变引用。**建议 lock 粒度为整个 Team 操作**，以避免在
/// `add_teammate` / `remove_teammate` / `assign_task` 等组合调用之间
/// 出现状态机不一致（例如 assign_task 半提交后被并发 remove 打断）。
pub fn create_team(name: &str) -> Team {
    if !is_swarm_enabled() {
        tracing::warn!(
            "SWARM_MODE 未启用，团队 '{}' 的 swarm 功能不可用（设置 AXAGENT_FF_SWARM_MODE=1 或 features.SwarmMode=true）",
            name
        );
    }
    Team::new(name)
}

/// 向团队添加一名队友
///
/// 返回创建的 Teammate。如果团队已满（达到 MAX_TEAM_MEMBERS）则返回 None。
///
/// ## 线程安全
///
/// **本函数不是线程安全的**，需要 `&mut Team` 独占访问。调用方应使用
/// `Arc<tokio::sync::RwLock<Team>>` 或 `Arc<parking_lot::RwLock<Team>>` 包装，
/// 并在持有写锁时调用。建议 lock 粒度为整个 Team 操作。
pub fn add_teammate(team: &mut Team, name: &str, backend: BackendType) -> Option<Teammate> {
    if team.members.len() >= MAX_TEAM_MEMBERS {
        return None;
    }
    let agent_id = teammate_id(name, &team.name);
    let teammate = Teammate::new(agent_id, name.to_string(), team.name.clone(), backend);
    team.members.push(teammate.clone());
    Some(teammate)
}

/// 从团队中移除指定队友
///
/// 副作用：移除前会调用 [`cancel_tasks_for_teammate`]，把该队友名下
/// 处于 `Assigned` 状态的任务标记为 `Failed("队友已离线")`，避免
/// 在 `team.tasks` 中留下无主幽灵任务。
///
/// 返回被移除的 Teammate，如果未找到则返回 None。
///
/// ## 线程安全
///
/// **本函数不是线程安全的**，需要 `&mut Team` 独占访问。调用方应使用
/// `Arc<tokio::sync::RwLock<Team>>` 或 `Arc<parking_lot::RwLock<Team>>` 包装，
/// 并在持有写锁时调用。建议 lock 粒度为整个 Team 操作。
pub fn remove_teammate(team: &mut Team, agent_id: &str) -> Option<Teammate> {
    // 先清理该队友名下处于 Assigned 状态的任务，避免幽灵任务
    cancel_tasks_for_teammate(team, agent_id);

    if let Some(pos) = team.members.iter().position(|m| m.agent_id == agent_id) {
        Some(team.members.remove(pos))
    } else {
        None
    }
}

/// 将任务分配给指定队友
///
/// 返回是否分配成功。队友必须存在且处于 `Idle` 状态；
/// 若队友处于 `Offline` 或 `Error` 状态则直接拒绝分配。
///
/// 成功分配时：
/// - 队友状态由 `Idle` 切换为 `Busy`
/// - 任务状态设为 `InProgress`（与队友 Busy 保持一致，避免半提交）
///
/// ## 线程安全
///
/// **本函数不是线程安全的**，需要 `&mut Team` 独占访问。调用方应使用
/// `Arc<tokio::sync::RwLock<Team>>` 或 `Arc<parking_lot::RwLock<Team>>` 包装，
/// 并在持有写锁时调用。建议 lock 粒度为整个 Team 操作。
pub fn assign_task(team: &mut Team, task: TeamTask, agent_id: &str) -> bool {
    // 查找队友
    let teammate = match team.members.iter_mut().find(|m| m.agent_id == agent_id) {
        Some(t) => t,
        None => return false,
    };

    // 队友必须处于 Idle 状态；Offline/Error 状态明确拒绝分配
    if !matches!(teammate.status, TeammateStatus::Idle) {
        return false;
    }

    // 更新队友状态和任务：队友 Busy + 任务 InProgress，保持状态机一致
    teammate.status = TeammateStatus::Busy;

    let mut assigned_task = task;
    assigned_task.assigned_to = Some(agent_id.to_string());
    assigned_task.status = TaskStatus::InProgress;
    team.tasks.push(assigned_task);

    true
}

/// 取消指定队友名下处于 `Assigned` 状态的任务
///
/// 把 `assigned_to == Some(agent_id) && status == Assigned` 的任务状态
/// 改为 `Failed("队友已离线".to_string())`，但保留在 `team.tasks` 列表中
/// 以便审计/重试/汇报 leader。`InProgress` 的任务不在本函数处理范围
/// （执行中任务的取消由队友自身上报 `TaskResult` 完成）。
///
/// 通常在 [`remove_teammate`] 内部调用，也可被外部观察者在队友
/// 异常离线时主动调用。
///
/// ## 线程安全
///
/// **本函数不是线程安全的**，需要 `&mut Team` 独占访问。
pub fn cancel_tasks_for_teammate(team: &mut Team, agent_id: &str) {
    for task in team.tasks.iter_mut() {
        if task.assigned_to.as_deref() == Some(agent_id)
            && matches!(task.status, TaskStatus::Assigned)
        {
            task.status = TaskStatus::Failed("队友已离线".to_string());
        }
    }
}

/// 生成队友 ID（格式: "name@team"）
pub fn teammate_id(name: &str, team_name: &str) -> String {
    format!("{}@{}", name, team_name)
}

/// 检查 Swarm 模式是否启用
///
/// 通过全局 FeatureFlags 中的 SWARM_MODE 标志判断。
pub fn is_swarm_enabled() -> bool {
    crate::feature_flags::global_feature_flags().swarm_mode_sync()
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_team() {
        let team = create_team("测试团队");
        assert_eq!(team.name, "测试团队");
        assert!(team.members.is_empty());
        assert!(team.tasks.is_empty());
    }

    #[test]
    fn test_add_teammate() {
        let mut team = create_team("测试团队");
        let tm = add_teammate(&mut team, "Alice", BackendType::SubProcess);
        assert!(tm.is_some());
        let tm = tm.unwrap();
        assert_eq!(tm.agent_id, "Alice@测试团队");
        assert_eq!(team.members.len(), 1);
    }

    #[test]
    fn test_remove_teammate() {
        let mut team = create_team("T");
        add_teammate(&mut team, "Bob", BackendType::InProcess);
        let removed = remove_teammate(&mut team, "Bob@T");
        assert!(removed.is_some());
        assert!(team.members.is_empty());
    }

    #[test]
    fn test_assign_task() {
        let mut team = create_team("T");
        add_teammate(&mut team, "Alice", BackendType::SubProcess);
        let task = TeamTask::new("修复登录 bug");
        let ok = assign_task(&mut team, task, "Alice@T");
        assert!(ok);
        assert_eq!(team.tasks.len(), 1);
        // 分配后任务应直接进入 InProgress 状态，与队友 Busy 保持一致
        assert_eq!(team.tasks[0].status, TaskStatus::InProgress);
        // 队友状态应为 Busy
        assert_eq!(team.members[0].status, TeammateStatus::Busy);
    }

    #[test]
    fn test_assign_task_busy_teammate() {
        let mut team = create_team("T");
        add_teammate(&mut team, "Alice", BackendType::SubProcess);
        let task1 = TeamTask::new("任务1");
        assign_task(&mut team, task1, "Alice@T");
        let task2 = TeamTask::new("任务2");
        let ok = assign_task(&mut team, task2, "Alice@T");
        assert!(!ok); // 队友忙碌，无法分配
        assert_eq!(team.tasks.len(), 1);
    }

    #[test]
    fn test_teammate_id_format() {
        let id = teammate_id("Alice", "DreamTeam");
        assert_eq!(id, "Alice@DreamTeam");
    }

    #[test]
    fn test_team_max_members() {
        let mut team = create_team("T");
        for i in 0..MAX_TEAM_MEMBERS {
            let result = add_teammate(&mut team, &format!("Agent{}", i), BackendType::InProcess);
            assert!(result.is_some());
        }
        // 超过上限
        let result = add_teammate(&mut team, "Overflow", BackendType::InProcess);
        assert!(result.is_none());
    }

    #[test]
    fn test_serialize_teammate_message() {
        let msg = TeammateMessage::Heartbeat {
            from: "Alice@T".into(),
            status: TeammateStatus::Idle,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("heartbeat"));
        assert!(json.contains("Alice@T"));
    }

    /// 移除队友时应当取消该队友名下处于 Assigned 状态的任务
    /// （修复缺陷 1.5：避免幽灵任务）
    #[test]
    fn test_remove_teammate_cancels_pending_tasks() {
        let mut team = create_team("T");
        add_teammate(&mut team, "Alice", BackendType::SubProcess);
        add_teammate(&mut team, "Bob", BackendType::SubProcess);

        // Alice 分配一个任务（状态变为 InProgress）
        let task_alice = TeamTask::new("Alice 的任务");
        assert!(assign_task(&mut team, task_alice, "Alice@T"));

        // Bob 也分配一个任务（状态变为 InProgress）
        let task_bob = TeamTask::new("Bob 的任务");
        assert!(assign_task(&mut team, task_bob, "Bob@T"));

        // 验证两条任务都在
        assert_eq!(team.tasks.len(), 2);
        assert!(team
            .tasks
            .iter()
            .all(|t| matches!(t.status, TaskStatus::InProgress)));

        // 手动制造一个 Assigned 状态的任务（模拟 assign_task 修复前
        // 的遗留场景或外部代码直接 push 的 Assigned 任务）
        let mut stale_task = TeamTask::new("遗留的 Assigned 任务");
        stale_task.assigned_to = Some("Alice@T".to_string());
        stale_task.status = TaskStatus::Assigned;
        team.tasks.push(stale_task);

        // 移除 Alice
        let removed = remove_teammate(&mut team, "Alice@T");
        assert!(removed.is_some());

        // Alice 已不在成员列表
        assert!(team.members.iter().all(|m| m.agent_id != "Alice@T"));
        // 任务列表仍保留 3 条任务（用于审计）
        assert_eq!(team.tasks.len(), 3);

        // 验证：Alice 名下所有 Assigned 状态的任务都已转为 Failed
        for task in &team.tasks {
            if task.assigned_to.as_deref() == Some("Alice@T") {
                match &task.status {
                    TaskStatus::Failed(reason) => {
                        assert_eq!(reason, "队友已离线");
                    }
                    other => panic!("Alice 的遗留任务应为 Failed，实际为 {:?}", other),
                }
            }
        }

        // Bob 的任务（包括 InProgress 状态）不应被 cancel_tasks_for_teammate 清理
        let bob_tasks: Vec<_> = team
            .tasks
            .iter()
            .filter(|t| t.assigned_to.as_deref() == Some("Bob@T"))
            .collect();
        assert_eq!(bob_tasks.len(), 1);
        assert!(matches!(bob_tasks[0].status, TaskStatus::InProgress));
    }

    /// 队友处于 Offline 或 Error 状态时，assign_task 应拒绝分配
    /// （修复缺陷 1.5：状态机修正）
    #[test]
    fn test_assign_task_rejects_offline_teammate() {
        // 场景 1: Offline 队友
        let mut team = create_team("T");
        add_teammate(&mut team, "Alice", BackendType::SubProcess);
        // 手动把 Alice 设为 Offline
        team.members[0].status = TeammateStatus::Offline;
        let task = TeamTask::new("任务");
        let ok = assign_task(&mut team, task, "Alice@T");
        assert!(!ok, "Offline 队友应拒绝分配");
        assert_eq!(team.tasks.len(), 0, "任务不应进入 tasks 列表");
        // 队友状态不应被改成 Busy
        assert_eq!(team.members[0].status, TeammateStatus::Offline);

        // 场景 2: Error 队友
        let mut team = create_team("T");
        add_teammate(&mut team, "Bob", BackendType::InProcess);
        team.members[0].status = TeammateStatus::Error("网络异常".to_string());
        let task2 = TeamTask::new("任务2");
        let ok2 = assign_task(&mut team, task2, "Bob@T");
        assert!(!ok2, "Error 队友应拒绝分配");
        assert_eq!(team.tasks.len(), 0);
        match &team.members[0].status {
            TeammateStatus::Error(msg) => assert_eq!(msg, "网络异常"),
            other => panic!("Error 状态应保持原样，实际为 {:?}", other),
        }

        // 场景 3: Idle 队友仍可正常分配（回归测试）
        let mut team = create_team("T");
        add_teammate(&mut team, "Carol", BackendType::SubProcess);
        let task3 = TeamTask::new("任务3");
        let ok3 = assign_task(&mut team, task3, "Carol@T");
        assert!(ok3, "Idle 队友应允许分配");
        assert_eq!(team.tasks.len(), 1);
        assert_eq!(team.members[0].status, TeammateStatus::Busy);
    }

    /// cancel_tasks_for_teammate 不影响 InProgress 任务
    /// （执行中任务的取消由队友自身上报 TaskResult 完成）
    #[test]
    fn test_cancel_tasks_for_teammate_preserves_in_progress() {
        let mut team = create_team("T");
        add_teammate(&mut team, "Alice", BackendType::SubProcess);
        let task = TeamTask::new("运行中任务");
        assert!(assign_task(&mut team, task, "Alice@T"));

        // 此时任务处于 InProgress
        assert!(matches!(team.tasks[0].status, TaskStatus::InProgress));

        // 显式调用 cancel_tasks_for_teammate，不应影响 InProgress
        cancel_tasks_for_teammate(&mut team, "Alice@T");
        assert!(
            matches!(team.tasks[0].status, TaskStatus::InProgress),
            "InProgress 任务不应被 cancel_tasks_for_teammate 取消"
        );
    }
}
