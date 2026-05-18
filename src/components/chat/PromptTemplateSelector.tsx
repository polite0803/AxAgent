import { usePromptTemplateStore } from "@/stores";
import type { PromptTemplate } from "@/types";
import { Input, List, Modal, Spin, Tag } from "antd";
import { Search } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

interface PromptTemplateSelectorProps {
  onSelect: (template: PromptTemplate, filledContent: string) => void;
}

export function PromptTemplateSelector({ onSelect }: PromptTemplateSelectorProps) {
  const { t } = useTranslation();
  const { templates, loading, loadTemplates } = usePromptTemplateStore();
  const [searchText, setSearchText] = useState("");
  const [selectedTemplate, setSelectedTemplate] = useState<PromptTemplate | null>(null);
  const [variableValues, setVariableValues] = useState<Record<string, string>>({});
  const [modalOpen, setModalOpen] = useState(false);

  useEffect(() => {
    loadTemplates();
  }, [loadTemplates]);

  const activeTemplates = useMemo(
    () => templates.filter((t) => t.isActive),
    [templates],
  );

  const filteredTemplates = useMemo(
    () =>
      activeTemplates.filter((t) => {
        const q = searchText.toLowerCase();
        return (
          t.name.toLowerCase().includes(q)
          || (t.description?.toLowerCase().includes(q) ?? false)
          || (t.category?.toLowerCase().includes(q) ?? false)
        );
      }),
    [activeTemplates, searchText],
  );

  const handleSelect = useCallback((template: PromptTemplate) => {
    setSelectedTemplate(template);
    setVariableValues({});
    setModalOpen(true);
  }, []);

  const handleFillVariables = useCallback(() => {
    if (!selectedTemplate) { return; }

    let content = selectedTemplate.content;

    // 解析 schema 或从内容自动检测变量
    let varNames: string[] = [];
    if (selectedTemplate.variablesSchema) {
      try {
        const schema = JSON.parse(selectedTemplate.variablesSchema);
        varNames = Object.keys(schema);
      } catch {
        varNames = [];
      }
    }
    // 回退：从内容中自动检测 {varName}
    if (varNames.length === 0) {
      varNames = parseVariables(selectedTemplate.content);
    }

    // 如果用户填了变量值就用，否则保留占位符
    // js-hoist-regexp: 模式依赖迭代变量 varName，无法提升
    for (const varName of varNames) {
      const value = variableValues[varName];
      if (value !== undefined && value !== "") {
        content = content.replace(new RegExp(`\\{${varName}\\}`, "g"), value);
      }
    }

    onSelect(selectedTemplate, content);
    setModalOpen(false);
    setSelectedTemplate(null);
    setVariableValues({});
  }, [selectedTemplate, variableValues, onSelect]);

  // 从内容中自动检测 {varName} 占位符
  const parsedVariables = useMemo(
    () => (selectedTemplate ? parseVariables(selectedTemplate.content) : []),
    [selectedTemplate],
  );

  // 决定显示哪些变量输入：schema 中定义的优先，回退到自动检测
  const displayVariables = useMemo(() => {
    if (!selectedTemplate) { return []; }
    if (selectedTemplate.variablesSchema) {
      try {
        return Object.entries(JSON.parse(selectedTemplate.variablesSchema));
      } catch {
        return parsedVariables.map((v) => [v, "string"] as [string, string]);
      }
    }
    return parsedVariables.map((v) => [v, "string"] as [string, string]);
  }, [selectedTemplate, parsedVariables]);

  return (
    <div className="w-80">
      <div className="px-3 pt-3">
        <Input
          id="prompt-template-selector-input-27"
          placeholder={t("promptTemplates.searchPlaceholder")}
          prefix={<Search size={14} />}
          value={searchText}
          onChange={(e) => setSearchText(e.target.value)}
          allowClear
        />
      </div>

      <div className="max-h-60 overflow-y-auto px-1">
        <Spin spinning={loading}>
          <List
            dataSource={filteredTemplates.slice(0, 10)}
            size="small"
            locale={{ emptyText: t("promptTemplates.noTemplates") }}
            renderItem={(template) => (
              <List.Item
                className="cursor-pointer hover:bg-zinc-50 rounded px-2 py-1.5 transition-colors"
                onClick={() => handleSelect(template)}
              >
                <div className="flex flex-col w-full min-w-0">
                  <div className="flex items-center gap-1.5 min-w-0">
                    <span className="text-sm font-medium truncate">{template.name}</span>
                    <Tag className="shrink-0" color="blue" style={{ fontSize: 10, lineHeight: "16px" }}>
                      v{template.version}
                    </Tag>
                    {template.category && (
                      <Tag className="shrink-0" style={{ fontSize: 10, lineHeight: "16px" }}>
                        {template.category}
                      </Tag>
                    )}
                  </div>
                  <span className="text-xs text-zinc-400 truncate mt-0.5">
                    {template.description || template.content.slice(0, 60)}
                  </span>
                </div>
              </List.Item>
            )}
          />
        </Spin>
      </div>

      <Modal
        title={selectedTemplate?.name ?? t("promptTemplates.title")}
        open={modalOpen}
        onOk={handleFillVariables}
        onCancel={() => setModalOpen(false)}
        okText={t("common.confirm")}
        cancelText={t("common.cancel")}
        width={640}
        destroyOnClose
      >
        {selectedTemplate && (
          <div className="py-2">
            {selectedTemplate.description && (
              <p className="text-sm text-zinc-400 mb-3">{selectedTemplate.description}</p>
            )}

            {/* 变量输入 */}
            {displayVariables.length > 0 && (
              <div className="mb-3">
                <p className="text-sm text-zinc-500 mb-2">{t("promptTemplates.fillVariables")}</p>
                <div className="space-y-2 max-h-48 overflow-y-auto">
                  {displayVariables.map(([varName, varType]) => (
                    <div key={varName}>
                      <label className="text-xs text-zinc-400 mb-0.5 block">
                        {varName} <span className="text-zinc-300">({String(varType)})</span>
                      </label>
                      <Input
                        id="prompt-template-selector-input-28"
                        size="small"
                        placeholder={`{${varName}}`}
                        value={variableValues[varName] || ""}
                        onChange={(e) => setVariableValues((prev) => ({ ...prev, [varName]: e.target.value }))}
                      />
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* 内容预览 */}
            <div>
              <p className="text-sm text-zinc-500 mb-1">{t("promptTemplates.preview")}</p>
              <div className="bg-zinc-50 border rounded p-2.5 text-sm whitespace-pre-wrap max-h-40 overflow-y-auto text-zinc-600">
                {selectedTemplate.content}
              </div>
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}

function parseVariables(content: string): string[] {
  const matches = content.match(/\{([^}]+)\}/g) || [];
  return [...new Set(matches.map((m) => m.slice(1, -1)))];
}
