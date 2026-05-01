use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtDocument {
    pub id: String,
    pub content: String,
    pub client_states: HashMap<String, ClientState>,
    pub version: u64,
    pub operations: Vec<CrdtOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientState {
    pub client_id: String,
    pub last_applied_op: u64,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtOperation {
    pub id: u64,
    pub op_type: OperationType,
    pub position: usize,
    pub client_id: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OperationType {
    #[serde(rename = "insert")]
    Insert { text: String },
    #[serde(rename = "delete")]
    Delete { length: usize },
    #[serde(rename = "replace")]
    Replace { text: String, length: usize },
}

pub struct CrdtEngine {
    documents: HashMap<String, CrdtDocument>,
}

impl Default for CrdtEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CrdtEngine {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    pub fn create_document(&mut self, id: &str, initial_content: &str) -> &CrdtDocument {
        let doc = CrdtDocument {
            id: id.to_string(),
            content: initial_content.to_string(),
            client_states: HashMap::new(),
            version: 0,
            operations: Vec::new(),
        };
        self.documents.insert(id.to_string(), doc);
        self.documents.get(id).unwrap()
    }

    pub fn apply_local_operation(
        &mut self,
        doc_id: &str,
        client_id: &str,
        op_type: OperationType,
        position: usize,
    ) -> Result<CrdtOperation, String> {
        let doc = self.documents.get_mut(doc_id).ok_or("Document not found")?;
        let op = CrdtOperation {
            id: doc.operations.len() as u64,
            op_type,
            position,
            client_id: client_id.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        doc.content = Self::apply_op_to_content(&doc.content, &op);
        doc.operations.push(op.clone());
        doc.version += 1;

        doc.client_states
            .entry(client_id.to_string())
            .and_modify(|s| s.last_applied_op = op.id)
            .or_insert(ClientState {
                client_id: client_id.to_string(),
                last_applied_op: op.id,
                last_seen: chrono::Utc::now().timestamp(),
            });

        Ok(op)
    }

    pub fn apply_remote_operation(
        &mut self,
        doc_id: &str,
        remote_op: CrdtOperation,
    ) -> Result<(), String> {
        let doc = self.documents.get_mut(doc_id).ok_or("Document not found")?;

        if doc.operations.iter().any(|o| o.id == remote_op.id) {
            return Ok(());
        }

        doc.content = Self::apply_op_to_content(&doc.content, &remote_op);
        doc.operations.push(remote_op.clone());
        doc.version += 1;

        doc.client_states
            .entry(remote_op.client_id.clone())
            .and_modify(|s| s.last_applied_op = remote_op.id)
            .or_insert(ClientState {
                client_id: remote_op.client_id.clone(),
                last_applied_op: remote_op.id,
                last_seen: chrono::Utc::now().timestamp(),
            });

        Ok(())
    }

    pub fn get_pending_operations(
        &self,
        doc_id: &str,
        since_op_id: u64,
    ) -> Result<Vec<CrdtOperation>, String> {
        let doc = self.documents.get(doc_id).ok_or("Document not found")?;
        Ok(doc
            .operations
            .iter()
            .filter(|op| op.id > since_op_id)
            .cloned()
            .collect())
    }

    pub fn get_document_content(&self, doc_id: &str) -> Result<String, String> {
        self.documents
            .get(doc_id)
            .map(|d| d.content.clone())
            .ok_or_else(|| "Document not found".to_string())
    }

    pub fn get_document(&self, doc_id: &str) -> Result<&CrdtDocument, String> {
        self.documents
            .get(doc_id)
            .ok_or_else(|| "Document not found".to_string())
    }

    fn apply_op_to_content(content: &str, op: &CrdtOperation) -> String {
        let chars: Vec<char> = content.chars().collect();
        match &op.op_type {
            OperationType::Insert { text } => {
                let mut result: String = chars[..op.position.min(chars.len())].iter().collect();
                result.push_str(text);
                result.push_str(
                    &chars[op.position.min(chars.len())..]
                        .iter()
                        .collect::<String>(),
                );
                result
            },
            OperationType::Delete { length } => {
                let start = op.position.min(chars.len());
                let end = (start + length).min(chars.len());
                let mut result: String = chars[..start].iter().collect();
                result.push_str(&chars[end..].iter().collect::<String>());
                result
            },
            OperationType::Replace { text, length } => {
                let start = op.position.min(chars.len());
                let end = (start + length).min(chars.len());
                let mut result: String = chars[..start].iter().collect();
                result.push_str(text);
                result.push_str(&chars[end..].iter().collect::<String>());
                result
            },
        }
    }
}
