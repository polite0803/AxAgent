// SPDX-License-Identifier: AGPL-3.0-only

import {
  ApiOutlined,
  AuditOutlined,
  BookOutlined,
  BugOutlined,
  CodeSandboxOutlined,
  CrownOutlined,
  DollarCircleOutlined,
  EditOutlined,
  ExperimentOutlined,
  FileSearchOutlined,
  FileTextOutlined,
  FundProjectionScreenOutlined,
  GlobalOutlined,
  LineChartOutlined,
  RocketOutlined,
  SearchOutlined,
  ShopOutlined,
  SolutionOutlined,
  TagOutlined,
  TrophyOutlined,
  UserOutlined,
  VideoCameraOutlined,
} from "@ant-design/icons";
import { IndustryTabLayout } from "./IndustryTabLayout";
import type { IndustryConfig } from "./types";

// ==========================================
// AI 研究行业页面
// ==========================================
const aiResearchConfig: IndustryConfig = {
  tabs: [
    {
      key: "research",
      label: "研究探索",
      icon: <FileSearchOutlined />,
      description: "论文检索、模型对比、应用分析",
      actions: [
        { key: "ai-paper", icon: <FileSearchOutlined />, type: "conversation", label: "论文调研" },
        { key: "ai-benchmark", icon: <LineChartOutlined />, type: "conversation", label: "性能对比" },
        { key: "ai-app", icon: <ExperimentOutlined />, type: "conversation", label: "场景分析" },
      ],
      workflows: [
        { id: "wf-acd-literature", name: "文献综述", description: "扫描最新 AI 论文", version: "1.0" },
        { id: "wf-acd-research", name: "研究方法论", description: "系统研究框架", version: "1.0" },
      ],
    },
    {
      key: "report",
      label: "成果输出",
      icon: <FileTextOutlined />,
      description: "生成研究报告和洞察",
      actions: [
        { key: "ai-report", icon: <FileTextOutlined />, type: "workflow", label: "生成报告" },
      ],
      workflows: [],
    },
  ],
};

export function AiResearchPage() {
  return <IndustryTabLayout industryId="ai-research" config={aiResearchConfig} />;
}

// ==========================================
// 软件工程行业页面
// ==========================================
const softwareDevConfig: IndustryConfig = {
  tabs: [
    {
      key: "design",
      label: "设计阶段",
      icon: <ApiOutlined />,
      description: "需求分析、架构设计、API 设计",
      actions: [
        { key: "sd-arch", icon: <ApiOutlined />, type: "conversation", label: "架构咨询" },
        { key: "sd-api-doc", icon: <BookOutlined />, type: "workflow", label: "API 文档" },
      ],
      workflows: [
        { id: "wf-eng-api-design", name: "API 设计", description: "RESTful API 设计", version: "1.0" },
        { id: "wf-eng-arch-review", name: "架构评审", description: "系统架构评审", version: "1.0" },
        { id: "wf-prod-spec", name: "需求规格", description: "产品需求文档", version: "1.0" },
      ],
    },
    {
      key: "develop",
      label: "开发阶段",
      icon: <EditOutlined />,
      description: "编码、代码审查、重构",
      actions: [
        { key: "sd-code-review", icon: <AuditOutlined />, type: "conversation", label: "代码审查" },
        { key: "sd-bug", icon: <BugOutlined />, type: "conversation", label: "Bug 修复" },
      ],
      workflows: [
        { id: "wf-eng-code-review", name: "代码审查", description: "自动化代码审查", version: "1.0" },
        { id: "wf-eng-refactor", name: "大规模重构", description: "系统性重构", version: "1.0" },
        { id: "wf-eng-refactor-lite", name: "快速追加重构", description: "增量变更通道", version: "1.0" },
        { id: "wf-eng-tech-debt", name: "技术债分析", description: "识别和管理技术债", version: "1.0" },
      ],
    },
    {
      key: "quality",
      label: "质量保障",
      icon: <SolutionOutlined />,
      description: "安全审查、测试、性能",
      actions: [],
      workflows: [
        { id: "wf-eng-security-review", name: "安全审查", description: "代码安全审计", version: "1.0" },
        { id: "wf-eng-perf-opt", name: "性能优化", description: "性能分析与优化", version: "1.0" },
        { id: "wf-tst-plan", name: "测试计划", description: "测试策略制定", version: "1.0" },
        { id: "wf-tst-automation", name: "自动化测试", description: "测试自动化框架", version: "1.0" },
        { id: "wf-tst-perf", name: "性能测试", description: "负载与压力测试", version: "1.0" },
      ],
    },
    {
      key: "devops",
      label: "DevOps",
      icon: <RocketOutlined />,
      description: "CI/CD、部署、监控",
      actions: [],
      workflows: [
        { id: "wf-eng-ci-setup", name: "CI/CD 配置", description: "持续集成流水线", version: "1.0" },
        { id: "wf-eng-deploy", name: "部署发布", description: "自动化部署", version: "1.0" },
        { id: "wf-eng-monitor-setup", name: "监控告警", description: "系统监控配置", version: "1.0" },
        { id: "wf-eng-db-migrate", name: "数据库迁移", description: "数据库版本管理", version: "1.0" },
      ],
    },
    {
      key: "team",
      label: "团队协作",
      icon: <UserOutlined />,
      description: "团队入职、知识管理",
      actions: [],
      workflows: [
        { id: "wf-eng-onboarding", name: "新成员入职", description: "团队上手流程", version: "1.0" },
      ],
    },
  ],
};

