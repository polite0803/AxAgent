//! v004 — news_archive 表(本地新闻语料库)
//!
//! 解决 as-of 模式下 `search_news` 报"当下语义"降级的问题:
//! - vendor 端(东方财富/雪球/同花顺/iwencai/akshare 等)搜索 API 都不支持
//!   `begin_time/end_time` 参数,无法直接拿"as_of_date 当时的搜索结果"。
//! - 本表把每次实时抓回的 NewsItem 持久化,as-of 模式查
//!   `WHERE publish_time <= as_of_ts` 拿到截止当时的本地索引。
//!
//! 设计要点:
//! - `UNIQUE(source, article_code)` 是去重关键。article_code 缺失时 fallback
//!   到 url,确保每个新闻最多一行。
//! - publish_time 存 unix 毫秒(精度足够,as_of 截断用 `<=` 比较)。
//! - stock_code + keyword 是触发此次抓取的上下文,便于后续按"某只股票"或
//!   "某关键词"做窄查询(非 as-of 模式下也能用)。
//! - 两组索引:publish_time 单列(全局 as-of 扫描),stock_code+publish_time
//!   复合(单股时序扫描)。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    for sql in &[
        "CREATE TABLE IF NOT EXISTS news_archive (\
            id TEXT NOT NULL PRIMARY KEY, \
            source TEXT NOT NULL, \
            article_code TEXT, \
            title TEXT NOT NULL, \
            summary TEXT, \
            url TEXT, \
            media_name TEXT, \
            publish_time INTEGER NOT NULL, \
            stock_code TEXT, \
            keyword TEXT, \
            fetched_at INTEGER NOT NULL, \
            sentiment_score REAL, \
            UNIQUE(source, article_code))",
        "CREATE INDEX IF NOT EXISTS idx_news_archive_publish \
            ON news_archive(publish_time)",
        "CREATE INDEX IF NOT EXISTS idx_news_archive_stock \
            ON news_archive(stock_code, publish_time)",
        "CREATE INDEX IF NOT EXISTS idx_news_archive_keyword \
            ON news_archive(keyword, publish_time)",
    ] {
        db.execute_unprepared(sql).await?;
    }
    Ok(())
}
