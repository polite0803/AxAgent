// SPDX-License-Identifier: AGPL-3.0-only

//! 市场平台仓库：预置平台种子数据与 CRUD 操作。

use axagent_entities::opc_market_platform;
use sea_orm::*;
use std::time::{SystemTime, UNIX_EPOCH};

/// 预置平台定义
struct PresetPlatform {
    id: &'static str,
    name: &'static str,
    platform_type: &'static str,
    enabled: bool,
    base_url: Option<&'static str>,
    description: &'static str,
}

/// 预置平台列表（对应 marketplace_scanner.rs 中注册的扫描器）
const PRESET_PLATFORMS: &[PresetPlatform] = &[
    // 技术社区扫描器
    PresetPlatform {
        id: "reddit",
        name: "Reddit",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://www.reddit.com"),
        description: "Reddit 技术社区扫描器，获取相关需求线索",
    },
    PresetPlatform {
        id: "hackernews",
        name: "HackerNews",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://news.ycombinator.com"),
        description: "HackerNews 扫描器，获取技术趋势和需求线索",
    },
    PresetPlatform {
        id: "github-issues",
        name: "GitHub Issues",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://github.com"),
        description: "GitHub Issues 扫描器，从开源项目中提取需求线索",
    },
    PresetPlatform {
        id: "github-discussions",
        name: "GitHub Discussions",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://github.com"),
        description: "GitHub Discussions 扫描器，从讨论中提取需求线索",
    },
    PresetPlatform {
        id: "stackoverflow",
        name: "StackOverflow",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://stackoverflow.com"),
        description: "StackOverflow 扫描器，从技术问答中提取需求线索",
    },
    // 产品生态扫描器
    PresetPlatform {
        id: "producthunt",
        name: "Product Hunt",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://www.producthunt.com"),
        description: "Product Hunt 扫描器，获取新产品趋势和需求",
    },
    PresetPlatform {
        id: "huggingface",
        name: "HuggingFace",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://huggingface.co"),
        description: "HuggingFace 扫描器，获取 AI 模型需求线索",
    },
    PresetPlatform {
        id: "package-ecosystem",
        name: "Package Ecosystem",
        platform_type: "scanner",
        enabled: true,
        base_url: None,
        description: "包生态扫描器，监控 npm/pypi/crates 等包的更新和需求",
    },
    // 研究动态扫描器
    PresetPlatform {
        id: "arxiv",
        name: "arXiv",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://arxiv.org"),
        description: "arXiv 论文扫描器，获取研究趋势和技术需求",
    },
    // 社交媒体扫描器
    PresetPlatform {
        id: "twitter",
        name: "Twitter/X",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://twitter.com"),
        description: "Twitter/X 扫描器，从社交媒体中提取需求线索",
    },
    // 中国市场扫描器
    PresetPlatform {
        id: "zhubajie",
        name: "猪八戒",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://www.zbj.com"),
        description: "猪八戒扫描器，获取外包需求线索",
    },
    PresetPlatform {
        id: "xianyu",
        name: "闲鱼",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://www.goofish.com"),
        description: "闲鱼扫描器，获取二手需求线索",
    },
    // B2B/企业需求扫描器
    PresetPlatform {
        id: "linkedin",
        name: "LinkedIn",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://www.linkedin.com"),
        description: "LinkedIn 扫描器，获取 B2B 企业需求线索",
    },
    // 中国开发者社区扫描器
    PresetPlatform {
        id: "zhihu",
        name: "知乎",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://www.zhihu.com"),
        description: "知乎扫描器，从问答社区提取需求线索",
    },
    PresetPlatform {
        id: "csdn",
        name: "CSDN",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://www.csdn.net"),
        description: "CSDN 扫描器，从技术博客中提取需求线索",
    },
    PresetPlatform {
        id: "juejin",
        name: "掘金",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://juejin.cn"),
        description: "掘金扫描器，从技术社区中提取需求线索",
    },
    // 设计需求扫描器
    PresetPlatform {
        id: "dribbble",
        name: "Dribbble",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://dribbble.com"),
        description: "Dribbble 扫描器，从设计社区提取需求线索",
    },
    // 国际外包市场扫描器
    PresetPlatform {
        id: "upwork",
        name: "Upwork",
        platform_type: "scanner",
        enabled: true,
        base_url: Some("https://www.upwork.com"),
        description: "Upwork 扫描器，获取国际外包需求线索",
    },
];

