// OPC 领域（Domain）模块 — 通用领域工作流定义与生成
//
// 领域与行业的区别：
// - 行业（industry）：面向特定行业（如金融投资、电商），提供差异化的适配器实现
// - 领域（domain）：面向通用业务场景（如内容创作、客户管理），以纯数据驱动工作流
//
// 领域工作流由 DomainWorkflowDef 描述，通过 DomainWorkflowGenerator 转换为
// WorkflowTemplateData，交由工作流引擎执行。

use std::collections::HashMap;

use axagent_harness::util_fns::now_ts;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, ApprovalNode, ApprovalNodeConfig, ConditionNode,
    ConditionNodeConfig, EdgeType, EndNode, EndNodeConfig, ErrorConfig, LogicalOperator,
    OnFailureAction, OutputMode, Position, RetryConfig, TriggerConfig, TriggerNode, Variable,
    WorkflowEdge, WorkflowNode, WorkflowNodeBase, WorkflowTemplateData,
};
use serde::{Deserialize, Serialize};

pub mod prompt_tpl;

// ── 领域工作流步骤定义 ──────────────────────────────────────────

/// 领域工作流步骤定义
///
/// 每个步骤对应工作流中的一个节点（agent 或 approval），
/// 包含提示词、工具白名单、输入映射等配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStepDef {
    /// 步骤唯一 ID
    pub id: String,
    /// 步骤标题（展示用）
    pub title: String,
    /// Agent 系统提示词（agent 节点必填）
    #[serde(default)]
    pub prompt: String,
    /// 上游输入映射：{ 输入变量名: 上游节点输出路径 }
    #[serde(default)]
    pub inputs: HashMap<String, String>,
    /// 工具白名单：节点可调用的工具名
    #[serde(default)]
    pub tools: Vec<String>,
    /// 节点类型：agent（默认）| approval（人工审批）
    #[serde(default = "default_node_type")]
    pub node_type: String,
    /// 步骤失败时的降级说明
    #[serde(default)]
    pub on_error: Option<String>,
    /// 上游失败时是否容错继续
    #[serde(default)]
    pub continue_on_fail: Option<bool>,
    /// approval 节点配置
    #[serde(default)]
    pub approval: Option<DomainApproval>,
    /// 条件执行表达式（Rhai 语法），满足条件时才执行此步骤
    #[serde(default)]
    pub condition: Option<String>,
    /// Agent 角色配置（指定执行此步骤的 Agent ID 和角色）
    #[serde(default)]
    pub agent: Option<DomainAgentDef>,
    /// 用户输入/审批表单配置（在执行前暂停等待用户输入）
    #[serde(default)]
    pub user_input: Option<DomainUserInput>,
}

fn default_node_type() -> String {
    "agent".into()
}

impl DomainStepDef {
    /// 创建 agent 类型的步骤
    pub fn agent(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            prompt: String::new(),
            inputs: HashMap::new(),
            tools: Vec::new(),
            node_type: "agent".to_string(),
            on_error: None,
            continue_on_fail: None,
            approval: None,
            condition: None,
            agent: None,
            user_input: None,
        }
    }

    /// 创建 approval 类型的步骤
    pub fn approval(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            prompt: String::new(),
            inputs: HashMap::new(),
            tools: Vec::new(),
            node_type: "approval".to_string(),
            on_error: None,
            continue_on_fail: None,
            approval: Some(DomainApproval::default()),
            condition: None,
            agent: None,
            user_input: None,
        }
    }

    /// 设置提示词
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// 添加工具
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    /// 设置输入映射
    pub fn with_inputs(mut self, inputs: HashMap<String, String>) -> Self {
        self.inputs = inputs;
        self
    }

    /// 设置容错继续
    pub fn with_continue_on_fail(mut self, val: bool) -> Self {
        self.continue_on_fail = Some(val);
        self
    }

    /// 设置失败降级说明
    pub fn with_on_error(mut self, msg: impl Into<String>) -> Self {
        self.on_error = Some(msg.into());
        self
    }

    /// 设置条件执行表达式
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }

    /// 设置 Agent 角色配置
    pub fn with_agent(mut self, agent: DomainAgentDef) -> Self {
        self.agent = Some(agent);
        self
    }

    /// 设置用户输入/审批表单
    pub fn with_user_input(mut self, user_input: DomainUserInput) -> Self {
        self.user_input = Some(user_input);
        self
    }
}

// ── Agent 角色配置 ───────────────────────────────────────────────

/// Agent 角色配置
///
/// 指定步骤由哪个 Agent 执行，包含 Agent ID 和角色描述。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainAgentDef {
    /// Agent 唯一 ID（如 "test_engineer"）
    pub id: String,
    /// Agent 角色描述（如 "自动化测试生成专家"）
    pub role: String,
}

impl DomainAgentDef {
    pub fn new(id: impl Into<String>, role: impl Into<String>) -> Self {
        Self { id: id.into(), role: role.into() }
    }
}

// ── 用户输入/审批表单 ───────────────────────────────────────────

/// 用户输入配置
///
/// 在步骤执行前暂停，等待用户填写表单或审批。
/// 支持两种模式：approval_gate（必须审批）和 optional_input（可选输入）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainUserInput {
    /// 是否启用用户输入
    #[serde(default)]
    pub enabled: bool,
    /// 模式：approval_gate（必须审批）| optional_input（可选输入）
    #[serde(default = "default_ui_mode")]
    pub mode: String,
    /// 提示文字（展示给用户）
    #[serde(default)]
    pub prompt: String,
    /// 表单字段列表
    #[serde(default)]
    pub fields: Vec<DomainUserInputField>,
}

fn default_ui_mode() -> String {
    "approval_gate".into()
}

