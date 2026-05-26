import { DropdownMenu } from "@/components/layout/DropdownMenu";
import { usePromptTemplateStore } from "@/stores/feature/promptTemplateStore";
import type {
  CreatePromptTemplateInput,
  ExportPromptFormat,
  PromptTemplate,
  PromptTemplateVersion,
  UpdatePromptTemplateInput,
} from "@/types";
import {
  DeleteOutlined,
  EditOutlined,
  ExportOutlined,
  HistoryOutlined,
  ImportOutlined,
  PlusOutlined,
  RollbackOutlined,
  SearchOutlined,
  StarFilled,
  StarOutlined,
} from "@ant-design/icons";
import { Button, Empty, Form, Input, List, message, Modal, Select, Space, Spin, Tag, Typography } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { PromptImportModal } from "./PromptImportModal";
import { SettingsGroup } from "./SettingsGroup";

const { Title, Text, Paragraph } = Typography;

// 从模板列表中提取所有分类
function extractCategories(templates: PromptTemplate[]): string[] {
  const cats = new Set<string>();
  for (const t of templates) {
    if (t.category) {
      cats.add(t.category);
    }
  }
  return Array.from(cats).sort();
}

export function PromptTemplatesSettings() {
  const { t } = useTranslation();
  const [messageApi, contextHolder] = message.useMessage();
  const [isEditorOpen, setIsEditorOpen] = useState(false);
  const [isVersionHistoryOpen, setIsVersionHistoryOpen] = useState(false);
  const [isImportOpen, setIsImportOpen] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<PromptTemplate | null>(
    null,
  );
  const [versionHistoryTemplate, setVersionHistoryTemplate] = useState<PromptTemplate | null>(null);
  const [viewingVersionContent, setViewingVersionContent] = useState<PromptTemplateVersion | null>(null);
  const [searchText, setSearchText] = useState("");
  const [categoryFilter, setCategoryFilter] = useState<string | undefined>();
  const [form] = Form.useForm();

  const {
    templates,
    versions,
    loading,
    loadTemplates,
    createTemplate,
    updateTemplate,
    deleteTemplate,
    loadVersions,
    rollbackTemplate,
    exportTemplates,
  } = usePromptTemplateStore();

  useEffect(() => {
    loadTemplates();
  }, [loadTemplates]);

  const categories = useMemo(() => extractCategories(templates), [templates]);

  const filteredTemplates = templates.filter((tmpl) => {
    const matchSearch = tmpl.name.toLowerCase().includes(searchText.toLowerCase())
      || (tmpl.description?.toLowerCase().includes(searchText.toLowerCase())
        ?? false);
    const matchCategory = !categoryFilter || tmpl.category === categoryFilter;
    return matchSearch && matchCategory;
  });

  const handleCreate = useCallback(() => {
    setEditingTemplate(null);
    form.resetFields();
    setIsEditorOpen(true);
  }, [form]);

  const handleEdit = useCallback(
    (tmpl: PromptTemplate) => {
      setEditingTemplate(tmpl);
      form.setFieldsValue({
        name: tmpl.name,
        description: tmpl.description,
        content: tmpl.content,
        variablesSchema: tmpl.variablesSchema,
        category: tmpl.category,
        tags: tmpl.tags?.join(", "),
        author: tmpl.author,
        source: tmpl.source,
      });
      setIsEditorOpen(true);
    },
    [form],
  );

  const handleSave = useCallback(async () => {
    try {
      const values = await form.validateFields();
      const tagsArr = values.tags
        ? values.tags.split(",").flatMap((s: string) => {
          const r = s.trim();
          return r ? [r] : [];
        })
        : undefined;

      if (editingTemplate) {
        const updateInput: UpdatePromptTemplateInput = {
          name: values.name,
          description: values.description,
          content: values.content,
          variablesSchema: values.variablesSchema,
          category: values.category,
          tags: tagsArr,
          author: values.author,
          source: values.source,
        };
        await updateTemplate(editingTemplate.id, updateInput);
        messageApi.success(t("promptTemplates.updated"));
      } else {
        const input: CreatePromptTemplateInput = {
          name: values.name,
          description: values.description,
          content: values.content,
          variablesSchema: values.variablesSchema,
          category: values.category,
          tags: tagsArr,
          author: values.author,
          source: values.source,
          sourceType: "manual",
          format: "plain",
        };
        await createTemplate(input);
        messageApi.success(t("promptTemplates.created"));
      }
      setIsEditorOpen(false);
      form.resetFields();
    } catch (e) {
      messageApi.error(String(e));
    }
  }, [editingTemplate, form, createTemplate, updateTemplate, messageApi, t]);

  const handleDelete = useCallback(
    async (tmpl: PromptTemplate) => {
      Modal.confirm({
        title: t("promptTemplates.deleteTemplateConfirm"),
        content: tmpl.name,
        okText: t("common.delete"),
        cancelText: t("common.cancel"),
        okButtonProps: { danger: true },
        onOk: async () => {
          await deleteTemplate(tmpl.id);
          messageApi.success(t("promptTemplates.deleted"));
        },
      });
    },
    [deleteTemplate, messageApi, t],
  );

  const handleViewHistory = useCallback(
    async (tmpl: PromptTemplate) => {
      setVersionHistoryTemplate(tmpl);
      setViewingVersionContent(null);
      await loadVersions(tmpl.id);
      setIsVersionHistoryOpen(true);
    },
    [loadVersions],
  );

  const handleRollback = useCallback(
    async (version: PromptTemplateVersion) => {
      if (!versionHistoryTemplate) {
        return;
      }
      Modal.confirm({
        title: t("promptTemplates.rollbackConfirm"),
        content: t("promptTemplates.rollbackConfirmDetail", {
          name: versionHistoryTemplate.name,
          version: version.version,
        }),
        onOk: async () => {
          const result = await rollbackTemplate(
            versionHistoryTemplate.id,
            version.version,
          );
          if (result) {
            messageApi.success(t("promptTemplates.rollbackSuccess"));
            setIsVersionHistoryOpen(false);
          }
        },
      });
    },
    [versionHistoryTemplate, rollbackTemplate, messageApi, t],
  );

  const handleExport = useCallback(
    async (format: ExportPromptFormat) => {
      const exported = await exportTemplates([], format);
      if (exported) {
        const ext = format === "markdown" ? "md" : format;
        const blob = new Blob([exported], { type: "text/plain;charset=utf-8" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = `prompt-templates.${ext}`;
        a.click();
        URL.revokeObjectURL(url);
        messageApi.success(t("promptTemplates.exportSuccess"));
      }
    },
    [exportTemplates, messageApi, t],
  );

  const { toggleFavorite } = usePromptTemplateStore();

  if (loading && templates.length === 0) {
    return (
      <div className="flex items-center justify-center p-8">
        <Spin />
      </div>
    );
  }

  return (
    <SettingsGroup>
      {contextHolder}
      <div className="px-4 py-3">
        {/* 工具栏 */}
        <div className="flex items-center justify-between mb-4 flex-wrap gap-2">
          <Title level={5} className="m-0">
            {t("promptTemplates.title")}
          </Title>
          <Space>
            <Button
              icon={<ImportOutlined />}
              onClick={() => setIsImportOpen(true)}
            >
              {t("promptTemplates.import")}
            </Button>
            <DropdownMenu
              items={[
                { key: "json", label: "JSON", onClick: () => handleExport("json") },
                { key: "yaml", label: "YAML", onClick: () => handleExport("yaml") },
                { key: "markdown", label: "Markdown", onClick: () => handleExport("markdown") },
              ]}
            >
              <Button icon={<ExportOutlined />}>
                {t("promptTemplates.export")}
              </Button>
            </DropdownMenu>
            <Button
              type="primary"
              icon={<PlusOutlined />}
              onClick={handleCreate}
            >
              {t("promptTemplates.newTemplate")}
            </Button>
          </Space>
        </div>

        {/* 搜索和筛选 */}
        <div className="flex gap-2 mb-4 flex-wrap">
          <Input
            id="prompt-templates-settings-input-110"
            placeholder={t("promptTemplates.searchPlaceholder")}
            prefix={<SearchOutlined />}
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            className="max-w-xs"
            allowClear
          />
          <Select
            id="prompt-templates-settings-select-111"
            placeholder={t("promptTemplates.filterByCategory")}
            value={categoryFilter}
            onChange={setCategoryFilter}
            allowClear
            className="min-w-[160px]"
            options={categories.map((c) => ({ label: c, value: c }))}
          />
        </div>

        {/* 列表 */}
        {filteredTemplates.length === 0 ? <Empty description={t("promptTemplates.noTemplates")} /> : (
          <List
            dataSource={filteredTemplates}
            renderItem={(tmpl) => (
              <List.Item
                actions={[
                  <Button
                    key="fav"
                    type="text"
                    size="small"
                    icon={tmpl.isFavorite ? <StarFilled style={{ color: "#faad14" }} /> : <StarOutlined />}
                    onClick={() => toggleFavorite(tmpl.id)}
                  />,
                  <Button
                    key="history"
                    type="text"
                    size="small"
                    icon={<HistoryOutlined />}
                    onClick={() => handleViewHistory(tmpl)}
                  >
                    {t("promptTemplates.history")}
                  </Button>,
                  <Button
                    key="edit"
                    type="text"
                    size="small"
                    icon={<EditOutlined />}
                    onClick={() => handleEdit(tmpl)}
                  >
                    {t("common.edit")}
                  </Button>,
                  <Button
                    key="delete"
                    type="text"
                    size="small"
                    danger
                    icon={<DeleteOutlined />}
                    onClick={() => handleDelete(tmpl)}
                  >
                    {t("common.delete")}
                  </Button>,
                ]}
              >
                <List.Item.Meta
                  title={
                    <Space wrap size={[0, 4]}>
                      <span>{tmpl.name}</span>
                      {tmpl.isActive && <Tag color="green">{t("common.active")}</Tag>}
                      {tmpl.abTestEnabled && <Tag color="blue">{t("promptTemplates.abTest")}</Tag>}
                      {tmpl.category && <Tag>{tmpl.category}</Tag>}
                      <Tag color="default">v{tmpl.version}</Tag>
                      {tmpl.sourceType === "imported" && (
                        <Tag color="purple">
                          {t("promptTemplates.imported")}
                        </Tag>
                      )}
                    </Space>
                  }
                  description={
                    <div>
                      <div className="text-xs text-zinc-500">
                        {tmpl.description || tmpl.content.slice(0, 80) + "..."}
                      </div>
                      {tmpl.tags && tmpl.tags.length > 0 && (
                        <div className="mt-1">
                          {tmpl.tags.map((tag) => (
                            <Tag key={tag} className="text-xs" color="geekblue">
                              {tag}
                            </Tag>
                          ))}
                        </div>
                      )}
                    </div>
                  }
                />
              </List.Item>
            )}
          />
        )}
      </div>

      {/* 导入弹窗 */}
      <PromptImportModal
        open={isImportOpen}
        onClose={() => setIsImportOpen(false)}
      />

      {/* 编辑弹窗 */}
      <Modal
        title={editingTemplate
          ? t("promptTemplates.editTemplate")
          : t("promptTemplates.newTemplate")}
        open={isEditorOpen}
        onOk={handleSave}
        onCancel={() => setIsEditorOpen(false)}
        width={700}
        destroyOnHidden
      >
        <Form form={form} layout="vertical" className="mt-4">
          <Form.Item
            name="name"
            label={t("common.name")}
            rules={[
              { required: true, message: t("promptTemplates.nameRequired") },
            ]}
          >
            <Input
              name="name"
              placeholder={t("promptTemplates.namePlaceholder")}
            />
          </Form.Item>
          <Form.Item name="description" label={t("common.description")}>
            <Input.TextArea
              name="description"
              placeholder={t("promptTemplates.descriptionPlaceholder")}
              rows={2}
            />
          </Form.Item>
          <Form.Item name="category" label={t("promptTemplates.category")}>
            <Input
              name="category"
              placeholder={t("promptTemplates.categoryPlaceholder")}
            />
          </Form.Item>
          <Form.Item name="tags" label={t("promptTemplates.tags")}>
            <Input
              name="tags"
              placeholder={t("promptTemplates.tagsPlaceholder")}
            />
          </Form.Item>
          <Form.Item name="author" label={t("promptTemplates.author")}>
            <Input
              name="author"
              placeholder={t("promptTemplates.authorPlaceholder")}
            />
          </Form.Item>
          <Form.Item name="source" label={t("promptTemplates.source")}>
            <Input
              name="source"
              placeholder={t("promptTemplates.sourcePlaceholder")}
            />
          </Form.Item>
          <Form.Item
            name="content"
            label={t("promptTemplates.content")}
            rules={[
              { required: true, message: t("promptTemplates.contentRequired") },
            ]}
          >
            <Input.TextArea
              name="content"
              placeholder={t("promptTemplates.contentPlaceholder")}
              rows={8}
              className="font-mono text-sm"
            />
          </Form.Item>
          <Form.Item
            name="variablesSchema"
            label={t("promptTemplates.variablesSchema") + " (JSON)"}
          >
            <Input.TextArea
              name="variablesSchema"
              placeholder='{"variable": "type"}'
              rows={3}
              className="font-mono text-sm"
            />
          </Form.Item>
        </Form>
      </Modal>

      {/* 版本历史弹窗 */}
      <Modal
        title={t("promptTemplates.versionHistory", {
          name: versionHistoryTemplate?.name ?? "",
        })}
        open={isVersionHistoryOpen}
        onCancel={() => {
          setIsVersionHistoryOpen(false);
          setViewingVersionContent(null);
        }}
        footer={null}
        width={700}
      >
        <div className="py-2">
          {versions.length === 0 ? <Empty description={t("promptTemplates.noVersions")} /> : (
            <List
              dataSource={versions}
              renderItem={(item) => (
                <List.Item
                  actions={[
                    <Button
                      key="view"
                      size="small"
                      onClick={() =>
                        setViewingVersionContent(
                          viewingVersionContent?.id === item.id ? null : item,
                        )}
                    >
                      {viewingVersionContent?.id === item.id
                        ? t("common.hide")
                        : t("common.view")}
                    </Button>,
                    <Button
                      key="rollback"
                      size="small"
                      icon={<RollbackOutlined />}
                      onClick={() => handleRollback(item)}
                    >
                      {t("promptTemplates.rollback")}
                    </Button>,
                  ]}
                >
                  <List.Item.Meta
                    title={
                      <Space>
                        <Tag color="blue">v{item.version}</Tag>
                        {item.category && <Tag>{item.category}</Tag>}
                      </Space>
                    }
                    description={
                      <div>
                        {item.changelog && (
                          <Text type="secondary" className="text-xs">
                            {item.changelog}
                          </Text>
                        )}
                      </div>
                    }
                  />
                </List.Item>
              )}
            />
          )}

          {/* 版本内容查看 */}
          {viewingVersionContent && (
            <div className="mt-4 p-3 bg-zinc-50 rounded">
              <Text strong>{t("promptTemplates.versionContent")}:</Text>
              <Paragraph className="mt-2">
                <pre className="whitespace-pre-wrap text-sm font-mono bg-white p-2 rounded border">
                  {viewingVersionContent.content}
                </pre>
              </Paragraph>
              {viewingVersionContent.variablesSchema && (
                <>
                  <Text strong>{t("promptTemplates.variablesSchema")}:</Text>
                  <pre className="whitespace-pre-wrap text-xs font-mono bg-white p-2 rounded border mt-1">
                    {viewingVersionContent.variablesSchema}
                  </pre>
                </>
              )}
            </div>
          )}
        </div>
      </Modal>
    </SettingsGroup>
  );
}
