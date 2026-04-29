import { Input, InputNumber } from "antd";
import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ModelSelect } from "@/components/shared/ModelSelect";
import { useProviderStore } from "@/stores";
import type { LLMNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface LLMPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const LLMPropertyPanel: React.FC<LLMPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const llmNode = node as LLMNode;
  const config = llmNode.config || {
    model: "",
    prompt: "",
    temperature: 0.7,
    max_tokens: 2048,
  };

  const { providers, fetchProviders } = useProviderStore();

  useEffect(() => {
    if (providers.length === 0) {
      fetchProviders();
    }
  }, [providers.length, fetchProviders]);

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.model")}</label>
        <ModelSelect
          value={config.model || undefined}
          onChange={(value) => handleConfigChange("model", value || "")}
          placeholder={t("workflow.props.selectModel")}
          allowClear
          style={{ width: "100%" }}
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.prompt")}</label>
        <Input.TextArea
          value={config.prompt || ""}
          onChange={(e) => handleConfigChange("prompt", e.target.value)}
          rows={5}
          size="small"
          placeholder={t("workflow.props.promptPlaceholder")}
        />
      </div>

      <div style={{ display: "flex", gap: 8 }}>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.temperature")}</label>
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
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.maxTokens")}</label>
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
    </div>
  );
};