impl DomainUserInput {
    pub fn new() -> Self {
        Self { enabled: true, mode: default_ui_mode(), prompt: String::new(), fields: Vec::new() }
    }

    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    pub fn with_fields(mut self, fields: Vec<DomainUserInputField>) -> Self {
        self.fields = fields;
        self
    }

    pub fn with_mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = mode.into();
        self
    }
}

impl Default for DomainUserInput {
    fn default() -> Self {
        Self::new()
    }
}

/// 用户输入表单字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainUserInputField {
    /// 字段名（用于提交时的 key）
    pub name: String,
    /// 字段类型：confirm | choice | text | multi_choice
    #[serde(rename = "type")]
    pub field_type: String,
    /// 字段标签（展示用）
    pub label: String,
    /// 可选项列表（choice/multi_choice 类型）
    #[serde(default)]
    pub options: Vec<String>,
    /// 是否必填
    #[serde(default)]
    pub required: bool,
    /// 占位符文本
    #[serde(default)]
    pub placeholder: Option<String>,
}

impl DomainUserInputField {
    pub fn new(
        name: impl Into<String>,
        field_type: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            field_type: field_type.into(),
            label: label.into(),
            options: Vec::new(),
            required: false,
            placeholder: None,
        }
    }

    pub fn with_options(mut self, options: Vec<String>) -> Self {
        self.options = options;
        self
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }
}

// ── 审批配置 ────────────────────────────────────────────────────

/// 审批节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainApproval {
    /// 审批消息
    #[serde(default = "default_approval_message")]
    pub message: String,
    /// 审批人角色
    #[serde(default)]
    pub approver: String,
    /// 超时秒数（默认 86400 = 24 小时）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// 超时动作：auto_reject（默认）| auto_approve
    #[serde(default = "default_timeout_action")]
    pub timeout_action: String,
    /// 通过按钮文案
    #[serde(default)]
    pub approve_label: Option<String>,
    /// 拒绝按钮文案
    #[serde(default)]
    pub reject_label: Option<String>,
}

fn default_approval_message() -> String {
    "请审批。24小时超时自动拒绝。".into()
}

fn default_timeout() -> u64 {
    86400
}

fn default_timeout_action() -> String {
    "auto_reject".into()
}

impl Default for DomainApproval {
    fn default() -> Self {
        Self {
            message: default_approval_message(),
            approver: String::new(),
            timeout_secs: default_timeout(),
            timeout_action: default_timeout_action(),
            approve_label: None,
            reject_label: None,
        }
    }
}

// ── 领域工作流定义 ──────────────────────────────────────────────

/// 领域工作流定义
///
/// 描述一个通用业务场景的完整工作流，包含触发、执行步骤和元数据。
/// 通过 DomainWorkflowGenerator 可转换为 WorkflowTemplateData。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainWorkflowDef {
    /// 工作流唯一 ID
    pub id: String,
    /// 工作流名称
    pub name: String,
    /// 工作流描述
    #[serde(default)]
    pub description: String,
    /// 图标（emoji 或 URL）
    #[serde(default = "default_icon")]
    pub icon: String,
    /// 标签列表（用于分类和搜索）
    #[serde(default)]
    pub tags: Vec<String>,
    /// 绑定的 Agent Profile ID（如 "opc-cmo-cmo-content-strategist"）
    #[serde(default)]
    pub profile_id: String,
    /// 步骤列表（按顺序执行）
    #[serde(default)]
    pub steps: Vec<DomainStepDef>,
}

fn default_icon() -> String {
    "📄".into()
}

impl DomainWorkflowDef {
    /// 创建新的领域工作流定义
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            icon: default_icon(),
            tags: Vec::new(),
            profile_id: String::new(),
            steps: Vec::new(),
        }
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 设置图标
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = icon.into();
        self
    }

    /// 添加标签
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 设置 profile_id
    pub fn with_profile_id(mut self, profile_id: impl Into<String>) -> Self {
        self.profile_id = profile_id.into();
        self
    }

    /// 添加步骤
    pub fn with_steps(mut self, steps: Vec<DomainStepDef>) -> Self {
        self.steps = steps;
        self
    }
}

// ── 领域适配器工厂 ──────────────────────────────────────────────

/// 领域适配器工厂
///
/// 负责注册和查询内建的领域工作流定义。
/// 与行业适配器工厂（IndustryAdapterFactory）并列，
/// 行业面向垂直行业，领域面向通用业务场景。
///
/// 内建 17 个领域，共 65 个工作流，源自 config/opc/domains/ 下的 YAML 配置。
pub struct DomainAdapterFactory;

include!("generated.rs");
// ── 领域工作流生成器 ────────────────────────────────────────────

/// 领域工作流生成器
///
/// 将 DomainWorkflowDef（纯数据）转换为 WorkflowTemplateData（工作流引擎可执行格式）。
/// 生成的工作流为线性链：trigger → step1 → step2 → ... → end，
/// 支持变量收集、工具注入、profile 绑定和审批分支。
pub struct DomainWorkflowGenerator;

