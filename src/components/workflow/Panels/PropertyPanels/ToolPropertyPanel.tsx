import { useLocalToolStore } from "@/stores";
import { Divider, Input, Select } from "antd";
import React, { useEffect, useMemo } from "react";
import type { ToolNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface ToolPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const ToolPropertyPanel: React.FC<ToolPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const toolNode = node as ToolNode;
  const config = toolNode.config || {
    tool_name: "",
    input_mapping: {},
    output_var: "",
  };

  const { groups: toolGroups, loadGroups } = useLocalToolStore();

  useEffect(() => {
    if (toolGroups.length === 0) {
      loadGroups();
    }
  }, [toolGroups.length, loadGroups]);

  const toolOptions = useMemo(() => {
    const options: { value: string; label: string }[] = [];
    for (const group of toolGroups) {
      if (!group.enabled) { continue; }
      for (const tool of group.tools) {
        options.push({ value: tool.toolName, label: `${group.groupName} / ${tool.toolName}` });
      }
    }
    return options;
  }, [toolGroups]);

  const handleConfigChange = (key: string, value: unknown) => {
    onUpdate({ config: { ...config, [key]: value } });
  };

  const handleAddInputMapping = () => {
    onUpdate({
      config: {
        ...config,
        input_mapping: {
          ...config.input_mapping,
          [`param_${Object.keys(config.input_mapping).length + 1}`]: "",
        },
      },
    });
  };

  const handleUpdateInputMapping = (key: string, value: string) => {
    onUpdate({
      config: {
        ...config,
        input_mapping: {
          ...config.input_mapping,
          [key]: value,
        },
      },
    });
  };

  const handleDeleteInputMapping = (key: string) => {
    const newMapping = { ...config.input_mapping };
    delete newMapping[key];
    onUpdate({
      config: {
        ...config,
        input_mapping: newMapping,
      },
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>工具</label>
        <Select
          value={config.tool_name || undefined}
          onChange={(value) => handleConfigChange("tool_name", value)}
          size="small"
          style={{ width: "100%" }}
          placeholder="选择工具..."
          showSearch
          options={toolOptions}
        />
      </div>

      <div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
          <label style={{ color: "#999", fontSize: 11 }}>输入映射</label>
          <a style={{ fontSize: 10 }} onClick={handleAddInputMapping}>
            + 添加
          </a>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {Object.entries(config.input_mapping || {}).map(([key, value]) => (
            <div key={key} style={{ display: "flex", gap: 4, alignItems: "center" }}>
              <Input
                value={key}
                size="small"
                disabled
                style={{ width: 80, fontSize: 10 }}
              />
              <span style={{ color: "#666", fontSize: 10 }}>←</span>
              <Input
                value={String(value)}
                onChange={(e) =>
                  handleUpdateInputMapping(key, e.target.value)}
                size="small"
                placeholder="变量"
                style={{ flex: 1 }}
              />
              <a
                style={{ fontSize: 10, color: "#ff4d4f" }}
                onClick={() =>
                  handleDeleteInputMapping(key)}
              >
                删除
              </a>
            </div>
          ))}

          {Object.keys(config.input_mapping || {}).length === 0 && (
            <div style={{ color: "#666", fontSize: 11, textAlign: "center", padding: 8 }}>
              点击"添加"创建输入映射
            </div>
          )}
        </div>
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>输出变量</label>
        <Input
          value={config.output_var || ""}
          onChange={(e) => handleConfigChange("output_var", e.target.value)}
          size="small"
          placeholder="tool_output"
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
