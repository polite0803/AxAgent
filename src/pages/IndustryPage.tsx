// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

import { RLLearningPanel } from "@/components/opc/RLLearningPanel";
import { invoke } from "@/lib/invoke";
import { evolveWorkflow, reflectOnWorkflow, runSelfImprovement } from "@/lib/opcLearning";
import { useConversationStore, useIndustryLearningStore, useSettingsStore } from "@/stores";
import type { IndustryLearningConfig } from "@/types";
import {
  ApiOutlined,
  AuditOutlined,
  BarChartOutlined,
  BookOutlined,
  BugOutlined,
  BulbOutlined,
  CalculatorOutlined,
  CodeOutlined,
  CodeSandboxOutlined,
  CrownOutlined,
  DashboardOutlined,
  DollarCircleOutlined,
  EditOutlined,
  ExperimentOutlined,
  FileSearchOutlined,
  FileTextOutlined,
  FundProjectionScreenOutlined,
  LineChartOutlined,
  PlayCircleOutlined,
  RocketOutlined,
  SearchOutlined,
  ShopOutlined,
  SolutionOutlined,
  SyncOutlined,
  TagOutlined,
  ThunderboltOutlined,
  TrophyOutlined,
  VideoCameraOutlined,
} from "@ant-design/icons";
import {
  Alert,
  App,
  Badge,
  Button,
  Card,
  Col,
  Collapse,
  Divider,
  Empty,
  Progress,
  Row,
  Segmented,
  Space,
  Spin,
  Statistic,
  Steps,
  Tag,
  Timeline,
  Typography,
} from "antd";
import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate, useParams } from "react-router-dom";

const { Title, Paragraph, Text } = Typography;

interface IndustryManifest {
  id: string;
  name: string;
  icon: string;
  description: string;
  version: number;
  enabled: boolean;
}

interface IndustryWorkflow {
  id: string;
  name: string;
  description: string;
  version: string;
}

interface ActionItem {
  key: string;
  icon: ReactNode;
  type: "conversation" | "workflow";
  label?: string;
}

interface IndustryConfig {
  actions: ActionItem[];
  workflows: IndustryWorkflow[];
}

// ── 行业运行时相关类型 ──────────────────────────────────

interface KpiValue {
  id: string;
  name: string;
  value: number;
  unit: string;
  period: string;
  trend?: "up" | "down" | "flat";
  change_percent?: number;
}

interface WorkflowStepInfo {
  id: string;
  name: string;
  description: string;
  order: number;
  status?: "pending" | "active" | "completed";
}

interface AutomationRuleInfo {
  id: string;
  name: string;
  enabled: boolean;
  conditions: Array<{ type: string; config: Record<string, unknown> }>;
  actions: Array<{ type: string; config: Record<string, unknown> }>;
  last_triggered?: string;
  trigger_count?: number;
}

interface IndustryDashboard {
  industry_id: string;
  kpis: KpiValue[];
  cards: Array<{
    id: string;
    title: string;
    kpi_id: string;
    default_display: string;
  }>;
  summary?: string;
}

// ── 分析决策相关类型 ──────────────────────────────────
interface AnalysisRecommendation {
  id: string;
  type: "action" | "warning" | "opportunity";
  description: string;
  priority: "low" | "medium" | "high";
}

interface OpcIndustryDecision {
  industry_id: string;
  decision_type: string;
  summary: string;
  confidence: number;
  kpis: KpiValue[];
  recommendations: AnalysisRecommendation[];
  risk_level: "low" | "medium" | "high";
}

// ── 工作流执行结果类型 ──────────────────────────────────
interface WorkflowExecutionResult {
  industry_id: string;
  workflow_id: string;
  status: "completed" | "failed" | "partial";
  node_results: Array<{
    id: string;
    name: string;
    status: "completed" | "failed" | "skipped";
    duration_ms: number;
    output?: Record<string, unknown>;
  }>;
  duration_ms: number;
  error_message?: string;
}

// ── 学习指标类型 ──────────────────────────────────────────
interface IndustryLearningMetrics {
  industry_id: string;
  total_samples: number;
  decision_accuracy: number;
  risk_prediction_accuracy: number;
  avg_feedback_score: number;
  improvement_trend: "improving" | "stable" | "declining";
  last_updated: number;
}

