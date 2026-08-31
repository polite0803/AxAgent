//! 能力组装器：将 `CapabilityPassportDto` 映射为工作流节点/边的桥接层。
//!
//! ## 设计原则
//!
//! 本模块属于 **foundation 层**，零 axagent-* crate 依赖，仅做纯 DTO 转换。
//! Toolchain 多步展开需要 CapabilityIndexer resolver，由上层（SaveAsWorkflow）在
//! hybrid 层展开后再喂给本模块。Skill 直接映射为 AgentNode（保留 LLM prompt 推理能力）。
//!
//! 核心职责：
//! - 单个 `CapabilityPassportDto` → `Option<WorkflowNode>`
//! - 一组节点（线性） → `Vec<WorkflowEdge>`（顺序串接）
//! - `CapabilityKind` → `WorkflowNode` 变体的映射规则定义
//!
//! ## 映射规则
//!
//! | CapabilityKind  | WorkflowNode 变体   | 关键字段来源                                                              |
//! |-----------------|---------------------|---------------------------------------------------------------------------|
//! | Tool            | Tool                | `tool_ref.tool_name` → config                                             |
//! | Workflow        | WorkflowRef         | `capability_id` → target_workflow_id                                      |
//! | KnowledgeBase   | VectorRetrieve      | `capability_id` → kb_id                                                   |
//! | Agent           | Agent               | `agent_profile_id` → agent_id                                             |
//! | Skill           | Agent               | `description` → system_prompt；`skill_steps` → tools/context_sources       |
//! | Toolchain       | 跳过（None）        | 需要 resolver 展开 steps，由上层先展开再组装                              |
//! | Template        | 跳过（None）        | 占位符能力，不生成节点                                                    |
//!
//! ## Skill → AgentNode（方式 B：保留 LLM prompt 推理能力）
//!
//! Skill 的 SKILL.md 正文 prompt 本质上就是给 LLM 的"多步任务指令"，
//! 与 WorkflowNode::Agent 的 ReAct 执行模式（LLM + function-call 循环）天然对齐。
//! 我们把 Skill 映射为 AgentNode，保留推理能力：
//!
//! - `system_prompt` = `passport.description`（SKILL.md frontmatter 描述 + 正文摘要）
//! - `tools` = 从 `skill_steps` 收集的 `ToolDef` 列表（name=capability_id）
//! - `context_sources` = `skill_steps` 中 KnowledgeBase 类型的 capability_id
//! - `rag_source_ids` = `context_sources`（KB 直接挂到 RAG 源上）
//!
//! 限制（foundation 层拿不到 resolver）：
//! - ToolDef.description 为空（无法查子能力的具体描述），运行时由 Agent executor 自行补全
//! - context_sources 仅根据 capability_id 前缀启发式判断（"kb:" 开头 → KB）
//!
//! Toolchain 不做这种映射——它没有 prompt（就是固定顺序工具串），
//! 继续在 hybrid 层做展开 → ToolNode 序列（方式 A）。

use serde::{Deserialize, Serialize};

use crate::capability::{CapabilityKind, CapabilityPassportDto};
use crate::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, Position, ToolNode,
    ToolNodeConfig, VectorRetrieveNode, VectorRetrieveNodeConfig, WorkflowEdge, WorkflowNode,
    WorkflowNodeBase, WorkflowRefNode, WorkflowRefNodeConfig,
};

/// 节点在组装阶段的相对顺序（用于线性布局 y 坐标计算）。
const NODE_SPACING_Y: f64 = 120.0;

/// 组装结果：一组节点 + 连接它们的边。
///
/// 由上层（SaveAsWorkflow Tool 等）消费，写入 WorkflowTemplate.nodes / edges。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssemblyResult {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

/// 组装器 trait——可替换实现（如未来需要 LLM 辅助组装、或基于执行历史的智能组装）。
pub trait AssemblyBuilder {
    /// 单个能力护照 → 工作流节点。
    ///
    /// 返回 `None` 表示该能力不直接生成节点（如 Template 占位符、
    /// Toolchain/Skill 的多步类型由上层展开）。
    fn build_node(&self, passport: &CapabilityPassportDto) -> Option<WorkflowNode>;

    /// 把一组节点按顺序串接为线性 DAG（source → target, EdgeType::Direct）。
    ///
    /// 同时在末尾追加一个 EndNode（如节点列表非空）。
    fn assemble_linear(&self, passports: &[CapabilityPassportDto]) -> AssemblyResult;
}

