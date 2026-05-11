//! Message Gateway — cross-platform agent communication.

pub mod message_batching;
pub mod message_gateway;
pub mod sse;
pub mod webhook_subscription;

pub use message_gateway::{AgentMessage, MessagePayload};
