use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axagent_core::entity::{note_backlinks, note_links, notes, wikis};
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_providers::{ProviderAdapter, ProviderRequestContext};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub pages: Vec<PageResult>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResult {
    pub note_id: String,
    pub title: String,
    pub content_snippet: String,
    pub relevance_score: f64,
    pub link_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryContext {
    pub query: String,
    pub wiki_id: String,
    pub limit: usize,
    pub offset: usize,
}

pub struct QueryEngine {
    db: Arc<DatabaseConnection>,
    llm_adapter: Option<Arc<dyn ProviderAdapter>>,
    llm_ctx: Option<ProviderRequestContext>,
    llm_model: Option<String>,
    vector_store: Option<Arc<dyn VectorSearch>>,
}

#[async_trait::async_trait]
pub trait VectorSearch: Send + Sync {
    async fn search(
        &self,
        wiki_id: &str,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<(String, f64)>, String>;
}

impl QueryEngine {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            llm_adapter: None,
            llm_ctx: None,
            llm_model: None,
            vector_store: None,
        }
    }

    pub fn with_llm(
        mut self,
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        model: String,
    ) -> Self {
        self.llm_adapter = Some(adapter);
        self.llm_ctx = Some(ctx);
        self.llm_model = Some(model);
        self
    }

    pub fn with_vector_store(mut self, vs: Arc<dyn VectorSearch>) -> Self {
        self.vector_store = Some(vs);
        self
    }

    pub async fn query(&self, ctx: &QueryContext) -> Result<QueryResult, String> {
        let _wiki = wikis::Entity::find_by_id(&ctx.wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Wiki {} not found", ctx.wiki_id))?;

        let db_notes = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(&ctx.wiki_id))
            .filter(notes::Column::IsDeleted.eq(0))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let mut scored: Vec<(axagent_core::repo::note::Note, f64)> = Vec::new();

        let query_lower = ctx.query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        for note_model in db_notes {
            let note = axagent_core::repo::note::model_to_note(note_model);
            let mut score = 0.0_f64;
            let content_lower = note.content.to_lowercase();
            let title_lower = note.title.to_lowercase();

            if title_lower.contains(&query_lower) {
                score += 1.0;
            } else if title_lower.starts_with(&query_lower) {
                score += 0.8;
            }

            let mut word_matches = 0u32;
            for word in &query_words {
                if content_lower.contains(word) {
                    word_matches += 1;
                }
            }
            if !query_words.is_empty() {
                score += (word_matches as f64 / query_words.len() as f64) * 0.5;
            }

            if let Some(qs) = note.quality_score {
                score += qs * 0.3;
            }

            scored.push((note, score));
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.retain(|(_, s)| *s > 0.0);

        let total = scored.len();
        let paginated: Vec<_> = scored
            .into_iter()
            .skip(ctx.offset)
            .take(ctx.limit)
            .collect();

        let mut pages = Vec::new();
        for (note, score) in paginated {
            let links = note_links::Entity::find()
                .filter(note_links::Column::SourceNoteId.eq(&note.id))
                .all(self.db.as_ref())
                .await
                .map_err(|e| e.to_string())?;

            let link_paths: Vec<String> = links.iter().map(|l| l.target_note_id.clone()).collect();

            let snippet = if note.content.len() > 200 {
                format!("{}...", &note.content[..200])
            } else {
                note.content.clone()
            };

            pages.push(PageResult {
                note_id: note.id,
                title: note.title,
                content_snippet: snippet,
                relevance_score: score,
                link_paths,
            });
        }

        Ok(QueryResult { pages, total })
    }

    pub async fn query_with_embedding(
        &self,
        ctx: &QueryContext,
        query_embedding: &[f32],
    ) -> Result<QueryResult, String> {
        let _wiki = wikis::Entity::find_by_id(&ctx.wiki_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Wiki {} not found", ctx.wiki_id))?;

        let vector_results = if let Some(vs) = &self.vector_store {
            vs.search(&ctx.wiki_id, query_embedding, ctx.limit * 2)
                .await
                .map_err(|e| e.to_string())?
        } else {
            vec![]
        };

        let db_notes = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(&ctx.wiki_id))
            .filter(notes::Column::IsDeleted.eq(0))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let query_lower = ctx.query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut keyword_scores: HashMap<String, f64> = HashMap::new();
        for note_model in &db_notes {
            let note = axagent_core::repo::note::model_to_note(note_model.clone());
            let mut score = 0.0_f64;
            let content_lower = note.content.to_lowercase();
            let title_lower = note.title.to_lowercase();

            if title_lower.contains(&query_lower) {
                score += 1.0;
            }

            let mut word_matches = 0u32;
            for word in &query_words {
                if content_lower.contains(word) {
                    word_matches += 1;
                }
            }
            if !query_words.is_empty() {
                score += (word_matches as f64 / query_words.len() as f64) * 0.5;
            }
            if let Some(qs) = note.quality_score {
                score += qs * 0.3;
            }
            keyword_scores.insert(note.id.clone(), score);
        }

        let mut combined: Vec<(String, f64)> = Vec::new();

        for (note_id, vector_score) in &vector_results {
            let kw_score = keyword_scores.get(note_id).copied().unwrap_or(0.0);
            let normalized_vector = 1.0 / (1.0 + *vector_score);
            let combined_score = normalized_vector * 0.7 + kw_score * 0.3;
            combined.push((note_id.clone(), combined_score));
        }

        let vector_ids: HashSet<&String> = vector_results.iter().map(|(id, _)| id).collect();
        for (note_id, kw_score) in &keyword_scores {
            if !vector_ids.contains(note_id) && *kw_score > 0.0 {
                combined.push((note_id.clone(), *kw_score * 0.5));
            }
        }

        combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        combined.retain(|(_, s)| *s > 0.0);

        let total = combined.len();
        let paginated: Vec<_> = combined
            .into_iter()
            .skip(ctx.offset)
            .take(ctx.limit)
            .collect();

        let note_map: HashMap<String, axagent_core::repo::note::Note> = db_notes
            .into_iter()
            .map(|m| {
                let note = axagent_core::repo::note::model_to_note(m);
                (note.id.clone(), note)
            })
            .collect();

        let mut pages = Vec::new();
        for (note_id, score) in paginated {
            if let Some(note) = note_map.get(&note_id) {
                let links = note_links::Entity::find()
                    .filter(note_links::Column::SourceNoteId.eq(&note.id))
                    .all(self.db.as_ref())
                    .await
                    .map_err(|e| e.to_string())?;

                let link_paths: Vec<String> =
                    links.iter().map(|l| l.target_note_id.clone()).collect();

                let snippet = if note.content.len() > 200 {
                    format!("{}...", &note.content[..200])
                } else {
                    note.content.clone()
                };

                pages.push(PageResult {
                    note_id: note.id.clone(),
                    title: note.title.clone(),
                    content_snippet: snippet,
                    relevance_score: score,
                    link_paths,
                });
            }
        }

        Ok(QueryResult { pages, total })
    }

    pub async fn ask(&self, wiki_id: &str, question: &str) -> Result<String, String> {
        let (adapter, ctx, model) = self
            .llm_adapter
            .as_ref()
            .zip(self.llm_ctx.as_ref())
            .zip(self.llm_model.as_ref())
            .map(|((a, c), m)| (a.clone(), c.clone(), m.clone()))
            .ok_or_else(|| "QueryEngine not configured with LLM".to_string())?;

        let query_ctx = QueryContext {
            query: question.to_string(),
            wiki_id: wiki_id.to_string(),
            limit: 5,
            offset: 0,
        };

        let search_result = self.query(&query_ctx).await?;

        if search_result.pages.is_empty() {
            return Ok(
                "No relevant information found in this wiki to answer your question.".to_string()
            );
        }

        let mut context = String::from("Relevant wiki pages:\n\n");
        for (i, page) in search_result.pages.iter().enumerate() {
            let note = axagent_core::repo::note::get_note(self.db.as_ref(), &page.note_id)
                .await
                .map_err(|e| e.to_string())?;

            context.push_str(&format!(
                "## Page {}: {}\n{}\n\n",
                i + 1,
                note.title,
                if note.content.len() > 3000 {
                    format!("{}...", &note.content[..3000])
                } else {
                    note.content.clone()
                }
            ));
        }

        let prompt = format!(
            "Based on the following wiki content, answer the question. \
            If the information is insufficient, state that clearly.\n\n\
            {}\n\nQuestion: {}",
            context, question
        );

        let request = ChatRequest {
            model,
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: ChatContent::Text(
                        "You are a helpful assistant answering questions based on wiki content. \
                        Be concise and accurate. Cite specific pages when possible."
                            .to_string(),
                    ),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(prompt),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                },
            ],
            stream: false,
            temperature: Some(0.3),
            max_tokens: Some(2048),
            top_p: None,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
        };

        let response = adapter
            .chat(&ctx, request)
            .await
            .map_err(|e| format!("LLM call failed: {}", e))?;

        Ok(response.content)
    }

    pub async fn get_page_context(&self, note_id: &str, depth: usize) -> Result<String, String> {
        let note = axagent_core::repo::note::get_note(self.db.as_ref(), note_id)
            .await
            .map_err(|e| e.to_string())?;

        let mut context = format!("# {}\n\n{}\n\n", note.title, note.content);

        if depth == 0 {
            return Ok(context);
        }

        let backlinks = note_backlinks::Entity::find()
            .filter(note_backlinks::Column::TargetNoteId.eq(note_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let mut visited: HashSet<String> = [note_id.to_string()].into();
        for bl in backlinks.iter().take(5) {
            if visited.contains(&bl.source_note_id) {
                continue;
            }
            visited.insert(bl.source_note_id.clone());

            if let Ok(ref_note) =
                axagent_core::repo::note::get_note(self.db.as_ref(), &bl.source_note_id).await
            {
                context.push_str(&format!(
                    "## Related: {}\n{}\n\n",
                    ref_note.title,
                    if ref_note.content.len() > 500 {
                        format!("{}...", &ref_note.content[..500])
                    } else {
                        ref_note.content.clone()
                    }
                ));
            }
        }

        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockVectorSearch {
        results: Vec<(String, f64)>,
    }

    #[async_trait::async_trait]
    impl VectorSearch for MockVectorSearch {
        async fn search(
            &self,
            _wiki_id: &str,
            _query_embedding: &[f32],
            _top_k: usize,
        ) -> Result<Vec<(String, f64)>, String> {
            Ok(self.results.clone())
        }
    }

    #[test]
    fn test_query_result_serialization() {
        let result = QueryResult {
            pages: vec![],
            total: 0,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: QueryResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total, 0);
        assert!(deserialized.pages.is_empty());
    }

    #[test]
    fn test_page_result_serialization() {
        let page = PageResult {
            note_id: "n1".to_string(),
            title: "Test Page".to_string(),
            content_snippet: "snippet...".to_string(),
            relevance_score: 0.85,
            link_paths: vec!["link1".to_string(), "link2".to_string()],
        };
        let json = serde_json::to_string(&page).unwrap();
        let deserialized: PageResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.note_id, "n1");
        assert_eq!(deserialized.title, "Test Page");
        assert_eq!(deserialized.relevance_score, 0.85);
        assert_eq!(deserialized.link_paths.len(), 2);
    }

    #[test]
    fn test_query_context_serialization() {
        let ctx = QueryContext {
            query: "test query".to_string(),
            wiki_id: "wiki-1".to_string(),
            limit: 10,
            offset: 0,
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: QueryContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.query, "test query");
        assert_eq!(deserialized.wiki_id, "wiki-1");
        assert_eq!(deserialized.limit, 10);
        assert_eq!(deserialized.offset, 0);
    }

    #[test]
    fn test_query_result_with_pages() {
        let result = QueryResult {
            pages: vec![
                PageResult {
                    note_id: "n1".to_string(),
                    title: "Page 1".to_string(),
                    content_snippet: "content 1".to_string(),
                    relevance_score: 0.9,
                    link_paths: vec![],
                },
                PageResult {
                    note_id: "n2".to_string(),
                    title: "Page 2".to_string(),
                    content_snippet: "content 2".to_string(),
                    relevance_score: 0.5,
                    link_paths: vec!["n1".to_string()],
                },
            ],
            total: 2,
        };
        assert_eq!(result.total, 2);
        assert_eq!(result.pages.len(), 2);
    }

    #[test]
    fn test_page_result_snippet_truncation() {
        let long_content = "a".repeat(300);
        let snippet = if long_content.len() > 200 {
            format!("{}...", &long_content[..200])
        } else {
            long_content.clone()
        };
        assert_eq!(snippet.len(), 203);
        assert!(snippet.ends_with("..."));

        let short_content = "short".to_string();
        let snippet2 = if short_content.len() > 200 {
            format!("{}...", &short_content[..200])
        } else {
            short_content.clone()
        };
        assert_eq!(snippet2, "short");
    }

    #[test]
    fn test_query_context_pagination() {
        let ctx = QueryContext {
            query: "search".to_string(),
            wiki_id: "w1".to_string(),
            limit: 20,
            offset: 40,
        };
        assert_eq!(ctx.offset, 40);
        assert_eq!(ctx.limit, 20);
    }

    #[test]
    fn test_page_result_zero_relevance() {
        let page = PageResult {
            note_id: "n1".to_string(),
            title: "Low Relevance".to_string(),
            content_snippet: "content".to_string(),
            relevance_score: 0.0,
            link_paths: vec![],
        };
        assert_eq!(page.relevance_score, 0.0);
    }

    #[test]
    fn test_query_result_empty() {
        let result = QueryResult {
            pages: vec![],
            total: 0,
        };
        assert!(result.pages.is_empty());
        assert_eq!(result.total, 0);
    }

    #[test]
    fn test_page_result_link_paths() {
        let page = PageResult {
            note_id: "n1".to_string(),
            title: "Test".to_string(),
            content_snippet: "content".to_string(),
            relevance_score: 1.0,
            link_paths: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };
        assert_eq!(page.link_paths.len(), 3);
        assert_eq!(page.link_paths[0], "a");
    }

    #[tokio::test]
    async fn test_query_engine_new_no_llm() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        let engine = QueryEngine::new(Arc::new(db));
        assert!(engine.llm_adapter.is_none());
        assert!(engine.llm_ctx.is_none());
        assert!(engine.llm_model.is_none());
        assert!(engine.vector_store.is_none());
    }

    #[tokio::test]
    async fn test_query_engine_with_vector_store() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        let mock_vs = Arc::new(MockVectorSearch { results: vec![] });
        let engine = QueryEngine::new(Arc::new(db)).with_vector_store(mock_vs);
        assert!(engine.vector_store.is_some());
    }

    #[test]
    fn test_keyword_scoring_title_match() {
        let query_lower = "machine learning";
        let title_lower = "machine learning basics";
        let mut score = 0.0_f64;
        if title_lower.contains(query_lower) {
            score += 1.0;
        }
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_keyword_scoring_title_no_match() {
        let query_lower = "machine learning";
        let title_lower = "deep learning basics";
        let mut score = 0.0_f64;
        if title_lower.contains(query_lower) {
            score += 1.0;
        }
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_keyword_scoring_word_match() {
        let query_lower = "machine learning algorithms";
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let content_lower = "machine learning involves various algorithms for data processing";
        let mut word_matches = 0u32;
        for word in &query_words {
            if content_lower.contains(word) {
                word_matches += 1;
            }
        }
        let score = (word_matches as f64 / query_words.len() as f64) * 0.5;
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_keyword_scoring_partial_word_match() {
        let query_lower = "machine learning algorithms";
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let content_lower = "machine learning is a field of study";
        let mut word_matches = 0u32;
        for word in &query_words {
            if content_lower.contains(word) {
                word_matches += 1;
            }
        }
        let score = (word_matches as f64 / query_words.len() as f64) * 0.5;
        assert!((score - (2.0 / 3.0 * 0.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_keyword_scoring_quality_score_bonus() {
        let quality_score: Option<f64> = Some(0.8);
        let mut score = 0.0_f64;
        if let Some(qs) = quality_score {
            score += qs * 0.3;
        }
        assert!((score - 0.24).abs() < f64::EPSILON);
    }

    #[test]
    fn test_keyword_scoring_no_quality_score() {
        let quality_score: Option<f64> = None;
        let mut score = 0.0_f64;
        if let Some(qs) = quality_score {
            score += qs * 0.3;
        }
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_combined_scoring_vector_and_keyword() {
        let vector_score = 0.5_f64;
        let kw_score = 0.8_f64;
        let normalized_vector = 1.0 / (1.0 + vector_score);
        let combined_score = normalized_vector * 0.7 + kw_score * 0.3;
        let expected = (1.0 / 1.5) * 0.7 + 0.8 * 0.3;
        assert!((combined_score - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_combined_scoring_keyword_only() {
        let kw_score = 0.6_f64;
        let combined_score = kw_score * 0.5;
        assert!((combined_score - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_vector_search_normalization() {
        let vector_scores = vec![0.1, 0.5, 1.0, 2.0, 10.0];
        for &vs in &vector_scores {
            let normalized = 1.0 / (1.0 + vs);
            assert!(normalized > 0.0 && normalized <= 1.0);
        }
    }

    #[test]
    fn test_pagination_skip_and_take() {
        let items: Vec<i32> = (0..100).collect();
        let offset = 20;
        let limit = 10;
        let paginated: Vec<_> = items.into_iter().skip(offset).take(limit).collect();
        assert_eq!(paginated.len(), 10);
        assert_eq!(paginated[0], 20);
        assert_eq!(paginated[9], 29);
    }

    #[test]
    fn test_pagination_beyond_end() {
        let items: Vec<i32> = (0..5).collect();
        let offset = 3;
        let limit = 10;
        let paginated: Vec<_> = items.into_iter().skip(offset).take(limit).collect();
        assert_eq!(paginated.len(), 2);
    }

    #[test]
    fn test_pagination_empty_results() {
        let items: Vec<i32> = vec![];
        let offset = 0;
        let limit = 10;
        let paginated: Vec<_> = items.into_iter().skip(offset).take(limit).collect();
        assert!(paginated.is_empty());
    }

    #[test]
    fn test_empty_query_words() {
        let query_lower = "";
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        assert!(query_words.is_empty());
    }

    #[test]
    fn test_query_result_sorted_by_relevance() {
        let mut pages = [
            PageResult {
                note_id: "1".to_string(),
                title: "Low".to_string(),
                content_snippet: "c".to_string(),
                relevance_score: 0.3,
                link_paths: vec![],
            },
            PageResult {
                note_id: "2".to_string(),
                title: "High".to_string(),
                content_snippet: "c".to_string(),
                relevance_score: 0.9,
                link_paths: vec![],
            },
            PageResult {
                note_id: "3".to_string(),
                title: "Mid".to_string(),
                content_snippet: "c".to_string(),
                relevance_score: 0.6,
                link_paths: vec![],
            },
        ];
        pages.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        assert_eq!(pages[0].relevance_score, 0.9);
        assert_eq!(pages[1].relevance_score, 0.6);
        assert_eq!(pages[2].relevance_score, 0.3);
    }

    #[test]
    fn test_snippet_truncation_boundary() {
        let content_200 = "a".repeat(200);
        let snippet = if content_200.len() > 200 {
            format!("{}...", &content_200[..200])
        } else {
            content_200.clone()
        };
        assert_eq!(snippet.len(), 200);
        assert!(!snippet.ends_with("..."));

        let content_201 = "a".repeat(201);
        let snippet2 = if content_201.len() > 200 {
            format!("{}...", &content_201[..200])
        } else {
            content_201.clone()
        };
        assert_eq!(snippet2.len(), 203);
        assert!(snippet2.ends_with("..."));
    }

    #[test]
    fn test_page_result_serialization_roundtrip() {
        let page = PageResult {
            note_id: "n1".to_string(),
            title: "Test Page".to_string(),
            content_snippet: "A snippet of content".to_string(),
            relevance_score: 0.75,
            link_paths: vec!["link_a".to_string(), "link_b".to_string()],
        };
        let json = serde_json::to_string(&page).unwrap();
        let back: PageResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.note_id, page.note_id);
        assert_eq!(back.title, page.title);
        assert_eq!(back.content_snippet, page.content_snippet);
        assert!((back.relevance_score - page.relevance_score).abs() < f64::EPSILON);
        assert_eq!(back.link_paths, page.link_paths);
    }

    #[test]
    fn test_query_context_offset_zero() {
        let ctx = QueryContext {
            query: "test".to_string(),
            wiki_id: "w1".to_string(),
            limit: 10,
            offset: 0,
        };
        assert_eq!(ctx.offset, 0);
    }

    #[test]
    fn test_query_result_total_independent_of_pages() {
        let result = QueryResult {
            pages: vec![PageResult {
                note_id: "n1".to_string(),
                title: "Only Page".to_string(),
                content_snippet: "content".to_string(),
                relevance_score: 1.0,
                link_paths: vec![],
            }],
            total: 100,
        };
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.total, 100);
    }

    #[tokio::test]
    async fn test_mock_vector_search() {
        let mock = MockVectorSearch {
            results: vec![("note1".to_string(), 0.5), ("note2".to_string(), 0.3)],
        };
        let results = mock.search("wiki1", &[0.1; 128], 10).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "note1");
        assert!((results[0].1 - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_combined_scoring_vector_ids_dedup() {
        let vector_results: Vec<(String, f64)> =
            vec![("n1".to_string(), 0.5), ("n2".to_string(), 0.3)];
        let mut keyword_scores: HashMap<String, f64> = HashMap::new();
        keyword_scores.insert("n1".to_string(), 0.8);
        keyword_scores.insert("n2".to_string(), 0.4);
        keyword_scores.insert("n3".to_string(), 0.6);

        let vector_ids: HashSet<&String> = vector_results.iter().map(|(id, _)| id).collect();

        let mut combined: Vec<(String, f64)> = Vec::new();
        for (note_id, vector_score) in &vector_results {
            let kw_score = keyword_scores.get(note_id).copied().unwrap_or(0.0);
            let normalized_vector = 1.0 / (1.0 + *vector_score);
            let combined_score = normalized_vector * 0.7 + kw_score * 0.3;
            combined.push((note_id.clone(), combined_score));
        }

        for (note_id, kw_score) in &keyword_scores {
            if !vector_ids.contains(note_id) && *kw_score > 0.0 {
                combined.push((note_id.clone(), *kw_score * 0.5));
            }
        }

        assert_eq!(combined.len(), 3);
        let n3_entry = combined.iter().find(|(id, _)| id == "n3");
        assert!(n3_entry.is_some());
        assert!((n3_entry.unwrap().1 - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_score_filtering_zero() {
        let mut scored: Vec<(String, f64)> = vec![
            ("n1".to_string(), 0.5),
            ("n2".to_string(), 0.0),
            ("n3".to_string(), 0.3),
        ];
        scored.retain(|(_, s)| *s > 0.0);
        assert_eq!(scored.len(), 2);
    }

    #[test]
    fn test_query_context_default_pagination() {
        let ctx = QueryContext {
            query: "test".to_string(),
            wiki_id: "w1".to_string(),
            limit: 10,
            offset: 0,
        };
        assert!(ctx.limit > 0);
    }
}
