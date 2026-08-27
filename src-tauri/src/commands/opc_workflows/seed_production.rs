// SPDX-License-Identifier: AGPL-3.0-only

//! 生产级 OPC 业务工作流 — 参考 OpenOPC examples 实现
//!
//! 包含 OpenOPC 定义的真实业务工作流：
//! 1. Landing Page Sprint — 4 agent 协作，1 天发出落地页
//! 2. Startup MVP — 7 agent 协作，4 周出 MVP

use axagent_harness::capability::Visibility;
use axagent_harness::hallucination_guard::HallucinationGuardConfig;
use axagent_harness::util_fns::now_ts;
use axagent_harness::workflow_types::*;
use sea_orm::DatabaseConnection;

use super::{OPC_TEMPLATE_VERSION, check_template_version, make_base, upsert_template};

fn base(id: &str, title: &str, x: f64, y: f64) -> WorkflowNodeBase {
    make_base(id, title, "", x, y)
}

fn agent(
    id: &str,
    title: &str,
    profile: &str,
    prompt: &str,
    x: f64,
    y: f64,
    output_var: &str,
) -> WorkflowNode {
    WorkflowNode::Agent(AgentNode {
        base: base(id, title, x, y),
        config: AgentNodeConfig {
            system_prompt: prompt.into(),
            context_sources: vec![],
            output_var: output_var.into(),
            model: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            exposed_tools: vec![],
            output_mode: OutputMode::Json,
            agent_profile_id: Some(profile.into()),
            max_tool_rounds: Some(10),
            execution_mode: None,
            rag_source_ids: vec![],
            model_role: Some("opc-worker".to_string()),
            consistency_check: None,
            hallucination_guard: Some(HallucinationGuardConfig {
                enabled: true,
                match_threshold: 0.4,
            }),
            fallback_model: None,
            task_scene: None,
            stream_chunk_timeout_secs: None,
            input_mapping: std::collections::HashMap::new(),
        },
    })
}

fn agent_wim(
    id: &str,
    title: &str,
    profile: &str,
    prompt: &str,
    x: f64,
    y: f64,
    output_var: &str,
    im: Vec<(&str, &str)>,
) -> WorkflowNode {
    let mut node = agent(id, title, profile, prompt, x, y, output_var);
    if let WorkflowNode::Agent(ref mut a) = node {
        a.config.input_mapping = im.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
    }
    node
}

fn end(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: base("end", "完成", x, y),
        config: EndNodeConfig { output_var: None },
    })
}

fn edge(src: &str, tgt: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: format!("e-{src}-{tgt}"),
        source: src.into(),
        source_handle: None,
        target: tgt.into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    }
}

fn edge_cond(src: &str, handle: &str, tgt: &str, etype: EdgeType) -> WorkflowEdge {
    WorkflowEdge {
        id: format!("e-{src}-{handle}-{tgt}"),
        source: src.into(),
        source_handle: Some(handle.into()),
        target: tgt.into(),
        target_handle: None,
        edge_type: etype,
        label: None,
    }
}

// ═══════════════════════════════════════════════════════════════════
// Workflow 1: Landing Page Sprint — 4 agent, 1 天
// ═══════════════════════════════════════════════════════════════════
// OpenOPC 定义:
//   Morning: Content Creator + UI Designer (并行)
//   Midday:  Frontend Developer (需要 copy + design 作为输入)
//   Afternoon: Growth Hacker (review → feedback loop)
//
// WorkflowNode 实现:
//   Trigger → Parallel(Content Creator || UI Designer)
//          → Merge → Frontend Developer
//          → Growth Hacker (review)
//          → Condition(feedback?) → Frontend Developer (revise)
//                                 → End

