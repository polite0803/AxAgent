// SPDX-License-Identifier: AGPL-3.0-only

import { DynamicUIRenderer } from "@/components/dynamicUI/DynamicUIRenderer";
import { SchemaTreeEditor } from "@/components/dynamicUI/SchemaTreeEditor";
import { generateUIFromNL } from "@/lib/dynamicUI/nl2ui";
import { editUIFromNL } from "@/lib/dynamicUI/nl2ui-edit";
import { validateSchema } from "@/lib/dynamicUI/SchemaValidator";
import { getNextPosition, PIN_GROUPS, setPinnedSchema } from "@/lib/pinned-schemas";
import { useDynamicUIStore } from "@/stores";
import type { DynamicUISchemaRecord, UISchema } from "@/types";
import { EditOutlined, SaveOutlined } from "@ant-design/icons";
import {
  Alert,
  Button,
  Divider,
  Empty,
  Input,
  InputNumber,
  message,
  Modal,
  Select,
  Space,
  Spin,
  Switch,
  Table,
  Tabs,
  Tag,
  Typography,
} from "antd";
import { Bot, Plus, Settings2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph, Title } = Typography;

export function DynamicPagesSettings() {
  const { t } = useTranslation();
  const { schemas, loading, fetchSchemas, createSchema, updateSchema } = useDynamicUIStore();

  const [selectedSchema, setSelectedSchema] = useState<DynamicUISchemaRecord | null>(null);
  const [previewSchema, setPreviewSchema] = useState<UISchema | null>(null);

  // 固定到导航配置（仅创建时显示）
  const [pinToNav, setPinToNav] = useState(true);
  const [pinGroup, setPinGroup] = useState("dashboard");

  // 统一的编辑/创建弹窗状态
  const [editOpen, setEditOpen] = useState(false);
  const [editingRecord, setEditingRecord] = useState<DynamicUISchemaRecord | null>(null);
  const [editSchema, setEditSchema] = useState<UISchema | null>(null);
  const [editJsonText, setEditJsonText] = useState("");
  const [editMode, setEditMode] = useState<"visual" | "ai" | "json">("visual");
  const [editNlPrompt, setEditNlPrompt] = useState("");
  const [editGenerating, setEditGenerating] = useState(false);
  const [editSaving, setEditSaving] = useState(false);
  const [, setEditParseError] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState("");
  const [editTags, setEditTags] = useState<string[]>([]);

  const isCreating = editingRecord === null;

  useEffect(() => {
    void fetchSchemas();
  }, [fetchSchemas]);

  // 表格选中行预览
  const handlePreview = (record: DynamicUISchemaRecord) => {
    setSelectedSchema(record);
    try {
      setPreviewSchema(JSON.parse(record.schema_json) as UISchema);
    } catch {
      setPreviewSchema(null);
    }
  };

  // 打开创建弹窗
  const handleOpenCreate = () => {
    setEditingRecord(null);
    setEditSchema(null);
    setEditJsonText("");
    setEditMode("ai");
    setEditNlPrompt("");
    setEditParseError(null);
    setEditTitle("");
    setEditTags([]);
    setPinToNav(true);
    setPinGroup("dashboard");
    setEditOpen(true);
  };

  // 打开编辑弹窗（加载已有 schema）
  const handleOpenEdit = (record: DynamicUISchemaRecord) => {
    try {
      const parsed = JSON.parse(record.schema_json) as UISchema;
      setEditingRecord(record);
      setEditSchema(parsed);
      setEditJsonText(JSON.stringify(parsed, null, 2));
      setEditMode("visual");
      setEditNlPrompt("");
      setEditParseError(null);
      setEditTitle(record.title);
      setEditTags(record.tags);
      setEditOpen(true);
    } catch {
      message.error(t("dynamicUIManager.invalidJson"));
    }
  };

  // 保存（创建或更新）
  const handleSaveEdit = async () => {
    // 验证 JSON
    let schemaToSave: UISchema;
    try {
      schemaToSave = JSON.parse(editJsonText) as UISchema;
    } catch {
      message.error(t("dynamicUIManager.invalidJson"));
      return;
    }

    // Schema 校验
    const validation = validateSchema(schemaToSave);
    if (!validation.valid) {
      message.error(
        `${t("dynamicUIManager.invalidSchema")}: ${
          validation.errors
            .slice(0, 3)
            .map((e) => e.message)
            .join("; ")
        }`,
      );
      return;
    }

    setEditSaving(true);
    try {
      if (isCreating) {
        if (!editTitle.trim()) {
          message.warning(t("dynamicUIManager.titleRequired"));
          setEditSaving(false);
          return;
        }
        const record = await createSchema({
          title: editTitle,
          description: "",
          category: pinGroup,
          tags: editTags,
          schema_json: editJsonText,
        });

        if (pinToNav) {
          const all = {};
          const pos = getNextPosition(pinGroup, all);
          setPinnedSchema({
            schemaId: record.id,
            title: editTitle,
            group: pinGroup,
            position: pos,
          });
        }
        message.success(t("common.saved"));
      } else {
        await updateSchema(editingRecord.id, { schema_json: editJsonText });
        message.success(t("dynamicUIManager.updateSuccess"));
      }
      setEditOpen(false);
    } catch (err: unknown) {
      message.error(err instanceof Error ? err.message : String(err));
    } finally {
      setEditSaving(false);
    }
  };

  // AI 生成（创建模式）或 AI 编辑（编辑模式）
  const handleAIEdit = async () => {
    if (!editNlPrompt.trim()) { return; }

    if (isCreating) {
      // 创建模式：从零生成
      setEditGenerating(true);
      try {
        const result = await generateUIFromNL(editNlPrompt);
        setEditSchema(result.schema);
        setEditJsonText(JSON.stringify(result.schema, null, 2));
        if (!editTitle) { setEditTitle(result.title); }
        setEditParseError(null);
        message.success(t("dynamicUIManager.generateSuccess"));
      } catch (err: unknown) {
        message.error(err instanceof Error ? err.message : String(err));
      } finally {
        setEditGenerating(false);
      }
    } else if (editSchema) {
      // 编辑模式：基于现有 schema 修改
      setEditGenerating(true);
      try {
        const result = await editUIFromNL(editSchema, editNlPrompt);
        setEditSchema(result.schema);
        setEditJsonText(JSON.stringify(result.schema, null, 2));
        setEditParseError(null);
        message.success(result.description);
      } catch (err: unknown) {
        message.error(err instanceof Error ? err.message : String(err));
      } finally {
        setEditGenerating(false);
      }
    }
  };

  // JSON 实时校验
  const derivedEditParseError = useMemo(() => {
    if (!editJsonText) { return null; }
    try {
      JSON.parse(editJsonText);
      return null;
    } catch (err: unknown) {
      return err instanceof Error ? err.message : String(err);
    }
  }, [editJsonText]);

  // 表格列定义
  const columns = [
    {
      title: t("settings.dynamicPages.actions"),
      key: "actions",
      width: 80,
      render: (_: unknown, record: DynamicUISchemaRecord) => (
        <Button
          type="text"
          icon={<EditOutlined />}
          onClick={(e) => {
            e.stopPropagation();
            handleOpenEdit(record);
          }}
        />
      ),
    },
    {
      title: t("dynamicUIManager.schemaTitle"),
      dataIndex: "title",
      key: "title",
      width: 200,
      render: (title: string) => <Text strong>{title}</Text>,
    },
    {
      title: t("dynamicUIManager.category"),
      dataIndex: "category",
      key: "category",
      width: 120,
      render: (cat: string) => <Tag color="blue">{cat}</Tag>,
    },
    {
      title: t("dynamicUIManager.tags"),
      dataIndex: "tags",
      key: "tags",
      width: 200,
      render: (tags: string[]) => (
        <Space size={4} wrap>
          {tags.length > 0
            ? tags.map((tag) => <Tag key={tag}>{tag}</Tag>)
            : <Text type="secondary">-</Text>}
        </Space>
      ),
    },
    {
      title: t("settings.dynamicPages.fixed"),
      dataIndex: "id",
      key: "pinned",
      width: 100,
      render: (_: string) => <Switch size="small" />,
    },
    {
      title: t("settings.dynamicPages.group"),
      dataIndex: "category",
      key: "group",
      width: 130,
      render: () => (
        <Select
          size="small"
          value="dashboard"
          style={{ width: 100 }}
          options={PIN_GROUPS.map((g) => ({ label: t(g.labelKey), value: g.key }))}
        />
      ),
    },
    {
      title: t("settings.dynamicPages.position"),
      key: "position",
      width: 90,
      render: () => <InputNumber size="small" min={0} defaultValue={0} style={{ width: 70 }} />,
    },
  ];

  return (
    <div>
      {/* 标题栏 */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <Title level={4}>
            <Settings2 size={18} className="inline mr-2" style={{ verticalAlign: -2 }} />
            {t("settings.dynamicPages.title")}
          </Title>
          <Paragraph type="secondary">
            {t("settings.dynamicPages.description")}
          </Paragraph>
        </div>
        <Button type="primary" icon={<Plus size={16} />} onClick={handleOpenCreate}>
          {t("settings.dynamicPages.createNew")}
        </Button>
      </div>

      {/* Schema 列表 */}
      <Spin spinning={loading}>
        <Table
          dataSource={schemas}
          columns={columns}
          rowKey="id"
          pagination={false}
          onRow={(record) => ({
            onClick: () => handlePreview(record),
            style: {
              cursor: "pointer",
              background: selectedSchema?.id === record.id
                ? "var(--color-primary-bg)"
                : undefined,
            },
          })}
          locale={{
            emptyText: (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={t("dynamicUIManager.noSchemas")}
              />
            ),
          }}
        />
      </Spin>

      {/* 表格下方预览区 */}
      {selectedSchema && previewSchema && (
        <div className="mt-4 border rounded-lg p-4 bg-white dark:bg-gray-900">
          <div className="mb-3">
            <Text strong className="text-base">{selectedSchema.title}</Text>
            <Space className="ml-3">
              <Tag color="blue">{selectedSchema.category}</Tag>
              {selectedSchema.tags.map((tag) => <Tag key={tag}>{tag}</Tag>)}
            </Space>
          </div>
          <DynamicUIRenderer schema={previewSchema} />
        </div>
      )}

      {/* 统一的创建/编辑弹窗 */}
      <Modal
        title={isCreating
          ? t("settings.dynamicPages.createNew")
          : `${t("settings.dynamicPages.editPage")} - ${editingRecord?.title ?? ""}`}
        open={editOpen}
        onCancel={() => setEditOpen(false)}
        width={1000}
        footer={
          <Space>
            <Button onClick={() => setEditOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button
              type="primary"
              icon={<SaveOutlined />}
              onClick={handleSaveEdit}
              loading={editSaving}
            >
              {t("common.save")}
            </Button>
          </Space>
        }
      >
        {/* 创建模式下显示标题、标签、固定到导航 */}
        {isCreating && (
          <div className="flex flex-col gap-3 mt-2 mb-4">
            <div className="flex gap-4">
              <div className="flex-1">
                <Text type="secondary" className="block mb-1 text-xs">
                  {t("dynamicUIManager.schemaTitle")}
                </Text>
                <Input
                  placeholder={t("dynamicUIManager.titlePlaceholder")}
                  value={editTitle}
                  onChange={(e) => setEditTitle(e.target.value)}
                />
              </div>
              <div className="flex-1">
                <Text type="secondary" className="block mb-1 text-xs">
                  {t("dynamicUIManager.tags")}
                </Text>
                <Select
                  mode="tags"
                  className="w-full"
                  placeholder={t("dynamicUIManager.tagsPlaceholder")}
                  value={editTags}
                  onChange={(v) => setEditTags(v)}
                />
              </div>
            </div>
            <div className="border-t pt-3">
              <div className="flex items-center gap-6">
                <Space>
                  <Switch checked={pinToNav} onChange={setPinToNav} />
                  <Text className="text-sm">{t("settings.dynamicPages.pinAfterCreate")}</Text>
                </Space>
                <Space>
                  <Text type="secondary" className="text-sm">{t("settings.dynamicPages.group")}:</Text>
                  <Select
                    size="small"
                    value={pinGroup}
                    onChange={setPinGroup}
                    style={{ width: 140 }}
                    options={PIN_GROUPS.map((g) => ({ label: t(g.labelKey), value: g.key }))}
                  />
                </Space>
              </div>
            </div>
            <Divider className="my-0" />
          </div>
        )}

        <Tabs
          activeKey={editMode}
          onChange={(k) => setEditMode(k as "visual" | "ai" | "json")}
          items={[
            {
              key: "visual",
              label: t("settings.dynamicPages.visualEdit"),
              children: editSchema
                ? (
                  <SchemaTreeEditor
                    schema={editSchema}
                    onChange={(newSchema) => {
                      setEditSchema(newSchema);
                      setEditJsonText(JSON.stringify(newSchema, null, 2));
                    }}
                  />
                )
                : (
                  <Empty
                    description={isCreating
                      ? t("dynamicUIManager.nlInputPlaceholder")
                      : t("dynamicUIManager.selectToPreview")}
                  />
                ),
            },
            {
              key: "ai",
              label: t("settings.dynamicPages.aiEdit"),
              children: (
                <div className="space-y-4 mt-2">
                  {editSchema && (
                    <div className="bg-gray-50 dark:bg-gray-800 rounded p-3">
                      <div className="text-xs text-gray-500 mb-1">
                        {t("settings.dynamicPages.currentSchema")}
                      </div>
                      <div className="flex items-center gap-2 text-sm">
                        <Tag color="geekblue">{editSchema.type}</Tag>
                        <span className="text-xs text-gray-500">
                          {editSchema.children?.length ?? 0} {t("settings.dynamicPages.childrenCount")}
                        </span>
                      </div>
                    </div>
                  )}

                  <div>
                    <div className="text-xs text-gray-500 mb-1">
                      {isCreating
                        ? t("settings.dynamicPages.aiGenerateInstruction")
                        : t("settings.dynamicPages.editInstruction")}
                    </div>
                    <Input.TextArea
                      rows={4}
                      placeholder={isCreating
                        ? t("dynamicUIManager.nlInputPlaceholder")
                        : t("settings.dynamicPages.aiEditPlaceholder")}
                      value={editNlPrompt}
                      onChange={(e) => setEditNlPrompt(e.target.value)}
                    />
                    <Button
                      type="primary"
                      icon={<Bot size={16} />}
                      onClick={handleAIEdit}
                      loading={editGenerating}
                      className="mt-3"
                    >
                      {editGenerating
                        ? t("dynamicUIManager.generating")
                        : isCreating
                        ? t("settings.dynamicPages.aiGenerate")
                        : t("settings.dynamicPages.aiEdit")}
                    </Button>
                  </div>

                  {editSchema && (
                    <div>
                      <Divider className="my-2" />
                      <div className="text-xs text-gray-500 mb-1">
                        {t("settings.dynamicPages.schemaPreview")}
                      </div>
                      <Input.TextArea
                        rows={12}
                        value={editJsonText}
                        readOnly
                        className="font-mono text-xs"
                      />
                    </div>
                  )}
                </div>
              ),
            },
            {
              key: "json",
              label: t("settings.dynamicPages.jsonEdit"),
              children: (
                <div className="space-y-2 mt-2">
                  <Input.TextArea
                    rows={15}
                    value={editJsonText}
                    onChange={(e) => {
                      setEditJsonText(e.target.value);
                      try {
                        const parsed = JSON.parse(e.target.value) as UISchema;
                        setEditSchema(parsed);
                        setEditParseError(null);
                      } catch {
                        setEditParseError(t("dynamicUIManager.parseError"));
                      }
                    }}
                    className="font-mono text-sm"
                  />
                  {derivedEditParseError
                    ? (
                      <Alert
                        type="error"
                        message={t("dynamicUIManager.parseError")}
                        description={derivedEditParseError}
                      />
                    )
                    : editSchema
                    ? (
                      <Alert
                        type="success"
                        message={t("dynamicUIManager.schemaValid")}
                      />
                    )
                    : null}
                </div>
              ),
            },
          ]}
        />
      </Modal>
    </div>
  );
}
