//! Message Gateway — cross-platform agent communication.

#![allow(clippy::await_holding_lock)]
#![allow(clippy::wrong_self_convention)]

pub mod message_batching;
pub mod message_gateway;
pub mod sse;
pub mod webhook_subscription;

pub use message_gateway::{AgentMessage, MessagePayload};