export function SoftwareDevPage() {
  return <IndustryTabLayout industryId="software-dev" config={softwareDevConfig} />;
}

// ==========================================
// 金融投资行业页面
// ==========================================
const financeInvestConfig: IndustryConfig = {
  tabs: [
    {
      key: "analysis",
      label: "市场分析",
      icon: <LineChartOutlined />,
      description: "个股分析、财报解读",
      actions: [
        { key: "fi-stock", icon: <FundProjectionScreenOutlined />, type: "conversation", label: "个股分析" },
        { key: "fi-financial", icon: <FileTextOutlined />, type: "conversation", label: "财报解读" },
      ],
      workflows: [],
    },
    {
      key: "valuation",
      label: "估值建模",
      icon: <EditOutlined />,
      description: "估值计算、财务分析",
      actions: [
        { key: "fi-valuation", icon: <EditOutlined />, type: "workflow", label: "估值计算" },
      ],
      workflows: [
        { id: "wf-fin-cost-analysis", name: "成本分析", description: "成本结构分析", version: "1.0" },
      ],
    },
    {
      key: "risk",
      label: "风险管理",
      icon: <SolutionOutlined />,
      description: "风险评估、投资决策",
      actions: [
        { key: "fi-risk", icon: <SolutionOutlined />, type: "conversation", label: "风险评估" },
      ],
      workflows: [
        { id: "wf-fin-budget", name: "预算规划", description: "投资预算制定", version: "1.0" },
      ],
    },
  ],
};

export function FinanceInvestPage() {
  return <IndustryTabLayout industryId="finance-invest" config={financeInvestConfig} />;
}

// ==========================================
// 销售增长行业页面
// ==========================================
const salesGrowthConfig: IndustryConfig = {
  tabs: [
    {
      key: "lead",
      label: "获客线索",
      icon: <CrownOutlined />,
      description: "潜在客户获取、线索管理",
      actions: [
        { key: "sg-lead", icon: <CrownOutlined />, type: "conversation", label: "线索获取" },
      ],
      workflows: [
        { id: "wf-sal-outbound", name: "外联拓展", description: "主动销售拓展", version: "1.0" },
        { id: "wf-mkt-influencer", name: "KOL 营销", description: "达人合作营销", version: "1.0" },
      ],
    },
    {
      key: "convert",
      label: "转化成交",
      icon: <RocketOutlined />,
      description: "销售漏斗、成交策略",
      actions: [
        { key: "sg-funnel", icon: <RocketOutlined />, type: "conversation", label: "漏斗优化" },
        { key: "sg-copy", icon: <EditOutlined />, type: "workflow", label: "文案撰写" },
      ],
      workflows: [
        { id: "wf-sal-deal-strategy", name: "成交策略", description: "谈判与成交", version: "1.0" },
        { id: "wf-sal-proposal", name: "提案生成", description: "销售提案", version: "1.0" },
        { id: "wf-mkt-ab-test", name: "A/B 测试", description: "营销优化测试", version: "1.0" },
      ],
    },
    {
      key: "manage",
      label: "客户管理",
      icon: <UserOutlined />,
      description: "客户计划、管道管理",
      actions: [
        { key: "sg-competitor", icon: <TrophyOutlined />, type: "conversation", label: "竞品分析" },
      ],
      workflows: [
        { id: "wf-sal-pipeline-review", name: "管道复盘", description: "销售管道审查", version: "1.0" },
        { id: "wf-sal-account-plan", name: "客户计划", description: "关键客户规划", version: "1.0" },
      ],
    },
  ],
};

