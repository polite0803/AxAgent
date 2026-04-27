use crate::research_state::{OutlineSection, ReportOutline, ResearchState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutlineStyle {
    Standard,
    Academic,
    Technical,
    Executive,
}

impl OutlineStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutlineStyle::Standard => "standard",
            OutlineStyle::Academic => "academic",
            OutlineStyle::Technical => "technical",
            OutlineStyle::Executive => "executive",
        }
    }
}

pub struct OutlineBuilder {
    style: OutlineStyle,
    max_sections: usize,
    include_subsections: bool,
}

impl OutlineBuilder {
    pub fn new() -> Self {
        Self {
            style: OutlineStyle::Standard,
            max_sections: 6,
            include_subsections: true,
        }
    }

    pub fn with_style(mut self, style: OutlineStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_max_sections(mut self, max: usize) -> Self {
        self.max_sections = max;
        self
    }

    pub fn with_subsections(mut self, include: bool) -> Self {
        self.include_subsections = include;
        self
    }

    pub async fn build(&self, state: &ResearchState) -> ReportOutline {
        let topic = &state.topic;

        let sections = match self.style {
            OutlineStyle::Standard => self.build_standard_outline(topic),
            OutlineStyle::Academic => self.build_academic_outline(topic),
            OutlineStyle::Technical => self.build_technical_outline(topic),
            OutlineStyle::Executive => self.build_executive_outline(topic),
        };

        let title = self.generate_title(topic);

        ReportOutline { title, sections }
    }

    fn generate_title(&self, topic: &str) -> String {
        match self.style {
            OutlineStyle::Academic => format!("Research Report: {}", topic),
            OutlineStyle::Technical => format!("Technical Analysis: {}", topic),
            OutlineStyle::Executive => format!("Executive Summary: {}", topic),
            OutlineStyle::Standard => format!("Report: {}", topic),
        }
    }

    fn build_standard_outline(&self, _topic: &str) -> Vec<OutlineSection> {
        let section_titles = vec![
            "Introduction",
            "Background",
            "Methods",
            "Findings",
            "Discussion",
            "Conclusion",
        ];

        section_titles
            .into_iter()
            .take(self.max_sections)
            .map(|title| {
                let description = match title {
                    "Introduction" => "Overview of the topic and research objectives",
                    "Background" => "Context and relevant prior work",
                    "Methods" => "Approach and methodology used",
                    "Findings" => "Key results and discoveries",
                    "Discussion" => "Analysis and interpretation of results",
                    "Conclusion" => "Summary and future directions",
                    _ => "",
                };

                OutlineSection::new(title.to_string())
                    .with_description(description.to_string())
                    .with_subsections(self.get_subsections_for_section(title))
            })
            .collect()
    }

    fn build_academic_outline(&self, _topic: &str) -> Vec<OutlineSection> {
        let section_titles = vec![
            "Abstract",
            "1. Introduction",
            "2. Literature Review",
            "3. Methodology",
            "4. Results",
            "5. Discussion",
            "6. Conclusion",
            "References",
        ];

        section_titles
            .into_iter()
            .take(self.max_sections)
            .map(|title| {
                let description = match title {
                    "Abstract" => "Brief summary of the research",
                    "1. Introduction" => "Research problem and objectives",
                    "2. Literature Review" => "Related work and theoretical framework",
                    "3. Methodology" => "Research design and methods",
                    "4. Results" => "Empirical findings",
                    "5. Discussion" => "Interpretation and implications",
                    "6. Conclusion" => "Contributions and future work",
                    "References" => "Citations and bibliography",
                    _ => "",
                };

                OutlineSection::new(title.to_string())
                    .with_description(description.to_string())
                    .with_subsections(self.get_academic_subsections(title))
            })
            .collect()
    }

    fn build_technical_outline(&self, _topic: &str) -> Vec<OutlineSection> {
        let section_titles = vec![
            "Overview",
            "Requirements",
            "Architecture",
            "Implementation",
            "Testing",
            "Deployment",
            "Maintenance",
        ];

        section_titles
            .into_iter()
            .take(self.max_sections)
            .map(|title| {
                let description = match title {
                    "Overview" => "High-level summary and goals",
                    "Requirements" => "Functional and non-functional requirements",
                    "Architecture" => "System design and components",
                    "Implementation" => "Technical approach and decisions",
                    "Testing" => "Verification and validation",
                    "Deployment" => "Rollout and operations",
                    "Maintenance" => "Long-term support and updates",
                    _ => "",
                };

                OutlineSection::new(title.to_string())
                    .with_description(description.to_string())
                    .with_subsections(self.get_technical_subsections(title))
            })
            .collect()
    }

    fn build_executive_outline(&self, _topic: &str) -> Vec<OutlineSection> {
        let section_titles = vec![
            "Executive Summary",
            "Key Findings",
            "Recommendations",
            "Implementation Plan",
            "Risks and Mitigations",
            "Conclusion",
        ];

        section_titles
            .into_iter()
            .take(self.max_sections)
            .map(|title| {
                let description = match title {
                    "Executive Summary" => "Brief overview for decision makers",
                    "Key Findings" => "Critical insights from the analysis",
                    "Recommendations" => "Suggested actions and priorities",
                    "Implementation Plan" => "Timeline and resource requirements",
                    "Risks and Mitigations" => "Potential issues and solutions",
                    "Conclusion" => "Final remarks and next steps",
                    _ => "",
                };

                OutlineSection::new(title.to_string())
                    .with_description(description.to_string())
                    .with_subsections(Vec::new())
            })
            .collect()
    }

    fn get_subsections_for_section(&self, section: &str) -> Vec<String> {
        if !self.include_subsections {
            return Vec::new();
        }

        match section {
            "Introduction" => vec![
                "Research Problem".to_string(),
                "Objectives".to_string(),
                "Scope".to_string(),
            ],
            "Background" => vec![
                "Historical Context".to_string(),
                "Current State".to_string(),
                "Related Work".to_string(),
            ],
            "Methods" => vec![
                "Data Collection".to_string(),
                "Analysis Approach".to_string(),
                "Tools Used".to_string(),
            ],
            "Findings" => vec![
                "Primary Results".to_string(),
                "Secondary Results".to_string(),
                "Unexpected Discoveries".to_string(),
            ],
            "Discussion" => vec![
                "Interpretation".to_string(),
                "Implications".to_string(),
                "Limitations".to_string(),
            ],
            "Conclusion" => vec![
                "Summary".to_string(),
                "Contributions".to_string(),
                "Future Work".to_string(),
            ],
            _ => Vec::new(),
        }
    }

    fn get_academic_subsections(&self, section: &str) -> Vec<String> {
        if !self.include_subsections {
            return Vec::new();
        }

        match section {
            "1. Introduction" => vec![
                "Problem Statement".to_string(),
                "Research Questions".to_string(),
                "Significance".to_string(),
            ],
            "2. Literature Review" => vec![
                "Theoretical Framework".to_string(),
                "Prior Studies".to_string(),
                "Research Gap".to_string(),
            ],
            "3. Methodology" => vec![
                "Research Design".to_string(),
                "Data Collection".to_string(),
                "Analysis Methods".to_string(),
            ],
            "4. Results" => vec![
                "Descriptive Statistics".to_string(),
                "Hypothesis Testing".to_string(),
                "Key Findings".to_string(),
            ],
            "5. Discussion" => vec![
                "Interpretation".to_string(),
                "Theoretical Implications".to_string(),
                "Practical Implications".to_string(),
            ],
            "6. Conclusion" => vec![
                "Summary of Findings".to_string(),
                "Contributions".to_string(),
                "Limitations".to_string(),
                "Future Research".to_string(),
            ],
            _ => Vec::new(),
        }
    }

    fn get_technical_subsections(&self, section: &str) -> Vec<String> {
        if !self.include_subsections {
            return Vec::new();
        }

        match section {
            "Overview" => vec!["Project Goals".to_string(), "Success Criteria".to_string()],
            "Requirements" => vec![
                "Functional Requirements".to_string(),
                "Non-Functional Requirements".to_string(),
                "Constraints".to_string(),
            ],
            "Architecture" => vec![
                "System Components".to_string(),
                "Data Flow".to_string(),
                "Technology Stack".to_string(),
            ],
            "Implementation" => vec![
                "Development Approach".to_string(),
                "Key Decisions".to_string(),
                "Code Organization".to_string(),
            ],
            "Testing" => vec![
                "Test Strategy".to_string(),
                "Test Cases".to_string(),
                "Results".to_string(),
            ],
            "Deployment" => vec![
                "Deployment Strategy".to_string(),
                "Configuration".to_string(),
                "Monitoring".to_string(),
            ],
            _ => Vec::new(),
        }
    }

    pub fn validate_outline(&self, outline: &ReportOutline) -> Vec<OutlineValidationError> {
        let mut errors = Vec::new();

        if outline.title.is_empty() {
            errors.push(OutlineValidationError {
                field: "title".to_string(),
                message: "Report title is required".to_string(),
            });
        }

        if outline.sections.is_empty() {
            errors.push(OutlineValidationError {
                field: "sections".to_string(),
                message: "At least one section is required".to_string(),
            });
        }

        for (i, section) in outline.sections.iter().enumerate() {
            if section.title.is_empty() {
                errors.push(OutlineValidationError {
                    field: format!("sections[{}].title", i),
                    message: "Section title is required".to_string(),
                });
            }
        }

        errors
    }
}

impl Default for OutlineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineValidationError {
    pub field: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build_standard_outline() {
        let builder = OutlineBuilder::new();
        let state = ResearchState::new("Test Topic".to_string());

        let outline = builder.build(&state).await;

        assert!(!outline.title.is_empty());
        assert!(!outline.sections.is_empty());
    }

