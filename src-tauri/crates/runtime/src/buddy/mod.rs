//! Buddy/Companion 陪伴系统 — 游戏化的陪伴角色
//!
//! 参考 claude-code-main 的 `src/buddy/` 目录设计，提供：
//! - 12 种可选的 Buddy 物种（鸭子、猫咪、小龙、凤凰等）
//! - 5 种属性系统（调试、耐心、混乱、智慧、毒舌）
//! - 经验值和等级成长机制
//! - 基于上下文的消息生成系统
//!
//! ## 使用示例
//!
//! ```ignore
//! use crate::buddy::manager::{BuddyManager, BuddyContext};
//!
//! let mut mgr = BuddyManager::new();
//! mgr.summon_random();                    // 随机召唤
//! mgr.grant_xp(50);                       // 给予经验
//! if let Some(msg) = mgr.generate_message(&BuddyContext::Startup) {
//!     println!("{}: {}", msg.buddy_name, msg.text);
//! }
//! ```

pub mod attributes;
pub mod manager;
pub mod species;
