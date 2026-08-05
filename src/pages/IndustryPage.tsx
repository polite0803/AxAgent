// SPDX-License-Identifier: AGPL-3.0-only

import { RLLearningPanel } from "@/components/opc/RLLearningPanel";
import { invoke } from "@/lib/invoke";
import { evolveWorkflow, reflectOnWorkflow, runSelfImprovement } from "@/lib/opcLearning";
import { useConversationStore, useIndustryLearningStore, useSettingsStore } from "@/stores";
import type { IndustryLearningConfig } from "@/types";
import {
  ApiOutlined,
  AuditOutlined,
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
  Empty,
  Row,
  Segmented,
  Space,
  Spin,
  Statistic,
  Steps,
  Tag,
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

/** 9 个行业专属配置 - 文本内容通过 i18n 获取 */
const INDUSTRY_CONFIGS: Record<string, IndustryConfig> = {
  "ai-research": {
    actions: [
      { key: "ai-paper", icon: <FileSearchOutlined />, type: "conversation" },
      { key: "ai-benchmark", icon: <LineChartOutlined />, type: "conversation" },
      { key: "ai-app", icon: <ExperimentOutlined />, type: "conversation" },
      { key: "ai-report", icon: <FileTextOutlined />, type: "workflow" },
    ],
    workflows: [
      { id: "wf-ai-research-1", name: "", description: "", version: "1.0" },
      { id: "wf-ai-research-2", name: "", description: "", version: "1.0" },
      { id: "wf-ai-research-3", name: "", description: "", version: "1.0" },
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
      { id: "wf-sd-1", name: "", description: "", version: "1.0" },
      { id: "wf-sd-2", name: "", description: "", version: "1.0" },
      { id: "wf-sd-3", name: "", description: "", version: "1.0" },
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
      { id: "wf-fi-1", name: "", description: "", version: "1.0" },
      { id: "wf-fi-2", name: "", description: "", version: "1.0" },
      { id: "wf-fi-3", name: "", description: "", version: "1.0" },
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
      { id: "wf-sg-1", name: "", description: "", version: "1.0" },
      { id: "wf-sg-2", name: "", description: "", version: "1.0" },
      { id: "wf-sg-3", name: "", description: "", version: "1.0" },
    ],
  },
  "content-media": {
    actions: [
      { key: "cm-article", icon: <EditOutlined />, type: "workflow" },
      { key: "cm-seo", icon: <SearchOutlined />, type: "conversation" },
      { key: "cm-video", icon: <VideoCameraOutlined />, type: "workflow" },
      { key: "cm-calendar", icon: <BookOutlined />, type: "workflow" },
    ],
    workflows: [
      { id: "wf-cm-1", name: "", description: "", version: "1.0" },
      { id: "wf-cm-2", name: "", description: "", version: "1.0" },
      { id: "wf-cm-3", name: "", description: "", version: "1.0" },
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
      { id: "wf-ic-1", name: "", description: "", version: "1.0" },
      { id: "wf-ic-2", name: "", description: "", version: "1.0" },
      { id: "wf-ic-3", name: "", description: "", version: "1.0" },
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
      { id: "wf-ac-1", name: "", description: "", version: "1.0" },
      { id: "wf-ac-2", name: "", description: "", version: "1.0" },
      { id: "wf-ac-3", name: "", description: "", version: "1.0" },
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
      { id: "wf-ec-1", name: "", description: "", version: "1.0" },
      { id: "wf-ec-2", name: "", description: "", version: "1.0" },
      { id: "wf-ec-3", name: "", description: "", version: "1.0" },
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
      { id: "wf-ed-1", name: "", description: "", version: "1.0" },
      { id: "wf-ed-2", name: "", description: "", version: "1.0" },
      { id: "wf-ed-3", name: "", description: "", version: "1.0" },
    ],
  },
};

/** 行业操作面板 — 根据行业 ID 加载专属配置，所有文本通过 i18n 获取 */
export function IndustryPage() {
  const { t } = useTranslation();
  const params = useParams<{ industryId: string }>();
  const location = useLocation();
  const navigate = useNavigate();
  const { message } = App.useApp();

  const industryId = params?.industryId
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
        }>("opc_get_industry_pack", { industry_id: industryId });
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
        { industry_id: industryId, days },
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
        { industry_id: industryId },
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
        { industry_id: industryId },
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
        industry_id: industryId,
        entity_type: "customer",
        entity_id: "manual_trigger",
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

  /** 触发反思 */
  const handleReflect = async () => {
    if (!learningConfig?.reflection_enabled) {
      message.warning(t("opc.industry.learning.reflection.notEnabled"));
      return;
    }
    try {
      message.loading({ content: t("opc.industry.learning.reflection.triggerDesc"), key: "reflect" });
      await reflectOnWorkflow({
        industry_id: industryId,
        workflow_id: `industry_${industryId}`,
        workflow_result: { status: "manual_triggered" },
      });
      message.success({ content: t("opc.industry.learning.reflection.triggerSuccess"), key: "reflect" });
    } catch (e) {
      message.error(t("opc.industry.learning.reflection.triggerFailed", { error: String(e) }));
    }
  };

  /** 触发进化 */
  const handleEvolve = async () => {
    if (!learningConfig?.evolution_enabled) {
      message.warning(t("opc.industry.learning.evolution.notEnabled"));
      return;
    }
    try {
      message.loading({ content: t("opc.industry.learning.evolution.triggerDesc"), key: "evolve" });
      await evolveWorkflow({
        industry_id: industryId,
        workflow_id: `industry_${industryId}`,
        reason: "manual_optimization",
      });
      message.success({ content: t("opc.industry.learning.evolution.triggerSuccess"), key: "evolve" });
    } catch (e) {
      message.error(t("opc.industry.learning.evolution.triggerFailed", { error: String(e) }));
    }
  };

  /** 执行自我改进 */
  const handleSelfImprove = async () => {
    if (!learningConfig?.self_improvement_enabled) {
      message.warning(t("opc.industry.learning.selfImprovement.notEnabled"));
      return;
    }
    try {
      message.loading({ content: t("opc.industry.learning.selfImprovement.triggerDesc"), key: "selfImprove" });
      await runSelfImprovement({
        industry_id: industryId,
        target: "overall_performance",
      });
      message.success({ content: t("opc.industry.learning.selfImprovement.triggerSuccess"), key: "selfImprove" });
    } catch (e) {
      message.error(t("opc.industry.learning.selfImprovement.triggerFailed", { error: String(e) }));
    }
  };

  /** 执行行业操作 - 调用后端命令获取真实 prompt */
  const handleAction = async (action: ActionItem) => {
    if (!settings?.default_provider_id || !settings?.default_model_id) {
      message.warning(t("opc.industry.noProviderConfig"));
      navigate("/settings/providers");
      return;
    }

    if (action.type === "workflow") {
      navigate(`/workflow/new?industry=${industryId}&template=${action.key}`);
      return;
    }

    try {
      const promptConfig = await invoke<{
        systemPrompt: string;
        userPrompt: string;
        actionKey: string;
        actionLabel: string;
        industryId: string;
      }>("opc_build_industry_prompt", {
        industry_id: industryId,
        action_key: action.key,
      });

      const conv = await createConversation(
        `${promptConfig.actionLabel} - ${manifest?.name || ""}`,
        settings.default_model_id,
        settings.default_provider_id,
        {
          system_prompt: promptConfig.systemPrompt,
        },
      );
      if (conv?.id) {
        navigate(`/chat?conversationId=${conv.id}&prompt=${encodeURIComponent(promptConfig.userPrompt)}`);
      }
    } catch (e) {
      message.error(t("opc.industry.loadFailed", { error: String(e) }));
    }
  };

  /** 使用预设工作流 */
  const handleUseWorkflow = async (wf: IndustryWorkflow) => {
    if (!settings?.default_provider_id || !settings?.default_model_id) {
      message.warning(t("opc.industry.noProviderConfig"));
      navigate("/settings/providers");
      return;
    }

    const wfName = t(`${workflowsPrefix}.${wf.id}.name`);

    try {
      const conv = await createConversation(
        t("opc.industry.executeSuffix", { name: wfName }),
        settings.default_model_id,
        settings.default_provider_id,
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
                      <Tag color={learningConfig.reflection_enabled ? "green" : "default"}>
                        {learningConfig.reflection_enabled
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
                      disabled={!learningConfig.reflection_enabled}
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
                      <Tag color={learningConfig.evolution_enabled ? "green" : "default"}>
                        {learningConfig.evolution_enabled
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
                      disabled={!learningConfig.evolution_enabled}
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
                      <Tag color={learningConfig.self_improvement_enabled ? "green" : "default"}>
                        {learningConfig.self_improvement_enabled
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
                      disabled={!learningConfig.self_improvement_enabled}
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
                      <Tag color={learningConfig.reinforcement_learning_enabled ? "green" : "default"}>
                        {learningConfig.reinforcement_learning_enabled
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
