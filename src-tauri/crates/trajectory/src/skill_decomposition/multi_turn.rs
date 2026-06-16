// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum MessageType {
    Thinking,
    Observation,
    Analysis,
    Decision,
    Warning,
    Error,
    Success,
    Question,
    FinalResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum TurnStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct AgentMessage {
    pub id: String,
    pub turn: u32,
    pub message_type: MessageType,
    pub timestamp: i64,
    pub content: String,
    pub metadata: Option<MessageMetadata>,
    pub children: Vec<String>,
    pub status: MessageStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum MessageStatus {
    Streaming,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct MessageMetadata {
    pub file: Option<String>,
    pub function: Option<String>,
    pub confidence: Option<f32>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct TurnContext {
    pub turn_id: u32,
    pub turn_name: String,
    pub status: TurnStatus,
    pub messages: Vec<String>,
    pub result: Option<serde_json::Value>,
}

#[allow(dead_code)]
impl TurnContext {
    pub(crate) fn new(turn_id: u32, turn_name: &str) -> Self {
        Self {
            turn_id,
            turn_name: turn_name.to_string(),
            status: TurnStatus::Pending,
            messages: Vec::new(),
            result: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct FileReference {
    pub from_file: String,
    pub to_file: String,
    pub to_function: Option<String>,
    pub reference_type: ReferenceType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum ReferenceType {
    Import,
    Include,
    Reference,
    Calls,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct SkillFile {
    pub path: String,
    pub file_type: FileType,
    pub content: String,
    pub code_blocks: Vec<CodeBlock>,
    pub references: Vec<FileReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum FileType {
    Markdown,
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Go,
    Java,
    CSharp,
    Cpp,
    Ruby,
    Shell,
    Config,
    Json,
    Yaml,
    Toml,
    Text,
    Other,
}

#[allow(dead_code)]
impl FileType {
    pub(crate) fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "md" | "markdown" => FileType::Markdown,
            "py" | "python" => FileType::Python,
            "js" | "javascript" | "jsx" => FileType::JavaScript,
            "ts" | "typescript" | "tsx" => FileType::TypeScript,
            "rs" | "rust" => FileType::Rust,
            "go" => FileType::Go,
            "java" => FileType::Java,
            "cs" | "csharp" => FileType::CSharp,
            "cpp" | "cc" | "cxx" | "c" | "h" | "hpp" => FileType::Cpp,
            "rb" | "ruby" => FileType::Ruby,
            "sh" | "bash" | "zsh" | "shell" => FileType::Shell,
            "yaml" | "yml" => FileType::Yaml,
            "json" => FileType::Json,
            "toml" | "ini" | "conf" | "cfg" | "properties" => FileType::Config,
            "txt" | "text" => FileType::Text,
            _ => FileType::Other,
        }
    }

    pub(crate) fn is_code(&self) -> bool {
        matches!(
            self,
            FileType::Python
                | FileType::JavaScript
                | FileType::TypeScript
                | FileType::Rust
                | FileType::Go
                | FileType::Java
                | FileType::CSharp
                | FileType::Cpp
                | FileType::Ruby
                | FileType::Shell
        )
    }

    pub(crate) fn language_name(&self) -> &'static str {
        match self {
            FileType::Python => "python",
            FileType::JavaScript => "javascript",
            FileType::TypeScript => "typescript",
            FileType::Rust => "rust",
            FileType::Go => "go",
            FileType::Java => "java",
            FileType::CSharp => "csharp",
            FileType::Cpp => "cpp",
            FileType::Ruby => "ruby",
            FileType::Shell => "shell",
            _ => "",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct CodeBlock {
    pub id: String,
    pub language: Option<String>,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct SkillPackage {
    pub files: Vec<SkillFile>,
    pub main_file: Option<String>,
    pub metadata: PackageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct PackageMetadata {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct ConversationTurn {
    pub role: MessageRole,
    pub content: String,
    pub artifacts: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum MessageRole {
    System,
    User,
    Assistant,
}

#[allow(dead_code)]
pub(crate) struct DecompositionSession {
    pub session_id: String,
    pub package: Option<SkillPackage>,
    pub turns: HashMap<u32, TurnContext>,
    pub messages: HashMap<String, AgentMessage>,
    pub current_turn: u32,
    pub current_message_id: Option<String>,
    pub conversation_history: Vec<ConversationTurn>,
    pub state: SessionState,
    pub partial_results: PartialResults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum SessionState {
    Initializing,
    AwaitingFiles,
    Understanding,
    Analyzing,
    Designing,
    Generating,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct PartialResults {
    pub file_structure: Option<FileStructure>,
    pub content_classifications: Vec<FileClassification>,
    pub function_analysis: Vec<FunctionAnalysis>,
    pub workflow_design: Option<WorkflowDesign>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct FileStructure {
    pub main_file: String,
    pub supporting_files: Vec<String>,
    pub relationships: Vec<FileRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct FileRelationship {
    pub from_file: String,
    pub to_file: String,
    pub relationship_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct FileClassification {
    pub file: String,
    pub types: Vec<String>,
    pub contained_elements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct FunctionAnalysis {
    pub name: String,
    pub function_type: String,
    pub description: String,
    pub input_params: Vec<ParamInfo>,
    pub output: String,
    pub dependencies: Vec<String>,
    pub can_be_independent: bool,
    pub suggested_skill_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct ParamInfo {
    pub name: String,
    pub param_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct WorkflowDesign {
    pub workflow_type: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub optimization_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct WorkflowNode {
    pub id: String,
    pub node_type: String,
    pub skill_ref: Option<String>,
    pub input_mapping: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct WorkflowEdge {
    pub from_node: String,
    pub to_node: String,
    pub edge_type: String,
    pub data_mapping: HashMap<String, String>,
}

#[allow(dead_code)]
impl DecompositionSession {
    pub(crate) fn new(session_id: String) -> Self {
        Self {
            session_id,
            package: None,
            turns: HashMap::new(),
            messages: HashMap::new(),
            current_turn: 0,
            current_message_id: None,
            conversation_history: Vec::new(),
            state: SessionState::Initializing,
            partial_results: PartialResults::default(),
        }
    }

    pub(crate) fn init_turns(&mut self) {
        let turn_names = vec!["文件理解", "内容分类", "功能分析", "工作流设计", "生成输出"];

        for (idx, name) in turn_names.into_iter().enumerate() {
            self.turns
                .insert(idx as u32 + 1, TurnContext::new(idx as u32 + 1, name));
        }
    }

    pub(crate) fn add_message(&mut self, message: AgentMessage) {
        let msg_id = message.id.clone();
        self.messages.insert(msg_id.clone(), message);

        if let Some(turn_ctx) = self.turns.get_mut(&self.current_turn) {
            turn_ctx.messages.push(msg_id);
        }
    }

    pub(crate) fn start_turn(&mut self, turn_id: u32) {
        self.current_turn = turn_id;
        if let Some(turn_ctx) = self.turns.get_mut(&turn_id) {
            turn_ctx.status = TurnStatus::InProgress;
        }
        self.current_message_id = None;
    }

    pub(crate) fn complete_turn(&mut self, turn_id: u32) {
        if let Some(turn_ctx) = self.turns.get_mut(&turn_id) {
            turn_ctx.status = TurnStatus::Completed;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct DecompositionEvent {
    pub event_type: EventType,
    pub turn: u32,
    pub message_id: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum EventType {
    TurnStart,
    TurnProgress,
    MessageStart,
    MessageChunk,
    MessageComplete,
    TurnComplete,
    FinalResult,
    Error,
}

#[allow(dead_code)]
impl DecompositionEvent {
    pub(crate) fn turn_start(turn: u32, content: &str) -> Self {
        Self {
            event_type: EventType::TurnStart,
            turn,
            message_id: uuid::Uuid::new_v4().to_string(),
            content: content.to_string(),
            metadata: None,
        }
    }

    pub(crate) fn message_chunk(turn: u32, msg_id: &str, content: &str) -> Self {
        Self {
            event_type: EventType::MessageChunk,
            turn,
            message_id: msg_id.to_string(),
            content: content.to_string(),
            metadata: None,
        }
    }

    pub(crate) fn message_complete(
        turn: u32,
        msg_id: &str,
        content: &str,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        Self {
            event_type: EventType::MessageComplete,
            turn,
            message_id: msg_id.to_string(),
            content: content.to_string(),
            metadata,
        }
    }

    pub(crate) fn turn_complete(
        turn: u32,
        content: &str,
        result: Option<serde_json::Value>,
    ) -> Self {
        Self {
            event_type: EventType::TurnComplete,
            turn,
            message_id: uuid::Uuid::new_v4().to_string(),
            content: content.to_string(),
            metadata: result.map(|r| serde_json::json!({ "result": r })),
        }
    }

    pub(crate) fn final_result(result: serde_json::Value) -> Self {
        Self {
            event_type: EventType::FinalResult,
            turn: 0,
            message_id: uuid::Uuid::new_v4().to_string(),
            content: String::new(),
            metadata: Some(serde_json::json!({ "result": result })),
        }
    }

    pub(crate) fn error(message: &str) -> Self {
        Self {
            event_type: EventType::Error,
            turn: 0,
            message_id: uuid::Uuid::new_v4().to_string(),
            content: message.to_string(),
            metadata: None,
        }
    }
}
