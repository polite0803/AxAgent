// SPDX-License-Identifier: AGPL-3.0-only

//! JSON Schema generation tool for workflow types.
//! Run with: `cargo run -p schema-gen`

use axagent_harness::workflow_types;
use schemars::schema_for;
use std::fs;
use std::path::Path;

fn main() {
    // Resolve output paths relative to the project root (2 levels up from src-tauri/schema-gen)
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("docs");
    fs::create_dir_all(&out_dir).unwrap();

    let schema = schema_for!(workflow_types::WorkflowTemplateInput);
    let schema_str = serde_json::to_string_pretty(&schema).unwrap();
    fs::write(out_dir.join("workflow-schema.json"), &schema_str).unwrap();
    eprintln!("Generated docs/workflow-schema.json");

    // Generate a Markdown summary
    let mut md = String::new();
    md.push_str("# 工作流 Schema 文档\n\n");
    md.push_str("> 自动生成自 `axagent-harness::workflow_types`。\n\n");

    md.push_str("## 核心类型\n\n");
    md.push_str("| 类型 | 说明 |\n");
    md.push_str("|------|------|\n");
    md.push_str("| `WorkflowNode` | 工作流节点（28 种变体的标签联合）|\n");
    md.push_str("| `WorkflowEdge` | 工作流边定义 |\n");
    md.push_str("| `WorkflowTemplateInput` | 工作流模板创建/更新入参 |\n");
    md.push_str("| `WorkflowTemplateResponse` | 工作流模板查询响应 |\n");
    md.push_str("| `TriggerConfig` | 触发配置（manual/schedule/webhook/event）|\n");
    md.push_str("| `RetryConfig` | 节点重试策略 |\n");
    md.push_str("| `ErrorConfig` | 节点错误处理配置 |\n\n");

    md.push_str("## WorkflowNode 变体\n\n");
    md.push_str("| `type` 标签 | 节点类型 | 配置结构体 |\n");
    md.push_str("|------------|----------|------------|\n");
    let variants = [
        ("trigger", "TriggerNode", "TriggerConfig"),
        ("agent", "AgentNode", "AgentNodeConfig"),
        ("llm", "LLMNode", "LLMNodeConfig"),
        ("condition", "ConditionNode", "ConditionNodeConfig"),
        ("parallel", "ParallelNode", "ParallelNodeConfig"),
        ("loop", "LoopNode", "LoopNodeConfig"),
        ("merge", "MergeNode", "MergeNodeConfig"),
        ("delay", "DelayNode", "DelayNodeConfig"),
        ("tool", "ToolNode", "ToolNodeConfig"),
        ("code", "CodeNode", "CodeNodeConfig"),
        ("subWorkflow", "SubWorkflowNode", "SubWorkflowNodeConfig"),
        ("workflowRef", "WorkflowRefNode", "WorkflowRefNodeConfig"),
        ("end", "EndNode", "EndNodeConfig"),
        ("switch", "SwitchNode", "SwitchNodeConfig"),
        ("httpRequest", "HttpRequestNode", "HttpRequestNodeConfig"),
        ("databaseQuery", "DatabaseQueryNode", "DatabaseQueryNodeConfig"),
        ("notification", "NotificationNode", "NotificationNodeConfig"),
        ("approval", "ApprovalNode", "ApprovalNodeConfig"),
        ("fileOperation", "FileOperationNode", "FileOperationNodeConfig"),
        ("dataTransformer", "DataTransformerNode", "DataTransformerNodeConfig"),
        ("webhookSend", "WebhookSendNode", "WebhookSendNodeConfig"),
        ("logging", "LoggingNode", "LoggingNodeConfig"),
        ("llmClassifier", "LlmClassifierNode", "LlmClassifierNodeConfig"),
        ("aggregator", "AggregatorNode", "AggregatorNodeConfig"),
        ("email", "EmailNode", "EmailNodeConfig"),
        ("debate", "DebateNode", "DebateNodeConfig"),
        ("validation", "ValidationNode", "ValidationNodeConfig"),
        ("documentParser", "DocumentParserNode", "DocumentParserNodeConfig"),
        ("vectorRetrieve", "VectorRetrieveNode", "VectorRetrieveNodeConfig"),
    ];
    for (tag, node, cfg) in &variants {
        md.push_str(&format!("| `{}` | `{}` | `{}` |\n", tag, node, cfg));
    }

    md.push_str("\n## 关键字段说明\n\n");

    // Read the raw schema JSON and generate field docs
    let schema_val: serde_json::Value = serde_json::from_str(&schema_str).unwrap();
    let defs = schema_val["definitions"].as_object().unwrap();
    for (name, def) in defs {
        let title = def["title"].as_str().unwrap_or(name);
        let desc = def["description"].as_str().unwrap_or("");
        if !desc.is_empty() || title.contains("Config") || title.contains("Node") {
            md.push_str(&format!("### `{}`\n\n", title));
            if !desc.is_empty() {
                md.push_str(&format!("{}\n\n", desc));
            }
            if let Some(props) = def["properties"].as_object() {
                md.push_str("| 字段 | 类型 | 必填 | 说明 |\n");
                md.push_str("|------|------|------|------|\n");
                let required = def["required"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                    .unwrap_or_default();
                for (fname, fdef) in props {
                    let ftype = fdef["type"].as_str().unwrap_or("object");
                    let fdesc = fdef["description"].as_str().unwrap_or("");
                    let is_req = required.contains(&fname.as_str());
                    md.push_str(&format!(
                        "| `{}` | `{}` | {} | {} |\n",
                        fname,
                        ftype,
                        if is_req { "✅" } else { "❌" },
                        fdesc
                    ));
                }
            }
            md.push('\n');
        }
    }

    md.push_str("## EdgeType 枚举\n\n");
    md.push_str("| 值 | 说明 |\n");
    md.push_str("|----|------|\n");
    md.push_str("| `direct` | 直接连线（默认） |\n");
    md.push_str("| `conditionTrue` | 条件节点 true 分支 |\n");
    md.push_str("| `conditionFalse` | 条件节点 false 分支 |\n");
    md.push_str("| `loopBack` | 循环回边 |\n");
    md.push_str("| `parallelBranch` | 并行分支边 |\n");
    md.push_str("| `merge` | 合并边 |\n");
    md.push_str("| `debateRound` | 辩论轮次边 |\n");
    md.push_str("| `error` | 错误处理边 |\n");
    md.push_str("| `grouping` | 装饰分组边（不参与校验和布局） |\n\n");

    md.push_str("## 嵌套限制\n\n");
    md.push_str("- WorkflowRef 嵌套深度：≤ 3 层\n");
    md.push_str("- 循环引用检测：执行时回溯调用栈\n\n");

    md.push_str("## 向后兼容\n\n");
    md.push_str("- 所有 Optional 字段均有 `#[serde(default)]`，向前兼容\n");
    md.push_str("- `kind` 字段默认 `\"executable\"`，旧数据自动兼容\n");
    md.push_str("- `edge_type` 新增 `\"grouping\"`，旧解析器忽略未知值\n\n");

    md.push_str("---\n");
    md.push_str("*文档自动生成，更新类型后请重新运行 `cargo run -p schema-gen`*\n");

    fs::write(out_dir.join("workflow-schema.md"), &md).unwrap();
    eprintln!("Generated docs/workflow-schema.md");
}
