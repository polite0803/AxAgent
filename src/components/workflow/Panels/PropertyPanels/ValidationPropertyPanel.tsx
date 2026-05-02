import { MinusCircleOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Input, Select } from "antd";
import React from "react";
import type { ValidationNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface ValidationPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const ValidationPropertyPanel: React.FC<ValidationPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
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
          <label style={{ color: "#999", fontSize: 11 }}>断言</label>
          <Button
            size="small"
            type="dashed"
            icon={<PlusOutlined />}
            onClick={handleAddAssertion}
            style={{ fontSize: 10 }}
          >
            添加
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
                    { value: "equals", label: "等于" },
                    { value: "contains", label: "包含" },
                    { value: "matches", label: "匹配" },
                    { value: "exists", label: "存在" },
                    { value: "custom", label: "自定义" },
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
                    value={assertion.expected || ""}
                    onChange={(e) => handleAssertionChange(index, "expected", e.target.value)}
                    size="small"
                    placeholder="期望值"
                    style={{ flex: 1 }}
                  />
                  <Input
                    value={assertion.actual || ""}
                    onChange={(e) => handleAssertionChange(index, "actual", e.target.value)}
                    size="small"
                    placeholder="实际值"
                    style={{ flex: 1 }}
                  />
                </div>
              )}
              {assertion.type === "custom" && (
                <Input
                  value={assertion.expression || ""}
                  onChange={(e) => handleAssertionChange(index, "expression", e.target.value)}
                  size="small"
                  placeholder="自定义表达式..."
                  style={{ marginTop: 4 }}
                />
              )}
            </div>
          ))}

          {(config.assertions || []).length === 0 && (
            <div style={{ color: "#666", fontSize: 11, textAlign: "center", padding: 8 }}>
              点击"添加"创建断言
            </div>
          )}
        </div>
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>失败策略</label>
        <Select
          value={config.on_fail || "stop"}
          onChange={(value) => handleConfigChange("on_fail", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "stop", label: "停止" },
            { value: "retry", label: "重试" },
            { value: "continue", label: "继续" },
          ]}
        />
      </div>

      {config.on_fail === "retry" && (
        <div>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>最大重试次数</label>
          <Input
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
