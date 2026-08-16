// SPDX-License-Identifier: AGPL-3.0-only

import { NL2SkillResultView } from "@/components/workflow/AIPanel/NL2SkillResultView";
import { NL2UIResultView } from "@/components/workflow/AIPanel/NL2UIResultView";
import { useDynamicUIStore } from "@/stores";
import { useSkillStore } from "@/stores/feature/skillStore";
import { useWorkflowStore } from "@/stores/feature/workflowStore";
import type { NL2SkillRequest, NL2SkillResult, NL2UIRequest, NL2UIResult, SkillDefinition, UISchema } from "@/types";
import { LayoutOutlined, SendOutlined, ThunderboltOutlined } from "@ant-design/icons";
import { App, Button, Empty, Input, Progress, Select, Space, Tabs, Typography } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

type GenerationMode = "skill" | "ui";

/**
 * NL 生成标签页
 *
 * 支持两种生成模式：
 * - NL2Skill：自然语言 → 技能定义（SkillDefinition），可应用到 skillStore
 * - NL2UI：自然语言 → 动态 UI Schema（UISchema），可保存到 dynamicUIStore
 */
export function AgentWfTab() {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const [mode, setMode] = useState<GenerationMode>("skill");
  const [prompt, setPrompt] = useState("");
  const [skillType, setSkillType] = useState<NL2SkillRequest["skillType"]>("chat");
  const [uiType, setUIType] = useState<NL2UIRequest["uiType"]>("dashboard");
  const [isGenerating, setIsGenerating] = useState(false);

  const parseSkillFromNaturalLanguage = useWorkflowStore((s) => s.parseSkillFromNaturalLanguage);
  const parseUIFromNaturalLanguage = useWorkflowStore((s) => s.parseUIFromNaturalLanguage);
  const parseProgress = useWorkflowStore((s) => s.parseProgress);

  const createSkill = useSkillStore((s) => s.createSkill);
  const createSchema = useDynamicUIStore((s) => s.createSchema);

  const [skillResult, setSkillResult] = useState<NL2SkillResult | null>(null);
  const [uiResult, setUIResult] = useState<NL2UIResult | null>(null);

  const skillTypeOptions = useMemo(() => [
    { label: t("agentPanel.nlGen.skillTypeChat"), value: "chat" },
    { label: t("agentPanel.nlGen.skillTypeTool"), value: "tool" },
    { label: t("agentPanel.nlGen.skillTypeWorkflow"), value: "workflow" },
    { label: t("agentPanel.nlGen.skillTypeAutomation"), value: "automation" },
  ], [t]);

  const uiTypeOptions = useMemo(() => [
    { label: t("agentPanel.nlGen.uiTypeDashboard"), value: "dashboard" },
    { label: t("agentPanel.nlGen.uiTypeForm"), value: "form" },
    { label: t("agentPanel.nlGen.uiTypeSettings"), value: "settings" },
    { label: t("agentPanel.nlGen.uiTypeReport"), value: "report" },
    { label: t("agentPanel.nlGen.uiTypeCustom"), value: "custom" },
  ], [t]);

  const handleGenerate = async () => {
    if (!prompt.trim()) { return; }
    setIsGenerating(true);
    setSkillResult(null);
    setUIResult(null);

    try {
      if (mode === "skill") {
        const result = await parseSkillFromNaturalLanguage({ prompt, skillType });
        setSkillResult(result);
      } else {
        const result = await parseUIFromNaturalLanguage({ prompt, uiType });
        setUIResult(result);
      }
    } catch (err) {
      console.warn("[AgentWfTab] NL generation failed:", err);
      message.error(t("agentPanel.nlGen.generateFailed"));
    } finally {
      setIsGenerating(false);
    }
  };

  const handleApplySkill = async (skill: SkillDefinition) => {
    try {
      const content = JSON.stringify(skill, null, 2);
      const result = await createSkill(skill.name, skill.description, content);
      if (result.canCreate) {
        message.success(t("agentPanel.nlGen.applySkillSuccess"));
      } else {
        message.warning(result.message || t("agentPanel.nlGen.applySkillFailed"));
      }
    } catch (err) {
      console.warn("[AgentWfTab] applySkill failed:", err);
      message.error(t("agentPanel.nlGen.applySkillFailed"));
    }
  };

  const handleApplyUI = async (schema: UISchema) => {
    try {
      await createSchema({
        title: schema.id || t("agentPanel.nlGen.defaultUITitle"),
        description: prompt,
        schemaJson: JSON.stringify(schema),
        category: "generated",
        tags: [],
      });
      message.success(t("agentPanel.nlGen.applyUISuccess"));
    } catch (err) {
      console.warn("[AgentWfTab] applyUI failed:", err);
      message.error(t("agentPanel.nlGen.applyUIFailed"));
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <Tabs
        activeKey={mode}
        onChange={(k) => {
          setMode(k as GenerationMode);
          setSkillResult(null);
          setUIResult(null);
        }}
        size="small"
        style={{ paddingLeft: 8, paddingRight: 8 }}
        items={[
          {
            key: "skill",
            label: (
              <Space>
                <ThunderboltOutlined />NL2Skill
              </Space>
            ),
          },
          {
            key: "ui",
            label: (
              <Space>
                <LayoutOutlined />NL2UI
              </Space>
            ),
          },
        ]}
      />

      <div style={{ padding: "0 12px 8px", display: "flex", flexDirection: "column", gap: 8 }}>
        <Input.TextArea
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          placeholder={mode === "skill"
            ? t("agentPanel.nlGen.skillPlaceholder")
            : t("agentPanel.nlGen.uiPlaceholder")}
          rows={3}
          disabled={isGenerating}
        />
        <div style={{ display: "flex", gap: 8 }}>
          {mode === "skill"
            ? (
              <Select
                value={skillType}
                onChange={setSkillType}
                size="small"
                style={{ width: 100 }}
                options={skillTypeOptions}
              />
            )
            : (
              <Select
                value={uiType}
                onChange={setUIType}
                size="small"
                style={{ width: 110 }}
                options={uiTypeOptions}
              />
            )}
          <Button
            type="primary"
            icon={<SendOutlined />}
            onClick={handleGenerate}
            loading={isGenerating}
            disabled={!prompt.trim()}
            style={{ marginLeft: "auto" }}
          >
            {t("agentPanel.nlGen.generate")}
          </Button>
        </div>
      </div>

      {isGenerating && (
        <div style={{ padding: "0 12px 8px" }}>
          <Progress percent={100} size="small" status="active" />
          <Text type="secondary" style={{ fontSize: 12 }}>{parseProgress}</Text>
        </div>
      )}

      <div style={{ flex: 1, overflow: "auto" }}>
        {mode === "skill" && skillResult && (
          <NL2SkillResultView result={skillResult} onApply={handleApplySkill} loading={false} />
        )}
        {mode === "ui" && uiResult && <NL2UIResultView result={uiResult} onApply={handleApplyUI} loading={false} />}
        {!isGenerating && !skillResult && !uiResult && (
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", padding: 24 }}>
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={
                <span style={{ color: "var(--color-text-secondary)", fontSize: 13 }}>
                  {t("agentPanel.nlGen.emptyHint")}
                </span>
              }
            />
          </div>
        )}
      </div>
    </div>
  );
}
