use crate::citation_tracker::CitationTracker;
use crate::research_state::Citation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceFormat {
    Markdown,
    Html,
    Json,
    BibTeX,
    APA,
    MLA,
    Chicago,
}

impl ReferenceFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReferenceFormat::Markdown => "markdown",
            ReferenceFormat::Html => "html",
            ReferenceFormat::Json => "json",
            ReferenceFormat::BibTeX => "bibtex",
            ReferenceFormat::APA => "apa",
            ReferenceFormat::MLA => "mla",
            ReferenceFormat::Chicago => "chicago",
        }
    }
}

pub struct ReferenceBuilder {
    citation_tracker: std::sync::Arc<CitationTracker>,
}

impl ReferenceBuilder {
    pub fn new(citation_tracker: std::sync::Arc<CitationTracker>) -> Self {
        Self { citation_tracker }
    }

    pub async fn build(&self, format: ReferenceFormat) -> String {
        let citations = self.citation_tracker.get_all_citations().await;

        match format {
            ReferenceFormat::Markdown => self.build_markdown(&citations),
            ReferenceFormat::Html => self.build_html(&citations),
            ReferenceFormat::Json => self.build_json(&citations),
            ReferenceFormat::BibTeX => self.build_bibtex(&citations),
            ReferenceFormat::APA => self.build_apa(&citations),
            ReferenceFormat::MLA => self.build_mla(&citations),
            ReferenceFormat::Chicago => self.build_chicago(&citations),
        }
    }

    pub async fn build_grouped(&self, format: ReferenceFormat) -> HashMap<String, String> {
        let citations = self.citation_tracker.get_all_citations().await;
        let mut grouped: HashMap<String, Vec<&Citation>> = HashMap::new();

        for citation in &citations {
            let key = format!("{:?}", citation.source_type).to_lowercase();
            grouped.entry(key).or_default().push(citation);
        }

        let mut result = HashMap::new();
        for (group, group_citations) in grouped {
            let content = match format {
                ReferenceFormat::Markdown => self.build_markdown_vec(group_citations),
                ReferenceFormat::Html => self.build_html_vec(group_citations),
                ReferenceFormat::BibTeX => self.build_bibtex_vec(group_citations),
                ReferenceFormat::APA => self.build_apa_vec(group_citations),
                ReferenceFormat::MLA => self.build_mla_vec(group_citations),
                ReferenceFormat::Chicago => self.build_chicago_vec(group_citations),
                ReferenceFormat::Json => self.build_json_vec(group_citations),
            };
            result.insert(group, content);
        }

        result
    }

    fn build_markdown(&self, citations: &[Citation]) -> String {
        self.build_markdown_vec(citations.iter().collect())
    }

    fn build_markdown_vec(&self, citations: Vec<&Citation>) -> String {
        let mut output = String::from("## References\n\n");

        for (i, citation) in citations.iter().enumerate() {
            let credibility_badge = if citation.credibility >= 0.7 {
                "[![High Credibility]](high)"
            } else if citation.credibility >= 0.4 {
                "[![Medium Credibility]](medium)"
            } else {
                "[![Low Credibility]](low)"
            };

            output.push_str(&format!(
                "{}. [{}]({}) {}\n\n",
                i + 1,
                citation.source_title,
                citation.source_url,
                credibility_badge
            ));
        }

        output
    }

    fn build_html(&self, citations: &[Citation]) -> String {
        self.build_html_vec(citations.iter().collect())
    }

    fn build_html_vec(&self, citations: Vec<&Citation>) -> String {
        let mut output = String::from("<section id=\"references\">\n<h2>References</h2>\n<ol>\n");

        for citation in citations {
            let credibility_class = if citation.credibility >= 0.7 {
                "credibility-high"
            } else if citation.credibility >= 0.4 {
                "credibility-medium"
            } else {
                "credibility-low"
            };

            output.push_str(&format!(
                r#"<li class="reference {}">
    <a href="{}">{}</a>
    <span class="credibility">Credibility: {:.0}%</span>
</li>
"#,
                credibility_class,
                citation.source_url,
                citation.source_title,
                citation.credibility * 100.0
            ));
        }

        output.push_str("</ol>\n</section>\n");
        output
    }

