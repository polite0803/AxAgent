// SPDX-License-Identifier: AGPL-3.0-only

//! AxInvest 公司运行时内核（company-runtime）。
//!
//! OpenOPC 三机制在 AxInvest 本地的实现，依赖基座既有能力：
//! - **Self-Run**（WorkItem 状态机）：`work_item`（纯函数）+ `work_item_service`（DB）
//! - **Self-Built**（组织/招聘）：`org` / `hiring`
//! - **Self-Grown**（经验闭环）：`experience`
//!
//! 分层：implementor。可依赖 harness + entities + dao + opc-entities；
//! 不依赖 consumer（agent / runtime-core / gateway / orchestrator）。
//! AxAgent 零改动——公司运行时是"一人公司"领域机制，非所有 fork 需要的横切能力。

pub mod error;
pub mod experience;
pub mod hiring;
pub mod org;
pub mod self_improving;
pub mod work_item;
pub mod work_item_service;

pub use error::{CompanyError, CompanyResult};
pub use experience::{ExperienceService, QualityGateService, Signal};
pub use hiring::HiringService;
pub use org::OrgService;
pub use self_improving::OpcWorkItemRound;
pub use work_item::{ManagementMode, Phase, Transition, TransitionError};
pub use work_item_service::{NewWorkItem, WorkItemService};
