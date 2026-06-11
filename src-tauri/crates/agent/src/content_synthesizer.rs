// SPDX-License-Identifier: AGPL-3.0-only

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

        if self.include_citations && !relevant_sources.is_empty() {
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

        let source_materials: Vec<String> = sources
            .iter()
            .enumerate()
            .take(5)
            .map(|(i, s)| {
                format!(
                    "[Source {}] Title: {}\nURL: {}\nContent: {}",
                    i + 1,
                    s.title,
                    s.url,
                    if s.snippet.len() > 500 {
                        format!("{}...", &s.snippet[..500])
                    } else {
                        s.snippet.clone()
                    }
                )
            })
            .collect();

        let key_findings = self.extract_key_findings(sources);
        let findings_section = if key_findings.is_empty() {
            String::new()
        } else {
            format!(
                "\n\n**Key Findings:**\n{}",
                key_findings
                    .iter()
                    .enumerate()
                    .map(|(i, f)| format!("{}. {}", i + 1, f))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };

        let synthesis = format!(
            "Based on the analysis of {} sources:\n\n{}\n\n{}{}",
            sources.len(),
            source_materials.join("\n\n"),
            self.generate_source_stats(sources),
            findings_section
        );

        synthesis
    }

    pub fn build_synthesis_prompt(&self, sources: &[SearchResult], section_title: &str) -> String {
        if sources.is_empty() {
            return String::new();
        }

        let source_materials: Vec<String> = sources
            .iter()
            .enumerate()
            .take(5)
            .map(|(i, s)| {
                format!(
                    "[Source {}] Title: {}\nURL: {}\nContent: {}",
                    i + 1,
                    s.title,
                    s.url,
                    if s.snippet.len() > 800 {
                        format!("{}...", &s.snippet[..800])
                    } else {
                        s.snippet.clone()
                    }
                )
            })
            .collect();

        format!(
            "You are a research synthesis expert. Based on the following sources, write a comprehensive and well-structured section about: {}\n\n\
             Requirements:\n\
             - Synthesize information from multiple sources, don't just list them\n\
             - Identify key findings, agreements, and contradictions between sources\n\
             - Use clear, professional language\n\
             - Include specific data points and facts from the sources\n\
             - If sources disagree, present multiple viewpoints\n\
             - Do not fabricate information not present in the sources\n\n\
             Sources:\n{}\n\n\
             Write the synthesized content for the section '{}':",
            section_title,
            source_materials.join("\n\n"),
            section_title
        )
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
                html.push_str(&format!("<p><em>{}</em></p>\n", &trimmed[1..trimmed.len() - 1]));
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
        assert_eq!(ContentFormatter::to_anchor("Hello World"), "hello-world");
        assert_eq!(ContentFormatter::to_anchor("Test 123"), "test-123");
    }

    #[tokio::test]
    async fn test_synthesize_section_with_description() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let section = OutlineSection::new("Introduction".to_string())
            .with_description("Overview of the research".to_string());

        let content = synthesizer.synthesize_section(&section, &[]).await;
        assert!(content.contains("## Introduction"));
        assert!(content.contains("Overview of the research"));
    }

    #[tokio::test]
    async fn test_synthesize_section_without_description() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let section = OutlineSection::new("Introduction".to_string());

        let content = synthesizer.synthesize_section(&section, &[]).await;
        assert!(content.contains("## Introduction"));
        assert!(!content.contains('*'));
    }

    #[tokio::test]
    async fn test_synthesize_section_with_sources() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let section = OutlineSection::new("Machine Learning Research".to_string())
            .with_description("Overview".to_string());

        let sources = vec![SearchResult::new(
            SourceType::Academic,
            "https://arxiv.org/abs/2103.00001".to_string(),
            "Machine Learning Advances".to_string(),
            "This paper presents significant advances in machine learning algorithms and their applications to real-world problems.".to_string(),
        )];

        let content = synthesizer.synthesize_section(&section, &sources).await;
        assert!(content.contains("## Machine Learning Research"));
        assert!(content.contains("Sources"));
    }

    #[tokio::test]
    async fn test_synthesize_section_without_citations() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker).with_citations(false);

        let section = OutlineSection::new("Introduction".to_string());
        let content = synthesizer.synthesize_section(&section, &[]).await;
        assert!(!content.contains("Sources"));
    }

    #[tokio::test]
    async fn test_synthesize_batch() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let sections = vec![
            OutlineSection::new("Introduction".to_string()),
            OutlineSection::new("Background".to_string()),
        ];

        let contents = synthesizer.synthesize_batch(&sections, &[]).await;
        assert_eq!(contents.len(), 2);
        assert!(contents[0].contains("## Introduction"));
        assert!(contents[1].contains("## Background"));
    }

    #[tokio::test]
    async fn test_generate_summary_empty() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let summary = synthesizer.generate_summary(&[]).await;
        assert!(summary.is_empty());
    }

    #[tokio::test]
    async fn test_generate_summary_with_content() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let sections_content = vec![
            "## Introduction\n\n- This is a very important finding about the research topic that spans multiple lines\n- Another significant discovery that provides insight into the problem domain".to_string(),
        ];

        let summary = synthesizer.generate_summary(&sections_content).await;
        assert!(summary.contains("# Summary"));
    }

    #[test]
    fn test_extract_keywords() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let keywords = synthesizer.extract_keywords("Machine Learning and Deep Neural Networks");
        assert!(keywords.contains(&"machine".to_string()));
        assert!(keywords.contains(&"learning".to_string()));
        assert!(keywords.contains(&"deep".to_string()));
        assert!(keywords.contains(&"neural".to_string()));
        assert!(keywords.contains(&"networks".to_string()));
        assert!(!keywords.contains(&"and".to_string()));
    }

    #[test]
    fn test_extract_keywords_filters_stop_words() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let keywords =
            synthesizer.extract_keywords("the a an and or but in on at to for of with by from");
        assert!(keywords.is_empty());
    }

    #[test]
    fn test_extract_keywords_filters_short_words() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let keywords = synthesizer.extract_keywords("AI is the best");
        assert!(!keywords.contains(&"AI".to_string()));
        assert!(!keywords.contains(&"is".to_string()));
        assert!(keywords.contains(&"best".to_string()));
    }

    #[test]
    fn test_calculate_relevance_with_matching_keywords() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let keywords = vec!["machine".to_string(), "learning".to_string()];
        let source = SearchResult::new(
            SourceType::Academic,
            "https://example.com".to_string(),
            "Machine Learning Research".to_string(),
            "About machine learning".to_string(),
        );

        let score = synthesizer.calculate_relevance(&keywords, &source);
        assert!(score > 0.2);
    }

    #[test]
    fn test_calculate_relevance_with_no_matching_keywords() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let keywords = vec!["quantum".to_string(), "physics".to_string()];
        let source = SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Cooking Recipes".to_string(),
            "About cooking".to_string(),
        );

        let score = synthesizer.calculate_relevance(&keywords, &source);
        assert!(score < 0.3);
    }

    #[test]
    fn test_calculate_relevance_source_type_boost() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let keywords = vec!["test".to_string()];
        let academic = SearchResult::new(
            SourceType::Academic,
            "https://example.com".to_string(),
            "test".to_string(),
            "test".to_string(),
        );
        let web = SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "test".to_string(),
            "test".to_string(),
        );

        let academic_score = synthesizer.calculate_relevance(&keywords, &academic);
        let web_score = synthesizer.calculate_relevance(&keywords, &web);
        assert!(academic_score > web_score);
    }

    #[test]
    fn test_generate_default_content_introduction() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let content = synthesizer.generate_default_content("Introduction");
        assert!(content.contains("overview"));
    }

    #[test]
    fn test_generate_default_content_background() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let content = synthesizer.generate_default_content("Background");
        assert!(content.contains("contextual"));
    }

    #[test]
    fn test_generate_default_content_method() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let content = synthesizer.generate_default_content("Methodology");
        assert!(content.contains("methodology"));
    }

    #[test]
    fn test_generate_default_content_findings() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let content = synthesizer.generate_default_content("Findings");
        assert!(content.contains("findings"));
    }

    #[test]
    fn test_generate_default_content_results() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let content = synthesizer.generate_default_content("Results");
        assert!(content.contains("findings"));
    }

    #[test]
    fn test_generate_default_content_discussion() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let content = synthesizer.generate_default_content("Discussion");
        assert!(content.contains("analysis"));
    }

    #[test]
    fn test_generate_default_content_conclusion() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let content = synthesizer.generate_default_content("Conclusion");
        assert!(content.contains("future work"));
    }

    #[test]
    fn test_generate_default_content_generic() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let content = synthesizer.generate_default_content("Custom Section");
        assert!(content.contains("Custom Section"));
    }

    #[test]
    fn test_extract_key_findings() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let sources = vec![SearchResult::new(
            SourceType::Academic,
            "https://example.com".to_string(),
            "Research".to_string(),
            "This is a long snippet that contains more than fifty characters and provides detailed information about the findings.".to_string(),
        )];

        let findings = synthesizer.extract_key_findings(&sources);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_extract_key_findings_short_snippet() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let sources = vec![SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Short".to_string(),
            "Short".to_string(),
        )];

        let findings = synthesizer.extract_key_findings(&sources);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_extract_key_findings_truncation() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let long_snippet = "A".repeat(300);
        let sources = vec![SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Long".to_string(),
            long_snippet,
        )];

        let findings = synthesizer.extract_key_findings(&sources);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].ends_with("..."));
    }

    #[test]
    fn test_generate_source_stats() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let sources = vec![
            SearchResult::new(
                SourceType::Academic,
                "url1".to_string(),
                "A".to_string(),
                "s".to_string(),
            ),
            SearchResult::new(
                SourceType::Web,
                "url2".to_string(),
                "B".to_string(),
                "s".to_string(),
            ),
            SearchResult::new(
                SourceType::Documentation,
                "url3".to_string(),
                "C".to_string(),
                "s".to_string(),
            ),
        ];

        let stats = synthesizer.generate_source_stats(&sources);
        assert!(stats.contains("3 sources"));
        assert!(stats.contains("1 academic"));
        assert!(stats.contains("1 web"));
        assert!(stats.contains("1 documentation"));
    }

    #[test]
    fn test_generate_source_stats_empty() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let stats = synthesizer.generate_source_stats(&[]);
        assert!(stats.contains("0 sources"));
    }

    #[test]
    fn test_content_formatter_to_markdown() {
        let content = "# Title\n\nSome text";
        let result = ContentFormatter::to_markdown(content);
        assert_eq!(result, content);
    }

    #[test]
    fn test_content_formatter_to_html_h2() {
        let md = "## Section Title";
        let html = ContentFormatter::to_html(md);
        assert!(html.contains("<h2>Section Title</h2>"));
    }

    #[test]
    fn test_content_formatter_to_html_h3() {
        let md = "### Subsection Title";
        let html = ContentFormatter::to_html(md);
        assert!(html.contains("<h3>Subsection Title</h3>"));
    }

    #[test]
    fn test_content_formatter_to_html_emphasis() {
        let md = "*emphasized text*";
        let html = ContentFormatter::to_html(md);
        assert!(html.contains("<em>emphasized text</em>"));
    }

    #[test]
    fn test_content_formatter_to_html_paragraph() {
        let md = "Just a paragraph";
        let html = ContentFormatter::to_html(md);
        assert!(html.contains("<p>Just a paragraph</p>"));
    }

    #[test]
    fn test_content_formatter_to_html_empty_lines() {
        let md = "# Title\n\n\n## Section";
        let html = ContentFormatter::to_html(md);
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<h2>Section</h2>"));
    }

    #[test]
    fn test_content_formatter_to_plain_text() {
        let md = "# Title\n\n## Section\n\n- Item 1\n\n*emphasis*\n\nNormal text";
        let plain = ContentFormatter::to_plain_text(md);
        assert!(plain.contains("TITLE"));
        assert!(plain.contains("Section"));
        assert!(plain.contains("• Item 1"));
        assert!(plain.contains("emphasis"));
        assert!(plain.contains("Normal text"));
    }

    #[test]
    fn test_content_formatter_to_plain_text_h3() {
        let md = "### Subsection";
        let plain = ContentFormatter::to_plain_text(md);
        assert!(plain.contains("Subsection"));
    }

    #[test]
    fn test_content_formatter_add_table_of_contents() {
        let sections = vec![
            OutlineSection::new("Introduction".to_string()),
            OutlineSection::new("Methods".to_string()),
        ];
        let content = "## Introduction\n\nSome text\n\n## Methods\n\nMore text";
        let result = ContentFormatter::add_table_of_contents(content, &sections);
        assert!(result.contains("# Table of Contents"));
        assert!(result.contains("[Introduction]"));
        assert!(result.contains("[Methods]"));
        assert!(result.contains("---"));
    }

    #[test]
    fn test_content_formatter_to_anchor_special_chars() {
        let anchor = ContentFormatter::to_anchor("Hello, World! Test #1");
        assert!(anchor.contains("hello"));
        assert!(anchor.contains("world"));
        assert!(anchor.contains("test"));
    }

    #[test]
    fn test_content_formatter_to_anchor_empty() {
        let anchor = ContentFormatter::to_anchor("");
        assert!(anchor.is_empty());
    }

    #[tokio::test]
    async fn test_with_min_credibility() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker).with_min_credibility(0.8);
        assert!((synthesizer.min_credibility - 0.8).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_with_citations() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker).with_citations(false);
        assert!(!synthesizer.include_citations);
    }

    #[tokio::test]
    async fn test_filter_relevant_sources() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let sources = vec![
            SearchResult::new(
                SourceType::Academic,
                "url1".to_string(),
                "Machine Learning Research".to_string(),
                "About machine learning algorithms".to_string(),
            ),
            SearchResult::new(
                SourceType::Web,
                "url2".to_string(),
                "Cooking Tips".to_string(),
                "How to cook pasta".to_string(),
            ),
        ];

        let filtered = synthesizer.filter_relevant_sources(&sources, "Machine Learning");
        assert!(filtered.len() <= 5);
        if !filtered.is_empty() {
            assert!(filtered[0].title.contains("Machine Learning"));
        }
    }

    #[tokio::test]
    async fn test_filter_relevant_sources_limit() {
        let tracker = Arc::new(CitationTracker::new());
        let synthesizer = ContentSynthesizer::new(tracker);

        let sources: Vec<SearchResult> = (0..10)
            .map(|i| {
                SearchResult::new(
                    SourceType::Web,
                    format!("url{}", i),
                    format!("Research Topic {}", i),
                    format!("About research topic {}", i),
                )
            })
            .collect();

        let filtered = synthesizer.filter_relevant_sources(&sources, "Research Topic");
        assert!(filtered.len() <= 5);
    }
}
