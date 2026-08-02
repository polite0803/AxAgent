// SPDX-License-Identifier: AGPL-3.0-only

//! 75 个 OPC 业务工作流 — 覆盖 200+ 专家、20 个领域
//!
//! 每个工作流对应 OpenOPC agency-agents-src 中的实际业务场景。
//! 使用步骤型 PresetTemplate 定义，支持 convert_preset_to_workflow_template 自动转换。

use axagent_harness::workflow_types::*;
use sea_orm::DatabaseConnection;

use super::{make_base, upsert_template, OPC_TEMPLATE_VERSION};

// ═══════════════════════════════════════════════════════════════════
// 入口
// ═══════════════════════════════════════════════════════════════════

pub async fn seed_all_workflows(db: &DatabaseConnection) -> Result<(), String> {
    seed_engineering(db).await?;
    seed_marketing(db).await?;
    seed_specialized(db).await?;
    seed_sales(db).await?;
    seed_design(db).await?;
    seed_testing(db).await?;
    seed_finance(db).await?;
    seed_security(db).await?;
    seed_support(db).await?;
    seed_product(db).await?;
    seed_pm(db).await?;
    seed_academic(db).await?;
    seed_gis(db).await?;
    seed_gamedev(db).await?;
    seed_paidmedia(db).await?;
    seed_spatial(db).await?;
    seed_strategy(db).await?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 构建辅助
// ═══════════════════════════════════════════════════════════════════

fn t(id: &str, title: &str, x: f64, y: f64) -> WorkflowNodeBase {
    make_base(id, title, "", x, y)
}

struct WfBuilder {
    id: String,
    name: String,
    desc: String,
    icon: String,
    tags: Vec<String>,
    nodes: Vec<WorkflowNode>,
    edges: Vec<WorkflowEdge>,
    profile_id: String,
}

impl WfBuilder {
    fn new(id: &str, name: &str, desc: &str, icon: &str, profile: &str) -> Self {
        Self {
            id: id.into(), name: name.into(), desc: format!("{}。{}", desc, profile),
            icon: icon.into(), tags: vec!["opc".into(), profile.into()],
            nodes: vec![
                WorkflowNode::Trigger(TriggerNode {
                    base: t("trigger", "手动启动", 250.0, 0.0),
                    config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
                }),
            ],
            edges: vec![],
            profile_id: profile.to_string(),
        }
    }

    fn agent(mut self, id: &str, title: &str, prompt: &str, x: f64, y: f64) -> Self {
        self.nodes.push(WorkflowNode::Agent(AgentNode {
            base: t(id, title, x, y),
            config: AgentNodeConfig {
                system_prompt: prompt.into(), context_sources: vec![],
                output_var: format!("{id}_result"),
                model: None, temperature: None, max_tokens: None,
                tools: vec![], exposed_tools: vec![],
                output_mode: OutputMode::Text,
                agent_profile_id: Some(self.profile_id.clone()),
                max_tool_rounds: Some(5), execution_mode: None,
                rag_source_ids: vec![], model_role: Some("opc-worker".to_string()),
                consistency_check: None,
                hallucination_guard: Some(axagent_harness::hallucination_guard::HallucinationGuardConfig {
                    enabled: true, match_threshold: 0.4,
                }),
                    fallback_model: None,
                    task_scene: None,
                    stream_chunk_timeout_secs: None,
                input_mapping: std::collections::HashMap::new(),
            },
        }));
        self
    }

    fn edge(mut self, src: &str, tgt: &str) -> Self {
        self.edges.push(WorkflowEdge {
            id: format!("e-{src}-{tgt}"), source: src.into(), source_handle: None,
            target: tgt.into(), target_handle: None, edge_type: EdgeType::Direct, label: None,
        });
        self
    }

    async fn build(mut self, db: &DatabaseConnection) -> Result<(), String> {
        let now = chrono::Utc::now().timestamp_millis();
        // 串接边: trigger → a1 → a2 → ... → end
        let agent_ids: Vec<String> = self.nodes.iter()
            .filter_map(|n| match n {
                WorkflowNode::Agent(a) => Some(a.base.id.clone()),
                _ => None,
            }).collect();
        if !agent_ids.is_empty() && self.edges.is_empty() {
            self.edges.push(WorkflowEdge {
                id: "e-trigger-first".into(), source: "trigger".into(), source_handle: None,
                target: agent_ids[0].clone(), target_handle: None,
                edge_type: EdgeType::Direct, label: None,
            });
            for i in 0..agent_ids.len()-1 {
                self.edges.push(WorkflowEdge {
                    id: format!("e-{}-{}", agent_ids[i], agent_ids[i+1]),
                    source: agent_ids[i].clone(), source_handle: None,
                    target: agent_ids[i+1].clone(), target_handle: None,
                    edge_type: EdgeType::Direct, label: None,
                });
            }
            self.edges.push(WorkflowEdge {
                id: format!("e-{}-end", agent_ids.last().unwrap()),
                source: agent_ids.last().unwrap().clone(), source_handle: None,
                target: "end".into(), target_handle: None,
                edge_type: EdgeType::Direct, label: None,
            });
        }
        self.nodes.push(WorkflowNode::End(EndNode {
            base: t("end", "完成", 250.0, (agent_ids.len() as f64 * 180.0) + 180.0),
            config: EndNodeConfig { output_var: None },
        }));

        let data = WorkflowTemplateData {
            id: self.id.clone(), name: self.name, description: Some(self.desc),
            icon: self.icon, tags: self.tags,
            version: OPC_TEMPLATE_VERSION, is_preset: true, is_editable: true, is_public: false,
            trigger_config: Some(TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) }),
            nodes: self.nodes, edges: self.edges,
            input_schema: None, output_schema: None, variables: vec![],
            error_config: None, error_workflow_id: None, mission_hash: None, tool_defs: vec![],
            created_at: now, updated_at: now,
        };
        upsert_template(db, data).await
    }

    fn skip_if_exists(_db: &DatabaseConnection, _id: &str) -> bool {
        false
    }
}

