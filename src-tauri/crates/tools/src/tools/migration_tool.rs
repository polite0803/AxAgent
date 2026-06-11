// SPDX-License-Identifier: AGPL-3.0-only

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_harness::MigrationRunner;
use serde_json::Value;
use std::sync::{Arc, OnceLock};

pub struct MigrationTool;

static RUNNER: OnceLock<Arc<dyn MigrationRunner>> = OnceLock::new();

/// 注入 `MigrationRunner` trait object（由 wiring 层在初始化时调用一次）
pub fn set_migration_runner(runner: Arc<dyn MigrationRunner>) {
    let _ = RUNNER.set(runner);
}

fn runner() -> &'static Arc<dyn MigrationRunner> {
    RUNNER
        .get()
        .expect("MigrationRunner not initialized; call set_migration_runner() at startup")
}

#[async_trait]
impl Tool for MigrationTool {
    fn name(&self) -> &str {
        "Migration"
    }

    fn description(&self) -> &str {
        "从其他 Agent 平台（OpenClaw / Hermes）迁移数据到 AxAgent。\
         支持以下操作：\
         - detect: 扫描已安装的 Agent 平台\
         - preview: 预览迁移内容（dry-run）\
         - migrate: 执行实际迁移\
         - rollback: 回滚到备份状态\
         迁移前会自动创建备份，支持 --overwrite 覆盖已有内容。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["detect", "preview", "migrate", "rollback"],
                    "description": "要执行的操作"
                },
                "platform": {
                    "type": "string",
                    "enum": ["openclaw", "hermes"],
                    "description": "目标平台（preview/migrate 时必需）"
                },
                "overwrite": {
                    "type": "boolean",
                    "description": "是否覆盖已有内容（默认 false）"
                },
                "backup_path": {
                    "type": "string",
                    "description": "回滚时使用的备份路径（rollback 时必需）"
                }
            },
            "required": ["action"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = input["action"].as_str().unwrap_or("").to_lowercase();

        match action.as_str() {
            "detect" => action_detect(),
            "preview" => action_preview(&input),
            "migrate" => action_migrate(&input),
            "rollback" => action_rollback(&input),
            _ => Err(ToolError::invalid_input(format!(
                "未知的 action: '{}'。支持: detect, preview, migrate, rollback",
                action
            ))),
        }
    }
}

fn action_detect() -> Result<ToolResult, ToolError> {
    let platforms = runner().detect_platforms();

    if platforms.is_empty() {
        return Ok(ToolResult {
            content: "## 平台检测\n\n未检测到任何已安装的 Agent 平台。\n\n扫描路径：\n- ~/.openclaw\n- ~/.hermes".to_string(),
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "platforms": [],
                "scanned": ["~/.openclaw", "~/.hermes"],
            })),
            duration_ms: None,
            progress: Vec::new(),
        });
    }

    let mut out = String::from("## 检测到的平台\n\n");
    for p in &platforms {
        out.push_str(&format!("### {} ({})\n", p.name, p.base_path.display()));
        out.push_str(&format!("- SOUL.md: {}\n", if p.has_soul { "✅" } else { "❌" }));
        out.push_str(&format!("- MEMORY: {}\n", if p.has_memory { "✅" } else { "❌" }));
        out.push_str(&format!(
            "- Skills: {} ({} 个)\n",
            if p.has_skills { "✅" } else { "❌" },
            p.skill_count
        ));
        out.push_str(&format!("- Config: {}\n", if p.has_config { "✅" } else { "❌" }));
        out.push_str(&format!("- .env: {}\n", if p.has_env { "✅" } else { "❌" }));
        out.push_str(&format!("- Cron: {}\n", if p.has_cron { "✅" } else { "❌" }));
        out.push_str(&format!(
            "- Personalities: {}\n",
            if p.has_personalities { "✅" } else { "❌" }
        ));
        out.push('\n');
    }

    let metadata_platforms: Vec<Value> = platforms
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "base_path": p.base_path.display().to_string(),
                "has_soul": p.has_soul,
                "has_memory": p.has_memory,
                "has_skills": p.has_skills,
                "has_config": p.has_config,
                "has_env": p.has_env,
                "has_cron": p.has_cron,
                "has_personalities": p.has_personalities,
                "skill_count": p.skill_count,
                "memory_count": p.memory_count,
            })
        })
        .collect();

    Ok(ToolResult {
        content: out,
        is_error: false,
        truncated: false,
        metadata: Some(serde_json::json!({
            "platforms": metadata_platforms,
        })),
        duration_ms: None,
        progress: Vec::new(),
    })
}