// ── 默认实现 ──────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
pub struct DefaultAssemblyBuilder {
    /// 节点 ID 前缀（避免与调用方已有节点冲突）。
    pub id_prefix: String,
}

impl DefaultAssemblyBuilder {
    pub fn new() -> Self {
        Self { id_prefix: "cap".to_string() }
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.id_prefix = prefix.into();
        self
    }

    fn make_id(&self, index: usize, passport_id: &str) -> String {
        format!("{}_{}_{}", self.id_prefix, index, passport_id)
    }

    fn base(&self, id: String, title: String, index: usize) -> WorkflowNodeBase {
        WorkflowNodeBase {
            id,
            title,
            description: None,
            position: Position { x: 200.0, y: index as f64 * NODE_SPACING_Y },
            retry: crate::workflow_types::RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        }
    }

    fn end_node(&self, id: String, index: usize) -> WorkflowNode {
        WorkflowNode::End(EndNode {
            base: self.base(id, "End".to_string(), index),
            config: EndNodeConfig { output_var: Some("result".to_string()) },
        })
    }
}

impl AssemblyBuilder for DefaultAssemblyBuilder {
    fn build_node(&self, passport: &CapabilityPassportDto) -> Option<WorkflowNode> {
        let kind = passport.kind;

        match kind {
            CapabilityKind::Tool => {
                let tool_name = passport
                    .tool_ref
                    .as_ref()
                    .map(|r| r.tool_name.clone())
                    .unwrap_or_else(|| passport.name.clone());

                let node_id = self.make_id(0, &passport.capability_id);
                let base = self.base(node_id, passport.name.clone(), 0);

                Some(WorkflowNode::Tool(ToolNode {
                    base,
                    config: ToolNodeConfig {
                        tool_name,
                        input_mapping: std::collections::HashMap::new(),
                        output_var: format!("out_{}", passport.capability_id),
                    },
                }))
            },

            CapabilityKind::Workflow => {
                let node_id = self.make_id(0, &passport.capability_id);
                let base = self.base(node_id, passport.name.clone(), 0);

                Some(WorkflowNode::WorkflowRef(WorkflowRefNode {
                    base,
                    config: WorkflowRefNodeConfig {
                        target_workflow_id: passport.capability_id.clone(),
                        input_mapping: std::collections::HashMap::new(),
                        output_var: format!("out_{}", passport.capability_id),
                        timeout: passport.timeout_ms.map(|t| t as i64),
                        context_mode: "inherit".to_string(),
                    },
                }))
            },

            CapabilityKind::KnowledgeBase => {
                let node_id = self.make_id(0, &passport.capability_id);
                let base = self.base(node_id, passport.name.clone(), 0);

                Some(WorkflowNode::VectorRetrieve(VectorRetrieveNode {
                    base,
                    config: VectorRetrieveNodeConfig {
                        // 查询语需要由上层在组装时填充（用户意图 / 任务描述），
                        // 这里用占位符，调用方负责在落库前修正。
                        query: "{{user_query}}".to_string(),
                        knowledge_base_id: passport.capability_id.clone(),
                        top_k: 5,
                        similarity_threshold: None,
                        output_var: format!("kb_{}", passport.capability_id),
                    },
                }))
            },

            CapabilityKind::Agent => {
                let node_id = self.make_id(0, &passport.capability_id);
                let base = self.base(node_id, passport.name.clone(), 0);

                Some(WorkflowNode::Agent(AgentNode {
                    base,
                    config: AgentNodeConfig {
                        system_prompt: passport.description.clone(),
                        context_sources: vec![],
                        input_mapping: std::collections::HashMap::new(),
                        output_var: format!("out_{}", passport.capability_id),
                        model: None,
                        temperature: None,
                        max_tokens: None,
                        tools: vec![],
                        exposed_tools: vec![],
                        output_mode: crate::workflow_types::OutputMode::Text,
                        agent_profile_id: passport.agent_profile_id.clone(),
                        max_tool_rounds: None,
                        execution_mode: None,
                        rag_source_ids: vec![],
                        model_role: None,
                        consistency_check: None,
                        hallucination_guard: None,
                        fallback_model: None,
                        task_scene: None,
                        stream_chunk_timeout_secs: None,
                    },
                }))
            },

            // Skill → AgentNode（方式 B：保留 LLM prompt 推理能力）。
            // 优先用 prompt_body（SKILL.md 完整正文）作 system_prompt，
            // 无则 fallback 到 description（frontmatter 摘要）。
            // skill_steps 里的子能力收集为 tools/context_sources，
            // 运行时由 Agent executor 驱动 ReAct 循环。
            CapabilityKind::Skill => {
                let node_id = self.make_id(0, &passport.capability_id);
                let base = self.base(node_id, passport.name.clone(), 0);

                let (tool_defs, rag_sources) = collect_skill_step_refs(passport);
                let system_prompt =
                    passport.prompt_body.clone().unwrap_or_else(|| passport.description.clone());

                Some(WorkflowNode::Agent(AgentNode {
                    base,
                    config: AgentNodeConfig {
                        system_prompt,
                        context_sources: rag_sources.clone(),
                        input_mapping: std::collections::HashMap::new(),
                        output_var: format!("out_{}", passport.capability_id),
                        model: None,
                        temperature: None,
                        max_tokens: None,
                        tools: tool_defs,
                        exposed_tools: vec![],
                        output_mode: crate::workflow_types::OutputMode::Text,
                        agent_profile_id: None,
                        max_tool_rounds: None,
                        execution_mode: None,
                        rag_source_ids: rag_sources,
                        model_role: None,
                        consistency_check: None,
                        hallucination_guard: None,
                        fallback_model: None,
                        task_scene: None,
                        stream_chunk_timeout_secs: None,
                    },
                }))
            },

            // Toolchain 没有 prompt（固定顺序工具串），在 hybrid 层由上层先展开
            // steps → ToolNode 序列（方式 A），本层只做纯 DTO 转换拿不到 resolver。
            CapabilityKind::Toolchain => None,

            // Template 是占位符能力，不直接生成节点。
            CapabilityKind::Template => None,
        }
    }

