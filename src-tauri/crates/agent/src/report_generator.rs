use crate::citation_tracker::CitationTracker;
use crate::content_synthesizer::{ContentFormatter, ContentSynthesizer};
use crate::outline_builder::OutlineBuilder;
use crate::reference_builder::{ReferenceBuilder, ReferenceFormat};
use crate::research_state::{ReportFormat, ResearchReport, ResearchState};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("Outline generation failed: {0}")]
    OutlineError(String),

    #[error("Content synthesis failed: {0}")]
    SynthesisError(String),

    #[error("Reference building failed: {0}")]
    ReferenceError(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportStyle {
    Standard,
    Academic,
    Technical,
    Executive,
}

pub struct ReportGenerator {
    outline_builder: OutlineBuilder,
    citation_tracker: Arc<CitationTracker>,
}

impl ReportGenerator {
    pub fn new(citation_tracker: Arc<CitationTracker>) -> Self {
        Self {
            outline_builder: OutlineBuilder::new(),
            citation_tracker,
        }
    }

    pub fn with_style(mut self, style: ReportStyle) -> Self {
        let outline_style = match style {
            ReportStyle::Standard => crate::outline_builder::OutlineStyle::Standard,
            ReportStyle::Academic => crate::outline_builder::OutlineStyle::Academic,
            ReportStyle::Technical => crate::outline_builder::OutlineStyle::Technical,
            ReportStyle::Executive => crate::outline_builder::OutlineStyle::Executive,
        };
        self.outline_builder = OutlineBuilder::new().with_style(outline_style);
        self
    }

    pub async fn generate(&self, state: &ResearchState) -> Result<ResearchReport, ReportError> {
        let outline = self
            .build_outline(state)
            .await
            .map_err(ReportError::OutlineError)?;

        let sections_content = self
            .generate_sections(&outline, state)
            .await
            .map_err(ReportError::SynthesisError)?;

        let content = sections_content.join("\n\n");

        let citations = self.citation_tracker.get_all_citations().await;

        let summary = self
            .generate_summary(&sections_content)
            .await
            .map_err(ReportError::SynthesisError)?;

        let report = ResearchReport {
            id: uuid::Uuid::new_v4().to_string(),
            topic: state.topic.clone(),
            outline,
            content,
            citations,
            summary,
            created_at: chrono::Utc::now(),
        };

        Ok(report)
    }

    pub async fn generate_with_format(
        &self,
        state: &ResearchState,
        format: ReportFormat,
    ) -> Result<String, ReportError> {
        let report = self.generate(state).await?;

        match format {
            ReportFormat::Markdown => Ok(report.content),
            ReportFormat::Html => Ok(self.to_html(&report)),
            ReportFormat::Json => serde_json::to_string_pretty(&report)
                .map_err(|e| ReportError::ValidationError(e.to_string())),
        }
    }

    async fn build_outline(
        &self,
        state: &ResearchState,
    ) -> Result<crate::research_state::ReportOutline, String> {
        Ok(self.outline_builder.build(state).await)
    }

    async fn generate_sections(
        &self,
        outline: &crate::research_state::ReportOutline,
        state: &ResearchState,
    ) -> Result<Vec<String>, String> {
        let synthesizer = ContentSynthesizer::new(self.citation_tracker.clone());

        let sources: Vec<_> = state.search_results.clone();

        let contents = synthesizer
            .synthesize_batch(&outline.sections, &sources)
            .await;

        let mut full_contents = Vec::new();

        full_contents.push(format!("# {}\n\n", outline.title));

        for content in contents {
            full_contents.push(content);
            full_contents.push("\n\n---\n\n".to_string());
        }

        Ok(full_contents)
    }

    async fn generate_summary(&self, sections_content: &[String]) -> Result<String, String> {
        let synthesizer = ContentSynthesizer::new(self.citation_tracker.clone());
        Ok(synthesizer.generate_summary(sections_content).await)
    }

    fn to_html(&self, report: &ResearchReport) -> String {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str("<title>");
        html.push_str(&report.topic);
        html.push_str("</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }\n");
        html.push_str("h1 { color: #333; }\nh2 { color: #555; border-bottom: 1px solid #eee; padding-bottom: 10px; }\n");
        html.push_str(".meta { color: #888; font-size: 0.9em; }\n");
        html.push_str("a { color: #0066cc; }\n");
        html.push_str("</style>\n</head>\n<body>\n");

        html.push_str(&format!("<h1>{}</h1>\n", report.outline.title));
        html.push_str(&format!(
            "<p class=\"meta\">Generated: {}</p>\n",
            report.created_at.format("%Y-%m-%d %H:%M UTC")
        ));

        html.push_str(&ContentFormatter::to_html(&report.content));

        if !report.citations.is_empty() {
            html.push_str("<hr>\n<h2>References</h2>\n<ol>\n");
            for citation in &report.citations {
                html.push_str(&format!(
                    "<li><a href=\"{}\">{}</a> (Credibility: {:.0}%)</li>\n",
                    citation.source_url,
                    citation.source_title,
                    citation.credibility * 100.0
                ));
            }
            html.push_str("</ol>\n");
        }

        if !report.summary.is_empty() {
            html.push_str("<hr>\n<div class=\"summary\">\n");
            html.push_str(&ContentFormatter::to_html(&report.summary));
            html.push_str("</div>\n");
        }

        html.push_str("</body>\n</html>");

        html
    }

    pub async fn generate_references(
        &self,
        format: ReferenceFormat,
    ) -> Result<String, ReportError> {
        let builder = ReferenceBuilder::new(self.citation_tracker.clone());
        Ok(builder.build(format).await)
    }

    pub async fn build_outline_markdown(
        &self,
        state: &ResearchState,
    ) -> Result<String, ReportError> {
        let outline = self
            .build_outline(state)
            .await
            .map_err(ReportError::OutlineError)?;

        let mut md = format!("# {}\n\n", outline.title);

        for (i, section) in outline.sections.iter().enumerate() {
            md.push_str(&format!("## {}. {}\n\n", i + 1, section.title));

            if !section.description.is_empty() {
                md.push_str(&format!("*{}*\n\n", section.description));
            }

            if !section.subsections.is_empty() {
                md.push_str("**Subsections:**\n");
                for subsection in &section.subsections {
                    md.push_str(&format!("- {}\n", subsection));
                }
                md.push('\n');
            }
        }

        Ok(md)
    }

    pub async fn validate_report(&self, report: &ResearchReport) -> Result<(), ReportError> {
        if report.outline.title.is_empty() {
            return Err(ReportError::ValidationError(
                "Report title is required".to_string(),
            ));
        }

        if report.outline.sections.is_empty() {
            return Err(ReportError::ValidationError(
                "Report must have at least one section".to_string(),
            ));
        }

        if report.content.is_empty() {
            return Err(ReportError::ValidationError(
                "Report content is empty".to_string(),
            ));
        }

        Ok(())
    }
}

pub struct ReportExporter;

impl ReportExporter {
    pub fn export_to_file(content: &str, path: &str) -> std::io::Result<()> {
        std::fs::write(path, content)
    }

    pub fn export_to_json<T: serde::Serialize>(data: &T) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(data)
    }

    pub fn export_to_markdown(report: &ResearchReport) -> String {
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", report.outline.title));
        md.push_str(&format!(
            "*Generated: {}*\n\n",
            report.created_at.format("%Y-%m-%d %H:%M UTC")
        ));

        md.push_str(&report.content);

        if !report.citations.is_empty() {
            md.push_str("\n\n## References\n\n");
            for citation in &report.citations {
                md.push_str(&format!(
                    "- [{}]({}) (Credibility: {:.0}%)\n",
                    citation.source_title,
                    citation.source_url,
                    citation.credibility * 100.0
                ));
            }
        }

        if !report.summary.is_empty() {
            md.push_str("\n\n## Summary\n\n");
            md.push_str(&report.summary);
        }

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_report() {
        let tracker = Arc::new(CitationTracker::new());
        let generator = ReportGenerator::new(tracker);

        let state = ResearchState::new("Test Topic".to_string());
        let report = generator.generate(&state).await;

        assert!(report.is_ok());
        let report = report.unwrap();
        assert!(!report.outline.title.is_empty());
    }

    #[tokio::test]
    async fn test_build_outline_markdown() {
        let tracker = Arc::new(CitationTracker::new());
        let generator = ReportGenerator::new(tracker);

        let state = ResearchState::new("Test Topic".to_string());
        let md = generator.build_outline_markdown(&state).await;

        assert!(md.is_ok());
        let md = md.unwrap();
        assert!(md.contains("# Test Topic"));
    }

    #[tokio::test]
    async fn test_validate_report() {
        let tracker = Arc::new(CitationTracker::new());
        let generator = ReportGenerator::new(tracker);

        let state = ResearchState::new("Test Topic".to_string());
        let report = generator.generate(&state).await.unwrap();

        let result = generator.validate_report(&report).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_to_markdown() {
        let report = ResearchReport {
            id: "test".to_string(),
            topic: "Test".to_string(),
            outline: crate::research_state::ReportOutline::new()
                .with_title("Test Report".to_string()),
            content: "Content".to_string(),
            citations: vec![],
            summary: "Summary".to_string(),
            created_at: chrono::Utc::now(),
        };

        let md = ReportExporter::export_to_markdown(&report);
        assert!(md.contains("# Test Report"));
        assert!(md.contains("Content"));
    }
}
