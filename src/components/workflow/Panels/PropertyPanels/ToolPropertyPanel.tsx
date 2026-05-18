import { useLocalToolStore } from "@/stores";
import { Button, Divider, Input, Select } from "antd";
import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ToolNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface ToolPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const ToolPropertyPanel: React.FC<ToolPropertyPanelProps> = ({
  node,
  onUpdate,
  onDelete,
}) => {
  const { t } = useTranslation();
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
      if (!group.enabled) {
        continue;
      }
      for (const tool of group.tools) {
        options.push({
          value: tool.name,
          label: `${group.groupName} / ${tool.name}`,
        });
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
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.tool")}
        </label>
        <Select
          value={config.tool_name || undefined}
          onChange={(value) => handleConfigChange("tool_name", value)}
          size="small"
          style={{ width: "100%" }}
          placeholder={t("workflow.props.selectTool")}
          showSearch
          options={toolOptions}
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
          <label style={{ color: "#999", fontSize: 12 }}>
            {t("workflow.props.inputMapping")}
          </label>
          <Button type="link" size="small" onClick={handleAddInputMapping}>
            {t("workflow.props.addMapping")}
          </Button>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {Object.entries(config.input_mapping || {}).map(([key, value]) => (
            <div
              key={key}
              style={{ display: "flex", gap: 4, alignItems: "center" }}
            >
              <Input
                id="tool-property-panel-input-110"
                value={key}
                size="small"
                disabled
                style={{ width: 80, fontSize: 12 }}
              />
              <span style={{ color: "#666", fontSize: 12 }}>←</span>
              <Input
                id="tool-property-panel-input-111"
                value={String(value)}
                onChange={(e) => handleUpdateInputMapping(key, e.target.value)}
                size="small"
                placeholder={t("workflow.props.variable")}
                style={{ flex: 1 }}
              />
              <Button
                type="link"
                size="small"
                danger
                onClick={() => handleDeleteInputMapping(key)}
              >
                {t("workflow.props.delete")}
              </Button>
            </div>
          ))}

          {Object.keys(config.input_mapping || {}).length === 0 && (
            <div
              style={{
                color: "#666",
                fontSize: 12,
                textAlign: "center",
                padding: 8,
              }}
            >
              {t("workflow.props.clickToAddMapping")}
            </div>
          )}
        </div>
      </div>

      <div>
        <label
          style={{
            display: "block",
            color: "#999",
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.outputVariable")}
        </label>
        <Input
          id="tool-property-panel-input-112"
          value={config.output_var || ""}
          onChange={(e) => handleConfigChange("output_var", e.target.value)}
          size="small"
          placeholder="tool_output"
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div
        style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}
      >
        <BasePropertyPanel
          node={node}
          onUpdate={onUpdate}
          onDelete={onDelete}
        />
      </div>
    </div>
  );
};
