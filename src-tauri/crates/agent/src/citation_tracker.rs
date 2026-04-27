use crate::research_state::{Citation, SourceType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationUsage {
    pub citation_id: String,
    pub used_in_section: String,
    pub used_at: DateTime<Utc>,
    pub quote_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationStats {
    pub total_citations: usize,
    pub citations_by_source_type: HashMap<String, usize>,
    pub most_used_citations: Vec<CitationUsageCount>,
    pub average_credibility: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationUsageCount {
    pub citation_id: String,
    pub source_title: String,
    pub usage_count: usize,
}

#[derive(Debug, Clone)]
pub struct CitationTracker {
    citations: Arc<RwLock<HashMap<String, Citation>>>,
    usage_records: Arc<RwLock<HashMap<String, Vec<CitationUsage>>>>,
}

impl CitationTracker {
    pub fn new() -> Self {
        Self {
            citations: Arc::new(RwLock::new(HashMap::new())),
            usage_records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_citation(&self, citation: Citation) -> String {
        let id = citation.id.clone();
        let mut citations = self.citations.write().await;
        citations.insert(id.clone(), citation);
        id
    }

    pub async fn add_citation_with_usage(
        &self,
        citation: Citation,
        usage: CitationUsage,
    ) -> String {
        let id = self.add_citation(citation).await;
        self.record_usage(id.clone(), usage).await;
        id
    }

    pub async fn get_citation(&self, id: &str) -> Option<Citation> {
        let citations = self.citations.read().await;
        citations.get(id).cloned()
    }

    pub async fn get_all_citations(&self) -> Vec<Citation> {
        let citations = self.citations.read().await;
        citations.values().cloned().collect()
    }

    pub async fn get_citations_by_source_type(&self, source_type: SourceType) -> Vec<Citation> {
        let citations = self.citations.read().await;
        citations
            .values()
            .filter(|c| c.source_type == source_type)
            .cloned()
            .collect()
    }

    pub async fn update_citation(&self, citation: Citation) -> Option<Citation> {
        let mut citations = self.citations.write().await;
        citations.insert(citation.id.clone(), citation.clone());
        Some(citation)
    }

    pub async fn remove_citation(&self, id: &str) -> Option<Citation> {
        let mut citations = self.citations.write().await;
        let removed = citations.remove(id);

        if removed.is_some() {
            let mut usage = self.usage_records.write().await;
            usage.remove(id);
        }

        removed
    }

    pub async fn record_usage(&self, citation_id: String, usage: CitationUsage) {
        let mut usage_records = self.usage_records.write().await;
        usage_records
            .entry(citation_id)
            .or_insert_with(Vec::new)
            .push(usage);
    }

    pub async fn get_usage(&self, citation_id: &str) -> Vec<CitationUsage> {
        let usage_records = self.usage_records.read().await;
        usage_records.get(citation_id).cloned().unwrap_or_default()
    }

    pub async fn get_all_usage(&self) -> HashMap<String, Vec<CitationUsage>> {
        let usage_records = self.usage_records.read().await;
        usage_records.clone()
    }

    pub async fn get_usage_count(&self, citation_id: &str) -> usize {
        let usage_records = self.usage_records.read().await;
        usage_records.get(citation_id).map(|v| v.len()).unwrap_or(0)
    }

    pub async fn get_stats(&self) -> CitationStats {
        let citations = self.citations.read().await;
        let usage_records = self.usage_records.read().await;

        let mut citations_by_source_type: HashMap<String, usize> = HashMap::new();
        let mut total_credibility = 0.0_f32;
        let mut citation_credibility_count = 0;

        for citation in citations.values() {
            let type_key = format!("{:?}", citation.source_type).to_lowercase();
            *citations_by_source_type.entry(type_key).or_insert(0) += 1;

            if citation.credibility > 0.0 {
                total_credibility += citation.credibility;
                citation_credibility_count += 1;
            }
        }

        let mut most_used: Vec<CitationUsageCount> = usage_records
            .iter()
            .map(|(id, usages)| {
                let source_title = citations
                    .get(id)
                    .map(|c| c.source_title.clone())
                    .unwrap_or_default();
                CitationUsageCount {
                    citation_id: id.clone(),
                    source_title,
                    usage_count: usages.len(),
                }
            })
            .collect();

        most_used.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        most_used.truncate(10);

        let average_credibility = if citation_credibility_count > 0 {
            total_credibility / citation_credibility_count as f32
        } else {
            0.0
        };

        CitationStats {
            total_citations: citations.len(),
            citations_by_source_type,
            most_used_citations: most_used,
            average_credibility,
        }
    }

    pub async fn find_citation_by_url(&self, url: &str) -> Option<Citation> {
        let citations = self.citations.read().await;
        citations.values().find(|c| c.source_url == url).cloned()
    }

    pub async fn mark_in_report(&self, citation_id: &str, in_report: bool) -> Option<Citation> {
        let mut citations = self.citations.write().await;
        if let Some(citation) = citations.get_mut(citation_id) {
            citation.in_report = in_report;
            return Some(citation.clone());
        }
        None
    }

    pub async fn get_unused_citations(&self) -> Vec<Citation> {
        let citations = self.citations.read().await;
        let usage_records = self.usage_records.read().await;

        citations
            .values()
            .filter(|c| {
                !c.in_report
                    && usage_records
                        .get(&c.id)
                        .map(|u| u.is_empty())
                        .unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    pub async fn get_report_citations(&self) -> Vec<Citation> {
        let citations = self.citations.read().await;
        citations
            .values()
            .filter(|c| c.in_report)
            .cloned()
            .collect()
    }

    pub async fn clear(&self) {
        let mut citations = self.citations.write().await;
        let mut usage_records = self.usage_records.write().await;
        citations.clear();
        usage_records.clear();
    }

    pub async fn import_citations(&self, citations: Vec<Citation>) -> usize {
        let mut count = 0;
        let mut citations_map = self.citations.write().await;

        for citation in citations {
            if !citations_map.contains_key(&citation.id) {
                citations_map.insert(citation.id.clone(), citation);
                count += 1;
            }
        }

        count
    }
}

impl Default for CitationTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationContext {
    pub citation: Citation,
    pub usage_count: usize,
    pub last_used: Option<DateTime<Utc>>,
    pub sections: Vec<String>,
}

pub struct CitationQuerier;

impl CitationQuerier {
    pub async fn query_by_min_credibility(
        tracker: &CitationTracker,
        min_credibility: f32,
    ) -> Vec<CitationContext> {
        let citations = tracker.get_all_citations().await;
        let usage = tracker.get_all_usage().await;

        citations
            .into_iter()
            .filter(|c| c.credibility >= min_credibility)
            .map(|c| {
                let usages = usage.get(&c.id).cloned().unwrap_or_default();
                let last_used = usages.iter().map(|u| u.used_at).max();
                let sections: Vec<String> =
                    usages.iter().map(|u| u.used_in_section.clone()).collect();

                CitationContext {
                    citation: c,
                    usage_count: usages.len(),
                    last_used,
                    sections,
                }
            })
            .collect()
    }

    pub async fn query_by_source_type(
        tracker: &CitationTracker,
        source_type: SourceType,
    ) -> Vec<CitationContext> {
        let citations = tracker.get_citations_by_source_type(source_type).await;
        let usage = tracker.get_all_usage().await;

        citations
            .into_iter()
            .map(|c| {
                let usages = usage.get(&c.id).cloned().unwrap_or_default();
                let last_used = usages.iter().map(|u| u.used_at).max();
                let sections: Vec<String> =
                    usages.iter().map(|u| u.used_in_section.clone()).collect();

                CitationContext {
                    citation: c,
                    usage_count: usages.len(),
                    last_used,
                    sections,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_citation() {
        let tracker = CitationTracker::new();

        let citation = Citation::new(
            "https://example.com".to_string(),
            "Example".to_string(),
            SourceType::Web,
        );

        let id = tracker.add_citation(citation.clone()).await;
        let retrieved = tracker.get_citation(&id).await;

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().source_url, "https://example.com");
    }

    #[tokio::test]
    async fn test_record_usage() {
        let tracker = CitationTracker::new();

        let citation = Citation::new(
            "https://example.com".to_string(),
            "Example".to_string(),
            SourceType::Web,
        );

        let id = tracker.add_citation(citation).await;

        let usage = CitationUsage {
            citation_id: id.clone(),
            used_in_section: "Introduction".to_string(),
            used_at: Utc::now(),
            quote_context: Some("Test quote".to_string()),
        };

        tracker.record_usage(id.clone(), usage).await;

        let retrieved_usage = tracker.get_usage(&id).await;
        assert_eq!(retrieved_usage.len(), 1);
        assert_eq!(retrieved_usage[0].used_in_section, "Introduction");
    }

    #[tokio::test]
    async fn test_stats() {
        let tracker = CitationTracker::new();

        let citation1 = Citation::new(
            "https://example1.com".to_string(),
            "Example 1".to_string(),
            SourceType::Web,
        );
        let citation2 = Citation::new(
            "https://example2.com".to_string(),
            "Example 2".to_string(),
            SourceType::Academic,
        );

        tracker.add_citation(citation1).await;
        tracker.add_citation(citation2).await;

        let stats = tracker.get_stats().await;
        assert_eq!(stats.total_citations, 2);
    }
}