export function SalesGrowthPage() {
  return <IndustryTabLayout industryId="sales-growth" config={salesGrowthConfig} />;
}

// ==========================================
// 内容媒体行业页面
// ==========================================
const contentMediaConfig: IndustryConfig = {
  tabs: [
    {
      key: "create",
      label: "内容创作",
      icon: <EditOutlined />,
      description: "选题策划、内容生产",
      actions: [
        { key: "cm-writing", icon: <EditOutlined />, type: "conversation", label: "文案写作" },
        { key: "cm-video", icon: <VideoCameraOutlined />, type: "conversation", label: "视频创作" },
      ],
      workflows: [
        { id: "wf-mkt-brand-guide", name: "品牌指南", description: "品牌视觉规范", version: "1.0" },
      ],
    },
    {
      key: "seo",
      label: "SEO 优化",
      icon: <SearchOutlined />,
      description: "搜索引擎优化、关键词",
      actions: [
        { key: "cm-seo", icon: <SearchOutlined />, type: "conversation", label: "SEO 分析" },
      ],
      workflows: [
        { id: "wf-mkt-seo-audit", name: "SEO 审计", description: "网站 SEO 检查", version: "1.0" },
      ],
    },
    {
      key: "distribute",
      label: "分发推广",
      icon: <RocketOutlined />,
      description: "多平台分发、营销活动",
      actions: [
        { key: "cm-calendar", icon: <BookOutlined />, type: "conversation", label: "内容日历" },
      ],
      workflows: [
        { id: "wf-mkt-social-plan", name: "社交计划", description: "社交媒体策略", version: "1.0" },
        { id: "wf-mkt-email-campaign", name: "邮件营销", description: "邮件活动策划", version: "1.0" },
        { id: "wf-mkt-webinar", name: "线上研讨会", description: "Webinar 策划", version: "1.0" },
        { id: "wf-mkt-influencer", name: "KOL 合作", description: "达人营销", version: "1.0" },
      ],
    },
  ],
};

export function ContentMediaPage() {
  return <IndustryTabLayout industryId="content-media" config={contentMediaConfig} />;
}

// ==========================================
// 行业咨询行业页面 (P0 补齐 wf-spc)
// ==========================================
const industryConsultingConfig: IndustryConfig = {
  tabs: [
    {
      key: "strategy",
      label: "战略规划",
      icon: <LineChartOutlined />,
      description: "业务规划、市场进入、竞争分析",
      actions: [
        { key: "ic-market", icon: <LineChartOutlined />, type: "conversation", label: "市场分析" },
        { key: "ic-entry", icon: <RocketOutlined />, type: "conversation", label: "进入策略" },
        { key: "ic-competitor", icon: <TrophyOutlined />, type: "conversation", label: "竞品分析" },
      ],
      workflows: [
        { id: "wf-strat-biz-plan", name: "业务规划", description: "制定业务计划", version: "1.0" },
        { id: "wf-strat-market-entry", name: "市场进入", description: "新市场进入策略", version: "1.0" },
        { id: "wf-mkt-competitive-intel", name: "竞争情报", description: "竞争对手分析", version: "1.0" },
      ],
    },
    {
      key: "compliance",
      label: "合规治理",
      icon: <AuditOutlined />,
      description: "ESG、法务、数据隐私",
      actions: [
        { key: "ic-report", icon: <FileTextOutlined />, type: "workflow", label: "合规报告" },
      ],
      workflows: [
        { id: "wf-spc-esg", name: "ESG 评估", description: "环境社会治理", version: "1.0" },
        { id: "wf-spc-legal-review", name: "法务审查", description: "合同法律审查", version: "1.0" },
        { id: "wf-spc-data-privacy", name: "数据隐私", description: "隐私合规检查", version: "1.0" },
        { id: "wf-sec-compliance", name: "安全合规", description: "安全合规审计", version: "1.0" },
      ],
    },
    {
      key: "capital",
      label: "资本运作",
      icon: <DollarCircleOutlined />,
      description: "并购、融资、政府补贴",
      actions: [],
      workflows: [
        { id: "wf-spc-m-a", name: "并购分析", description: "并购交易分析", version: "1.0" },
        { id: "wf-spc-grant", name: "融资申请", description: "政府补贴/基金申请", version: "1.0" },
      ],
    },
    {
      key: "organization",
      label: "组织发展",
      icon: <UserOutlined />,
      description: "招聘、培训、变革管理",
      actions: [],
      workflows: [
        { id: "wf-spc-hire", name: "人才招聘", description: "招聘流程优化", version: "1.0" },
        { id: "wf-spc-onboard", name: "员工入职", description: "入职培训流程", version: "1.0" },
        { id: "wf-spc-change-mgmt", name: "变革管理", description: "组织变革引导", version: "1.0" },
      ],
    },
    {
      key: "supply",
      label: "供应链",
      icon: <GlobalOutlined />,
      description: "供应链管理、本地化",
      actions: [],
      workflows: [
        { id: "wf-spc-supply-chain", name: "供应链优化", description: "供应链管理", version: "1.0" },
        { id: "wf-spc-localization", name: "本地化适配", description: "区域本地化", version: "1.0" },
      ],
    },
  ],
};

