//! v012 — news_archive.article_code 改为 NOT NULL
//!
//! ## 背景
//!
//! v010 建表时 `article_code TEXT`（可空），配合 `UNIQUE(source, article_code)`
//! 去重。但 SQLite 中 NULL ≠ NULL，多条 article_code=NULL 的记录不会触发
//! UNIQUE 冲突，导致同一文章可重复入库。
//!
//! C5.2 修复：
//! 1. 应用层（NewsArchiveSinkImpl）始终用 url/title 的 sha256 hash 兜底，
//!    保证 article_code 永不为空。
//! 2. 本 migration 将列改为 NOT NULL，DDL 层兜底。
//!
//! ## 幂等保护
//!
//! 检查 `pragma_table_info` 的 `notnull` 标志：若已为 NOT NULL 则跳过。
//!
//! ## 实现方式
//!
//! SQLite 不支持 `ALTER TABLE ... ALTER COLUMN`，采用表重建策略：
//! 1. 建新表（article_code TEXT NOT NULL）
//! 2. 复制数据（NULL → 用 random hash 兜底，避免 UNIQUE 冲突）
//! 3. drop 旧表 + rename
//! 4. 重建索引

use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // 幂等检查：article_code 是否已是 NOT NULL — 使用 PRAGMA table_info 逐行读取
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "PRAGMA table_info('news_archive')".to_string(),
        ))
        .await?;
    let already_notnull: bool = rows.iter().any(|row| {
        let name: Option<String> = row.try_get_by("name").ok();
        if name.as_deref() != Some("article_code") {
            return false;
        }
        // `notnull` 列：0 = nullable, 1 = NOT NULL
        let nn: i32 = row.try_get_by("notnull").unwrap_or(0);
        nn == 1
    });
    if already_notnull {
        return Ok(());
    }

    // 1. 建新表（article_code TEXT NOT NULL）
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS news_archive_new (\
            id TEXT NOT NULL PRIMARY KEY, \
            source TEXT NOT NULL, \
            article_code TEXT NOT NULL, \
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
    )
    .await?;

    // 2. 复制数据 — NULL article_code 用 random hash 兜底
    //    COALESCE 保证 NOT NULL；hex(randomblob(16)) 生成唯一占位符
    db.execute_unprepared(
        "INSERT OR IGNORE INTO news_archive_new \
         (id, source, article_code, title, summary, url, media_name, \
          publish_time, stock_code, keyword, fetched_at, sentiment_score) \
         SELECT id, source, \
                COALESCE(article_code, 'gen_' || hex(randomblob(16))), \
                title, summary, url, media_name, \
                publish_time, stock_code, keyword, fetched_at, sentiment_score \
         FROM news_archive",
    )
    .await?;

    // 3. drop 旧表 + rename
    db.execute_unprepared("DROP TABLE news_archive").await?;
    db.execute_unprepared("ALTER TABLE news_archive_new RENAME TO news_archive").await?;

    // 4. 重建索引
    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_news_archive_publish ON news_archive(publish_time)",
        "CREATE INDEX IF NOT EXISTS idx_news_archive_stock ON news_archive(stock_code, publish_time)",
        "CREATE INDEX IF NOT EXISTS idx_news_archive_keyword ON news_archive(keyword, publish_time)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    Ok(())
}