    fn assemble_linear(&self, passports: &[CapabilityPassportDto]) -> AssemblyResult {
        let mut nodes: Vec<WorkflowNode> = Vec::new();
        let mut edges: Vec<WorkflowEdge> = Vec::new();

        // 第一遍：为每个 passport 尝试生成节点
        for (i, p) in passports.iter().enumerate() {
            let mut node = self.build_node(p);

            // Toolchain：展开 steps（如果上层已把 steps 对应护照传进来了，
            // 但我们这里只有单个 Toolchain 护照，没 resolver，跳过）。
            //
            // Skill：展开 skill_steps（同理，无 resolver 跳过）。
            //
            // 这两种类型由上层显式展开后，把每步对应的 passport 塞进来，
            // 我们按普通 Tool 节点处理。

            if let Some(n) = node.as_mut() {
                // 修正节点的 index/position（build_node 内部写死 index=0）
                *n = adjust_position(n.clone(), i);

                // 修复 ID：build_node 内部用 index=0，这里按真实序号重算
                let new_id = self.make_id(i, &p.capability_id);
                n.set_base_id(new_id);
            }

            if let Some(n) = node {
                nodes.push(n);
            }
        }

        // 第二遍：线性串接 edges（source[i] → target[i+1]）
        for i in 0..nodes.len().saturating_sub(1) {
            let source_id = nodes[i].base_id().to_string();
            let target_id = nodes[i + 1].base_id().to_string();
            edges.push(WorkflowEdge {
                id: format!("edge_{}_{}", self.id_prefix, i),
                source: source_id,
                source_handle: Some("out".to_string()),
                target: target_id,
                target_handle: Some("in".to_string()),
                edge_type: EdgeType::Direct,
                label: None,
            });
        }

        // 第三遍：追加 EndNode
        if !nodes.is_empty() {
            let end_index = nodes.len();
            let end_id = format!("{}_end", self.id_prefix);
            let end = self.end_node(end_id.clone(), end_index);
            let prev_id = nodes.last().unwrap().base_id().to_string();
            nodes.push(end);
            edges.push(WorkflowEdge {
                id: format!("edge_{}_final", self.id_prefix),
                source: prev_id,
                source_handle: Some("out".to_string()),
                target: end_id,
                target_handle: Some("in".to_string()),
                edge_type: EdgeType::Direct,
                label: None,
            });
        }

        AssemblyResult { nodes, edges }
    }
}

// ── 辅助函数 ──────────────────────────────────────────────────────────

