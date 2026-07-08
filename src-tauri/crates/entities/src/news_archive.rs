use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 本地新闻语料库(由 v004 migration 创建)
///
/// 用途:实时抓回的 NewsItem 全部 upsert 到此表,as-of 模式 `search_news`
/// 改查 `WHERE publish_time <= as_of_ts`,拿到截止当时的本地索引。
///
/// 去重:UNIQUE(source, article_code)。article_code 缺失时由调用方用
/// url 的 hash 兜底(避免不同源抓到同 URL 被去重)。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "news_archive")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// vendor 标识(同 StockVendor::name)
    pub source: String,
    /// 第三方文章 id(无则用 url 的 hash)
    pub article_code: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub url: Option<String>,
    pub media_name: Option<String>,
    /// 发布时间的 unix 毫秒
    pub publish_time: i64,
    /// 关联个股(可空,关键词搜索时为空)
    pub stock_code: Option<String>,
    /// 触发此次抓取的关键词(可空,按个股抓时为空)
    pub keyword: Option<String>,
    /// 抓取入库的 unix 毫秒
    pub fetched_at: i64,
    /// 情感分数(预留,LLM 后处理填)
    pub sentiment_score: Option<f64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
