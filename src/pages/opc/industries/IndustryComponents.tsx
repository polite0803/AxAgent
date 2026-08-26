// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 行业 UI 组件 — 可复用的行业展示组件
 */

import { useConversationStore, useSettingsStore } from "@/stores";
import {
  ApiOutlined,
  BarChartOutlined,
  BulbOutlined,
  CodeOutlined,
  DashboardOutlined,
  FileTextOutlined,
  FundProjectionScreenOutlined,
  LineChartOutlined,
  PlayCircleOutlined,
  RocketOutlined,
  SyncOutlined,
  ThunderboltOutlined,
} from "@ant-design/icons";
import {
  Alert,
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
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import type { ActionItem, IndustryConfig, IndustryWorkflow } from "./types";
import { useIndustryData } from "./useIndustryData";

const { Title, Paragraph, Text } = Typography;

/** 行业页面属性 */
export interface IndustryPageProps {
  industryId: string;
  config: IndustryConfig;
}

/**
 * 行业仪表盘组件
 */
export function IndustryDashboard({
  dashboard,
  loading,
  kpiTimeRange,
  onTimeRangeChange,
  onRefresh,
}: {
  dashboard: ReturnType<typeof useIndustryData>["dashboard"];
  loading: boolean;
  kpiTimeRange: "7" | "30" | "90";
  onTimeRangeChange: (range: "7" | "30" | "90") => void;
  onRefresh: () => void;
}) {
  const { t } = useTranslation();

  return (
    <Card
      style={{ marginBottom: 24 }}
      title={
        <span>
          <DashboardOutlined style={{ marginRight: 8 }} />
          {t("opc.industry.dashboard.title")}
        </span>
      }
      extra={
        <Space>
          <Segmented
            value={kpiTimeRange}
            onChange={(v) => onTimeRangeChange(v as "7" | "30" | "90")}
            options={[
              { label: t("opc.industry.dashboard.7days"), value: "7" },
              { label: t("opc.industry.dashboard.30days"), value: "30" },
              { label: t("opc.industry.dashboard.90days"), value: "90" },
            ]}
          />
          <Button icon={<SyncOutlined spin={loading} />} onClick={onRefresh}>
            {t("opc.industry.refresh")}
          </Button>
        </Space>
      }
    >
      {loading
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
                        color: kpi.trend === "improving"
                          ? "#3f8600"
                          : kpi.trend === "declining"
                          ? "#cf1322"
                          : undefined,
                      }}
                    />
                    {kpi.trend && (
                      <Text
                        type={kpi.trend === "declining" ? "danger" : "secondary"}
                        style={{ fontSize: 12 }}
                      >
                        {kpi.trend === "improving" ? "↑" : kpi.trend === "declining" ? "↓" : "→"} {kpi.trend}
                      </Text>
                    )}
                  </Card>
                </Col>
              ))}
            </Row>
          </>
        )
        : (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t("opc.industry.dashboard.noData")}
          />
        )}
    </Card>
  );
}

/**
 * 行业工作流步骤组件
 */