/// 从 Skill 的 `skill_steps` 中收集子能力引用。
///
/// 返回 `(ToolDef 列表, RAG source IDs)`：
/// - ToolDef：capability_id 不以 "kb:" 开头的都作为 AgentNode 可调用工具
/// - RAG source IDs：capability_id 以 "kb:" 开头的作为知识库 RAG 源
///
/// 注：harness foundation 层拿不到 CapabilityIndexer，无法精确查询子能力类型，
/// 只能按 capability_id 前缀启发式判断。运行时 Agent executor 会补全 ToolDef.description。
fn collect_skill_step_refs(
    passport: &CapabilityPassportDto,
) -> (Vec<crate::workflow_types::ToolDef>, Vec<String>) {
    use crate::workflow_types::ToolDef;

    let mut tool_defs: Vec<ToolDef> = Vec::new();
    let mut rag_sources: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for step in &passport.skill_steps {
        let cid = &step.capability_id;
        if !seen.insert(cid.clone()) {
            continue;
        }

        if cid.starts_with("kb:") || cid.starts_with("knowledge_base:") || cid.starts_with("kb_") {
            // 知识库 → RAG source（Agent executor 的 VectorRetrieve context source）
            rag_sources.push(cid.clone());
        } else {
            // 其他（tool / agent / workflow / skill / toolchain 等）→ Agent 可调用工具
            // 先去掉前缀（如 "tool:send_email" → "send_email"），保留完整 capability_id 作为 name
            // Agent executor 运行时会根据 name 在全局工具注册表查找实际定义
            tool_defs.push(ToolDef { name: cid.clone(), description: None, parameters: None });
        }
    }

    // 如果没有 skill_steps，fallback 到 Toolchain 风格的 steps 列表
    if tool_defs.is_empty() && rag_sources.is_empty() && !passport.steps.is_empty() {
        for step_id in &passport.steps {
            if !seen.insert(step_id.clone()) {
                continue;
            }
            if step_id.starts_with("kb:") || step_id.starts_with("knowledge_base:") {
                rag_sources.push(step_id.clone());
            } else {
                tool_defs.push(ToolDef {
                    name: step_id.clone(),
                    description: None,
                    parameters: None,
                });
            }
        }
    }

    (tool_defs, rag_sources)
}

fn adjust_position(mut node: WorkflowNode, index: usize) -> WorkflowNode {
    let y = index as f64 * NODE_SPACING_Y;
    let base_mut = match &mut node {
        WorkflowNode::Trigger(n) => &mut n.base,
        WorkflowNode::Agent(n) => &mut n.base,
        WorkflowNode::Llm(n) => &mut n.base,
        WorkflowNode::Condition(n) => &mut n.base,
        WorkflowNode::Parallel(n) => &mut n.base,
        WorkflowNode::Loop(n) => &mut n.base,
        WorkflowNode::Merge(n) => &mut n.base,
        WorkflowNode::Delay(n) => &mut n.base,
        WorkflowNode::Validation(n) => &mut n.base,
        WorkflowNode::SubWorkflow(n) => &mut n.base,
        WorkflowNode::WorkflowRef(n) => &mut n.base,
        WorkflowNode::DocumentParser(n) => &mut n.base,
        WorkflowNode::VectorRetrieve(n) => &mut n.base,
        WorkflowNode::End(n) => &mut n.base,
        WorkflowNode::HttpRequest(n) => &mut n.base,
        WorkflowNode::Switch(n) => &mut n.base,
        WorkflowNode::DatabaseQuery(n) => &mut n.base,
        WorkflowNode::Notification(n) => &mut n.base,
        WorkflowNode::Approval(n) => &mut n.base,
        WorkflowNode::FileOperation(n) => &mut n.base,
        WorkflowNode::DataTransformer(n) => &mut n.base,
        WorkflowNode::WebhookSend(n) => &mut n.base,
        WorkflowNode::Logging(n) => &mut n.base,
        WorkflowNode::LlmClassifier(n) => &mut n.base,
        WorkflowNode::Aggregator(n) => &mut n.base,
        WorkflowNode::Email(n) => &mut n.base,
        WorkflowNode::Debate(n) => &mut n.base,
        WorkflowNode::Swarm(n) => &mut n.base,
        WorkflowNode::MultiAgent(n) => &mut n.base,
        WorkflowNode::Storage(n) => &mut n.base,
        WorkflowNode::Tool(n) => &mut n.base,
        WorkflowNode::Code(n) => &mut n.base,
    };
    base_mut.position = Position { x: 200.0, y };
    node
}

