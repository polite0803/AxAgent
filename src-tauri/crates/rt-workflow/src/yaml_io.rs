// SPDX-License-Identifier: AGPL-3.0-only

//! YAML 工作流导入 / 导出
//!
//! 格式与 Workflow 的 JSON 结构等价，包含 version / metadata / nodes / edges。
//! 前端可调用 `export_workflow_yaml` / `import_workflow_yaml` 完成 YAML 交互。

use crate::workflow_engine::{Workflow, WorkflowStatus};
use axagent_harness::workflow_types::{WorkflowEdge, WorkflowNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── YAML 文件顶层结构 ──

pub const YAML_FORMAT_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowYamlFormat {
    /// 格式版本号（用于向前兼容）
    pub format_version: String,
    /// 工作流元信息
    pub metadata: WorkflowYamlMetadata,
    /// 节点列表
    pub nodes: Vec<WorkflowNode>,
    /// 边列表
    pub edges: Vec<WorkflowEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowYamlMetadata {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    /// 导出时间（导入时忽略）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exported_at: Option<u64>,
    /// 来源应用标识
    pub source: String,
    /// 自定义标签
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

// ── Error type ──

#[derive(Debug, thiserror::Error)]
pub enum YamlIoError {
    #[error("YAML serialization failed: {0}")]
    Serialize(String),
    #[error("YAML deserialization failed: {0}")]
    Deserialize(String),
    #[error("Unsupported format version: {0}")]
    UnsupportedVersion(String),
    #[error("Validation error: {0}")]
    Validation(String),
}

// ── Export ──

/// 将 Workflow 导出为 YAML 字符串。
pub fn export_workflow_yaml(workflow: &Workflow) -> Result<String, YamlIoError> {
    let yaml = WorkflowYamlFormat {
        format_version: YAML_FORMAT_VERSION.to_string(),
        metadata: WorkflowYamlMetadata {
            id: workflow.id.clone(),
            name: workflow.name.clone(),
            description: None,
            created_at: workflow.created_at,
            updated_at: current_epoch_ms(),
            exported_at: Some(current_epoch_ms()),
            source: "AxAgent".to_string(),
            tags: Vec::new(),
        },
        nodes: workflow.nodes.clone(),
        edges: workflow.edges.clone(),
    };

    serde_yaml::to_string(&yaml).map_err(|e| YamlIoError::Serialize(e.to_string()))
}

// ── Import ──

/// 从 YAML 字符串导入 Workflow。
///
/// 校验 format_version 是否支持；解析后返回 Workflow 容器和元信息。
pub fn import_workflow_yaml(
    yaml_str: &str,
) -> Result<(Workflow, WorkflowYamlMetadata), YamlIoError> {
    let yaml: WorkflowYamlFormat =
        serde_yaml::from_str(yaml_str).map_err(|e| YamlIoError::Deserialize(e.to_string()))?;

    // 版本校验
    let version = yaml.format_version.as_str();
    if version != "1.0" {
        return Err(YamlIoError::UnsupportedVersion(version.to_string()));
    }

    // 基本校验：至少有一个节点
    if yaml.nodes.is_empty() {
        return Err(YamlIoError::Validation("Workflow must have at least one node".to_string()));
    }

    let workflow = Workflow {
        id: yaml.metadata.id.clone(),
        name: yaml.metadata.name.clone(),
        nodes: yaml.nodes,
        edges: yaml.edges,
        status: WorkflowStatus::Created,
        created_at: yaml.metadata.created_at,
        completed_at: None,
        results: HashMap::new(),
        node_states: HashMap::new(),
        output: None,
        error_config: None,
        error_workflow_id: None,
    };

    Ok((workflow, yaml.metadata))
}

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::workflow_types::*;

    fn make_mock_workflow() -> Workflow {
        let node = WorkflowNode::Trigger(TriggerNode {
            base: WorkflowNodeBase {
                id: "n1".into(),
                title: "手动触发".into(),
                description: None,
                position: Position::default(),
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: TriggerConfig {
                trigger_type: TriggerType::Manual,
                config: serde_json::json!({}),
            },
        });
        let edge = WorkflowEdge {
            id: "e1".into(),
            source: "n1".into(),
            source_handle: None,
            target: "n2".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        };
        Workflow {
            id: "wf-test".into(),
            name: "测试工作流".into(),
            nodes: vec![node],
            edges: vec![edge],
            status: WorkflowStatus::Created,
            created_at: 1,
            completed_at: None,
            results: HashMap::new(),
            node_states: HashMap::new(),
            output: None,
            error_config: None,
            error_workflow_id: None,
        }
    }

    #[test]
    fn test_roundtrip_yaml() {
        let wf = make_mock_workflow();
        let yaml_str = export_workflow_yaml(&wf).expect("export");
        let (wf2, meta) = import_workflow_yaml(&yaml_str).expect("import");
        assert_eq!(wf2.id, "wf-test");
        assert_eq!(wf2.name, "测试工作流");
        assert_eq!(meta.source, "AxAgent");
        assert_eq!(wf2.nodes.len(), 1);
    }

    #[test]
    fn test_reject_unknown_version() {
        let yaml = r#"
format_version: "9.9"
metadata:
  id: "x"
  name: "x"
  created_at: 0
  updated_at: 0
  source: "test"
nodes: []
edges: []
"#;
        let err = import_workflow_yaml(yaml).unwrap_err();
        assert!(matches!(err, YamlIoError::UnsupportedVersion(_)));
    }

    #[test]
    fn test_reject_empty_nodes() {
        let yaml = r#"
format_version: "1.0"
metadata:
  id: "x"
  name: "x"
  created_at: 0
  updated_at: 0
  source: "test"
nodes: []
edges: []
"#;
        let err = import_workflow_yaml(yaml).unwrap_err();
        assert!(matches!(err, YamlIoError::Validation(_)));
    }
}
