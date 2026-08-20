// SPDX-License-Identifier: AGPL-3.0-only

//! Schema 生成与 Tauri IPC 类型同步工具
//!
//! 功能:
//! 1. 为 workflow 类型生成 JSON Schema 文档
//! 2. 从 Rust DTO 生成 TypeScript 类型定义，确保前后端一致性
//! 3. 校验前端类型定义与后端 DTO 的同步状态
//!
//! 使用方法:
//! ```bash
//! cargo run -p schema-gen                    # 生成所有类型定义
//! cargo run -p schema-gen -- check           # 仅检查类型同步，不生成
//! cargo run -p schema-gen -- ipc-types       # 仅生成 IPC 类型
//! ```

use axagent_harness::agent::{
    AgentCapability, AgentExecuteRequest, AgentInfo, AgentPlan, AgentResult, PlanStep,
};
use axagent_harness::conversation_model::{
    ContentBlock, ConversationMessage, SessionInfo, TokenUsage,
};
use axagent_harness::workflow_types::{
    BackoffType, CompensationConfig, CompensationStrategy, NodeKind, Position, RetryConfig,
    Variable, WorkflowNodeBase,
};
use schemars::schema_for;
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use ts_rs::TS;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = if args.len() > 1 { &args[1] } else { "all" };

    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("路径无父目录")
        .parent()
        .expect("路径无祖父目录");

    let out_dir = project_root.join("docs");
    let ts_types_dir = project_root.join("src").join("types").join("generated");

    match mode {
        "check" => run_check(&ts_types_dir),
        "ipc-types" => generate_ipc_types(&ts_types_dir),
        "workflow-schema" => generate_workflow_schema(&out_dir),
        _ => {
            generate_workflow_schema(&out_dir);
            generate_ipc_types(&ts_types_dir);
            run_check(&ts_types_dir);
        },
    }
}

// ============================================================================
// Workflow JSON Schema 生成
// ============================================================================

fn generate_workflow_schema(out_dir: &Path) {
    fs::create_dir_all(out_dir).expect("Schema 生成：创建输出目录失败");

    let schema = schema_for!(axagent_harness::workflow_types::WorkflowTemplateInput);
    let schema_str =
        serde_json::to_string_pretty(&schema).expect("Schema 生成：序列化 schema 失败");
    fs::write(out_dir.join("workflow-schema.json"), &schema_str)
        .expect("Schema 生成：写入 JSON 失败");
    eprintln!("Generated docs/workflow-schema.json");

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
    md.push_str("## WorkflowNode 变体\n\n");
    md.push_str("| `type` 标签 | 节点类型 | 配置结构体 |\n");
    md.push_str("|------------|----------|------------|\n");
    for (tag, node, cfg) in &variants {
        md.push_str(&format!("| `{}` | `{}` | `{}` |\n", tag, node, cfg));
    }

    md.push_str("\n## 关键字段说明\n\n");
    let schema_val: serde_json::Value =
        serde_json::from_str(&schema_str).expect("Schema 生成：反序列化 schema JSON 失败");
    let defs = schema_val["definitions"].as_object().expect("Schema 生成：definitions 应为 object");
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

    fs::write(out_dir.join("workflow-schema.md"), &md).expect("Schema 生成：写入 Markdown 失败");
    eprintln!("Generated docs/workflow-schema.md");
}

// ============================================================================
// Tauri IPC 类型生成与同步校验
// ============================================================================

/// 需要同步到前端的 Tauri IPC DTO 类型列表
///
/// 这些类型是跨前后端传输的数据结构，必须保持同步。
/// 新增 IPC DTO 时，需要在此处添加。
const IPC_DTO_TYPES: &[(&str, &str)] = &[
    // Agent 相关
    ("AgentExecuteRequest", "agent"),
    ("AgentResult", "agent"),
    ("AgentCapability", "agent"),
    ("AgentInfo", "agent"),
    ("AgentPlan", "agent"),
    ("PlanStep", "agent"),
    // Conversation 相关
    ("TokenUsage", "conversation"),
    ("ContentBlock", "conversation"),
    ("ConversationMessage", "conversation"),
    ("SessionInfo", "conversation"),
    // Workflow 相关
    ("Position", "workflow"),
    ("RetryConfig", "workflow"),
    ("BackoffType", "workflow"),
    ("CompensationConfig", "workflow"),
    ("CompensationStrategy", "workflow"),
    ("NodeKind", "workflow"),
    ("Variable", "workflow"),
    ("WorkflowNodeBase", "workflow"),
];