fn action_preview(input: &Value) -> Result<ToolResult, ToolError> {
    let platform = input["platform"].as_str().unwrap_or("").to_lowercase();

    if platform.is_empty() {
        return Err(ToolError::invalid_input(
            "platform 参数在 preview 操作中是必需的（openclaw / hermes）",
        ));
    }

    let items = match platform.as_str() {
        "openclaw" => runner().preview_openclaw(),
        "hermes" => runner().preview_hermes(),
        _ => {
            return Err(ToolError::invalid_input(format!(
                "不支持的平台: '{}'。支持: openclaw, hermes",
                platform
            )));
        },
    };

    if items.is_empty() {
        return Ok(ToolResult {
            content: format!("## 预览: {}\n\n没有可迁移的内容。", platform),
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "platform": platform,
                "items": [],
            })),
            duration_ms: None,
            progress: Vec::new(),
        });
    }

    let mut out = format!("## 预览: {} 迁移\n\n", platform);
    let mut new_count = 0usize;
    let mut existing_count = 0usize;

    for item in &items {
        let status = if item.exists_at_dest {
            existing_count += 1;
            "⚠️ 已存在"
        } else {
            new_count += 1;
            "🆕 新增"
        };
        out.push_str(&format!(
            "- {} **{}**: {}\n  `{} → {}`\n\n",
            status,
            item.item_type,
            item.description,
            item.source.display(),
            item.destination.display(),
        ));
    }

    out.push_str(&format!(
        "\n**总计**: {} 项（新增 {}, 已存在 {}）\n\n使用 `action: \"migrate\"` 执行迁移。添加 `overwrite: true` 覆盖已存在的内容。",
        items.len(),
        new_count,
        existing_count,
    ));

    let metadata_items: Vec<Value> = items
        .iter()
        .map(|i| {
            serde_json::json!({
                "source": i.source.display().to_string(),
                "destination": i.destination.display().to_string(),
                "item_type": i.item_type,
                "description": i.description,
                "exists_at_dest": i.exists_at_dest,
            })
        })
        .collect();

    Ok(ToolResult {
        content: out,
        is_error: false,
        truncated: false,
        metadata: Some(serde_json::json!({
            "platform": platform,
            "items": metadata_items,
            "new_count": new_count,
            "existing_count": existing_count,
        })),
        duration_ms: None,
        progress: Vec::new(),
    })
}

fn action_migrate(input: &Value) -> Result<ToolResult, ToolError> {
    let platform = input["platform"].as_str().unwrap_or("").to_lowercase();

    if platform.is_empty() {
        return Err(ToolError::invalid_input(
            "platform 参数在 migrate 操作中是必需的（openclaw / hermes）",
        ));
    }

    let overwrite = input["overwrite"].as_bool().unwrap_or(false);

    let backup = runner()
        .create_backup(&platform)
        .map_err(|e| ToolError::execution_failed(format!("创建备份失败: {}", e)))?;

    let report = match platform.as_str() {
        "openclaw" => runner().migrate_openclaw(overwrite),
        "hermes" => runner().migrate_hermes(overwrite),
        _ => {
            return Err(ToolError::invalid_input(format!(
                "不支持的平台: '{}'。支持: openclaw, hermes",
                platform
            )));
        },
    };

    let mut out = format!("## 迁移报告: {}\n\n", report.platform);
    out.push_str(&format!("**时间**: {}\n", report.timestamp));
    out.push_str(&format!("**备份**: `{}`\n\n", backup.backup_path.display()));

    if !report.migrated.is_empty() {
        out.push_str(&format!("### ✅ 已迁移 ({} 项)\n\n", report.migrated.len()));
        for entry in &report.migrated {
            out.push_str(&format!("- **{}**: {}\n", entry.item_type, entry.description));
        }
        out.push('\n');
    }

    if !report.skipped.is_empty() {
        out.push_str(&format!("### ⏭️ 已跳过 ({} 项)\n\n", report.skipped.len()));
        for entry in &report.skipped {
            out.push_str(&format!(
                "- **{}**: {} — {}\n",
                entry.item_type, entry.description, entry.reason
            ));
        }
        out.push('\n');
    }

    if !report.failed.is_empty() {
        out.push_str(&format!("### ❌ 失败 ({} 项)\n\n", report.failed.len()));
        for entry in &report.failed {
            out.push_str(&format!(
                "- **{}**: {} — {}\n",
                entry.item_type, entry.description, entry.reason
            ));
        }
        out.push('\n');
    }

    if report.migrated.is_empty() && report.skipped.is_empty() && report.failed.is_empty() {
        out.push_str("没有需要迁移的内容。\n");
    }

    out.push_str(&format!(
        "\n如需回滚，使用 `action: \"rollback\"`，备份路径: `{}`",
        backup.backup_path.display()
    ));

    Ok(ToolResult {
        content: out,
        is_error: false,
        truncated: false,
        metadata: Some(serde_json::json!({
            "platform": report.platform,
            "timestamp": report.timestamp,
            "backup_path": backup.backup_path.display().to_string(),
            "migrated_count": report.migrated.len(),
            "skipped_count": report.skipped.len(),
            "failed_count": report.failed.len(),
            "migrated": report.migrated,
            "skipped": report.skipped,
            "failed": report.failed,
        })),
        duration_ms: None,
        progress: Vec::new(),
    })
}

