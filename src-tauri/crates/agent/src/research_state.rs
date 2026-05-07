use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchPhase {
    Planning,
    Searching,
    Extracting,
    Analyzing,
    Synthesizing,
    Reporting,
}

impl ResearchPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResearchPhase::Planning => "planning",
            ResearchPhase::Searching => "searching",
            ResearchPhase::Extracting => "extracting",
            ResearchPhase::Analyzing => "analyzing",
            ResearchPhase::Synthesizing => "synthesizing",
            ResearchPhase::Reporting => "reporting",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ResearchPhase::Planning => "规划中",
            ResearchPhase::Searching => "搜索中",
            ResearchPhase::Extracting => "信息提取",
            ResearchPhase::Analyzing => "分析中",
            ResearchPhase::Synthesizing => "综合中",
            ResearchPhase::Reporting => "报告生成",
        }
    }

    pub fn progress_percentage(&self) -> u8 {
        match self {
            ResearchPhase::Planning => 10,
            ResearchPhase::Searching => 30,
            ResearchPhase::Extracting => 50,
            ResearchPhase::Analyzing => 70,
            ResearchPhase::Synthesizing => 85,
            ResearchPhase::Reporting => 95,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchStatus {
    Pending,
    InProgress,
    Paused,
    Completed,
    Failed,
}

impl ResearchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResearchStatus::Pending => "pending",
            ResearchStatus::InProgress => "in_progress",
            ResearchStatus::Paused => "paused",
            ResearchStatus::Completed => "completed",
            ResearchStatus::Failed => "failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, ResearchStatus::Completed | ResearchStatus::Failed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub source_type: SourceType,
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub published_date: Option<String>,
    pub credibility_score: Option<f32>,
    pub relevance_score: f32,
    pub extracted_at: DateTime<Utc>,
}

impl SearchResult {
    pub fn new(source_type: SourceType, url: String, title: String, snippet: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_type,
            url,
            title,
            snippet,
            published_date: None,
            credibility_score: None,
            relevance_score: 0.0,
            extracted_at: Utc::now(),
        }
    }

    pub fn with_published_date(mut self, date: String) -> Self {
        self.published_date = Some(date);
        self
    }

    pub fn with_credibility(mut self, score: f32) -> Self {
        self.credibility_score = Some(score);
        self
    }

    pub fn with_relevance(mut self, score: f32) -> Self {
        self.relevance_score = score;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceType {
    Web,
    Academic,
    Wikipedia,
    GitHub,
    Documentation,
    News,
    Blog,
    Forum,
    Unknown,
}

impl SourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::Web => "web",
            SourceType::Academic => "academic",
            SourceType::Wikipedia => "wikipedia",
            SourceType::GitHub => "github",
            SourceType::Documentation => "documentation",
            SourceType::News => "news",
            SourceType::Blog => "blog",
            SourceType::Forum => "forum",
            SourceType::Unknown => "unknown",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SourceType::Web => "网页",
            SourceType::Academic => "学术",
            SourceType::Wikipedia => "维基百科",
            SourceType::GitHub => "GitHub",
            SourceType::Documentation => "文档",
            SourceType::News => "新闻",
            SourceType::Blog => "博客",
            SourceType::Forum => "论坛",
            SourceType::Unknown => "未知",
        }
    }

    pub fn default_credibility(&self) -> f32 {
        match self {
            SourceType::Academic => 0.9,
            SourceType::Wikipedia => 0.7,
            SourceType::Documentation => 0.8,
            SourceType::News => 0.6,
            SourceType::GitHub => 0.75,
            SourceType::Web => 0.5,
            SourceType::Blog => 0.4,
            SourceType::Forum => 0.3,
            SourceType::Unknown => 0.2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub id: String,
    pub source_url: String,
    pub source_title: String,
    pub source_type: SourceType,
    pub accessed_at: DateTime<Utc>,
    pub quoted_text: Option<String>,
    pub page_number: Option<u32>,
    pub credibility: f32,
    pub in_report: bool,
}

impl Citation {
    pub fn new(source_url: String, source_title: String, source_type: SourceType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_url,
            source_title,
            source_type,
            accessed_at: Utc::now(),
            quoted_text: None,
            page_number: None,
            credibility: source_type.default_credibility(),
            in_report: false,
        }
    }

    pub fn with_quoted_text(mut self, text: String) -> Self {
        self.quoted_text = Some(text);
        self
    }

    pub fn with_page(mut self, page: u32) -> Self {
        self.page_number = Some(page);
        self
    }

    pub fn with_credibility(mut self, score: f32) -> Self {
        self.credibility = score;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchProgress {
    pub phase: ResearchPhase,
    pub percentage: u8,
    pub current_query: Option<String>,
    pub sources_found: usize,
    pub sources_processed: usize,
    pub citations_added: usize,
    pub errors: Vec<String>,
}

impl ResearchProgress {
    pub fn new() -> Self {
        Self {
            phase: ResearchPhase::Planning,
            percentage: 0,
            current_query: None,
            sources_found: 0,
            sources_processed: 0,
            citations_added: 0,
            errors: Vec::new(),
        }
    }

    pub fn with_phase(mut self, phase: ResearchPhase) -> Self {
        self.phase = phase;
        self.percentage = phase.progress_percentage();
        self
    }

    pub fn with_query(mut self, query: String) -> Self {
        self.current_query = Some(query);
        self
    }

    pub fn increment_sources_found(&mut self, count: usize) {
        self.sources_found += count;
    }

    pub fn increment_sources_processed(&mut self) {
        self.sources_processed += 1;
    }

    pub fn increment_citations(&mut self) {
        self.citations_added += 1;
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
}

impl Default for ResearchProgress {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConfig {
    pub max_sources: usize,
    pub max_citations: usize,
    pub parallel_searches: usize,
    pub include_credibility_check: bool,
    pub report_format: ReportFormat,
}

impl Default for ResearchConfig {
    fn default() -> Self {
        Self {
            max_sources: 50,
            max_citations: 20,
            parallel_searches: 5,
            include_credibility_check: true,
            report_format: ReportFormat::Markdown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportFormat {
    Markdown,
    Html,
    Json,
}

impl ReportFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReportFormat::Markdown => "markdown",
            ReportFormat::Html => "html",
            ReportFormat::Json => "json",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchState {
    pub id: String,
    pub topic: String,
    pub status: ResearchStatus,
    pub current_phase: ResearchPhase,
    pub search_results: Vec<SearchResult>,
    pub citations: Vec<Citation>,
    pub progress: ResearchProgress,
    pub config: ResearchConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl ResearchState {
    pub fn new(topic: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic,
            status: ResearchStatus::Pending,
            current_phase: ResearchPhase::Planning,
            search_results: Vec::new(),
            citations: Vec::new(),
            progress: ResearchProgress::new(),
            config: ResearchConfig::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn with_config(mut self, config: ResearchConfig) -> Self {
        self.config = config;
        self
    }

    pub fn start(&mut self) {
        self.status = ResearchStatus::InProgress;
        self.current_phase = ResearchPhase::Planning;
        self.progress = ResearchProgress::new().with_phase(ResearchPhase::Planning);
        self.updated_at = Utc::now();
    }

    pub fn pause(&mut self) {
        self.status = ResearchStatus::Paused;
        self.updated_at = Utc::now();
    }

    pub fn resume(&mut self) {
        self.status = ResearchStatus::InProgress;
        self.updated_at = Utc::now();
    }

    pub fn complete(&mut self) {
        self.status = ResearchStatus::Completed;
        self.current_phase = ResearchPhase::Reporting;
        self.progress.percentage = 100;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn fail(&mut self, error: String) {
        self.status = ResearchStatus::Failed;
        self.progress.add_error(error);
        self.updated_at = Utc::now();
    }

    pub fn set_phase(&mut self, phase: ResearchPhase) {
        self.current_phase = phase;
        self.progress = self.progress.clone().with_phase(phase);
        self.updated_at = Utc::now();
    }

    pub fn add_search_result(&mut self, result: SearchResult) {
        self.search_results.push(result);
        self.progress.increment_sources_found(1);
        self.updated_at = Utc::now();
    }

    pub fn add_citation(&mut self, citation: Citation) {
        self.citations.push(citation);
        self.progress.increment_citations();
        self.updated_at = Utc::now();
    }

    pub fn is_complete(&self) -> bool {
        self.status == ResearchStatus::Completed
    }

    pub fn is_failed(&self) -> bool {
        self.status == ResearchStatus::Failed
    }

    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub id: String,
    pub query: String,
    pub source_types: Vec<SourceType>,
    pub max_results: usize,
}

impl SearchQuery {
    pub fn new(query: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            query,
            source_types: vec![SourceType::Web, SourceType::Wikipedia],
            max_results: 10,
        }
    }

    pub fn with_sources(mut self, sources: Vec<SourceType>) -> Self {
        self.source_types = sources;
        self
    }

    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPlan {
    pub id: String,
    pub queries: Vec<SearchQuery>,
    pub parallel_groups: Vec<Vec<String>>,
}

impl SearchPlan {
    pub fn new(queries: Vec<SearchQuery>) -> Self {
        let query_ids: Vec<String> = queries.iter().map(|q| q.id.clone()).collect();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            queries,
            parallel_groups: vec![query_ids],
        }
    }

    pub fn with_parallel_groups(mut self, groups: Vec<Vec<String>>) -> Self {
        self.parallel_groups = groups;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchReport {
    pub id: String,
    pub topic: String,
    pub outline: ReportOutline,
    pub content: String,
    pub citations: Vec<Citation>,
    pub summary: String,
    pub created_at: DateTime<Utc>,
}

impl ResearchReport {
    pub fn new(topic: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic,
            outline: ReportOutline::new(),
            content: String::new(),
            citations: Vec::new(),
            summary: String::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_outline(mut self, outline: ReportOutline) -> Self {
        self.outline = outline;
        self
    }

    pub fn with_content(mut self, content: String) -> Self {
        self.content = content;
        self
    }

    pub fn with_citations(mut self, citations: Vec<Citation>) -> Self {
        self.citations = citations;
        self
    }

    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = summary;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportOutline {
    pub title: String,
    pub sections: Vec<OutlineSection>,
}

impl ReportOutline {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            sections: Vec::new(),
        }
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.title = title;
        self
    }

    pub fn add_section(mut self, section: OutlineSection) -> Self {
        self.sections.push(section);
        self
    }
}

impl Default for ReportOutline {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineSection {
    pub id: String,
    pub title: String,
    pub description: String,
    pub subsections: Vec<String>,
}

impl OutlineSection {
    pub fn new(title: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            description: String::new(),
            subsections: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn with_subsections(mut self, subsections: Vec<String>) -> Self {
        self.subsections = subsections;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_research_phase_as_str() {
        assert_eq!(ResearchPhase::Planning.as_str(), "planning");
        assert_eq!(ResearchPhase::Searching.as_str(), "searching");
        assert_eq!(ResearchPhase::Extracting.as_str(), "extracting");
        assert_eq!(ResearchPhase::Analyzing.as_str(), "analyzing");
        assert_eq!(ResearchPhase::Synthesizing.as_str(), "synthesizing");
        assert_eq!(ResearchPhase::Reporting.as_str(), "reporting");
    }

    #[test]
    fn test_research_phase_display_name() {
        assert!(!ResearchPhase::Planning.display_name().is_empty());
        assert!(!ResearchPhase::Searching.display_name().is_empty());
    }

    #[test]
    fn test_research_phase_progress_percentage() {
        assert!(ResearchPhase::Planning.progress_percentage() < ResearchPhase::Searching.progress_percentage());
        assert!(ResearchPhase::Searching.progress_percentage() < ResearchPhase::Extracting.progress_percentage());
        assert!(ResearchPhase::Extracting.progress_percentage() < ResearchPhase::Analyzing.progress_percentage());
        assert!(ResearchPhase::Analyzing.progress_percentage() < ResearchPhase::Synthesizing.progress_percentage());
        assert!(ResearchPhase::Synthesizing.progress_percentage() < ResearchPhase::Reporting.progress_percentage());
    }

    #[test]
    fn test_research_status_as_str() {
        assert_eq!(ResearchStatus::Pending.as_str(), "pending");
        assert_eq!(ResearchStatus::InProgress.as_str(), "in_progress");
        assert_eq!(ResearchStatus::Paused.as_str(), "paused");
        assert_eq!(ResearchStatus::Completed.as_str(), "completed");
        assert_eq!(ResearchStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_research_status_is_terminal() {
        assert!(!ResearchStatus::Pending.is_terminal());
        assert!(!ResearchStatus::InProgress.is_terminal());
        assert!(!ResearchStatus::Paused.is_terminal());
        assert!(ResearchStatus::Completed.is_terminal());
        assert!(ResearchStatus::Failed.is_terminal());
    }

    #[test]
    fn test_source_type_as_str() {
        assert_eq!(SourceType::Web.as_str(), "web");
        assert_eq!(SourceType::Academic.as_str(), "academic");
        assert_eq!(SourceType::Wikipedia.as_str(), "wikipedia");
        assert_eq!(SourceType::GitHub.as_str(), "github");
        assert_eq!(SourceType::Documentation.as_str(), "documentation");
        assert_eq!(SourceType::News.as_str(), "news");
        assert_eq!(SourceType::Blog.as_str(), "blog");
        assert_eq!(SourceType::Forum.as_str(), "forum");
        assert_eq!(SourceType::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_source_type_default_credibility() {
        assert!(SourceType::Academic.default_credibility() > SourceType::Web.default_credibility());
        assert!(SourceType::Documentation.default_credibility() > SourceType::Blog.default_credibility());
        assert!(SourceType::Forum.default_credibility() < SourceType::News.default_credibility());
    }

    #[test]
    fn test_source_type_display_name() {
        assert!(!SourceType::Web.display_name().is_empty());
        assert!(!SourceType::Academic.display_name().is_empty());
    }

    #[test]
    fn test_search_result_new() {
        let result = SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Example".to_string(),
            "A snippet".to_string(),
        );
        assert_eq!(result.source_type, SourceType::Web);
        assert_eq!(result.url, "https://example.com");
        assert!(result.published_date.is_none());
        assert!(result.credibility_score.is_none());
        assert_eq!(result.relevance_score, 0.0);
    }

    #[test]
    fn test_search_result_builder() {
        let result = SearchResult::new(SourceType::Academic, "url".to_string(), "Title".to_string(), "snippet".to_string())
            .with_published_date("2024-01-01".to_string())
            .with_credibility(0.9)
            .with_relevance(0.85);
        assert_eq!(result.published_date, Some("2024-01-01".to_string()));
        assert_eq!(result.credibility_score, Some(0.9));
        assert!((result.relevance_score - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn test_citation_new() {
        let citation = Citation::new("url".to_string(), "Title".to_string(), SourceType::Wikipedia);
        assert_eq!(citation.source_type, SourceType::Wikipedia);
        assert!((citation.credibility - 0.7).abs() < f32::EPSILON);
        assert!(citation.quoted_text.is_none());
        assert!(citation.page_number.is_none());
        assert!(!citation.in_report);
    }

    #[test]
    fn test_citation_builder() {
        let citation = Citation::new("url".to_string(), "Title".to_string(), SourceType::Academic)
            .with_quoted_text("quoted text".to_string())
            .with_page(42)
            .with_credibility(0.95);
        assert_eq!(citation.quoted_text, Some("quoted text".to_string()));
        assert_eq!(citation.page_number, Some(42));
        assert!((citation.credibility - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn test_research_progress_new() {
        let progress = ResearchProgress::new();
        assert_eq!(progress.phase, ResearchPhase::Planning);
        assert_eq!(progress.percentage, 0);
        assert!(progress.current_query.is_none());
        assert_eq!(progress.sources_found, 0);
        assert_eq!(progress.errors.len(), 0);
    }

    #[test]
    fn test_research_progress_with_phase() {
        let progress = ResearchProgress::new().with_phase(ResearchPhase::Analyzing);
        assert_eq!(progress.phase, ResearchPhase::Analyzing);
        assert_eq!(progress.percentage, 70);
    }

    #[test]
    fn test_research_progress_increment() {
        let mut progress = ResearchProgress::new();
        progress.increment_sources_found(5);
        assert_eq!(progress.sources_found, 5);
        progress.increment_sources_processed();
        assert_eq!(progress.sources_processed, 1);
        progress.increment_citations();
        assert_eq!(progress.citations_added, 1);
    }

    #[test]
    fn test_research_progress_add_error() {
        let mut progress = ResearchProgress::new();
        progress.add_error("test error".to_string());
        assert_eq!(progress.errors.len(), 1);
        assert_eq!(progress.errors[0], "test error");
    }

    #[test]
    fn test_research_config_default() {
        let config = ResearchConfig::default();
        assert_eq!(config.max_sources, 50);
        assert_eq!(config.max_citations, 20);
        assert_eq!(config.parallel_searches, 5);
        assert!(config.include_credibility_check);
        assert_eq!(config.report_format, ReportFormat::Markdown);
    }

    #[test]
    fn test_report_format_as_str() {
        assert_eq!(ReportFormat::Markdown.as_str(), "markdown");
        assert_eq!(ReportFormat::Html.as_str(), "html");
        assert_eq!(ReportFormat::Json.as_str(), "json");
    }

    #[test]
    fn test_research_state_new() {
        let state = ResearchState::new("Test topic".to_string());
        assert_eq!(state.topic, "Test topic");
        assert_eq!(state.status, ResearchStatus::Pending);
        assert_eq!(state.current_phase, ResearchPhase::Planning);
        assert!(state.search_results.is_empty());
        assert!(state.citations.is_empty());
        assert!(state.completed_at.is_none());
    }

    #[test]
    fn test_research_state_lifecycle() {
        let mut state = ResearchState::new("Topic".to_string());
        state.start();
        assert_eq!(state.status, ResearchStatus::InProgress);

        state.pause();
        assert_eq!(state.status, ResearchStatus::Paused);

        state.resume();
        assert_eq!(state.status, ResearchStatus::InProgress);

        state.complete();
        assert!(state.is_complete());
        assert!(state.is_terminal());
        assert!(state.completed_at.is_some());
    }

    #[test]
    fn test_research_state_fail() {
        let mut state = ResearchState::new("Topic".to_string());
        state.start();
        state.fail("Something went wrong".to_string());
        assert!(state.is_failed());
        assert!(state.is_terminal());
        assert_eq!(state.progress.errors.len(), 1);
    }

    #[test]
    fn test_research_state_set_phase() {
        let mut state = ResearchState::new("Topic".to_string());
        state.start();
        state.set_phase(ResearchPhase::Searching);
        assert_eq!(state.current_phase, ResearchPhase::Searching);
        assert_eq!(state.progress.phase, ResearchPhase::Searching);
    }

    #[test]
    fn test_research_state_add_search_result() {
        let mut state = ResearchState::new("Topic".to_string());
        let result = SearchResult::new(SourceType::Web, "url".to_string(), "Title".to_string(), "snippet".to_string());
        state.add_search_result(result);
        assert_eq!(state.search_results.len(), 1);
        assert_eq!(state.progress.sources_found, 1);
    }

    #[test]
    fn test_research_state_add_citation() {
        let mut state = ResearchState::new("Topic".to_string());
        let citation = Citation::new("url".to_string(), "Title".to_string(), SourceType::Academic);
        state.add_citation(citation);
        assert_eq!(state.citations.len(), 1);
        assert_eq!(state.progress.citations_added, 1);
    }

    #[test]
    fn test_search_query_new() {
        let query = SearchQuery::new("test query".to_string());
        assert_eq!(query.query, "test query");
        assert_eq!(query.source_types, vec![SourceType::Web, SourceType::Wikipedia]);
        assert_eq!(query.max_results, 10);
    }

    #[test]
    fn test_search_query_builder() {
        let query = SearchQuery::new("test".to_string())
            .with_sources(vec![SourceType::Academic, SourceType::GitHub])
            .with_max_results(20);
        assert_eq!(query.source_types.len(), 2);
        assert_eq!(query.max_results, 20);
    }

    #[test]
    fn test_search_plan_new() {
        let queries = vec![
            SearchQuery::new("q1".to_string()),
            SearchQuery::new("q2".to_string()),
        ];
        let plan = SearchPlan::new(queries);
        assert_eq!(plan.queries.len(), 2);
        assert_eq!(plan.parallel_groups.len(), 1);
        assert_eq!(plan.parallel_groups[0].len(), 2);
    }

    #[test]
    fn test_research_report_new() {
        let report = ResearchReport::new("Topic".to_string());
        assert_eq!(report.topic, "Topic");
        assert!(report.content.is_empty());
        assert!(report.citations.is_empty());
        assert!(report.summary.is_empty());
    }

    #[test]
    fn test_research_report_builder() {
        let report = ResearchReport::new("Topic".to_string())
            .with_content("Full content".to_string())
            .with_summary("Summary".to_string());
        assert_eq!(report.content, "Full content");
        assert_eq!(report.summary, "Summary");
    }

    #[test]
    fn test_report_outline_new() {
        let outline = ReportOutline::new();
        assert!(outline.title.is_empty());
        assert!(outline.sections.is_empty());
    }

    #[test]
    fn test_report_outline_builder() {
        let outline = ReportOutline::new()
            .with_title("My Report".to_string())
            .add_section(OutlineSection::new("Introduction".to_string()));
        assert_eq!(outline.title, "My Report");
        assert_eq!(outline.sections.len(), 1);
    }

    #[test]
    fn test_outline_section_new() {
        let section = OutlineSection::new("Section Title".to_string());
        assert_eq!(section.title, "Section Title");
        assert!(section.description.is_empty());
        assert!(section.subsections.is_empty());
    }

    #[test]
    fn test_outline_section_builder() {
        let section = OutlineSection::new("Title".to_string())
            .with_description("Description".to_string())
            .with_subsections(vec!["Sub1".to_string(), "Sub2".to_string()]);
        assert_eq!(section.description, "Description");
        assert_eq!(section.subsections.len(), 2);
    }
}
