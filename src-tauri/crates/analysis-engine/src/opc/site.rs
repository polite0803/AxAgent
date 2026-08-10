// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ── 网站分析指标（stock-analysis 域） ──────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteMetric {
    pub id: i64,
    pub site_type: SiteType,
    pub url: String,
    pub metric_name: String,
    pub value: f64,
    pub unit: String,
    pub collected_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SiteType {
    #[default]
    Website,
    Blog,
    Social,
    Marketplace,
    App,
    Other,
}

impl std::fmt::Display for SiteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SiteType::Website => write!(f, "website"),
            SiteType::Blog => write!(f, "blog"),
            SiteType::Social => write!(f, "social"),
            SiteType::Marketplace => write!(f, "marketplace"),
            SiteType::App => write!(f, "app"),
            SiteType::Other => write!(f, "other"),
        }
    }
}

impl std::str::FromStr for SiteType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "website" => Ok(SiteType::Website),
            "blog" => Ok(SiteType::Blog),
            "social" => Ok(SiteType::Social),
            "marketplace" => Ok(SiteType::Marketplace),
            "app" => Ok(SiteType::App),
            "other" => Ok(SiteType::Other),
            _ => Err(format!("Unknown site type: {}", s)),
        }
    }
}

impl SiteMetric {
    pub fn new(
        site_type: SiteType,
        url: impl Into<String>,
        metric_name: impl Into<String>,
        value: f64,
        unit: impl Into<String>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_secs() as i64;
        Self {
            id: 0,
            site_type,
            url: url.into(),
            metric_name: metric_name.into(),
            value,
            unit: unit.into(),
            collected_at: now,
        }
    }
}

// ── OPC 站点/落地页/博客管理 ─────────────────────────────────────

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use axagent_entities::{
    opc_blog_posts, opc_contact_submissions, opc_content_assets, opc_landing_pages,
    opc_publish_schedules,
};
use axagent_harness::util_fns::{gen_id, now_ts};

use super::content_asset::{
    content_asset_entity_to_dto, ContentAsset, ContentAssetService, CreateContentAssetInput,
    UpdateContentAssetInput,
};
use super::error::{OpcError, OpcResult};
use super::publish_schedule::{
    publish_schedule_entity_to_dto, CreatePublishScheduleInput, PublishSchedule,
    PublishScheduleService, UpdatePublishScheduleInput,
};