/** 9 个行业专属配置 - 文本内容通过 i18n 获取 */
/** 工作流 ID 对应 Rust 后端生成的领域工作流 (src-tauri/crates/analysis-engine/src/opc/domain/generated.rs) */
const INDUSTRY_CONFIGS: Record<string, IndustryConfig> = {
  "ai-research": {
    actions: [
      { key: "ai-paper", icon: <FileSearchOutlined />, type: "conversation" },
      { key: "ai-benchmark", icon: <LineChartOutlined />, type: "conversation" },
      { key: "ai-app", icon: <ExperimentOutlined />, type: "conversation" },
      { key: "ai-report", icon: <FileTextOutlined />, type: "workflow" },
    ],
    workflows: [
      { id: "wf-acd-literature", name: "", description: "", version: "1.0" },
      { id: "wf-acd-research", name: "", description: "", version: "1.0" },
    ],
  },
  "software-dev": {
    actions: [
      { key: "sd-code-review", icon: <AuditOutlined />, type: "conversation" },
      { key: "sd-arch", icon: <ApiOutlined />, type: "conversation" },
      { key: "sd-api-doc", icon: <BookOutlined />, type: "workflow" },
      { key: "sd-bug", icon: <BugOutlined />, type: "conversation" },
    ],
    workflows: [
      { id: "wf-eng-api-design", name: "", description: "", version: "1.0" },
      { id: "wf-eng-arch-review", name: "", description: "", version: "1.0" },
      { id: "wf-eng-code-review", name: "", description: "", version: "1.0" },
      { id: "wf-eng-refactor", name: "", description: "", version: "1.0" },
      { id: "wf-eng-security-review", name: "", description: "", version: "1.0" },
    ],
  },
  "finance-invest": {
    actions: [
      { key: "fi-stock", icon: <FundProjectionScreenOutlined />, type: "conversation" },
      { key: "fi-financial", icon: <FileTextOutlined />, type: "conversation" },
      { key: "fi-valuation", icon: <CalculatorOutlined />, type: "workflow" },
      { key: "fi-risk", icon: <SolutionOutlined />, type: "conversation" },
    ],
    workflows: [
      { id: "wf-fin-budget", name: "", description: "", version: "1.0" },
      { id: "wf-fin-cost-analysis", name: "", description: "", version: "1.0" },
      { id: "wf-fin-tax", name: "", description: "", version: "1.0" },
    ],
  },
  "sales-growth": {
    actions: [
      { key: "sg-lead", icon: <CrownOutlined />, type: "conversation" },
      { key: "sg-funnel", icon: <RocketOutlined />, type: "conversation" },
      { key: "sg-copy", icon: <EditOutlined />, type: "workflow" },
      { key: "sg-competitor", icon: <TrophyOutlined />, type: "conversation" },
    ],
    workflows: [
      { id: "wf-sal-outbound", name: "", description: "", version: "1.0" },
      { id: "wf-sal-deal-strategy", name: "", description: "", version: "1.0" },
      { id: "wf-sal-pipeline-review", name: "", description: "", version: "1.0" },
      { id: "wf-mkt-ab-test", name: "", description: "", version: "1.0" },
    ],
  },
  "content-media": {
    actions: [
      { key: "cm-writing", icon: <EditOutlined />, type: "conversation" },
      { key: "cm-seo", icon: <SearchOutlined />, type: "conversation" },
      { key: "cm-video", icon: <VideoCameraOutlined />, type: "conversation" },
      { key: "cm-calendar", icon: <BookOutlined />, type: "conversation" },
    ],
    workflows: [
      { id: "wf-mkt-social-plan", name: "", description: "", version: "1.0" },
      { id: "wf-mkt-seo-audit", name: "", description: "", version: "1.0" },
      { id: "wf-mkt-email-campaign", name: "", description: "", version: "1.0" },
    ],
  },
  "industry-consulting": {
    actions: [
      { key: "ic-report", icon: <FileTextOutlined />, type: "workflow" },
      { key: "ic-market", icon: <LineChartOutlined />, type: "conversation" },
      { key: "ic-entry", icon: <RocketOutlined />, type: "conversation" },
      { key: "ic-competitor", icon: <TrophyOutlined />, type: "conversation" },
    ],
    workflows: [
      { id: "wf-strat-biz-plan", name: "", description: "", version: "1.0" },
      { id: "wf-strat-market-entry", name: "", description: "", version: "1.0" },
      { id: "wf-spc-esg", name: "", description: "", version: "1.0" },
    ],
  },
  accounting: {
    actions: [
      { key: "ac-tax", icon: <DollarCircleOutlined />, type: "conversation" },
      { key: "ac-report", icon: <FileTextOutlined />, type: "conversation" },
      { key: "ac-cost", icon: <CalculatorOutlined />, type: "conversation" },
      { key: "ac-budget", icon: <FundProjectionScreenOutlined />, type: "workflow" },
    ],
    workflows: [
      { id: "wf-fin-budget", name: "", description: "", version: "1.0" },
      { id: "wf-fin-cost-analysis", name: "", description: "", version: "1.0" },
      { id: "wf-fin-tax", name: "", description: "", version: "1.0" },
    ],
  },
  ecommerce: {
    actions: [
      { key: "ec-product", icon: <SearchOutlined />, type: "conversation" },
      { key: "ec-price", icon: <TagOutlined />, type: "conversation" },
      { key: "ec-promote", icon: <RocketOutlined />, type: "workflow" },
      { key: "ec-shop", icon: <ShopOutlined />, type: "conversation" },
    ],
    workflows: [
      { id: "wf-mkt-ab-test", name: "", description: "", version: "1.0" },
      { id: "wf-mkt-analytics", name: "", description: "", version: "1.0" },
      { id: "wf-prod-launch", name: "", description: "", version: "1.0" },
      { id: "wf-prod-spec", name: "", description: "", version: "1.0" },
    ],
  },
  education: {
    actions: [
      { key: "ed-course", icon: <BookOutlined />, type: "workflow" },
      { key: "ed-knowledge", icon: <CodeSandboxOutlined />, type: "conversation" },
      { key: "ed-path", icon: <LineChartOutlined />, type: "conversation" },
      { key: "ed-content", icon: <FileTextOutlined />, type: "workflow" },
    ],
    workflows: [
      { id: "wf-acd-literature", name: "", description: "", version: "1.0" },
      { id: "wf-sup-faq", name: "", description: "", version: "1.0" },
      { id: "wf-sup-satisfaction", name: "", description: "", version: "1.0" },
    ],
  },
};

