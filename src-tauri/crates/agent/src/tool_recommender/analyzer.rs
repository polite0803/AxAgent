use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    InformationRetrieval,
    CodeGeneration,
    DataAnalysis,
    FileOperation,
    WebInteraction,
    ContentCreation,
    ProblemSolving,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_analyzer_new() {
        let analyzer = ContextAnalyzer::new();
        let ctx = analyzer.analyze("search for rust tutorials");
        assert_eq!(ctx.task_type, TaskType::InformationRetrieval);
    }

    #[test]
    fn test_analyze_information_retrieval() {
        let analyzer = ContextAnalyzer::new();
        let ctx = analyzer.analyze("find the best restaurants");
        assert_eq!(ctx.task_type, TaskType::InformationRetrieval);
    }

    #[test]
    fn test_analyze_code_generation() {
        let analyzer = ContextAnalyzer::new();
        let ctx = analyzer.analyze("implement a sorting function");
        assert_eq!(ctx.task_type, TaskType::CodeGeneration);
    }

    #[test]
    fn test_analyze_data_analysis() {
        let analyzer = ContextAnalyzer::new();
        let ctx = analyzer.analyze("analyze the sales data from Q4");
        assert_eq!(ctx.task_type, TaskType::DataAnalysis);
    }

    #[test]
    fn test_analyze_file_operation() {
        let analyzer = ContextAnalyzer::new();
        let ctx = analyzer.analyze("read the file at /tmp/config.txt");
        assert_eq!(ctx.task_type, TaskType::FileOperation);
    }

    #[test]
    fn test_analyze_web_interaction() {
        let analyzer = ContextAnalyzer::new();
        let ctx = analyzer.analyze("browse the website for product info");
        assert_eq!(ctx.task_type, TaskType::WebInteraction);
    }

    #[test]
    fn test_analyze_content_creation() {
        let analyzer = ContextAnalyzer::new();
        let ctx = analyzer.analyze("write a blog post about AI");
        assert_eq!(ctx.task_type, TaskType::ContentCreation);
    }

    #[test]
    fn test_analyze_problem_solving_default() {
        let analyzer = ContextAnalyzer::new();
        let ctx = analyzer.analyze("optimize the system performance");
        assert_eq!(ctx.task_type, TaskType::ProblemSolving);
    }

    #[test]
    fn test_analyze_extracts_url_entity() {
        let analyzer = ContextAnalyzer::new();
        let ctx = analyzer.analyze("visit https://example.com for details");
        let urls: Vec<_> = ctx.entities.iter().filter(|e| e.entity_type == EntityType::Url).collect();
        assert!(!urls.is_empty());
        assert!(urls[0].value.contains("example.com"));
        assert!((urls[0].confidence - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_analyze_extracts_language_entity() {
        let analyzer = ContextAnalyzer::new();
        let ctx = analyzer.analyze("write a python script for data processing");
        let langs: Vec<_> = ctx.entities.iter().filter(|e| e.entity_type == EntityType::Language).collect();
        assert!(!langs.is_empty());
        assert_eq!(langs[0].value, "python");
    }

    #[test]
    fn test_analyze_extracts_speed_constraint() {
        let analyzer = ContextAnalyzer::new();
        let ctx = analyzer.analyze("search for results fast");
        let speed_constraints: Vec<_> = ctx.constraints.iter().filter(|c| c.constraint_type == "speed").collect();
        assert!(!speed_constraints.is_empty());
    }

    #[test]
    fn test_analyze_extracts_accuracy_constraint() {
        let analyzer = ContextAnalyzer::new();
        let ctx = analyzer.analyze("provide accurate measurements");
        let acc_constraints: Vec<_> = ctx.constraints.iter().filter(|c| c.constraint_type == "accuracy").collect();
        assert!(!acc_constraints.is_empty());
    }

    #[test]
    fn test_analyze_preserves_description() {
        let analyzer = ContextAnalyzer::new();
        let desc = "test task description";
        let ctx = analyzer.analyze(desc);
        assert_eq!(ctx.task_description, desc);
    }

    #[test]
    fn test_context_analyzer_default() {
        let analyzer = ContextAnalyzer::default();
        let ctx = analyzer.analyze("test");
        assert_eq!(ctx.task_description, "test");
    }

    #[test]
    fn test_task_type_variants() {
        let types = vec![
            TaskType::InformationRetrieval,
            TaskType::CodeGeneration,
            TaskType::DataAnalysis,
            TaskType::FileOperation,
            TaskType::WebInteraction,
            TaskType::ContentCreation,
            TaskType::ProblemSolving,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let de: TaskType = serde_json::from_str(&json).unwrap();
            assert_eq!(de, t);
        }
    }

    #[test]
    fn test_entity_type_variants() {
        let types = vec![
            EntityType::FilePath,
            EntityType::Url,
            EntityType::CodeSnippet,
            EntityType::Command,
            EntityType::Language,
            EntityType::Framework,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let de: EntityType = serde_json::from_str(&json).unwrap();
            assert_eq!(de, t);
        }
    }

    #[test]
    fn test_entity_serialization() {
        let entity = Entity {
            entity_type: EntityType::Url,
            value: "https://example.com".to_string(),
            confidence: 0.95,
        };
        let json = serde_json::to_string(&entity).unwrap();
        let de: Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(de.value, "https://example.com");
    }

    #[test]
    fn test_constraint_serialization() {
        let constraint = Constraint {
            constraint_type: "speed".to_string(),
            value: "fast".to_string(),
        };
        let json = serde_json::to_string(&constraint).unwrap();
        let de: Constraint = serde_json::from_str(&json).unwrap();
        assert_eq!(de.constraint_type, "speed");
    }
}
