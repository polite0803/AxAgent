// SPDX-License-Identifier: AGPL-3.0-only

//! axagent-dao — Marketplace / Review 服务
//!
//! SeaORM 数据访问实现。DTO 和 trait 契约在 `axagent_harness::marketplace` 中定义，
//! 本模块实现 `MarketplaceService` trait，通过 harness 暴露给上层。

use async_trait::async_trait;
use sea_orm::*;
use uuid::Uuid;

use axagent_entities::{workflow_marketplace, workflow_marketplace_review, workflow_template};
use axagent_harness::marketplace::{
    CATALOG_ITEM_TYPE_SKILL, CATALOG_ITEM_TYPE_TEMPLATE, CatalogItem, CatalogPage, CatalogQuery,
    CreateReviewRequest, MarketplaceCatalogService as MarketplaceCatalogServiceTrait,
    MarketplaceService as MarketplaceServiceTrait, MarketplaceStats, ReviewResponse,
    UpdateReviewRequest,
};

/// Default SeaORM-backed implementation of `MarketplaceService`.
pub struct MarketplaceServiceImpl;

#[async_trait]
impl MarketplaceServiceTrait for MarketplaceServiceImpl {
    async fn create_review(
        &self,
        db: &DatabaseConnection,
        req: CreateReviewRequest,
    ) -> Result<ReviewResponse, String> {
        if req.rating < 1 || req.rating > 5 {
            return Err("Rating must be between 1 and 5".to_string());
        }

        let now = chrono::Utc::now().timestamp();

        let review = workflow_marketplace_review::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            marketplace_id: Set(req.marketplace_id.clone()),
            user_id: Set(req.user_id),
            rating: Set(req.rating),
            comment: Set(req.comment),
            is_hidden: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = review.insert(db).await.map_err(|e| e.to_string())?;

        self.update_marketplace_rating(db, &req.marketplace_id).await?;

        Ok(model_to_response(result))
    }

    async fn get_reviews(
        &self,
        db: &DatabaseConnection,
        marketplace_id: &str,
    ) -> Result<Vec<ReviewResponse>, String> {
        let reviews = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::MarketplaceId.eq(marketplace_id))
            .filter(workflow_marketplace_review::Column::IsHidden.eq(false))
            .order_by_desc(workflow_marketplace_review::Column::CreatedAt)
            .all(db)
            .await
            .map_err(|e| e.to_string())?;

