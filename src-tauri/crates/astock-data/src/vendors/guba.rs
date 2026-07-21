//! 东方财富股吧 (guba.eastmoney.com) 社交舆情数据源
//!
//! 通过东方财富股吧 HTML 页面获取个股讨论热度、帖子数、情感倾向。
//! 仅实现 `get_social_sentiment`，其他行情/财务类方法返回空或错误，
//! 由路由层 (`VendorRouting`) 自动降级到其他 vendor。

use crate::error::DataError;
use crate::types::*;
use crate::vendors::StockVendor;
use async_trait::async_trait;
use serde_json::Value;

pub struct GubaVendor {
    pub http: reqwest::Client,
}

impl GubaVendor {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    /// 带反爬头的 GET 请求（返回 HTML 文本）
    ///
    /// 修复(2026-07-21): 原接口 `guba.eastmoney.com/interface/GetData.aspx`
    /// 已废弃(返回 Content-Length: 0)。改用 `guba.eastmoney.com/list,{code}.html`
    /// HTML 页面,该页面在 `<script>` 标签中嵌入了 `var article_list=[...]`
    /// 格式的 JSON 数据。需要用 text() 拿原文后手动剥 JSON。
    async fn guba_get_html(&self, url: &str) -> Result<String, DataError> {
        let resp = self
            .http
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Referer", "https://guba.eastmoney.com/")
            .header("Accept", "text/html, application/xhtml+xml, */*")
            .send()
            .await
            .map_err(DataError::from)?;

        if !resp.status().is_success() {
            return Err(DataError::VendorError {
                vendor: "guba".into(),
                message: format!("HTTP {}", resp.status()),
            });
        }

        resp.text().await.map_err(DataError::from)
    }

    /// 从 HTML 文本中提取 `var article_list={"re":[...]}` 中的 `re` 数组字符串
    ///
    /// 实际页面格式(2026-07-21 验证):
    ///   `var article_list={"re":[{...},{...}],...,"bar_name":"华如科技",...};    var other_list=...`
    ///   (注意 `};` 和 `var` 之间可能有多个空格)
    ///
    /// 返回完整的 `[...]` 数组字符串(含方括号),可直接 serde_json::from_str。
    ///
    /// 定位策略:
    ///   1. 找 `article_list=` 后的 `{` 开头对象
    ///   2. 用 `var other_list` 作为对象结束锚点(空格数量可变),向前找最近的 `};`
    ///   3. 在对象内找 `"re":[` 子串定位数组开始
    ///   4. 用 **平衡括号法** 找数组结束 `]`(因为帖子对象内有嵌套的 `]`,
    ///      如 `user_extendinfos` 字段结束,简单字符串匹配会误判)
    fn extract_article_list_json(html: &str) -> Option<&str> {
        let key = "article_list=";
        let start_idx = html.find(key)? + key.len();
        let rest = &html[start_idx..];

        // article_list 后面是 { 开头的对象
        if !rest.starts_with('{') {
            return None;
        }

        // 先确定 article_list 对象的范围
        // 实际页面用 `};    var other_list` (多空格) 作为对象结束标记
        // 用 `var other_list` 作为锚点(空格数量可变),向前找最近的 `};`
        let anchor = "var other_list";
        let anchor_rel = rest.find(anchor)?;
        let before_anchor = &rest[..anchor_rel];
        let obj_end_rel = before_anchor.rfind("};")?;
        let obj_str = &rest[..obj_end_rel + 1]; // 包含 `}`

        // 在对象内找 "re":[ 子串
        let re_key = "\"re\":[";
        let re_start_rel = obj_str.find(re_key)?;
        // 数组开始位置 = "re":[ 中的 [ 的位置
        let arr_start = re_start_rel + re_key.len() - 1; // 指向 `[`
        let arr_rest = &obj_str[arr_start..]; // 以 `[` 开头

        // 用平衡括号法找数组的结束 `]`
        // (帖子对象内有嵌套的 ],如 user_extendinfos 字段,简单字符串匹配会误判)
        let end_rel = Self::find_balanced_array_end(arr_rest)?;
        Some(&arr_rest[..end_rel + 1]) // 包含 `]`
    }