export function IndustryConsultingPage() {
  return <IndustryTabLayout industryId="industry-consulting" config={industryConsultingConfig} />;
}

// ==========================================
// 会计行业页面
// ==========================================
const accountingConfig: IndustryConfig = {
  tabs: [
    {
      key: "finance",
      label: "财务核算",
      icon: <FundProjectionScreenOutlined />,
      description: "账务处理、报表生成",
      actions: [
        { key: "ac-report", icon: <FileTextOutlined />, type: "conversation", label: "报表解读" },
        { key: "ac-cost", icon: <EditOutlined />, type: "conversation", label: "成本核算" },
      ],
      workflows: [
        { id: "wf-fin-cost-analysis", name: "成本分析", description: "成本结构分析", version: "1.0" },
      ],
    },
    {
      key: "tax",
      label: "税务管理",
      icon: <DollarCircleOutlined />,
      description: "税务申报、税务筹划",
      actions: [
        { key: "ac-tax", icon: <DollarCircleOutlined />, type: "conversation", label: "税务咨询" },
      ],
      workflows: [
        { id: "wf-fin-tax", name: "税务计算", description: "税务申报计算", version: "1.0" },
      ],
    },
    {
      key: "budget",
      label: "预算规划",
      icon: <LineChartOutlined />,
      description: "预算制定、财务规划",
      actions: [
        { key: "ac-budget", icon: <FundProjectionScreenOutlined />, type: "workflow", label: "预算编制" },
      ],
      workflows: [
        { id: "wf-fin-budget", name: "预算规划", description: "年度预算制定", version: "1.0" },
      ],
    },
  ],
};

export function AccountingPage() {
  return <IndustryTabLayout industryId="accounting" config={accountingConfig} />;
}

// ==========================================
// 电子商务行业页面
// ==========================================
const ecommerceConfig: IndustryConfig = {
  tabs: [
    {
      key: "product",
      label: "产品管理",
      icon: <SearchOutlined />,
      description: "选品、产品规划、规格",
      actions: [
        { key: "ec-product", icon: <SearchOutlined />, type: "conversation", label: "选品分析" },
      ],
      workflows: [
        { id: "wf-prod-spec", name: "产品规格", description: "产品需求文档", version: "1.0" },
        { id: "wf-prod-launch", name: "产品发布", description: "新产品上市", version: "1.0" },
        { id: "wf-prod-roadmap", name: "产品路线图", description: "产品规划路线", version: "1.0" },
      ],
    },
    {
      key: "pricing",
      label: "价格策略",
      icon: <TagOutlined />,
      description: "定价、促销、折扣",
      actions: [
        { key: "ec-price", icon: <TagOutlined />, type: "conversation", label: "定价策略" },
        { key: "ec-promote", icon: <RocketOutlined />, type: "workflow", label: "促销策划" },
      ],
      workflows: [
        { id: "wf-mkt-pr-plan", name: "定价规划", description: "价格策略制定", version: "1.0" },
      ],
    },
    {
      key: "marketing",
      label: "营销推广",
      icon: <RocketOutlined />,
      description: "营销活动、数据分析",
      actions: [],
      workflows: [
        { id: "wf-mkt-ab-test", name: "A/B 测试", description: "营销优化测试", version: "1.0" },
        { id: "wf-mkt-analytics", name: "营销分析", description: "营销数据分析", version: "1.0" },
        { id: "wf-mkt-email-campaign", name: "邮件营销", description: "邮件活动", version: "1.0" },
      ],
    },
    {
      key: "operation",
      label: "运营管理",
      icon: <ShopOutlined />,
      description: "店铺运营、客户服务",
      actions: [
        { key: "ec-shop", icon: <ShopOutlined />, type: "conversation", label: "店铺管理" },
      ],
      workflows: [
        { id: "wf-sup-ticket", name: "工单处理", description: "客户工单流程", version: "1.0" },
        { id: "wf-spc-supply-chain", name: "供应链管理", description: "电商供应链", version: "1.0" },
      ],
    },
  ],
};