    #[tokio::test]
    async fn test_build_academic_outline() {
        let builder = OutlineBuilder::new().with_style(OutlineStyle::Academic);
        let state = ResearchState::new("Research Topic".to_string());

        let outline = builder.build(&state).await;

        assert!(outline.title.contains("Research Report"));
        assert!(outline
            .sections
            .iter()
            .any(|s| s.title.contains("Abstract")));
    }

    #[tokio::test]
    async fn test_build_executive_outline() {
        let builder = OutlineBuilder::new()
            .with_style(OutlineStyle::Executive)
            .with_subsections(false);
        let state = ResearchState::new("Business Topic".to_string());

        let outline = builder.build(&state).await;

        assert!(outline.title.contains("Executive Summary"));
        assert!(outline.sections.iter().all(|s| s.subsections.is_empty()));
    }

    #[test]
    fn test_validate_outline() {
        let builder = OutlineBuilder::new();

        let empty_outline = ReportOutline::new();
        let errors = builder.validate_outline(&empty_outline);
        assert!(!errors.is_empty());

        let valid_outline = ReportOutline::new()
            .with_title("Test Report".to_string())
            .add_section(OutlineSection::new("Introduction".to_string()));

        let errors = builder.validate_outline(&valid_outline);
        assert!(errors.is_empty());
    }
}