    fn build_json(&self, citations: &[Citation]) -> String {
        self.build_json_vec(citations.iter().collect())
    }

    fn build_json_vec(&self, citations: Vec<&Citation>) -> String {
        let refs: Vec<serde_json::Value> = citations
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "title": c.source_title,
                    "url": c.source_url,
                    "type": format!("{:?}", c.source_type).to_lowercase(),
                    "credibility": c.credibility,
                    "accessedAt": c.accessed_at.to_rfc3339()
                })
            })
            .collect();

        serde_json::to_string_pretty(&refs).unwrap_or_default()
    }

    fn build_bibtex(&self, citations: &[Citation]) -> String {
        self.build_bibtex_vec(citations.iter().collect())
    }

    fn build_bibtex_vec(&self, citations: Vec<&Citation>) -> String {
        let mut output = String::new();

        for citation in citations {
            let entry_type = match citation.source_type {
                crate::research_state::SourceType::Academic => "article",
                crate::research_state::SourceType::Documentation => "misc",
                crate::research_state::SourceType::GitHub => "misc",
                crate::research_state::SourceType::News => "article",
                crate::research_state::SourceType::Blog => "misc",
                crate::research_state::SourceType::Forum => "misc",
                crate::research_state::SourceType::Wikipedia => "misc",
                crate::research_state::SourceType::Web => "misc",
                crate::research_state::SourceType::Unknown => "misc",
            };

            let key = format!(
                "{}{}",
                citation
                    .source_title
                    .split_whitespace()
                    .next()
                    .unwrap_or("ref")
                    .to_lowercase(),
                &citation.id[..8]
            );

            output.push_str(&format!(
                "@{}{{{},\n  title = {{{}}},\n  url = {{{}}},\n  note = {{Accessed: {}}}\n}}\n\n",
                entry_type,
                key,
                citation.source_title,
                citation.source_url,
                citation.accessed_at.format("%Y-%m-%d")
            ));
        }

        output
    }

    fn build_apa(&self, citations: &[Citation]) -> String {
        self.build_apa_vec(citations.iter().collect())
    }

    fn build_apa_vec(&self, citations: Vec<&Citation>) -> String {
        let mut output = String::new();

        for (i, citation) in citations.iter().enumerate() {
            let source_type_str = match citation.source_type {
                crate::research_state::SourceType::Academic => "Journal Article",
                crate::research_state::SourceType::News => "News Article",
                crate::research_state::SourceType::Blog => "Blog Post",
                crate::research_state::SourceType::Web => "Web Page",
                crate::research_state::SourceType::Documentation => "Technical Documentation",
                crate::research_state::SourceType::GitHub => "Repository",
                _ => "Source",
            };

            output.push_str(&format!(
                "[{}] {}. ({}). {}. Retrieved from {}\n\n",
                i + 1,
                source_type_str,
                citation.accessed_at.format("%Y, %B %d"),
                citation.source_title,
                citation.source_url
            ));
        }

        output
    }

    fn build_mla(&self, citations: &[Citation]) -> String {
        self.build_mla_vec(citations.iter().collect())
    }

    fn build_mla_vec(&self, citations: Vec<&Citation>) -> String {
        let mut output = String::new();

        for (i, citation) in citations.iter().enumerate() {
            output.push_str(&format!(
                "{}. \"{}\". Web. {}. {}.\n\n",
                i + 1,
                citation.source_title,
                citation.source_url,
                citation.accessed_at.format("%d %b. %Y")
            ));
        }

        output
    }

    fn build_chicago(&self, citations: &[Citation]) -> String {
        self.build_chicago_vec(citations.iter().collect())
    }

    fn build_chicago_vec(&self, citations: Vec<&Citation>) -> String {
        let mut output = String::new();

        for (i, citation) in citations.iter().enumerate() {
            output.push_str(&format!(
                "{}. \"{}\". Accessed {}. {}.\n\n",
                i + 1,
                citation.source_title,
                citation.accessed_at.format("%B %d, %Y"),
                citation.source_url
            ));
        }

        output
    }

    pub async fn build_inline_citations(
        &self,
        citations: &[Citation],
        format: ReferenceFormat,
    ) -> String {
        match format {
            ReferenceFormat::Markdown => {
                let mut output = String::new();
                for citation in citations {
                    output.push_str(&format!("[^{}]", &citation.id[..8]));
                }
                output
            },
            ReferenceFormat::Html => {
                let mut output = String::new();
                for citation in citations {
                    output.push_str(&format!(
                        "<sup><a href=\"#ref-{}\">[{}]</a></sup>",
                        citation.id,
                        &citation.id[..8]
                    ));
                }
                output
            },
            _ => String::new(),
        }
    }

    pub async fn build_footnote_references(&self, citations: &[Citation]) -> String {
        let mut output = String::new();

        for (i, citation) in citations.iter().enumerate() {
            output.push_str(&format!(
                "{}. {}. {}. {}\n",
                i + 1,
                citation.source_title,
                citation.source_url,
                citation.accessed_at.format("%Y-%m-%d")
            ));
        }

        output
    }
}

