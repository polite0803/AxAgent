use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};

use axagent_core::entity::{notes, wiki_pages};

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
}

impl QueryEngine {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn query(&self, ctx: &QueryContext) -> Result<QueryResult, String> {
        let pages = self.search_pages(ctx).await?;
        let enriched = self.enrich_with_links(pages).await?;
        let scored = self.score_by_relevance(enriched, &ctx.query).await?;

        let total = scored.len();
        let pages_vec: Vec<PageResult> = scored
            .into_iter()
            .skip(ctx.offset)
            .take(ctx.limit)
            .collect();

        Ok(QueryResult {
            pages: pages_vec,
            total,
        })
    }

    async fn search_pages(&self, ctx: &QueryContext) -> Result<Vec<wiki_pages::Model>, String> {
        let pages = wiki_pages::Entity::find()
            .filter(wiki_pages::Column::WikiId.eq(&ctx.wiki_id))
            .order_by_asc(wiki_pages::Column::Title)
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        let note_ids: Vec<String> = pages.iter().map(|p| p.note_id.clone()).collect();

        let notes_map: HashMap<String, notes::Model> = notes::Entity::find()
            .filter(notes::Column::Id.is_in(note_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|n| (n.id.clone(), n))
            .collect();

        let query_lower = ctx.query.to_lowercase();
        let mut results: Vec<(wiki_pages::Model, f64)> = Vec::new();

        for page in pages {
            if let Some(note) = notes_map.get(&page.note_id) {
                let score = self.calculate_match_score(&page, note, &query_lower);
                if score > 0.0 {
                    results.push((page, score));
                }
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results.into_iter().map(|(p, _)| p).collect())
    }

    fn calculate_match_score(&self, page: &wiki_pages::Model, note: &notes::Model, query: &str) -> f64 {
        let mut score = 0.0_f64;

        if page.title.to_lowercase().contains(query) {
            score += 0.5;
        }

        if note.content.to_lowercase().contains(query) {
            score += 0.3;
        }

        if let Some(ref tags) = note.page_type {
            if tags.to_lowercase().contains(query) {
                score += 0.2;
            }
        }

        if let Some(ref related) = note.related_pages {
            if let Some(related_arr) = related.as_array() {
                score += related_arr.len() as f64 * 0.05;
            }
        }

        if let Some(qs) = page.quality_score {
            score *= qs;
        }

        score
    }

    async fn enrich_with_links(
        &self,
        pages: Vec<wiki_pages::Model>,
    ) -> Result<Vec<(wiki_pages::Model, Vec<String>)>, String> {
        let note_ids: Vec<String> = pages.iter().map(|p| p.note_id.clone()).collect();

        let links: Vec<(String, String)> = axagent_core::entity::note_links::Entity::find()
            .filter(axagent_core::entity::note_links::Column::SourceNoteId.is_in(
                note_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            ))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|l| (l.source_note_id, l.target_note_id))
            .collect();

        let mut page_links: HashMap<String, Vec<String>> = HashMap::new();
        for (source, target) in links {
            page_links.entry(source).or_default().push(target);
        }

        Ok(pages
            .into_iter()
            .map(|p| {
                let link_paths = page_links.get(&p.note_id).cloned().unwrap_or_default();
                (p, link_paths)
            })
            .collect())
    }

    async fn score_by_relevance(
        &self,
        pages: Vec<(wiki_pages::Model, Vec<String>)>,
        _query: &str,
    ) -> Result<Vec<PageResult>, String> {
        let note_ids: Vec<String> = pages.iter().map(|(p, _)| p.note_id.clone()).collect();

        let notes_map: HashMap<String, notes::Model> = notes::Entity::find()
            .filter(notes::Column::Id.is_in(note_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|n| (n.id.clone(), n))
            .collect();

        let results: Vec<PageResult> = pages
            .into_iter()
            .filter_map(|(page, link_paths)| {
                notes_map.get(&page.note_id).map(|note| PageResult {
                    note_id: note.id.clone(),
                    title: note.title.clone(),
                    content_snippet: note.content.chars().take(200).collect(),
                    relevance_score: page.quality_score.unwrap_or(0.5),
                    link_paths,
                })
            })
            .collect();

        Ok(results)
    }

    pub async fn get_page_context(&self, note_id: &str, depth: usize) -> Result<String, String> {
        let mut context = String::new();
        let mut visited = std::collections::HashSet::new();
        self.collect_backlinks_boxed(note_id, depth, &mut visited, &mut context).await?;
        Ok(context)
    }

    async fn collect_backlinks_boxed(
        &self,
        note_id: &str,
        depth: usize,
        visited: &mut std::collections::HashSet<String>,
        context: &mut String,
    ) -> Result<(), String> {
        self.collect_backlinks_impl(note_id, depth, visited, context).await
    }

    async fn collect_backlinks_impl(
        &self,
        note_id: &str,
        depth: usize,
        visited: &mut std::collections::HashSet<String>,
        context: &mut String,
    ) -> Result<(), String> {
        if depth == 0 || visited.contains(note_id) {
            return Ok(());
        }
        visited.insert(note_id.to_string());

        let note = notes::Entity::find_by_id(note_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Note {} not found", note_id))?;

        context.push_str(&format!("\n# {}\n{}\n", note.title, note.content));

        let backlinks = axagent_core::entity::note_backlinks::Entity::find()
            .filter(axagent_core::entity::note_backlinks::Column::TargetNoteId.eq(note_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;

        for backlink in backlinks {
            let next_note_id = backlink.source_note_id.clone();
            Box::pin(self.collect_backlinks_impl(&next_note_id, depth - 1, visited, context)).await?;
        }

        Ok(())
    }
}