export function EcommercePage() {
  return <IndustryTabLayout industryId="ecommerce" config={ecommerceConfig} />;
}

// ==========================================
// 教育行业页面
// ==========================================
const educationConfig: IndustryConfig = {
  tabs: [
    {
      key: "course",
      label: "课程设计",
      icon: <BookOutlined />,
      description: "课程开发、内容创作",
      actions: [
        { key: "ed-course", icon: <BookOutlined />, type: "workflow", label: "课程设计" },
        { key: "ed-content", icon: <FileTextOutlined />, type: "workflow", label: "教材生成" },
      ],
      workflows: [
        { id: "wf-acd-literature", name: "教学研究", description: "教育文献研究", version: "1.0" },
      ],
    },
    {
      key: "learning",
      label: "学习路径",
      icon: <LineChartOutlined />,
      description: "知识图谱、学习规划",
      actions: [
        { key: "ed-knowledge", icon: <CodeSandboxOutlined />, type: "conversation", label: "知识图谱" },
        { key: "ed-path", icon: <LineChartOutlined />, type: "conversation", label: "学习路径" },
      ],
      workflows: [],
    },
    {
      key: "support",
      label: "学员服务",
      icon: <UserOutlined />,
      description: "FAQ、满意度、支持",
      actions: [],
      workflows: [
        { id: "wf-sup-faq", name: "FAQ 知识库", description: "常见问题解答", version: "1.0" },
        { id: "wf-sup-satisfaction", name: "满意度调查", description: "学员满意度", version: "1.0" },
      ],
    },
  ],
};

export function EducationPage() {
  return <IndustryTabLayout industryId="education" config={educationConfig} />;
}

// ==========================================
// 设计行业页面 (P2 新增 wf-des)
// ==========================================
const designConfig: IndustryConfig = {
  tabs: [
    {
      key: "research",
      label: "用户研究",
      icon: <SearchOutlined />,
      description: "UX 研究、可访问性",
      actions: [],
      workflows: [
        { id: "wf-des-ux-research", name: "UX 研究", description: "用户体验研究", version: "1.0" },
        { id: "wf-des-accessibility", name: "可访问性", description: "无障碍设计检查", version: "1.0" },
      ],
    },
    {
      key: "system",
      label: "设计系统",
      icon: <ApiOutlined />,
      description: "设计系统构建、原型",
      actions: [],
      workflows: [
        { id: "wf-des-design-system", name: "设计系统", description: "设计系统搭建", version: "1.0" },
        { id: "wf-des-prototype", name: "原型设计", description: "快速原型制作", version: "1.0" },
      ],
    },
  ],
};

export function DesignPage() {
  return <IndustryTabLayout industryId="design" config={designConfig} />;
}

