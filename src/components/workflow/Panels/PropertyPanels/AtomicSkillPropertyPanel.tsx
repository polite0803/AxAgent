import { useAtomicSkillStore } from "@/stores/feature/atomicSkillStore";
import { EditOutlined } from "@ant-design/icons";
import { Button, Divider, Input, Select, Space, Tag } from "antd";
import React from "react";
import type { WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface AtomicSkillPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
  onEditSkill?: (skillId: string) => void;
}

export const AtomicSkillPropertyPanel: React.FC<AtomicSkillPropertyPanelProps> = ({
  node,
  onUpdate,
  onDelete,
  onEditSkill,
}) => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const config = (node as any).config || {
    skill_id: "",
    input_mapping: {},
    output_var: "",
  };

  const { skills } = useAtomicSkillStore();

  const handleConfigChange = (key: string, value: unknown) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    onUpdate({ config: { ...config, [key]: value } } as any);
  };

  const handleAddInputMapping = () => {
    handleConfigChange("input_mapping", {
      ...config.input_mapping,
      [`param_${Object.keys(config.input_mapping).length + 1}`]: "",
    });
  };

  const handleUpdateInputMapping = (key: string, value: string) => {
    handleConfigChange("input_mapping", {
      ...config.input_mapping,
      [key]: value,
    });
  };

  const handleDeleteInputMapping = (key: string) => {
    const newMapping = { ...config.input_mapping };
    delete newMapping[key];
    handleConfigChange("input_mapping", newMapping);
  };

  const selectedSkill = skills.find((s) => s.id === config.skill_id);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>原子Skill</label>
        <Space.Compact style={{ width: "100%" }}>
          <Select
            value={config.skill_id || undefined}
            onChange={(value) => handleConfigChange("skill_id", value)}
            size="small"
            style={{ flex: 1 }}
            placeholder="选择原子Skill..."
            showSearch
            options={skills.map((s) => ({
              value: s.id,
              label: (
                <span>
                  {s.name}
                  <Tag
                    style={{ marginLeft: 4, fontSize: 9 }}
                    color={s.entry_type === "builtin"
                      ? "blue"
                      : s.entry_type === "mcp"
                      ? "purple"
                      : s.entry_type === "local"
                      ? "green"
                      : "orange"}
                  >
                    {s.entry_type}
                  </Tag>
                </span>
              ),
            }))}
          />
          {onEditSkill && config.skill_id && (
            <Button
              size="small"
              icon={<EditOutlined />}
              onClick={() => onEditSkill(config.skill_id)}
            />
          )}
        </Space.Compact>
        {selectedSkill && (
          <div style={{ fontSize: 10, color: "#888", marginTop: 4 }}>
            {selectedSkill.description}
          </div>
        )}
      </div>

      <div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
          <label style={{ color: "#999", fontSize: 11 }}>输入映射</label>
          <a style={{ fontSize: 10 }} onClick={handleAddInputMapping}>+ 添加</a>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {Object.entries(config.input_mapping || {}).map(([key, value]) => (
            <div key={key} style={{ display: "flex", gap: 4, alignItems: "center" }}>
              <Input value={key} size="small" disabled style={{ width: 80, fontSize: 10 }} />
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
          placeholder="skill_output"
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
