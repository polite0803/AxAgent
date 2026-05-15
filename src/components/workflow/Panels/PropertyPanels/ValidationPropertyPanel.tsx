import { MinusCircleOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Input, Select } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { ValidationNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface ValidationPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const ValidationPropertyPanel: React.FC<ValidationPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const { t } = useTranslation();
  const validationNode = node as ValidationNode;
  const config = validationNode.config || {
    assertions: [],
    on_fail: "stop" as const,
    max_retries: 0,
  };

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const handleAssertionChange = (index: number, field: string, value: string) => {
    const newAssertions = [...(config.assertions || [])];
    newAssertions[index] = { ...newAssertions[index], [field]: value };
    handleConfigChange("assertions", newAssertions);
  };

  const handleAddAssertion = () => {
    const newAssertions = [...(config.assertions || []), { type: "equals" as const, expected: "", actual: "" }];
    handleConfigChange("assertions", newAssertions);
  };

  const handleRemoveAssertion = (index: number) => {
    const newAssertions = (config.assertions || []).filter((_, i) => i !== index);
    handleConfigChange("assertions", newAssertions);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
          <label style={{ color: "#999", fontSize: 11 }}>{t("workflow.props.assertions")}</label>
          <Button
            size="small"
            type="dashed"
            icon={<PlusOutlined />}
            onClick={handleAddAssertion}
            style={{ fontSize: 10 }}
          >
            {t("workflow.props.add")}
          </Button>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {(config.assertions || []).map((assertion, index) => (
            <div key={index} style={{ background: "#252525", borderRadius: 4, padding: 8 }}>
              <div style={{ display: "flex", gap: 4, alignItems: "center", marginBottom: 4 }}>
                <Select
                  value={assertion.type}
                  onChange={(value) =>
                    handleAssertionChange(index, "type", value)}
                  size="small"
                  style={{ flex: 1 }}
                  options={[
                    { value: "equals", label: t("workflow.props.equals") },
                    { value: "contains", label: t("workflow.props.contains") },
                    { value: "matches", label: t("workflow.props.matches") },
                    { value: "exists", label: t("workflow.props.exists") },
                    { value: "custom", label: t("workflow.props.custom") },
                  ]}
                />
                <MinusCircleOutlined
                  onClick={() =>
                    handleRemoveAssertion(index)}
                  style={{ color: "#ff4d4f", cursor: "pointer", fontSize: 12 }}
                />
              </div>
              {assertion.type !== "exists" && (
                <div style={{ display: "flex", gap: 4 }}>
                  <Input
                    id="validation-property-panel-input-116"
                    value={assertion.expected || ""}
                    onChange={(e) => handleAssertionChange(index, "expected", e.target.value)}
                    size="small"
                    placeholder={t("workflow.props.expectedValue")}
                    style={{ flex: 1 }}
                  />
                  <Input
                    id="validation-property-panel-input-117"
                    value={assertion.actual || ""}
                    onChange={(e) => handleAssertionChange(index, "actual", e.target.value)}
                    size="small"
                    placeholder={t("workflow.props.actualValue")}
                    style={{ flex: 1 }}
                  />
                </div>
              )}
              {assertion.type === "custom" && (
                <Input
                  id="validation-property-panel-input-118"
                  value={assertion.expression || ""}
                  onChange={(e) => handleAssertionChange(index, "expression", e.target.value)}
                  size="small"
                  placeholder={t("workflow.props.customExpression")}
                  style={{ marginTop: 4 }}
                />
              )}
            </div>
          ))}

          {(config.assertions || []).length === 0 && (
            <div style={{ color: "#666", fontSize: 11, textAlign: "center", padding: 8 }}>
              {t("workflow.props.clickToAddAssertion")}
            </div>
          )}
        </div>
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
          {t("workflow.props.failStrategy")}
        </label>
        <Select
          value={config.on_fail || "stop"}
          onChange={(value) => handleConfigChange("on_fail", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "stop", label: t("workflow.props.stop") },
            { value: "retry", label: t("workflow.props.retry") },
            { value: "continue", label: t("workflow.props.continue") },
          ]}
        />
      </div>

      {config.on_fail === "retry" && (
        <div>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
            {t("workflow.props.maxRetries")}
          </label>
          <Input
            id="validation-property-panel-input-119"
            type="number"
            value={config.max_retries ?? 0}
            onChange={(e) => handleConfigChange("max_retries", parseInt(e.target.value) || 0)}
            size="small"
            min={0}
          />
        </div>
      )}

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
