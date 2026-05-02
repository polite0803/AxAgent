import { useAtomicSkillStore } from "@/stores/feature/atomicSkillStore";
import { EditOutlined } from "@ant-design/icons";
import { Button, Divider, Input, message, Modal, Select, Space, Tag } from "antd";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { usePromptTemplateStore } from "@/stores";
import type { WorkflowNode } from "../../types";
import type { PromptTemplate } from "@/types";
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
  const { t } = useTranslation();
  const [messageApi, contextHolder] = message.useMessage();
  const [templateModalOpen, setTemplateModalOpen] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<PromptTemplate | null>(null);
  const [variableValues, setVariableValues] = useState<Record<string, string>>({});

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const config = (node as any).config || {
    skill_id: "",
    input_mapping: {},
    output_var: "",
  };

  const { skills } = useAtomicSkillStore();
  const { templates, loadTemplates } = usePromptTemplateStore();

  useEffect(() => {
    loadTemplates();
  }, []);

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
  const activeTemplates = templates.filter((t) => t.isActive);

  const handleSelectTemplate = (template: PromptTemplate) => {
    setSelectedTemplate(template);
    setVariableValues({});
    setTemplateModalOpen(true);
  };

  const handleApplyTemplate = () => {
    if (!selectedTemplate) return;

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

    handleConfigChange("prompt_template_id", selectedTemplate.id);
    handleConfigChange("prompt_template_content", content);
    setTemplateModalOpen(false);
    setSelectedTemplate(null);
    setVariableValues({});
    messageApi.success(t("promptTemplates.applied"));
  };

  const parseVariables = (content: string): string[] => {
    const matches = content.match(/\{([^}]+)\}/g) || [];
    return matches.map((m) => m.slice(1, -1)).filter((v, i, arr) => arr.indexOf(v) === i);
  };

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

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>Prompt模板（可选）</label>
        <Button
          size="small"
          type="link"
          onClick={() => setTemplateModalOpen(true)}
          style={{ padding: 0 }}
        >
          {config.prompt_template_id ? t("promptTemplates.changeTemplate") : t("promptTemplates.selectFromLibrary")}
        </Button>
        {config.prompt_template_id && (
          <Tag
            closable
            onClose={() => {
              handleConfigChange("prompt_template_id", null);
              handleConfigChange("prompt_template_content", null);
            }}
            style={{ marginLeft: 8 }}
          >
            {templates.find((t) => t.id === config.prompt_template_id)?.name || t("promptTemplates.selected")}
          </Tag>
        )}
      </div>

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

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
        {selectedTemplate ? (
          <div style={{ padding: "12px 0" }}>
            <p style={{ marginBottom: 8 }}>{t("promptTemplates.fillVariables")}</p>
            {Object.entries(selectedTemplate.variablesSchema ? JSON.parse(selectedTemplate.variablesSchema) : {}).map(([varName, varType]) => (
              <div key={varName} style={{ marginBottom: 8 }}>
                <label style={{ display: "block", fontSize: 12, marginBottom: 2 }}>{varName} ({String(varType)})</label>
                <Input
                  placeholder={`${varName} (${String(varType)})`}
                  value={variableValues[varName] || ""}
                  onChange={(e) => setVariableValues((prev) => ({ ...prev, [varName]: e.target.value }))}
                />
              </div>
            ))}
            {parseVariables(selectedTemplate.content).length > 0 && Object.keys(selectedTemplate.variablesSchema ? JSON.parse(selectedTemplate.variablesSchema) : {}).length === 0 && (
              <p style={{ color: "#f59e0b", fontSize: 12 }}>
                {t("promptTemplates.hasVariables", { variables: parseVariables(selectedTemplate.content).join(", ") })}
              </p>
            )}
          </div>
        ) : (
          <div style={{ maxHeight: 400, overflowY: "auto" }}>
            {activeTemplates.length === 0 ? (
              <div style={{ textAlign: "center", padding: 24, color: "#999" }}>
                {t("promptTemplates.noTemplates")}
              </div>
            ) : (
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