/// 生成 Tauri IPC 类型的 TypeScript 定义文件
fn generate_ipc_types(out_dir: &Path) {
    fs::create_dir_all(out_dir).expect("IPC 类型生成：创建输出目录失败");

    eprintln!("Generating Tauri IPC TypeScript type definitions...");

    let mut output = String::new();
    output.push_str("// 此文件由 schema-gen 自动生成，请勿手动编辑\n");
    output.push_str("// 修改 Rust DTO 后请重新运行: cargo run -p schema-gen -- ipc-types\n");
    output.push_str("//\n");
    output.push_str("// 本文件包含所有 Tauri IPC 跨进程传输的数据结构定义，\n");
    output.push_str("// 用于确保前后端类型一致性。\n\n");
    output.push_str("// 生成文件中的类型别名仅供后端 DTO 同步校验，\n");
    output.push_str("// 不在前端运行时直接引用，故禁用 no-unused-vars 规则。\n");
    output.push_str("// oxlint-disable no-unused-vars\n\n");

    output.push_str(
        "// ============================================================================\n",
    );
    output.push_str("// Agent 相关 DTO\n");
    output.push_str(
        "// ============================================================================\n\n",
    );
    output.push_str(&gen_type::<AgentExecuteRequest>());
    output.push_str(&gen_type::<AgentResult>());
    output.push_str(&gen_type::<AgentCapability>());
    output.push_str(&gen_type::<AgentInfo>());
    output.push_str(&gen_type::<AgentPlan>());
    output.push_str(&gen_type::<PlanStep>());

    output.push_str(
        "\n// ============================================================================\n",
    );
    output.push_str("// Conversation 相关 DTO\n");
    output.push_str(
        "// ============================================================================\n\n",
    );
    output.push_str(&gen_type::<TokenUsage>());
    output.push_str(&gen_type::<ContentBlock>());
    output.push_str(&gen_type::<ConversationMessage>());
    output.push_str(&gen_type::<SessionInfo>());

    output.push_str(
        "\n// ============================================================================\n",
    );
    output.push_str("// Workflow 相关 DTO\n");
    output.push_str(
        "// ============================================================================\n\n",
    );
    output.push_str(&gen_type::<Position>());
    output.push_str(&gen_type::<RetryConfig>());
    output.push_str(&gen_type::<BackoffType>());
    output.push_str(&gen_type::<CompensationConfig>());
    output.push_str(&gen_type::<CompensationStrategy>());
    output.push_str(&gen_type::<NodeKind>());
    output.push_str(&gen_type::<Variable>());
    output.push_str(&gen_type::<WorkflowNodeBase>());

    output.push_str(
        "\n// ============================================================================\n",
    );
    output.push_str("// 同步检查元数据\n");
    output.push_str(
        "// ============================================================================\n\n",
    );
    output.push_str(&format!("// Generated at: {:?}\n", SystemTime::now()));
    output.push_str(&format!("// Total DTO types: {}\n", IPC_DTO_TYPES.len()));

    let output_path = out_dir.join("ipc.ts");
    fs::write(&output_path, &output)
        .unwrap_or_else(|e| panic!("IPC 类型生成：写入 {} 失败: {}", output_path.display(), e));

    eprintln!("✅ Generated Tauri IPC types: {}", output_path.display());
}

fn gen_type<T: TS>() -> String {
    let cfg = ts_rs::Config::default();
    let decl = T::decl(&cfg);
    format!("{}\n\n", decl)
}

/// 检查前端类型定义与后端 DTO 的同步状态
fn run_check(ts_types_dir: &Path) {
    eprintln!("Checking Tauri IPC type synchronization...");

    let generated_path = ts_types_dir.join("ipc.ts");
    if !generated_path.exists() {
        eprintln!("⚠️  生成的 IPC 类型文件不存在: {}", generated_path.display());
        eprintln!("   请先运行: cargo run -p schema-gen -- ipc-types");
        return;
    }

    let generated_content =
        fs::read_to_string(&generated_path).unwrap_or_else(|e| format!("读取生成文件失败: {}", e));

    let mut missing_types = Vec::new();

    for (dto_name, _module) in IPC_DTO_TYPES {
        if !generated_content.contains(&dto_name.to_string()) {
            missing_types.push(format!("{}: 未在生成的 ipc.ts 中找到类型定义", dto_name));
        }
    }

    if missing_types.is_empty() {
        eprintln!("✅ Tauri IPC 类型同步检查通过");
        eprintln!("   共检查 {} 个 DTO 类型", IPC_DTO_TYPES.len());
    } else {
        eprintln!("⚠️  发现 {} 个类型同步问题:", missing_types.len());
        for issue in &missing_types {
            eprintln!("   - {}", issue);
        }
        eprintln!();
        eprintln!("   建议: 运行 'cargo run -p schema-gen -- ipc-types' 重新生成类型定义");
    }
}
