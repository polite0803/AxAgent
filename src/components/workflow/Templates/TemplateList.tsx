import { DropdownMenu } from "@/components/layout/DropdownMenu";
import { invoke } from "@/lib/invoke";
import { useWorkflowEditorStore } from "@/stores";
import { Button, Card, Empty, Input, message, Modal, Select, Spin, Tag, theme } from "antd";
import { Copy, Download, Edit2, Eye, History, MoreVertical, Plus, Search, Trash2 } from "lucide-react";
import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import type { WorkflowTemplateResponse } from "../types";
import { VersionHistoryModal } from "./VersionHistoryModal";

/** Maps preset template IDs to exact i18n translation keys. */
const PRESET_I18N_KEYS = {
  "code-review": {
    name: "workflow.codeReview.name",
    description: "workflow.codeReview.description",
  },
  "bug-fix": {
    name: "workflow.bugFix.name",
    description: "workflow.bugFix.description",
  },
  "doc-gen": {
    name: "workflow.docGen.name",
    description: "workflow.docGen.description",
  },
  "test-gen": {
    name: "workflow.testGen.name",
    description: "workflow.testGen.description",
  },
  refactor: {
    name: "workflow.refactor.name",
    description: "workflow.refactor.description",
  },
  explore: {
    name: "workflow.explore.name",
    description: "workflow.explore.description",
  },
  performance: {
    name: "workflow.performance.name",
    description: "workflow.performance.description",
  },
  security: {
    name: "workflow.security.name",
    description: "workflow.security.description",
  },
  migration: {
    name: "workflow.migration.name",
    description: "workflow.migration.description",
  },
  "api-design": {
    name: "workflow.apiDesign.name",
    description: "workflow.apiDesign.description",
  },
  "debug-env": {
    name: "workflow.debugEnv.name",
    description: "workflow.debugEnv.description",
  },
  feature: {
    name: "workflow.feature.name",
    description: "workflow.feature.description",
  },
  "knowledge-extract": {
    name: "workflow.knowledgeExtract.name",
    description: "workflow.knowledgeExtract.description",
  },
  "knowledge-to-code": {
    name: "workflow.knowledgeToCode.name",
    description: "workflow.knowledgeToCode.description",
  },
} as const;

type PresetI18nKey = keyof typeof PRESET_I18N_KEYS;

interface TemplateListProps {
  onSelectTemplate: (template: WorkflowTemplateResponse) => void;
  onCreateNew: () => void;
  onEditTemplate?: (template: WorkflowTemplateResponse) => void;
}

const TAG_COLORS: Record<string, string> = {
  ai: "blue",
  automation: "green",
  workflow: "cyan",
  agent: "purple",
  chatbot: "magenta",
  "data-processing": "orange",
  code: "geekblue",
  review: "lime",
  quality: "green",
  debug: "red",
  fix: "volcano",
  troubleshoot: "orange",
  docs: "purple",
  api: "blue",
  readme: "cyan",
  testing: "green",
  tdd: "lime",
  coverage: "geekblue",
};

