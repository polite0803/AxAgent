// SPDX-License-Identifier: AGPL-3.0-only

//! 站点管理服务实现 — SeaORM CRUD for landing pages, blog posts, contacts

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryOrder, Set};

use axagent_harness::util_fns::{gen_id, now_ts};
use axagent_opc_entities::{opc_blog_posts, opc_contact_submissions, opc_landing_pages};
use axagent_opc_types::{
    BlogPost, ContactSubmission, CreateBlogPostInput, CreateLandingPageInput, LandingPage,
    OpcError, OpcResult, SiteService,
};

// ── LandingPage 转换 ───────────────────────────────────────────

fn landing_entity_to_dto(e: opc_landing_pages::Model) -> LandingPage {
    LandingPage {
        id: e.id,
        title: e.title,
        slug: e.slug,
        description: e.description,
        content: e.content,
        published: e.published,
        published_at: e.published_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}

// ── BlogPost 转换 ──────────────────────────────────────────────

fn blog_entity_to_dto(e: opc_blog_posts::Model) -> BlogPost {
    let tags: Vec<String> = serde_json::from_str(&e.tags_json).unwrap_or_default();
    BlogPost {
        id: e.id,
        title: e.title,
        slug: e.slug,
        excerpt: e.excerpt,
        content: e.content,
        tags,
        published: e.published,
        published_at: e.published_at,
        view_count: e.view_count,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}

// ── ContactSubmission 转换 ─────────────────────────────────────

fn contact_entity_to_dto(e: opc_contact_submissions::Model) -> ContactSubmission {
    ContactSubmission {
        id: e.id,
        name: e.name,
        email: e.email,
        message: e.message,
        source: e.source,
        read: e.is_read,
        created_at: e.created_at,
    }
}

// ── DefaultSiteService ──────────────────────────────────────────

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
    // ── Landing Pages ──

    async fn create_landing_page(&self, input: CreateLandingPageInput) -> OpcResult<LandingPage> {
        let now = now_ts();
        let slug = input.slug.trim().to_lowercase().replace(' ', "-");
        let am = opc_landing_pages::ActiveModel {
            id: Set(gen_id()),
            title: Set(input.title),
            slug: Set(slug),
            description: Set(input.description),
            content: Set(input.content),
            published: Set(false),
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
        am.published = Set(true);
        am.published_at = Set(Some(now_ts()));
        am.updated_at = Set(now_ts());
        let updated = am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(landing_entity_to_dto(updated))
    }

    // ── Blog Posts ──

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
            published: Set(false),
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
        am.published = Set(true);
        am.published_at = Set(Some(now_ts()));
        am.updated_at = Set(now_ts());
        let updated = am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(blog_entity_to_dto(updated))
    }

    // ── Contact Submissions ──

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
            is_read: Set(false),
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
        am.is_read = Set(true);
        am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(())
    }
}
