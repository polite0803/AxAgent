use crate::citation_tracker::CitationTracker;
use crate::credibility_evaluator::CredibilityEvaluator;
use crate::research_state::{OutlineSection, SearchResult, SourceType};
use std::sync::Arc;

pub struct ContentSynthesizer {
    citation_tracker: Arc<CitationTracker>,
    credibility_evaluator: CredibilityEvaluator,
    min_credibility: f32,
    include_citations: bool,
}

impl ContentSynthesizer {
    pub fn new(citation_tracker: Arc<CitationTracker>) -> Self {
        Self {
            citation_tracker,
            credibility_evaluator: CredibilityEvaluator::new(),
            min_credibility: 0.3,
            include_citations: true,
        }
    }

    pub fn with_min_credibility(mut self, credibility: f32) -> Self {
        self.min_credibility = credibility;
        self
    }

    pub fn with_citations(mut self, include: bool) -> Self {
        self.include_citations = include;
        self
    }

    pub async fn synthesize_section(
        &self,
        section: &OutlineSection,
        sources: &[SearchResult],
    ) -> String {
        let mut content = format!("## {}\n\n", section.title);

        if !section.description.is_empty() {
            content.push_str(&format!("*{}*\n\n", section.description));
        }

        let relevant_sources = self.filter_relevant_sources(sources, &section.title);

        let mut trusted_sources = Vec::new();
        for source in relevant_sources.iter() {
            let assessment = self.credibility_evaluator.evaluate(source).await;
            if assessment.credibility.overall >= self.min_credibility {
                trusted_sources.push(source.clone());
            }
        }

        if trusted_sources.is_empty() {
            content.push_str(&self.generate_default_content(&section.title));
        } else {
            content.push_str(
                &self
                    .synthesize_from_sources(&trusted_sources, &section.title)
                    .await,
            );
            for source in &trusted_sources {
                let citation = crate::research_state::Citation::new(
                    source.url.clone(),
                    source.title.clone(),
                    source.source_type,
                );
                let _ = self.citation_tracker.add_citation(citation).await;
            }
        }

        if self.include_citations {
            content.push_str("\n\n**Sources:**\n");
            for source in relevant_sources.iter().take(3) {
                content.push_str(&format!("- [{}]({})\n", source.title, source.url));
            }
        }

        content
    }

