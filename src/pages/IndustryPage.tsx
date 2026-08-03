// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { useConversationStore, useSettingsStore } from "@/stores";
import {
  ApiOutlined,
  AuditOutlined,
  BookOutlined,
  BugOutlined,
  CalculatorOutlined,
  CodeOutlined,
  CodeSandboxOutlined,
  CrownOutlined,
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
  TagOutlined,
  ThunderboltOutlined,
  TrophyOutlined,
  VideoCameraOutlined,
} from "@ant-design/icons";
import { App, Button, Card, Col, Empty, Row, Space, Spin, Tag, Typography } from "antd";
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

  const createConversation = useConversationStore((s) => s.createConversation);
  const settings = useSettingsStore((s) => s.settings);

  const config = useMemo(() => INDUSTRY_CONFIGS[industryId], [industryId]);

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
        `${wfName} - 执行`,
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
        <Spin size="large" tip={t("common.loading")} />
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
        <Title level={3} style={{ marginBottom: 8 }}>
          <span style={{ fontSize: 28, marginRight: 12 }}>{manifest.icon}</span>
          {manifest.name}
        </Title>
        <Paragraph type="secondary">{manifest.description}</Paragraph>
      </div>

      {/* 专属操作入口 */}
      <Card
        style={{ marginBottom: 24 }}
        bodyStyle={{ padding: 20 }}
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
                bodyStyle={{ padding: 16 }}
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
    </div>
  );
}