    /// 用平衡括号法找 JSON 数组的结束位置
    ///
    /// 输入 `s` 必须以 `[` 开头。返回匹配的 `]` 的位置(从 0 开始)。
    /// 处理字符串内的 `[`/`]`(忽略)、转义字符 `\\`、嵌套数组/对象。
    fn find_balanced_array_end(s: &str) -> Option<usize> {
        if !s.starts_with('[') {
            return None;
        }
        let bytes = s.as_bytes();
        let mut depth: i32 = 0;
        let mut in_str = false;
        let mut escape = false;
        for (i, &b) in bytes.iter().enumerate() {
            if escape {
                escape = false;
                continue;
            }
            match b {
                b'\\' if in_str => escape = true,
                b'"' => in_str = !in_str,
                b'[' | b'{' if !in_str => depth += 1,
                b']' | b'}' if !in_str => {
                    depth -= 1;
                    if depth == 0 && b == b']' {
                        return Some(i);
                    }
                },
                _ => {},
            }
        }
        None
    }
}

#[async_trait]
impl StockVendor for GubaVendor {
    // ── 行情/财务类方法：股吧不提供，返回空或错误，由路由层降级 ──

    async fn get_quote(&self, _stock_code: &str) -> Result<StockQuote, DataError> {
        Err(DataError::VendorError {
            vendor: "guba".into(), message: "股吧不提供行情数据".into()
        })
    }

    async fn get_klines(
        &self,
        _stock_code: &str,
        _period: &str,
        _limit: u32,
        _adj: Option<AdjType>,
    ) -> Result<Vec<KLine>, DataError> {
        Ok(vec![])
    }

    async fn get_financials(&self, _stock_code: &str) -> Result<Vec<FinancialReport>, DataError> {
        Ok(vec![])
    }

    async fn get_news(&self, _stock_code: &str, _limit: u32) -> Result<Vec<NewsItem>, DataError> {
        Ok(vec![])
    }

    async fn get_money_flow(&self, _: &str) -> Result<Option<MoneyFlow>, DataError> {
        Ok(None)
    }

    async fn get_dragon_tiger(&self, _: &str) -> Result<Vec<DragonTigerEntry>, DataError> {
        Ok(vec![])
    }

    async fn get_lockup_schedule(&self, _: &str) -> Result<Vec<LockupSchedule>, DataError> {
        Ok(vec![])
    }

    async fn search_stock(&self, _: &str) -> Result<Vec<StockSearchResult>, DataError> {
        Ok(vec![])
    }

    // ── 社交舆情：核心实现 ──