impl DomainWorkflowGenerator {
    /// 从 DomainWorkflowDef 生成 WorkflowTemplateData
    ///
    /// # 生成逻辑
    ///
    /// 1. **变量收集**：扫描所有 step.inputs 中的 `{var}` 引用，声明为工作流变量
    /// 2. **触发节点**：创建 Manual 类型的 Trigger 节点
    /// 3. **步骤链**：按顺序将 DomainStepDef 映射为 AgentNode 或 ApprovalNode
    ///    - agent 节点：注入 prompt + 工具 + profile_id + 输入映射
    ///    - approval 节点：配置审批消息、超时、通过/拒绝分支
    /// 4. **结束节点**：创建 End 节点
    /// 5. **边串接**：trigger → s0 → s1 → ... → end
    ///    - approval 节点产生条件分支：通过 → 下一节点，拒绝 → end
    ///
    /// # 参数
    ///
    /// - `def`: 领域工作流定义
    /// - `version`: 工作流版本号（用于种子化幂等判断）
    pub fn gen_to_template_data(def: &DomainWorkflowDef, version: i32) -> WorkflowTemplateData {
        let now = now_ts();
        let mut nodes: Vec<WorkflowNode> = Vec::new();
        let mut edges: Vec<WorkflowEdge> = Vec::new();

        // ── 1. 变量收集 ──
        let variables = Self::collect_variables(def);

        // ── 2. 触发节点 ──
        nodes.push(WorkflowNode::Trigger(TriggerNode {
            base: Self::make_base("trigger", "手动启动", "用户选择后启动工作流", 250.0, 0.0),
            config: TriggerConfig {
                trigger_type: axagent_harness::workflow_types::TriggerType::Manual,
                config: serde_json::json!({}),
            },
        }));

        // ── 3. 构建节点序列（含 condition 展开 + approval_gate 展开） ──
        // NodeDesc: 节点描述符，支持 condition/approval/agent 三种类型
        #[derive(Clone)]
        struct NodeDesc {
            id: String,
            is_condition: bool,
            is_approval: bool,
            node: WorkflowNode,
        }
        let mut node_descs: Vec<NodeDesc> = Vec::new();
        let mut y_offset: f64 = 150.0;

        for step in &def.steps {
            // 如果步骤有 condition，先插入 ConditionNode
            if Self::has_condition(step) {
                let condition_y = y_offset;
                if let Some(condition_node) = Self::build_condition_node(step, condition_y) {
                    let condition_id = format!("{}-cond", step.id);
                    node_descs.push(NodeDesc {
                        id: condition_id,
                        is_condition: true,
                        is_approval: false,
                        node: condition_node,
                    });
                    y_offset += 200.0;
                }
            }

            // 计算当前 step 的 y 坐标
            let y = y_offset;

            if step.node_type == "approval" {
                // 原生 approval 节点
                let node = Self::build_approval_node(step, y);
                node_descs.push(NodeDesc {
                    id: step.id.clone(),
                    is_condition: false,
                    is_approval: true,
                    node,
                });
                y_offset += 200.0;
            } else {
                // Agent 节点
                let node = Self::build_agent_node(step, def, y);
                node_descs.push(NodeDesc {
                    id: step.id.clone(),
                    is_condition: false,
                    is_approval: false,
                    node,
                });
                y_offset += 200.0;

                // 如果步骤有 approval_gate，追加 Approval 节点
                if Self::has_approval_gate(step) {
                    let approval_node_y = y_offset;
                    if let Some(approval_node) =
                        Self::build_approval_node_from_user_input(step, approval_node_y)
                    {
                        let approval_id = format!("{}-approval", step.id);
                        node_descs.push(NodeDesc {
                            id: approval_id,
                            is_condition: false,
                            is_approval: true,
                            node: approval_node,
                        });
                        y_offset += 200.0;
                    }
                }
            }
        }

        // 将所有节点添加到 nodes 向量
        for desc in &node_descs {
            nodes.push(desc.node.clone());
        }

        // ── 4. 结束节点 ──
        let end_y = y_offset;
        nodes.push(WorkflowNode::End(EndNode {
            base: Self::make_base("end", "完成", "", 250.0, end_y),
            config: EndNodeConfig { output_var: None },
        }));

        // ── 5. 边串接（单元驱动） ──
        // 步骤单元 = [前置 condition?] + 主节点(agent/approval) + [approval_gate?]
        // 语义：
        //   - condition true  → 执行本单元主节点；false → 跳过本单元，继续下一单元
        //   - approval 拒绝   → end（人工裁决终止）；通过 → 下一单元
        //   - 普通节点        → Direct 链式
        if node_descs.is_empty() {
            edges.push(Self::make_edge("e-trigger-end", "trigger", "end"));
            return Self::assemble_template(def, version, now, nodes, edges, variables);
        }

        // 划分步骤单元
        struct Unit {
            cond_id: Option<String>,
            main_id: String,
            main_is_approval: bool,
        }
        let mut units: Vec<Unit> = Vec::new();
        for desc in &node_descs {
            if desc.is_condition {
                units.push(Unit {
                    cond_id: Some(desc.id.clone()),
                    main_id: String::new(),
                    main_is_approval: false,
                });
            } else if units.is_empty() || !units.last().unwrap().main_id.is_empty() {
                units.push(Unit {
                    cond_id: None,
                    main_id: desc.id.clone(),
                    main_is_approval: desc.is_approval,
                });
            } else {
                // 条件节点后的首个非条件节点即该单元的主节点
                let last = units.last_mut().unwrap();
                last.main_id = desc.id.clone();
                last.main_is_approval = desc.is_approval;
            }
        }

        // 单元入口：入口 = cond（若有）否则主节点
        let unit_entry = |u: &Unit| u.cond_id.as_ref().unwrap_or(&u.main_id).clone();

        for (i, unit) in units.iter().enumerate() {
            let entry = unit_entry(unit);
            let next_entry = units.get(i + 1).map(unit_entry).unwrap_or_else(|| "end".to_string());

            // 首个单元从 trigger 进入（Direct）；后续单元的接入由上一单元出口逻辑负责
            if i == 0 {
                edges.push(Self::make_edge(&format!("e-trigger-{}", entry), "trigger", &entry));
            }

            // condition 节点：true → 主节点；false → 跳过本单元，直接下一单元入口
            if let Some(cond_id) = &unit.cond_id {
                edges.push(Self::make_cond_edge(
                    &format!("e-{}-{}-true", cond_id, unit.main_id),
                    cond_id,
                    &unit.main_id,
                    true,
                ));
                edges.push(Self::make_cond_edge(
                    &format!("e-{}-{}-false", cond_id, next_entry),
                    cond_id,
                    &next_entry,
                    false,
                ));
            }

            // 主节点出口（负责接入下一单元）
            if unit.main_is_approval {
                // 原生 approval：通过 → 下一单元，拒绝 → end
                edges.push(Self::make_cond_edge(
                    &format!("e-{}-{}-true", unit.main_id, next_entry),
                    &unit.main_id,
                    &next_entry,
                    true,
                ));
                edges.push(Self::make_cond_edge(
                    &format!("e-{}-end-false", unit.main_id),
                    &unit.main_id,
                    "end",
                    false,
                ));
            } else {
                // 普通 agent：Direct → 下一单元
                edges.push(Self::make_edge(
                    &format!("e-{}-{}", unit.main_id, next_entry),
                    &unit.main_id,
                    &next_entry,
                ));
            }
        }

        Self::assemble_template(def, version, now, nodes, edges, variables)
    }

