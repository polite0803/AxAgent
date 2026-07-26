// SPDX-License-Identifier: AGPL-3.0-only

pub mod closed_loop;
pub mod entity;
pub mod service;
// G21: Skill 摘要 MemoryProvider — 把 SkillManager 缓存中的技能摘要暴露为 MemoryEntry
pub mod skill_summary_provider;
// G21: MemoryHookProvider — 会话生命周期记忆同步 Hook（PluginHook 实现）
pub mod memory_hook_provider;