export const TemplateList: React.FC<TemplateListProps> = ({
  onSelectTemplate,
  onCreateNew,
  onEditTemplate,
}) => {
  const { t } = useTranslation("translation");
  const { token } = theme.useToken();
  const {
    templates,
    isLoading,
    loadTemplates,
    deleteTemplate,
    duplicateTemplate,
  } = useWorkflowEditorStore();
  const [searchText, setSearchText] = useState("");
  const [filterTag, setFilterTag] = useState<string | undefined>(undefined);
  const [filterPreset, setFilterPreset] = useState<boolean | undefined>(
    undefined,
  );
  const [deleteModalVisible, setDeleteModalVisible] = useState(false);
  const [templateToDelete, setTemplateToDelete] = useState<WorkflowTemplateResponse | null>(null);
  const [versionHistoryVisible, setVersionHistoryVisible] = useState(false);
  const [templateForVersionHistory, setTemplateForVersionHistory] = useState<WorkflowTemplateResponse | null>(null);
  const [importingPresets, setImportingPresets] = useState(false);

  const handleImportPresetTemplates = async () => {
    setImportingPresets(true);
    try {
      await invoke<number>("seed_preset_templates");
      message.success(t("workflow.templateList.presetsImported"));
      await loadTemplates();
    } catch (e) {
      message.error(t("workflow.templateList.presetsImportFailed"));
    } finally {
      setImportingPresets(false);
    }
  };

  React.useEffect(() => {
    loadTemplates();
  }, [loadTemplates]);

  const allTags = React.useMemo(() => {
    const tagSet = new Set<string>();
    templates.forEach((t) => {
      t.tags?.forEach((tag) => tagSet.add(tag));
    });
    return Array.from(tagSet).sort();
  }, [templates]);

  const filteredTemplates = React.useMemo(() => {
    return templates.filter((template) => {
      const matchesSearch = !searchText
        || template.name.toLowerCase().includes(searchText.toLowerCase())
        || template.description?.toLowerCase().includes(searchText.toLowerCase());
      const matchesTag = !filterTag || template.tags?.includes(filterTag);
      const matchesPreset = filterPreset === undefined || template.is_preset === filterPreset;
      return matchesSearch && matchesTag && matchesPreset;
    });
  }, [templates, searchText, filterTag, filterPreset]);

  const handleDelete = async () => {
    if (!templateToDelete) {
      return;
    }
    try {
      await deleteTemplate(templateToDelete.id);
      message.success(t("workflow.templateList.deleted"));
      setDeleteModalVisible(false);
      setTemplateToDelete(null);
    } catch (error) {
      message.error(t("workflow.templateList.deleteFailed"));
    }
  };

  const handleDuplicate = async (template: WorkflowTemplateResponse) => {
    try {
      await duplicateTemplate(template.id);
      message.success(t("workflow.templateList.copied"));
    } catch (error) {
      message.error(t("workflow.templateList.copyFailed"));
    }
  };

  const renderTemplateCard = (template: WorkflowTemplateResponse) => {
    const menuItems = [
      {
        key: "view",
        icon: <Eye size={14} />,
        label: t("workflow.templateList.view"),
        onClick: () => onSelectTemplate(template),
      },
    ];

    if (template.is_editable) {
      menuItems.push(
        {
          key: "edit",
          icon: <Edit2 size={14} />,
          label: t("workflow.templateList.edit"),
          onClick: () => onEditTemplate?.(template),
        },
        {
          key: "versionHistory",
          icon: <History size={14} />,
          label: t("workflow.templateList.versionHistory"),
          onClick: () => {
            setTemplateForVersionHistory(template);
            setVersionHistoryVisible(true);
          },
        },
        {
          key: "duplicate",
          icon: <Copy size={14} />,
          label: t("workflow.templateList.duplicate"),
          onClick: () => handleDuplicate(template),
        },
        {
          key: "delete",
          icon: <Trash2 size={14} style={{ color: "#ff4d4f" }} />,
          label: t("workflow.templateList.delete"),
          onClick: () => {
            setTemplateToDelete(template);
            setDeleteModalVisible(true);
          },
        },
      );
    }

    return (
      <Card
        key={template.id}
        size="small"
        hoverable
        onClick={() => onSelectTemplate(template)}
        style={{
          background: token.colorBgContainer,
          border: `1px solid ${token.colorBorderSecondary}`,
          cursor: "pointer",
          transition: "box-shadow 0.2s, transform 0.2s",
        }}
        styles={{
          body: { padding: 12 },
        }}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "flex-start",
          }}
        >
          <div style={{ flex: 1 }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                marginBottom: 6,
              }}
            >
              <span style={{ fontSize: 16 }}>{template.icon || "📋"}</span>
              <span
                style={{
                  fontWeight: 500,
                  color: token.colorText,
                  fontSize: 14,
                }}
              >
                {(() => {
                  const presetI18n = PRESET_I18N_KEYS[template.id as PresetI18nKey];
                  return presetI18n ? t(presetI18n.name) : template.name;
                })()}
              </span>
              {template.is_preset && (
                <Tag color="gold" style={{ marginLeft: 4, fontSize: 12 }}>
                  {t("workflow.templateList.preset")}
                </Tag>
              )}
              {!template.is_editable && (
                <Tag color="default" style={{ fontSize: 12 }}>
                  {t("workflow.templateList.readonly")}
                </Tag>
              )}
            </div>
            <div
              style={{
                color: token.colorTextSecondary,
                fontSize: 12,
                marginBottom: 8,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {(() => {
                const presetI18n = PRESET_I18N_KEYS[template.id as PresetI18nKey];
                return presetI18n
                  ? t(presetI18n.description)
                  : template.description
                    || t("workflow.templateList.noDescription");
              })()}
            </div>
            <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
              {template.tags?.slice(0, 4).map((tag) => (
                <Tag
                  key={tag}
                  color={TAG_COLORS[tag] || "default"}
                  style={{ fontSize: 12, margin: 0 }}
                >
                  {tag}
                </Tag>
              ))}
              {template.tags && template.tags.length > 4 && (
                <Tag style={{ fontSize: 12, margin: 0 }}>
                  +{template.tags.length - 4}
                </Tag>
              )}
            </div>
          </div>
          <DropdownMenu
            items={menuItems}
            trigger={["click"]}
          >
            <Button
              type="text"
              size="small"
              data-testid="template-card-more-btn"
              icon={<MoreVertical size={14} />}
              onClick={(e) => e.stopPropagation()}
              style={{ color: token.colorTextTertiary }}
            />
          </DropdownMenu>
        </div>
      </Card>
    );
  };

  if (isLoading) {
    return (
      <div
        style={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          height: 200,
        }}
      >
        <Spin size="large" />
      </div>
    );
  }

  return (
    <div style={{ padding: 16 }}>
      <div style={{ marginBottom: 16 }}>
        <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
          <Input
            id="template-list-input-129"
            data-testid="template-list-search"
            placeholder={t("workflow.templateList.searchPlaceholder")}
            prefix={<Search size={14} color={token.colorTextTertiary} />}
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            size="small"
            style={{ flex: 1 }}
            allowClear
          />
          <Select
            placeholder={t("workflow.templateList.tagPlaceholder")}
            value={filterTag}
            onChange={setFilterTag}
            allowClear
            size="small"
            style={{ width: 100 }}
            options={allTags.map((tag) => ({ value: tag, label: tag }))}
          />
          <Select
            placeholder={t("workflow.templateList.typePlaceholder")}
            value={filterPreset}
            onChange={setFilterPreset}
            allowClear
            size="small"
            style={{ width: 100 }}
            options={[
              { value: true, label: t("workflow.templateList.preset") },
              { value: false, label: t("workflow.templateList.custom") },
            ]}
          />
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <Button
            type="primary"
            icon={<Plus size={14} />}
            onClick={onCreateNew}
            style={{ flex: 1 }}
            size="small"
          >
            {t("workflow.templateList.newTemplate")}
          </Button>
          <Button
            icon={<Download size={14} />}
            onClick={handleImportPresetTemplates}
            loading={importingPresets}
            size="small"
            title={t("workflow.templateList.importPresetsTitle")}
          >
            {t("workflow.templateList.importPresets")}
          </Button>
        </div>
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))",
          gap: 12,
        }}
      >
        {filteredTemplates.map(renderTemplateCard)}
      </div>

      {filteredTemplates.length === 0 && !isLoading && (
        <Empty
          description={searchText || filterTag
            ? t("workflow.templateList.noMatchFound")
            : t("workflow.templateList.noTemplates")}
          style={{ marginTop: 48 }}
        />
      )}

      <Modal
        title={t("workflow.templateList.confirmDelete")}
        open={deleteModalVisible}
        onOk={handleDelete}
        onCancel={() => {
          setDeleteModalVisible(false);
          setTemplateToDelete(null);
        }}
        okText={t("workflow.templateList.delete")}
        okButtonProps={{ danger: true }}
      >
        <p>
          {t("workflow.templateList.confirmDeleteMessage", {
            name: templateToDelete?.name,
          })}
        </p>
        <p style={{ color: "#ff4d4f", fontSize: 12 }}>
          {t("workflow.templateList.irreversible")}
        </p>
      </Modal>

      <VersionHistoryModal
        visible={versionHistoryVisible}
        template={templateForVersionHistory}
        onClose={() => {
          setVersionHistoryVisible(false);
          setTemplateForVersionHistory(null);
        }}
        onLoadVersion={onSelectTemplate}
      />
    </div>
  );
};
