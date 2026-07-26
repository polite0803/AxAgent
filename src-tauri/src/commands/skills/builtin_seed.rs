// SPDX-License-Identifier: AGPL-3.0-only
//! G13 内置 SKILL.md 种子化
//!
//! 将项目内 `agency_experts/skills/<name>/SKILL.md` 4 个内置 skill 同步到
//! 用户目录 `~/.axagent/skills/<name>/SKILL.md`，使内置 skill 在首次启动时
//! 即可被 SkillIndex / SkillPromptCache 识别。
//!
//! 同步策略：仅在目标 SKILL.md 不存在时写入（不覆盖用户修改）。

use std::path::PathBuf;

/// 内置 skill 列表（name, SKILL.md 内容）
const BUILTIN_SKILLS: &[(&str, &str)] = &[
    ("stock-pick", include_str!("../../../agency_experts/skills/stock-pick/SKILL.md")),
    (
        "industry-chain-analysis",
        include_str!("../../../agency_experts/skills/industry-chain-analysis/SKILL.md"),
    ),
    ("risk-management", include_str!("../../../agency_experts/skills/risk-management/SKILL.md")),
    ("market-mainline", include_str!("../../../agency_experts/skills/market-mainline/SKILL.md")),
];

/// 获取用户 skills 目录（~/.axagent/skills/）
fn user_skills_dir() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".axagent").join("skills"))
}

/// 同步内置 SKILL.md 到用户目录
///
/// - 仅在目标文件不存在时写入（不覆盖用户修改）
/// - 创建中间目录（如 ~/.axagent/skills/stock-pick/）
/// - 同步后调用 `SkillPromptCache::invalidate()` 失效缓存
pub fn seed_builtin_skills() {
    let Some(skills_dir) = user_skills_dir() else {
        tracing::warn!("[skills] 无法定位用户目录，跳过内置 SKILL 种子化");
        return;
    };

    let mut synced_count = 0;
    for (name, content) in BUILTIN_SKILLS {
        let skill_dir = skills_dir.join(name);
        let skill_md = skill_dir.join("SKILL.md");

        // 仅在目标文件不存在时写入
        if skill_md.exists() {
            continue;
        }

        // 创建中间目录
        if let Err(e) = std::fs::create_dir_all(&skill_dir) {
            tracing::warn!("[skills] 创建目录 {} 失败: {e}", skill_dir.display());
            continue;
        }

        // 写入 SKILL.md
        if let Err(e) = std::fs::write(&skill_md, content) {
            tracing::warn!("[skills] 写入 {} 失败: {e}", skill_md.display());
            continue;
        }

        tracing::info!("[skills] 内置 SKILL 已同步: {name}");
        synced_count += 1;
    }

    if synced_count > 0 {
        // 失效 SkillPromptCache，让其下次访问时重建
        axagent_tools::tools::skill::SkillPromptCache::invalidate();
        tracing::info!("[skills] 内置 SKILL 同步完成（{synced_count} 个），缓存已失效");
    }
}
