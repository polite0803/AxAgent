use std::collections::HashSet;
use std::sync::Arc;

use axagent_core::entity::{notes, note_links, note_backlinks, wikis};
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
            return Ok("No relevant information found in this wiki to answer your question.".to_string());
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
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(prompt),
                    tool_calls: None,
                    tool_call_id: None,
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

    pub async fn get_page_context(
        &self,
        note_id: &str,
        depth: usize,
    ) -> Result<String, String> {
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
