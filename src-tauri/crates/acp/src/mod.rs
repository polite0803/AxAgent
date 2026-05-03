//! ACP (Agent Client Protocol) — 标准化 Agent 通信协议
//! Feature flag: ACP_PROTOCOL
//!
//! 协议基于 JSON-RPC 2.0，支持：
//! - HTTP REST 接口
//! - WebSocket 实时事件流
//! - stdio 子进程 JSON 行协议
//!
//! API 端点：
//! - POST /acp/v1/sessions — 创建会话
//! - GET  /acp/v1/sessions/:id — 查询会话状态
//! - POST /acp/v1/sessions/:id/prompts — 发送 prompt
//! - POST /acp/v1/sessions/:id/interrupt — 中断执行
//! - POST /acp/v1/sessions/:id/close — 关闭会话
//! - POST /acp/v1/hooks — 注册 hook 回调
//! - WS   /acp/v1/ws — WebSocket 实时事件流

pub mod protocol;
pub mod server;
pub mod client;
pub mod session;
pub mod types;
