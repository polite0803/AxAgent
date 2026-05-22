//! 集中式常量定义
//!
//! 避免在代码中硬编码字符串字面量。

/// LLM 角色常量
pub mod role {
    /// user 角色
    pub const USER: &str = "user";
    /// system 角色
    pub const SYSTEM: &str = "system";
    /// assistant 角色
    pub const ASSISTANT: &str = "assistant";
    /// tool 角色
    pub const TOOL: &str = "tool";
}
