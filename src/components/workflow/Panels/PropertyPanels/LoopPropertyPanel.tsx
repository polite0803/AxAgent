import { useWorkflowEditorStore } from "@/stores";
import { Divider, Input, InputNumber, Select, Switch, Tag } from "antd";
import { X } from "lucide-react";
import React from "react";
import type { LoopNode, LoopType, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface LoopPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const LoopPropertyPanel: React.FC<LoopPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const loopNode = node as LoopNode;
  const config = loopNode.config || {
    loop_type: "forEach" as LoopType,
    items_var: "",
    iteratee_var: "",
    max_iterations: 100,
    continue_on_error: false,
    body_steps: [],
  };

  const { nodes } = useWorkflowEditorStore();

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const handleAddStep = (nodeId: string) => {
    if (!config.body_steps.includes(nodeId)) {
      handleConfigChange("body_steps", [...config.body_steps, nodeId]);
    }
  };

  const handleRemoveStep = (nodeId: string) => {
    handleConfigChange("body_steps", config.body_steps.filter(id => id !== nodeId));
  };

  const availableNodes = nodes.filter(n =>
    n.id !== node.id
    && !config.body_steps.includes(n.id)
  );

  const getNodeLabel = (nodeId: string) => {
    const found = nodes.find(n => n.id === nodeId);
    return found ? `${found.title || found.id} (${found.type})` : nodeId;
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>循环类型</label>
        <Select
          value={config.loop_type}
          onChange={(value) => handleConfigChange("loop_type", value)}
          size="small"
          style={{ width: "100%" }}
          options={[
            { value: "forEach", label: "🔁 遍历 (ForEach)" },
            { value: "while", label: "⏳ 当...时 (While)" },
            { value: "doWhile", label: "↻ 执行...直到 (DoWhile)" },
            { value: "until", label: "🔚 直到... (Until)" },
          ]}
        />
      </div>

      {config.loop_type === "forEach" && (
        <>
          <div>
            <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>数组变量</label>
            <Input
              value={config.items_var || ""}
              onChange={(e) => handleConfigChange("items_var", e.target.value)}
              size="small"
              placeholder="如: input.items"
            />
          </div>
          <div>
            <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>迭代变量名</label>
            <Input
              value={config.iteratee_var || ""}
              onChange={(e) => handleConfigChange("iteratee_var", e.target.value)}
              size="small"
              placeholder="如: item"
            />
          </div>
        </>
      )}

      {config.loop_type === "while" && (
        <div>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>继续条件</label>
          <Input.TextArea
            value={config.continue_condition || ""}
            onChange={(e) => handleConfigChange("continue_condition", e.target.value)}
            rows={2}
            size="small"
            placeholder="如: count < 10 && hasMore"
          />
        </div>
      )}

      {config.loop_type === "until" && (
        <div>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>停止条件</label>
          <Input.TextArea
            value={config.continue_condition || ""}
            onChange={(e) => handleConfigChange("continue_condition", e.target.value)}
            rows={2}
            size="small"
            placeholder="如: isComplete == true"
          />
        </div>
      )}

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>最大迭代次数</label>
        <InputNumber
          value={config.max_iterations ?? 100}
          onChange={(value) => handleConfigChange("max_iterations", value)}
          min={1}
          max={10000}
          size="small"
          style={{ width: "100%" }}
        />
      </div>

      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <label style={{ color: "#999", fontSize: 11 }}>错误时继续</label>
        <Switch
          size="small"
          checked={config.continue_on_error ?? false}
          onChange={(checked) => handleConfigChange("continue_on_error", checked)}
        />
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>
          循环体步骤 ({config.body_steps?.length || 0})
        </label>
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {config.body_steps.map((stepId) => (
            <Tag
              key={stepId}
              closable
              onClose={() => handleRemoveStep(stepId)}
              style={{ background: "#2a2a2a", border: "1px solid #444", color: "#ddd" }}
              closeIcon={<X size={10} />}
            >
              {getNodeLabel(stepId)}
            </Tag>
          ))}
          {config.body_steps.length === 0 && <div style={{ color: "#666", fontSize: 11 }}>暂无循环体步骤</div>}
        </div>
      </div>

      {availableNodes.length > 0 && (
        <div>
          <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>添加步骤</label>
          <Select
            placeholder="选择要添加的节点"
            size="small"
            style={{ width: "100%" }}
            onChange={handleAddStep}
            options={availableNodes.map(n => ({
              value: n.id,
              label: `${n.title || n.id} (${n.type})`,
            }))}
          />
        </div>
      )}

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