fn action_rollback(input: &Value) -> Result<ToolResult, ToolError> {
    let backup_path = input["backup_path"].as_str().unwrap_or("").to_string();

    if backup_path.is_empty() {
        let backups = runner().list_backups();
        if backups.is_empty() {
            return Ok(ToolResult {
                content: "## 回滚\n\n没有可用的备份。\n备份路径: ~/.axagent/migration-backup/"
                    .to_string(),
                is_error: false,
                truncated: false,
                metadata: Some(serde_json::json!({
                    "backups": [],
                })),
                duration_ms: None,
                progress: Vec::new(),
            });
        }

        let mut out = String::from("## 可用的备份\n\n");
        for b in &backups {
            out.push_str(&format!(
                "- **{}**: `{}` ({} 项)\n",
                b.timestamp,
                b.backup_path.display(),
                b.items_backed_up.len(),
            ));
        }
        out.push_str("\n使用 `backup_path` 参数指定要回滚到的备份。");

        let metadata_backups: Vec<Value> = backups
            .iter()
            .map(|b| {
                serde_json::json!({
                    "timestamp": b.timestamp,
                    "backup_path": b.backup_path.display().to_string(),
                    "items_backed_up": b.items_backed_up,
                })
            })
            .collect();

        return Ok(ToolResult {
            content: out,
            is_error: false,
            truncated: false,
            metadata: Some(serde_json::json!({
                "backups": metadata_backups,
            })),
            duration_ms: None,
            progress: Vec::new(),
        });
    }

    let path = std::path::PathBuf::from(&backup_path);
    let report = runner()
        .rollback(&path)
        .map_err(|e| ToolError::execution_failed(format!("回滚失败: {}", e)))?;

    let mut out = String::from("## 回滚报告\n\n");
    out.push_str(&format!("**备份路径**: `{}`\n", backup_path));
    out.push_str(&format!("**时间**: {}\n\n", report.timestamp));

    if !report.migrated.is_empty() {
        out.push_str(&format!("### ✅ 已恢复 ({} 项)\n\n", report.migrated.len()));
        for entry in &report.migrated {
            out.push_str(&format!("- **{}**: {}\n", entry.item_type, entry.description));
        }
        out.push('\n');
    }

    if !report.failed.is_empty() {
        out.push_str(&format!("### ❌ 恢复失败 ({} 项)\n\n", report.failed.len()));
        for entry in &report.failed {
            out.push_str(&format!(
                "- **{}**: {} — {}\n",
                entry.item_type, entry.description, entry.reason
            ));
        }
        out.push('\n');
    }

    Ok(ToolResult {
        content: out,
        is_error: false,
        truncated: false,
        metadata: Some(serde_json::json!({
            "backup_path": backup_path,
            "timestamp": report.timestamp,
            "restored_count": report.migrated.len(),
            "failed_count": report.failed.len(),
            "migrated": report.migrated,
            "failed": report.failed,
        })),
        duration_ms: None,
        progress: Vec::new(),
    })
}
