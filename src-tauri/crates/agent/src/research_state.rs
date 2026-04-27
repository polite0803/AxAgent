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
