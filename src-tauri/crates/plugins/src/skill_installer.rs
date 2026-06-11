// SPDX-License-Identifier: AGPL-3.0-only

use std::fs;
use std::path::{Path, PathBuf};

use tracing::info;

use crate::PluginSkillEntry;

#[derive(Debug)]
pub struct SkillInstaller {
    skills_root: PathBuf,
}

impl SkillInstaller {
    pub fn new(skills_root: impl Into<PathBuf>) -> Self {
        Self {
            skills_root: skills_root.into(),
        }
    }

    /// 将插件 skills 部署到系统技能目录
    pub fn install_plugin_skills(
        &self,
        plugin_id: &str,
        skills: &[PluginSkillEntry],
        plugin_root: &Path,
    ) -> Result<Vec<PathBuf>, std::io::Error> {
        let plugin_skill_dir = self.skills_root.join(sanitize_for_path(plugin_id));
        fs::create_dir_all(&plugin_skill_dir)?;
        let mut installed = Vec::new();
        for skill in skills {
            let src = plugin_root.join(&skill.path);
            let dest = plugin_skill_dir.join(&skill.path);
            if src.exists() {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&src, &dest)?;
                info!(
                    "skill: installed `{}` from plugin `{}` to `{}`",
                    skill.name,
                    plugin_id,
                    dest.display()
                );
                installed.push(dest);
            }
        }
        Ok(installed)
    }

    /// 卸载插件 skills
    pub fn remove_plugin_skills(&self, plugin_id: &str) -> Result<(), std::io::Error> {
        let plugin_skill_dir = self.skills_root.join(sanitize_for_path(plugin_id));
        if plugin_skill_dir.exists() {
            fs::remove_dir_all(&plugin_skill_dir)?;
            info!("skill: removed skills for plugin `{}`", plugin_id);
        }
        Ok(())
    }
}

/// 将 plugin_id (如 "@clawd/ths@external") 转换为安全的文件系统名称
fn sanitize_for_path(id: &str) -> String {
    id.chars()
        .map(|ch| match ch {
            '/' | '\\' | '@' | ':' => '-',
            other => other,
        })
        .collect()
}