        Ok(reviews.into_iter().map(model_to_response).collect())
    }

    async fn get_user_review(
        &self,
        db: &DatabaseConnection,
        marketplace_id: &str,
        user_id: &str,
    ) -> Result<Option<ReviewResponse>, String> {
        let review = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::MarketplaceId.eq(marketplace_id))
            .filter(workflow_marketplace_review::Column::UserId.eq(user_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?;

        Ok(review.map(model_to_response))
    }

    async fn update_review(
        &self,
        db: &DatabaseConnection,
        review_id: &str,
        req: UpdateReviewRequest,
    ) -> Result<ReviewResponse, String> {
        if let Some(rating) = req.rating
            && !(1..=5).contains(&rating)
        {
            return Err("Rating must be between 1 and 5".to_string());
        }

        let review = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::Id.eq(review_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        let marketplace_id = review.marketplace_id.clone();

        let mut active_model: workflow_marketplace_review::ActiveModel = review.into();
        if let Some(rating) = req.rating {
            active_model.rating = Set(rating);
        }
        if let Some(comment) = req.comment {
            active_model.comment = Set(Some(comment));
        }
        active_model.updated_at = Set(chrono::Utc::now().timestamp());

        let result = active_model.update(db).await.map_err(|e| e.to_string())?;

        self.update_marketplace_rating(db, &marketplace_id).await?;

        Ok(model_to_response(result))
    }

    async fn delete_review(&self, db: &DatabaseConnection, review_id: &str) -> Result<(), String> {
        let review = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::Id.eq(review_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        let marketplace_id = review.marketplace_id.clone();

        let active_model: workflow_marketplace_review::ActiveModel = review.into();
        active_model.delete(db).await.map_err(|e| e.to_string())?;

        self.update_marketplace_rating(db, &marketplace_id).await?;

        Ok(())
    }

    async fn get_stats(
        &self,
        db: &DatabaseConnection,
        marketplace_id: &str,
    ) -> Result<MarketplaceStats, String> {
        let reviews = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::MarketplaceId.eq(marketplace_id))
            .filter(workflow_marketplace_review::Column::IsHidden.eq(false))
            .all(db)
            .await
            .map_err(|e| e.to_string())?;

        let total_reviews = reviews.len() as i32;
        let rating_average = if total_reviews > 0 {
            let sum: i32 = reviews.iter().map(|r| r.rating).sum();
            sum as f64 / total_reviews as f64
        } else {
            0.0
        };

        Ok(MarketplaceStats {
            marketplace_id: marketplace_id.to_string(),
            total_reviews,
            rating_average,
        })
    }

    async fn get_marketplace_id_for_review(
        &self,
        db: &DatabaseConnection,
        review_id: &str,
    ) -> Result<String, String> {
        let review = workflow_marketplace_review::Entity::find()
            .filter(workflow_marketplace_review::Column::Id.eq(review_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Review not found".to_string())?;

        Ok(review.marketplace_id)
    }
}

// ── Private helpers ──

fn model_to_response(model: workflow_marketplace_review::Model) -> ReviewResponse {
    ReviewResponse {
        id: model.id,
        marketplace_id: model.marketplace_id,
        user_id: model.user_id,
        rating: model.rating,
        comment: model.comment,
        created_at: model.created_at,
    }
}

impl MarketplaceServiceImpl {
    async fn update_marketplace_rating(
        &self,
        db: &DatabaseConnection,
        marketplace_id: &str,
    ) -> Result<(), String> {
        let stats = <Self as MarketplaceServiceTrait>::get_stats(self, db, marketplace_id).await?;

        let marketplace = workflow_marketplace::Entity::find()
            .filter(workflow_marketplace::Column::Id.eq(marketplace_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Marketplace not found".to_string())?;

        let mut active_model: workflow_marketplace::ActiveModel = marketplace.into();
        active_model.rating_average = Set(stats.rating_average);
        active_model.rating_count = Set(stats.total_reviews);
        active_model.updated_at = Set(chrono::Utc::now().timestamp());

        active_model.update(db).await.map_err(|e| e.to_string())?;

        Ok(())
    }
}

// ── Backward compatibility: type alias for old `MarketplaceService` struct name ──
/// Deprecated alias — use `MarketplaceServiceImpl` directly or the
/// `axagent_harness::marketplace::MarketplaceService` trait.
pub type MarketplaceService = MarketplaceServiceImpl;

// ── 目录浏览/搜索/安装/发布实现 ─────────────────────────────────────
//
// 单用户桌面场景下「市场」= 本地已导入的工作流模板 + 本地技能目录的统一浏览。
// 模板的安装/发布状态记录在 `workflow_marketplace` 表（`is_public` 字段）。
// 本地技能通过扫描 `~/.axagent/skills/` 目录下的 `SKILL.md` 文件收集。

/// 本地 skills 目录：`~/.axagent/skills/`
fn local_skills_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".axagent").join("skills"))
}

/// 从 `SKILL.md` 文件解析技能元数据（极简实现：仅识别标题与描述）。
fn parse_skill_manifest(content: &str) -> (Option<String>, Option<String>) {
    let mut name = None;
    let mut description = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(h) = trimmed.strip_prefix("# ") {
            if name.is_none() {
                name = Some(h.trim().to_string());
            }
        } else if let Some(d) = trimmed.strip_prefix("> ")
            && description.is_none()
        {
            description = Some(d.trim().to_string());
        }
    }
    (name, description)
}

/// 扫描本地 skills 目录，返回所有技能条目。
///
/// 单个技能目录结构：`~/.axagent/skills/<skill_name>/SKILL.md`
/// 仅读取目录名作为 id，`SKILL.md` 首行 `# 标题` 作为 name，`> 描述` 作为 description。
fn scan_local_skills() -> Vec<CatalogItem> {
    let mut items = Vec::new();
    let Some(skills_dir) = local_skills_dir() else {
        return items;
    };

    let entries = match std::fs::read_dir(&skills_dir) {
        Ok(e) => e,
        Err(_) => return items,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let id = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let manifest_path = path.join("SKILL.md");
        let (name, description) = match std::fs::read_to_string(&manifest_path) {
            Ok(content) => parse_skill_manifest(&content),
            Err(_) => (None, None),
        };

        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let updated_at = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or_else(|| chrono::Utc::now().timestamp());

        items.push(CatalogItem {
            id,
            item_type: CATALOG_ITEM_TYPE_SKILL.to_string(),
            name: name.unwrap_or_else(|| "Unnamed Skill".to_string()),
            description,
            category: None,
            author: None,
            tags: Vec::new(),
            version: None,
            installed: true,
            rating_average: None,
            download_count: None,
            created_at: updated_at,
            updated_at,
        });
    }

    items
}

/// 把 `workflow_template::Model` + `workflow_marketplace::Model` 转换为 `CatalogItem`。
fn template_to_catalog_item(
    template: &workflow_template::Model,
    marketplace: Option<&workflow_marketplace::Model>,
) -> CatalogItem {
    let tags = template
        .tags
        .as_ref()
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default();

    let (category, rating_average, download_count, installed) = match marketplace {
        Some(m) => {
            (Some(m.category.clone()), Some(m.rating_average), Some(m.downloads), m.is_public)
        },
        None => (None, None, None, false),
    };

    CatalogItem {
        id: template.id.clone(),
        item_type: CATALOG_ITEM_TYPE_TEMPLATE.to_string(),
        name: template.name.clone(),
        description: template.description.clone(),
        category,
        author: None,
        tags,
        version: Some(template.version.to_string()),
        installed,
        rating_average,
        download_count,
        created_at: template.created_at,
        updated_at: template.updated_at,
    }
}

/// 关键词匹配：在 name/description/tags 中模糊匹配（大小写不敏感）。
fn matches_keyword(item: &CatalogItem, keyword: &str) -> bool {
    let kw = keyword.to_lowercase();
    if item.name.to_lowercase().contains(&kw) {
        return true;
    }
    if let Some(desc) = &item.description
        && desc.to_lowercase().contains(&kw)
    {
        return true;
    }
    item.tags.iter().any(|t| t.to_lowercase().contains(&kw))
}

/// 市场目录服务实现。
pub struct MarketplaceCatalogServiceImpl;

#[async_trait]
impl MarketplaceCatalogServiceTrait for MarketplaceCatalogServiceImpl {
    async fn list_catalog(
        &self,
        db: &DatabaseConnection,
        query: CatalogQuery,
    ) -> Result<CatalogPage, String> {
        let limit = query.limit.unwrap_or(50).clamp(1, 500);
        let offset = query.offset.unwrap_or(0);

        // ── 工作流模板：左连 workflow_marketplace 表 ──
        let templates = workflow_template::Entity::find()
            .order_by_desc(workflow_template::Column::UpdatedAt)
            .all(db)
            .await
            .map_err(|e| e.to_string())?;

        let marketplace_rows = workflow_marketplace::Entity::find().all(db).await.map_err(|e| {
            tracing::warn!("[marketplace_catalog] 读取 workflow_marketplace 表失败: {e}");
            e.to_string()
        })?;

        let marketplace_map: std::collections::HashMap<String, workflow_marketplace::Model> =
            marketplace_rows.into_iter().map(|m| (m.template_id.clone(), m)).collect();

        let mut items: Vec<CatalogItem> = templates
            .iter()
            .map(|t| template_to_catalog_item(t, marketplace_map.get(&t.id)))
            .collect();

        // ── 本地技能：仅当未限定 item_type 或限定为 skill 时扫描 ──
        let want_skills =
            query.item_type.as_deref().map(|t| t == CATALOG_ITEM_TYPE_SKILL).unwrap_or(true);
        if want_skills {
            items.extend(scan_local_skills());
        }

        // ── 客户端过滤：keyword / category / item_type ──
        if let Some(ref kw) = query.keyword
            && !kw.is_empty()
        {
            items.retain(|i| matches_keyword(i, kw));
        }
        if let Some(ref cat) = query.category
            && !cat.is_empty()
        {
            items.retain(|i| i.category.as_deref() == Some(cat.as_str()));
        }
        if let Some(ref it) = query.item_type
            && !it.is_empty()
        {
            items.retain(|i| &i.item_type == it);
        }

        let total = items.len() as u64;
        let start = (offset as usize).min(items.len());
        let end = (start + limit as usize).min(items.len());
        items.drain(end..);
        let page_items: Vec<CatalogItem> = items.drain(start..).collect();

        Ok(CatalogPage { items: page_items, total, offset, limit })
    }

    async fn get_catalog_item(
        &self,
        db: &DatabaseConnection,
        item_id: &str,
        item_type: &str,
    ) -> Result<Option<CatalogItem>, String> {
        match item_type {
            CATALOG_ITEM_TYPE_TEMPLATE => {
                let template = workflow_template::Entity::find_by_id(item_id)
                    .one(db)
                    .await
                    .map_err(|e| e.to_string())?;
                let Some(template) = template else {
                    return Ok(None);
                };

                let marketplace = workflow_marketplace::Entity::find()
                    .filter(workflow_marketplace::Column::TemplateId.eq(item_id))
                    .one(db)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(Some(template_to_catalog_item(&template, marketplace.as_ref())))
            },
            CATALOG_ITEM_TYPE_SKILL => {
                // 在本地技能扫描结果中查找匹配 ID
                Ok(scan_local_skills().into_iter().find(|i| i.id == item_id))
            },
            other => Err(format!("Unknown item_type: {other}")),
        }
    }

    async fn install_template(
        &self,
        db: &DatabaseConnection,
        template_id: &str,
    ) -> Result<(), String> {
        // 模板必须存在
        let template = workflow_template::Entity::find_by_id(template_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Template not found: {template_id}"))?;

        let now = chrono::Utc::now().timestamp();

        let existing = workflow_marketplace::Entity::find()
            .filter(workflow_marketplace::Column::TemplateId.eq(template_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(m) = existing {
            // 已有记录：置 is_public = true
            let mut am: workflow_marketplace::ActiveModel = m.into();
            am.is_public = Set(true);
            am.updated_at = Set(now);
            am.update(db).await.map_err(|e| e.to_string())?;
        } else {
            // 无记录：插入新行，id 与 template_id 一致以便关联
            let am = workflow_marketplace::ActiveModel {
                id: Set(uuid::Uuid::new_v4().to_string()),
                template_id: Set(template.id.clone()),
                author_id: Set("local".to_string()),
                name: Set(template.name.clone()),
                description: Set(template.description.clone()),
                category: Set("general".to_string()),
                icon: Set(template.icon.clone()),
                tags: Set(template.tags.clone()),
                downloads: Set(0),
                rating_average: Set(0.0),
                rating_count: Set(0),
                is_featured: Set(false),
                is_verified: Set(false),
                is_public: Set(true),
                created_at: Set(now),
                updated_at: Set(now),
            };
            am.insert(db).await.map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    async fn uninstall_template(
        &self,
        db: &DatabaseConnection,
        template_id: &str,
    ) -> Result<(), String> {
        let existing = workflow_marketplace::Entity::find()
            .filter(workflow_marketplace::Column::TemplateId.eq(template_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Template not installed: {template_id}"))?;

        let mut am: workflow_marketplace::ActiveModel = existing.into();
        am.is_public = Set(false);
        am.updated_at = Set(chrono::Utc::now().timestamp());
        am.update(db).await.map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn publish_template(
        &self,
        db: &DatabaseConnection,
        template_id: &str,
        category: Option<String>,
        tags: Vec<String>,
    ) -> Result<(), String> {
        // 发布前必须先 install（即 workflow_marketplace 表中存在记录）
        let existing = workflow_marketplace::Entity::find()
            .filter(workflow_marketplace::Column::TemplateId.eq(template_id))
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Template not installed, install first: {template_id}"))?;

        let mut am: workflow_marketplace::ActiveModel = existing.into();
        if let Some(cat) = category {
            am.category = Set(cat);
        }
        if !tags.is_empty() {
            am.tags = Set(Some(serde_json::to_string(&tags).unwrap_or_default()));
        }
        am.is_public = Set(true);
        am.updated_at = Set(chrono::Utc::now().timestamp());
        am.update(db).await.map_err(|e| e.to_string())?;

        Ok(())
    }
}
