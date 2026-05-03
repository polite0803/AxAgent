//! Bash 安全执行模块
//!
//! 三层安全防护：
//! 1. 命令白名单 + 危险模式检测
//! 2. 路径边界验证
//! 3. 沙箱隔离（可选）

pub mod parser;
pub mod path_validation;
pub mod sandbox;
pub mod security;

pub use path_validation::PathValidator;
pub use sandbox::SandboxRunner;
pub use security::SecurityAnalyzer;