/// 确保预置平台存在（幂等操作）
///
/// 对 `opc_market_platform` 表执行 UPSERT：若指定 id 已存在则跳过，不存在则插入。
/// 每次启动时调用，保证新平台能被自动注册。
pub async fn ensure_preset_platforms(db: &DatabaseConnection) -> Result<(), String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;

    for preset in PRESET_PLATFORMS {
        let existing = opc_market_platform::Entity::find_by_id(preset.id)
            .one(db)
            .await
            .map_err(|e| format!("Failed to check platform {}: {}", preset.id, e))?;

        if existing.is_none() {
            let entity = opc_market_platform::ActiveModel {
                id: Set(preset.id.to_string()),
                name: Set(preset.name.to_string()),
                platform_type: Set(preset.platform_type.to_string()),
                enabled: Set(if preset.enabled { 1 } else { 0 }),
                base_url: Set(preset.base_url.map(|s| s.to_string())),
                config_json: Set(serde_json::json!({
                    "description": preset.description,
                    "auto_sync": true,
                    "sync_interval_minutes": 60,
                })
                .to_string()),
                last_sync_at: Set(None),
                status: Set("idle".to_string()),
                created_at: Set(now),
                updated_at: Set(now),
            };

            entity
                .insert(db)
                .await
                .map_err(|e| format!("Failed to insert platform {}: {}", preset.id, e))?;

            tracing::info!(
                platform_id = preset.id,
                platform_name = preset.name,
                "[market-platform] Seeded preset platform"
            );
        }
    }

    Ok(())
}

/// 列出所有平台（确保预置存在）
///
/// 每次调用时都会触发种子化，如果种子化失败会返回错误。
pub async fn list_platforms(
    db: &DatabaseConnection,
) -> Result<Vec<opc_market_platform::Model>, String> {
    // 确保预置平台存在（如果表为空则插入种子数据）
    ensure_preset_platforms(db).await?;

    opc_market_platform::Entity::find()
        .order_by_desc(opc_market_platform::Column::Enabled)
        .order_by_asc(opc_market_platform::Column::Name)
        .all(db)
        .await
        .map_err(|e| format!("Failed to list platforms: {}", e))
}

/// 根据 ID 查找平台
pub async fn get_platform_by_id(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<opc_market_platform::Model>, String> {
    opc_market_platform::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| format!("Failed to get platform {}: {}", id, e))
}

/// 保存平台（新增或更新）
pub async fn save_platform(
    db: &DatabaseConnection,
    input: &serde_json::Value,
) -> Result<opc_market_platform::Model, String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;

    let id = input
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(&format!("mp-{}", uuid::Uuid::new_v4().simple()))
        .to_string();

    let existing = opc_market_platform::Entity::find_by_id(&id)
        .one(db)
        .await
        .map_err(|e| format!("Failed to find platform {}: {}", id, e))?;

    let name = input.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let platform_type =
        input.get("platform_type").and_then(|v| v.as_str()).unwrap_or("manual").to_string();
    let enabled = input.get("enabled").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
    let base_url = input.get("base_url").and_then(|v| v.as_str()).map(|s| s.to_string());
    let config = input.get("config").cloned().unwrap_or(serde_json::json!({}));

    if let Some(existing) = existing {
        let mut am: opc_market_platform::ActiveModel = existing.into();
        am.name = Set(name);
        am.platform_type = Set(platform_type);
        am.enabled = Set(enabled);
        am.base_url = Set(base_url);
        am.config_json = Set(serde_json::to_string(&config).unwrap_or_default());
        am.updated_at = Set(now);

        let saved =
            am.update(db).await.map_err(|e| format!("Failed to update platform {}: {}", id, e))?;
        Ok(saved)
    } else {
        let entity = opc_market_platform::ActiveModel {
            id: Set(id),
            name: Set(name),
            platform_type: Set(platform_type),
            enabled: Set(enabled),
            base_url: Set(base_url),
            config_json: Set(serde_json::to_string(&config).unwrap_or_default()),
            last_sync_at: Set(None),
            status: Set("idle".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let saved =
            entity.insert(db).await.map_err(|e| format!("Failed to insert platform: {}", e))?;
        Ok(saved)
    }
}

/// 删除平台
pub async fn delete_platform(db: &DatabaseConnection, id: &str) -> Result<(), String> {
    opc_market_platform::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(|e| format!("Failed to delete platform {}: {}", id, e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ensure_preset_platforms() {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("Failed to connect to database");

        // 初始化表结构
        db.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS opc_market_platform (
                id TEXT NOT NULL PRIMARY KEY,
                name TEXT NOT NULL,
                platform_type TEXT NOT NULL DEFAULT 'manual',
                enabled INTEGER NOT NULL DEFAULT 1,
                base_url TEXT,
                config_json TEXT NOT NULL DEFAULT '{}',
                last_sync_at INTEGER,
                status TEXT NOT NULL DEFAULT 'idle',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        )
        .await
        .expect("Failed to create table");

        // 执行种子数据
        ensure_preset_platforms(&db).await.expect("Failed to seed platforms");

        // 验证预置数量
        let platforms = list_platforms(&db).await.expect("Failed to list platforms");
        assert_eq!(platforms.len(), PRESET_PLATFORMS.len());

        // 验证幂等性
        ensure_preset_platforms(&db).await.expect("Failed to seed platforms (idempotent)");
        let platforms = list_platforms(&db).await.expect("Failed to list platforms");
        assert_eq!(platforms.len(), PRESET_PLATFORMS.len());
    }
}
