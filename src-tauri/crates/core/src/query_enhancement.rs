use std::pin::Pin;
use std::sync::Arc;

use crate::error::Result;
use crate::types::*;

/// LLM 调用函数类型（由调用方注入）
pub type QueryLlmFn = Arc<
    dyn Fn(String) -> Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>
        + Send
        + Sync,
>;

/// 查询增强器 —— 将用户查询变换为多路增强查询
#[derive(Clone)]
pub struct QueryEnhancer {
    config: EnhancementConfig,
    /// 调用 LLM 完成文本生成的函数指针，由调用方注入
    llm_fn: QueryLlmFn,
}

impl QueryEnhancer {
    pub fn new(
        config: EnhancementConfig,
        llm_fn: impl Fn(String) -> Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            config,
            llm_fn: Arc::new(llm_fn),
        }
    }

    /// 对原始查询进行增强，返回增强后的查询列表。
    /// 若 config.enabled == false 或 strategy == None，直接返回原始查询。
    pub async fn enhance(&self, query: &str) -> Result<Vec<EnhancedQuery>> {
        if !self.config.enabled {
            return Ok(vec![EnhancedQuery {
                text: query.to_string(),
                strategy: EnhancementStrategy::None,
                weight: 1.0,
            }]);
        }

        match self.config.strategy {
            EnhancementStrategy::None => Ok(vec![EnhancedQuery {
                text: query.to_string(),
                strategy: EnhancementStrategy::None,
                weight: 1.0,
            }]),
            EnhancementStrategy::Hyde => self.enhance_hyde(query).await,
            EnhancementStrategy::MultiQuery => self.enhance_multi_query(query).await,
            EnhancementStrategy::Decomposition => self.enhance_decomposition(query).await,
            EnhancementStrategy::Auto => self.enhance_auto(query).await,
        }
    }

    async fn enhance_hyde(&self, query: &str) -> Result<Vec<EnhancedQuery>> {
        let prompt = format!(
            "你是一个知识助手。请针对以下问题，写一段简洁的百科式答案（100-200字），\
             包含关键事实和专业术语。\n\n问题：{query}\n\n假设答案："
        );

        let hyde_answer = (self.llm_fn)(prompt).await?;
        let trimmed = hyde_answer.trim().to_string();
        if trimmed.is_empty() {
            return Ok(vec![EnhancedQuery {
                text: query.to_string(),
                strategy: EnhancementStrategy::Hyde,
                weight: 1.0,
            }]);
        }

        Ok(vec![EnhancedQuery {
            text: trimmed,
            strategy: EnhancementStrategy::Hyde,
            weight: 1.0,
        }])
    }

    async fn enhance_multi_query(&self, query: &str) -> Result<Vec<EnhancedQuery>> {
        let prompt = format!(
            "你是一个搜索查询优化器。将用户问题改写为 {n} 个不同视角的搜索查询，\
             每个查询聚焦问题的不同方面。返回 JSON 数组。\n\n\
             用户问题：{query}\n\n\
             返回格式：[\"查询1\", \"查询2\", ...]",
            n = self.config.max_variants.min(5)
        );

        let response = (self.llm_fn)(prompt).await?;
        let variants = parse_json_string_array(&response);

        if variants.is_empty() {
            return Ok(vec![EnhancedQuery {
                text: query.to_string(),
                strategy: EnhancementStrategy::MultiQuery,
                weight: 1.0,
            }]);
        }

        let count = variants.len();
        Ok(variants
            .into_iter()
            .take(self.config.max_variants)
            .map(|text| EnhancedQuery {
                text,
                strategy: EnhancementStrategy::MultiQuery,
                weight: 1.0 / count as f32,
            })
            .collect())
    }

    async fn enhance_decomposition(&self, query: &str) -> Result<Vec<EnhancedQuery>> {
        let prompt = format!(
            "将以下复杂问题分解为 2-4 个简单的子问题，每个子问题独立可回答。\
             返回 JSON 数组。\n\n复杂问题：{query}\n\n\
             返回格式：[\"子问题1\", \"子问题2\", ...]"
        );

        let response = (self.llm_fn)(prompt).await?;
        let sub_queries = parse_json_string_array(&response);

        if sub_queries.is_empty() {
            return Ok(vec![EnhancedQuery {
                text: query.to_string(),
                strategy: EnhancementStrategy::Decomposition,
                weight: 1.0,
            }]);
        }

        Ok(sub_queries
            .into_iter()
            .map(|text| EnhancedQuery {
                text,
                strategy: EnhancementStrategy::Decomposition,
                weight: 1.0,
            })
            .collect())
    }

    async fn enhance_auto(&self, query: &str) -> Result<Vec<EnhancedQuery>> {
        let has_conceptual = query.contains("什么是")
            || query.contains("解释")
            || query.contains("原理")
            || query.contains("概念")
            || query.contains("总结")
            || query.contains("概述");
        let is_complex = query.len() > 40
            || query.contains("并且")
            || query.contains("同时")
            || query.contains("对比")
            || query.contains("区别")
            || query.contains("先后");

        if is_complex {
            self.enhance_multi_query(query).await
        } else if has_conceptual {
            self.enhance_hyde(query).await
        } else {
            Ok(vec![EnhancedQuery {
                text: query.to_string(),
                strategy: EnhancementStrategy::None,
                weight: 1.0,
            }])
        }
    }
}

/// 从 LLM 响应中提取 JSON 字符串数组
fn parse_json_string_array(raw: &str) -> Vec<String> {
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(raw.trim()) {
        return arr;
    }
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str::<Vec<String>>(cleaned).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_string_array_direct() {
        let result = parse_json_string_array(r#"["a", "b", "c"]"#);
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_json_string_array_markdown() {
        let result = parse_json_string_array("```json\n[\"x\"]\n```");
        assert_eq!(result, vec!["x"]);
    }

    #[test]
    fn test_parse_json_string_array_invalid() {
        let result = parse_json_string_array("not json at all");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_empty_string() {
        let result = parse_json_string_array("");
        assert!(result.is_empty());
    }
}