    fn filter_relevant_sources(
        &self,
        sources: &[SearchResult],
        section_title: &str,
    ) -> Vec<SearchResult> {
        let section_keywords = self.extract_keywords(section_title);

        let mut scored: Vec<(SearchResult, f32)> = sources
            .iter()
            .filter_map(|s| {
                let score = self.calculate_relevance(&section_keywords, s);
                if score > 0.2 {
                    Some((s.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored.into_iter().map(|(s, _)| s).take(5).collect()
    }

    fn extract_keywords(&self, text: &str) -> Vec<String> {
        let stop_words = vec![
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "by", "from", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
            "do", "does", "did", "will", "would", "could", "should", "may", "might", "must",
            "shall", "can", "this", "that", "these", "those", "it", "its",
        ];

        text.split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 3 && !stop_words.contains(&w.as_str()))
            .collect()
    }

    fn calculate_relevance(&self, keywords: &[String], source: &SearchResult) -> f32 {
        let source_text = format!("{} {} {}", source.title, source.snippet, source.url);
        let source_lower = source_text.to_lowercase();

        let keyword_matches = keywords
            .iter()
            .filter(|k| source_lower.contains(&k.to_lowercase()))
            .count();

        let base_score = keyword_matches as f32 / keywords.len().max(1) as f32;

        let source_type_boost = match source.source_type {
            SourceType::Academic => 0.2,
            SourceType::Documentation => 0.15,
            SourceType::News => 0.1,
            _ => 0.0,
        };

        base_score + source_type_boost
    }

    async fn synthesize_from_sources(
        &self,
        sources: &[SearchResult],
        section_title: &str,
    ) -> String {
        if sources.is_empty() {
            return self.generate_default_content(section_title);
        }

        let mut synthesis = String::new();

        synthesis.push_str("Based on the analysis of available sources:\n\n");

        let key_findings = self.extract_key_findings(sources);
        for finding in key_findings {
            synthesis.push_str(&format!("- {}\n", finding));
        }

        synthesis.push_str("\n\n");

        let stats = self.generate_source_stats(sources);
        synthesis.push_str(&format!("*{}*\n", stats));

        synthesis
    }

    fn extract_key_findings(&self, sources: &[SearchResult]) -> Vec<String> {
        let mut findings = Vec::new();

        for source in sources.iter().take(5) {
            let snippet = &source.snippet;
            if snippet.len() > 50 {
                let truncated = if snippet.len() > 200 {
                    format!("{}...", &snippet[..200])
                } else {
                    snippet.clone()
                };
                findings.push(truncated);
            }
        }

        findings
    }

    fn generate_source_stats(&self, sources: &[SearchResult]) -> String {
        let total = sources.len();
        let academic = sources
            .iter()
            .filter(|s| s.source_type == SourceType::Academic)
            .count();
        let web = sources
            .iter()
            .filter(|s| s.source_type == SourceType::Web)
            .count();
        let docs = sources
            .iter()
            .filter(|s| s.source_type == SourceType::Documentation)
            .count();

        format!(
            "Analysis based on {} sources: {} academic papers, {} web sources, {} documentation entries.",
            total, academic, web, docs
        )
    }

    fn generate_default_content(&self, section_title: &str) -> String {
        let lower = section_title.to_lowercase();

        if lower.contains("introduction") {
            "This section provides an overview of the research topic and outlines the main objectives of this investigation.".to_string()
        } else if lower.contains("background") {
            "This section covers the contextual information and prior work relevant to understanding the current research.".to_string()
        } else if lower.contains("method") {
            "This section describes the approach and methodology used in this research.".to_string()
        } else if lower.contains("finding") || lower.contains("result") {
            "This section presents the key findings and observations from the research.".to_string()
        } else if lower.contains("discussion") {
            "This section provides an analysis and interpretation of the results.".to_string()
        } else if lower.contains("conclusion") {
            "This section summarizes the main contributions and suggests directions for future work.".to_string()
        } else {
            format!(
                "Content for {} will be developed based on the research findings.",
                section_title
            )
        }
    }

    pub async fn synthesize_batch(
        &self,
        sections: &[OutlineSection],
        sources: &[SearchResult],
    ) -> Vec<String> {
        let mut contents = Vec::new();

        for section in sections {
            let content = self.synthesize_section(section, sources).await;
            contents.push(content);
        }

        contents
    }

    pub async fn generate_summary(&self, sections_content: &[String]) -> String {
        if sections_content.is_empty() {
            return String::new();
        }

        let mut summary = String::from("# Summary\n\n");

        let key_points = self.extract_key_points(sections_content);
        for (i, point) in key_points.iter().enumerate() {
            summary.push_str(&format!("{}. {}\n", i + 1, point));
        }

        summary
    }

    fn extract_key_points(&self, sections_content: &[String]) -> Vec<String> {
        let mut points = Vec::new();

        for content in sections_content {
            let lines: Vec<&str> = content.lines().collect();

            for line in lines {
                let trimmed = line.trim();
                if trimmed.starts_with("- ") && trimmed.len() > 10 {
                    let point = trimmed[2..].trim().to_string();
                    if !point.is_empty() && point.len() > 20 {
                        points.push(point);
                    }
                }
            }
        }

        points.truncate(5);
        points
    }
}

pub struct ContentFormatter;

impl ContentFormatter {
    pub fn to_markdown(content: &str) -> String {
        content.to_string()
    }

    pub fn to_html(content: &str) -> String {
        let mut html = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(stripped) = trimmed.strip_prefix("# ") {
                html.push_str(&format!("<h1>{}</h1>\n", stripped));
            } else if let Some(stripped) = trimmed.strip_prefix("## ") {
                html.push_str(&format!("<h2>{}</h2>\n", stripped));
            } else if let Some(stripped) = trimmed.strip_prefix("### ") {
                html.push_str(&format!("<h3>{}</h3>\n", stripped));
            } else if let Some(stripped) = trimmed.strip_prefix("- ") {
                html.push_str(&format!("<li>{}</li>\n", stripped));
            } else if trimmed.starts_with('*') && trimmed.ends_with('*') {
                html.push_str(&format!(
                    "<p><em>{}</em></p>\n",
                    &trimmed[1..trimmed.len() - 1]
                ));
            } else if !trimmed.is_empty() {
                html.push_str(&format!("<p>{}</p>\n", trimmed));
            }
        }

        html
    }

    pub fn to_plain_text(content: &str) -> String {
        content
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                if let Some(stripped) = trimmed.strip_prefix("# ") {
                    format!("\n{}\n", stripped.to_uppercase())
                } else if let Some(stripped) = trimmed.strip_prefix("## ") {
                    format!("\n{}\n", stripped)
                } else if let Some(stripped) = trimmed.strip_prefix("- ") {
                    format!("  • {}\n", stripped)
                } else if trimmed.starts_with('*') && trimmed.ends_with('*') {
                    format!("{}\n", &trimmed[1..trimmed.len() - 1])
                } else {
                    format!("{}\n", trimmed)
                }
            })
            .collect()
    }

    pub fn add_table_of_contents(content: &str, sections: &[OutlineSection]) -> String {
        let mut result = String::from("# Table of Contents\n\n");

        for (i, section) in sections.iter().enumerate() {
            result.push_str(&format!(
                "{}. [{}](#{})\n",
                i + 1,
                section.title,
                Self::to_anchor(&section.title)
            ));
        }

        result.push_str("\n---\n\n");
        result.push_str(content);

        result
    }

    fn to_anchor(text: &str) -> String {
        text.to_lowercase()
            .split_whitespace()
            .map(|s| s.replace(|c: char| !c.is_alphanumeric(), ""))
            .collect::<Vec<_>>()
            .join("-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_synthesize_section() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let section = OutlineSection::new("Introduction".to_string())
            .with_description("Overview of the research".to_string());

        let sources = vec![];

        let content = synthesizer.synthesize_section(&section, &sources).await;
        assert!(!content.is_empty());
        assert!(content.contains("Introduction"));
    }

    #[test]
    fn test_content_to_html() {
        let md = "# Title\n\nSome content\n\n- Item 1\n- Item 2";
        let html = ContentFormatter::to_html(md);

        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<li>Item 1</li>"));
    }

    #[test]
    fn test_to_anchor() {
        assert_eq!(ContentFormatter::to_anchor("Hello World"), "helloworld");
        assert_eq!(ContentFormatter::to_anchor("Test 123"), "test123");
    }
}