/// 落地页
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandingPage {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub content: String,
    pub published: bool,
    pub published_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLandingPageInput {
    pub title: String,
    pub slug: String,
    pub description: String,
    pub content: String,
}

/// 博客文章
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPost {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub content: String,
    pub tags: Vec<String>,
    pub published: bool,
    pub published_at: Option<i64>,
    pub view_count: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBlogPostInput {
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub content: String,
    pub tags: Vec<String>,
}

/// 联系表单提交
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactSubmission {
    pub id: String,
    pub name: String,
    pub email: String,
    pub message: String,
    pub source: String,
    pub read: bool,
    pub created_at: i64,
}

// ── SiteService Trait ──────────────────────────────────────────

#[async_trait]
pub trait SiteService: Send + Sync {
    async fn create_landing_page(&self, input: CreateLandingPageInput) -> OpcResult<LandingPage>;
    async fn get_landing_page(&self, id: &str) -> OpcResult<LandingPage>;
    async fn list_landing_pages(&self) -> OpcResult<Vec<LandingPage>>;
    async fn publish_landing_page(&self, id: &str) -> OpcResult<LandingPage>;

    async fn create_blog_post(&self, input: CreateBlogPostInput) -> OpcResult<BlogPost>;
    async fn get_blog_post(&self, id: &str) -> OpcResult<BlogPost>;
    async fn list_blog_posts(&self) -> OpcResult<Vec<BlogPost>>;
    async fn publish_blog_post(&self, id: &str) -> OpcResult<BlogPost>;

    async fn submit_contact(
        &self,
        name: &str,
        email: &str,
        message: &str,
        source: &str,
    ) -> OpcResult<ContactSubmission>;
    async fn list_contacts(&self) -> OpcResult<Vec<ContactSubmission>>;
    async fn mark_contact_read(&self, id: &str) -> OpcResult<()>;
}

/// Noop 实现
#[derive(Debug)]
pub struct NoopSiteService;

#[async_trait]
impl SiteService for NoopSiteService {
    async fn create_landing_page(&self, _: CreateLandingPageInput) -> OpcResult<LandingPage> {
        Err(OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn get_landing_page(&self, _: &str) -> OpcResult<LandingPage> {
        Err(OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn list_landing_pages(&self) -> OpcResult<Vec<LandingPage>> {
        Ok(Vec::new())
    }
    async fn publish_landing_page(&self, _: &str) -> OpcResult<LandingPage> {
        Err(OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn create_blog_post(&self, _: CreateBlogPostInput) -> OpcResult<BlogPost> {
        Err(OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn get_blog_post(&self, _: &str) -> OpcResult<BlogPost> {
        Err(OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn list_blog_posts(&self) -> OpcResult<Vec<BlogPost>> {
        Ok(Vec::new())
    }
    async fn publish_blog_post(&self, _: &str) -> OpcResult<BlogPost> {
        Err(OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn submit_contact(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> OpcResult<ContactSubmission> {
        Err(OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn list_contacts(&self) -> OpcResult<Vec<ContactSubmission>> {
        Ok(Vec::new())
    }
    async fn mark_contact_read(&self, _: &str) -> OpcResult<()> {
        Ok(())
    }
}

// ── Entity ↔ DTO 转换 ──────────────────────────────────────────

fn landing_entity_to_dto(e: opc_landing_pages::Model) -> LandingPage {
    LandingPage {
        id: e.id,
        title: e.title,
        slug: e.slug,
        description: e.description,
        content: e.content,
        published: e.published != 0,
        published_at: e.published_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}

fn blog_entity_to_dto(e: opc_blog_posts::Model) -> BlogPost {
    let tags: Vec<String> = serde_json::from_str(&e.tags_json).unwrap_or_default();
    BlogPost {
        id: e.id,
        title: e.title,
        slug: e.slug,
        excerpt: e.excerpt,
        content: e.content,
        tags,
        published: e.published != 0,
        published_at: e.published_at,
        view_count: e.view_count,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}

fn contact_entity_to_dto(e: opc_contact_submissions::Model) -> ContactSubmission {
    ContactSubmission {
        id: e.id,
        name: e.name,
        email: e.email,
        message: e.message,
        source: e.source,
        read: e.is_read != 0,
        created_at: e.created_at,
    }
}

// ── DefaultSiteService (SeaORM) ────────────────────────────────

/// 默认站点服务实现
pub struct DefaultSiteService {
    pub db: DatabaseConnection,
}

impl DefaultSiteService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SiteService for DefaultSiteService {
    async fn create_landing_page(&self, input: CreateLandingPageInput) -> OpcResult<LandingPage> {
        let now = now_ts();
        let slug = input.slug.trim().to_lowercase().replace(' ', "-");
        let am = opc_landing_pages::ActiveModel {
            id: Set(gen_id()),
            title: Set(input.title),
            slug: Set(slug),
            description: Set(input.description),
            content: Set(input.content),
            published: Set(0),
            published_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let entity = am.insert(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(landing_entity_to_dto(entity))
    }

    async fn get_landing_page(&self, id: &str) -> OpcResult<LandingPage> {
        let entity = opc_landing_pages::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("LandingPage {id}")))?;
        Ok(landing_entity_to_dto(entity))
    }

    async fn list_landing_pages(&self) -> OpcResult<Vec<LandingPage>> {
        let entities = opc_landing_pages::Entity::find()
            .order_by_desc(opc_landing_pages::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(entities.into_iter().map(landing_entity_to_dto).collect())
    }

    async fn publish_landing_page(&self, id: &str) -> OpcResult<LandingPage> {
        let entity = opc_landing_pages::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("LandingPage {id}")))?;
        let mut am: opc_landing_pages::ActiveModel = entity.into();
        am.published = Set(1);
        am.published_at = Set(Some(now_ts()));
        am.updated_at = Set(now_ts());
        let updated = am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(landing_entity_to_dto(updated))
    }

    async fn create_blog_post(&self, input: CreateBlogPostInput) -> OpcResult<BlogPost> {
        let now = now_ts();
        let slug = input.slug.trim().to_lowercase().replace(' ', "-");
        let tags_json = serde_json::to_string(&input.tags).unwrap_or_else(|_| "[]".to_string());
        let am = opc_blog_posts::ActiveModel {
            id: Set(gen_id()),
            title: Set(input.title),
            slug: Set(slug),
            excerpt: Set(input.excerpt),
            content: Set(input.content),
            tags_json: Set(tags_json),
            published: Set(0),
            published_at: Set(None),
            view_count: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let entity = am.insert(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(blog_entity_to_dto(entity))
    }

    async fn get_blog_post(&self, id: &str) -> OpcResult<BlogPost> {
        let entity = opc_blog_posts::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("BlogPost {id}")))?;
        Ok(blog_entity_to_dto(entity))
    }

    async fn list_blog_posts(&self) -> OpcResult<Vec<BlogPost>> {
        let entities = opc_blog_posts::Entity::find()
            .order_by_desc(opc_blog_posts::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(entities.into_iter().map(blog_entity_to_dto).collect())
    }

    async fn publish_blog_post(&self, id: &str) -> OpcResult<BlogPost> {
        let entity = opc_blog_posts::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("BlogPost {id}")))?;
        let mut am: opc_blog_posts::ActiveModel = entity.into();
        am.published = Set(1);
        am.published_at = Set(Some(now_ts()));
        am.updated_at = Set(now_ts());
        let updated = am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(blog_entity_to_dto(updated))
    }

    async fn submit_contact(
        &self,
        name: &str,
        email: &str,
        message: &str,
        source: &str,
    ) -> OpcResult<ContactSubmission> {
        let now = now_ts();
        let am = opc_contact_submissions::ActiveModel {
            id: Set(gen_id()),
            name: Set(name.to_string()),
            email: Set(email.to_string()),
            message: Set(message.to_string()),
            source: Set(source.to_string()),
            is_read: Set(0),
            created_at: Set(now),
        };
        let entity = am.insert(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(contact_entity_to_dto(entity))
    }

    async fn list_contacts(&self) -> OpcResult<Vec<ContactSubmission>> {
        let entities = opc_contact_submissions::Entity::find()
            .order_by_desc(opc_contact_submissions::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(entities.into_iter().map(contact_entity_to_dto).collect())
    }

    async fn mark_contact_read(&self, id: &str) -> OpcResult<()> {
        let entity = opc_contact_submissions::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("ContactSubmission {id}")))?;
        let mut am: opc_contact_submissions::ActiveModel = entity.into();
        am.is_read = Set(1);
        am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(())
    }
}

// ── ContentAssetService impl for DefaultSiteService ───────────────

#[async_trait]
impl ContentAssetService for DefaultSiteService {
    async fn create_content_asset(
        &self,
        input: CreateContentAssetInput,
    ) -> OpcResult<ContentAsset> {
        let now = now_ts();
        let tags_json = serde_json::to_string(&input.tags).unwrap_or_else(|_| "[]".to_string());
        let am = opc_content_assets::ActiveModel {
            id: Set(gen_id()),
            title: Set(input.title),
            content_type: Set(input.content_type),
            body: Set(input.body),
            tags_json: Set(tags_json),
            status: Set("draft".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let entity = am.insert(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(content_asset_entity_to_dto(entity))
    }

    async fn get_content_asset(&self, id: &str) -> OpcResult<ContentAsset> {
        let entity = opc_content_assets::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("ContentAsset {id}")))?;
        Ok(content_asset_entity_to_dto(entity))
    }

    async fn list_content_assets(&self) -> OpcResult<Vec<ContentAsset>> {
        let entities = opc_content_assets::Entity::find()
            .order_by_desc(opc_content_assets::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(entities.into_iter().map(content_asset_entity_to_dto).collect())
    }

    async fn update_content_asset(
        &self,
        id: &str,
        input: UpdateContentAssetInput,
    ) -> OpcResult<ContentAsset> {
        let entity = opc_content_assets::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("ContentAsset {id}")))?;
        let mut am: opc_content_assets::ActiveModel = entity.into();
        if let Some(title) = input.title {
            am.title = Set(title);
        }
        if let Some(content_type) = input.content_type {
            am.content_type = Set(content_type);
        }
        if let Some(body) = input.body {
            am.body = Set(body);
        }
        if let Some(tags) = input.tags {
            am.tags_json = Set(serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string()));
        }
        if let Some(status) = input.status {
            am.status = Set(status);
        }
        am.updated_at = Set(now_ts());
        let updated = am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(content_asset_entity_to_dto(updated))
    }

    async fn delete_content_asset(&self, id: &str) -> OpcResult<()> {
        let result = opc_content_assets::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        if result.rows_affected == 0 {
            return Err(OpcError::NotFound(format!("ContentAsset {id}")));
        }
        Ok(())
    }
}

// ── PublishScheduleService impl for DefaultSiteService ───────────────

#[async_trait]
impl PublishScheduleService for DefaultSiteService {
    async fn create_publish_schedule(
        &self,
        input: CreatePublishScheduleInput,
    ) -> OpcResult<PublishSchedule> {
        let now = now_ts();
        let am = opc_publish_schedules::ActiveModel {
            id: Set(gen_id()),
            content_ref_type: Set(input.content_ref_type),
            content_ref_id: Set(input.content_ref_id),
            scheduled_at: Set(input.scheduled_at),
            status: Set("pending".to_string()),
            published_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let entity = am.insert(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(publish_schedule_entity_to_dto(entity))
    }

    async fn get_publish_schedule(&self, id: &str) -> OpcResult<PublishSchedule> {
        let entity = opc_publish_schedules::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("PublishSchedule {id}")))?;
        Ok(publish_schedule_entity_to_dto(entity))
    }

    async fn list_publish_schedules(&self) -> OpcResult<Vec<PublishSchedule>> {
        let entities = opc_publish_schedules::Entity::find()
            .order_by_desc(opc_publish_schedules::Column::ScheduledAt)
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(entities.into_iter().map(publish_schedule_entity_to_dto).collect())
    }

    async fn update_publish_schedule(
        &self,
        id: &str,
        input: UpdatePublishScheduleInput,
    ) -> OpcResult<PublishSchedule> {
        let entity = opc_publish_schedules::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("PublishSchedule {id}")))?;
        let mut am: opc_publish_schedules::ActiveModel = entity.into();
        if let Some(scheduled_at) = input.scheduled_at {
            am.scheduled_at = Set(scheduled_at);
        }
        if let Some(status) = input.status {
            am.status = Set(status);
        }
        am.updated_at = Set(now_ts());
        let updated = am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(publish_schedule_entity_to_dto(updated))
    }

    async fn delete_publish_schedule(&self, id: &str) -> OpcResult<()> {
        let result = opc_publish_schedules::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        if result.rows_affected == 0 {
            return Err(OpcError::NotFound(format!("PublishSchedule {id}")));
        }
        Ok(())
    }

    async fn process_due_schedules(&self) -> OpcResult<Vec<PublishSchedule>> {
        let now = now_ts();
        let due_schedules = opc_publish_schedules::Entity::find()
            .filter(opc_publish_schedules::Column::Status.eq("pending"))
            .filter(opc_publish_schedules::Column::ScheduledAt.lte(now))
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;

        let mut published_results = Vec::new();

        for schedule in due_schedules {
            let schedule_id = schedule.id.clone();
            let content_ref_type = schedule.content_ref_type.clone();
            let content_ref_id = schedule.content_ref_id.clone();

            // 尝试发布内容，返回是否成功
            let publish_success = match content_ref_type.as_str() {
                "blog_post" => {
                    // 发布博客文章
                    self.publish_blog_post(&content_ref_id).await.is_ok()
                },
                "content_asset" => {
                    // 更新内容资产状态为 published
                    use super::content_asset::{ContentAssetService, UpdateContentAssetInput};
                    let input = UpdateContentAssetInput {
                        title: None,
                        content_type: None,
                        body: None,
                        tags: None,
                        status: Some("published".to_string()),
                    };
                    self.update_content_asset(&content_ref_id, input).await.is_ok()
                },
                _ => false,
            };

            // 根据发布结果更新计划状态
            let status = if publish_success {
                "published".to_string()
            } else {
                "failed".to_string()
            };

            let mut am: opc_publish_schedules::ActiveModel = schedule.into();
            am.status = Set(status);
            am.published_at = Set(Some(now));
            am.updated_at = Set(now);

            match am.update(&self.db).await {
                Ok(updated) => {
                    published_results.push(publish_schedule_entity_to_dto(updated));
                },
                Err(e) => {
                    tracing::error!("Failed to update publish schedule {}: {}", schedule_id, e);
                },
            }
        }

        Ok(published_results)
    }
}
