// SPDX-License-Identifier: AGPL-3.0-only

import { IndustryHub } from "@/components/opc/IndustryHub";
import i18n from "@/i18n";
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
import type { IndustryConfig } from "./types";

const t = i18n.t;

// ==========================================
// 各行业 inputFields 配置
// ==========================================

const SOFTWARE_DEV_INPUT_FIELDS = [
  {
    key: "project_name",
    label: "opc.input_fields.software_dev.project_name.label",
    type: "string" as const,
    required: true,
  },
  { key: "project_goal", label: "opc.input_fields.software_dev.project_goal.label", type: "string" as const },
  { key: "tech_stack", label: "opc.input_fields.software_dev.tech_stack.label", type: "string" as const },
];

// 代码重构：大规模重构
const REFACTOR_FULL_INPUT_FIELDS = [
  {
    key: "project_name",
    label: "opc.input_fields.software_dev.project_name.label",
    type: "string" as const,
    required: true,
  },
  {
    key: "codebase_path",
    label: "opc.refactor.fields.codebase_path.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.refactor.fields.codebase_path.placeholder",
  },
  {
    key: "refactor_goal",
    label: "opc.refactor.fields.refactor_goal.label",
    type: "textarea" as const,
    required: true,
    placeholder: "opc.refactor.fields.refactor_goal.placeholder",
  },
  {
    key: "target_modules",
    label: "opc.refactor.fields.target_modules.label",
    type: "string" as const,
    placeholder: "opc.refactor.fields.target_modules.placeholder",
  },
  {
    key: "risk_level",
    label: "opc.refactor.fields.risk_level.label",
    type: "string" as const,
    placeholder: "opc.refactor.fields.risk_level.placeholder",
  },
  {
    key: "test_coverage_min",
    label: "opc.refactor.fields.test_coverage_min.label",
    type: "number" as const,
  },
];

// 代码重构：快速追加（轻量）
const REFACTOR_LITE_INPUT_FIELDS = [
  {
    key: "project_name",
    label: "opc.input_fields.software_dev.project_name.label",
    type: "string" as const,
    required: true,
  },
  {
    key: "codebase_path",
    label: "opc.refactor.fields.codebase_path.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.refactor.fields.codebase_path.placeholder",
  },
  {
    key: "refactor_goal",
    label: "opc.refactor.fields.refactor_goal.label",
    type: "textarea" as const,
    required: true,
    placeholder: "opc.refactor.fields.refactor_goal.placeholder",
  },
  {
    key: "target_modules",
    label: "opc.refactor.fields.target_modules.label",
    type: "string" as const,
    placeholder: "opc.refactor.fields.target_modules.placeholder",
  },
];

// 技术债分析
const TECH_DEBT_INPUT_FIELDS = [
  {
    key: "project_name",
    label: "opc.input_fields.software_dev.project_name.label",
    type: "string" as const,
    required: true,
  },
  {
    key: "codebase_path",
    label: "opc.refactor.fields.codebase_path.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.refactor.fields.codebase_path.placeholder",
  },
  {
    key: "tech_debt_categories",
    label: "opc.refactor.fields.tech_debt_categories.label",
    type: "string" as const,
    placeholder: "opc.refactor.fields.tech_debt_categories.placeholder",
  },
  {
    key: "severity_threshold",
    label: "opc.refactor.fields.severity_threshold.label",
    type: "string" as const,
    placeholder: "opc.refactor.fields.severity_threshold.placeholder",
  },
  {
    key: "estimated_hours",
    label: "opc.refactor.fields.estimated_hours.label",
    type: "number" as const,
  },
];

const FINANCE_INVEST_INPUT_FIELDS = [
  {
    key: "portfolio_value",
    label: "opc.input_fields.finance_invest.portfolio_value.label",
    type: "number" as const,
  },
  {
    key: "market_focus",
    label: "opc.input_fields.finance_invest.market_focus.label",
    type: "string" as const,
  },
  {
    key: "risk_tolerance",
    label: "opc.input_fields.finance_invest.risk_tolerance.label",
    type: "string" as const,
  },
];

const SALES_GROWTH_INPUT_FIELDS = [
  {
    key: "target_segment",
    label: "opc.input_fields.sales_growth.target_segment.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.input_fields.sales_growth.target_segment.placeholder",
  },
  {
    key: "product",
    label: "opc.input_fields.sales_growth.product.label",
    type: "string" as const,
    placeholder: "opc.input_fields.sales_growth.product.placeholder",
  },
  { key: "growth_goal", label: "opc.input_fields.sales_growth.growth_goal.label", type: "string" as const },
];

const CONTENT_MEDIA_INPUT_FIELDS = [
  {
    key: "content_topic",
    label: "opc.input_fields.content_media.content_topic.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.input_fields.content_media.content_topic.placeholder",
  },
  {
    key: "target_platform",
    label: "opc.input_fields.content_media.target_platform.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.input_fields.content_media.target_platform.placeholder",
  },
];

const LITERARY_CREATION_INPUT_FIELDS = [
  {
    key: "genre",
    label: "opc.input_fields.content_media.literary_genre.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.input_fields.content_media.literary_genre.placeholder",
  },
  {
    key: "topic",
    label: "opc.input_fields.content_media.literary_topic.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.input_fields.content_media.literary_topic.placeholder",
  },
  {
    key: "word_count_target",
    label: "opc.input_fields.content_media.word_count_target.label",
    type: "number" as const,
    placeholder: "opc.input_fields.content_media.word_count_target.placeholder",
  },
];

const INDUSTRY_CONSULTING_INPUT_FIELDS = [
  {
    key: "industry_name",
    label: "opc.input_fields.industry_consulting.industry_name.label",
    type: "string" as const,
    required: true,
  },
  {
    key: "client_goal",
    label: "opc.input_fields.industry_consulting.client_goal.label",
    type: "string" as const,
  },
  { key: "region", label: "opc.input_fields.industry_consulting.region.label", type: "string" as const },
];

const ACCOUNTING_INPUT_FIELDS = [
  {
    key: "company_name",
    label: "opc.input_fields.accounting.company_name.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.input_fields.accounting.company_name.placeholder",
  },
  {
    key: "period",
    label: "opc.input_fields.accounting.period.label",
    type: "string" as const,
    placeholder: "opc.input_fields.accounting.period.placeholder",
  },
  {
    key: "focus_area",
    label: "opc.input_fields.accounting.focus_area.label",
    type: "string" as const,
    placeholder: "opc.input_fields.accounting.focus_area.placeholder",
  },
];

const ECOMMERCE_INPUT_FIELDS = [
  {
    key: "product_name",
    label: "opc.input_fields.ecommerce.product_name.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.input_fields.ecommerce.product_name.placeholder",
  },
  {
    key: "target_market",
    label: "opc.input_fields.ecommerce.target_market.label",
    type: "string" as const,
    placeholder: "opc.input_fields.ecommerce.target_market.placeholder",
  },
  {
    key: "promo_budget",
    label: "opc.input_fields.ecommerce.promo_budget.label",
    type: "number" as const,
    placeholder: "opc.input_fields.ecommerce.promo_budget.placeholder",
  },
];

const EDUCATION_INPUT_FIELDS = [
  {
    key: "course_topic",
    label: "opc.input_fields.education.course_topic.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.input_fields.education.course_topic.placeholder",
  },
  {
    key: "target_audience",
    label: "opc.input_fields.education.target_audience.label",
    type: "string" as const,
    placeholder: "opc.input_fields.education.target_audience.placeholder",
  },
  {
    key: "course_level",
    label: "opc.input_fields.education.course_level.label",
    type: "string" as const,
    placeholder: "opc.input_fields.education.course_level.placeholder",
  },
];

