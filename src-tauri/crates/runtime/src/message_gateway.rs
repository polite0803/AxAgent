// SPDX-License-Identifier: AGPL-3.0-only

//! Message Gateway — re-exported from axagent-rt-messaging.

pub mod platform_bridge;
pub mod platform_config;
pub mod platform_manager;
pub mod platforms;
pub mod session_router;

pub use axagent_rt_messaging::message_gateway::*;