export function IndustryWorkflowSteps({
  steps,
  loading,
}: {
  steps: ReturnType<typeof useIndustryData>["workflowSteps"];
  loading: boolean;
}) {
  const { t } = useTranslation();

  return (
    <Card
      style={{ marginBottom: 24 }}
      title={
        <span>
          <LineChartOutlined style={{ marginRight: 8 }} />
          {t("opc.industry.workflowSteps.title")}
        </span>
      }
    >
      {loading
        ? (
          <div style={{ textAlign: "center", padding: 40 }}>
            <Spin />
          </div>
        )
        : steps.length > 0
        ? (
          <Steps
            direction="vertical"
            current={-1}
            items={steps.map((step) => ({
              title: (
                <Space>
                  <Text strong>{step.name}</Text>
                  <Tag color="blue">
                    {t("opc.industry.workflowSteps.step")} {step.step_order}
                  </Tag>
                </Space>
              ),
              description: step.description,
              status: step.success_rate > 0.9 ? "finish" : step.success_rate > 0.5 ? "process" : "wait",
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
  );
}

/**
 * 行业自动化规则组件
 */
export function IndustryAutomationRules({
  rules,
  loading,
  running,
  onRunAll,
}: {
  rules: ReturnType<typeof useIndustryData>["automationRules"];
  loading: boolean;
  running: boolean;
  onRunAll: () => Promise<string[]>;
}) {
  const { t } = useTranslation();

  return (
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
          loading={running}
          onClick={onRunAll}
          disabled={rules.filter((r) => r.enabled).length === 0}
        >
          {t("opc.industry.rules.runAll")}
        </Button>
      }
    >
      {loading
        ? (
          <div style={{ textAlign: "center", padding: 40 }}>
            <Spin />
          </div>
        )
        : rules.length > 0
        ? (
          <Row gutter={[16, 16]}>
            {rules.map((rule) => (
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
                      <Tag color="blue">{rule.trigger_event}</Tag>
                    </div>
                  </div>
                  <div>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {t("opc.industry.rules.actions")}:
                    </Text>
                    <div style={{ marginTop: 4 }}>
                      <Tag color="green">{rule.action}</Tag>
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
  );
}

/**
 * 行业分析决策组件
 */
export function IndustryAnalysisDecision({
  decision,
  loading,
  decisionDays,
  onDaysChange,
  onExecute,
}: {
  decision: ReturnType<typeof useIndustryData>["decision"];
  loading: boolean;
  decisionDays: number;
  onDaysChange: (days: number) => void;
  onExecute: () => Promise<void>;
}) {
  const { t } = useTranslation();

  return (
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
            onChange={(v) => onDaysChange(Number(v))}
            options={[
              { label: t("opc.industry.analysis.timeRange7d"), value: "7" },
              { label: t("opc.industry.analysis.timeRange30d"), value: "30" },
              { label: t("opc.industry.analysis.timeRange90d"), value: "90" },
            ]}
          />
          <Button
            type="primary"
            icon={<PlayCircleOutlined />}
            loading={loading}
            onClick={onExecute}
          >
            {t("opc.industry.analysis.execute")}
          </Button>
        </Space>
      }
    >
      {loading
        ? (
          <div style={{ textAlign: "center", padding: 40 }}>
            <Spin />
          </div>
        )
        : decision
        ? (
          <>
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
            {decision.recommendations.length > 0 && <Divider>{t("opc.industry.analysis.recommendations")}</Divider>}
            <Timeline
              items={decision.recommendations.map((rec) => ({
                color: rec.type === "action"
                  ? "blue"
                  : rec.type === "warning"
                  ? "red"
                  : "green",
                children: (
                  <Space direction="vertical">
                    <Space>
                      <Tag
                        color={rec.priority === "high"
                          ? "red"
                          : rec.priority === "medium"
                          ? "orange"
                          : "blue"}
                      >
                        {rec.priority}
                      </Tag>
                      <Tag
                        color={rec.type === "action"
                          ? "blue"
                          : rec.type === "warning"
                          ? "red"
                          : "green"}
                      >
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
  );
}

/**
 * 行业学习指标组件
 */
export function IndustryLearningMetrics({
  metrics,
  loading,
  onRefresh,
}: {
  metrics: ReturnType<typeof useIndustryData>["learningMetrics"];
  loading: boolean;
  onRefresh: () => Promise<void>;
}) {
  const { t } = useTranslation();

  return (
    <Card
      style={{ marginBottom: 24 }}
      title={
        <span>
          <FundProjectionScreenOutlined style={{ marginRight: 8 }} />
          {t("opc.industry.metrics.title")}
        </span>
      }
      extra={
        <Button icon={<SyncOutlined spin={loading} />} loading={loading} onClick={onRefresh}>
          {t("opc.industry.metrics.refresh")}
        </Button>
      }
    >
      {loading
        ? (
          <div style={{ textAlign: "center", padding: 40 }}>
            <Spin />
          </div>
        )
        : metrics
        ? (
          <Row gutter={[16, 16]}>
            <Col xs={12} sm={6}>
              <Card size="small">
                <Statistic
                  title={t("opc.industry.metrics.totalSamples")}
                  value={metrics.total_samples}
                  prefix={<BulbOutlined />}
                />
              </Card>
            </Col>
            <Col xs={12} sm={6}>
              <Card size="small" title={t("opc.industry.metrics.decisionAccuracy")}>
                <Progress
                  type="circle"
                  percent={Math.round(metrics.decision_accuracy * 100)}
                />
              </Card>
            </Col>
            <Col xs={12} sm={6}>
              <Card size="small" title={t("opc.industry.metrics.riskAccuracy")}>
                <Progress
                  type="circle"
                  percent={Math.round(metrics.risk_prediction_accuracy * 100)}
                />
              </Card>
            </Col>
            <Col xs={12} sm={6}>
              <Card size="small">
                <Statistic
                  title={t("opc.industry.metrics.avgFeedback")}
                  value={metrics.avg_feedback_score}
                  precision={2}
                  prefix={<BulbOutlined />}
                />
                <Tag
                  color={metrics.improvement_trend === "improving"
                    ? "green"
                    : metrics.improvement_trend === "stable"
                    ? "blue"
                    : "red"}
                  style={{ marginTop: 8 }}
                >
                  {t("opc.industry.metrics.trend_" + metrics.improvement_trend)}
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
  );
}

/**
 * 行业操作面板组件
 */
export function IndustryActionsPanel({
  industryId,
  actions,
}: {
  industryId: string;
  actions: ActionItem[];
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const createConversation = useConversationStore((s) => s.createConversation);
  const settings = useSettingsStore((s) => s.settings);
  const { message } = (window as unknown as {
    antd?: { app?: { useApp: () => { message: { warning: (msg: string) => void; error: (msg: string) => void } } } };
  }).antd?.app?.useApp() || {
    message: { warning: (msg: string) => console.warn(msg), error: (msg: string) => console.error(msg) },
  };

  const actionsPrefix = `opc.industry.actions.${industryId}`;

  const handleAction = async (action: ActionItem) => {
    if (!settings?.defaultModel?.a || !settings?.defaultModel?.b) {
      message.warning(t("opc.industry.noProviderConfig"));
      navigate("/settings/providers");
      return;
    }

    if (action.type === "workflow") {
      const templateId = action.template_id || action.key;
      navigate(`/workflow/new?industry=${industryId}&template=${templateId}`);
      return;
    }

    const actionLabel = action.label || action.key;

    try {
      const { invoke } = await import("@/lib/invoke");
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
        promptConfig.actionLabel,
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
        actionLabel,
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

  return (
    <Card style={{ marginBottom: 24 }} styles={{ body: { padding: 20 } }}>
      <Title level={5} style={{ marginBottom: 16 }}>
        <ThunderboltOutlined style={{ marginRight: 8 }} />
        {t("opc.industry.exclusiveActions")}
      </Title>
      <Row gutter={[16, 16]}>
        {actions.map((action) => (
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
  );
}

/**
 * 行业工作流面板组件
 */
export function IndustryWorkflowsPanel({
  industryId,
  workflows,
}: {
  industryId: string;
  workflows: IndustryWorkflow[];
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const createConversation = useConversationStore((s) => s.createConversation);
  const settings = useSettingsStore((s) => s.settings);
  const { message } = (window as unknown as {
    antd?: { app?: { useApp: () => { message: { warning: (msg: string) => void; error: (msg: string) => void } } } };
  }).antd?.app?.useApp() || {
    message: { warning: (msg: string) => console.warn(msg), error: (msg: string) => console.error(msg) },
  };

  const workflowsPrefix = `opc.industry.workflows.${industryId}`;

  const handleUseWorkflow = async (wf: IndustryWorkflow) => {
    if (!settings?.defaultModel?.a || !settings?.defaultModel?.b) {
      message.warning(t("opc.industry.noProviderConfig"));
      navigate("/settings/providers");
      return;
    }

    try {
      const conv = await createConversation(
        t("opc.industry.executeSuffix", { name: wf.name || wf.id }),
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

  return (
    <Card
      title={
        <span>
          <CodeOutlined style={{ marginRight: 8 }} />
          {t("opc.industry.exclusiveWorkflows")}
        </span>
      }
    >
      <Row gutter={[16, 16]}>
        {workflows.map((wf) => (
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
  );
}

/**
 * 行业工作流执行组件
 */
export function IndustryWorkflowExecution({
  workflowResult,
  executing,
  onExecute,
}: {
  workflowResult: ReturnType<typeof useIndustryData>["workflowResult"];
  executing: boolean;
  onExecute: () => Promise<void>;
}) {
  const { t } = useTranslation();

  return (
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
          loading={executing}
          onClick={onExecute}
        >
          {t("opc.industry.workflow.execute")}
        </Button>
      }
    >
      {executing
        ? (
          <div style={{ textAlign: "center", padding: 40 }}>
            <Spin tip={t("opc.industry.workflow.executing")} />
          </div>
        )
        : workflowResult
        ? (
          <>
            <Alert
              type={workflowResult.status === "success" ? "success" : "error"}
              showIcon
              message={t("opc.industry.workflow.status_" + workflowResult.status)}
              description={workflowResult.error
                || `${t("opc.industry.workflow.duration")}: ${(workflowResult.duration_ms / 1000).toFixed(2)}s`}
              style={{ marginBottom: 16 }}
            />
            {workflowResult.output && (
              <Collapse
                items={[
                  {
                    key: "output",
                    label: (
                      <Space>
                        <Tag color={workflowResult.status === "success" ? "green" : "red"}>
                          {workflowResult.status}
                        </Tag>
                        <Text strong>Output</Text>
                      </Space>
                    ),
                    children: (
                      <pre
                        style={{
                          maxHeight: 300,
                          overflow: "auto",
                          background: "#f5f5f5",
                          padding: 8,
                          borderRadius: 4,
                        }}
                      >
                      {JSON.stringify(workflowResult.output, null, 2)}
                      </pre>
                    ),
                  },
                ]}
              />
            )}
          </>
        )
        : (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t("opc.industry.workflow.noData")}
          />
        )}
    </Card>
  );
}

/**
 * 学习与进化配置面板
 */
export function IndustryLearningPanel({
  industryId: _industryId,
  learningConfig,
  onReflect,
  onEvolve,
  onSelfImprove,
}: {
  industryId: string;
  learningConfig: NonNullable<ReturnType<typeof useIndustryData>["learningConfig"]> | null;
  onReflect: () => Promise<void>;
  onEvolve: () => Promise<void>;
  onSelfImprove: () => Promise<void>;
}) {
  const { t } = useTranslation();
  const { message } =
    (window as unknown as { antd?: { app?: { useApp: () => { message: { warning: (msg: string) => void } } } } }).antd
      ?.app?.useApp() || {
      message: { warning: (msg: string) => console.warn(msg) },
    };

  if (!learningConfig) {
    return (
      <Card
        title={
          <span>
            <ApiOutlined style={{ marginRight: 8 }} />
            {t("opc.industry.learning.title")}
          </span>
        }
      >
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("opc.industry.learning.actions.configNotFound")}
        />
      </Card>
    );
  }

  return (
    <Card
      title={
        <span>
          <ApiOutlined style={{ marginRight: 8 }} />
          {t("opc.industry.learning.title")}
        </span>
      }
    >
      <Row gutter={[16, 16]}>
        {/* 反思 */}
        <Col xs={24} sm={12} md={6}>
          <Card size="small" style={{ height: "100%" }}>
            <Space direction="vertical" size={8} style={{ width: "100%" }}>
              <Space>
                <BulbOutlined />
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
                onClick={async () => {
                  if (!learningConfig.reflectionEnabled) {
                    message.warning(t("opc.industry.learning.reflection.notEnabled"));
                    return;
                  }
                  await onReflect();
                }}
                disabled={!learningConfig.reflectionEnabled}
                block
              >
                {t("opc.industry.learning.reflection.trigger")}
              </Button>
            </Space>
          </Card>
        </Col>

        {/* 进化 */}
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
                icon={<RocketOutlined />}
                onClick={async () => {
                  if (!learningConfig.evolutionEnabled) {
                    message.warning(t("opc.industry.learning.evolution.notEnabled"));
                    return;
                  }
                  await onEvolve();
                }}
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
                <PlayCircleOutlined />
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
                onClick={async () => {
                  if (!learningConfig.selfImprovementEnabled) {
                    message.warning(t("opc.industry.learning.selfImprovement.notEnabled"));
                    return;
                  }
                  await onSelfImprove();
                }}
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
    </Card>
  );
}

/**
 * 行业页面头部
 */
export function IndustryHeader({
  industryId,
  manifest,
  onRefresh,
  refreshing,
}: {
  industryId: string;
  manifest: { icon: string; name: string } | null;
  onRefresh: () => void;
  refreshing: boolean;
}) {
  const { t } = useTranslation();
  const industryKey = industryId.replace(/-/g, "_");

  return (
    <div style={{ marginBottom: 24 }}>
      <Space align="center" style={{ width: "100%", justifyContent: "space-between" }}>
        <div>
          <Title level={3} style={{ marginBottom: 8 }}>
            <span style={{ fontSize: 28, marginRight: 12 }}>{manifest?.icon || "🏢"}</span>
            {t(`opc.industries.${industryKey}`)}
          </Title>
          <Paragraph type="secondary">{t(`opc.industries.${industryKey}_desc`)}</Paragraph>
        </div>
        <Button icon={<SyncOutlined spin={refreshing} />} onClick={onRefresh}>
          {t("opc.industry.refresh")}
        </Button>
      </Space>
    </div>
  );
}

/**
 * 基础行业页面布局
 */
export function IndustryPageLayout({
  industryId,
  config,
  children,
}: IndustryPageProps & { children?: ReactNode }) {
  const { t } = useTranslation();
  const data = useIndustryData(industryId);

  if (data.loading) {
    return (
      <div style={{ padding: 48, textAlign: "center" }}>
        <Spin size="large" />
      </div>
    );
  }

  if (!data.manifest) {
    return (
      <div style={{ padding: 48, textAlign: "center" }}>
        <Empty description={t("opc.industry.notFound")} />
      </div>
    );
  }

  const handleRefreshAll = () => {
    data.loadDashboard();
    data.loadWorkflowSteps();
    data.loadAutomationRules();
    data.loadLearningMetrics();
  };

  const handleRunRules = async (): Promise<string[]> => {
    const triggered = await data.runAutomationRules();
    const { message } = (window as unknown as {
      antd?: { app?: { useApp: () => { message: { success: (msg: string) => void; info: (msg: string) => void } } } };
    }).antd?.app?.useApp() || {
      message: { success: (msg: string) => console.log(msg), info: (msg: string) => console.log(msg) },
    };
    if (triggered.length > 0) {
      message.success(t("opc.industry.rules.triggered", { count: triggered.length }));
    } else {
      message.info(t("opc.industry.rules.nothingTriggered"));
    }
    return triggered;
  };

  const handleExecuteAnalysis = async () => {
    await data.loadDecision();
  };

  const handleExecuteWorkflow = async () => {
    await data.executeWorkflow(industryId);
  };

  return (
    <div style={{ padding: 24, height: "100%", overflow: "auto" }}>
      <IndustryHeader
        industryId={industryId}
        manifest={data.manifest}
        onRefresh={handleRefreshAll}
        refreshing={data.dashboardLoading || data.stepsLoading || data.rulesLoading}
      />

      {/* KPI 仪表盘 */}
      <IndustryDashboard
        dashboard={data.dashboard}
        loading={data.dashboardLoading}
        kpiTimeRange={data.kpiTimeRange}
        onTimeRangeChange={data.setKpiTimeRange}
        onRefresh={data.loadDashboard}
      />

      {/* 行业专属内容（可由子类定制） */}
      {children}

      {/* 工作流步骤 */}
      <IndustryWorkflowSteps steps={data.workflowSteps} loading={data.stepsLoading} />

      {/* 自动化规则 */}
      <IndustryAutomationRules
        rules={data.automationRules}
        loading={data.rulesLoading}
        running={data.rulesRunning}
        onRunAll={handleRunRules}
      />

      {/* 分析决策 */}
      <IndustryAnalysisDecision
        decision={data.decision}
        loading={data.decisionLoading}
        decisionDays={data.decisionDays}
        onDaysChange={data.setDecisionDays}
        onExecute={handleExecuteAnalysis}
      />

      {/* 工作流执行 */}
      <IndustryWorkflowExecution
        workflowResult={data.workflowResult}
        executing={data.workflowExecuting}
        onExecute={handleExecuteWorkflow}
      />

      {/* 学习指标 */}
      <IndustryLearningMetrics
        metrics={data.learningMetrics}
        loading={data.metricsLoading}
        onRefresh={data.loadLearningMetrics}
      />

      {/* 专属操作 */}
      {config.actions && config.actions.length > 0 && (
        <IndustryActionsPanel industryId={industryId} actions={config.actions} />
      )}

      {/* 专属工作流 */}
      {config.workflows && config.workflows.length > 0 && (
        <IndustryWorkflowsPanel industryId={industryId} workflows={config.workflows} />
      )}

      {/* 学习与进化配置 */}
      <IndustryLearningPanel
        industryId={industryId}
        learningConfig={data.learningConfig}
        onReflect={data.reflectOnWorkflow}
        onEvolve={data.evolveWorkflow}
        onSelfImprove={data.runSelfImprovement}
      />
    </div>
  );
}