pub struct ReferenceFormatter;

impl ReferenceFormatter {
    pub fn format_date_apa(date: chrono::DateTime<chrono::Utc>) -> String {
        date.format("%Y, %B %d").to_string()
    }

    pub fn format_date_mla(date: chrono::DateTime<chrono::Utc>) -> String {
        date.format("%d %b. %Y").to_string()
    }

    pub fn format_date_chicago(date: chrono::DateTime<chrono::Utc>) -> String {
        date.format("%B %d, %Y").to_string()
    }

    pub fn truncate_url(url: &str, max_len: usize) -> String {
        if url.len() <= max_len {
            url.to_string()
        } else {
            format!("{}...", &url[..max_len.saturating_sub(3)])
        }
    }

    pub fn sanitize_for_bibtex(input: &str) -> String {
        input.replace(['{', '}', '\\'], "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::citation_tracker::CitationTracker;
    use crate::research_state::SourceType;

    #[tokio::test]
    async fn test_build_markdown() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let citation = Citation::new(
            "https://example.com".to_string(),
            "Example Title".to_string(),
            SourceType::Web,
        );
        tracker.add_citation(citation).await;

        let builder = ReferenceBuilder::new(tracker);
        let markdown = builder.build(ReferenceFormat::Markdown).await;
        assert!(markdown.contains("## References"));
        assert!(markdown.contains("Example Title"));
    }

    #[tokio::test]
    async fn test_build_json() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let citation = Citation::new(
            "https://example.com".to_string(),
            "Example Title".to_string(),
            SourceType::Web,
        );
        tracker.add_citation(citation).await;

        let builder = ReferenceBuilder::new(tracker);
        let json = builder.build(ReferenceFormat::Json).await;
        assert!(json.contains("\"title\": \"Example Title\""));
    }

    #[test]
    fn test_sanitize_bibtex() {
        let input = "Test {something} with \\ special";
        let output = ReferenceFormatter::sanitize_for_bibtex(input);
        assert_eq!(output, "Test something with  special");
    }

    #[test]
    fn test_truncate_url() {
        let url = "https://example.com/very/long/path";
        let truncated = ReferenceFormatter::truncate_url(url, 20);
        assert!(truncated.len() <= 20);
        assert!(truncated.ends_with("..."));
    }

    #[tokio::test]
    async fn test_build_html() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let citation = Citation::new(
            "https://example.com".to_string(),
            "HTML Title".to_string(),
            SourceType::Web,
        );
        tracker.add_citation(citation).await;

