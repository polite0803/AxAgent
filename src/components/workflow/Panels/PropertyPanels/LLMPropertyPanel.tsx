import { ModelSelect } from "@/components/shared/ModelSelect";
import { usePromptTemplateStore, useProviderStore } from "@/stores";
import type { PromptTemplate } from "@/types";
import { Button, Input, InputNumber, message, Modal } from "antd";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { LLMNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface LLMPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const LLMPropertyPanel: React.FC<LLMPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const [messageApi, contextHolder] = message.useMessage();
  const [templateModalOpen, setTemplateModalOpen] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<PromptTemplate | null>(null);
  const [variableValues, setVariableValues] = useState<Record<string, string>>({});

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
  }, []);

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const handleSelectTemplate = (template: PromptTemplate) => {
    setSelectedTemplate(template);
    setVariableValues({});
    setTemplateModalOpen(true);
  };

  const handleApplyTemplate = () => {
    if (!selectedTemplate) { return; }

    let content = selectedTemplate.content;
    try {
      const schema = selectedTemplate.variablesSchema ? JSON.parse(selectedTemplate.variablesSchema) : {};
      for (const [varName, _varType] of Object.entries(schema)) {
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
    messageApi.success(t("promptTemplates.applied"));
  };

  const parseVariables = (content: string): string[] => {
    const matches = content.match(/\{([^}]+)\}/g) || [];
    return matches.map((m) => m.slice(1, -1)).filter((v, i, arr) => arr.indexOf(v) === i);
  };

  const activeTemplates = templates.filter((t) => t.isActive);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
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
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
          {t("workflow.props.prompt")}
        </label>
        <Input.TextArea
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
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
            {t("workflow.props.temperature")}
          </label>
          <InputNumber
            value={config.temperature ?? 0.7}
            onChange={(value) => handleConfigChange("temperature", value)}
            min={0}
            max={2}
            step={0.1}
            size="small"
            style={{ width: "100%" }}
          />
          <div style={{ fontSize: 9, color: "#666", marginTop: 2 }}>
            {t("workflow.props.temperatureHint")}
          </div>
        </div>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
            {t("workflow.props.maxTokens")}
          </label>
          <InputNumber
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

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
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
              <p style={{ marginBottom: 8 }}>{t("promptTemplates.fillVariables")}</p>
              {Object.entries(selectedTemplate.variablesSchema ? JSON.parse(selectedTemplate.variablesSchema) : {}).map(
                ([varName, varType]) => (
                  <div key={varName} style={{ marginBottom: 8 }}>
                    <label style={{ display: "block", fontSize: 12, marginBottom: 2 }}>
                      {varName} ({String(varType)})
                    </label>
                    <Input
                      placeholder={`${varName} (${String(varType)})`}
                      value={variableValues[varName] || ""}
                      onChange={(e) => setVariableValues((prev) => ({ ...prev, [varName]: e.target.value }))}
                    />
                  </div>
                ),
              )}
              {parseVariables(selectedTemplate.content).length > 0
                && Object.keys(selectedTemplate.variablesSchema ? JSON.parse(selectedTemplate.variablesSchema) : {})
                    .length === 0
                && (
                  <p style={{ color: "#f59e0b", fontSize: 12 }}>
                    {t("promptTemplates.hasVariables", {
                      variables: parseVariables(selectedTemplate.content).join(", "),
                    })}
                  </p>
                )}
            </div>
          )
          : (
            <div style={{ maxHeight: 400, overflowY: "auto" }}>
              {activeTemplates.length === 0
                ? (
                  <div style={{ textAlign: "center", padding: 24, color: "#999" }}>
                    {t("promptTemplates.noTemplates")}
                  </div>
                )
                : (
                  activeTemplates.map((template) => (
                    <div
                      key={template.id}
                      onClick={() => handleSelectTemplate(template)}
                      style={{
                        padding: "8px 12px",
                        cursor: "pointer",
                        borderBottom: "1px solid #333",
                      }}
                    >
                      <div style={{ fontWeight: 500 }}>{template.name}</div>
                      <div style={{ fontSize: 12, color: "#999" }}>
                        {template.description || template.content.slice(0, 60) + "..."}
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
