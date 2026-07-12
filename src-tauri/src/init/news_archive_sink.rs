// SPDX-License-Identifier: AGPL-3.0-only

//! C5.1: NewsArchiveSink 的数据库实现。
//!
//! astock-data 定义了 `NewsArchiveSink` trait 但从未注入实现，
//! 导致 `news_archive` 表永不写入。本模块提供基于 `axagent_dao` 的
//! 具体实现，在 `create_app_state` 中通过 `with_news_archive_sink` 注入。
//!
//! C5.2: `article_code` 始终用 url 的 sha256 hash 兜底，
//! 确保 NOT NULL 约束满足，UNIQUE(source, article_code) 不会因 NULL 失效。

use axagent_astock_data::NewsArchiveSink;
use axagent_astock_data::types::NewsItem;
use axagent_dao::repo::news_archive::{self, ArchivedNews, NewsArchiveEntry};
use sea_orm::DatabaseConnection;
use sha2::{Digest, Sha256};

pub struct NewsArchiveSinkImpl {
    db: DatabaseConnection,
}

impl NewsArchiveSinkImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// sha256 → hex 字符串
    fn sha256_hex(s: &str) -> String {
        let mut h = Sha256::new();
        h.update(s.as_bytes());
        h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// 把 NewsItem.publish_time(String) 解析为 unix 毫秒。
    /// 逻辑与 astock-data 内部 parse_news_publish_time_ms 一致。
    fn parse_publish_time_ms(s: &str) -> Option<i64> {
        use chrono::NaiveDateTime;
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
            return dt.and_utc().timestamp_millis().into();
        }
        if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            return d.and_hms_opt(0, 0, 0)?.and_utc().timestamp_millis().into();
        }
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
            return dt.and_utc().timestamp_millis().into();
        }
        None
    }

    /// NewsItem → NewsArchiveEntry（含 C5.2 article_code 兜底）
    fn to_entry(
        item: &NewsItem,
        source: &str,
        stock_code: Option<&str>,
        keyword: Option<&str>,
    ) -> Option<NewsArchiveEntry> {
        let publish_time_ms = Self::parse_publish_time_ms(&item.publish_time)?;
        // C5.2: article_code 始终非空 — url 优先，空则用 title
        let hash_input = if item.url.is_empty() {
            &item.title
        } else {
            &item.url
        };
        let article_code = Self::sha256_hex(hash_input);
        Some(NewsArchiveEntry {
            source: source.to_string(),
            article_code,
            title: item.title.clone(),
            summary: if item.summary.is_empty() {
                None
            } else {
                Some(item.summary.clone())
            },
            url: if item.url.is_empty() {
                None
            } else {
                Some(item.url.clone())
            },
            media_name: None,
            publish_time_ms,
            stock_code: stock_code.map(|s| s.to_string()),
            keyword: keyword.map(|s| s.to_string()),
            sentiment_score: item.sentiment_score,
        })
    }

    /// ArchivedNews → NewsItem
    fn archived_to_news(r: ArchivedNews) -> NewsItem {
        NewsItem {
            title: r.title,
            summary: r.summary.unwrap_or_default(),
            source: r.source,
            url: r.url.unwrap_or_default(),
            publish_time: r.publish_time,
            sentiment_score: r.sentiment_score,
        }
    }
}

#[async_trait::async_trait]
impl NewsArchiveSink for NewsArchiveSinkImpl {
    async fn upsert(
        &self,
        source: &str,
        stock_code: Option<&str>,
        keyword: Option<&str>,
        items: &[NewsItem],
    ) {
        let entries: Vec<NewsArchiveEntry> = items
            .iter()
            .filter_map(|item| Self::to_entry(item, source, stock_code, keyword))
            .collect();
        if entries.is_empty() {
            return;
        }
        match news_archive::upsert_batch(&self.db, &entries).await {
            Ok(n) => {
                tracing::debug!(
                    "[NewsArchiveSink] upsert {} 条 (新增 {}), source={}",
                    entries.len(),
                    n,
                    source
                );
            },
            Err(e) => {
                tracing::warn!("[NewsArchiveSink] upsert 失败: {}", e);
            },
        }
    }

    async fn search_asof(
        &self,
        keyword: &str,
        stock_code: Option<&str>,
        as_of_ts_ms: i64,
        limit: u32,
    ) -> Vec<NewsItem> {
        match news_archive::search_asof(&self.db, keyword, stock_code, as_of_ts_ms, limit).await {
            Ok(rows) => rows.into_iter().map(Self::archived_to_news).collect(),
            Err(e) => {
                tracing::warn!("[NewsArchiveSink] search_asof 失败: {}", e);
                vec![]
            },
        }
    }
}