impl WorkflowNode {
    pub fn set_base_id(&mut self, new_id: String) {
        let base_mut = match self {
            WorkflowNode::Trigger(n) => &mut n.base,
            WorkflowNode::Agent(n) => &mut n.base,
            WorkflowNode::Llm(n) => &mut n.base,
            WorkflowNode::Condition(n) => &mut n.base,
            WorkflowNode::Parallel(n) => &mut n.base,
            WorkflowNode::Loop(n) => &mut n.base,
            WorkflowNode::Merge(n) => &mut n.base,
            WorkflowNode::Delay(n) => &mut n.base,
            WorkflowNode::Validation(n) => &mut n.base,
            WorkflowNode::SubWorkflow(n) => &mut n.base,
            WorkflowNode::WorkflowRef(n) => &mut n.base,
            WorkflowNode::DocumentParser(n) => &mut n.base,
            WorkflowNode::VectorRetrieve(n) => &mut n.base,
            WorkflowNode::End(n) => &mut n.base,
            WorkflowNode::HttpRequest(n) => &mut n.base,
            WorkflowNode::Switch(n) => &mut n.base,
            WorkflowNode::DatabaseQuery(n) => &mut n.base,
            WorkflowNode::Notification(n) => &mut n.base,
            WorkflowNode::Approval(n) => &mut n.base,
            WorkflowNode::FileOperation(n) => &mut n.base,
            WorkflowNode::DataTransformer(n) => &mut n.base,
            WorkflowNode::WebhookSend(n) => &mut n.base,
            WorkflowNode::Logging(n) => &mut n.base,
            WorkflowNode::LlmClassifier(n) => &mut n.base,
            WorkflowNode::Aggregator(n) => &mut n.base,
            WorkflowNode::Email(n) => &mut n.base,
            WorkflowNode::Debate(n) => &mut n.base,
            WorkflowNode::Swarm(n) => &mut n.base,
            WorkflowNode::MultiAgent(n) => &mut n.base,
            WorkflowNode::Storage(n) => &mut n.base,
            WorkflowNode::Tool(n) => &mut n.base,
            WorkflowNode::Code(n) => &mut n.base,
        };
        base_mut.id = new_id;
    }
}

