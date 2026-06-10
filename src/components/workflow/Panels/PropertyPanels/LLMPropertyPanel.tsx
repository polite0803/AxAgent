import { ModelSelect } from "@/components/shared/ModelSelect";
import { usePromptTemplateStore, useProviderStore, useWorkflowEditorStore } from "@/stores";
import type { PromptTemplate } from "@/types";
import { Button, Input, InputNumber, message, Modal, theme } from "antd";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { AIAssistButton, useNodeAIAssist } from "../../Hooks";
import type { LLMNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface LLMPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const LLMPropertyPanel: React.FC<LLMPropertyPanelProps> = ({
  node,
  onUpdate,
  onDelete,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [messageApi, contextHolder] = message.useMessage();
  const [templateModalOpen, setTemplateModalOpen] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<PromptTemplate | null>(null);
  const [variableValues, setVariableValues] = useState<Record<string, string>>(
    {},
  );

  const llmNode = node as LLMNode;
  const config = llmNode.config || {
    model: "",
    prompt: "",
    temperature: 0.7,
    max_tokens: 2048,
  };

  const { providers, fetchProviders } = useProviderStore();
  const { templates, loadTemplates } = usePromptTemplateStore();

  useEffect(() => {
    if (providers.length === 0) {
      fetchProviders();
    }
  }, [providers.length, fetchProviders]);

  useEffect(() => {
    loadTemplates();
  }, [loadTemplates]);

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const handleSelectTemplate = (template: PromptTemplate) => {
    setSelectedTemplate(template);
    setVariableValues({});
    setTemplateModalOpen(true);
  };

  const incrementUsage = usePromptTemplateStore((s) => s.incrementUsage);

  const handleApplyTemplate = () => {
    if (!selectedTemplate) {
      return;
    }

    let content = selectedTemplate.content;
    try {
      const schema = selectedTemplate.variablesSchema
        ? JSON.parse(selectedTemplate.variablesSchema)
        : {};
      // js-hoist-regexp: 模式依赖迭代变量 varName，无法提升
      for (const [varName] of Object.entries(schema)) {
        const value = variableValues[varName] || `{${varName}}`;
        content = content.replace(new RegExp(`\\{${varName}\\}`, "g"), value);
      }
    } catch {
      content = selectedTemplate.content;
    }

    handleConfigChange("prompt", content);
    handleConfigChange("promptTemplateId", selectedTemplate.id);
    setTemplateModalOpen(false);
    setSelectedTemplate(null);
    setVariableValues({});
    incrementUsage(selectedTemplate.id);
    messageApi.success(t("promptTemplates.applied"));
  };

  const parseVariables = (content: string): string[] => {
    const matches = content.match(/\{([^}]+)\}/g) || [];
    return [...new Set(matches.map((m) => m.slice(1, -1)))];
  };

  const activeTemplates = templates.filter((t) => t.isActive);

  const { generate: aiGenerate, generating: aiGenerating } = useNodeAIAssist();
  const handleAIOptimizePrompt = async () => {
    const current = config.prompt || "";
    if (!current.trim()) {
      messageApi.warning(t("workflow.aiPanel.enterPromptToOptimize"));
      return;
    }
    const result = await aiGenerate({
      systemPrompt: "你是一个提示词优化专家。改进用户提供的 LLM 提示词，使其更清晰、更具体、效果更好。"
        + "保留原有结构和变量占位符（如 {varName}）。"
        + "只输出优化后的提示词正文，不要任何解释、前缀或 Markdown 标记。",
      userPrompt: current,
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    handleConfigChange("prompt", result);
    messageApi.success(t("workflow.aiAssist.applied"));
  };

  const handleAIContextComplete = async () => {
    const current = config.prompt || "";
    const store = useWorkflowEditorStore.getState();
    const upstreamEdgeIds = store.edges.filter((e) => e.target === node.id).map((e) => e.source);
    const upstreamNodes = store.nodes.filter((n) => upstreamEdgeIds.includes(n.id));
    const downstreamEdgeIds = store.edges.filter((e) => e.source === node.id).map((e) => e.target);
    const downstreamNodes = store.nodes.filter((n) => downstreamEdgeIds.includes(n.id));
    const contextInfo = [
      `当前节点: "${node.title}" (类型: ${node.type})`,
      upstreamNodes.length > 0
        ? `上游节点: ${upstreamNodes.map((n) => `"${n.title}"(${n.type})`).join(", ")}`
        : "无上游节点",
      downstreamNodes.length > 0
        ? `下游节点: ${downstreamNodes.map((n) => `"${n.title}"(${n.type})`).join(", ")}`
        : "无下游节点",
    ].join("\n");
    const result = await aiGenerate({
      systemPrompt: "你是工作流上下文补全助手。根据工作流上下文和当前提示词，生成可追加到提示词末尾的补充内容，"
        + "帮助 LLM 理解可用的上下文信息、上游数据来源和输出目标。"
        + "只输出纯文本补充内容，不要解释、前缀或 Markdown 标记。",
      userPrompt: current
        ? `工作流上下文:\n${contextInfo}\n\n当前提示词:\n${current}\n\n请根据工作流上下文，生成可以追加到提示词末尾的补充内容。`
        : `工作流上下文:\n${contextInfo}\n\n当前没有提示词。请根据工作流上下文生成一个初始提示词。`,
    });
    if (!result) {
      messageApi.error(t("workflow.aiAssist.failed"));
      return;
    }
    handleConfigChange("prompt", current ? `${current}\n\n${result}` : result);
    messageApi.success(t("workflow.aiAssist.contextCompleteApplied"));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.model")}
        </label>
        <ModelSelect
          value={config.model || undefined}
          onChange={(value) => handleConfigChange("model", value || "")}
          placeholder={t("workflow.props.selectModel")}
          allowClear
          style={{ width: "100%" }}
        />
      </div>

      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: 4,
          }}
        >
          <label
            style={{ color: token.colorTextTertiary, fontSize: 12 }}
          >
            {t("workflow.props.prompt")}
          </label>
          <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
            <AIAssistButton
              labelKey="optimize"
              loading={aiGenerating}
              onClick={handleAIOptimizePrompt}
              compact
            />
            <AIAssistButton
              labelKey="contextComplete"
              loading={aiGenerating}
              onClick={handleAIContextComplete}
              compact
            />
          </div>
        </div>
        <Input.TextArea
          id="l-l-m-property-panel-input-textarea-96"
          value={config.prompt || ""}
          onChange={(e) => handleConfigChange("prompt", e.target.value)}
          rows={5}
          size="small"
          placeholder={t("workflow.props.promptPlaceholder")}
        />
        <Button
          size="small"
          type="link"
          onClick={() => setTemplateModalOpen(true)}
          style={{ padding: 0, marginTop: 4 }}
        >
          {t("promptTemplates.selectFromLibrary")}
        </Button>
      </div>

      <div style={{ display: "flex", gap: 8 }}>
        <div style={{ flex: 1 }}>
          <label
            style={{
              display: "block",
              color: token.colorTextTertiary,
              fontSize: 12,
              marginBottom: 4,
            }}
          >
            {t("workflow.props.temperature")}
          </label>
          <InputNumber
            id="l-l-m-property-panel-inputnumber-97"
            value={config.temperature ?? 0.7}
            onChange={(value) => handleConfigChange("temperature", value)}
            min={0}
            max={2}
            step={0.1}
            size="small"
            style={{ width: "100%" }}
          />
          <div style={{ fontSize: 9, color: token.colorTextTertiary, marginTop: 2 }}>
            {t("workflow.props.temperatureHint")}
          </div>
        </div>
        <div style={{ flex: 1 }}>
          <label
            style={{
              display: "block",
              color: token.colorTextTertiary,
              fontSize: 12,
              marginBottom: 4,
            }}
          >
            {t("workflow.props.maxTokens")}
          </label>
          <InputNumber
            id="l-l-m-property-panel-inputnumber-98"
            value={config.max_tokens ?? 2048}
            onChange={(value) => handleConfigChange("max_tokens", value)}
            min={100}
            max={128000}
            step={100}
            size="small"
            style={{ width: "100%" }}
          />
        </div>
      </div>

      <div
        style={{ borderTop: `1px solid ${token.colorBorderSecondary}`, paddingTop: 12, marginTop: 4 }}
      >
        <BasePropertyPanel
          node={node}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      </div>

      <Modal
        title={t("promptTemplates.selectFromLibrary")}
        open={templateModalOpen}
        onOk={handleApplyTemplate}
        onCancel={() => setTemplateModalOpen(false)}
        okText={t("common.confirm")}
        cancelText={t("common.cancel")}
        width={600}
      >
        {contextHolder}
        {selectedTemplate
          ? (
            <div style={{ padding: "12px 0" }}>
              <p style={{ marginBottom: 8 }}>
                {t("promptTemplates.fillVariables")}
              </p>
              {Object.entries(
                selectedTemplate.variablesSchema
                  ? JSON.parse(selectedTemplate.variablesSchema)
                  : {},
              ).map(([varName, varType]) => (
                <div key={varName} style={{ marginBottom: 8 }}>
                  <label
                    style={{ display: "block", fontSize: 12, marginBottom: 2 }}
                  >
                    {varName} ({String(varType)})
                  </label>
                  <Input
                    id="l-l-m-property-panel-input-99"
                    placeholder={`${varName} (${String(varType)})`}
                    value={variableValues[varName] || ""}
                    onChange={(e) =>
                      setVariableValues((prev) => ({
                        ...prev,
                        [varName]: e.target.value,
                      }))}
                  />
                </div>
              ))}
              {parseVariables(selectedTemplate.content).length > 0
                && Object.keys(
                    selectedTemplate.variablesSchema
                      ? JSON.parse(selectedTemplate.variablesSchema)
                      : {},
                  ).length === 0
                && (
                  <p style={{ color: token.colorWarning, fontSize: 12 }}>
                    {t("promptTemplates.hasVariables", {
                      variables: parseVariables(selectedTemplate.content).join(
                        ", ",
                      ),
                    })}
                  </p>
                )}
            </div>
          )
          : (
            <div style={{ maxHeight: 400, overflowY: "auto" }}>
              {activeTemplates.length === 0
                ? (
                  <div style={{ textAlign: "center", padding: 24, color: token.colorTextTertiary }}>
                    {t("promptTemplates.noTemplates")}
                  </div>
                )
                : (
                  activeTemplates.map((template) => (
                    <div
                      key={template.id}
                      role="button"
                      tabIndex={0}
                      onClick={() => handleSelectTemplate(template)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          handleSelectTemplate(template);
                        }
                      }}
                      style={{
                        padding: "8px 12px",
                        cursor: "pointer",
                        borderBottom: "1px solid #333",
                      }}
                    >
                      <div style={{ fontWeight: 500 }}>{template.name}</div>
                      <div style={{ fontSize: 12, color: token.colorTextTertiary }}>
                        {template.description
                          || template.content.slice(0, 60) + "..."}
                      </div>
                    </div>
                  ))
                )}
            </div>
          )}
      </Modal>
    </div>
  );
};