// ==========================================
// 项目管理行业页面 (P2 新增 wf-pm)
// ==========================================
const projectManagementConfig: IndustryConfig = {
  tabs: [
    {
      key: "planning",
      label: "项目规划",
      icon: <LineChartOutlined />,
      description: "活动规划、ROI 分析",
      actions: [],
      workflows: [
        { id: "wf-pm-campaign", name: "活动管理", description: "项目活动规划", version: "1.0" },
        { id: "wf-pm-roi", name: "投资回报", description: "项目 ROI 分析", version: "1.0" },
      ],
    },
    {
      key: "execution",
      label: "执行监控",
      icon: <RocketOutlined />,
      description: "敏捷冲刺、状态报告",
      actions: [],
      workflows: [
        { id: "wf-pm-sprint", name: "敏捷冲刺", description: "Sprint 规划与执行", version: "1.0" },
        { id: "wf-pm-status", name: "状态报告", description: "项目状态跟踪", version: "1.0" },
      ],
    },
    {
      key: "risk",
      label: "风险管理",
      icon: <SolutionOutlined />,
      description: "风险识别与应对",
      actions: [],
      workflows: [
        { id: "wf-pm-risk", name: "风险管理", description: "项目风险分析", version: "1.0" },
      ],
    },
  ],
};

export function ProjectManagementPage() {
  return <IndustryTabLayout industryId="project-management" config={projectManagementConfig} />;
}

// ==========================================
// 安全合规行业页面 (P2 新增 wf-sec)
// ==========================================
const securityConfig: IndustryConfig = {
  tabs: [
    {
      key: "prevention",
      label: "安全防护",
      icon: <SolutionOutlined />,
      description: "渗透测试、威胁情报",
      actions: [],
      workflows: [
        { id: "wf-sec-pentest", name: "渗透测试", description: "安全渗透测试", version: "1.0" },
        { id: "wf-sec-threat-intel", name: "威胁情报", description: "威胁情报分析", version: "1.0" },
      ],
    },
    {
      key: "response",
      label: "应急响应",
      icon: <RocketOutlined />,
      description: "事件响应、合规审计",
      actions: [],
      workflows: [
        { id: "wf-sec-incident", name: "事件响应", description: "安全事件处理", version: "1.0" },
      ],
    },
  ],
};

export function SecurityPage() {
  return <IndustryTabLayout industryId="security" config={securityConfig} />;
}

// ==========================================
// 地理信息行业页面 (P4 新增 wf-gis + wf-spatial)
// ==========================================
const geospatialConfig: IndustryConfig = {
  tabs: [
    {
      key: "mapping",
      label: "测绘分析",
      icon: <GlobalOutlined />,
      description: "地图绘制、空间分析",
      actions: [],
      workflows: [
        { id: "wf-gis-mapping", name: "地图绘制", description: "地理数据制图", version: "1.0" },
        { id: "wf-gis-analysis", name: "空间分析", description: "GIS 空间分析", version: "1.0" },
      ],
    },
    {
      key: "collection",
      label: "数据采集",
      icon: <SearchOutlined />,
      description: "无人机测绘、3D 场景",
      actions: [],
      workflows: [
        { id: "wf-gis-drone", name: "无人机测绘", description: "无人机数据采集", version: "1.0" },
        { id: "wf-gis-3d-scene", name: "3D 场景", description: "三维场景建模", version: "1.0" },
      ],
    },
    {
      key: "spatial",
      label: "空间计算",
      icon: <ApiOutlined />,
      description: "AR 应用、空间场景",
      actions: [],
      workflows: [
        { id: "wf-spatial-ar", name: "AR 应用", description: "增强现实应用", version: "1.0" },
        { id: "wf-spatial-scene", name: "空间场景", description: "空间场景设计", version: "1.0" },
      ],
    },
  ],
};

export function GeospatialPage() {
  return <IndustryTabLayout industryId="geospatial" config={geospatialConfig} />;
}

// ==========================================
// 游戏开发行业页面 (P4 新增 wf-gd)
// ==========================================
const gameDevConfig: IndustryConfig = {
  tabs: [
    {
      key: "concept",
      label: "概念设计",
      icon: <EditOutlined />,
      description: "游戏概念、原型",
      actions: [],
      workflows: [
        { id: "wf-gd-concept", name: "游戏概念", description: "游戏创意设计", version: "1.0" },
        { id: "wf-gd-prototype", name: "原型开发", description: "游戏原型实现", version: "1.0" },
      ],
    },
    {
      key: "qa",
      label: "质量保证",
      icon: <BugOutlined />,
      description: "游戏 QA 测试",
      actions: [],
      workflows: [
        { id: "wf-gd-qa", name: "游戏测试", description: "游戏质量测试", version: "1.0" },
      ],
    },
  ],
};

export function GameDevPage() {
  return <IndustryTabLayout industryId="game-dev" config={gameDevConfig} />;
}
