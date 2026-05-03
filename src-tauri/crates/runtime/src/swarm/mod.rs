//! Swarm/Teammate 跨进程多 Agent 团队协作系统
//!
//! ## 概述
//!
//! Swarm 系统通过 Feature Flag `SWARM_MODE` 控制启用。
//! 启用后，一个 Leader Agent 可以创建 Team，并通过
//! SubProcess 或 Tmux 后端启动多个 Teammate 子进程，
//! 通过 stdin/stdout JSON 行协议进行通信。
//!
//! ## 模块结构
//!
//! - [`constants`]: 团队配置常量
//! - [`team_helpers`]: 团队核心数据结构（Team, Teammate, TeamTask 等）
//! - [`spawn_utils`]: 队友进程启动与 JSON 消息收发
//! - [`permission_sync`]: leader 权限桥接到队友
//! - [`reconnection`]: 队友断开后自动重连
//! - [`backends`]: 后端实现（process_backend 等）
//!
//! ## 快速开始
//!
//! ```ignore
//! use axagent_runtime::swarm::*;
//!
//! // 1. 检查 Swarm 模式是否启用
//! assert!(!is_swarm_enabled()); // 默认关闭
//!
//! // 2. 创建团队
//! let mut team = create_team("DreamTeam");
//!
//! // 3. 添加队友
//! add_teammate(&mut team, "Alice", BackendType::SubProcess);
//! add_teammate(&mut team, "Bob", BackendType::SubProcess);
//!
//! // 4. 分配任务
//! let task = TeamTask::new("修复登录模块 bug");
//! assign_task(&mut team, task, "Alice@DreamTeam");
//! ```

pub mod constants;
pub mod team_helpers;
pub mod spawn_utils;
pub mod permission_sync;
pub mod reconnection;
pub mod backends;

// 重新导出最常用的类型和函数
pub use constants::*;
pub use team_helpers::{
    add_teammate, assign_task, create_team, is_swarm_enabled, remove_teammate, teammate_id,
    BackendType, TaskStatus, Team, TeamTask, Teammate, TeammateMessage, TeammateStatus,
};
