// SPDX-License-Identifier: AGPL-3.0-only

use std::fs;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use tracing::{info, warn};

use crate::PluginSkillEntry;

#[derive(Debug)]
pub struct SkillInstaller {
    skills_root: PathBuf,
}

impl SkillInstaller {
    pub fn new(skills_root: impl Into<PathBuf>) -> Self {
        Self { skills_root: skills_root.into() }
    }

    /// 将插件 skills 部署到系统技能目录
    ///
    /// # 安全性
    /// 会验证 skill.path 不包含路径穿越（`..`），确保源文件在 plugin_root 范围内，
    /// 目标文件在 skills_root 范围内。
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
            // 安全检查：拒绝包含 `..` 的路径穿越
            if contains_path_traversal(&skill.path) {
                warn!(
                    "skill: skipped `{}` from plugin `{}` — path contains traversal: `{}`",
                    skill.name, plugin_id, skill.path
                );
                continue;
            }

            let src = plugin_root.join(&skill.path);

            // 安全检查：确保源文件在 plugin_root 范围内
            if !is_path_within_directory(&src, plugin_root) {
                warn!(
                    "skill: skipped `{}` from plugin `{}` — source path escapes plugin_root: `{}`",
                    skill.name, plugin_id, skill.path
                );
                continue;
            }

            if !src.exists() {
                warn!(
                    "skill: skipped `{}` from plugin `{}` — source file not found: `{}`",
                    skill.name, plugin_id, skill.path
                );
                continue;
            }

            let dest = plugin_skill_dir.join(&skill.path);

            // 安全检查：确保目标路径在 plugin_skill_dir 范围内
            if !is_path_within_directory(&dest, &plugin_skill_dir) {
                warn!(
                    "skill: skipped `{}` from plugin `{}` — dest path escapes skills_root: `{}`",
                    skill.name, plugin_id, skill.path
                );
                continue;
            }

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

/// 将 plugin_id (如 "@clawd/ths@external") 转换为安全的文件系统名称。
/// 使用原始 ID 的哈希后缀避免不同插件 ID 映射到相同目录名。
fn sanitize_for_path(id: &str) -> String {
    let sanitized: String = id
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | '@' | ':' => '-',
            other => other,
        })
        .collect();
    // 附加 SHA-256 哈希后缀以避免碰撞（如 "@clawd/ths@external" 和 "-clawd-ths-external"）
    // 使用加密哈希确保相同输入在不同进程中产生相同输出
    let mut hasher = Sha256::new();
    hasher.update(id.as_bytes());
    let result = hasher.finalize();
    let hash_hex = hex::encode(&result[..]);
    // 截取前 16 字符作为目录名后缀
    let short_hash = &hash_hex[..16];
    format!("{}-{}", sanitized, short_hash)
}

/// 检查路径是否包含穿越组件（`..`）
fn contains_path_traversal(path: &str) -> bool {
    Path::new(path).components().any(|c| matches!(c, Component::ParentDir))
}

/// 检查路径是否在指定目录范围内（防止路径穿越）
/// 使用 canonicalize 解析符号链接和相对路径后进行比较
fn is_path_within_directory(path: &Path, base: &Path) -> bool {
    // 如果路径不存在，无法 canonicalize，此时使用简单的组件检查
    let Ok(canonical_path) = path.canonicalize() else {
        // 回退到组件检查：路径必须不以 ParentDir 开头
        return !path.components().any(|c| matches!(c, Component::ParentDir));
    };

    let Ok(canonical_base) = base.canonicalize() else {
        // 基目录不存在时，拒绝访问
        return false;
    };

    canonical_path.starts_with(&canonical_base)
}