const DESIGN_INPUT_FIELDS = [
  {
    key: "project_brief",
    label: "opc.input_fields.design.project_brief.label",
    type: "textarea" as const,
    required: true,
    placeholder: "opc.input_fields.design.project_brief.placeholder",
  },
  {
    key: "brand_style",
    label: "opc.input_fields.design.brand_style.label",
    type: "string" as const,
    placeholder: "opc.input_fields.design.brand_style.placeholder",
  },
  {
    key: "design_target",
    label: "opc.input_fields.design.design_target.label",
    type: "string" as const,
    placeholder: "opc.input_fields.design.design_target.placeholder",
  },
];

const PROJECT_MANAGEMENT_INPUT_FIELDS = [
  {
    key: "project_name",
    label: "opc.input_fields.project_management.project_name.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.input_fields.project_management.project_name.placeholder",
  },
  {
    key: "project_scope",
    label: "opc.input_fields.project_management.project_scope.label",
    type: "textarea" as const,
  },
  { key: "deadline", label: "opc.input_fields.project_management.deadline.label", type: "string" as const },
];

const SECURITY_INPUT_FIELDS = [
  {
    key: "scope",
    label: "opc.input_fields.security.scope.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.input_fields.security.scope.placeholder",
  },
  {
    key: "compliance_standard",
    label: "opc.input_fields.security.compliance_standard.label",
    type: "string" as const,
    placeholder: "opc.input_fields.security.compliance_standard.placeholder",
  },
  { key: "incident_type", label: "opc.input_fields.security.incident_type.label", type: "string" as const },
];

const GEOSPATIAL_INPUT_FIELDS = [
  {
    key: "region",
    label: "opc.input_fields.geospatial.region.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.input_fields.geospatial.region.placeholder",
  },
  {
    key: "data_type",
    label: "opc.input_fields.geospatial.data_type.label",
    type: "string" as const,
    placeholder: "opc.input_fields.geospatial.data_type.placeholder",
  },
  {
    key: "output_format",
    label: "opc.input_fields.geospatial.output_format.label",
    type: "string" as const,
    placeholder: "opc.input_fields.geospatial.output_format.placeholder",
  },
];

const AI_RESEARCH_INPUT_FIELDS = [
  {
    key: "research_topic",
    label: "opc.input_fields.ai_research.research_topic.label",
    type: "string" as const,
    required: true,
    placeholder: "opc.input_fields.ai_research.research_topic.placeholder",
  },
  {
    key: "scope",
    label: "opc.input_fields.ai_research.scope.label",
    type: "string" as const,
    placeholder: "opc.input_fields.ai_research.scope.placeholder",
  },
];

