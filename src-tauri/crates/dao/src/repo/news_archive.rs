// SPDX-License-Identifier: AGPL-3.0-only

//! `news_archive` 表的访问层
//!
//! 提供三组操作:
//! - `upsert_batch`: 实时抓回的 NewsItem 批量入库(vendor 抓完自动调)
//! - `search_asof`:  as-of 模式查询,截断 `publish_time <= as_of_ts_ms`
//! - `purge_older_than`: 定期清理(避免表无限增长)
//!
//! 去重策略:`UNIQUE(source, article_code)`。article_code 缺失时调用方应
//! 用 url 的 hash 兜底(dao 层不替调用方猜,避免错杀)。

use sea_orm::*;
use serde::{Deserialize, Serialize};

use axagent_entities::news_archive;
use axagent_harness::util_fns::gen_id;

use chrono::Utc;

/// 入库单条记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsArchiveEntry {
    pub source: String,
    pub article_code: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub url: Option<String>,
    pub media_name: Option<String>,
    pub publish_time_ms: i64,
    pub stock_code: Option<String>,
    pub keyword: Option<String>,
    pub sentiment_score: Option<f64>,
}

/// as-of 模式查询返回的精简 NewsItem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedNews {
    pub title: String,
    pub summary: Option<String>,
    pub source: String,
    pub media_name: Option<String>,
    pub url: Option<String>,
    pub publish_time: String, // 原始字符串(如有),否则 publish_time_ms 转 ISO
    pub publish_time_ms: i64,
    pub sentiment_score: Option<f64>,
}

/// 批量 upsert。`INSERT OR IGNORE` 命中 UNIQUE 索引则跳过,确保同一条
/// 新闻不会因重复抓取而膨胀。
///
/// 返回实际写入条数(已存在的不计)。
pub async fn upsert_batch(
    db: &DatabaseConnection,
    entries: &[NewsArchiveEntry],
) -> Result<u64, DbErr> {
    if entries.is_empty() {
        return Ok(0);
    }
    let now_ms = Utc::now().timestamp_millis();
    let mut count = 0u64;
    // 用 ActiveModel::insert + try catch 替代 ON CONFLICT,避免在 SQLite
    // 上依赖特定 SQL 扩展。每条失败(UQ 冲突)不算错。
    for e in entries {
        let id = gen_id();
        let am = news_archive::ActiveModel {
            id: Set(id),
            source: Set(e.source.clone()),
            article_code: Set(e.article_code.clone()),
            title: Set(e.title.clone()),
            summary: Set(e.summary.clone()),
            url: Set(e.url.clone()),
            media_name: Set(e.media_name.clone()),
            publish_time: Set(e.publish_time_ms),
            stock_code: Set(e.stock_code.clone()),
            keyword: Set(e.keyword.clone()),
            fetched_at: Set(now_ms),
            sentiment_score: Set(e.sentiment_score),
        };
        match am.insert(db).await {
            Ok(_) => count += 1,
            Err(DbErr::Exec(_)) | Err(DbErr::Query(_)) => {
                // UNIQUE(source, article_code) 冲突,静默跳过
            },
            Err(e) => return Err(e),
        }
    }
    Ok(count)
}

/// as-of 模式查询:`publish_time <= as_of_ts_ms` 内的最新 limit 条。
///
/// 关键词匹配 `title` 或 `summary` 子串(LIKE %kw%)。stock_code 给定
/// 时额外过滤;None 表示不按个股过滤。
pub async fn search_asof(
    db: &DatabaseConnection,
    keyword: &str,
    stock_code: Option<&str>,
    as_of_ts_ms: i64,
    limit: u32,
) -> Result<Vec<ArchivedNews>, DbErr> {
    let mut q = news_archive::Entity::find()
        .filter(news_archive::Column::PublishTime.lte(as_of_ts_ms))
        .order_by_desc(news_archive::Column::PublishTime)
        .limit(limit as u64);

    if let Some(code) = stock_code {
        q = q.filter(news_archive::Column::StockCode.eq(code));
    }

    if !keyword.is_empty() {
        // LIKE 模糊匹配 title OR summary。escape 单引号防 SQL 注入。
        let escaped = keyword.replace('\'', "''");
        let pattern = format!("%{escaped}%");
        q = q.filter(
            Condition::any()
                .add(news_archive::Column::Title.like(&pattern))
                .add(news_archive::Column::Summary.like(&pattern)),
        );
    }

    let rows = q.all(db).await?;
    Ok(rows.into_iter().map(row_to_archived).collect())
}

fn row_to_archived(r: news_archive::Model) -> ArchivedNews {
    let publish_time = chrono::DateTime::from_timestamp_millis(r.publish_time)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default();
    ArchivedNews {
        title: r.title,
        summary: r.summary,
        source: r.source,
        media_name: r.media_name,
        url: r.url,
        publish_time,
        publish_time_ms: r.publish_time,
        sentiment_score: r.sentiment_score,
    }
}

/// 清理早于 `before_ts_ms` 的旧记录,返回删除条数。
///
/// 建议每月由 cron 触发一次,保留窗口默认 1 年(由调用方传入阈值)。
pub async fn purge_older_than(db: &DatabaseConnection, before_ts_ms: i64) -> Result<u64, DbErr> {
    let res = news_archive::Entity::delete_many()
        .filter(news_archive::Column::PublishTime.lt(before_ts_ms))
        .exec(db)
        .await?;
    Ok(res.rows_affected)
}