        let builder = ReferenceBuilder::new(tracker);
        let html = builder.build(ReferenceFormat::Html).await;
        assert!(html.contains("<section id=\"references\">"));
        assert!(html.contains("HTML Title"));
        assert!(html.contains("https://example.com"));
        assert!(html.contains("</section>"));
    }

    #[tokio::test]
    async fn test_build_bibtex() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let citation = Citation::new(
            "https://example.com".to_string(),
            "BibTeX Title".to_string(),
            SourceType::Academic,
        );
        tracker.add_citation(citation).await;

        let builder = ReferenceBuilder::new(tracker);
        let bibtex = builder.build(ReferenceFormat::BibTeX).await;
        assert!(bibtex.contains("@article"));
        assert!(bibtex.contains("BibTeX Title"));
        assert!(bibtex.contains("https://example.com"));
    }

    #[tokio::test]
    async fn test_build_bibtex_non_academic() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let citation = Citation::new(
            "https://example.com".to_string(),
            "Blog Post".to_string(),
            SourceType::Blog,
        );
        tracker.add_citation(citation).await;

        let builder = ReferenceBuilder::new(tracker);
        let bibtex = builder.build(ReferenceFormat::BibTeX).await;
        assert!(bibtex.contains("@misc"));
        assert!(bibtex.contains("Blog Post"));
    }

    #[tokio::test]
    async fn test_build_apa() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let citation = Citation::new(
            "https://example.com".to_string(),
            "APA Title".to_string(),
            SourceType::Academic,
        );
        tracker.add_citation(citation).await;

        let builder = ReferenceBuilder::new(tracker);
        let apa = builder.build(ReferenceFormat::APA).await;
        assert!(apa.contains("Journal Article"));
        assert!(apa.contains("APA Title"));
        assert!(apa.contains("Retrieved from https://example.com"));
    }

    #[tokio::test]
    async fn test_build_apa_source_types() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let types = vec![
            (SourceType::News, "News Article"),
            (SourceType::Blog, "Blog Post"),
            (SourceType::Web, "Web Page"),
            (SourceType::Documentation, "Technical Documentation"),
            (SourceType::GitHub, "Repository"),
            (SourceType::Wikipedia, "Source"),
            (SourceType::Forum, "Source"),
            (SourceType::Unknown, "Source"),
        ];
        for (source_type, expected_label) in types {
            let citation = Citation::new(
                "https://example.com".to_string(),
                format!("Title {:?}", source_type),
                source_type,
            );
            tracker.add_citation(citation).await;
        }

        let builder = ReferenceBuilder::new(tracker);
        let apa = builder.build(ReferenceFormat::APA).await;
        for (_, expected_label) in types {
            assert!(apa.contains(expected_label), "APA should contain '{}'", expected_label);
        }
    }

    #[tokio::test]
    async fn test_build_mla() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let citation = Citation::new(
            "https://example.com".to_string(),
            "MLA Title".to_string(),
            SourceType::Web,
        );
        tracker.add_citation(citation).await;

        let builder = ReferenceBuilder::new(tracker);
        let mla = builder.build(ReferenceFormat::MLA).await;
        assert!(mla.contains("MLA Title"));
        assert!(mla.contains("https://example.com"));
        assert!(mla.contains("Web."));
    }

    #[tokio::test]
    async fn test_build_chicago() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let citation = Citation::new(
            "https://example.com".to_string(),
            "Chicago Title".to_string(),
            SourceType::Web,
        );
        tracker.add_citation(citation).await;

        let builder = ReferenceBuilder::new(tracker);
        let chicago = builder.build(ReferenceFormat::Chicago).await;
        assert!(chicago.contains("Chicago Title"));
        assert!(chicago.contains("Accessed"));
        assert!(chicago.contains("https://example.com"));
    }

    #[tokio::test]
    async fn test_build_empty_citations() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let builder = ReferenceBuilder::new(tracker);

        let markdown = builder.build(ReferenceFormat::Markdown).await;
        assert!(markdown.contains("## References"));

        let html = builder.build(ReferenceFormat::Html).await;
        assert!(html.contains("<section id=\"references\">"));

        let json = builder.build(ReferenceFormat::Json).await;
        assert_eq!(json, "[]");

        let bibtex = builder.build(ReferenceFormat::BibTeX).await;
        assert!(bibtex.is_empty());

        let apa = builder.build(ReferenceFormat::APA).await;
        assert!(apa.is_empty());

        let mla = builder.build(ReferenceFormat::MLA).await;
        assert!(mla.is_empty());

        let chicago = builder.build(ReferenceFormat::Chicago).await;
        assert!(chicago.is_empty());
    }

    #[tokio::test]
    async fn test_build_grouped() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        tracker
            .add_citation(Citation::new(
                "https://example.com".to_string(),
                "Web Title".to_string(),
                SourceType::Web,
            ))
            .await;
        tracker
            .add_citation(Citation::new(
                "https://paper.com".to_string(),
                "Academic Title".to_string(),
                SourceType::Academic,
            ))
            .await;

        let builder = ReferenceBuilder::new(tracker);
        let grouped = builder.build_grouped(ReferenceFormat::Markdown).await;
        assert!(grouped.contains_key("web"));
        assert!(grouped.contains_key("academic"));
        assert!(grouped["web"].contains("Web Title"));
        assert!(grouped["academic"].contains("Academic Title"));
    }

    #[tokio::test]
    async fn test_build_grouped_html() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        tracker
            .add_citation(Citation::new(
                "https://example.com".to_string(),
                "Title".to_string(),
                SourceType::Web,
            ))
            .await;

        let builder = ReferenceBuilder::new(tracker);
        let grouped = builder.build_grouped(ReferenceFormat::Html).await;
        assert!(grouped.contains_key("web"));
        assert!(grouped["web"].contains("<section"));
    }

    #[tokio::test]
    async fn test_build_grouped_bibtex() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        tracker
            .add_citation(Citation::new(
                "https://example.com".to_string(),
                "Title".to_string(),
                SourceType::Academic,
            ))
            .await;

        let builder = ReferenceBuilder::new(tracker);
        let grouped = builder.build_grouped(ReferenceFormat::BibTeX).await;
        assert!(grouped.contains_key("academic"));
        assert!(grouped["academic"].contains("@article"));
    }

    #[tokio::test]
    async fn test_build_grouped_apa() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        tracker
            .add_citation(Citation::new(
                "https://example.com".to_string(),
                "Title".to_string(),
                SourceType::News,
            ))
            .await;

        let builder = ReferenceBuilder::new(tracker);
        let grouped = builder.build_grouped(ReferenceFormat::APA).await;
        assert!(grouped.contains_key("news"));
        assert!(grouped["news"].contains("News Article"));
    }

    #[tokio::test]
    async fn test_build_grouped_mla() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        tracker
            .add_citation(Citation::new(
                "https://example.com".to_string(),
                "Title".to_string(),
                SourceType::Web,
            ))
            .await;

        let builder = ReferenceBuilder::new(tracker);
        let grouped = builder.build_grouped(ReferenceFormat::MLA).await;
        assert!(grouped.contains_key("web"));
        assert!(grouped["web"].contains("Web."));
    }

    #[tokio::test]
    async fn test_build_grouped_chicago() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        tracker
            .add_citation(Citation::new(
                "https://example.com".to_string(),
                "Title".to_string(),
                SourceType::Web,
            ))
            .await;

        let builder = ReferenceBuilder::new(tracker);
        let grouped = builder.build_grouped(ReferenceFormat::Chicago).await;
        assert!(grouped.contains_key("web"));
        assert!(grouped["web"].contains("Accessed"));
    }

    #[tokio::test]
    async fn test_build_grouped_json() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        tracker
            .add_citation(Citation::new(
                "https://example.com".to_string(),
                "Title".to_string(),
                SourceType::Web,
            ))
            .await;

        let builder = ReferenceBuilder::new(tracker);
        let grouped = builder.build_grouped(ReferenceFormat::Json).await;
        assert!(grouped.contains_key("web"));
        assert!(grouped["web"].contains("\"title\": \"Title\""));
    }

    #[tokio::test]
    async fn test_build_inline_citations_markdown() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let citation =
            Citation::new("https://example.com".to_string(), "Title".to_string(), SourceType::Web);
        let id_prefix = citation.id[..8].to_string();
        tracker.add_citation(citation).await;

        let builder = ReferenceBuilder::new(tracker);
        let citations = tracker.get_all_citations().await;
        let inline = builder
            .build_inline_citations(&citations, ReferenceFormat::Markdown)
            .await;
        assert!(inline.contains(&format!("[^{}]", id_prefix)));
    }

    #[tokio::test]
    async fn test_build_inline_citations_html() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let citation =
            Citation::new("https://example.com".to_string(), "Title".to_string(), SourceType::Web);
        let id_clone = citation.id.clone();
        tracker.add_citation(citation).await;

        let builder = ReferenceBuilder::new(tracker);
        let citations = tracker.get_all_citations().await;
        let inline = builder
            .build_inline_citations(&citations, ReferenceFormat::Html)
            .await;
        assert!(inline.contains(&format!("#ref-{}", id_clone)));
        assert!(inline.contains("<sup>"));
    }

    #[tokio::test]
    async fn test_build_inline_citations_other_format() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let citation =
            Citation::new("https://example.com".to_string(), "Title".to_string(), SourceType::Web);
        tracker.add_citation(citation).await;

        let builder = ReferenceBuilder::new(tracker);
        let citations = tracker.get_all_citations().await;
        let inline = builder
            .build_inline_citations(&citations, ReferenceFormat::BibTeX)
            .await;
        assert!(inline.is_empty());
    }

    #[tokio::test]
    async fn test_build_footnote_references() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        let citation = Citation::new(
            "https://example.com".to_string(),
            "Footnote Title".to_string(),
            SourceType::Web,
        );
        tracker.add_citation(citation).await;

        let builder = ReferenceBuilder::new(tracker);
        let citations = tracker.get_all_citations().await;
        let footnotes = builder.build_footnote_references(&citations).await;
        assert!(footnotes.contains("Footnote Title"));
        assert!(footnotes.contains("https://example.com"));
    }

    #[tokio::test]
    async fn test_build_footnote_references_multiple() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        tracker
            .add_citation(Citation::new(
                "https://a.com".to_string(),
                "Title A".to_string(),
                SourceType::Web,
            ))
            .await;
        tracker
            .add_citation(Citation::new(
                "https://b.com".to_string(),
                "Title B".to_string(),
                SourceType::Academic,
            ))
            .await;

        let builder = ReferenceBuilder::new(tracker);
        let citations = tracker.get_all_citations().await;
        let footnotes = builder.build_footnote_references(&citations).await;
        assert!(footnotes.contains("1."));
        assert!(footnotes.contains("2."));
        assert!(footnotes.contains("Title A"));
        assert!(footnotes.contains("Title B"));
    }

    #[test]
    fn test_reference_format_as_str() {
        assert_eq!(ReferenceFormat::Markdown.as_str(), "markdown");
        assert_eq!(ReferenceFormat::Html.as_str(), "html");
        assert_eq!(ReferenceFormat::Json.as_str(), "json");
        assert_eq!(ReferenceFormat::BibTeX.as_str(), "bibtex");
        assert_eq!(ReferenceFormat::APA.as_str(), "apa");
        assert_eq!(ReferenceFormat::MLA.as_str(), "mla");
        assert_eq!(ReferenceFormat::Chicago.as_str(), "chicago");
    }

    #[test]
    fn test_reference_format_equality() {
        assert_eq!(ReferenceFormat::Markdown, ReferenceFormat::Markdown);
        assert_ne!(ReferenceFormat::Markdown, ReferenceFormat::Html);
    }

    #[test]
    fn test_reference_format_serialization() {
        let format = ReferenceFormat::BibTeX;
        let json = serde_json::to_string(&format).unwrap();
        let deserialized: ReferenceFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ReferenceFormat::BibTeX);
    }

    #[test]
    fn test_format_date_apa() {
        let date = chrono::Utc::now();
        let formatted = ReferenceFormatter::format_date_apa(date);
        assert!(!formatted.is_empty());
        assert!(formatted.contains(&date.format("%Y").to_string()));
    }

    #[test]
    fn test_format_date_mla() {
        let date = chrono::Utc::now();
        let formatted = ReferenceFormatter::format_date_mla(date);
        assert!(!formatted.is_empty());
    }

    #[test]
    fn test_format_date_chicago() {
        let date = chrono::Utc::now();
        let formatted = ReferenceFormatter::format_date_chicago(date);
        assert!(!formatted.is_empty());
        assert!(formatted.contains(&date.format("%Y").to_string()));
    }

    #[test]
    fn test_truncate_url_short() {
        let url = "https://a.co";
        let truncated = ReferenceFormatter::truncate_url(url, 50);
        assert_eq!(truncated, url);
    }

    #[test]
    fn test_truncate_url_exact_length() {
        let url = "https://example.com/exact";
        let truncated = ReferenceFormatter::truncate_url(url, url.len());
        assert_eq!(truncated, url);
    }

    #[test]
    fn test_sanitize_bibtex_empty() {
        let output = ReferenceFormatter::sanitize_for_bibtex("");
        assert_eq!(output, "");
    }

    #[test]
    fn test_sanitize_bibtex_no_special() {
        let output = ReferenceFormatter::sanitize_for_bibtex("Hello World");
        assert_eq!(output, "Hello World");
    }

    #[tokio::test]
    async fn test_build_markdown_credibility_levels() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        tracker
            .add_citation(Citation::new(
                "https://high.com".to_string(),
                "High".to_string(),
                SourceType::Academic,
            ))
            .await;
        tracker
            .add_citation(Citation::new(
                "https://low.com".to_string(),
                "Low".to_string(),
                SourceType::Forum,
            ))
            .await;

        let builder = ReferenceBuilder::new(tracker);
        let markdown = builder.build(ReferenceFormat::Markdown).await;
        assert!(markdown.contains("[![High Credibility]](high)"));
        assert!(markdown.contains("[![Low Credibility]](low)"));
    }

    #[tokio::test]
    async fn test_build_html_credibility_classes() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        tracker
            .add_citation(Citation::new(
                "https://high.com".to_string(),
                "High".to_string(),
                SourceType::Academic,
            ))
            .await;
        tracker
            .add_citation(Citation::new(
                "https://low.com".to_string(),
                "Low".to_string(),
                SourceType::Forum,
            ))
            .await;

        let builder = ReferenceBuilder::new(tracker);
        let html = builder.build(ReferenceFormat::Html).await;
        assert!(html.contains("credibility-high"));
        assert!(html.contains("credibility-low"));
    }

    #[tokio::test]
    async fn test_build_bibtex_news_type() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        tracker
            .add_citation(Citation::new(
                "https://news.com".to_string(),
                "News Title".to_string(),
                SourceType::News,
            ))
            .await;

        let builder = ReferenceBuilder::new(tracker);
        let bibtex = builder.build(ReferenceFormat::BibTeX).await;
        assert!(bibtex.contains("@article"));
    }

    #[tokio::test]
    async fn test_build_json_structure() {
        let tracker = std::sync::Arc::new(CitationTracker::new());
        tracker
            .add_citation(Citation::new(
                "https://example.com".to_string(),
                "JSON Title".to_string(),
                SourceType::GitHub,
            ))
            .await;

        let builder = ReferenceBuilder::new(tracker);
        let json = builder.build(ReferenceFormat::Json).await;
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[0]["title"], "JSON Title");
        assert_eq!(parsed[0]["type"], "github");
        assert!(parsed[0]["id"].is_string());
    }
}