// ==========================================
// AI 研究行业页面
// ==========================================
const aiResearchConfig: IndustryConfig = {
  tabs: [
    {
      key: "research",
      label: i18n.t("opc.industry.ai_research.tabs.research.label"),
      icon: <FileSearchOutlined />,
      description: i18n.t("opc.industry.ai_research.tabs.research.description"),
      actions: [
        {
          key: "ai-paper",
          icon: <FileSearchOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.ai_research.tabs.research.actions.ai_paper"),
        },
        {
          key: "ai-benchmark",
          icon: <LineChartOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.ai_research.tabs.research.actions.ai_benchmark"),
        },
        {
          key: "ai-app",
          icon: <ExperimentOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.ai_research.tabs.research.actions.ai_app"),
        },
      ],
      workflows: [
        {
          id: "ai_research_harness_workflow",
          name: i18n.t("opc.industry.ai_research.tabs.research.workflows.wf_acd_literature.name"),
          description: i18n.t("opc.industry.ai_research.tabs.research.workflows.wf_acd_literature.description"),
          version: "1.0",
          inputFields: AI_RESEARCH_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "report",
      label: i18n.t("opc.industry.ai_research.tabs.report.label"),
      icon: <FileTextOutlined />,
      description: i18n.t("opc.industry.ai_research.tabs.report.description"),
      actions: [
        {
          key: "ai-report",
          icon: <FileTextOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.ai_research.tabs.report.actions.ai_report"),
        },
      ],
      workflows: [],
    },
  ],
};

export function AiResearchPage() {
  return (
    <IndustryHub
      industryId="ai-research"
      config={aiResearchConfig}
      industryTitle={t(`opc.industries.ai_research`)}
      industryIcon={<ExperimentOutlined />}
    />
  );
}

// ==========================================
// 软件工程行业页面
// ==========================================
const softwareDevConfig: IndustryConfig = {
  tabs: [
    {
      key: "design",
      label: i18n.t("opc.industry.software_dev.tabs.design.label"),
      icon: <ApiOutlined />,
      description: i18n.t("opc.industry.software_dev.tabs.design.description"),
      actions: [
        {
          key: "sd-arch",
          icon: <ApiOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.software_dev.tabs.design.actions.sd_arch"),
        },
        {
          key: "sd-api-doc",
          icon: <BookOutlined />,
          type: "workflow",
          template_id: "software_dev_harness_workflow",
          label: i18n.t("opc.industry.software_dev.tabs.design.actions.sd_api_doc"),
        },
      ],
      workflows: [
        {
          id: "wf-eng-api-design",
          name: i18n.t("opc.industry.software_dev.tabs.design.workflows.wf_eng_api_design.name"),
          description: i18n.t("opc.industry.software_dev.tabs.design.workflows.wf_eng_api_design.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
        {
          id: "wf-eng-arch-review",
          name: i18n.t("opc.industry.software_dev.tabs.design.workflows.wf_eng_arch_review.name"),
          description: i18n.t("opc.industry.software_dev.tabs.design.workflows.wf_eng_arch_review.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
        {
          id: "wf-prod-spec",
          name: i18n.t("opc.industry.software_dev.tabs.design.workflows.wf_prod_spec.name"),
          description: i18n.t("opc.industry.software_dev.tabs.design.workflows.wf_prod_spec.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "develop",
      label: i18n.t("opc.industry.software_dev.tabs.develop.label"),
      icon: <EditOutlined />,
      description: i18n.t("opc.industry.software_dev.tabs.develop.description"),
      actions: [
        {
          key: "sd-code-review",
          icon: <AuditOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.software_dev.tabs.develop.actions.sd_code_review"),
        },
        {
          key: "sd-bug",
          icon: <BugOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.software_dev.tabs.develop.actions.sd_bug"),
        },
      ],
      workflows: [
        {
          id: "wf-eng-code-review",
          name: i18n.t("opc.industry.software_dev.tabs.develop.workflows.wf_eng_code_review.name"),
          description: i18n.t("opc.industry.software_dev.tabs.develop.workflows.wf_eng_code_review.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
        {
          id: "wf-eng-refactor",
          name: i18n.t("opc.industry.software_dev.tabs.develop.workflows.wf_eng_refactor.name"),
          description: i18n.t("opc.industry.software_dev.tabs.develop.workflows.wf_eng_refactor.description"),
          version: "2.0",
          inputFields: REFACTOR_FULL_INPUT_FIELDS,
        },
        {
          id: "wf-eng-refactor-lite",
          name: i18n.t("opc.industry.software_dev.tabs.develop.workflows.wf_eng_refactor_lite.name"),
          description: i18n.t("opc.industry.software_dev.tabs.develop.workflows.wf_eng_refactor_lite.description"),
          version: "2.0",
          inputFields: REFACTOR_LITE_INPUT_FIELDS,
        },
        {
          id: "wf-eng-tech-debt",
          name: i18n.t("opc.industry.software_dev.tabs.develop.workflows.wf_eng_tech_debt.name"),
          description: i18n.t("opc.industry.software_dev.tabs.develop.workflows.wf_eng_tech_debt.description"),
          version: "2.0",
          inputFields: TECH_DEBT_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "quality",
      label: i18n.t("opc.industry.software_dev.tabs.quality.label"),
      icon: <SolutionOutlined />,
      description: i18n.t("opc.industry.software_dev.tabs.quality.description"),
      actions: [],
      workflows: [
        {
          id: "wf-eng-security-review",
          name: i18n.t("opc.industry.software_dev.tabs.quality.workflows.wf_eng_security_review.name"),
          description: i18n.t("opc.industry.software_dev.tabs.quality.workflows.wf_eng_security_review.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
        {
          id: "wf-eng-perf-opt",
          name: i18n.t("opc.industry.software_dev.tabs.quality.workflows.wf_eng_perf_opt.name"),
          description: i18n.t("opc.industry.software_dev.tabs.quality.workflows.wf_eng_perf_opt.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
        {
          id: "wf-tst-plan",
          name: i18n.t("opc.industry.software_dev.tabs.quality.workflows.wf_tst_plan.name"),
          description: i18n.t("opc.industry.software_dev.tabs.quality.workflows.wf_tst_plan.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
        {
          id: "wf-tst-automation",
          name: i18n.t("opc.industry.software_dev.tabs.quality.workflows.wf_tst_automation.name"),
          description: i18n.t("opc.industry.software_dev.tabs.quality.workflows.wf_tst_automation.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
        {
          id: "wf-tst-perf",
          name: i18n.t("opc.industry.software_dev.tabs.quality.workflows.wf_tst_perf.name"),
          description: i18n.t("opc.industry.software_dev.tabs.quality.workflows.wf_tst_perf.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "devops",
      label: i18n.t("opc.industry.software_dev.tabs.devops.label"),
      icon: <RocketOutlined />,
      description: i18n.t("opc.industry.software_dev.tabs.devops.description"),
      actions: [],
      workflows: [
        {
          id: "wf-eng-ci-setup",
          name: i18n.t("opc.industry.software_dev.tabs.devops.workflows.wf_eng_ci_setup.name"),
          description: i18n.t("opc.industry.software_dev.tabs.devops.workflows.wf_eng_ci_setup.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
        {
          id: "wf-eng-deploy",
          name: i18n.t("opc.industry.software_dev.tabs.devops.workflows.wf_eng_deploy.name"),
          description: i18n.t("opc.industry.software_dev.tabs.devops.workflows.wf_eng_deploy.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
        {
          id: "wf-eng-monitor-setup",
          name: i18n.t("opc.industry.software_dev.tabs.devops.workflows.wf_eng_monitor_setup.name"),
          description: i18n.t("opc.industry.software_dev.tabs.devops.workflows.wf_eng_monitor_setup.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
        {
          id: "wf-eng-db-migrate",
          name: i18n.t("opc.industry.software_dev.tabs.devops.workflows.wf_eng_db_migrate.name"),
          description: i18n.t("opc.industry.software_dev.tabs.devops.workflows.wf_eng_db_migrate.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "team",
      label: i18n.t("opc.industry.software_dev.tabs.team.label"),
      icon: <UserOutlined />,
      description: i18n.t("opc.industry.software_dev.tabs.team.description"),
      actions: [],
      workflows: [
        {
          id: "wf-eng-onboarding",
          name: i18n.t("opc.industry.software_dev.tabs.team.workflows.wf_eng_onboarding.name"),
          description: i18n.t("opc.industry.software_dev.tabs.team.workflows.wf_eng_onboarding.description"),
          version: "1.0",
          inputFields: SOFTWARE_DEV_INPUT_FIELDS,
        },
      ],
    },
  ],
};

export function SoftwareDevPage() {
  return (
    <IndustryHub
      industryId="software-dev"
      config={softwareDevConfig}
      industryTitle={t(`opc.industries.software_dev`)}
      industryIcon={<CodeSandboxOutlined />}
    />
  );
}

// ==========================================
// 金融投资行业页面
// ==========================================
const financeInvestConfig: IndustryConfig = {
  tabs: [
    {
      key: "analysis",
      label: i18n.t("opc.industry.finance_invest.tabs.analysis.label"),
      icon: <LineChartOutlined />,
      description: i18n.t("opc.industry.finance_invest.tabs.analysis.description"),
      actions: [
        {
          key: "fi-stock",
          icon: <FundProjectionScreenOutlined />,
          type: "workflow",
          template_id: "stock-analysis",
          label: i18n.t("opc.industry.finance_invest.tabs.analysis.actions.fi_stock"),
        },
        {
          key: "fi-financial",
          icon: <FileTextOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.finance_invest.tabs.analysis.actions.fi_financial"),
        },
      ],
      workflows: [],
    },
    {
      key: "valuation",
      label: i18n.t("opc.industry.finance_invest.tabs.valuation.label"),
      icon: <EditOutlined />,
      description: i18n.t("opc.industry.finance_invest.tabs.valuation.description"),
      actions: [
        {
          key: "fi-valuation",
          icon: <EditOutlined />,
          type: "workflow",
          template_id: "finance_invest_harness_workflow",
          label: i18n.t("opc.industry.finance_invest.tabs.valuation.actions.fi_valuation"),
        },
      ],
      workflows: [
        {
          id: "wf-fin-cost-analysis",
          name: i18n.t("opc.industry.finance_invest.tabs.valuation.workflows.wf_fin_cost_analysis.name"),
          description: i18n.t("opc.industry.finance_invest.tabs.valuation.workflows.wf_fin_cost_analysis.description"),
          version: "1.0",
          inputFields: FINANCE_INVEST_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "risk",
      label: i18n.t("opc.industry.finance_invest.tabs.risk.label"),
      icon: <SolutionOutlined />,
      description: i18n.t("opc.industry.finance_invest.tabs.risk.description"),
      actions: [
        {
          key: "fi-risk",
          icon: <SolutionOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.finance_invest.tabs.risk.actions.fi_risk"),
        },
      ],
      workflows: [
        {
          id: "wf-fin-budget",
          name: i18n.t("opc.industry.finance_invest.tabs.risk.workflows.wf_fin_budget.name"),
          description: i18n.t("opc.industry.finance_invest.tabs.risk.workflows.wf_fin_budget.description"),
          version: "1.0",
          inputFields: FINANCE_INVEST_INPUT_FIELDS,
        },
      ],
    },
  ],
};

export function FinanceInvestPage() {
  return (
    <IndustryHub
      industryId="finance-invest"
      config={financeInvestConfig}
      industryTitle={t(`opc.industries.finance_invest`)}
      industryIcon={<DollarCircleOutlined />}
    />
  );
}

// ==========================================
// 销售增长行业页面
// ==========================================
const salesGrowthConfig: IndustryConfig = {
  tabs: [
    {
      key: "lead",
      label: i18n.t("opc.industry.sales_growth.tabs.lead.label"),
      icon: <CrownOutlined />,
      description: i18n.t("opc.industry.sales_growth.tabs.lead.description"),
      actions: [
        {
          key: "sg-lead",
          icon: <CrownOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.sales_growth.tabs.lead.actions.sg_lead"),
        },
      ],
      workflows: [
        {
          id: "wf-sal-outbound",
          name: i18n.t("opc.industry.sales_growth.tabs.lead.workflows.wf_sal_outbound.name"),
          description: i18n.t("opc.industry.sales_growth.tabs.lead.workflows.wf_sal_outbound.description"),
          version: "1.0",
          inputFields: SALES_GROWTH_INPUT_FIELDS,
        },
        {
          id: "wf-mkt-influencer",
          name: i18n.t("opc.industry.sales_growth.tabs.lead.workflows.wf_mkt_influencer.name"),
          description: i18n.t("opc.industry.sales_growth.tabs.lead.workflows.wf_mkt_influencer.description"),
          version: "1.0",
          inputFields: SALES_GROWTH_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "convert",
      label: i18n.t("opc.industry.sales_growth.tabs.convert.label"),
      icon: <RocketOutlined />,
      description: i18n.t("opc.industry.sales_growth.tabs.convert.description"),
      actions: [
        {
          key: "sg-funnel",
          icon: <RocketOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.sales_growth.tabs.convert.actions.sg_funnel"),
        },
        {
          key: "sg-copy",
          icon: <EditOutlined />,
          type: "workflow",
          template_id: "sales_growth_harness_workflow",
          label: i18n.t("opc.industry.sales_growth.tabs.convert.actions.sg_copy"),
        },
      ],
      workflows: [
        {
          id: "wf-sal-deal-strategy",
          name: i18n.t("opc.industry.sales_growth.tabs.convert.workflows.wf_sal_deal_strategy.name"),
          description: i18n.t("opc.industry.sales_growth.tabs.convert.workflows.wf_sal_deal_strategy.description"),
          version: "1.0",
          inputFields: SALES_GROWTH_INPUT_FIELDS,
        },
        {
          id: "wf-sal-proposal",
          name: i18n.t("opc.industry.sales_growth.tabs.convert.workflows.wf_sal_proposal.name"),
          description: i18n.t("opc.industry.sales_growth.tabs.convert.workflows.wf_sal_proposal.description"),
          version: "1.0",
          inputFields: SALES_GROWTH_INPUT_FIELDS,
        },
        {
          id: "wf-mkt-ab-test",
          name: i18n.t("opc.industry.sales_growth.tabs.convert.workflows.wf_mkt_ab_test.name"),
          description: i18n.t("opc.industry.sales_growth.tabs.convert.workflows.wf_mkt_ab_test.description"),
          version: "1.0",
          inputFields: SALES_GROWTH_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "manage",
      label: i18n.t("opc.industry.sales_growth.tabs.manage.label"),
      icon: <UserOutlined />,
      description: i18n.t("opc.industry.sales_growth.tabs.manage.description"),
      actions: [
        {
          key: "sg-competitor",
          icon: <TrophyOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.sales_growth.tabs.manage.actions.sg_competitor"),
        },
      ],
      workflows: [
        {
          id: "wf-sal-pipeline-review",
          name: i18n.t("opc.industry.sales_growth.tabs.manage.workflows.wf_sal_pipeline_review.name"),
          description: i18n.t("opc.industry.sales_growth.tabs.manage.workflows.wf_sal_pipeline_review.description"),
          version: "1.0",
          inputFields: SALES_GROWTH_INPUT_FIELDS,
        },
        {
          id: "wf-sal-account-plan",
          name: i18n.t("opc.industry.sales_growth.tabs.manage.workflows.wf_sal_account_plan.name"),
          description: i18n.t("opc.industry.sales_growth.tabs.manage.workflows.wf_sal_account_plan.description"),
          version: "1.0",
          inputFields: SALES_GROWTH_INPUT_FIELDS,
        },
      ],
    },
  ],
};

export function SalesGrowthPage() {
  return (
    <IndustryHub
      industryId="sales-growth"
      config={salesGrowthConfig}
      industryTitle={t(`opc.industries.sales_growth`)}
      industryIcon={<LineChartOutlined />}
    />
  );
}

// ==========================================
// 内容媒体行业页面
// ==========================================
const contentMediaConfig: IndustryConfig = {
  tabs: [
    {
      key: "create",
      label: i18n.t("opc.industry.content_media.tabs.create.label"),
      icon: <EditOutlined />,
      description: i18n.t("opc.industry.content_media.tabs.create.description"),
      actions: [
        {
          key: "cm-writing",
          icon: <EditOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.content_media.tabs.create.actions.cm_writing"),
        },
        {
          key: "cm-video",
          icon: <VideoCameraOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.content_media.tabs.create.actions.cm_video"),
        },
      ],
      workflows: [
        {
          id: "wf-mkt-brand-guide",
          name: i18n.t("opc.industry.content_media.tabs.create.workflows.wf_mkt_brand_guide.name"),
          description: i18n.t("opc.industry.content_media.tabs.create.workflows.wf_mkt_brand_guide.description"),
          version: "1.0",
          inputFields: CONTENT_MEDIA_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "seo",
      label: i18n.t("opc.industry.content_media.tabs.seo.label"),
      icon: <SearchOutlined />,
      description: i18n.t("opc.industry.content_media.tabs.seo.description"),
      actions: [
        {
          key: "cm-seo",
          icon: <SearchOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.content_media.tabs.seo.actions.cm_seo"),
        },
      ],
      workflows: [
        {
          id: "wf-mkt-seo-audit",
          name: i18n.t("opc.industry.content_media.tabs.seo.workflows.wf_mkt_seo_audit.name"),
          description: i18n.t("opc.industry.content_media.tabs.seo.workflows.wf_mkt_seo_audit.description"),
          version: "1.0",
          inputFields: CONTENT_MEDIA_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "distribute",
      label: i18n.t("opc.industry.content_media.tabs.distribute.label"),
      icon: <RocketOutlined />,
      description: i18n.t("opc.industry.content_media.tabs.distribute.description"),
      actions: [
        {
          key: "cm-calendar",
          icon: <BookOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.content_media.tabs.distribute.actions.cm_calendar"),
        },
      ],
      workflows: [
        {
          id: "wf-mkt-social-plan",
          name: i18n.t("opc.industry.content_media.tabs.distribute.workflows.wf_mkt_social_plan.name"),
          description: i18n.t("opc.industry.content_media.tabs.distribute.workflows.wf_mkt_social_plan.description"),
          version: "1.0",
          inputFields: CONTENT_MEDIA_INPUT_FIELDS,
        },
        {
          id: "wf-mkt-email-campaign",
          name: i18n.t("opc.industry.content_media.tabs.distribute.workflows.wf_mkt_email_campaign.name"),
          description: i18n.t("opc.industry.content_media.tabs.distribute.workflows.wf_mkt_email_campaign.description"),
          version: "1.0",
          inputFields: CONTENT_MEDIA_INPUT_FIELDS,
        },
        {
          id: "wf-mkt-webinar",
          name: i18n.t("opc.industry.content_media.tabs.distribute.workflows.wf_mkt_webinar.name"),
          description: i18n.t("opc.industry.content_media.tabs.distribute.workflows.wf_mkt_webinar.description"),
          version: "1.0",
          inputFields: CONTENT_MEDIA_INPUT_FIELDS,
        },
        {
          id: "wf-mkt-influencer",
          name: i18n.t("opc.industry.content_media.tabs.distribute.workflows.wf_mkt_influencer.name"),
          description: i18n.t("opc.industry.content_media.tabs.distribute.workflows.wf_mkt_influencer.description"),
          version: "1.0",
          inputFields: CONTENT_MEDIA_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "literary",
      label: i18n.t("opc.industry.content_media.tabs.literary.label"),
      icon: <BookOutlined />,
      description: i18n.t("opc.industry.content_media.tabs.literary.description"),
      actions: [
        {
          key: "lm-novel",
          icon: <BookOutlined />,
          type: "workflow",
          template_id: "workflow-cm-literary-creation",
          label: i18n.t("opc.industry.content_media.tabs.literary.actions.lm_novel"),
        },
        {
          key: "lm-poetry",
          icon: <EditOutlined />,
          type: "workflow",
          template_id: "workflow-cm-literary-creation",
          label: i18n.t("opc.industry.content_media.tabs.literary.actions.lm_poetry"),
        },
      ],
      workflows: [
        {
          id: "workflow-cm-literary-creation",
          name: i18n.t("opc.industry.content_media.tabs.literary.workflows.wf_cm_literary_creation.name"),
          description: i18n.t("opc.industry.content_media.tabs.literary.workflows.wf_cm_literary_creation.description"),
          version: "3.0",
          inputFields: LITERARY_CREATION_INPUT_FIELDS,
        },
      ],
    },
  ],
};

export function ContentMediaPage() {
  return (
    <IndustryHub
      industryId="content-media"
      config={contentMediaConfig}
      industryTitle={t(`opc.industries.content_media`)}
      industryIcon={<VideoCameraOutlined />}
    />
  );
}

// ==========================================
// 行业咨询行业页面 (P0 补齐 wf-spc)
// ==========================================
const industryConsultingConfig: IndustryConfig = {
  tabs: [
    {
      key: "strategy",
      label: i18n.t("opc.industry.industry_consulting.tabs.strategy.label"),
      icon: <LineChartOutlined />,
      description: i18n.t("opc.industry.industry_consulting.tabs.strategy.description"),
      actions: [
        {
          key: "ic-market",
          icon: <LineChartOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.industry_consulting.tabs.strategy.actions.ic_market"),
        },
        {
          key: "ic-entry",
          icon: <RocketOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.industry_consulting.tabs.strategy.actions.ic_entry"),
        },
        {
          key: "ic-competitor",
          icon: <TrophyOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.industry_consulting.tabs.strategy.actions.ic_competitor"),
        },
      ],
      workflows: [
        {
          id: "wf-strat-biz-plan",
          name: i18n.t("opc.industry.industry_consulting.tabs.strategy.workflows.wf_strat_biz_plan.name"),
          description: i18n.t("opc.industry.industry_consulting.tabs.strategy.workflows.wf_strat_biz_plan.description"),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
        {
          id: "wf-strat-market-entry",
          name: i18n.t("opc.industry.industry_consulting.tabs.strategy.workflows.wf_strat_market_entry.name"),
          description: i18n.t(
            "opc.industry.industry_consulting.tabs.strategy.workflows.wf_strat_market_entry.description",
          ),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
        {
          id: "wf-mkt-competitive-intel",
          name: i18n.t("opc.industry.industry_consulting.tabs.strategy.workflows.wf_mkt_competitive_intel.name"),
          description: i18n.t(
            "opc.industry.industry_consulting.tabs.strategy.workflows.wf_mkt_competitive_intel.description",
          ),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "compliance",
      label: i18n.t("opc.industry.industry_consulting.tabs.compliance.label"),
      icon: <AuditOutlined />,
      description: i18n.t("opc.industry.industry_consulting.tabs.compliance.description"),
      actions: [
        {
          key: "ic-report",
          icon: <FileTextOutlined />,
          type: "workflow",
          template_id: "industry_consulting_harness_workflow",
          label: i18n.t("opc.industry.industry_consulting.tabs.compliance.actions.ic_report"),
        },
      ],
      workflows: [
        {
          id: "wf-spc-esg",
          name: i18n.t("opc.industry.industry_consulting.tabs.compliance.workflows.wf_spc_esg.name"),
          description: i18n.t("opc.industry.industry_consulting.tabs.compliance.workflows.wf_spc_esg.description"),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
        {
          id: "wf-spc-legal-review",
          name: i18n.t("opc.industry.industry_consulting.tabs.compliance.workflows.wf_spc_legal_review.name"),
          description: i18n.t(
            "opc.industry.industry_consulting.tabs.compliance.workflows.wf_spc_legal_review.description",
          ),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
        {
          id: "wf-spc-data-privacy",
          name: i18n.t("opc.industry.industry_consulting.tabs.compliance.workflows.wf_spc_data_privacy.name"),
          description: i18n.t(
            "opc.industry.industry_consulting.tabs.compliance.workflows.wf_spc_data_privacy.description",
          ),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
        {
          id: "wf-sec-compliance",
          name: i18n.t("opc.industry.industry_consulting.tabs.compliance.workflows.wf_sec_compliance.name"),
          description: i18n.t(
            "opc.industry.industry_consulting.tabs.compliance.workflows.wf_sec_compliance.description",
          ),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "capital",
      label: i18n.t("opc.industry.industry_consulting.tabs.capital.label"),
      icon: <DollarCircleOutlined />,
      description: i18n.t("opc.industry.industry_consulting.tabs.capital.description"),
      actions: [],
      workflows: [
        {
          id: "wf-spc-m-a",
          name: i18n.t("opc.industry.industry_consulting.tabs.capital.workflows.wf_spc_m_a.name"),
          description: i18n.t("opc.industry.industry_consulting.tabs.capital.workflows.wf_spc_m_a.description"),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
        {
          id: "wf-spc-grant",
          name: i18n.t("opc.industry.industry_consulting.tabs.capital.workflows.wf_spc_grant.name"),
          description: i18n.t("opc.industry.industry_consulting.tabs.capital.workflows.wf_spc_grant.description"),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "organization",
      label: i18n.t("opc.industry.industry_consulting.tabs.organization.label"),
      icon: <UserOutlined />,
      description: i18n.t("opc.industry.industry_consulting.tabs.organization.description"),
      actions: [],
      workflows: [
        {
          id: "wf-spc-hire",
          name: i18n.t("opc.industry.industry_consulting.tabs.organization.workflows.wf_spc_hire.name"),
          description: i18n.t("opc.industry.industry_consulting.tabs.organization.workflows.wf_spc_hire.description"),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
        {
          id: "wf-spc-onboard",
          name: i18n.t("opc.industry.industry_consulting.tabs.organization.workflows.wf_spc_onboard.name"),
          description: i18n.t(
            "opc.industry.industry_consulting.tabs.organization.workflows.wf_spc_onboard.description",
          ),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
        {
          id: "wf-spc-change-mgmt",
          name: i18n.t("opc.industry.industry_consulting.tabs.organization.workflows.wf_spc_change_mgmt.name"),
          description: i18n.t(
            "opc.industry.industry_consulting.tabs.organization.workflows.wf_spc_change_mgmt.description",
          ),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "supply",
      label: i18n.t("opc.industry.industry_consulting.tabs.supply.label"),
      icon: <GlobalOutlined />,
      description: i18n.t("opc.industry.industry_consulting.tabs.supply.description"),
      actions: [],
      workflows: [
        {
          id: "wf-spc-supply-chain",
          name: i18n.t("opc.industry.industry_consulting.tabs.supply.workflows.wf_spc_supply_chain.name"),
          description: i18n.t("opc.industry.industry_consulting.tabs.supply.workflows.wf_spc_supply_chain.description"),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
        {
          id: "wf-spc-localization",
          name: i18n.t("opc.industry.industry_consulting.tabs.supply.workflows.wf_spc_localization.name"),
          description: i18n.t("opc.industry.industry_consulting.tabs.supply.workflows.wf_spc_localization.description"),
          version: "1.0",
          inputFields: INDUSTRY_CONSULTING_INPUT_FIELDS,
        },
      ],
    },
  ],
};

export function IndustryConsultingPage() {
  return (
    <IndustryHub
      industryId="industry-consulting"
      config={industryConsultingConfig}
      industryTitle={t(`opc.industries.industry_consulting`)}
      industryIcon={<CrownOutlined />}
    />
  );
}

// ==========================================
// 会计行业页面
// ==========================================
const accountingConfig: IndustryConfig = {
  tabs: [
    {
      key: "finance",
      label: i18n.t("opc.industry.accounting.tabs.finance.label"),
      icon: <FundProjectionScreenOutlined />,
      description: i18n.t("opc.industry.accounting.tabs.finance.description"),
      actions: [
        {
          key: "ac-report",
          icon: <FileTextOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.accounting.tabs.finance.actions.ac_report"),
        },
        {
          key: "ac-cost",
          icon: <EditOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.accounting.tabs.finance.actions.ac_cost"),
        },
      ],
      workflows: [
        {
          id: "wf-fin-cost-analysis",
          name: i18n.t("opc.industry.accounting.tabs.finance.workflows.wf_fin_cost_analysis.name"),
          description: i18n.t("opc.industry.accounting.tabs.finance.workflows.wf_fin_cost_analysis.description"),
          version: "1.0",
          inputFields: ACCOUNTING_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "tax",
      label: i18n.t("opc.industry.accounting.tabs.tax.label"),
      icon: <DollarCircleOutlined />,
      description: i18n.t("opc.industry.accounting.tabs.tax.description"),
      actions: [
        {
          key: "ac-tax",
          icon: <DollarCircleOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.accounting.tabs.tax.actions.ac_tax"),
        },
      ],
      workflows: [
        {
          id: "wf-fin-tax",
          name: i18n.t("opc.industry.accounting.tabs.tax.workflows.wf_fin_tax.name"),
          description: i18n.t("opc.industry.accounting.tabs.tax.workflows.wf_fin_tax.description"),
          version: "1.0",
          inputFields: ACCOUNTING_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "budget",
      label: i18n.t("opc.industry.accounting.tabs.budget.label"),
      icon: <LineChartOutlined />,
      description: i18n.t("opc.industry.accounting.tabs.budget.description"),
      actions: [
        {
          key: "ac-budget",
          icon: <FundProjectionScreenOutlined />,
          type: "workflow",
          template_id: "accounting_harness_workflow",
          label: i18n.t("opc.industry.accounting.tabs.budget.actions.ac_budget"),
        },
      ],
      workflows: [
        {
          id: "wf-fin-budget",
          name: i18n.t("opc.industry.accounting.tabs.budget.workflows.wf_fin_budget.name"),
          description: i18n.t("opc.industry.accounting.tabs.budget.workflows.wf_fin_budget.description"),
          version: "1.0",
          inputFields: ACCOUNTING_INPUT_FIELDS,
        },
      ],
    },
  ],
};

export function AccountingPage() {
  return (
    <IndustryHub
      industryId="accounting"
      config={accountingConfig}
      industryTitle={t(`opc.industries.accounting`)}
      industryIcon={<AuditOutlined />}
    />
  );
}

// ==========================================
// 电子商务行业页面
// ==========================================
const ecommerceConfig: IndustryConfig = {
  tabs: [
    {
      key: "product",
      label: i18n.t("opc.industry.ecommerce.tabs.product.label"),
      icon: <SearchOutlined />,
      description: i18n.t("opc.industry.ecommerce.tabs.product.description"),
      actions: [
        {
          key: "ec-product",
          icon: <SearchOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.ecommerce.tabs.product.actions.ec_product"),
        },
      ],
      workflows: [
        {
          id: "wf-prod-spec",
          name: i18n.t("opc.industry.ecommerce.tabs.product.workflows.wf_prod_spec.name"),
          description: i18n.t("opc.industry.ecommerce.tabs.product.workflows.wf_prod_spec.description"),
          version: "1.0",
          inputFields: ECOMMERCE_INPUT_FIELDS,
        },
        {
          id: "wf-prod-launch",
          name: i18n.t("opc.industry.ecommerce.tabs.product.workflows.wf_prod_launch.name"),
          description: i18n.t("opc.industry.ecommerce.tabs.product.workflows.wf_prod_launch.description"),
          version: "1.0",
          inputFields: ECOMMERCE_INPUT_FIELDS,
        },
        {
          id: "wf-prod-roadmap",
          name: i18n.t("opc.industry.ecommerce.tabs.product.workflows.wf_prod_roadmap.name"),
          description: i18n.t("opc.industry.ecommerce.tabs.product.workflows.wf_prod_roadmap.description"),
          version: "1.0",
          inputFields: ECOMMERCE_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "pricing",
      label: i18n.t("opc.industry.ecommerce.tabs.pricing.label"),
      icon: <TagOutlined />,
      description: i18n.t("opc.industry.ecommerce.tabs.pricing.description"),
      actions: [
        {
          key: "ec-price",
          icon: <TagOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.ecommerce.tabs.pricing.actions.ec_price"),
        },
        {
          key: "ec-promote",
          icon: <RocketOutlined />,
          type: "workflow",
          template_id: "ecommerce_harness_workflow",
          label: i18n.t("opc.industry.ecommerce.tabs.pricing.actions.ec_promote"),
        },
      ],
      workflows: [
        {
          id: "wf-mkt-pr-plan",
          name: i18n.t("opc.industry.ecommerce.tabs.pricing.workflows.wf_mkt_pr_plan.name"),
          description: i18n.t("opc.industry.ecommerce.tabs.pricing.workflows.wf_mkt_pr_plan.description"),
          version: "1.0",
          inputFields: ECOMMERCE_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "marketing",
      label: i18n.t("opc.industry.ecommerce.tabs.marketing.label"),
      icon: <RocketOutlined />,
      description: i18n.t("opc.industry.ecommerce.tabs.marketing.description"),
      actions: [],
      workflows: [
        {
          id: "wf-mkt-ab-test",
          name: i18n.t("opc.industry.ecommerce.tabs.marketing.workflows.wf_mkt_ab_test.name"),
          description: i18n.t("opc.industry.ecommerce.tabs.marketing.workflows.wf_mkt_ab_test.description"),
          version: "1.0",
          inputFields: ECOMMERCE_INPUT_FIELDS,
        },
        {
          id: "wf-mkt-analytics",
          name: i18n.t("opc.industry.ecommerce.tabs.marketing.workflows.wf_mkt_analytics.name"),
          description: i18n.t("opc.industry.ecommerce.tabs.marketing.workflows.wf_mkt_analytics.description"),
          version: "1.0",
          inputFields: ECOMMERCE_INPUT_FIELDS,
        },
        {
          id: "wf-mkt-email-campaign",
          name: i18n.t("opc.industry.ecommerce.tabs.marketing.workflows.wf_mkt_email_campaign.name"),
          description: i18n.t("opc.industry.ecommerce.tabs.marketing.workflows.wf_mkt_email_campaign.description"),
          version: "1.0",
          inputFields: ECOMMERCE_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "operation",
      label: i18n.t("opc.industry.ecommerce.tabs.operation.label"),
      icon: <ShopOutlined />,
      description: i18n.t("opc.industry.ecommerce.tabs.operation.description"),
      actions: [
        {
          key: "ec-shop",
          icon: <ShopOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.ecommerce.tabs.operation.actions.ec_shop"),
        },
      ],
      workflows: [
        {
          id: "wf-sup-ticket",
          name: i18n.t("opc.industry.ecommerce.tabs.operation.workflows.wf_sup_ticket.name"),
          description: i18n.t("opc.industry.ecommerce.tabs.operation.workflows.wf_sup_ticket.description"),
          version: "1.0",
          inputFields: ECOMMERCE_INPUT_FIELDS,
        },
        {
          id: "wf-spc-supply-chain",
          name: i18n.t("opc.industry.ecommerce.tabs.operation.workflows.wf_spc_supply_chain.name"),
          description: i18n.t("opc.industry.ecommerce.tabs.operation.workflows.wf_spc_supply_chain.description"),
          version: "1.0",
          inputFields: ECOMMERCE_INPUT_FIELDS,
        },
      ],
    },
  ],
};

export function EcommercePage() {
  return (
    <IndustryHub
      industryId="ecommerce"
      config={ecommerceConfig}
      industryTitle={t(`opc.industries.ecommerce`)}
      industryIcon={<ShopOutlined />}
    />
  );
}

// ==========================================
// 教育行业页面
// ==========================================
const educationConfig: IndustryConfig = {
  tabs: [
    {
      key: "course",
      label: i18n.t("opc.industry.education.tabs.course.label"),
      icon: <BookOutlined />,
      description: i18n.t("opc.industry.education.tabs.course.description"),
      actions: [
        {
          key: "ed-course",
          icon: <BookOutlined />,
          type: "workflow",
          template_id: "education_harness_workflow",
          label: i18n.t("opc.industry.education.tabs.course.actions.ed_course"),
        },
        {
          key: "ed-content",
          icon: <FileTextOutlined />,
          type: "workflow",
          template_id: "education_harness_workflow",
          label: i18n.t("opc.industry.education.tabs.course.actions.ed_content"),
        },
      ],
      workflows: [
        {
          id: "wf-acd-literature",
          name: i18n.t("opc.industry.education.tabs.course.workflows.wf_acd_literature.name"),
          description: i18n.t("opc.industry.education.tabs.course.workflows.wf_acd_literature.description"),
          version: "1.0",
          inputFields: EDUCATION_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "learning",
      label: i18n.t("opc.industry.education.tabs.learning.label"),
      icon: <LineChartOutlined />,
      description: i18n.t("opc.industry.education.tabs.learning.description"),
      actions: [
        {
          key: "ed-knowledge",
          icon: <CodeSandboxOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.education.tabs.learning.actions.ed_knowledge"),
        },
        {
          key: "ed-path",
          icon: <LineChartOutlined />,
          type: "conversation",
          label: i18n.t("opc.industry.education.tabs.learning.actions.ed_path"),
        },
      ],
      workflows: [],
    },
    {
      key: "support",
      label: i18n.t("opc.industry.education.tabs.support.label"),
      icon: <UserOutlined />,
      description: i18n.t("opc.industry.education.tabs.support.description"),
      actions: [],
      workflows: [
        {
          id: "wf-sup-faq",
          name: i18n.t("opc.industry.education.tabs.support.workflows.wf_sup_faq.name"),
          description: i18n.t("opc.industry.education.tabs.support.workflows.wf_sup_faq.description"),
          version: "1.0",
          inputFields: EDUCATION_INPUT_FIELDS,
        },
        {
          id: "wf-sup-satisfaction",
          name: i18n.t("opc.industry.education.tabs.support.workflows.wf_sup_satisfaction.name"),
          description: i18n.t("opc.industry.education.tabs.support.workflows.wf_sup_satisfaction.description"),
          version: "1.0",
          inputFields: EDUCATION_INPUT_FIELDS,
        },
      ],
    },
  ],
};

export function EducationPage() {
  return (
    <IndustryHub
      industryId="education"
      config={educationConfig}
      industryTitle={t(`opc.industries.education`)}
      industryIcon={<BookOutlined />}
    />
  );
}

// ==========================================
// 设计行业页面 (P2 新增 wf-des)
// ==========================================
const designConfig: IndustryConfig = {
  tabs: [
    {
      key: "research",
      label: i18n.t("opc.industry.design.tabs.research.label"),
      icon: <SearchOutlined />,
      description: i18n.t("opc.industry.design.tabs.research.description"),
      actions: [],
      workflows: [
        {
          id: "wf-des-ux-research",
          name: i18n.t("opc.industry.design.tabs.research.workflows.wf_des_ux_research.name"),
          description: i18n.t("opc.industry.design.tabs.research.workflows.wf_des_ux_research.description"),
          version: "1.0",
          inputFields: DESIGN_INPUT_FIELDS,
        },
        {
          id: "wf-des-accessibility",
          name: i18n.t("opc.industry.design.tabs.research.workflows.wf_des_accessibility.name"),
          description: i18n.t("opc.industry.design.tabs.research.workflows.wf_des_accessibility.description"),
          version: "1.0",
          inputFields: DESIGN_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "system",
      label: i18n.t("opc.industry.design.tabs.system.label"),
      icon: <ApiOutlined />,
      description: i18n.t("opc.industry.design.tabs.system.description"),
      actions: [],
      workflows: [
        {
          id: "wf-des-design-system",
          name: i18n.t("opc.industry.design.tabs.system.workflows.wf_des_design_system.name"),
          description: i18n.t("opc.industry.design.tabs.system.workflows.wf_des_design_system.description"),
          version: "1.0",
          inputFields: DESIGN_INPUT_FIELDS,
        },
        {
          id: "wf-des-prototype",
          name: i18n.t("opc.industry.design.tabs.system.workflows.wf_des_prototype.name"),
          description: i18n.t("opc.industry.design.tabs.system.workflows.wf_des_prototype.description"),
          version: "1.0",
          inputFields: DESIGN_INPUT_FIELDS,
        },
      ],
    },
  ],
};

export function DesignPage() {
  return (
    <IndustryHub
      industryId="design"
      config={designConfig}
      industryTitle={t(`opc.industries.design`)}
      industryIcon={<EditOutlined />}
    />
  );
}

// ==========================================
// 项目管理行业页面 (P2 新增 wf-pm)
// ==========================================
const projectManagementConfig: IndustryConfig = {
  tabs: [
    {
      key: "planning",
      label: i18n.t("opc.industry.project_management.tabs.planning.label"),
      icon: <LineChartOutlined />,
      description: i18n.t("opc.industry.project_management.tabs.planning.description"),
      actions: [],
      workflows: [
        {
          id: "wf-pm-campaign",
          name: i18n.t("opc.industry.project_management.tabs.planning.workflows.wf_pm_campaign.name"),
          description: i18n.t("opc.industry.project_management.tabs.planning.workflows.wf_pm_campaign.description"),
          version: "1.0",
          inputFields: PROJECT_MANAGEMENT_INPUT_FIELDS,
        },
        {
          id: "wf-pm-roi",
          name: i18n.t("opc.industry.project_management.tabs.planning.workflows.wf_pm_roi.name"),
          description: i18n.t("opc.industry.project_management.tabs.planning.workflows.wf_pm_roi.description"),
          version: "1.0",
          inputFields: PROJECT_MANAGEMENT_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "execution",
      label: i18n.t("opc.industry.project_management.tabs.execution.label"),
      icon: <RocketOutlined />,
      description: i18n.t("opc.industry.project_management.tabs.execution.description"),
      actions: [],
      workflows: [
        {
          id: "wf-pm-sprint",
          name: i18n.t("opc.industry.project_management.tabs.execution.workflows.wf_pm_sprint.name"),
          description: i18n.t("opc.industry.project_management.tabs.execution.workflows.wf_pm_sprint.description"),
          version: "1.0",
          inputFields: PROJECT_MANAGEMENT_INPUT_FIELDS,
        },
        {
          id: "wf-pm-status",
          name: i18n.t("opc.industry.project_management.tabs.execution.workflows.wf_pm_status.name"),
          description: i18n.t("opc.industry.project_management.tabs.execution.workflows.wf_pm_status.description"),
          version: "1.0",
          inputFields: PROJECT_MANAGEMENT_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "risk",
      label: i18n.t("opc.industry.project_management.tabs.risk.label"),
      icon: <SolutionOutlined />,
      description: i18n.t("opc.industry.project_management.tabs.risk.description"),
      actions: [],
      workflows: [
        {
          id: "wf-pm-risk",
          name: i18n.t("opc.industry.project_management.tabs.risk.workflows.wf_pm_risk.name"),
          description: i18n.t("opc.industry.project_management.tabs.risk.workflows.wf_pm_risk.description"),
          version: "1.0",
          inputFields: PROJECT_MANAGEMENT_INPUT_FIELDS,
        },
      ],
    },
  ],
};

export function ProjectManagementPage() {
  return (
    <IndustryHub
      industryId="project-management"
      config={projectManagementConfig}
      industryTitle={t(`opc.industries.project_management`)}
      industryIcon={<RocketOutlined />}
    />
  );
}

// ==========================================
// 安全合规行业页面 (P2 新增 wf-sec)
// ==========================================
const securityConfig: IndustryConfig = {
  tabs: [
    {
      key: "prevention",
      label: i18n.t("opc.industry.security.tabs.prevention.label"),
      icon: <SolutionOutlined />,
      description: i18n.t("opc.industry.security.tabs.prevention.description"),
      actions: [],
      workflows: [
        {
          id: "wf-sec-pentest",
          name: i18n.t("opc.industry.security.tabs.prevention.workflows.wf_sec_pentest.name"),
          description: i18n.t("opc.industry.security.tabs.prevention.workflows.wf_sec_pentest.description"),
          version: "1.0",
          inputFields: SECURITY_INPUT_FIELDS,
        },
        {
          id: "wf-sec-threat-intel",
          name: i18n.t("opc.industry.security.tabs.prevention.workflows.wf_sec_threat_intel.name"),
          description: i18n.t("opc.industry.security.tabs.prevention.workflows.wf_sec_threat_intel.description"),
          version: "1.0",
          inputFields: SECURITY_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "response",
      label: i18n.t("opc.industry.security.tabs.response.label"),
      icon: <RocketOutlined />,
      description: i18n.t("opc.industry.security.tabs.response.description"),
      actions: [],
      workflows: [
        {
          id: "wf-sec-incident",
          name: i18n.t("opc.industry.security.tabs.response.workflows.wf_sec_incident.name"),
          description: i18n.t("opc.industry.security.tabs.response.workflows.wf_sec_incident.description"),
          version: "1.0",
          inputFields: SECURITY_INPUT_FIELDS,
        },
      ],
    },
  ],
};

export function SecurityPage() {
  return (
    <IndustryHub
      industryId="security"
      config={securityConfig}
      industryTitle={t(`opc.industries.security`)}
      industryIcon={<SolutionOutlined />}
    />
  );
}

// ==========================================
// 地理信息行业页面 (P4 新增 wf-gis + wf-spatial)
// ==========================================
const geospatialConfig: IndustryConfig = {
  tabs: [
    {
      key: "mapping",
      label: i18n.t("opc.industry.geospatial.tabs.mapping.label"),
      icon: <GlobalOutlined />,
      description: i18n.t("opc.industry.geospatial.tabs.mapping.description"),
      actions: [],
      workflows: [
        {
          id: "wf-gis-mapping",
          name: i18n.t("opc.industry.geospatial.tabs.mapping.workflows.wf_gis_mapping.name"),
          description: i18n.t("opc.industry.geospatial.tabs.mapping.workflows.wf_gis_mapping.description"),
          version: "1.0",
          inputFields: GEOSPATIAL_INPUT_FIELDS,
        },
        {
          id: "wf-gis-analysis",
          name: i18n.t("opc.industry.geospatial.tabs.mapping.workflows.wf_gis_analysis.name"),
          description: i18n.t("opc.industry.geospatial.tabs.mapping.workflows.wf_gis_analysis.description"),
          version: "1.0",
          inputFields: GEOSPATIAL_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "collection",
      label: i18n.t("opc.industry.geospatial.tabs.collection.label"),
      icon: <SearchOutlined />,
      description: i18n.t("opc.industry.geospatial.tabs.collection.description"),
      actions: [],
      workflows: [
        {
          id: "wf-gis-drone",
          name: i18n.t("opc.industry.geospatial.tabs.collection.workflows.wf_gis_drone.name"),
          description: i18n.t("opc.industry.geospatial.tabs.collection.workflows.wf_gis_drone.description"),
          version: "1.0",
          inputFields: GEOSPATIAL_INPUT_FIELDS,
        },
        {
          id: "wf-gis-3d-scene",
          name: i18n.t("opc.industry.geospatial.tabs.collection.workflows.wf_gis_3d_scene.name"),
          description: i18n.t("opc.industry.geospatial.tabs.collection.workflows.wf_gis_3d_scene.description"),
          version: "1.0",
          inputFields: GEOSPATIAL_INPUT_FIELDS,
        },
      ],
    },
    {
      key: "spatial",
      label: i18n.t("opc.industry.geospatial.tabs.spatial.label"),
      icon: <ApiOutlined />,
      description: i18n.t("opc.industry.geospatial.tabs.spatial.description"),
      actions: [],
      workflows: [
        {
          id: "wf-spatial-ar",
          name: i18n.t("opc.industry.geospatial.tabs.spatial.workflows.wf_spatial_ar.name"),
          description: i18n.t("opc.industry.geospatial.tabs.spatial.workflows.wf_spatial_ar.description"),
          version: "1.0",
          inputFields: GEOSPATIAL_INPUT_FIELDS,
        },
        {
          id: "wf-spatial-scene",
          name: i18n.t("opc.industry.geospatial.tabs.spatial.workflows.wf_spatial_scene.name"),
          description: i18n.t("opc.industry.geospatial.tabs.spatial.workflows.wf_spatial_scene.description"),
          version: "1.0",
          inputFields: GEOSPATIAL_INPUT_FIELDS,
        },
      ],
    },
  ],
};

export function GeospatialPage() {
  return (
    <IndustryHub
      industryId="geospatial"
      config={geospatialConfig}
      industryTitle={t(`opc.industries.geospatial`)}
      industryIcon={<GlobalOutlined />}
    />
  );
}

// ==========================================
// 游戏开发行业页面 (P4 新增 wf-gd)
// ==========================================
const gameDevConfig: IndustryConfig = {
  tabs: [
    {
      key: "concept",
      label: i18n.t("opc.industry.game_dev.tabs.concept.label"),
      icon: <EditOutlined />,
      description: i18n.t("opc.industry.game_dev.tabs.concept.description"),
      actions: [],
      workflows: [
        {
          id: "wf-gd-concept",
          name: i18n.t("opc.industry.game_dev.tabs.concept.workflows.wf_gd_concept.name"),
          description: i18n.t("opc.industry.game_dev.tabs.concept.workflows.wf_gd_concept.description"),
          version: "1.0",
          inputFields: [
            {
              key: "game_title",
              label: "opc.input_fields.game_dev.game_title.label",
              type: "string" as const,
              required: true,
              placeholder: "opc.input_fields.game_dev.game_title.placeholder",
            },
            {
              key: "game_engine",
              label: "opc.input_fields.game_dev.game_engine.label",
              type: "string" as const,
              required: true,
              placeholder: "opc.input_fields.game_dev.game_engine.placeholder",
            },
            {
              key: "genre",
              label: "opc.input_fields.game_dev.genre.label",
              type: "string" as const,
              required: true,
              placeholder: "opc.input_fields.game_dev.genre.placeholder",
            },
          ],
        },
        {
          id: "wf-gd-prototype",
          name: i18n.t("opc.industry.game_dev.tabs.concept.workflows.wf_gd_prototype.name"),
          description: i18n.t("opc.industry.game_dev.tabs.concept.workflows.wf_gd_prototype.description"),
          version: "1.0",
          inputFields: [
            {
              key: "game_title",
              label: "opc.input_fields.game_dev.game_title.label",
              type: "string" as const,
              required: true,
              placeholder: "opc.input_fields.game_dev.game_title.placeholder",
            },
            {
              key: "game_engine",
              label: "opc.input_fields.game_dev.game_engine.label",
              type: "string" as const,
              required: true,
              placeholder: "opc.input_fields.game_dev.game_engine.placeholder",
            },
            {
              key: "genre",
              label: "opc.input_fields.game_dev.genre.label",
              type: "string" as const,
              required: true,
              placeholder: "opc.input_fields.game_dev.genre.placeholder",
            },
          ],
        },
      ],
    },
    {
      key: "qa",
      label: i18n.t("opc.industry.game_dev.tabs.qa.label"),
      icon: <BugOutlined />,
      description: i18n.t("opc.industry.game_dev.tabs.qa.description"),
      actions: [],
      workflows: [
        {
          id: "wf-gd-qa",
          name: i18n.t("opc.industry.game_dev.tabs.qa.workflows.wf_gd_qa.name"),
          description: i18n.t("opc.industry.game_dev.tabs.qa.workflows.wf_gd_qa.description"),
          version: "1.0",
        },
      ],
    },
  ],
};

export function GameDevPage() {
  return (
    <IndustryHub
      industryId="game-dev"
      config={gameDevConfig}
      industryTitle={t(`opc.industries.game_dev`)}
      industryIcon={<BugOutlined />}
    />
  );
}