/** 行业操作面板 — 根据行业 ID 加载专属配置，所有文本通过 i18n 获取 */
export function IndustryPage() {
  const { t } = useTranslation();
  const params = useParams<{ id?: string; industryId?: string }>();
  const location = useLocation();
  const navigate = useNavigate();
  const { message } = App.useApp();

  // 支持两种路由参数：新格式 /opc/industry/:id 和旧格式 /opc/industries/:industryId
  const industryId = params?.id
    || params?.industryId
    || location.pathname.split("/").pop()
    || "";

  const [loading, setLoading] = useState(true);
  const [manifest, setManifest] = useState<IndustryManifest | null>(null);
  const [learningConfig, setLearningConfig] = useState<IndustryLearningConfig | null>(null);
  const [learningLoading, setLearningLoading] = useState(false);

  // 行业运行时数据
  const [dashboard, setDashboard] = useState<IndustryDashboard | null>(null);
  const [dashboardLoading, setDashboardLoading] = useState(false);
  const [workflowSteps, setWorkflowSteps] = useState<WorkflowStepInfo[]>([]);
  const [stepsLoading, setStepsLoading] = useState(false);
  const [automationRules, setAutomationRules] = useState<AutomationRuleInfo[]>([]);
  const [rulesLoading, setRulesLoading] = useState(false);
  const [rulesRunning, setRulesRunning] = useState(false);
  const [kpiTimeRange, setKpiTimeRange] = useState<"7" | "30" | "90">("30");

  // 分析决策数据
  const [decision, setDecision] = useState<OpcIndustryDecision | null>(null);
  const [decisionLoading, setDecisionLoading] = useState(false);
  const [decisionDays, setDecisionDays] = useState<number>(30);

  // 工作流执行数据
  const [workflowResult, setWorkflowResult] = useState<WorkflowExecutionResult | null>(null);
  const [workflowExecuting, setWorkflowExecuting] = useState(false);

  // 学习指标数据
  const [learningMetrics, setLearningMetrics] = useState<IndustryLearningMetrics | null>(null);
  const [metricsLoading, setMetricsLoading] = useState(false);

  const createConversation = useConversationStore((s) => s.createConversation);
  const settings = useSettingsStore((s) => s.settings);
  const learningStore = useIndustryLearningStore();

  const config = useMemo(() => INDUSTRY_CONFIGS[industryId], [industryId]);

  // i18n key 转换: ai-research → ai_research
  const industryKey = industryId.replace(/-/g, "_");

  // 生成 i18n key 前缀
  const actionsPrefix = `opc.industry.actions.${industryId}`;
  const workflowsPrefix = `opc.industry.workflows.${industryId}`;

  useEffect(() => {
    if (!industryId) {
      setLoading(false);
      return;
    }

    const loadIndustry = async () => {
      setLoading(true);
      try {
        const result = await invoke<{
          manifest: IndustryManifest;
        }>("opc_get_industry_pack", { industryId });
        setManifest(result.manifest);
      } catch (e) {
        console.error("[IndustryPage] load failed:", e);
        message.error(t("opc.industry.loadFailed", { error: String(e) }));
      } finally {
        setLoading(false);
      }
    };

    loadIndustry();
  }, [industryId, message, t]);

  // 加载行业学习配置
  useEffect(() => {
    if (!industryId) {
      return;
    }
    const loadLearning = async () => {
      setLearningLoading(true);
      try {
        const config = await learningStore.loadConfig(industryId);
        setLearningConfig(config);
      } catch {
        setLearningConfig(null);
      } finally {
        setLearningLoading(false);
      }
    };
    loadLearning();
  }, [industryId, learningStore]);

  // 加载行业仪表盘（KPI 聚合）
  const loadDashboard = async () => {
    if (!industryId) { return; }
    setDashboardLoading(true);
    try {
      const days = Number(kpiTimeRange);
      const result = await invoke<IndustryDashboard>(
        "opc_get_industry_dashboard",
        { industryId, days },
      );
      setDashboard(result);
    } catch (e) {
      console.error("[IndustryPage] load dashboard failed:", e);
    } finally {
      setDashboardLoading(false);
    }
  };

  // 加载行业工作流步骤
  const loadWorkflowSteps = async () => {
    if (!industryId) { return; }
    setStepsLoading(true);
    try {
      const result = await invoke<{ steps: WorkflowStepInfo[] }>(
        "opc_get_industry_workflow_steps",
        { industryId },
      );
      setWorkflowSteps(result.steps || []);
    } catch (e) {
      console.error("[IndustryPage] load workflow steps failed:", e);
      setWorkflowSteps([]);
    } finally {
      setStepsLoading(false);
    }
  };

  // 加载行业自动化规则
  const loadAutomationRules = async () => {
    if (!industryId) { return; }
    setRulesLoading(true);
    try {
      const result = await invoke<{ rules: AutomationRuleInfo[] }>(
        "opc_get_industry_automation_rules",
        { industryId },
      );
      setAutomationRules(result.rules || []);
    } catch (e) {
      console.error("[IndustryPage] load automation rules failed:", e);
      setAutomationRules([]);
    } finally {
      setRulesLoading(false);
    }
  };

  // 行业数据初始化（行业 ID 变化时触发）
  useEffect(() => {
    if (!industryId) { return; }
    loadDashboard();
    loadWorkflowSteps();
    loadAutomationRules();
  }, [industryId]);

  // KPI 时间范围变化时刷新
  useEffect(() => {
    if (!industryId) { return; }
    loadDashboard();
  }, [kpiTimeRange]);

  /** 手动执行自动化规则 */
  const handleRunRules = async () => {
    if (!industryId) { return; }
    setRulesRunning(true);
    try {
      const triggered = await invoke<string[]>("opc_run_automation_rules", {
        industryId,
        entityType: "customer",
        entityId: "manual_trigger",
      });
      if (triggered.length > 0) {
        message.success(
          t("opc.industry.rules.triggered", { count: triggered.length }),
        );
      } else {
        message.info(t("opc.industry.rules.nothingTriggered"));
      }
    } catch (e) {
      message.error(t("opc.industry.rules.runFailed", { error: String(e) }));
    } finally {
      setRulesRunning(false);
    }
  };

  /** 刷新所有行业数据 */
  const handleRefreshAll = () => {
    loadDashboard();
    loadWorkflowSteps();
    loadAutomationRules();
    message.success(t("opc.industry.refreshSuccess"));
  };

  /** 执行行业分析决策（对接 opc_execute_analysis 命令） */
  const handleExecuteAnalysis = async () => {
    if (!industryId) { return; }
    setDecisionLoading(true);
    try {
      const result = await invoke<OpcIndustryDecision>(
        "opc_execute_analysis",
        { industryId, days: decisionDays },
      );
      setDecision(result);
      message.success(t("opc.industry.analysis.executeSuccess"));
    } catch (e) {
      console.error("[IndustryPage] execute analysis failed:", e);
      message.error(t("opc.industry.analysis.executeFailed", { error: String(e) }));
    } finally {
      setDecisionLoading(false);
    }
  };

  /** 执行行业工作流（对接 opc_execute_workflow 命令） */
  const handleExecuteWorkflow = async () => {
    if (!industryId) { return; }
    setWorkflowExecuting(true);
    setWorkflowResult(null);
    try {
      const result = await invoke<WorkflowExecutionResult>(
        "opc_execute_workflow",
        { industryId, days: decisionDays },
      );
      setWorkflowResult(result);
      if (result.status === "completed") {
        message.success(t("opc.industry.workflow.executeSuccess"));
      } else {
        message.warning(t("opc.industry.workflow.executePartial"));
      }
    } catch (e) {
      console.error("[IndustryPage] execute workflow failed:", e);
      message.error(t("opc.industry.workflow.executeFailed", { error: String(e) }));
    } finally {
      setWorkflowExecuting(false);
    }
  };

  /** 获取学习指标（对接 opc_get_learning_metrics 命令） */
  const handleGetLearningMetrics = async () => {
    if (!industryId) { return; }
    setMetricsLoading(true);
    try {
      const result = await invoke<IndustryLearningMetrics>(
        "opc_get_learning_metrics",
        { industryId },
      );
      setLearningMetrics(result);
    } catch (e) {
      console.error("[IndustryPage] get learning metrics failed:", e);
      setLearningMetrics(null);
    } finally {
      setMetricsLoading(false);
    }
  };

  /** 触发反思 */
  const handleReflect = async () => {
    if (!learningConfig?.reflectionEnabled) {
      message.warning(t("opc.industry.learning.reflection.notEnabled"));
      return;
    }
    try {
      message.loading({ content: t("opc.industry.learning.reflection.triggerDesc"), key: "reflect" });
      await reflectOnWorkflow({
        industryId,
        workflowId: `industry_${industryId}`,
        workflowResult: { status: "manual_triggered" },
      });
      message.success({ content: t("opc.industry.learning.reflection.triggerSuccess"), key: "reflect" });
    } catch (e) {
      message.error(t("opc.industry.learning.reflection.triggerFailed", { error: String(e) }));
    }
  };

  /** 触发进化 */
  const handleEvolve = async () => {
    if (!learningConfig?.evolutionEnabled) {
      message.warning(t("opc.industry.learning.evolution.notEnabled"));
      return;
    }
    try {
      message.loading({ content: t("opc.industry.learning.evolution.triggerDesc"), key: "evolve" });
      await evolveWorkflow({
        industryId,
        workflowId: `industry_${industryId}`,
        reason: "manual_optimization",
      });
      message.success({ content: t("opc.industry.learning.evolution.triggerSuccess"), key: "evolve" });
    } catch (e) {
      message.error(t("opc.industry.learning.evolution.triggerFailed", { error: String(e) }));
    }
  };

  /** 执行自我改进 */
  const handleSelfImprove = async () => {
    if (!learningConfig?.selfImprovementEnabled) {
      message.warning(t("opc.industry.learning.selfImprovement.notEnabled"));
      return;
    }
    try {
      message.loading({ content: t("opc.industry.learning.selfImprovement.triggerDesc"), key: "selfImprove" });
      await runSelfImprovement({
        industryId,
        target: "overall_performance",
      });
      message.success({ content: t("opc.industry.learning.selfImprovement.triggerSuccess"), key: "selfImprove" });
    } catch (e) {
      message.error(t("opc.industry.learning.selfImprovement.triggerFailed", { error: String(e) }));
    }
  };

  /** 执行行业操作 - 调用后端命令获取真实 prompt */
  const handleAction = async (action: ActionItem) => {
    if (!settings?.defaultModel?.a || !settings?.defaultModel?.b) {
      message.warning(t("opc.industry.noProviderConfig"));
      navigate("/settings/providers");
      return;
    }

    if (action.type === "workflow") {
      navigate(`/workflow/new?industry=${industryId}&template=${action.key}`);
      return;
    }

    const actionLabel = action.label || action.key;

    try {
      const promptConfig = await invoke<{
        systemPrompt: string;
        userPrompt: string;
        actionKey: string;
        actionLabel: string;
        industryId: string;
      }>("opc_build_industry_prompt", {
        industryId,
        actionKey: action.key,
      });

      const conv = await createConversation(
        `${promptConfig.actionLabel} - ${manifest?.name || ""}`,
        settings.defaultModel.b,
        settings.defaultModel.a,
        {
          systemPrompt: promptConfig.systemPrompt,
        },
      );
      if (conv?.id) {
        navigate(`/chat?conversationId=${conv.id}&prompt=${encodeURIComponent(promptConfig.userPrompt)}`);
      }
    } catch {
      const conv = await createConversation(
        `${actionLabel} - ${manifest?.name || ""}`,
        settings.defaultModel.b,
        settings.defaultModel.a,
        {
          systemPrompt:
            `你是一位专业的${industryId}领域助手，擅长${actionLabel}相关的分析和咨询。请根据用户需求提供高质量的分析和建议。`,
        },
      );
      if (conv?.id) {
        navigate(`/chat?conversationId=${conv.id}&prompt=${encodeURIComponent(actionLabel)}`);
      }
    }
  };

  /** 使用预设工作流 */
  const handleUseWorkflow = async (wf: IndustryWorkflow) => {
    if (!settings?.defaultModel?.a || !settings?.defaultModel?.b) {
      message.warning(t("opc.industry.noProviderConfig"));
      navigate("/settings/providers");
      return;
    }

    const wfName = t(`${workflowsPrefix}.${wf.id}.name`);

    try {
      const conv = await createConversation(
        t("opc.industry.executeSuffix", { name: wfName }),
        settings.defaultModel.b,
        settings.defaultModel.a,
      );
      if (conv?.id) {
        navigate(`/chat?conversationId=${conv.id}&workflow=${wf.id}`);
      }
    } catch (e) {
      message.error(t("opc.industry.loadFailed", { error: String(e) }));
    }
  };

  if (loading) {
    return (
      <div style={{ padding: 48, textAlign: "center" }}>
        <Spin size="large" description={t("common.loading")} />
      </div>
    );
  }

  if (!manifest || !config) {
    return (
      <div style={{ padding: 48, textAlign: "center" }}>
        <Empty description={t("opc.industry.notFound")} />
      </div>
    );
  }

  return (
    <div style={{ padding: 24, height: "100%", overflow: "auto" }}>
      {/* 行业标题 */}
      <div style={{ marginBottom: 24 }}>
        <Space align="center" style={{ width: "100%", justifyContent: "space-between" }}>
          <div>
            <Title level={3} style={{ marginBottom: 8 }}>
              <span style={{ fontSize: 28, marginRight: 12 }}>{manifest.icon}</span>
              {t(`opc.industries.${industryKey}`)}
            </Title>
            <Paragraph type="secondary">{t(`opc.industries.${industryKey}_desc`)}</Paragraph>
          </div>
          <Button
            icon={<SyncOutlined spin={dashboardLoading || stepsLoading || rulesLoading} />}
            onClick={handleRefreshAll}
          >
            {t("opc.industry.refresh")}
          </Button>
        </Space>
      </div>

      {/* 行业运行时 - KPI 仪表盘 */}
      <Card
        style={{ marginBottom: 24 }}
        title={
          <span>
            <DashboardOutlined style={{ marginRight: 8 }} />
            {t("opc.industry.dashboard.title")}
          </span>
        }
        extra={
          <Segmented
            value={kpiTimeRange}
            onChange={(v) => setKpiTimeRange(v as "7" | "30" | "90")}
            options={[
              { label: t("opc.industry.dashboard.7days"), value: "7" },
              { label: t("opc.industry.dashboard.30days"), value: "30" },
              { label: t("opc.industry.dashboard.90days"), value: "90" },
            ]}
          />
        }
      >
        {dashboardLoading
          ? (
            <div style={{ textAlign: "center", padding: 40 }}>
              <Spin />
            </div>
          )
          : dashboard && dashboard.kpis.length > 0
          ? (
            <>
              <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
                {dashboard.kpis.map((kpi) => (
                  <Col xs={12} sm={8} md={6} key={kpi.id}>
                    <Card size="small" className="h-full">
                      <Statistic
                        title={kpi.name}
                        value={kpi.value}
                        precision={2}
                        prefix={kpi.unit === "%" ? "%" : ""}
                        suffix={kpi.unit !== "%" ? kpi.unit : ""}
                        valueStyle={{
                          color: kpi.trend === "up"
                            ? "#3f8600"
                            : kpi.trend === "down"
                            ? "#cf1322"
                            : undefined,
                        }}
                      />
                      {kpi.change_percent !== undefined && (
                        <Text
                          type={kpi.trend === "down" ? "danger" : "secondary"}
                          style={{ fontSize: 12 }}
                        >
                          {kpi.trend === "up" ? "↑" : kpi.trend === "down" ? "↓" : "→"}{" "}
                          {Math.abs(kpi.change_percent).toFixed(1)}%
                        </Text>
                      )}
                    </Card>
                  </Col>
                ))}
              </Row>
              {dashboard.summary && (
                <Alert
                  type="info"
                  showIcon
                  message={dashboard.summary}
                  style={{ marginTop: 8 }}
                />
              )}
            </>
          )
          : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("opc.industry.dashboard.noData")}
            />
          )}
      </Card>

      {/* 行业工作流步骤 */}
      <Card
        style={{ marginBottom: 24 }}
        title={
          <span>
            <LineChartOutlined style={{ marginRight: 8 }} />
            {t("opc.industry.workflowSteps.title")}
          </span>
        }
      >
        {stepsLoading
          ? (
            <div style={{ textAlign: "center", padding: 40 }}>
              <Spin />
            </div>
          )
          : workflowSteps.length > 0
          ? (
            <Steps
              direction="vertical"
              current={-1}
              items={workflowSteps.map((step) => ({
                title: (
                  <Space>
                    <Text strong>{step.name}</Text>
                    <Tag color="blue">{t("opc.industry.workflowSteps.step")} {step.order}</Tag>
                  </Space>
                ),
                description: step.description,
                status: step.status === "completed" ? "finish" : step.status === "active" ? "process" : "wait",
              }))}
            />
          )
          : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("opc.industry.workflowSteps.noData")}
            />
          )}
      </Card>

      {/* 行业自动化规则 */}
      <Card
        style={{ marginBottom: 24 }}
        title={
          <span>
            <ThunderboltOutlined style={{ marginRight: 8 }} />
            {t("opc.industry.rules.title")}
          </span>
        }
        extra={
          <Button
            type="primary"
            size="small"
            icon={<PlayCircleOutlined />}
            loading={rulesRunning}
            onClick={handleRunRules}
            disabled={automationRules.filter((r) => r.enabled).length === 0}
          >
            {t("opc.industry.rules.runAll")}
          </Button>
        }
      >
        {rulesLoading
          ? (
            <div style={{ textAlign: "center", padding: 40 }}>
              <Spin />
            </div>
          )
          : automationRules.length > 0
          ? (
            <Row gutter={[16, 16]}>
              {automationRules.map((rule) => (
                <Col xs={24} sm={12} md={8} key={rule.id}>
                  <Card
                    size="small"
                    title={
                      <Space>
                        <Text strong>{rule.name}</Text>
                        <Badge
                          status={rule.enabled ? "success" : "default"}
                          text={rule.enabled
                            ? t("opc.industry.rules.enabled")
                            : t("opc.industry.rules.disabled")}
                        />
                      </Space>
                    }
                  >
                    <div style={{ marginBottom: 8 }}>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        {t("opc.industry.rules.conditions")}:
                      </Text>
                      <div style={{ marginTop: 4 }}>
                        {rule.conditions.map((cond, i) => (
                          <Tag key={i} color="blue" style={{ marginBottom: 2 }}>
                            {cond.type}
                          </Tag>
                        ))}
                      </div>
                    </div>
                    <div>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        {t("opc.industry.rules.actions")}:
                      </Text>
                      <div style={{ marginTop: 4 }}>
                        {rule.actions.map((act, i) => (
                          <Tag key={i} color="green" style={{ marginBottom: 2 }}>
                            {act.type}
                          </Tag>
                        ))}
                      </div>
                    </div>
                  </Card>
                </Col>
              ))}
            </Row>
          )
          : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("opc.industry.rules.noData")}
            />
          )}
      </Card>

      {/* 行业分析决策引擎（对接 opc_execute_analysis 命令） */}
      <Card
        style={{ marginBottom: 24 }}
        title={
          <span>
            <BarChartOutlined style={{ marginRight: 8 }} />
            {t("opc.industry.analysis.title")}
          </span>
        }
        extra={
          <Space>
            <Segmented
              value={String(decisionDays)}
              onChange={(v) => setDecisionDays(Number(v))}
              options={[
                { label: t("opc.industry.analysis.timeRange7d"), value: "7" },
                { label: t("opc.industry.analysis.timeRange30d"), value: "30" },
                { label: t("opc.industry.analysis.timeRange90d"), value: "90" },
              ]}
            />
            <Button
              type="primary"
              icon={<PlayCircleOutlined />}
              loading={decisionLoading}
              onClick={handleExecuteAnalysis}
            >
              {t("opc.industry.analysis.execute")}
            </Button>
          </Space>
        }
      >
        {decisionLoading
          ? (
            <div style={{ textAlign: "center", padding: 40 }}>
              <Spin />
            </div>
          )
          : decision
          ? (
            <>
              {/* 决策摘要 */}
              <Alert
                type={decision.risk_level === "high"
                  ? "error"
                  : decision.risk_level === "medium"
                  ? "warning"
                  : "success"}
                showIcon
                message={decision.summary}
                description={t("opc.industry.analysis.riskLevel") + ": " + decision.risk_level}
                style={{ marginBottom: 16 }}
              />
              {/* 置信度 */}
              <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
                <Col xs={12} sm={8}>
                  <Card size="small">
                    <Progress
                      type="dashboard"
                      percent={Math.round(decision.confidence * 100)}
                      format={(p) => `${p}%`}
                    />
                    <div style={{ textAlign: "center", marginTop: 8 }}>
                      <Text type="secondary">{t("opc.industry.analysis.confidence")}</Text>
                    </div>
                  </Card>
                </Col>
                <Col xs={12} sm={8}>
                  <Card size="small">
                    <Statistic
                      title={t("opc.industry.analysis.decisionType")}
                      value={decision.decision_type}
                    />
                  </Card>
                </Col>
                <Col xs={12} sm={8}>
                  <Card size="small">
                    <Statistic
                      title={t("opc.industry.analysis.riskLevelTitle")}
                      value={decision.risk_level}
                      valueStyle={{
                        color: decision.risk_level === "high"
                          ? "#cf1322"
                          : decision.risk_level === "medium"
                          ? "#d48806"
                          : "#3f8600",
                      }}
                    />
                  </Card>
                </Col>
              </Row>
              {/* 建议列表 */}
              {decision.recommendations.length > 0 && <Divider>{t("opc.industry.analysis.recommendations")}</Divider>}
              <Timeline
                items={decision.recommendations.map((rec) => ({
                  color: rec.type === "action" ? "blue" : rec.type === "warning" ? "red" : "green",
                  children: (
                    <Space direction="vertical">
                      <Space>
                        <Tag color={rec.priority === "high" ? "red" : rec.priority === "medium" ? "orange" : "blue"}>
                          {rec.priority}
                        </Tag>
                        <Tag color={rec.type === "action" ? "blue" : rec.type === "warning" ? "red" : "green"}>
                          {rec.type}
                        </Tag>
                      </Space>
                      <Text>{rec.description}</Text>
                    </Space>
                  ),
                }))}
              />
            </>
          )
          : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("opc.industry.analysis.noData")}
            />
          )}
      </Card>

      {/* 行业工作流执行（对接 opc_execute_workflow 命令） */}
      <Card
        style={{ marginBottom: 24 }}
        title={
          <span>
            <ThunderboltOutlined style={{ marginRight: 8 }} />
            {t("opc.industry.workflow.executionTitle")}
          </span>
        }
        extra={
          <Button
            type="primary"
            icon={<PlayCircleOutlined />}
            loading={workflowExecuting}
            onClick={handleExecuteWorkflow}
          >
            {t("opc.industry.workflow.execute")}
          </Button>
        }
      >
        {workflowExecuting
          ? (
            <div style={{ textAlign: "center", padding: 40 }}>
              <Spin tip={t("opc.industry.workflow.executing")} />
            </div>
          )
          : workflowResult
          ? (
            <>
              {/* 执行状态 */}
              <Alert
                type={workflowResult.status === "completed" ? "success" : "error"}
                showIcon
                message={t("opc.industry.workflow.status_" + workflowResult.status)}
                description={workflowResult.error_message
                  || `${t("opc.industry.workflow.duration")}: ${(workflowResult.duration_ms / 1000).toFixed(2)}s`}
                style={{ marginBottom: 16 }}
              />
              {/* 节点执行结果 */}
              <Collapse
                items={workflowResult.node_results.map((node) => ({
                  key: node.id,
                  label: (
                    <Space>
                      <Tag color={node.status === "completed" ? "green" : node.status === "failed" ? "red" : "default"}>
                        {node.status}
                      </Tag>
                      <Text strong>{node.name}</Text>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        ({(node.duration_ms / 1000).toFixed(2)}s)
                      </Text>
                    </Space>
                  ),
                  children: node.output
                    ? (
                      <pre
                        style={{ maxHeight: 200, overflow: "auto", background: "#f5f5f5", padding: 8, borderRadius: 4 }}
                      >
                      {JSON.stringify(node.output, null, 2)}
                      </pre>
                    )
                    : null,
                }))}
              />
            </>
          )
          : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("opc.industry.workflow.noData")}
            />
          )}
      </Card>

      {/* 行业学习指标（对接 opc_get_learning_metrics 命令） */}
      <Card
        style={{ marginBottom: 24 }}
        title={
          <span>
            <FundProjectionScreenOutlined style={{ marginRight: 8 }} />
            {t("opc.industry.metrics.title")}
          </span>
        }
        extra={
          <Button
            icon={<SyncOutlined spin={metricsLoading} />}
            loading={metricsLoading}
            onClick={handleGetLearningMetrics}
          >
            {t("opc.industry.metrics.refresh")}
          </Button>
        }
      >
        {metricsLoading
          ? (
            <div style={{ textAlign: "center", padding: 40 }}>
              <Spin />
            </div>
          )
          : learningMetrics
          ? (
            <Row gutter={[16, 16]}>
              <Col xs={12} sm={6}>
                <Card size="small">
                  <Statistic
                    title={t("opc.industry.metrics.totalSamples")}
                    value={learningMetrics.total_samples}
                    prefix={<BookOutlined />}
                  />
                </Card>
              </Col>
              <Col xs={12} sm={6}>
                <Card size="small" title={t("opc.industry.metrics.decisionAccuracy")}>
                  <Progress
                    type="circle"
                    percent={Math.round(learningMetrics.decision_accuracy * 100)}
                  />
                </Card>
              </Col>
              <Col xs={12} sm={6}>
                <Card size="small" title={t("opc.industry.metrics.riskAccuracy")}>
                  <Progress
                    type="circle"
                    percent={Math.round(learningMetrics.risk_prediction_accuracy * 100)}
                  />
                </Card>
              </Col>
              <Col xs={12} sm={6}>
                <Card size="small">
                  <Statistic
                    title={t("opc.industry.metrics.avgFeedback")}
                    value={learningMetrics.avg_feedback_score}
                    precision={2}
                    prefix={<BulbOutlined />}
                  />
                  <Tag
                    color={learningMetrics.improvement_trend === "improving"
                      ? "green"
                      : learningMetrics.improvement_trend === "stable"
                      ? "blue"
                      : "red"}
                    style={{ marginTop: 8 }}
                  >
                    {t("opc.industry.metrics.trend_" + learningMetrics.improvement_trend)}
                  </Tag>
                </Card>
              </Col>
            </Row>
          )
          : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("opc.industry.metrics.noData")}
            />
          )}
      </Card>

      {/* 专属操作入口 */}
      <Card
        style={{ marginBottom: 24 }}
        styles={{ body: { padding: 20 } }}
      >
        <Title level={5} style={{ marginBottom: 16 }}>
          <ThunderboltOutlined style={{ marginRight: 8 }} />
          {t("opc.industry.exclusiveActions")}
        </Title>
        <Row gutter={[16, 16]}>
          {config.actions.map((action) => (
            <Col xs={24} sm={12} md={12} lg={6} key={action.key}>
              <Card
                hoverable
                size="small"
                onClick={() => handleAction(action)}
                style={{
                  cursor: "pointer",
                  border: "1px solid var(--color-border)",
                  transition: "all 0.2s",
                }}
                styles={{ body: { padding: 16 } }}
              >
                <div style={{ display: "flex", alignItems: "flex-start", gap: 12 }}>
                  <div
                    style={{
                      fontSize: 28,
                      color: "var(--color-primary)",
                      flexShrink: 0,
                    }}
                  >
                    {action.icon}
                  </div>
                  <div style={{ flex: 1 }}>
                    <Text strong style={{ display: "block", marginBottom: 4 }}>
                      {t(`${actionsPrefix}.${action.key}.label`)}
                    </Text>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {t(`${actionsPrefix}.${action.key}.description`)}
                    </Text>
                    {action.type === "workflow" && (
                      <Tag color="orange" style={{ marginTop: 8 }}>
                        {t("opc.industry.workflowTag")}
                      </Tag>
                    )}
                  </div>
                </div>
              </Card>
            </Col>
          ))}
        </Row>
      </Card>

      {/* 行业预设工作流 */}
      <Card
        title={
          <span>
            <CodeOutlined style={{ marginRight: 8 }} />
            {t("opc.industry.exclusiveWorkflows")}
          </span>
        }
      >
        <Row gutter={[16, 16]}>
          {config.workflows.map((wf) => (
            <Col xs={24} sm={12} md={8} key={wf.id}>
              <Card
                size="small"
                title={
                  <Space>
                    <FileTextOutlined />
                    {t(`${workflowsPrefix}.${wf.id}.name`)}
                  </Space>
                }
                extra={<Tag color="blue">v{wf.version}</Tag>}
              >
                <Paragraph type="secondary" style={{ fontSize: 13, marginBottom: 12 }}>
                  {t(`${workflowsPrefix}.${wf.id}.description`)}
                </Paragraph>
                <Button
                  type="primary"
                  size="small"
                  icon={<PlayCircleOutlined />}
                  block
                  onClick={() => handleUseWorkflow(wf)}
                >
                  {t("opc.industry.useThisWorkflow")}
                </Button>
              </Card>
            </Col>
          ))}
        </Row>
      </Card>

      {/* 学习与进化配置面板 */}
      <Card
        title={
          <span>
            <BulbOutlined style={{ marginRight: 8 }} />
            {t("opc.industry.learning.title")}
          </span>
        }
        extra={
          <Button
            size="small"
            icon={<SyncOutlined spin={learningLoading} />}
            onClick={() => learningStore.clearCache()}
          >
            {t("opc.industry.learning.actions.refreshConfig")}
          </Button>
        }
      >
        <Paragraph type="secondary" style={{ marginBottom: 16 }}>
          {t("opc.industry.learning.subtitle")}
        </Paragraph>

        {learningLoading && !learningConfig
          ? (
            <div style={{ textAlign: "center", padding: 24 }}>
              <Spin tip={t("opc.industry.learning.actions.loadFailed", { error: "..." })} />
            </div>
          )
          : learningConfig
          ? (
            <Row gutter={[16, 16]}>
              {/* 版本信息 */}
              <Col span={24}>
                <Space>
                  <Text type="secondary">{t("opc.industry.learning.version")}:</Text>
                  <Tag color="blue">v{learningConfig.version}</Tag>
                </Space>
              </Col>

              {/* 反思评估 */}
              <Col xs={24} sm={12} md={6}>
                <Card size="small" style={{ height: "100%" }}>
                  <Space direction="vertical" size={8} style={{ width: "100%" }}>
                    <Space>
                      <ExperimentOutlined />
                      <strong>{t("opc.industry.learning.reflection.label")}</strong>
                      <Tag color={learningConfig.reflectionEnabled ? "green" : "default"}>
                        {learningConfig.reflectionEnabled
                          ? t("opc.industry.learning.reflection.enabled")
                          : t("opc.industry.learning.reflection.disabled")}
                      </Tag>
                    </Space>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {t("opc.industry.learning.reflection.description")}
                    </Text>
                    <Button
                      size="small"
                      icon={<BulbOutlined />}
                      onClick={handleReflect}
                      disabled={!learningConfig.reflectionEnabled}
                      block
                    >
                      {t("opc.industry.learning.reflection.trigger")}
                    </Button>
                  </Space>
                </Card>
              </Col>

              {/* 工作流进化 */}
              <Col xs={24} sm={12} md={6}>
                <Card size="small" style={{ height: "100%" }}>
                  <Space direction="vertical" size={8} style={{ width: "100%" }}>
                    <Space>
                      <ThunderboltOutlined />
                      <strong>{t("opc.industry.learning.evolution.label")}</strong>
                      <Tag color={learningConfig.evolutionEnabled ? "green" : "default"}>
                        {learningConfig.evolutionEnabled
                          ? t("opc.industry.learning.evolution.enabled")
                          : t("opc.industry.learning.evolution.disabled")}
                      </Tag>
                    </Space>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {t("opc.industry.learning.evolution.description")}
                    </Text>
                    <Button
                      size="small"
                      icon={<SyncOutlined />}
                      onClick={handleEvolve}
                      disabled={!learningConfig.evolutionEnabled}
                      block
                    >
                      {t("opc.industry.learning.evolution.trigger")}
                    </Button>
                  </Space>
                </Card>
              </Col>

              {/* 自我改进 */}
              <Col xs={24} sm={12} md={6}>
                <Card size="small" style={{ height: "100%" }}>
                  <Space direction="vertical" size={8} style={{ width: "100%" }}>
                    <Space>
                      <RocketOutlined />
                      <strong>{t("opc.industry.learning.selfImprovement.label")}</strong>
                      <Tag color={learningConfig.selfImprovementEnabled ? "green" : "default"}>
                        {learningConfig.selfImprovementEnabled
                          ? t("opc.industry.learning.selfImprovement.enabled")
                          : t("opc.industry.learning.selfImprovement.disabled")}
                      </Tag>
                    </Space>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {t("opc.industry.learning.selfImprovement.description")}
                    </Text>
                    <Button
                      size="small"
                      icon={<PlayCircleOutlined />}
                      onClick={handleSelfImprove}
                      disabled={!learningConfig.selfImprovementEnabled}
                      block
                    >
                      {t("opc.industry.learning.selfImprovement.trigger")}
                    </Button>
                  </Space>
                </Card>
              </Col>

              {/* 强化学习 */}
              <Col xs={24} sm={12} md={6}>
                <Card size="small" style={{ height: "100%" }}>
                  <Space direction="vertical" size={8} style={{ width: "100%" }}>
                    <Space>
                      <FundProjectionScreenOutlined />
                      <strong>{t("opc.industry.learning.reinforcementLearning.label")}</strong>
                      <Tag color={learningConfig.reinforcementLearningEnabled ? "green" : "default"}>
                        {learningConfig.reinforcementLearningEnabled
                          ? t("opc.industry.learning.reinforcementLearning.enabled")
                          : t("opc.industry.learning.reinforcementLearning.disabled")}
                      </Tag>
                    </Space>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {t("opc.industry.learning.reinforcementLearning.description")}
                    </Text>
                  </Space>
                </Card>
              </Col>
            </Row>
          )
          : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("opc.industry.learning.actions.configNotFound")}
            />
          )}
      </Card>

      {/* RL 强化学习面板 */}
      <Card
        style={{ marginTop: 16 }}
        title={
          <span>
            <FundProjectionScreenOutlined style={{ marginRight: 8 }} />
            {t("opc.rl.panelTitle")}
          </span>
        }
      >
        <RLLearningPanel industryId={industryId} />
      </Card>
    </div>
  );
}
