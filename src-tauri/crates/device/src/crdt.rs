// SPDX-License-Identifier: AGPL-3.0-only

//! CRDT（冲突-free 可复制数据类型）模块
//!
//! 为多设备同步提供冲突解决支持。
//! 基于操作转换（OT）原理实现，支持 Insert/Delete/Replace 操作的并发转换。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CRDT 文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtDocument {
    pub id: String,
    pub content: String,
    pub client_states: HashMap<String, ClientState>,
    pub version: u64,
    pub operations: Vec<CrdtOperation>,
}

/// 客户端状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientState {
    pub client_id: String,
    pub last_applied_op: u64,
    pub last_seen: i64,
}

/// CRDT 操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtOperation {
    pub id: u64,
    pub op_type: OperationType,
    pub position: usize,
    pub client_id: String,
    pub timestamp: i64,
}

/// 操作类型
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

impl Clone for CrdtEngine {
    fn clone(&self) -> Self {
        Self { documents: self.documents.clone() }
    }
}

/// CRDT 引擎
#[derive(Debug, Default)]
pub struct CrdtEngine {
    documents: HashMap<String, CrdtDocument>,
}

impl CrdtEngine {
    pub fn new() -> Self {
        Self { documents: HashMap::new() }
    }

    /// 创建文档
    pub fn create_document(&mut self, id: &str, initial_content: &str) {
        let doc = CrdtDocument {
            id: id.to_string(),
            content: initial_content.to_string(),
            client_states: HashMap::new(),
            version: 0,
            operations: Vec::new(),
        };
        self.documents.insert(id.to_string(), doc);
    }

    /// 检查文档是否存在
    pub fn has_document(&self, id: &str) -> bool {
        self.documents.contains_key(id)
    }

    /// 应用本地操作
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

    /// 应用远程操作（带去重）
    pub fn apply_remote_operation(
        &mut self,
        doc_id: &str,
        remote_op: CrdtOperation,
    ) -> Result<bool, String> {
        // 先检查是否存在并获取文档内容
        if !self.documents.contains_key(doc_id) {
            return Err("Document not found".to_string());
        }

        let doc = self.documents.get_mut(doc_id).expect("CRDT：文档应在 contains_key 检查后存在");

        // 去重：跳过已应用的操作
        if doc.operations.iter().any(|o| o.id == remote_op.id) {
            return Ok(false);
        }

        // 获取当前操作列表用于转换
        let local_ops: Vec<CrdtOperation> = doc.operations.clone();

        // 操作转换：根据本地已有的操作调整远程操作位置
        let adjusted_op = Self::transform_operation_from_ops(&local_ops, &remote_op);

        doc.content = Self::apply_op_to_content(&doc.content, &adjusted_op);
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

        Ok(true)
    }

    /// 获取待同步的操作
    pub fn get_pending_operations(
        &self,
        doc_id: &str,
        since_op_id: u64,
    ) -> Result<Vec<CrdtOperation>, String> {
        let doc = self.documents.get(doc_id).ok_or("Document not found")?;
        Ok(doc.operations.iter().filter(|op| op.id > since_op_id).cloned().collect())
    }

    /// 获取文档内容
    pub fn get_document_content(&self, doc_id: &str) -> Result<String, String> {
        self.documents
            .get(doc_id)
            .map(|d| d.content.clone())
            .ok_or_else(|| "Document not found".to_string())
    }

    /// 获取文档版本
    pub fn get_document_version(&self, doc_id: &str) -> Result<u64, String> {
        self.documents
            .get(doc_id)
            .map(|d| d.version)
            .ok_or_else(|| "Document not found".to_string())
    }

    /// 合并远程操作（用于同步）
    pub fn merge_operations(
        &mut self,
        doc_id: &str,
        remote_ops: Vec<CrdtOperation>,
    ) -> Result<Vec<CrdtOperation>, String> {
        let mut applied_ops = Vec::new();

        for op in remote_ops {
            match self.apply_remote_operation(doc_id, op.clone()) {
                Ok(true) => applied_ops.push(op),
                Ok(false) => { /* 已存在，跳过 */ },
                Err(e) => return Err(e),
            }
        }

        Ok(applied_ops)
    }

