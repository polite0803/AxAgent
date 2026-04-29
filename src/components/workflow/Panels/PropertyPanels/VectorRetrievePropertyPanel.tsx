import { Divider, Input, InputNumber, Select } from "antd";
import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useKnowledgeStore } from "@/stores";
import type { VectorRetrieveNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface VectorRetrievePropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const VectorRetrievePropertyPanel: React.FC<VectorRetrievePropertyPanelProps> = (
  { node, onUpdate, onDelete },
) => {
  const { t } = useTranslation();
  const vectorRetrieveNode = node as VectorRetrieveNode;
  const config = vectorRetrieveNode.config || {
    query: "",
    knowledge_base_id: "",
    top_k: 5,
    similarity_threshold: 0.7,
    output_var: "",
  };

  const { bases, loadBases } = useKnowledgeStore();

  useEffect(() => {
    if (bases.length === 0) {
      loadBases();
    }
  }, [bases.length, loadBases]);

  const knowledgeBaseOptions = useMemo(
    () =>
      bases
        .filter((b) => b.enabled)
        .map((b) => ({ value: b.id, label: b.name })),
    [bases],
  );

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>查询</label>
        <Input.TextArea
          value={config.query || ""}
          onChange={(e) => handleConfigChange("query", e.target.value)}
          rows={2}
          size="small"
          placeholder="输入检索查询..."
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>知识库</label>
        <Select
          value={config.knowledge_base_id || undefined}
          onChange={(value) => handleConfigChange("knowledge_base_id", value)}
          size="small"
          style={{ width: "100%" }}
          placeholder={t("workflow.props.selectKnowledgeBase")}
          showSearch
          optionFilterProp="label"
          options={knowledgeBaseOptions}
        />
      </div>

      <div style={{ display: "flex", gap: 8 }}>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.topK")}</label>
          <InputNumber
            value={config.top_k ?? 5}
            onChange={(value) => handleConfigChange("top_k", value)}
            min={1}
            max={100}
            size="small"
            style={{ width: "100%" }}
          />
        </div>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.similarityThreshold")}</label>
          <InputNumber
            value={config.similarity_threshold !== undefined ? Math.round(config.similarity_threshold * 100) : 70}
            onChange={(value) => handleConfigChange("similarity_threshold", (value != null ? value : 70) / 100)}
            min={0}
            max={100}
            size="small"
            style={{ width: "100%" }}
            formatter={(value) => `${value}%`}
            parser={(value) => Number(value?.replace("%", ""))}
          />
        </div>
      </div>

      <div>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>{t("workflow.props.outputVariable")}</label>
        <Input
          value={config.output_var || ""}
          onChange={(e) => handleConfigChange("output_var", e.target.value)}
          size="small"
          placeholder={t("workflow.props.outputVarRetrieved")}
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
