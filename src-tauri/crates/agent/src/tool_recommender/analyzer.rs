use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    InformationRetrieval,
    CodeGeneration,
    DataAnalysis,
    FileOperation,
    WebInteraction,
    ContentCreation,
    ProblemSolving,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    FilePath,
    Url,
    CodeSnippet,
    Command,
    Language,
    Framework,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub entity_type: EntityType,
    pub value: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub constraint_type: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPattern {
    pub pattern_id: String,
    pub task_signature: String,
    pub tools_used: Vec<String>,
    pub success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub task_description: String,
    pub task_type: TaskType,
    pub entities: Vec<Entity>,
    pub constraints: Vec<Constraint>,
    pub historical_patterns: Vec<TaskPattern>,
}

pub struct ContextAnalyzer {
    task_parser: TaskParser,
    entity_extractor: EntityExtractor,
    intent_classifier: IntentClassifier,
}

impl ContextAnalyzer {
    pub fn new() -> Self {
        Self {
            task_parser: TaskParser::new(),
            entity_extractor: EntityExtractor::new(),
            intent_classifier: IntentClassifier::new(),
        }
    }

    pub fn analyze(&self, task_description: &str) -> TaskContext {
        let task_type = self.intent_classifier.classify(task_description);
        let entities = self.entity_extractor.extract(task_description);
        let constraints = self.task_parser.parse_constraints(task_description);
        let historical_patterns = Vec::new();

        TaskContext {
            task_description: task_description.to_string(),
            task_type,
            entities,
            constraints,
            historical_patterns,
        }
    }
}

impl Default for ContextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

struct TaskParser;

impl TaskParser {
    fn new() -> Self {
        Self
    }

    fn parse_constraints(&self, task_description: &str) -> Vec<Constraint> {
        let mut constraints = Vec::new();

        if task_description.contains("fast") || task_description.contains("quick") {
            constraints.push(Constraint {
                constraint_type: "speed".to_string(),
                value: "fast".to_string(),
            });
        }

        if task_description.contains("accurate") || task_description.contains("precise") {
            constraints.push(Constraint {
                constraint_type: "accuracy".to_string(),
                value: "high".to_string(),
            });
        }

        constraints
    }
}

struct EntityExtractor;

impl EntityExtractor {
    fn new() -> Self {
        Self
    }

    fn extract(&self, text: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        let url_regex = regex_lite::Regex::new(r"https?://[^\s]+").unwrap();
        for cap in url_regex.find_iter(text) {
            entities.push(Entity {
                entity_type: EntityType::Url,
                value: cap.as_str().to_string(),
                confidence: 0.95,
            });
        }

        let file_path_regex = regex_lite::Regex::new(r"[a-zA-Z]:\\[^\s]+|/[^\s]+").unwrap();
        for cap in file_path_regex.find_iter(text) {
            entities.push(Entity {
                entity_type: EntityType::FilePath,
                value: cap.as_str().to_string(),
                confidence: 0.9,
            });
        }

        let code_keywords = [
            "python",
            "javascript",
            "rust",
            "java",
            "cpp",
            "go",
            "typescript",
        ];
        for keyword in code_keywords {
            if text.to_lowercase().contains(keyword) {
                entities.push(Entity {
                    entity_type: EntityType::Language,
                    value: keyword.to_string(),
                    confidence: 0.85,
                });
            }
        }

        entities
    }
}

struct IntentClassifier;

impl IntentClassifier {
    fn new() -> Self {
        Self
    }

    fn classify(&self, task_description: &str) -> TaskType {
        let desc_lower = task_description.to_lowercase();

        if desc_lower.contains("search")
            || desc_lower.contains("find")
            || desc_lower.contains("lookup")
        {
            TaskType::InformationRetrieval
        } else if desc_lower.contains("code")
            || desc_lower.contains("function")
            || desc_lower.contains("implement")
        {
            TaskType::CodeGeneration
        } else if desc_lower.contains("analyze")
            || desc_lower.contains("data")
            || desc_lower.contains("statistics")
        {
            TaskType::DataAnalysis
        } else if desc_lower.contains("file")
            || desc_lower.contains("folder")
            || desc_lower.contains("directory")
        {
            TaskType::FileOperation
        } else if desc_lower.contains("browse")
            || desc_lower.contains("web")
            || desc_lower.contains("website")
        {
            TaskType::WebInteraction
        } else if desc_lower.contains("write")
            || desc_lower.contains("create")
            || desc_lower.contains("generate")
        {
            TaskType::ContentCreation
        } else {
            TaskType::ProblemSolving
        }
    }
}
