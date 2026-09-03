// SPDX-License-Identifier: AGPL-3.0-only

//! 后台任务类型模块
//!
//! 注意：`in_process_teammate_task` 和 `remote_agent_task` 依赖未声明的
//! `swarm` 模块（swarm 目录尚未在 lib.rs 中声明），暂时注释掉以避免
//! 编译错误。待 swarm 模块正式启用后再恢复声明。

pub mod coevolution_task;
pub mod dream_task;
pub mod in_process_teammate_task;
pub mod insight_task;
pub mod pattern_task;
pub mod remote_agent_task;
