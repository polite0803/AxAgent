// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 行业 Tab 内容组件 — 渲染指定 tab 的 actions 和 workflows
 * 工作流采用向导模式，点击"开始"后通过分步向导引导用户配置并执行
 */

import { Alert, Button, Card, Empty, message, Tag, Typography } from "antd";
import { Settings } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import { CodeRefactorSettingsModal } from "@/components/settings/CodeRefactorSettingsModal";
import { LiteraryCreationSettingsModal } from "@/components/settings/LiteraryCreationSettingsModal";
import { useConversationStore, useSettingsStore } from "@/stores";

import type { ActionItem, IndustryConfig, IndustryTab, IndustryWorkflow } from "./types";
import { useIndustryData } from "./useIndustryData";
import { WorkflowWizard } from "./WorkflowWizard";

const { Text } = Typography;

interface IndustryTabContentProps {
  industryId: string;
  config: IndustryConfig;
  tabKey: string;
}

function findTab(config: IndustryConfig, tabKey: string): IndustryTab | undefined {
  return config.tabs?.find((t) => t.key === tabKey);
}

export function IndustryTabContent({ industryId, config, tabKey }: IndustryTabContentProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const createConversation = useConversationStore((s) => s.createConversation);
  const settings = useSettingsStore((s) => s.settings);
  const data = useIndustryData(industryId);

  const tab = useMemo(() => findTab(config, tabKey), [config, tabKey]);

  const [wizardOpen, setWizardOpen] = useState(false);
  const [activeWorkflow, setActiveWorkflow] = useState<IndustryWorkflow | null>(null);
  const [literarySettingsOpen, setLiterarySettingsOpen] = useState(false);
  const [refactorSettingsOpen, setRefactorSettingsOpen] = useState(false);
  const [refactorWorkflowId, setRefactorWorkflowId] = useState<string>("");

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

    const { invoke } = await import("@/lib/invoke");
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
        promptConfig.actionLabel,
        settings.default_model_id,
        settings.default_provider_id,
        {
          system_prompt: promptConfig.systemPrompt,
        },
      );
      if (conv?.id) {
        navigate(`/chat?conversationId=${conv.id}&prompt=${encodeURIComponent(promptConfig.userPrompt)}`);
      }
    } catch {
      const conv = await createConversation(
        actionLabel,
        settings.default_model_id,
        settings.default_provider_id,
        {
          system_prompt:
            `你是一位专业的${industryId}领域助手，擅长${actionLabel}相关的分析和咨询。请根据用户需求提供高质量的分析和建议。`,
        },
      );
      if (conv?.id) {
        navigate(`/chat?conversationId=${conv.id}&prompt=${encodeURIComponent(actionLabel)}`);
      }
    }
  };

  const handleStartWorkflow = (wf: IndustryWorkflow) => {
    setActiveWorkflow(wf);
    setWizardOpen(true);
  };

  const handleWizardClose = () => {
    setWizardOpen(false);
    setActiveWorkflow(null);
  };

  if (!tab) {
    return (
      <div style={{ padding: 24 }}>
        <Empty description={t("opc.industry.notFound")} />
      </div>
    );
  }

  return (
    <div style={{ padding: "16px 24px", height: "100%", overflow: "auto" }}>
      {tab.description && (
        <Alert
          style={{ marginBottom: 16 }}
          type="info"
          showIcon
          message={tab.description}
        />
      )}

      {tab.actions && tab.actions.length > 0 && (
        <Card
          title={
            <span>
              <strong>{t("opc.industry.tab.actions")}</strong>
              <Text type="secondary" style={{ marginLeft: 8 }}>
                ({tab.actions.length})
              </Text>
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
                onClick={() => handleAction(action)}
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
                <Text strong style={{ fontSize: 13 }}>
                  {action.label || action.key}
                </Text>
                {action.type === "workflow" && (
                  <Tag color="orange" style={{ margin: 0 }}>
                    {t("opc.industry.workflowTag")}
                  </Tag>
                )}
              </div>
            ))}
          </div>
        </Card>
      )}

      {tab.workflows && tab.workflows.length > 0 && (
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
                <div style={{ flex: 1 }}>
                  <Text strong>{wf.name || wf.id}</Text>
                  {wf.description && (
                    <div>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        {wf.description}
                      </Text>
                    </div>
                  )}
                </div>
                <div style={{ display: "flex", gap: 8 }}>
                  {wf.id === "workflow-cm-literary-creation" && (
                    <Button
                      size="small"
                      icon={<Settings size={14} />}
                      onClick={() => setLiterarySettingsOpen(true)}
                    >
                      {t("literaryCreation.settings.saveConfig")}
                    </Button>
                  )}
                  {(wf.id === "wf-eng-refactor"
                    || wf.id === "wf-eng-refactor-lite"
                    || wf.id === "wf-eng-tech-debt") && (
                    <Button
                      size="small"
                      icon={<Settings size={14} />}
                      onClick={() => {
                        setRefactorWorkflowId(wf.id);
                        setRefactorSettingsOpen(true);
                      }}
                    >
                      {t("opc.refactor.settings.saveConfig")}
                    </Button>
                  )}
                  <Button
                    size="small"
                    type="primary"
                    onClick={() => handleStartWorkflow(wf)}
                  >
                    {t("opc.industry.wizard.start")}
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}

      {(!tab.actions || tab.actions.length === 0)
        && (!tab.workflows || tab.workflows.length === 0) && <Empty description={t("opc.industry.tab.noContent")} />}

      <WorkflowWizard
        open={wizardOpen}
        workflow={activeWorkflow}
        data={data}
        onClose={handleWizardClose}
      />

      <LiteraryCreationSettingsModal
        open={literarySettingsOpen}
        onClose={() => setLiterarySettingsOpen(false)}
      />

      <CodeRefactorSettingsModal
        open={refactorSettingsOpen}
        onClose={() => setRefactorSettingsOpen(false)}
        workflowId={refactorWorkflowId}
      />
    </div>
  );
}
