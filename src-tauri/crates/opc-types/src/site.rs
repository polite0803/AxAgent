// SPDX-License-Identifier: AGPL-3.0-only

//! 站点/落地页/博客管理领域 — DTO 定义与 trait 接口

use serde::{Deserialize, Serialize};

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

use crate::OpcResult;

#[async_trait::async_trait]
pub trait SiteService: Send + Sync {
    // Landing pages
    async fn create_landing_page(&self, input: CreateLandingPageInput) -> OpcResult<LandingPage>;
    async fn get_landing_page(&self, id: &str) -> OpcResult<LandingPage>;
    async fn list_landing_pages(&self) -> OpcResult<Vec<LandingPage>>;
    async fn publish_landing_page(&self, id: &str) -> OpcResult<LandingPage>;

    // Blog posts
    async fn create_blog_post(&self, input: CreateBlogPostInput) -> OpcResult<BlogPost>;
    async fn get_blog_post(&self, id: &str) -> OpcResult<BlogPost>;
    async fn list_blog_posts(&self) -> OpcResult<Vec<BlogPost>>;
    async fn publish_blog_post(&self, id: &str) -> OpcResult<BlogPost>;

    // Contacts
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

#[derive(Debug)]
pub struct NoopSiteService;

#[async_trait::async_trait]
impl SiteService for NoopSiteService {
    async fn create_landing_page(&self, _: CreateLandingPageInput) -> OpcResult<LandingPage> {
        Err(crate::OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn get_landing_page(&self, _: &str) -> OpcResult<LandingPage> {
        Err(crate::OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn list_landing_pages(&self) -> OpcResult<Vec<LandingPage>> {
        Ok(Vec::new())
    }
    async fn publish_landing_page(&self, _: &str) -> OpcResult<LandingPage> {
        Err(crate::OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn create_blog_post(&self, _: CreateBlogPostInput) -> OpcResult<BlogPost> {
        Err(crate::OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn get_blog_post(&self, _: &str) -> OpcResult<BlogPost> {
        Err(crate::OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn list_blog_posts(&self) -> OpcResult<Vec<BlogPost>> {
        Ok(Vec::new())
    }
    async fn publish_blog_post(&self, _: &str) -> OpcResult<BlogPost> {
        Err(crate::OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn submit_contact(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> OpcResult<ContactSubmission> {
        Err(crate::OpcError::NotFound("SiteService not implemented".into()))
    }
    async fn list_contacts(&self) -> OpcResult<Vec<ContactSubmission>> {
        Ok(Vec::new())
    }
    async fn mark_contact_read(&self, _: &str) -> OpcResult<()> {
        Ok(())
    }
}