    /// 操作转换：根据本地已有的操作调整远程操作位置
    fn transform_operation_from_ops(
        local_ops: &[CrdtOperation],
        remote_op: &CrdtOperation,
    ) -> CrdtOperation {
        let mut adjusted = remote_op.clone();
        let mut position_offset: i64 = 0;

        // 遍历本地操作，计算位置偏移
        for local_op in local_ops {
            if local_op.client_id == remote_op.client_id {
                continue; // 跳过自己的操作
            }

            match &local_op.op_type {
                OperationType::Insert { text } => {
                    // 本地插入操作：如果远程操作位置 >= 本地插入位置，偏移量增加
                    if remote_op.position >= local_op.position {
                        position_offset += text.len() as i64;
                    }
                },
                OperationType::Delete { length } => {
                    // 本地删除操作：计算位置影响
                    let del_end = local_op.position + length;
                    if remote_op.position >= local_op.position {
                        if remote_op.position >= del_end {
                            position_offset -= *length as i64;
                        } else {
                            // 远程操作在删除范围内，调整到删除位置
                            adjusted.position = local_op.position;
                        }
                    }
                },
                OperationType::Replace { length, .. } => {
                    // 替换操作：类似删除
                    if remote_op.position >= local_op.position {
                        let rep_end = local_op.position + length;
                        if remote_op.position >= rep_end {
                            position_offset -= *length as i64;
                        } else {
                            adjusted.position = local_op.position;
                        }
                    }
                },
            }
        }

        adjusted.position = (adjusted.position as i64 + position_offset).max(0) as usize;
        adjusted
    }

    fn apply_op_to_content(content: &str, op: &CrdtOperation) -> String {
        let chars: Vec<char> = content.chars().collect();
        match &op.op_type {
            OperationType::Insert { text } => {
                let pos = op.position.min(chars.len());
                let mut result: String = chars[..pos].iter().collect();
                result.push_str(text);
                result.push_str(&chars[pos..].iter().collect::<String>());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_document() {
        let mut engine = CrdtEngine::new();
        engine.create_document("doc-1", "Hello World");
        assert!(engine.has_document("doc-1"));
        assert_eq!(
            engine.get_document_content("doc-1").expect("测试：get_document_content 应成功"),
            "Hello World"
        );
    }

    #[test]
    fn test_apply_local_insert() {
        let mut engine = CrdtEngine::new();
        engine.create_document("doc-1", "Hello World");

        let op = engine
            .apply_local_operation(
                "doc-1",
                "client-1",
                OperationType::Insert { text: "Beautiful ".to_string() },
                6,
            )
            .expect("测试应成功");

        assert_eq!(op.id, 0);
        assert_eq!(
            engine.get_document_content("doc-1").expect("测试：get_document_content 应成功"),
            "Hello Beautiful World"
        );
    }

    #[test]
    fn test_apply_local_delete() {
        let mut engine = CrdtEngine::new();
        engine.create_document("doc-1", "Hello Beautiful World");

        engine
            .apply_local_operation("doc-1", "client-1", OperationType::Delete { length: 10 }, 6)
            .expect("测试应成功");

        assert_eq!(
            engine.get_document_content("doc-1").expect("测试：get_document_content 应成功"),
            "Hello World"
        );
    }

    #[test]
    fn test_remote_operation_with_transform() {
        let mut engine = CrdtEngine::new();
        engine.create_document("doc-1", "Hello World");

        // 客户端1在位置6插入
        engine
            .apply_local_operation(
                "doc-1",
                "client-1",
                OperationType::Insert { text: "Beautiful ".to_string() },
                6,
            )
            .expect("测试应成功");

        // 客户端2也在位置6插入（模拟并发），使用不同的 ID 避免去重
        let remote_op = CrdtOperation {
            id: 1000, // 使用不同于本地操作的 ID
            op_type: OperationType::Insert { text: "Nice ".to_string() },
            position: 6,
            client_id: "client-2".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        // 应用远程操作，应该被转换
        let result = engine
            .apply_remote_operation("doc-1", remote_op)
            .expect("测试：apply_remote_operation 应成功");
        assert!(result);

        // 远程操作的插入位置应该被调整到本地操作之后
        let content =
            engine.get_document_content("doc-1").expect("测试：get_document_content 应成功");
        assert_eq!(content, "Hello Beautiful Nice World");
    }

    #[test]
    fn test_deduplication() {
        let mut engine = CrdtEngine::new();
        engine.create_document("doc-1", "Hello World");

        let op = engine
            .apply_local_operation(
                "doc-1",
                "client-1",
                OperationType::Insert { text: "Test".to_string() },
                0,
            )
            .expect("测试应成功");

        // 尝试再次应用相同的操作
        let result = engine
            .apply_remote_operation("doc-1", op)
            .expect("测试：apply_remote_operation 应成功");
        assert!(!result); // 应该返回 false 表示已存在
    }
}