// ── 单元测试 ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{CapabilityDomain, CapabilityToolRef, PlanningComplexity};

    fn make_passport(kind: CapabilityKind, id: &str, name: &str) -> CapabilityPassportDto {
        CapabilityPassportDto {
            capability_id: id.to_string(),
            name: name.to_string(),
            description: format!("{} desc", name),
            summary: None,
            version: None,
            owner: None,
            created_at: None,
            updated_at: None,
            kind,
            domain: CapabilityDomain::General,
            sub_category: String::new(),
            visibility: crate::capability::Visibility::default(),
            caller_permissions: crate::capability::CallerPermissions::default(),
            input_schema: None,
            output_schema: None,
            implementation: None,
            tags: vec![],
            negative_scenarios: vec![],
            security_level: crate::capability::SecurityLevel::Public,
            modality_support: crate::capability::ModalitySupport::default(),
            output_capabilities: crate::capability::OutputCapabilities::default(),
            estimated_cost_usd: None,
            avg_duration_seconds: None,
            execution_mode: crate::capability::ExecutionMode::default(),
            timeout_ms: None,
            planning_complexity: PlanningComplexity::Simple,
            model_iq_requirement: 0,
            experiment_group: None,
            agent_profile_id: None,
            level: crate::capability::CapabilityLevel::default(),
            stats: crate::capability::CapabilityStats::default(),
            enabled: true,
            source: crate::capability::CapabilitySource::default(),
            evolvable: crate::capability::CapabilityEvolvability::default(),
            exposure: crate::capability::CapabilityExposure::default(),
            tool_ref: None,
            aliases: vec![],
            steps: vec![],
            skill_steps: vec![],
            placeholders: vec![],
            prompt_body: None,
            template_body: None,
            instantiates_to: None,
            example_instance: None,
            upstream: vec![],
            downstream: vec![],
            preconditions: vec![],
            attached_snippets: vec![],
        }
    }

    #[test]
    fn tool_capability_maps_to_tool_node() {
        let mut p = make_passport(CapabilityKind::Tool, "tool_send_email", "Send Email");
        p.tool_ref = Some(CapabilityToolRef {
            tool_name: "send_email".to_string(),
            registry: String::new(),
        });

        let b = DefaultAssemblyBuilder::new();
        let node = b.build_node(&p).unwrap();

        match node {
            WorkflowNode::Tool(t) => {
                assert_eq!(t.config.tool_name, "send_email");
                assert_eq!(t.base.title, "Send Email");
            },
            other => panic!("expected ToolNode, got {:?}", other),
        }
    }

    #[test]
    fn knowledge_base_capability_maps_to_vector_retrieve() {
        let p = make_passport(CapabilityKind::KnowledgeBase, "kb_company", "Company KB");
        let b = DefaultAssemblyBuilder::new();
        let node = b.build_node(&p).unwrap();

        match node {
            WorkflowNode::VectorRetrieve(v) => {
                assert_eq!(v.config.knowledge_base_id, "kb_company");
            },
            other => panic!("expected VectorRetrieveNode, got {:?}", other),
        }
    }

    #[test]
    fn template_capability_returns_none() {
        let p = make_passport(CapabilityKind::Template, "tpl_ip_scan", "IP Scan Tpl");
        let b = DefaultAssemblyBuilder::new();
        assert!(b.build_node(&p).is_none());
    }

    #[test]
    fn assemble_linear_produces_chain_with_end_node() {
        let mut p1 = make_passport(CapabilityKind::Tool, "t1", "Step1");
        p1.tool_ref =
            Some(CapabilityToolRef { tool_name: "tool_a".to_string(), ..Default::default() });
        let mut p2 = make_passport(CapabilityKind::Tool, "t2", "Step2");
        p2.tool_ref =
            Some(CapabilityToolRef { tool_name: "tool_b".to_string(), ..Default::default() });

        let b = DefaultAssemblyBuilder::new().with_prefix("test");
        let result = b.assemble_linear(&[p1, p2]);

        // 3 个节点：Tool + Tool + End
        assert_eq!(result.nodes.len(), 3);
        // 2 条边：Tool1→Tool2, Tool2→End
        assert_eq!(result.edges.len(), 2);

        // 验证最后一个是 EndNode
        assert!(matches!(result.nodes.last(), Some(WorkflowNode::End(_))));
    }

    #[test]
    fn assemble_linear_skips_toolchain_but_not_skill() {
        let tc = make_passport(CapabilityKind::Toolchain, "tc1", "My Chain");
        let mut sk = make_passport(CapabilityKind::Skill, "sk1", "My Skill");
        // Skill 有 skill_steps，应该生成 AgentNode
        sk.skill_steps = vec![
            crate::capability::SkillStep {
                step_id: "s1".to_string(),
                capability_id: "tool:scan".to_string(),
                ..Default::default()
            },
            crate::capability::SkillStep {
                step_id: "s2".to_string(),
                capability_id: "kb:company_kb".to_string(),
                ..Default::default()
            },
        ];

        let b = DefaultAssemblyBuilder::new();
        let result = b.assemble_linear(&[tc, sk]);

        // Toolchain 跳过，Skill 生成 AgentNode + End → 共 2 个节点
        assert_eq!(result.nodes.len(), 2);
        assert!(matches!(result.nodes[0], WorkflowNode::Agent(_)));

        // 验证 AgentNode 内部结构
        if let WorkflowNode::Agent(an) = &result.nodes[0] {
            assert_eq!(an.config.system_prompt, "My Skill desc");
            // tools 应包含 tool:scan
            assert!(an.config.tools.iter().any(|t| t.name == "tool:scan"));
            // rag_source_ids 应包含 kb:company_kb
            assert!(an.config.rag_source_ids.iter().any(|r| r == "kb:company_kb"));
            assert_eq!(an.config.context_sources, an.config.rag_source_ids);
        }
        // End 节点
        assert!(matches!(result.nodes.last(), Some(WorkflowNode::End(_))));
    }

    #[test]
    fn skill_without_steps_still_generates_agent_node() {
        let sk = make_passport(CapabilityKind::Skill, "sk_empty", "Empty Skill");
        let b = DefaultAssemblyBuilder::new();
        let node = b.build_node(&sk).unwrap();

        match node {
            WorkflowNode::Agent(an) => {
                // 没有 skill_steps → tools/rag_source_ids 都为空
                assert!(an.config.tools.is_empty());
                assert!(an.config.rag_source_ids.is_empty());
                assert_eq!(an.config.system_prompt, "Empty Skill desc");
            },
            other => panic!("expected AgentNode for Skill, got {:?}", other),
        }
    }
}