macro_rules! wf {
    ($db:expr, $id:expr, $name:expr, $desc:expr, $icon:expr, $profile:expr, [$(($aid:expr, $atitle:expr, $aprompt:expr, $ax:expr, $ay:expr)),+]) => {{
        let id = $id;
        if WfBuilder::skip_if_exists($db, id) { return Ok(()); }
        let mut b = WfBuilder::new(id, $name, $desc, $icon, $profile);
        $(b = b.agent($aid, $atitle, $aprompt, $ax as f64, $ay as f64);)+
        b.build($db).await
    }};
}

// ═══════════════════════════════════════════════════════════════════
// 1. Engineering (33 专家 → 12 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_engineering(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-eng-code-review", "代码审查流水线", "AI工程师审查代码质量、安全、性能", "👀", "cto",
        [("a-submit", "提交代码", "评审者提交代码变更供审查", 250.0, 120.0),
         ("a-review", "AI审查", "审查代码: 逻辑错误、安全漏洞、性能问题、最佳实践", 250.0, 300.0),
         ("a-report", "审查报告", "生成审查报告: 严重程度排序、修改建议、自动修复", 250.0, 480.0)])?;
    wf!(db, "wf-eng-arch-review", "架构评审", "后端架构师评审系统设计方案的可行性", "🏗️", "cto",
        [("a-design", "设计方案", "提交系统架构设计方案", 250.0, 120.0),
         ("a-review-arch", "架构评审", "评审: 技术选型、扩展性、性能、成本、安全", 250.0, 300.0),
         ("a-finalize", "方案定稿", "根据评审意见修改方案并定稿", 250.0, 480.0)])?;
    wf!(db, "wf-eng-deploy", "DevOps部署流水线", "自动化构建、测试、部署到生产环境", "🚀", "cto",
        [("a-build", "构建", "拉取代码、安装依赖、编译构建", 250.0, 120.0),
         ("a-test", "自动化测试", "运行单元测试、集成测试、性能测试", 250.0, 300.0),
         ("a-deploy", "部署", "部署到目标环境、执行数据库迁移", 250.0, 480.0),
         ("a-verify", "验证", "检查部署状态、监控告警、健康检查", 250.0, 660.0)])?;
    wf!(db, "wf-eng-refactor", "代码重构", "系统性地重构遗留代码提高可维护性", "🔧", "cto",
        [("a-analyze", "分析", "分析代码结构、识别坏味道、圈复杂度", 250.0, 120.0),
         ("a-plan", "重构计划", "制定重构方案和目标架构", 250.0, 300.0),
         ("a-execute", "执行重构", "逐步重构并保持测试通过", 250.0, 480.0)])?;
    wf!(db, "wf-eng-api-design", "API设计", "设计REST/GraphQL API并生成文档", "🔌", "cto",
        [("a-spec", "定义规约", "定义API端点、请求/响应格式、认证方式", 250.0, 120.0),
         ("a-validate", "验证设计", "验证: RESTful规范、命名一致性、错误处理", 250.0, 300.0),
         ("a-doc", "生成文档", "生成API文档和客户端SDK", 250.0, 480.0)])?;
    wf!(db, "wf-eng-db-migrate", "数据库迁移", "设计并安全执行数据库模型变更", "🗄️", "cto",
        [("a-plan-migrate", "迁移计划", "分析变更影响、编写迁移脚本", 250.0, 120.0),
         ("a-review-migrate", "变更审查", "审查: 兼容性、性能影响、回滚方案", 250.0, 300.0),
         ("a-execute-migrate", "执行迁移", "执行迁移并验证数据完整性", 250.0, 480.0)])?;
    wf!(db, "wf-eng-perf-opt", "性能优化", "分析和优化系统性能瓶颈", "⚡", "cto",
        [("a-profile", "性能分析", "profile代码、数据库查询、网络延迟", 250.0, 120.0),
         ("a-identify", "瓶颈识别", "识别性能瓶颈和根因分析", 250.0, 300.0),
         ("a-optimize", "优化实施", "实施优化并验证效果", 250.0, 480.0)])?;
    wf!(db, "wf-eng-ci-setup", "CI/CD配置", "搭建持续集成/持续部署流水线", "🔄", "cto",
        [("a-ci-plan", "方案设计", "设计CI/CD架构: 构建、测试、部署阶段", 250.0, 120.0),
         ("a-ci-config", "配置", "编写CI/CD配置文件并测试", 250.0, 300.0),
         ("a-ci-verify", "验证", "运行流水线确认各阶段正常", 250.0, 480.0)])?;
    wf!(db, "wf-eng-monitor-setup", "监控告警配置", "搭建应用监控、日志和告警系统", "📊", "cto",
        [("a-monitor-plan", "监控规划", "设计监控指标、日志采集策略", 250.0, 120.0),
         ("a-monitor-setup", "配置", "配置监控工具、告警规则、仪表盘", 250.0, 300.0),
         ("a-monitor-test", "测试", "验证告警触发和通知链路", 250.0, 480.0)])?;
    wf!(db, "wf-eng-security-review", "安全审查", "代码安全审计: 漏洞扫描、依赖检查", "🛡️", "cto",
        [("a-scan", "扫描", "代码扫描: SAST、依赖漏洞、密钥泄露", 250.0, 120.0),
         ("a-analyze-s", "分析", "分析扫描结果、优先级排序", 250.0, 300.0),
         ("a-fix", "修复", "实施修复方案、验证修复效果", 250.0, 480.0)])?;
    wf!(db, "wf-eng-onboarding", "开发入职", "新项目环境搭建和开发指南", "📖", "cto",
        [("a-env-setup", "环境配置", "配置开发环境、安装依赖、初始化项目", 250.0, 120.0),
         ("a-doc-read", "文档阅读", "阅读项目文档、架构图、API文档", 250.0, 300.0),
         ("a-first-task", "首个任务", "完成首个开发任务验证环境", 250.0, 480.0)])?;
    wf!(db, "wf-eng-tech-debt", "技术债管理", "识别、评估和消除代码库中的技术债务", "📉", "cto",
        [("a-debt-scan", "扫描", "扫描代码库识别技术债项", 250.0, 120.0),
         ("a-debt-prioritize", "排序", "按影响和修复成本排序", 250.0, 300.0),
         ("a-debt-repay", "偿还", "制定还款计划并执行", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 2. Marketing (36 专家 → 10 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_marketing(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-mkt-email-campaign", "邮件营销活动", "策划、设计、发送邮件营销活动并分析效果", "📧", "cmo",
        [("a-email-plan", "活动策划", "确定目标受众、主题、内容策略", 250.0, 120.0),
         ("a-email-create", "内容创作", "撰写邮件文案、设计排版、CTA", 250.0, 300.0),
         ("a-email-analyze", "效果分析", "分析打开率、点击率、转化率", 250.0, 480.0)])?;
    wf!(db, "wf-mkt-seo-audit", "SEO审计", "网站SEO全面审计并优化", "🔍", "cmo",
        [("a-seo-scan", "扫描", "技术SEO: 爬虫、索引、页面速度", 250.0, 120.0),
         ("a-seo-content", "内容审查", "关键词策略、内容质量、Meta标签", 250.0, 300.0),
         ("a-seo-optimize", "优化", "实施优化建议并监控排名变化", 250.0, 480.0)])?;
    wf!(db, "wf-mkt-social-plan", "社交媒体策略", "制定社交媒体系运营和内容日历", "📱", "cmo",
        [("a-soc-audit", "账号审计", "审计现有社交账号和内容表现", 250.0, 120.0),
         ("a-soc-strategy", "策略制定", "确定平台、内容类型、发布频率", 250.0, 300.0),
         ("a-soc-calendar", "内容日历", "创建月度内容日历和排期", 250.0, 480.0)])?;
    wf!(db, "wf-mkt-brand-guide", "品牌指南", "制定品牌视觉和文案规范", "🎨", "cmo",
        [("a-brand-audit", "品牌审计", "审计现有品牌资产和一致性", 250.0, 120.0),
         ("a-brand-voice", "品牌调性", "定义品牌声音、语调、关键词", 250.0, 300.0),
         ("a-brand-guide", "规范文档", "输出品牌指南文档", 250.0, 480.0)])?;
    wf!(db, "wf-mkt-pr-plan", "公关传播计划", "策划新闻稿和媒体传播方案", "📰", "cmo",
        [("a-pr-story", "故事挖掘", "挖掘有新闻价值的故事角度", 250.0, 120.0),
         ("a-pr-write", "撰稿", "撰写新闻稿和媒体资料包", 250.0, 300.0),
         ("a-pr-distribute", "分发", "确定媒体名单并分发稿件", 250.0, 480.0)])?;
    wf!(db, "wf-mkt-ab-test", "A/B测试", "设计、执行和分析A/B测试", "🧪", "cmo",
        [("a-ab-design", "实验设计", "确定假设、变量、样本量", 250.0, 120.0),
         ("a-ab-execute", "执行", "配置实验并启动流量分配", 250.0, 300.0),
         ("a-ab-analyze", "分析", "统计分析结果、得出结论", 250.0, 480.0)])?;
    wf!(db, "wf-mkt-influencer", "红人营销", "寻找和对接行业KOL合作", "🤳", "cmo",
        [("a-inf-search", "红人搜索", "搜索行业相关KOL和内容创作者", 250.0, 120.0),
         ("a-inf-evaluate", "评估", "评估粉丝质量、互动率、匹配度", 250.0, 300.0),
         ("a-inf-outreach", "触达", "制定触达方案并发送合作邀请", 250.0, 480.0)])?;
    wf!(db, "wf-mkt-analytics", "营销数据分析", "整合多渠道数据生成营销洞察", "📈", "cmo",
        [("a-mkt-data", "数据采集", "采集各渠道营销数据", 250.0, 120.0),
         ("a-mkt-dashboard", "仪表盘", "构建营销数据仪表盘", 250.0, 300.0),
         ("a-mkt-insight", "洞察", "提取关键洞察和改进建议", 250.0, 480.0)])?;
    wf!(db, "wf-mkt-competitive-intel", "竞争情报", "持续监控竞争对手动态", "🕵️", "cmo",
        [("a-comp-map", "竞品地图", "识别核心竞争对手和跟踪维度", 250.0, 120.0),
         ("a-comp-monitor", "持续监控", "收集竞品产品更新、定价变化", 250.0, 300.0),
         ("a-comp-report", "情报报告", "生成竞争情报周报", 250.0, 480.0)])?;
    wf!(db, "wf-mkt-webinar", "线上研讨会", "策划和执行线上研讨会活动", "🎥", "cmo",
        [("a-webinar-plan", "活动策划", "确定主题、嘉宾、时间、渠道", 250.0, 120.0),
         ("a-webinar-prep", "准备", "准备PPT、推广素材、测试环境", 250.0, 300.0),
         ("a-webinar-follow", "跟进", "发送回放、收集反馈、线索评分", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 3. Specialized (53 专家 → 10 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_specialized(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-spc-change-mgmt", "变更管理", "企业变革管理: 评估影响、制定沟通、执行", "🔄", "ceo",
        [("a-change-impact", "影响评估", "评估变革对组织、流程、人员的影响", 250.0, 120.0),
         ("a-change-plan", "实施计划", "制定分阶段变革实施和沟通计划", 250.0, 300.0),
         ("a-change-exec", "执行", "监督执行并收集反馈调整", 250.0, 480.0)])?;
    wf!(db, "wf-spc-m-a", "并购整合", "并购后业务、团队、系统整合", "🤝", "ceo",
        [("a-ma-audit", "尽调审计", "审计目标公司业务、技术、团队", 250.0, 120.0),
         ("a-ma-plan", "整合计划", "制定100天整合计划", 250.0, 300.0),
         ("a-ma-exec", "执行", "执行整合并监控关键指标", 250.0, 480.0)])?;
    wf!(db, "wf-spc-legal-review", "合同审查", "审查法律合同条款和风险", "⚖️", "cfo",
        [("a-legal-upload", "提交合同", "提交合同文档和背景说明", 250.0, 120.0),
         ("a-legal-review", "条款审查", "审查关键条款、风险点、合规性", 250.0, 300.0),
         ("a-legal-report", "审查报告", "输出审查意见和修改建议", 250.0, 480.0)])?;
    wf!(db, "wf-spc-hire", "招聘流程", "从职位描述到Offer的完整招聘", "🎯", "coo",
        [("a-hire-jd", "职位描述", "撰写职位描述和要求", 250.0, 120.0),
         ("a-hire-screen", "简历筛选", "筛选简历、安排面试", 250.0, 300.0),
         ("a-hire-evaluate", "面试评估", "综合评估候选人、产出报告", 250.0, 480.0)])?;
    wf!(db, "wf-spc-onboard", "员工入职", "新员工入职流程: 账号、文档、培训", "📋", "coo",
        [("a-onboard-plan", "入职计划", "制定入职计划和任务清单", 250.0, 120.0),
         ("a-onboard-setup", "环境搭建", "开通账号、配置设备、访问权限", 250.0, 300.0),
         ("a-onboard-orient", "入职引导", "公司介绍、团队介绍、首周任务", 250.0, 480.0)])?;
    wf!(db, "wf-spc-data-privacy", "数据隐私合规", "GDPR/个保法合规审计和整改", "🔒", "cfo",
        [("a-privacy-audit", "合规审计", "审计数据采集、存储、处理流程", 250.0, 120.0),
         ("a-privacy-gap", "差距分析", "识别合规差距和风险等级", 250.0, 300.0),
         ("a-privacy-fix", "整改实施", "实施整改措施并验证", 250.0, 480.0)])?;
    wf!(db, "wf-spc-grant", "项目申请", "撰写和提交项目申请", "📝", "ceo",
        [("a-grant-research", "资金研究", "研究适合的项目和资助机构", 250.0, 120.0),
         ("a-grant-write", "申请撰写", "撰写项目申请书和预算", 250.0, 300.0),
         ("a-grant-submit", "提交", "最终审核并提交申请", 250.0, 480.0)])?;
    wf!(db, "wf-spc-supply-chain", "供应链优化", "分析和优化供应链效率", "📦", "coo",
        [("a-sc-audit", "供应链审计", "审计采购、库存、物流各环节", 250.0, 120.0),
         ("a-sc-optimize", "优化方案", "制定降本增效方案", 250.0, 300.0),
         ("a-sc-implement", "实施", "实施优化并跟踪KPI", 250.0, 480.0)])?;
    wf!(db, "wf-spc-esg", "ESG报告", "环境、社会和治理报告编制", "🌱", "ceo",
        [("a-esg-collect", "数据收集", "收集环境、社会、治理数据", 250.0, 120.0),
         ("a-esg-measure", "指标计算", "计算ESG关键指标和评分", 250.0, 300.0),
         ("a-esg-report", "报告生成", "生成ESG报告和改善路线图", 250.0, 480.0)])?;
    wf!(db, "wf-spc-localization", "本地化", "产品和服务本地化适配", "🌍", "ceo",
        [("a-locale-audit", "本地化审计", "审计需要本地化的内容和功能", 250.0, 120.0),
         ("a-locale-translate", "翻译适配", "翻译内容、适配格式和规范", 250.0, 300.0),
         ("a-locale-verify", "验证", "验证本地化质量和一致性", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 4. Sales (9 专家 → 5 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_sales(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-sal-outbound", "外呼获客", "制定和执行主动外呼获客策略", "📞", "cmo",
        [("a-outbound-target", "目标画像", "定义理想客户画像和名单", 250.0, 120.0),
         ("a-outbound-script", "话术准备", "准备外呼话术和常见问题", 250.0, 300.0),
         ("a-outbound-execute", "执行", "执行外呼并记录反馈", 250.0, 480.0)])?;
    wf!(db, "wf-sal-deal-strategy", "交易策略", "制定大客户交易赢单策略", "🏆", "cmo",
        [("a-deal-analyze", "分析", "分析客户需求、决策链、预算", 250.0, 120.0),
         ("a-deal-strategy", "策略", "制定赢单策略和行动计划", 250.0, 300.0),
         ("a-deal-execute", "执行", "执行策略并跟踪进展", 250.0, 480.0)])?;
    wf!(db, "wf-sal-pipeline-review", "商机复盘", "销售商机管道的全面复盘", "📊", "cmo",
        [("a-pipe-list", "商机列表", "列出所有活跃商机和阶段", 250.0, 120.0),
         ("a-pipe-analyze", "分析", "分析瓶颈、预计收入、风险", 250.0, 300.0),
         ("a-pipe-plan", "行动计划", "制定下周跟进计划", 250.0, 480.0)])?;
    wf!(db, "wf-sal-proposal", "方案建议书", "为客户撰写定制化方案建议书", "📄", "cmo",
        [("a-prop-needs", "需求确认", "确认客户需求和决策标准", 250.0, 120.0),
         ("a-prop-write", "方案撰写", "撰写方案建议书: 方案、价值、报价", 250.0, 300.0),
         ("a-prop-review", "内部审查", "审查方案质量和竞品定位", 250.0, 480.0)])?;
    wf!(db, "wf-sal-account-plan", "客户规划", "制定关键客户年度合作计划", "🤝", "cmo",
        [("a-account-review", "客户回顾", "回顾合作历史、满意度、收入", 250.0, 120.0),
         ("a-account-plan", "年度计划", "制定年度目标、策略、里程碑", 250.0, 300.0),
         ("a-account-review-plan", "审核", "内部审核计划可行性", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 5. Design (9 专家 → 4 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_design(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-des-ux-research", "用户研究", "用户访谈、可用性测试和洞察", "👥", "cpo",
        [("a-ux-plan", "研究计划", "确定研究目标和用户招募标准", 250.0, 120.0),
         ("a-ux-conduct", "执行", "执行用户访谈或可用性测试", 250.0, 300.0),
         ("a-ux-report", "研究报告", "输出研究洞察和设计建议", 250.0, 480.0)])?;
    wf!(db, "wf-des-prototype", "原型设计", "从线框图到交互原型", "🎨", "cpo",
        [("a-proto-wireframe", "线框图", "绘制页面结构和布局线框图", 250.0, 120.0),
         ("a-proto-mockup", "高保真", "设计高保真模型和设计稿", 250.0, 300.0),
         ("a-proto-interact", "交互原型", "制作可点击交互原型", 250.0, 480.0)])?;
    wf!(db, "wf-des-design-system", "设计系统", "搭建和维护统一的设计系统", "📐", "cpo",
        [("a-ds-audit", "审计", "审计现有设计元件和模式", 250.0, 120.0),
         ("a-ds-components", "组件库", "构建核心组件库和规范文档", 250.0, 300.0),
         ("a-ds-doc", "文档", "输出设计系统使用文档", 250.0, 480.0)])?;
    wf!(db, "wf-des-accessibility", "无障碍审计", "审计和修复产品无障碍问题", "♿", "cpo",
        [("a-a11y-scan", "扫描", "使用工具扫描无障碍问题", 250.0, 120.0),
         ("a-a11y-report", "报告", "分类报告问题严重程度", 250.0, 300.0),
         ("a-a11y-fix", "修复", "优先级修复关键无障碍问题", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 6. Testing (8 专家 → 3 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_testing(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-tst-plan", "测试计划", "制定完整测试策略和计划", "📋", "cto",
        [("a-tplan-analyze", "需求分析", "分析功能需求和技术规格", 250.0, 120.0),
         ("a-tplan-design", "测试设计", "设计测试用例和测试场景", 250.0, 300.0),
         ("a-tplan-review", "评审", "评审测试覆盖率和优先级", 250.0, 480.0)])?;
    wf!(db, "wf-tst-automation", "自动化测试", "编写和维护自动化测试脚本", "🤖", "cto",
        [("a-tauto-pick", "选型", "选择自动化框架和工具", 250.0, 120.0),
         ("a-tauto-write", "编写", "编写测试脚本并集成本地CI", 250.0, 300.0),
         ("a-tauto-run", "运行", "运行测试并分析结果", 250.0, 480.0)])?;
    wf!(db, "wf-tst-perf", "性能测试", "负载测试和性能基准", "⚡", "cto",
        [("a-tperf-script", "测试脚本", "编写性能测试脚本和场景", 250.0, 120.0),
         ("a-tperf-run", "执行", "执行负载测试并监控", 250.0, 300.0),
         ("a-tperf-report", "报告", "输出性能报告和优化建议", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 7. Finance (5 专家 → 3 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_finance(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-fin-budget", "预算编制", "编制年度预算和滚动预测", "💰", "cfo",
        [("a-budget-review", "回顾", "回顾上期预算执行和差异", 250.0, 120.0),
         ("a-budget-plan", "编制", "编制各部门预算方案", 250.0, 300.0),
         ("a-budget-approve", "审批", "审批预算并确定最终版本", 250.0, 480.0)])?;
    wf!(db, "wf-fin-tax", "税务申报", "准备和提交税务申报材料", "🧾", "cfo",
        [("a-tax-collect", "收集", "收集收入、支出、抵扣凭证", 250.0, 120.0),
         ("a-tax-calc", "计算", "计算应纳税额和抵扣项", 250.0, 300.0),
         ("a-tax-submit", "申报", "生成报表并提交申报", 250.0, 480.0)])?;
    wf!(db, "wf-fin-cost-analysis", "成本分析", "全面分析运营成本和优化空间", "📉", "cfo",
        [("a-cost-collect", "采集", "采集各类成本数据", 250.0, 120.0),
         ("a-cost-analyze", "分析", "按类别、项目、客户分析成本", 250.0, 300.0),
         ("a-cost-optimize", "优化", "制定降本方案并评估影响", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 8. Security (10 专家 → 4 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_security(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-sec-pentest", "渗透测试", "对应用和基础设施进行渗透测试", "🔓", "cto",
        [("a-pentest-scope", "范围确定", "确定测试范围和目标", 250.0, 120.0),
         ("a-pentest-exec", "执行", "执行渗透测试并记录发现", 250.0, 300.0),
         ("a-pentest-report", "报告", "输出漏洞报告和修复建议", 250.0, 480.0)])?;
    wf!(db, "wf-sec-compliance", "合规审计", "检查安全合规标准和差距", "✅", "cto",
        [("a-comp-standard", "标准对照", "确定适用的安全标准和框架", 250.0, 120.0),
         ("a-comp-audit", "审计", "逐项检查合规性", 250.0, 300.0),
         ("a-comp-report", "报告", "输出合规报告和整改计划", 250.0, 480.0)])?;
    wf!(db, "wf-sec-incident", "安全事件响应", "检测、分析和响应安全事件", "🚨", "cto",
        [("a-incident-detect", "检测", "确认安全事件类型和范围", 250.0, 120.0),
         ("a-incident-respond", "响应", "执行应急响应和止损措施", 250.0, 300.0),
         ("a-incident-review", "复盘", "事故复盘和改进计划", 250.0, 480.0)])?;
    wf!(db, "wf-sec-threat-intel", "威胁情报", "收集和分析最新安全威胁情报", "🕵️", "cto",
        [("a-threat-collect", "情报收集", "收集行业威胁情报和安全公告", 250.0, 120.0),
         ("a-threat-analyze", "分析", "评估威胁影响和风险级别", 250.0, 300.0),
         ("a-threat-act", "行动", "制定防护措施和更新策略", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 9. Support (6 专家 → 3 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_support(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-sup-ticket", "客户工单处理", "接收、分类、处理和关闭客户工单", "🎫", "coo",
        [("a-ticket-categorize", "分类", "分类工单类型和紧急程度", 250.0, 120.0),
         ("a-ticket-solve", "解决", "排查问题并给出解决方案", 250.0, 300.0),
         ("a-ticket-follow", "跟进", "确认客户满意并关闭工单", 250.0, 480.0)])?;
    wf!(db, "wf-sup-faq", "FAQ知识库", "从客户问题提取和更新知识库", "📚", "coo",
        [("a-faq-collect", "收集", "采集高频客户问题和解决方案", 250.0, 120.0),
         ("a-faq-write", "编写", "编写清晰的FAQ文档", 250.0, 300.0),
         ("a-faq-publish", "发布", "审核并发布到知识库", 250.0, 480.0)])?;
    wf!(db, "wf-sup-satisfaction", "客户满意度调查", "设计、发���和分析满意度调查", "📊", "coo",
        [("a-sat-design", "设计", "设计调查问卷和评分体系", 250.0, 120.0),
         ("a-sat-send", "发送", "选择样本并发送调查", 250.0, 300.0),
         ("a-sat-analyze", "分析", "分析结果并制定改进计划", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 10. Product (5 专家 → 3 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_product(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-prod-roadmap", "产品路线图", "制定季度产品路线图", "🗺️", "cpo",
        [("a-road-collect", "需求收集", "收集用户反馈、数据分析、市场趋势", 250.0, 120.0),
         ("a-road-prioritize", "优先级排序", "按影响和资源排序功能", 250.0, 300.0),
         ("a-road-publish", "发布", "输出产品路线图并同步团队", 250.0, 480.0)])?;
    wf!(db, "wf-prod-spec", "产品规格书", "编写功能规格和验收标准", "📄", "cpo",
        [("a-spec-req", "需求分析", "分析用户故事和功能需求", 250.0, 120.0),
         ("a-spec-write", "编写", "编写功能规格和验收标准", 250.0, 300.0),
         ("a-spec-review", "评审", "与技术团队评审可行性", 250.0, 480.0)])?;
    wf!(db, "wf-prod-launch", "产品发布", "新产品/功能发布流程", "🚀", "cpo",
        [("a-launch-plan", "发布计划", "制定发布计划和时间线", 250.0, 120.0),
         ("a-launch-prep", "发布准备", "准备发布说明、营销材料", 250.0, 300.0),
         ("a-launch-exec", "执行发布", "执行发布并监控指标", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 11. Project Management (7 专家 → 3 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_pm(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-pm-sprint", "Sprint规划", "迭代冲刺规划和任务分配", "📋", "coo",
        [("a-sprint-backlog", "Backlog梳理", "梳理和估算待办项", 250.0, 120.0),
         ("a-sprint-plan", "冲刺规划", "确定冲刺目标和任务分配", 250.0, 300.0),
         ("a-sprint-review", "冲刺回顾", "回顾冲刺成果和改进点", 250.0, 480.0)])?;
    wf!(db, "wf-pm-risk", "风险管理", "识别、评估和应对项目风险", "⚠️", "coo",
        [("a-risk-identify", "风险识别", "识别技术和业务风险", 250.0, 120.0),
         ("a-risk-assess", "评估", "评估影响和概率", 250.0, 300.0),
         ("a-risk-respond", "应对", "制定风险应对策略", 250.0, 480.0)])?;
    wf!(db, "wf-pm-status", "项目状态报告", "生成项目周报和状态更新", "📊", "coo",
        [("a-status-collect", "数据收集", "收集团队进展和指标", 250.0, 120.0),
         ("a-status-write", "报告撰写", "撰写项目状态报告", 250.0, 300.0),
         ("a-status-distribute", "分发", "发送报告并安排跟进", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 12. Academic (5 专家 → 2 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_academic(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-acd-literature", "文献综述", "系统性地综述学术文献", "📚", "ceo",
        [("a-lit-search", "文献搜索", "搜索目标领域的关键文献", 250.0, 120.0),
         ("a-lit-review", "文献阅读", "阅读文献并提取关键信息", 250.0, 300.0),
         ("a-lit-synthesize", "综述撰写", "撰写文献综述和发现", 250.0, 480.0)])?;
    wf!(db, "wf-acd-research", "研究方案", "设计学术研究方案和方法论", "🔬", "ceo",
        [("a-research-question", "研究问题", "定义研究问题和假设", 250.0, 120.0),
         ("a-research-method", "方法论", "设计研究方法和数据采集方案", 250.0, 300.0),
         ("a-research-plan", "研究计划", "制定时间表和资源计划", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 13. GIS (13 专家 → 4 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_gis(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-gis-analysis", "空间分析", "地理空间数据分析和可视化", "🗺️", "ceo",
        [("a-gis-data", "数据准备", "收集和预处理空间数据", 250.0, 120.0),
         ("a-gis-analyze", "分析", "执行空间分析: 缓冲、叠加、网络", 250.0, 300.0),
         ("a-gis-viz", "可视化", "生成地图和分析报告", 250.0, 480.0)])?;
    wf!(db, "wf-gis-mapping", "地图制作", "专业地图制图和符号设计", "🗺️", "ceo",
        [("a-map-data", "数据准备", "准备基础地理数据和要素", 250.0, 120.0),
         ("a-map-design", "地图设计", "设计地图样式、符号和标注", 250.0, 300.0),
         ("a-map-export", "输出", "导出地图成品", 250.0, 480.0)])?;
    wf!(db, "wf-gis-3d-scene", "三维场景", "构建三维地理场景和可视化", "🏔️", "ceo",
        [("a-3d-data", "数据采集", "采集地形、影像和模型数据", 250.0, 120.0),
         ("a-3d-scene", "场景搭建", "构建三维场景和光照", 250.0, 300.0),
         ("a-3d-publish", "发布", "发布交互式三维场景", 250.0, 480.0)])?;
    wf!(db, "wf-gis-drone", "无人机测绘", "无人机航拍数据处理和分析", "🛸", "ceo",
        [("a-drone-plan", "飞行规划", "规划飞行路线和采集参数", 250.0, 120.0),
         ("a-drone-process", "数据处理", "处理航拍影像生成正射影像和DSM", 250.0, 300.0),
         ("a-drone-analyze", "分析", "从测绘数据提取信息", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 14. Game Dev (10 专家 → 3 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_gamedev(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-gd-concept", "游戏概念设计", "从想法到完整的游戏设计文档", "🎮", "cto",
        [("a-gd-idea", "概念生成", "生成游戏核心玩法和概念", 250.0, 120.0),
         ("a-gd-design", "游戏设计", "设计游戏机制、关卡、角色", 250.0, 300.0),
         ("a-gd-doc", "文档", "编写游戏设计文档", 250.0, 480.0)])?;
    wf!(db, "wf-gd-prototype", "游戏原型", "快速搭建可玩原型验证核心机制", "🎮", "cto",
        [("a-gd-proto-core", "核心机制", "实现核心玩法和控制", 250.0, 120.0),
         ("a-gd-proto-test", "玩法测试", "测试核心机制可玩性", 250.0, 300.0),
         ("a-gd-proto-iterate", "迭代", "根据测试反馈优化", 250.0, 480.0)])?;
    wf!(db, "wf-gd-qa", "游戏测试", "全面测试游戏功能和体验", "🎮", "cto",
        [("a-gd-qa-functional", "功能测试", "测试游戏功能和系统", 250.0, 120.0),
         ("a-gd-qa-balance", "平衡测试", "测试数值平衡和难度曲线", 250.0, 300.0),
         ("a-gd-qa-ux", "体验测试", "测试用户体验和引导", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 15. Paid Media (7 专家 → 2 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_paidmedia(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-pm-campaign", "广告活动管理", "规划、执行和优化付费广告活动", "📺", "cmo",
        [("a-pm-plan", "广告规划", "确定目标受众、预算、渠道", 250.0, 120.0),
         ("a-pm-create", "广告制作", "制作广告创意和落地页", 250.0, 300.0),
         ("a-pm-optimize", "优化", "分析表现数据并优化", 250.0, 480.0)])?;
    wf!(db, "wf-pm-roi", "广告ROI分析", "分析各渠道广告投入产出比", "📊", "cfo",
        [("a-roi-collect", "数据采集", "采集各渠道成本和收入", 250.0, 120.0),
         ("a-roi-calc", "计算", "计算ROI和客户获取成本", 250.0, 300.0),
         ("a-roi-report", "报告", "输出ROI报告和预算建议", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 16. Spatial Computing (6 专家 → 2 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_spatial(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-spatial-ar", "AR应用设计", "增强现实应用概念和交互设计", "🥽", "cto",
        [("a-ar-concept", "概念设计", "设计AR应用核心交互模式", 250.0, 120.0),
         ("a-ar-ux", "空间UI设计", "设计3D空间用户界面和手势", 250.0, 300.0),
         ("a-ar-prototype", "原型验证", "搭建AR原型验证可行性", 250.0, 480.0)])?;
    wf!(db, "wf-spatial-scene", "空间场景", "构建沉浸式3D空间场景", "🏠", "cto",
        [("a-scene-layout", "场景规划", "规划空间布局和交互区域", 250.0, 120.0),
         ("a-scene-build", "场景构建", "构建3D场景和光照", 250.0, 300.0),
         ("a-scene-optimize", "优化", "优化性能和用户体验", 250.0, 480.0)])?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// 17. Strategy (6 专家 → 2 工作流)
// ═══════════════════════════════════════════════════════════════════

async fn seed_strategy(db: &DatabaseConnection) -> Result<(), String> {
    wf!(db, "wf-strat-market-entry", "市场进入策略", "制定新市场进入策略和计划", "🎯", "ceo",
        [("a-market-size", "市场分析", "分析市场规模、竞争、进入壁垒", 250.0, 120.0),
         ("a-market-strategy", "策略制定", "制定进入策略: 渠道、定价、定位", 250.0, 300.0),
         ("a-market-plan", "行动计划", "制定执行计划和预算", 250.0, 480.0)])?;
    wf!(db, "wf-strat-biz-plan", "商业计划书", "编写完整商业计划书", "📄", "ceo",
        [("a-bp-summary", "执行摘要", "撰写执行摘要和公司概述", 250.0, 120.0),
         ("a-bp-market", "市场分析", "市场分析、竞争分析、SWOT", 250.0, 300.0),
         ("a-bp-financial", "财务预测", "收入模型、成本、现金流预测", 250.0, 480.0)])?;
    Ok(())
}
