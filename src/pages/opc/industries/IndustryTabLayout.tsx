// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 行业 Tab 业务流程布局组件
 */

import { Alert, Button, Card, Empty, message, Spin, Tabs, Tag, Typography } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import { useConversationStore, useSettingsStore } from "@/stores";

import { IndustryDashboard } from "./IndustryComponents";
import type { IndustryConfig, IndustryTab } from "./types";
import type { UseIndustryDataReturn } from "./useIndustryData";
import { useIndustryData } from "./useIndustryData";

const { Title, Text, Paragraph } = Typography;

/** Tab 业务阶段内容 */
function IndustryTabContent({
  tab,
  data,
  industryId,
}: {
  tab: IndustryTab;
  data: UseIndustryDataReturn;
  industryId: string;
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const createConversation = useConversationStore((s) => s.createConversation);
  const settings = useSettingsStore((s) => s.settings);

  const handleAction = async (actionKey: string) => {
    if (!settings?.defaultModel?.a || !settings?.defaultModel?.b) {
      message.warning(t("opc.industry.noProviderConfig"));
      navigate("/settings/providers");
      return;
    }

    const action = tab.actions.find((a) => a.key === actionKey);
    const actionLabel = action?.label || actionKey;

    if (action?.type === "workflow") {
      const templateId = action.template_id || actionKey;
      navigate(`/workflow/new?industry=${industryId}&template=${templateId}`);
      return;
    }

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
        actionKey,
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

  const handleExecute = async (workflowId: string) => {
    try {
      const result = await data.executeWorkflow(workflowId);
      if (result.status === "completed") {
        message.success(t("opc.industry.tab.executeSuccess", { id: workflowId }));
      } else {
        message.error(t("opc.industry.tab.executeFailed", { id: workflowId }));
      }
    } catch {
      message.error(t("opc.industry.tab.executeFailed", { id: workflowId }));
    }
  };

  return (
    <div style={{ padding: "16px 0" }}>
      {tab.description && (
        <Alert
          style={{ marginBottom: 16 }}
          type="info"
          showIcon
          message={tab.description}
        />
      )}

      {/* 操作项 */}
      {tab.actions.length > 0 && (
        <Card
          title={
            <span>
              <strong>{t("opc.industry.tab.actions")}</strong>
            </span>
          }
          style={{ marginBottom: 16 }}
          size="small"
        >
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))",
              gap: 12,
            }}
          >
            {tab.actions.map((action) => (
              <div
                key={action.key}
                onClick={() => handleAction(action.key)}
                style={{
                  cursor: "pointer",
                  padding: 12,
                  border: "1px solid var(--color-border)",
                  borderRadius: 8,
                  transition: "all 0.2s",
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "center",
                  gap: 6,
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.borderColor = "var(--color-primary)";
                  e.currentTarget.style.boxShadow = "0 2px 8px rgba(0,0,0,0.1)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.borderColor = "var(--color-border)";
                  e.currentTarget.style.boxShadow = "none";
                }}
              >
                <div style={{ fontSize: 28 }}>{action.icon}</div>
                <Text strong style={{ fontSize: 13 }}>{action.label || action.key}</Text>
                <Tag color={action.type === "workflow" ? "blue" : "green"} style={{ fontSize: 11 }}>
                  {action.type === "workflow"
                    ? t("opc.industry.tab.type.workflow")
                    : t("opc.industry.tab.type.conversation")}
                </Tag>
              </div>
            ))}
          </div>
        </Card>
      )}

      {/* 工作流列表 */}
      {tab.workflows.length > 0 && (
        <Card
          title={
            <span>
              <strong>{t("opc.industry.tab.workflows")}</strong>
              <Text type="secondary" style={{ marginLeft: 8 }}>
                ({tab.workflows.length})
              </Text>
            </span>
          }
          style={{ marginBottom: 16 }}
          size="small"
        >
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {tab.workflows.map((wf) => (
              <div
                key={wf.id}
                style={{
                  padding: 12,
                  border: "1px solid var(--color-border)",
                  borderRadius: 6,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                }}
              >
                <div>
                  <Text strong>{wf.name || wf.id}</Text>
                  {wf.description && (
                    <div>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        {wf.description}
                      </Text>
                    </div>
                  )}
                </div>
                <Button
                  size="small"
                  type="primary"
                  loading={data.workflowExecuting}
                  onClick={() => handleExecute(wf.id)}
                >
                  {t("opc.industry.tab.execute")}
                </Button>
              </div>
            ))}
          </div>
        </Card>
      )}

      {tab.actions.length === 0 && tab.workflows.length === 0 && (
        <Empty
          description={t("opc.industry.tab.noContent")}
        />
      )}
    </div>
  );
}

/**
 * 行业 Tab 布局主组件
 */
export function IndustryTabLayout({
  industryId,
  config,
}: {
  industryId: string;
  config: IndustryConfig;
}) {
  const { t } = useTranslation();
  const data = useIndustryData(industryId);
  const [activeTab, setActiveTab] = useState<string | undefined>(
    config.tabs?.[0]?.key,
  );

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

  const effectiveTabs = config.tabs && config.tabs.length > 0
    ? config.tabs
    : [
      {
        key: "all",
        label: t("opc.industry.tab.all"),
        actions: config.actions || [],
        workflows: config.workflows || [],
      },
    ];

  // 构建 Tab items
  const tabItems = effectiveTabs.map((tab) => ({
    key: tab.key,
    label: (
      <span>
        {tab.icon && <span style={{ marginRight: 8 }}>{tab.icon}</span>}
        {tab.label}
      </span>
    ),
    children: <IndustryTabContent tab={tab} data={data} industryId={industryId} />,
  }));

  return (
    <div style={{ padding: 24, height: "100%", overflow: "auto" }}>
      {/* 头部信息 */}
      <div
        style={{
          marginBottom: 24,
          padding: 16,
          background: "var(--color-fill-tertiary)",
          borderRadius: 8,
        }}
      >
        <Title level={4} style={{ margin: 0 }}>
          {data.manifest.name}
        </Title>
        <Paragraph type="secondary" style={{ margin: "4px 0 0 0" }}>
          {data.manifest.description}
        </Paragraph>
      </div>

      {/* KPI 仪表盘 */}
      <IndustryDashboard
        dashboard={data.dashboard}
        loading={data.dashboardLoading}
        kpiTimeRange={data.kpiTimeRange}
        onTimeRangeChange={data.setKpiTimeRange}
        onRefresh={data.loadDashboard}
      />

      {/* Tab 业务流程 */}
      <Card
        style={{ marginBottom: 24 }}
        bodyStyle={{ padding: 16 }}
      >
        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          items={tabItems}
        />
      </Card>

      {/* 工作流步骤 */}
      {data.workflowSteps.length > 0 && (
        <Card title={t("opc.industry.steps.title")} style={{ marginBottom: 24 }}>
          <Text type="secondary">{t("opc.industry.steps.total", { count: data.workflowSteps.length })}</Text>
        </Card>
      )}

      {/* 执行结果 */}
      {data.workflowResult && (
        <Card
          title={t("opc.industry.tab.executionResult")}
          style={{ marginBottom: 24 }}
        >
          <div style={{ display: "flex", gap: 24, alignItems: "center" }}>
            <Tag color={data.workflowResult.status === "completed" ? "green" : "red"}>
              {data.workflowResult.status}
            </Tag>
            <Text>
              {t("opc.industry.tab.durationMs", { ms: data.workflowResult.duration_ms })}
            </Text>
          </div>
        </Card>
      )}
    </div>
  );
}