pub async fn seed_landing_page_workflow(db: &DatabaseConnection) -> Result<(), String> {
    let id = "prod-landing-page";
    if !check_template_version(db, id, OPC_TEMPLATE_VERSION).await? {
        return Ok(());
    }
    let now = now_ts();

    let mut nodes: Vec<WorkflowNode> = vec![];
    let mut edges: Vec<WorkflowEdge> = vec![];

    // 1. Trigger
    nodes.push(WorkflowNode::Trigger(TriggerNode {
        base: base("trigger", "启动落地页项目", 250.0, 0.0),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    }));

    // 2. Parallel: Content Creator + UI Designer
    nodes.push(WorkflowNode::Parallel(ParallelNode {
        base: base("p-morning", "上午并行", 250.0, 120.0),
        config: ParallelNodeConfig {
            branches: vec![
                Branch {
                    id: "branch-content".into(),
                    title: "Content Creator".into(),
                    steps: vec!["a-content".into()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch-design".into(),
                    title: "UI Designer".into(),
                    steps: vec!["a-design".into()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
            ],
            wait_for_all: true,
            timeout: Some(240),
            aggregation: None,
            auto_input_from_parent: true,
            sub_graph: None,
        },
    }));

    // 2a. Content Creator (Parallel 子节点)
    nodes.push(agent("a-content", "内容撰写", "opc-cmo-cmo-content-strategist",
        "你的任务：撰写 SaaS 落地页文案。\n\n产品：FlowSync — API 集成平台，5分钟连接任意两个 SaaS 工具\n客户群：开发者和技术PM\n语气：自信、简洁、略带活泼\n\n需要包含：\n1. Hero（标题+副标题+CTA）\n2. 问题陈述（3个痛点）\n3. 原理（3步）\n4. 社会证明（占位推荐格式）\n5. 定价（3档：免费/专业/企业）\n6. 最终CTA\n\n原则：可扫描、无废话。\n输出 JSON 格式。",
        100.0, 240.0, "copy_result"));

    // 2b. UI Designer (Parallel 子节点)
    nodes.push(agent("a-design", "UI 设计", "opc-cpo-cpo-product-manager",
        "你的任务：设计 SaaS 落地页 UI 规范。\n\n产品：FlowSync API 集成平台\n风格：简洁现代，深色模式可选（Linear/Vercel 风格）\n\n交付物：\n1. 布局线框图（区块顺序+间距）\n2. 调色板（主色/辅助色/强调色/背景色）\n3. 字体搭配（标题字号/正文字号）\n4. 组件规范：Hero区/功能卡片/定价表/CTA按钮\n5. 响应式断点（移动端/平板/桌面）\n\n输出 JSON 格式。",
        400.0, 240.0, "design_result"));

    // 3. MergeNode: 汇聚 copy + design
    nodes.push(WorkflowNode::Merge(MergeNode {
        base: base("m-morning", "合并上午产出", 250.0, 400.0),
        config: MergeNodeConfig {
            merge_type: MergeStrategy::All,
            inputs: vec!["a-content".into(), "a-design".into()],
            auto_inputs_from_branches: true,
        },
    }));

    // 4. Frontend Developer（需要 copy + design 作为输入）
    nodes.push(agent_wim("a-frontend", "前端开发", "opc-cto-cto-ai-engineer",
        "你的任务：根据文案和设计规范构建落地页。\n\n技术栈：HTML、Tailwind CSS、少量原生JS（不需框架）\n要求：\n- 移动优先响应式\n- 加载快（系统字体即可，不用重资源）\n- 无障碍（正确标题层级、alt文本、聚焦状态）\n- 包含可用邮箱注册表单（action: /api/subscribe）\n\n输出：可部署的完整 HTML 文件和内容摘要。",
        250.0, 520.0, "page_result",
        vec![("copy","a-content.result"),("design","a-design.result")]));

    // 5. Growth Hacker 做转化审查
    nodes.push(agent_wim("a-growth", "转化优化", "opc-cmo-cmo-content-strategist",
        "你的任务：审查落地页的转化优化效果。\n\n评估维度：\n1. CTA是否在首屏可见？\n2. 价值主张5秒内是否清晰？\n3. 注册流程有无摩擦？\n4. 首批A/B测试做什么？\n5. SEO基础：meta描述/OG标签/结构化数据\n\n输出 JSON {issues, improvements, ab_tests, seo_score}",
        250.0, 660.0, "growth_result",
        vec![("page","a-frontend.result")]));

    // 6. ConditionNode: 还有问题需要修改？
    nodes.push(WorkflowNode::Condition(ConditionNode {
        base: base("c-feedback", "需要修改?", 250.0, 800.0),
        config: ConditionNodeConfig {
            conditions: vec![], logical_op: LogicalOperator::And,
            judge_by_llm: Some(true),
            routing_prompt: Some("检查审查结果。如果 issues 不为空或严重程度高，返回 true（需要修改）；如果 0 issues 或只有 cosmetic 级别问题，返回 false（直接发布）".into()),
            routing_model: None, confidence_threshold: None,
        },
    }));

    // 6a. Feedback loop: 回到前端修改
    nodes.push(agent_wim(
        "a-revise",
        "前端修改",
        "opc-cto-cto-ai-engineer",
        "你的任务：按转化审查意见修改落地页。\n\n修改要求：解决审查报告中的所有 issue。",
        450.0,
        900.0,
        "revise_result",
        vec![("page", "a-frontend.result"), ("feedback", "a-growth.result")],
    ));

    // 7. End
    nodes.push(end(250.0, 1040.0));

    // ── 边 ──
    edges.push(edge("trigger", "p-morning"));
    edges.push(edge("p-morning", "m-morning"));
    edges.push(edge("m-morning", "a-frontend"));
    edges.push(edge("a-frontend", "a-growth"));
    edges.push(edge("a-growth", "c-feedback"));
    edges.push(edge_cond("c-feedback", "true", "a-revise", EdgeType::ConditionTrue));
    edges.push(edge_cond("c-feedback", "false", "end", EdgeType::ConditionFalse));
    edges.push(edge("a-revise", "end"));

    let data = WorkflowTemplateData {
        id: id.into(),
        name: "Landing Page Sprint".into(),
        description: Some("OpenOPC 定义的 4-agent 协作工作流：Content Creator + UI Designer(并行) → Frontend Developer → Growth Hacker(转化审查) → 反馈循环。1天交付可部署落地页。".into()),
        icon: "🚀".into(),
        cluster_id: None,
        route_path: None,
        tags: vec!["landing-page".into(),"marketing".into(),"web".into()],
        version: OPC_TEMPLATE_VERSION, is_preset: true, is_editable: true, is_public: false,
        visibility: Visibility::Public,
        trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
        nodes, edges, input_schema: None, output_schema: None, variables: vec![],
        error_config: Some(ErrorConfig {
            retry_policy: Some(WorkflowRetryPolicy { max_retries: 3, base_delay_ms: 1000, max_delay_ms: 30000 }),
            on_failure: OnFailureAction::Abort, error_branch: None, compensation_steps: None,
        }),
        error_workflow_id: None, mission_hash: None, tool_defs: vec![], created_at: now, updated_at: now,
    };
    upsert_template(db, data).await
}

// ═══════════════════════════════════════════════════════════════════
// Workflow 2: Startup MVP — 7 agent, 4 周
// ═══════════════════════════════════════════════════════════════════
// OpenOPC 定义:
//   Week 1: Sprint Prioritizer + UX Researcher(并行) → Backend Architect
//   Week 2: Frontend Developer + Rapid Prototyper → Reality Checker(质量门)
//   Week 3: Growth Hacker(launch plan)
//   Week 4: Final Reality Check → GO/NO-GO
//
// 本实现将 4 周过程建模为 4 个阶段的 Workflow DAG。

// nodes 由下方大量分支动态 push 构建，无法用 vec![] 字面量替代
#[allow(clippy::vec_init_then_push)]
pub async fn seed_startup_mvp_workflow(db: &DatabaseConnection) -> Result<(), String> {
    let id = "prod-startup-mvp";
    if !check_template_version(db, id, OPC_TEMPLATE_VERSION).await? {
        return Ok(());
    }
    let now = now_ts();

    let mut nodes: Vec<WorkflowNode> = Vec::new();

    // Phase 1: Discovery + Architecture
    nodes.push(WorkflowNode::Trigger(TriggerNode {
        base: base("trigger", "启动 MVP 项目", 250.0, 0.0),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    }));

    // Parallel: Sprint Prioritizer + UX Researcher
    nodes.push(WorkflowNode::Parallel(ParallelNode {
        base: base("p-week1", "第一周并行", 250.0, 120.0),
        config: ParallelNodeConfig {
            branches: vec![
                Branch {
                    id: "branch-sprint".into(),
                    title: "Sprint Prioritizer".into(),
                    steps: vec!["a-sprint".into()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch-ux".into(),
                    title: "UX Researcher".into(),
                    steps: vec!["a-ux".into()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
            ],
            wait_for_all: true,
            timeout: Some(360),
            aggregation: None,
            auto_input_from_parent: true,
            sub_graph: None,
        },
    }));

    nodes.push(agent("a-sprint", "Sprint 规划", "opc-cto-cto-ai-engineer",
        "你的任务：制定 4 周 MVP 冲刺计划。\n\n项目：RetroBoard — 远程团队回顾工具\n核心功能：用户认证、创建看板、添加卡片、投票、行动项\n约束：单人开发，React+Node.js，Vercel+Railway\n\n输出：拆分为 4 周冲刺，包含交付物和验收标准。JSON 格式。",
        100.0, 240.0, "sprint_result"));

    nodes.push(agent("a-ux", "UX 研究", "opc-cpo-cpo-product-manager",
        "你的任务：竞争分析和差异化研究。\n\n产品：远程团队回顾工具（5-20人）\n竞品：EasyRetro、Retrium、Parabol\n\n输出：\n1. 哪些功能是基本要求\n2. 竞品短板\n3. 一个我们可以占领的差异化点\nJSON 格式的一页研究简报。",
        400.0, 240.0, "ux_result"));

    // Merge → Backend Architect
    nodes.push(WorkflowNode::Merge(MergeNode {
        base: base("m-week1", "合并", 250.0, 400.0),
        config: MergeNodeConfig {
            merge_type: MergeStrategy::All,
            inputs: vec!["a-sprint".into(), "a-ux".into()],
            auto_inputs_from_branches: true,
        },
    }));

    nodes.push(agent_wim("a-backend", "后端架构", "opc-cto-cto-ai-engineer",
        "你的任务：设计 API 和数据库模型。\n\n技术栈：Node.js、Express、PostgreSQL、Socket.io\n\n交付物：\n1. 数据库 Schema（SQL）\n2. REST API 端点列表\n3. WebSocket 事件（实时看板）\n4. 认证策略建议\nJSON 格式。",
        250.0, 520.0, "backend_result",
        vec![("sprint","a-sprint.result"),("research","a-ux.result")]));

    // Phase 2: Build Core (Week 2)
    nodes.push(agent_wim("a-frontend", "前端开发", "opc-cto-cto-ai-engineer",
        "你的任务：构建 RetroBoard 前端。\n\n技术栈：React、TypeScript、Tailwind、Socket.io-client\n页面：登录、仪表盘、看板\n组件：RetroCard、VoteButton、ActionItem\n\n优先看板视图——这是核心体验。确保实时同步。JSON 格式。",
        250.0, 660.0, "frontend_result",
        vec![("api","a-backend.result")]));

    // Quality Gate 1: Reality Checker
    nodes.push(agent_wim("a-reality-1", "中期质量检查", "opc-ceo-ceo-business-strategist",
        "你的任务：评估项目是否按时交付。\n\n第 2 周结束，4 周 MVP 已过半。\n\n评估：\n1. 2 周内能否交付？\n2. 哪些功能需要砍掉？\n3. 哪些技术债会在上线时出问题？\n\n输出 JSON {can_ship, items_to_cut, risks}",
        250.0, 800.0, "reality1_result",
        vec![("backend","a-backend.result"),("frontend","a-frontend.result")]));

    // Phase 3: Growth + Prep (Week 3)
    nodes.push(agent_wim("a-growth", "增长策略", "opc-cmo-cmo-content-strategist",
        "你的任务：制定上市计划。\n\n产品：RetroBoard — 团队回顾工具，1周后上线\n目标：远程优先公司的工程经理和 Scrum Master\n预算：$0（纯有机推广）\n\n创建：\n1. 落地页文案（Hero、功能、CTA）\n2. 推广渠道（Product Hunt、Reddit、HN、Twitter）\n3. 逐日上市序列\n4. 第一周跟踪指标\nJSON 格式。",
        250.0, 940.0, "growth_result",
        vec![("product","a-frontend.result")]));

    // Phase 4: Launch (Week 4)
    nodes.push(agent_wim("a-reality-2", "最终质量门", "opc-ceo-ceo-business-strategist",
        "你的任务：最终上线评估。\n\n运行上线检查清单，给出 GO / NO-GO 决策。每个标准需要证据。\n\n检查项：错误监控、数据库备份、用户认证流程、核心功能可用性。\n输出 JSON {decision, checklist, evidence}",
        250.0, 1080.0, "reality2_result",
        vec![("frontend","a-frontend.result"),("growth","a-growth.result"),("reality","a-reality-1.result")]));

    // Switch: GO / NO-GO
    nodes.push(WorkflowNode::Switch(SwitchNode {
        base: base("s-gonogo", "GO/NO-GO决策", 250.0, 1220.0),
        config: SwitchNodeConfig {
            input_var: "reality2_result.decision".into(),
            cases: vec![
                SwitchCase { value: "go".into(), label: "GO 上线".into() },
                SwitchCase { value: "no-go".into(), label: "NO-GO 打回".into() },
            ],
            default_case: Some("no-go".into()),
            match_mode: "exact".into(),
            use_llm: None,
            llm_prompt: None,
            llm_model: None,
            output_var: "launch_decision".into(),
        },
    }));

    // Post-GO: 上线通知
    nodes.push(agent("a-launch", "上线执行", "opc-coo-coo-operations-manager",
        "你的任务：执行上线操作。\n\n确认上线步骤完成：\n1. 最后代码推送\n2. 域名配置\n3. 监控告警\n4. 发布公告\n\n输出 JSON {deploy_status, url, next_steps}",
        400.0, 1360.0, "launch_result"));

    // Post-NO-GO: 复盘反馈
    nodes.push(agent_wim("a-postpone", "复盘反馈", "opc-ceo-ceo-business-strategist",
        "你的任务：总结 NO-GO 原因，制定修复计划和时间表。\n\n输出 JSON {blockers, fix_plan, timeline}",
        100.0, 1360.0, "postpone_result",
        vec![("reality","a-reality-2.result")]));

    nodes.push(end(250.0, 1500.0));

    // ── 边 ──
    let edges = vec![
        edge("trigger", "p-week1"),
        edge("p-week1", "m-week1"),
        edge("m-week1", "a-backend"),
        edge("a-backend", "a-frontend"),
        edge("a-frontend", "a-reality-1"),
        edge("a-reality-1", "a-growth"),
        edge("a-growth", "a-reality-2"),
        edge("a-reality-2", "s-gonogo"),
        WorkflowEdge {
            id: "e-go".into(),
            source: "s-gonogo".into(),
            source_handle: Some("go".into()),
            target: "a-launch".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
        WorkflowEdge {
            id: "e-nogo".into(),
            source: "s-gonogo".into(),
            source_handle: Some("no-go".into()),
            target: "a-postpone".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
        WorkflowEdge {
            id: "e-launch-end".into(),
            source: "a-launch".into(),
            source_handle: None,
            target: "end".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
        WorkflowEdge {
            id: "e-postpone-end".into(),
            source: "a-postpone".into(),
            source_handle: None,
            target: "end".into(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        },
    ];

    let data = WorkflowTemplateData {
        id: id.into(),
        name: "Startup MVP 构建".into(),
        description: Some("OpenOPC 定义的 7-agent 4周 MVP 工作流：Sprint规划+UX研究(并行)→后端架构→前端开发→质量检查→增长策略→最终质量门→GO/NO-GO决策。含 ParallelNode 并行执行、ConditionNode 质量门、SwitchNode GO/NO-GO 决策。".into()),
        icon: "🏗️".into(),
        cluster_id: None,
        route_path: None,
        tags: vec!["startup".into(),"mvp".into(),"product".into()],
        version: OPC_TEMPLATE_VERSION, is_preset: true, is_editable: true, is_public: false,
        visibility: Visibility::Public,
        trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
        nodes, edges, input_schema: None, output_schema: None, variables: vec![],
        error_config: Some(ErrorConfig {
            retry_policy: Some(WorkflowRetryPolicy { max_retries: 3, base_delay_ms: 1000, max_delay_ms: 30000 }),
            on_failure: OnFailureAction::Abort, error_branch: None, compensation_steps: None,
        }),
        error_workflow_id: None, mission_hash: None, tool_defs: vec![], created_at: now, updated_at: now,
    };
    upsert_template(db, data).await
}