    // ── 内部辅助方法 ──

    /// 收集所有步骤 inputs 中的 `{var}` 引用，生成变量声明列表
    fn collect_variables(def: &DomainWorkflowDef) -> Vec<Variable> {
        let mut variables: Vec<Variable> = Vec::new();
        for step in &def.steps {
            for v in step.inputs.values() {
                if let Some(name) = v.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                    if !variables.iter().any(|x| x.name == name) {
                        variables.push(Variable {
                            name: name.to_string(),
                            var_type: "string".to_string(),
                            value: serde_json::Value::String(String::new()),
                            description: Some(format!("工作流输入变量 {name}")),
                            is_secret: false,
                        });
                    }
                }
            }
        }
        variables
    }

    /// 工作流步骤专家声明（with_agent 的专家语义 id）→ 专家库真实 AgentProfile（exp-*）映射。
    ///
    /// 背景：wf-eng-refactor 等 62 步工作流的 with_agent 声明了架构分析师 / 代码审计专家等
    /// 27 种专家（DomainAgentDef.id），但专家语义 id 并非真实 profile id。此前生成器全部
    /// fallback 到工作流级 `opc-cto-cto-ai-engineer`，导致 62 步全由同一专家执行、分工空转。
    /// 本映射表将专家语义 id 绑定到 agency-agents-src 专家库的真实组合 profile：
    ///   - 专家维度：expert_id = `agency-{dir}-{file_stem}`（agency_experts 表人才）
    ///   - 角色维度：agent_role = 域映射真实角色（如 engineering → cto，agent_roles 表岗位）
    ///   - profile id = `exp-{source_dir}-{expert.id}`（agent_profiles 表组合，seed 自
    ///     seed_bulk_expert_profiles，agent_role 由 role_for_source_dir 解析）
    /// 使「专家 × 角色」组合分工真实生效。来源：output/opc-software-dev-roles-experts-2026-08-09.md §2。
    const EXPERT_PROFILE_MAP: &[(&str, &str)] = &[
        // ── 架构/设计 ──
        ("architect_analyst", "exp-engineering-agency-engineering-engineering-backend-architect"),
        ("solution_architect", "exp-engineering-agency-engineering-engineering-backend-architect"),
        ("refactor_consultant", "exp-engineering-agency-engineering-engineering-code-reviewer"),
        // ── 工程/开发 ──
        ("code_auditor", "exp-engineering-agency-engineering-engineering-code-reviewer"),
        ("code_reviewer", "exp-engineering-agency-engineering-engineering-code-reviewer"),
        ("senior_engineer", "exp-engineering-agency-engineering-engineering-senior-developer"),
        (
            "ts_framework_specialist",
            "exp-engineering-agency-engineering-engineering-frontend-developer",
        ),
        (
            "frontend_framework_specialist",
            "exp-engineering-agency-engineering-engineering-frontend-developer",
        ),
        (
            "backend_integration_specialist",
            "exp-engineering-agency-engineering-engineering-backend-architect",
        ),
        // ── 质量/测试 ──
        ("quality_expert", "exp-specialized-agency-specialized-specialized-model-qa"),
        ("quality_engineer", "exp-testing-agency-testing-testing-reality-checker"),
        ("quality_director", "exp-testing-agency-testing-testing-reality-checker"),
        ("behavior_tester", "exp-testing-agency-testing-testing-api-tester"),
        ("behavior_verifier", "exp-testing-agency-testing-testing-reality-checker"),
        ("test_engineer", "exp-testing-agency-testing-testing-api-tester"),
        ("integration_engineer", "exp-testing-agency-testing-testing-api-tester"),
        ("performance_analyst", "exp-testing-agency-testing-testing-performance-benchmarker"),
        ("cross_language_verifier", "exp-testing-agency-testing-testing-cross-language-verifier"),
        // ── 交付/运维 ──
        ("devops_engineer", "exp-engineering-agency-engineering-engineering-devops-automator"),
        ("ops_engineer", "exp-engineering-agency-engineering-engineering-sre"),
        ("tech_writer", "exp-engineering-agency-engineering-engineering-technical-writer"),
        // ── 管理/流程 ──
        (
            "project_manager",
            "exp-project-management-agency-project-management-project-manager-senior",
        ),
        (
            "tech_project_manager",
            "exp-project-management-agency-project-management-project-manager-senior",
        ),
        ("change_manager", "exp-specialized-agency-specialized-change-management-consultant"),
        (
            "knowledge_engineer",
            "exp-engineering-agency-engineering-engineering-codebase-onboarding-engineer",
        ),
        // ── 跨语言迁移（3 个缺口专家，补 md 后启用） ──
        ("cpp_rust_migrator", "exp-engineering-agency-engineering-engineering-rust-migrator"),
        ("code_converter", "exp-engineering-agency-engineering-engineering-code-converter"),
    ];

    /// 将步骤的专家语义 id 解析为专家库真实 AgentProfile id（exp-*）。
    /// 命中映射表 → 返回真实 profile；未命中 → None（调用方 fallback）。
    fn resolve_expert_profile(agent_id: &str) -> Option<String> {
        Self::EXPERT_PROFILE_MAP
            .iter()
            .find(|(id, _)| *id == agent_id)
            .map(|(_, profile)| profile.to_string())
    }

    /// 构建 Agent 节点（对应 DomainStepDef 中 node_type == "agent"）
    fn build_agent_node(step: &DomainStepDef, def: &DomainWorkflowDef, y: f64) -> WorkflowNode {
        // 输入映射
        let mut input_mapping: HashMap<String, String> = HashMap::new();
        for (k, v) in &step.inputs {
            input_mapping.insert(k.clone(), v.clone());
        }

        // 工具白名单 → ToolDef 列表
        let node_tools = if step.tools.is_empty() {
            Vec::new()
        } else {
            step.tools
                .iter()
                .map(|t| axagent_harness::workflow_types::ToolDef {
                    name: t.clone(),
                    description: None,
                    parameters: None,
                })
                .collect()
        };

        // 步骤级专家优先于工作流级 profile。
        // 解析优先级：
        //   1. 专家语义 id（code_auditor 等）命中 EXPERT_PROFILE_MAP → 绑定专家库真实 profile（exp-*）
        //   2. 已是真实 profile 标识（opc-*/exp-* 前缀）→ 直接使用
        //   3. 否则 fallback 工作流级 profile（opc-cto-cto-ai-engineer）
        // 专家分工通过 agent_profile_id 绑定真实人才；system_prompt 的"你扮演"前缀保留岗位职责说明。
        let profile_id = step
            .agent
            .as_ref()
            .and_then(|a| Self::resolve_expert_profile(&a.id))
            .or_else(|| {
                step.agent.as_ref().map(|a| a.id.clone()).filter(|id| {
                    !id.is_empty() && (id.starts_with("opc-") || id.starts_with("exp-"))
                })
            })
            .or_else(|| (!def.profile_id.is_empty()).then(|| def.profile_id.clone()));

        // 组装系统提示词（含角色前缀 + on_error 降级说明）
        let mut system_prompt = String::new();
        if let Some(agent) = &step.agent {
            if !agent.role.is_empty() {
                system_prompt.push_str(&format!("你扮演：{}。\n\n", agent.role));
            }
        }
        system_prompt.push_str(&step.prompt);
        if let Some(on_error) = &step.on_error {
            system_prompt.push_str(&format!("\n\n[失败降级] {on_error}"));
        }

        let mut base = Self::make_base(&step.id, &step.title, "", 250.0, y);
        base.continue_on_fail = step.continue_on_fail.unwrap_or(false);

        WorkflowNode::Agent(AgentNode {
            base,
            config: AgentNodeConfig {
                system_prompt,
                context_sources: vec![],
                output_var: format!("{}_result", step.id),
                model: None,
                temperature: None,
                max_tokens: None,
                tools: node_tools.clone(),
                exposed_tools: node_tools.iter().map(|t| t.name.clone()).collect(),
                output_mode: OutputMode::Json,
                agent_profile_id: profile_id,
                max_tool_rounds: Some(10),
                execution_mode: None,
                rag_source_ids: vec![],
                model_role: Some("opc-worker".to_string()),
                consistency_check: None,
                hallucination_guard: Some(
                    axagent_harness::hallucination_guard::HallucinationGuardConfig {
                        enabled: true,
                        match_threshold: 0.4,
                    },
                ),
                fallback_model: None,
                task_scene: None,
                stream_chunk_timeout_secs: None,
                input_mapping,
            },
        })
    }

    /// 构建 Approval 节点（对应 DomainStepDef 中 node_type == "approval"）
    fn build_approval_node(step: &DomainStepDef, y: f64) -> WorkflowNode {
        let cfg = step.approval.clone().unwrap_or_default();

        // 审批消息：附加按钮文案
        let mut message = cfg.message.clone();
        if let Some(label) = &cfg.approve_label {
            message.push_str(&format!("\n[通过] {label}"));
        }
        if let Some(label) = &cfg.reject_label {
            message.push_str(&format!("\n[拒绝] {label}"));
        }

        let mut base = Self::make_base(&step.id, &step.title, "", 250.0, y);
        base.continue_on_fail = step.continue_on_fail.unwrap_or(false);

        WorkflowNode::Approval(ApprovalNode {
            base,
            config: ApprovalNodeConfig {
                message,
                approver: if cfg.approver.is_empty() {
                    None
                } else {
                    Some(cfg.approver)
                },
                timeout_secs: cfg.timeout_secs,
                timeout_action: cfg.timeout_action,
                output_var: format!("{}_result", step.id),
            },
        })
    }

    /// 从 user_input 创建 Approval 节点（用于 approval_gate 模式）
    fn build_approval_node_from_user_input(step: &DomainStepDef, y: f64) -> Option<WorkflowNode> {
        let ui = step.user_input.as_ref()?;
        if ui.mode != "approval_gate" || !ui.enabled {
            return None;
        }

        // 从 confirm 字段提取 approve/reject 标签
        let mut approve_label = None;
        let mut reject_label = None;
        for field in &ui.fields {
            if field.field_type == "confirm" {
                if !field.options.is_empty() {
                    approve_label = Some(field.options[0].clone());
                }
                if field.options.len() >= 2 {
                    reject_label = Some(field.options[1].clone());
                }
                break;
            }
        }

        let mut message = ui.prompt.clone();
        if let Some(ref label) = approve_label {
            message.push_str(&format!("\n[批准] {label}"));
        }
        if let Some(ref label) = reject_label {
            message.push_str(&format!("[驳回] {label}"));
        }

        let approval_id = format!("{}-approval", step.id);
        let approval_title = format!("{} - 审批", step.title);

        let base = Self::make_base(&approval_id, &approval_title, "人工审批节点", 250.0, y);

        Some(WorkflowNode::Approval(ApprovalNode {
            base,
            config: ApprovalNodeConfig {
                message,
                approver: None,
                timeout_secs: 86400,
                timeout_action: "auto_reject".to_string(),
                output_var: format!("{}_result", approval_id),
            },
        }))
    }

    /// 判断步骤是否需要 approval_gate（user_input 模式）
    fn has_approval_gate(step: &DomainStepDef) -> bool {
        step.user_input.as_ref().map(|ui| ui.mode == "approval_gate" && ui.enabled).unwrap_or(false)
    }

    /// 判断步骤是否有条件执行
    fn has_condition(step: &DomainStepDef) -> bool {
        step.condition.as_ref().map(|c| !c.is_empty()).unwrap_or(false)
    }

    /// 构建 Condition 节点（用于在带 condition 的步骤前做条件分流）
    fn build_condition_node(step: &DomainStepDef, y: f64) -> Option<WorkflowNode> {
        let condition_expr = step.condition.as_ref()?;
        if condition_expr.is_empty() {
            return None;
        }

        let condition_id = format!("{}-cond", step.id);
        let condition_title = format!("{} - 条件判断", step.title);

        let base = Self::make_base(&condition_id, &condition_title, "条件分流节点", 250.0, y);

        // 使用 LLM 路由模式：将 Rhai 表达式作为路由提示词
        Some(WorkflowNode::Condition(ConditionNode {
            base,
            config: ConditionNodeConfig {
                conditions: Vec::new(),
                logical_op: LogicalOperator::And,
                judge_by_llm: Some(true),
                routing_prompt: Some(condition_expr.clone()),
                routing_model: None,
                confidence_threshold: None,
            },
        }))
    }

    /// 创建 WorkflowNodeBase 基础配置
    fn make_base(id: &str, title: &str, desc: &str, x: f64, y: f64) -> WorkflowNodeBase {
        WorkflowNodeBase {
            id: id.into(),
            title: title.into(),
            description: Some(desc.into()),
            position: Position { x, y },
            retry: RetryConfig::default(),
            timeout: Some(300),
            enabled: true,
            parent_id: None,
            compensation: None,
            continue_on_fail: false,
        }
    }

    /// 创建直线边（Direct）
    fn make_edge(id: &str, src: &str, tgt: &str) -> WorkflowEdge {
        WorkflowEdge {
            id: id.into(),
            source: src.into(),
            source_handle: None,
            target: tgt.into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        }
    }

    /// 创建条件边（ConditionTrue / ConditionFalse）
    fn make_cond_edge(id: &str, src: &str, tgt: &str, is_true: bool) -> WorkflowEdge {
        WorkflowEdge {
            id: id.into(),
            source: src.into(),
            source_handle: Some(if is_true {
                "true".into()
            } else {
                "false".into()
            }),
            target: tgt.into(),
            target_handle: None,
            edge_type: if is_true {
                EdgeType::ConditionTrue
            } else {
                EdgeType::ConditionFalse
            },
            label: None,
        }
    }

    /// 组装最终的 WorkflowTemplateData
    fn assemble_template(
        def: &DomainWorkflowDef,
        version: i32,
        now: i64,
        nodes: Vec<WorkflowNode>,
        edges: Vec<WorkflowEdge>,
        variables: Vec<Variable>,
    ) -> WorkflowTemplateData {
        WorkflowTemplateData {
            id: def.id.clone(),
            name: def.name.clone(),
            description: Some(def.description.clone()),
            icon: if def.icon.is_empty() {
                default_icon()
            } else {
                def.icon.clone()
            },
            tags: def.tags.clone(),
            version,
            is_preset: true,
            is_editable: true,
            is_public: false,
            trigger_config: Some(TriggerConfig {
                trigger_type: axagent_harness::workflow_types::TriggerType::Manual,
                config: serde_json::json!({}),
            }),
            nodes,
            edges,
            input_schema: None,
            output_schema: None,
            variables,
            error_config: Some(ErrorConfig {
                retry_policy: None,
                on_failure: OnFailureAction::RetryThenAbort,
                error_branch: None,
                compensation_steps: None,
            }),
            error_workflow_id: None,
            mission_hash: None,
            tool_defs: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

// ── 单元测试 ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_workflow_def_creation() {
        let def = DomainWorkflowDef::new("test", "测试工作流")
            .with_description("这是一个测试")
            .with_icon("🧪")
            .with_tags(vec!["测试".to_string()])
            .with_profile_id("test-profile");

        assert_eq!(def.id, "test");
        assert_eq!(def.name, "测试工作流");
        assert_eq!(def.profile_id, "test-profile");
        assert_eq!(def.tags, vec!["测试".to_string()]);
    }

    #[test]
    fn test_domain_step_def_agent() {
        let step = DomainStepDef::agent("step-1", "步骤一")
            .with_prompt("执行任务")
            .with_tools(vec!["tool_a".to_string()])
            .with_continue_on_fail(true);

        assert_eq!(step.id, "step-1");
        assert_eq!(step.node_type, "agent");
        assert_eq!(step.prompt, "执行任务");
        assert_eq!(step.tools, vec!["tool_a"]);
        assert_eq!(step.continue_on_fail, Some(true));
    }

    #[test]
    fn test_domain_step_def_approval() {
        let step = DomainStepDef::approval("approve-1", "审批");

        assert_eq!(step.node_type, "approval");
        assert!(step.approval.is_some());
        let approval = step.approval.unwrap();
        assert_eq!(approval.timeout_secs, 86400);
        assert_eq!(approval.timeout_action, "auto_reject");
    }

    #[test]
    fn test_factory_list_all() {
        let list = DomainAdapterFactory::list_all();
        assert!(!list.is_empty());
        assert!(list.iter().any(|(id, _)| *id == "academic"));
    }

    #[test]
    fn test_factory_create_known() {
        let def = DomainAdapterFactory::create("academic");
        assert!(def.is_some());
        let def = def.unwrap();
        assert_eq!(def.id, "wf-acd-literature");
        assert!(!def.steps.is_empty());
    }

    #[test]
    fn test_factory_create_unknown() {
        let def = DomainAdapterFactory::create("nonexistent");
        assert!(def.is_none());
    }

    #[test]
    fn test_gen_to_template_data_empty_steps() {
        let def = DomainWorkflowDef::new("empty", "空工作流");
        let template = DomainWorkflowGenerator::gen_to_template_data(&def, 1);

        assert_eq!(template.id, "empty");
        assert_eq!(template.nodes.len(), 2); // trigger + end
        assert_eq!(template.edges.len(), 1); // trigger → end
    }

    #[test]
    fn test_gen_to_template_data_linear_chain() {
        let def = DomainWorkflowDef::new("linear", "线性工作流").with_steps(vec![
            DomainStepDef::agent("s1", "步骤一").with_prompt("任务一"),
            DomainStepDef::agent("s2", "步骤二").with_prompt("任务二"),
        ]);

        let template = DomainWorkflowGenerator::gen_to_template_data(&def, 1);

        // trigger + s1 + s2 + end = 4 节点
        assert_eq!(template.nodes.len(), 4);
        // trigger→s1, s1→s2, s2→end = 3 边
        assert_eq!(template.edges.len(), 3);
    }

    #[test]
    fn test_gen_to_template_data_with_approval() {
        let def = DomainWorkflowDef::new("with_approval", "含审批工作流").with_steps(vec![
            DomainStepDef::agent("s1", "前置步骤").with_prompt("任务一"),
            DomainStepDef::approval("a1", "审批"),
            DomainStepDef::agent("s2", "后置步骤").with_prompt("任务二"),
        ]);

        let template = DomainWorkflowGenerator::gen_to_template_data(&def, 1);

        // trigger + s1 + a1 + s2 + end = 5 节点
        assert_eq!(template.nodes.len(), 5);

        // trigger→s1, s1→a1(Direct), a1→end(false), a1→s2(true), s2→end = 5 边
        assert_eq!(template.edges.len(), 5);
    }

    #[test]
    fn test_gen_to_template_data_condition_skip() {
        // 带 condition 的步骤：false 分支应跳到下一节点（跳过本步骤），而非 end
        let def = DomainWorkflowDef::new("with_cond", "条件跳过工作流").with_steps(vec![
            DomainStepDef::agent("s1", "前置步骤").with_prompt("任务一"),
            DomainStepDef::agent("s2", "条件步骤")
                .with_prompt("任务二")
                .with_condition("flag == true"),
            DomainStepDef::agent("s3", "后置步骤").with_prompt("任务三"),
        ]);

        let template = DomainWorkflowGenerator::gen_to_template_data(&def, 1);

        // trigger + s1 + s2-cond + s2 + s3 + end = 6 节点
        assert_eq!(template.nodes.len(), 6);

        // 关键断言：s2-cond 的 false 分支必须指向 s3（跳过 s2），而非 end
        let cond_false_edge = template
            .edges
            .iter()
            .find(|e| e.source == "s2-cond" && e.source_handle.as_deref() == Some("false"));
        assert!(cond_false_edge.is_some(), "s2-cond 必须存在 false 分支");
        assert_eq!(
            cond_false_edge.unwrap().target,
            "s3",
            "condition false 应跳过本步骤指向下一节点 s3，而非 end"
        );

        // s2-cond 的 true 分支指向 s2
        let cond_true_edge = template
            .edges
            .iter()
            .find(|e| e.source == "s2-cond" && e.source_handle.as_deref() == Some("true"));
        assert!(cond_true_edge.is_some());
        assert_eq!(cond_true_edge.unwrap().target, "s2");
    }

    #[test]
    fn test_gen_to_template_data_approval_gate_expansion() {
        // approval_gate（user_input）应展开为独立的 Approval 节点，拒绝 → end
        let mut ui = DomainUserInput::new();
        ui = ui.with_mode("approval_gate");
        ui = ui.with_prompt("请审批");
        ui = ui.with_fields(vec![DomainUserInputField::new("approve", "confirm", "是否批准")
            .with_options(vec!["批准执行".to_string(), "驳回".to_string()])
            .with_required(true)]);
        let def = DomainWorkflowDef::new("with_gate", "审批门工作流").with_steps(vec![
            DomainStepDef::agent("s1", "前置步骤").with_prompt("任务一").with_user_input(ui),
            DomainStepDef::agent("s2", "后置步骤").with_prompt("任务二"),
        ]);

        let template = DomainWorkflowGenerator::gen_to_template_data(&def, 1);

        // trigger + s1 + s1-approval + s2 + end = 5 节点
        assert_eq!(template.nodes.len(), 5);
        assert!(
            template.nodes.iter().any(|n| n.base_id() == "s1-approval"),
            "approval_gate 应展开为独立 Approval 节点"
        );

        // s1-approval 拒绝 → end
        let reject_edge = template
            .edges
            .iter()
            .find(|e| e.source == "s1-approval" && e.source_handle.as_deref() == Some("false"));
        assert!(reject_edge.is_some(), "审批拒绝分支必须存在");
        assert_eq!(reject_edge.unwrap().target, "end");
    }

    #[test]
    fn test_gen_to_template_data_variable_collection() {
        let def = DomainWorkflowDef::new("with_vars", "含变量工作流").with_steps(vec![
            DomainStepDef::agent("s1", "步骤").with_inputs(HashMap::from([
                ("audience".to_string(), "{audience}".to_string()),
                ("topic".to_string(), "{topic}".to_string()),
                ("redundant".to_string(), "{audience}".to_string()),
            ])),
        ]);

        let template = DomainWorkflowGenerator::gen_to_template_data(&def, 1);

        assert_eq!(template.variables.len(), 2); // audience + topic（去重）
        assert!(template.variables.iter().any(|v| v.name == "audience"));
        assert!(template.variables.iter().any(|v| v.name == "topic"));
    }

    #[test]
    fn test_gen_to_template_data_profile_binding() {
        let def = DomainWorkflowDef::new("with_profile", "绑定Profile")
            .with_profile_id("my-profile")
            .with_steps(vec![DomainStepDef::agent("s1", "步骤").with_prompt("任务")]);

        let template = DomainWorkflowGenerator::gen_to_template_data(&def, 1);

        // 找到 agent 节点并检查 profile_id
        let has_profile = template.nodes.iter().any(|n| {
            if let WorkflowNode::Agent(agent) = n {
                agent.config.agent_profile_id == Some("my-profile".to_string())
            } else {
                false
            }
        });
        assert!(has_profile);
    }

    #[test]
    fn test_resolve_expert_profile_map() {
        // 专家语义 id → 专家库真实 profile（exp-*）解析
        let cases = [
            ("code_auditor", "exp-engineering-agency-engineering-engineering-code-reviewer"),
            ("code_reviewer", "exp-engineering-agency-engineering-engineering-code-reviewer"),
            ("senior_engineer", "exp-engineering-agency-engineering-engineering-senior-developer"),
            ("devops_engineer", "exp-engineering-agency-engineering-engineering-devops-automator"),
            ("tech_writer", "exp-engineering-agency-engineering-engineering-technical-writer"),
            ("performance_analyst", "exp-testing-agency-testing-testing-performance-benchmarker"),
            ("test_engineer", "exp-testing-agency-testing-testing-api-tester"),
            ("quality_director", "exp-testing-agency-testing-testing-reality-checker"),
            (
                "project_manager",
                "exp-project-management-agency-project-management-project-manager-senior",
            ),
            ("change_manager", "exp-specialized-agency-specialized-change-management-consultant"),
            ("cpp_rust_migrator", "exp-engineering-agency-engineering-engineering-rust-migrator"),
            ("code_converter", "exp-engineering-agency-engineering-engineering-code-converter"),
            (
                "cross_language_verifier",
                "exp-testing-agency-testing-testing-cross-language-verifier",
            ),
        ];
        for (agent_id, expected) in cases {
            let resolved = DomainWorkflowGenerator::resolve_expert_profile(agent_id);
            assert_eq!(resolved.as_deref(), Some(expected), "专家 {agent_id} 应解析为 {expected}");
        }
        // 未注册的专家语义 id → None（fallback）
        assert_eq!(DomainWorkflowGenerator::resolve_expert_profile("ghost_expert"), None);
    }

    #[test]
    fn test_gen_to_template_data_expert_profile_resolution() {
        // with_agent 声明的专家应绑定到专家库真实 profile（exp-*），而非工作流级
        let def = DomainWorkflowDef::new("with_expert", "专家绑定")
            .with_profile_id("opc-cto-cto-ai-engineer")
            .with_steps(vec![DomainStepDef::agent("s1", "代码审查")
                .with_prompt("任务")
                .with_agent(DomainAgentDef::new("code_auditor", "代码审计专家"))]);

        let template = DomainWorkflowGenerator::gen_to_template_data(&def, 1);

        let agent_profile = template.nodes.iter().find_map(|n| {
            if let WorkflowNode::Agent(agent) = n {
                agent.config.agent_profile_id.clone()
            } else {
                None
            }
        });
        assert_eq!(
            agent_profile.as_deref(),
            Some("exp-engineering-agency-engineering-engineering-code-reviewer"),
            "code_auditor 应绑定专家库真实 profile，而非工作流级 opc-cto-cto-ai-engineer"
        );
    }

    #[test]
    fn test_gen_to_template_tool_injection() {
        let def = DomainWorkflowDef::new("with_tools", "工具注入").with_steps(vec![
            DomainStepDef::agent("s1", "步骤")
                .with_prompt("任务")
                .with_tools(vec!["tool_a".to_string(), "tool_b".to_string()]),
        ]);

        let template = DomainWorkflowGenerator::gen_to_template_data(&def, 1);

        // 找到 agent 节点并检查工具
        let node = template.nodes.iter().find(|n| matches!(n, WorkflowNode::Agent(_agent_inner)));
        assert!(node.is_some());
        if let Some(WorkflowNode::Agent(agent)) = node {
            assert_eq!(agent.config.tools.len(), 2);
            assert_eq!(agent.config.exposed_tools.len(), 2);
        }
    }

    #[test]
    fn test_factory_create_all_domains() {
        for (id, _) in DomainAdapterFactory::list_all() {
            let def = DomainAdapterFactory::create(id);
            assert!(def.is_some(), "domain {id} 未注册");
            let def = def.unwrap();
            let template = DomainWorkflowGenerator::gen_to_template_data(&def, 1);
            assert!(template.nodes.len() >= 2); // 至少 trigger + end
        }
    }
}