    /// 获取股吧社交舆情数据
    ///
    /// 修复(2026-07-21): 原接口 `GetData.aspx` 已废弃,返回空 body。
    /// 改用 `https://guba.eastmoney.com/list,{code}.html` HTML 页面,
    /// 从嵌入的 `var article_list=[...]` 中解析帖子数据。
    ///
    /// 情感分析基于帖子标题关键词（"涨"/"牛"/"利好"/"加仓" → 看多,
    /// "跌"/"熊"/"利空"/"减仓" → 看空）。
    async fn get_social_sentiment(
        &self,
        stock_code: &str,
    ) -> Result<Vec<SocialSentiment>, DataError> {
        let url = format!("https://guba.eastmoney.com/list,{stock_code}.html");
        let html = self.guba_get_html(&url).await?;

        // 从 HTML 中提取 article_list JSON 数组
        let json_str =
            Self::extract_article_list_json(&html).ok_or_else(|| DataError::VendorError {
                vendor: "guba".into(),
                message: format!("股吧 HTML 中未找到 article_list 数据, stock_code={stock_code}"),
            })?;

        let posts: Vec<Value> = serde_json::from_str(json_str).map_err(|e| {
            DataError::ParseError(format!(
                "股吧 article_list JSON 解析失败: {e}, preview={}",
                &json_str[..json_str.len().min(200)]
            ))
        })?;

        if posts.is_empty() {
            return Ok(vec![SocialSentiment {
                stock_code: stock_code.to_string(),
                stock_name: String::new(),
                platform: "guba".to_string(),
                post_count: 0,
                hot_rank: None,
                sentiment_score: None,
                bull_ratio: None,
                fetched_at: chrono::Utc::now().timestamp(),
            }]);
        }

        // 统计帖子数 + 简单情感分析
        let post_count = posts.len() as u32;
        let mut bull_count = 0u32;
        let mut bear_count = 0u32;
        let mut stock_name = String::new();

        for post in &posts {
            let title = post["post_title"].as_str().unwrap_or("");
            // 提取股票名称(从第一条帖子的 stockbar_name 字段)
            if stock_name.is_empty() {
                if let Some(name) = post["stockbar_name"].as_str() {
                    // 去掉 "华如科技吧" 的 "吧" 后缀
                    stock_name = name.trim_end_matches("吧").to_string();
                }
            }

            // 基于关键词的情感分析（粗略估计）
            if title.contains("涨")
                || title.contains("牛")
                || title.contains("利好")
                || title.contains("加仓")
            {
                bull_count += 1;
            } else if title.contains("跌")
                || title.contains("熊")
                || title.contains("利空")
                || title.contains("减仓")
            {
                bear_count += 1;
            }
        }

        let total = bull_count + bear_count;
        let bull_ratio = if total > 0 {
            Some(bull_count as f64 / total as f64)
        } else {
            None
        };
        // sentiment_score: -1.0 ~ 1.0，bull_ratio 0.5 → 0.0
        let sentiment_score = bull_ratio.map(|r| (r - 0.5) * 2.0);

        Ok(vec![SocialSentiment {
            stock_code: stock_code.to_string(),
            stock_name,
            platform: "guba".to_string(),
            post_count,
            hot_rank: None,
            sentiment_score,
            bull_ratio,
            fetched_at: chrono::Utc::now().timestamp(),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_article_list_json_finds_array() {
        // 实际页面格式: article_list={"re":[...],"bar_name":"..."}
        // 注意 `};` 和 `var` 之间有多个空格(实际页面格式)
        let html = r#"<script>var article_list={"re":[{"post_id":1,"post_title":"涨"},{"post_id":2,"post_title":"跌"}],"count":2,"bar_name":"华如科技","bar_code":"301302"};    var other_list={"re":[]};</script>"#;
        let json_str = GubaVendor::extract_article_list_json(html).unwrap();
        assert!(json_str.starts_with('['));
        assert!(json_str.ends_with(']'));
        let posts: Vec<Value> = serde_json::from_str(json_str).unwrap();
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0]["post_title"].as_str(), Some("涨"));
        assert_eq!(posts[1]["post_title"].as_str(), Some("跌"));
    }

    #[test]
    fn extract_article_list_json_returns_none_when_missing() {
        let html = "<html>no article_list here</html>";
        assert!(GubaVendor::extract_article_list_json(html).is_none());
    }

    #[test]
    fn extract_article_list_json_handles_empty_re() {
        // re 为空数组的情况
        let html = r#"<script>var article_list={"re":[],"count":0,"bar_name":null}; var other_list={"re":[]};</script>"#;
        let json_str = GubaVendor::extract_article_list_json(html).unwrap();
        assert_eq!(json_str, "[]");
        let posts: Vec<Value> = serde_json::from_str(json_str).unwrap();
        assert!(posts.is_empty());
    }

    #[test]
    fn extract_article_list_json_handles_nested_arrays_in_posts() {
        // 帖子对象内有嵌套数组(如 modules:["a","b"]),不能用简单字符串匹配
        // 必须用平衡括号法才能正确找到 re 数组的结束
        let html = r#"<script>var article_list={"re":[{"post_id":1,"post_title":"涨","modules":["news","video"],"user_extendinfos":{"medal_list":["a","b"]}},{"post_id":2,"post_title":"跌"}],"count":2};    var other_list={"re":[]};</script>"#;
        let json_str = GubaVendor::extract_article_list_json(html).unwrap();
        assert!(json_str.starts_with('['));
        assert!(json_str.ends_with(']'));
        let posts: Vec<Value> = serde_json::from_str(json_str).unwrap();
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0]["post_title"].as_str(), Some("涨"));
        assert_eq!(posts[0]["modules"][0].as_str(), Some("news"));
        assert_eq!(posts[1]["post_title"].as_str(), Some("跌"));
    }
}
