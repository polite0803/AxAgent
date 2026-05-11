//! MCP 协议实现 — 合并自 runtime crate
pub mod autostart;
pub mod client;
pub mod lifecycle_hardened;
pub mod server;
pub mod stdio;
pub mod tool_bridge;

pub use autostart::*;
pub use client::*;
pub use lifecycle_hardened::*;
pub use server::*;
pub use stdio::*;
pub use tool_bridge::*;
