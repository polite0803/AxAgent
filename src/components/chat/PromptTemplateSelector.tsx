import { invoke } from "@/lib/invoke";
import type { PromptTemplate } from "@/types";
import { Input, List, message, Modal, Tag } from "antd";
import { Search } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface PromptTemplateSelectorProps {
  onSelect: (template: PromptTemplate, filledContent: string) => void;
}

export function PromptTemplateSelector({ onSelect }: PromptTemplateSelectorProps) {
  const { t } = useTranslation();
  const [messageApi, contextHolder] = message.useMessage();
  const [templates, setTemplates] = useState<PromptTemplate[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchText, setSearchText] = useState("");
  const [selectedTemplate, setSelectedTemplate] = useState<PromptTemplate | null>(null);
  const [variableValues, setVariableValues] = useState<Record<string, string>>({});
  const [modalOpen, setModalOpen] = useState(false);

  useEffect(() => {
    loadTemplates();
  }, []);

  const loadTemplates = async () => {
    setLoading(true);
    try {
      const result = await invoke<PromptTemplate[]>("list_prompt_templates");
      setTemplates(result.filter((t) => t.isActive));
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleSelect = useCallback((template: PromptTemplate) => {
    setSelectedTemplate(template);
    setVariableValues({});
    setModalOpen(true);
  }, []);

  const handleFillVariables = useCallback(() => {
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

    onSelect(selectedTemplate, content);
    setModalOpen(false);
    setSelectedTemplate(null);
    setVariableValues({});
  }, [selectedTemplate, variableValues, onSelect]);

  const filteredTemplates = templates.filter(
    (t) =>
      t.name.toLowerCase().includes(searchText.toLowerCase())
      || (t.description?.toLowerCase().includes(searchText.toLowerCase()) ?? false),
  );

  const renderVariableInput = (varName: string, _varType: unknown) => {
    const typeStr = String(_varType || "string");
    if (typeStr === "string" || typeStr === "number") {
      return (
        <Input
          key={varName}
          placeholder={`${varName} (${typeStr})`}
          value={variableValues[varName] || ""}
          onChange={(e) => setVariableValues((prev) => ({ ...prev, [varName]: e.target.value }))}
        />
      );
    }
    return null;
  };

  const parseVariables = (content: string): string[] => {
    const matches = content.match(/\{([^}]+)\}/g) || [];
    return matches.map((m) => m.slice(1, -1)).filter((v, i, arr) => arr.indexOf(v) === i);
  };

  return (
    <>
      {contextHolder}
      <div className="p-2">
        <Input
          placeholder={t("promptTemplates.searchPlaceholder")}
          prefix={<Search size={14} />}
          value={searchText}
          onChange={(e) => setSearchText(e.target.value)}
          className="mb-2"
          allowClear
        />
        <List
          loading={loading}
          dataSource={filteredTemplates.slice(0, 10)}
          size="small"
          renderItem={(template) => (
            <List.Item
              className="cursor-pointer hover:bg-gray-100 rounded px-2 py-1"
              onClick={() => handleSelect(template)}
            >
              <List.Item.Meta
                title={
                  <span className="text-sm">
                    {template.name}
                    <Tag className="ml-2" color="blue" style={{ fontSize: 10 }}>
                      v{template.version}
                    </Tag>
                  </span>
                }
                description={
                  <span className="text-xs text-gray-500">
                    {template.description || template.content.slice(0, 50) + "..."}
                  </span>
                }
              />
            </List.Item>
          )}
          locale={{ emptyText: t("promptTemplates.noTemplates") }}
        />
      </div>

      <Modal
        title={selectedTemplate?.name}
        open={modalOpen}
        onOk={handleFillVariables}
        onCancel={() => setModalOpen(false)}
        okText={t("common.confirm")}
        cancelText={t("common.cancel")}
      >
        {selectedTemplate && (
          <div className="py-2">
            {selectedTemplate.variablesSchema
              ? (
                <div className="space-y-2">
                  <p className="text-sm text-gray-500">{t("promptTemplates.fillVariables")}</p>
                  {Object.entries(JSON.parse(selectedTemplate.variablesSchema)).map(([varName, varType]) => (
                    <div key={varName}>
                      <label className="text-sm font-medium">{varName}</label>
                      {renderVariableInput(varName, varType)}
                    </div>
                  ))}
                </div>
              )
              : (
                <div className="space-y-2">
                  <p className="text-sm text-gray-500">{t("promptTemplates.preview")}</p>
                  <div className="bg-gray-50 p-2 rounded text-sm whitespace-pre-wrap">
                    {selectedTemplate.content}
                  </div>
                  {parseVariables(selectedTemplate.content).length > 0 && (
                    <p className="text-xs text-orange-500">
                      {t("promptTemplates.hasVariables", {
                        variables: parseVariables(selectedTemplate.content).join(", "),
                      })}
                    </p>
                  )}
                </div>
              )}
          </div>
        )}
      </Modal>
    </>
  );
}
