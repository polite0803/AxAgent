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
}
