// SPDX-License-Identifier: AGPL-3.0-only
//! C5.1 修复：实现 `NewsArchiveSink` trait，把 astock-data 抓回的新闻
//! 入库到 `news_archive` 表，使 as-of 模式下 `search_news` 能命中本地语料库。
//!
//! ## 架构定位
//!
//! 本模块属于 wiring 层（`src/init/`），把 dao 的 `news_archive` repo
//! 适配成 astock-data 定义的 `NewsArchiveSink` trait，再通过
//! `AStockClient::with_news_archive_sink` 注入。
//!
//! - astock-data（implementor）只定义 trait，不依赖 dao
//! - dao（implementor）提供 `upsert_batch` / `search_asof` 函数式 API
//! - 本模块（wiring）做胶水转换，把二者粘合

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use axagent_astock_data::{NewsArchiveSink, NewsItem, parse_news_publish_time_ms};
use axagent_dao::repo::news_archive::{
    ArchivedNews, NewsArchiveEntry, search_asof as dao_search_asof, upsert_batch as dao_upsert_batch,
};
use sha2::{Digest, Sha256};

/// 用 url 的 sha256 hex 前 32 字符作为 article_code 兜底。
///
/// C5.2 修复：`UNIQUE(source, article_code)` 对 NULL 失效，
/// 因此 article_code 为 None 时必须用 url hash 兜底，避免 NULL
/// 导致同一 source 的多条记录绕过去重。
fn sha256_hex_prefix(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let bytes = hasher.finalize();
    // 取前 32 字符（128 bit），足够去重且不会过长
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    hex.chars().take(32).collect()
}

/// 把 `NewsItem` 转换为 dao 层的 `NewsArchiveEntry`。
///
/// - `article_code` 始终用 url 的 sha256 hash 兜底（C5.2 NOT NULL 约束）
/// - `publish_time_ms` 由 `parse_news_publish_time_ms` 解析；解析失败跳过
fn news_item_to_entry(
    item: &NewsItem,
    source: &str,
    stock_code: Option<&str>,
    keyword: Option<&str>,
) -> Option<NewsArchiveEntry> {
    let publish_time_ms = parse_news_publish_time_ms(&item.publish_time)?;
    // C5.2: article_code 始终用 url hash（NOT NULL，避免 UNIQUE 对 NULL 失效）
    let article_code = sha256_hex_prefix(&item.url);
    let summary = if item.summary.is_empty() {
        None
    } else {
        Some(item.summary.clone())
    };
    let url = if item.url.is_empty() {
        None
    } else {
        Some(item.url.clone())
    };
    Some(NewsArchiveEntry {
        source: source.to_string(),
        article_code,
        title: item.title.clone(),
        summary,
        url,
        media_name: None,
        publish_time_ms,
        stock_code: stock_code.map(|s| s.to_string()),
        keyword: keyword.map(|s| s.to_string()),
        sentiment_score: item.sentiment_score,
    })
}

/// 把 dao 返回的 `ArchivedNews` 转回 `NewsItem`，供 astock-data 上层使用。
fn archived_to_news(a: ArchivedNews) -> NewsItem {
    NewsItem {
        title: a.title,
        summary: a.summary.unwrap_or_default(),
        source: a.source,
        url: a.url.unwrap_or_default(),
        publish_time: a.publish_time,
        sentiment_score: a.sentiment_score,
    }
}

/// `NewsArchiveSink` 的 dao-backed 实现。
pub struct NewsArchiveSinkImpl {
    db: DatabaseConnection,
}

impl NewsArchiveSinkImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
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
            .filter_map(|n| news_item_to_entry(n, source, stock_code, keyword))
            .collect();
        if entries.is_empty() {
            return;
        }
        match dao_upsert_batch(&self.db, &entries).await {
            Ok(n) => {
                tracing::info!(
                    "[news_archive] upsert {} 条 (source={}, stock={:?}, keyword={:?}, 入库={})",
                    items.len(),
                    source,
                    stock_code,
                    keyword,
                    n
                );
            },
            Err(e) => {
                // trait 契约：失败仅记录日志，不影响主流程
                tracing::warn!(
                    "[news_archive] upsert 失败 (source={}, stock={:?}, keyword={:?}): {e}",
                    source,
                    stock_code,
                    keyword
                );
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
        match dao_search_asof(&self.db, keyword, stock_code, as_of_ts_ms, limit).await {
            Ok(rows) => {
                tracing::info!(
                    "[news_archive] search_asof 命中 {} 条 (keyword={}, stock={:?}, as_of={})",
                    rows.len(),
                    keyword,
                    stock_code,
                    as_of_ts_ms
                );
                rows.into_iter().map(archived_to_news).collect()
            },
            Err(e) => {
                tracing::warn!(
                    "[news_archive] search_asof 失败 (keyword={}, stock={:?}): {e}",
                    keyword,
                    stock_code
                );
                Vec::new()
            },
        }
    }
}
