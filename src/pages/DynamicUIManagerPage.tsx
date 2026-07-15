// SPDX-License-Identifier: AGPL-3.0-only

import { DynamicUIRenderer } from "@/components/dynamicUI/DynamicUIRenderer";
import { SchemaIdContext } from "@/components/dynamicUI/SchemaIdContext";
import { VisualEditor } from "@/components/dynamicUI/VisualEditor";
import { generateUIFromNLBackend } from "@/lib/dynamicUI/nl2ui-edit";
import { validateSchema } from "@/lib/dynamicUI/SchemaValidator";
import { useDynamicUIStore } from "@/stores";
import type {
  CreateDynamicUISchemaParams,
  DynamicUISchemaRecord,
  DynamicUISchemaVersion,
  UISchema,
  UpdateDynamicUISchemaParams,
} from "@/types";
import {
  AppstoreAddOutlined,
  DeleteOutlined,
  EditOutlined,
  ExportOutlined,
  EyeOutlined,
  HistoryOutlined,
  ImportOutlined,
  PlusOutlined,
  RobotOutlined,
  RollbackOutlined,
  SaveOutlined,
  TagOutlined,
} from "@ant-design/icons";
import {
  Alert,
  Button,
  Card,
  Divider,
  Empty,
  Form,
  Input,
  message,
  Modal,
  Popconfirm,
  Select,
  Space,
  Spin,
  Table,
  Tabs,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import { type ChangeEvent, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";

const { Title, Paragraph, Text } = Typography;
const { TextArea } = Input;

const CATEGORIES = ["form", "dashboard", "report", "custom"];

export function DynamicUIManagerPage() {
  const { t } = useTranslation();
  const {
    schemas,
    loading,
    fetchSchemas,
    createSchema,
    updateSchema,
    deleteSchema,
    versionList,
    versionLoading,
    loadVersions,
    restoreVersion,
  } = useDynamicUIStore();

  const [selectedSchema, setSelectedSchema] = useState<DynamicUISchemaRecord | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingRecord, setEditingRecord] = useState<DynamicUISchemaRecord | null>(null);
  const [editorMode, setEditorMode] = useState<"visual" | "json">("json");
  const [form] = Form.useForm();
  const [jsonSchemaText, setJsonSchemaText] = useState("");
  const [parseError, setParseError] = useState<string | null>(null);
  const [nlPrompt, setNlPrompt] = useState("");
  const [generating, setGenerating] = useState(false);

  // 版本历史面板
  const [versionPanelOpen, setVersionPanelOpen] = useState(false);
  const [versionPreview, setVersionPreview] = useState<DynamicUISchemaVersion | null>(null);

  useEffect(() => {
    void fetchSchemas();
  }, [fetchSchemas]);

  // 支持 ?schema=<标题> URL 查询参数自动选中
  const [searchParams] = useSearchParams();
  const schemaParam = searchParams.get("schema");
  const autoSelectedRef = useRef(false);

  useEffect(() => {
    if (autoSelectedRef.current || !schemaParam || schemas.length === 0) { return; }
    const match = schemas.find((s) => s.title === schemaParam);
    if (match) {
      setTimeout(() => setSelectedSchema(match), 0);
      autoSelectedRef.current = true;
    }
  }, [schemaParam, schemas, setSelectedSchema]);

  useEffect(() => {
    // 仅在编辑器打开时操作 form，避免 Modal 未挂载时调用 form 方法触发警告
    if (!editorOpen) {
      return;
    }
    if (editingRecord) {
      form.setFieldsValue({
        title: editingRecord.title,
        description: editingRecord.description,
        category: editingRecord.category,
        tags: editingRecord.tags,
        version: "",
        change_log: "",
      });
      setTimeout(() => setJsonSchemaText(editingRecord.schema_json), 0);
    } else {
      form.resetFields();
      form.setFieldsValue({ category: "custom", version: "", change_log: "" });
      setTimeout(() => setJsonSchemaText(""), 0);
    }
  }, [editingRecord, editorOpen]);
  // eslint-disable-next-line react-hooks/exhaustive-deps — form/setJsonSchemaText are stable references

  const derivedParseError = useMemo(() => {
    if (!jsonSchemaText) { return null; }
    try {
      const parsed = JSON.parse(jsonSchemaText);
      const result = validateSchema(parsed);
      if (!result.valid) {
        return result.errors.map((e) => `${e.path}: ${e.message}`).join("\n");
      }
      return null;
    } catch (err: unknown) {
      return err instanceof Error ? err.message : String(err);
    }
  }, [jsonSchemaText]);

  const parsedPreview = useMemo(() => {
    if (!jsonSchemaText || derivedParseError) { return null; }
    try {
      return JSON.parse(jsonSchemaText) as UISchema;
    } catch {
      return null;
    }
  }, [jsonSchemaText, derivedParseError]);

  const handleCreate = () => {
    setEditingRecord(null);
    setEditorMode("json");
    setEditorOpen(true);
  };

  const handleEdit = (record: DynamicUISchemaRecord) => {
    setEditingRecord(record);
    setEditorMode("json");
    setEditorOpen(true);
  };

  const handlePreview = (record: DynamicUISchemaRecord) => {
    setSelectedSchema(record);
  };

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      if (!jsonSchemaText.trim()) {
        setParseError(t("dynamicUIManager.schemaRequired"));
        return;
      }
      let parsed: UISchema;
      try {
        parsed = JSON.parse(jsonSchemaText) as UISchema;
      } catch {
        setParseError(t("dynamicUIManager.invalidJson"));
        return;
      }
      const validation = validateSchema(parsed);
      if (!validation.valid) {
        setParseError(validation.errors.map((e) => `${e.path}: ${e.message}`).join("\n"));
        return;
      }

      const updateParams: UpdateDynamicUISchemaParams = {
        title: values.title,
        description: values.description,
        category: values.category,
        tags: values.tags,
        schema_json: jsonSchemaText,
        version: values.version?.trim() || undefined,
        change_log: values.change_log?.trim() || undefined,
      };

      if (editingRecord) {
        await updateSchema(editingRecord.id, updateParams);
        message.success(t("dynamicUIManager.updateSuccess"));
      } else {
        // 注意：后端 CreateSchemaRequest 不支持 version/change_log，
        // 创建时这两个字段会被静默忽略（如需在创建时指定版本号需后端扩展）
        const createParams: CreateDynamicUISchemaParams = {
          title: values.title,
          description: values.description,
          category: values.category,
          tags: values.tags || [],
          schema_json: jsonSchemaText,
        };
        await createSchema(createParams);
        message.success(t("dynamicUIManager.createSuccess"));
      }
      setEditorOpen(false);
      setEditingRecord(null);
    } catch {
      // form validation error
    }
  };

  const handleGenerateFromNL = async () => {
    if (!nlPrompt.trim()) {
      return;
    }
    setGenerating(true);
    try {
      const result = await generateUIFromNLBackend(nlPrompt);
      setJsonSchemaText(JSON.stringify(result.schema, null, 2));
      form.setFieldsValue({
        title: form.getFieldValue("title") || result.title,
        description: form.getFieldValue("description") || result.description,
      });
      setParseError(null);
      message.success(t("dynamicUIManager.generateSuccess"));
    } catch (err: unknown) {
      message.error(err instanceof Error ? err.message : String(err));
    } finally {
      setGenerating(false);
    }
  };

  const handleDelete = async (id: string) => {
    await deleteSchema(id);
    if (selectedSchema?.id === id) {
      setSelectedSchema(null);
    }
    message.success(t("dynamicUIManager.deleteSuccess"));
  };

  // ── 导入 / 导出（额外短板） ──

  const fileInputRef = useRef<HTMLInputElement>(null);

  const triggerImport = () => fileInputRef.current?.click();

  const downloadJson = (filename: string, data: unknown) => {
    const blob = new Blob([JSON.stringify(data, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleExportSchema = (record: DynamicUISchemaRecord) => {
    downloadJson(`${record.title || "schema"}.json`, {
      title: record.title,
      description: record.description,
      category: record.category,
      tags: record.tags,
      schema_json: record.schema_json,
    });
    message.success(t("dynamicUIManager.exportSuccess"));
  };

  const handleExportAll = () => {
    if (schemas.length === 0) {
      message.warning(t("dynamicUIManager.noSchemas"));
      return;
    }
    downloadJson(
      "dynamic-ui-schemas.json",
      schemas.map((s) => ({
        title: s.title,
        description: s.description,
        category: s.category,
        tags: s.tags,
        schema_json: s.schema_json,
      })),
    );
    message.success(t("dynamicUIManager.exportSuccess"));
  };

  const handleImportFile = async (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file) {
      return;
    }
    try {
      const text = await file.text();
      const parsed = JSON.parse(text);
      const list = Array.isArray(parsed) ? parsed : [parsed];
      let count = 0;
      for (const item of list) {
        const schemaJson = typeof item === "string" ? item : item?.schema_json;
        if (!schemaJson || typeof schemaJson !== "string") {
          continue;
        }
        let schemaObj: UISchema;
        try {
          schemaObj = JSON.parse(schemaJson) as UISchema;
        } catch {
          continue;
        }
        const validation = validateSchema(schemaObj);
        if (!validation.valid) {
          continue;
        }
        const fallbackTitle = (schemaObj.props?.content as string) || "Imported UI";
        await createSchema({
          title: item?.title || fallbackTitle,
          description: item?.description || "",
          category: item?.category || "custom",
          tags: item?.tags || [],
          schema_json: schemaJson,
        });
        count += 1;
      }
      if (count > 0) {
        message.success(t("dynamicUIManager.importSuccess", { count }));
        await fetchSchemas();
      } else {
        message.warning(t("dynamicUIManager.importInvalid"));
      }
    } catch {
      message.error(t("dynamicUIManager.importInvalid"));
    }
  };

  // ── 版本管理 ──

  const handleOpenVersionPanel = async (record: DynamicUISchemaRecord) => {
    setSelectedSchema(record);
    setVersionPreview(null);
    setVersionPanelOpen(true);
    await loadVersions(record.id);
  };

  const handlePreviewVersion = async (version: DynamicUISchemaVersion) => {
    setVersionPreview(version);
  };

  const handleRestoreVersion = async (version: DynamicUISchemaVersion) => {
    if (!selectedSchema) { return; }
    const result = await restoreVersion(selectedSchema.id, version.id);
    if (result) {
      message.success(t("dynamicUIManager.restoreSuccess"));
      // 刷新版本列表
      await loadVersions(selectedSchema.id);
      setVersionPreview(null);
      // 恢复后同步刷新预览（缺陷 5）
      setSelectedSchema(result);
    } else {
      message.error(t("dynamicUIManager.restoreFailed"));
    }
  };

  const renderPreview = () => {
    // 如果正在查看版本历史中的某个版本
    const schemaToRender = versionPreview
      ? {
        title: versionPreview.title,
        description: versionPreview.description,
        schema_json: versionPreview.schema_json,
        category: versionPreview.category,
        tags: versionPreview.tags,
      }
      : selectedSchema;

    if (!schemaToRender) {
      return (
        <Empty
          description={t("dynamicUIManager.selectToPreview")}
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      );
    }
    try {
      const schema = JSON.parse(schemaToRender.schema_json) as UISchema;
      const isHistorical = !!versionPreview;
      return (
        <div className="p-4">
          <div className="mb-4">
            <div className="flex items-center gap-2">
              <Title level={4} style={{ margin: 0 }}>
                {schemaToRender.title}
              </Title>
              {isHistorical && (
                <Tag color="orange" icon={<HistoryOutlined />}>
                  v{versionPreview!.version} ({t("dynamicUIManager.versionHistory")})
                </Tag>
              )}
            </div>
            <Space wrap className="mt-2">
              <Tag color="blue">{schemaToRender.category}</Tag>
              {schemaToRender.tags.map((tag) => <Tag key={tag}>{tag}</Tag>)}
              {!isHistorical && selectedSchema
                ? (
                  <Tag color="green" icon={<TagOutlined />}>
                    {t("dynamicUIManager.version")}: {selectedSchema.version}
                  </Tag>
                )
                : null}
            </Space>
            {schemaToRender.description
              ? <Paragraph type="secondary">{schemaToRender.description}</Paragraph>
              : null}
          </div>
          <div className="border rounded-lg p-4 bg-white dark:bg-gray-900">
            <SchemaIdContext.Provider
              value={{ schemaId: isHistorical ? null : (selectedSchema?.id ?? null) }}
            >
              <DynamicUIRenderer schema={schema} />
            </SchemaIdContext.Provider>
          </div>
        </div>
      );
    } catch {
      return <Alert type="error" title={t("dynamicUIManager.invalidSchema")} />;
    }
  };

  const versionColumns = [
    {
      title: t("dynamicUIManager.version"),
      dataIndex: "version",
      key: "version",
      width: 100,
      render: (v: string, _record: DynamicUISchemaVersion) => (
        <Tag color={selectedSchema?.version === v ? "green" : "default"}>
          v{v}
          {selectedSchema?.version === v ? ` (${t("dynamicUIManager.currentVersion")})` : ""}
        </Tag>
      ),
    },
    {
      title: t("dynamicUIManager.changeLog"),
      dataIndex: "change_log",
      key: "change_log",
      ellipsis: true,
    },
    {
      title: t("dynamicUIManager.versionUpdatedAt"),
      dataIndex: "created_at",
      key: "created_at",
      width: 160,
      render: (ts: number) => new Date(ts * 1000).toLocaleString(),
    },
    {
      title: t("common.actions"),
      key: "action",
      width: 160,
      render: (_: unknown, record: DynamicUISchemaVersion) => (
        <Space>
          <Button
            type="link"
            size="small"
            icon={<EyeOutlined />}
            onClick={() => handlePreviewVersion(record)}
          >
            {t("dynamicUIManager.preview")}
          </Button>
          {selectedSchema?.version !== record.version
            ? (
              <Popconfirm
                title={t("dynamicUIManager.confirmRestore")}
                onConfirm={() => handleRestoreVersion(record)}
                okText={t("common.confirm")}
                cancelText={t("common.cancel")}
              >
                <Button
                  type="link"
                  size="small"
                  icon={<RollbackOutlined />}
                  danger
                >
                  {t("dynamicUIManager.restore")}
                </Button>
              </Popconfirm>
            )
            : null}
        </Space>
      ),
    },
  ];

  return (
    <div className="h-full flex flex-col p-4">
      <div className="flex items-center justify-between mb-4">
        <div>
          <Title level={3} style={{ margin: 0 }}>
            <AppstoreAddOutlined className="mr-2" />
            {t("dynamicUIManager.title")}
          </Title>
          <Paragraph type="secondary" style={{ margin: 0 }}>
            {t("dynamicUIManager.subtitle")}
          </Paragraph>
        </div>
        <Button type="primary" icon={<PlusOutlined />} onClick={handleCreate}>
          {t("dynamicUIManager.createNew")}
        </Button>
      </div>

      <div className="flex-1 grid grid-cols-1 lg:grid-cols-2 gap-4 min-h-0">
        <Card
          className="flex flex-col min-h-0"
          title={t("dynamicUIManager.schemaList")}
          extra={
            <Space size={4}>
              <Button
                size="small"
                icon={<ImportOutlined />}
                onClick={triggerImport}
              >
                {t("dynamicUIManager.import")}
              </Button>
              <Button
                size="small"
                icon={<ExportOutlined />}
                onClick={handleExportAll}
              >
                {t("dynamicUIManager.exportAll")}
              </Button>
            </Space>
          }
          styles={{ body: { flex: 1, overflow: "auto" } }}
        >
          <Spin spinning={loading}>
            {schemas.length === 0
              ? <Empty description={t("dynamicUIManager.noSchemas")} />
              : (
                <div className="divide-y divide-gray-100">
                  {schemas.map((item) => (
                    <div
                      key={item.id}
                      className="flex items-start justify-between gap-3 py-3"
                    >
                      <div className="min-w-0 flex-1">
                        <Space>
                          <Text strong>{item.title}</Text>
                          {item.is_builtin
                            ? <Tag color="purple">{t("dynamicUIManager.builtin")}</Tag>
                            : null}
                          <Tag color="green" style={{ fontSize: 11 }}>
                            v{item.version}
                          </Tag>
                        </Space>
                        <div>
                          <Text type="secondary" className="block truncate">
                            {item.description || t("dynamicUIManager.noDescription")}
                          </Text>
                          <Space size={4} className="mt-1">
                            <Tag color="blue">{item.category}</Tag>
                            <Text type="secondary" className="text-xs">
                              {new Date(item.updated_at).toLocaleString()}
                            </Text>
                          </Space>
                        </div>
                      </div>
                      <Space size={2}>
                        <Tooltip title={t("dynamicUIManager.versionHistory")}>
                          <Button
                            type="text"
                            icon={<HistoryOutlined />}
                            onClick={() => handleOpenVersionPanel(item)}
                          />
                        </Tooltip>
                        <Button
                          type="text"
                          icon={<EyeOutlined />}
                          onClick={() => handlePreview(item)}
                        />
                        <Tooltip title={t("dynamicUIManager.export")}>
                          <Button
                            type="text"
                            icon={<ExportOutlined />}
                            onClick={() => handleExportSchema(item)}
                          />
                        </Tooltip>
                        <Button
                          type="text"
                          icon={<EditOutlined />}
                          onClick={() => handleEdit(item)}
                          disabled={item.is_builtin}
                        />
                        <Popconfirm
                          title={t("dynamicUIManager.confirmDelete")}
                          onConfirm={() => handleDelete(item.id)}
                          okText={t("common.confirm")}
                          cancelText={t("common.cancel")}
                        >
                          <Button
                            type="text"
                            danger
                            icon={<DeleteOutlined />}
                            disabled={item.is_builtin}
                          />
                        </Popconfirm>
                      </Space>
                    </div>
                  ))}
                </div>
              )}
          </Spin>
        </Card>

        <Card
          className="flex flex-col min-h-0"
          title={
            <Space>
              <span>{t("dynamicUIManager.preview")}</span>
              {selectedSchema && !versionPreview
                ? <Tag color="green">v{selectedSchema.version}</Tag>
                : null}
            </Space>
          }
          styles={{ body: { flex: 1, overflow: "auto", padding: 0 } }}
        >
          {renderPreview()}
        </Card>
      </div>

      {/* 编辑弹窗 */}
      <Modal
        title={editingRecord
          ? t("dynamicUIManager.editSchema")
          : t("dynamicUIManager.createSchema")}
        open={editorOpen}
        onCancel={() => {
          setEditorOpen(false);
          setEditingRecord(null);
        }}
        width={800}
        footer={
          <Space>
            <Button
              onClick={() => {
                setEditorOpen(false);
                setEditingRecord(null);
              }}
            >
              {t("common.cancel")}
            </Button>
            <Button type="primary" icon={<SaveOutlined />} onClick={handleSave}>
              {t("common.save")}
            </Button>
          </Space>
        }
      >
        <Form form={form} layout="vertical" className="mt-4">
          <Form.Item
            name="title"
            label={t("dynamicUIManager.schemaTitle")}
            rules={[{ required: true, message: t("dynamicUIManager.titleRequired") }]}
          >
            <Input placeholder={t("dynamicUIManager.titlePlaceholder")} />
          </Form.Item>
          <Form.Item name="description" label={t("dynamicUIManager.description")}>
            <TextArea rows={2} placeholder={t("dynamicUIManager.descPlaceholder")} />
          </Form.Item>
          <Form.Item name="category" label={t("dynamicUIManager.category")}>
            <Select
              options={CATEGORIES.map((c) => ({
                label: t(`dynamicUIManager.cat${c.charAt(0).toUpperCase() + c.slice(1)}`),
                value: c,
              }))}
            />
          </Form.Item>
          <Form.Item name="tags" label={t("dynamicUIManager.tags")}>
            <Select mode="tags" placeholder={t("dynamicUIManager.tagsPlaceholder")} />
          </Form.Item>

          {/* 版本管理和变更说明（仅编辑时显示） */}
          {editingRecord
            ? (
              <>
                <Divider plain>
                  <Space size={4}>
                    <TagOutlined />
                    <span>{t("dynamicUIManager.version")}</span>
                  </Space>
                </Divider>
                <Space className="w-full" style={{ alignItems: "flex-start" }}>
                  <Form.Item
                    name="version"
                    label={t("dynamicUIManager.version")}
                    tooltip={t("dynamicUIManager.versionAutoHint")}
                    style={{ flex: 1 }}
                  >
                    <Input
                      placeholder={t("dynamicUIManager.versionAutoHint")}
                    />
                  </Form.Item>
                  <Form.Item
                    name="change_log"
                    label={t("dynamicUIManager.changeLog")}
                    style={{ flex: 2 }}
                  >
                    <Input placeholder={t("dynamicUIManager.changeLogPlaceholder")} />
                  </Form.Item>
                </Space>
                <Text type="secondary" className="text-xs">
                  {t("dynamicUIManager.currentVersion")}: {editingRecord.version}
                  {" | "}
                  {t("dynamicUIManager.versionAutoHint")}
                </Text>
                <Divider />
              </>
            )
            : null}

          {!editingRecord
            ? (
              <>
                <Divider plain>
                  <Space size={4}>
                    <RobotOutlined />
                    <span>{t("dynamicUIManager.generateWithNL")}</span>
                  </Space>
                </Divider>
                <Space.Compact style={{ width: "100%" }}>
                  <Input
                    placeholder={t("dynamicUIManager.nlInputPlaceholder")}
                    value={nlPrompt}
                    onChange={(e) => setNlPrompt(e.target.value)}
                    onPressEnter={handleGenerateFromNL}
                    disabled={generating}
                  />
                  <Button
                    type="primary"
                    icon={<RobotOutlined />}
                    onClick={handleGenerateFromNL}
                    loading={generating}
                  >
                    {generating ? t("dynamicUIManager.generating") : t("dynamicUIManager.generateWithNL")}
                  </Button>
                </Space.Compact>
                <div className="h-3" />
              </>
            )
            : null}

          <div className="mb-2">
            <Tabs
              activeKey={editorMode}
              onChange={(k) => setEditorMode(k as "visual" | "json")}
              size="small"
              items={[
                { key: "visual", label: t("dynamicUIManager.visualEditor") },
                { key: "json", label: t("dynamicUIManager.jsonEditor") },
              ]}
            />
          </div>

          {editorMode === "visual"
            ? (
              <div className="min-h-75">
                <VisualEditor
                  key={editingRecord?.id ?? "new"}
                  schema={parsedPreview}
                  onChange={(newSchema) => setJsonSchemaText(JSON.stringify(newSchema, null, 2))}
                />
              </div>
            )
            : null}

          {editorMode === "json"
            ? (
              <>
                <TextArea
                  rows={12}
                  value={jsonSchemaText}
                  onChange={(e) => setJsonSchemaText(e.target.value)}
                  placeholder={t("dynamicUIManager.jsonPlaceholder")}
                  className="font-mono text-sm"
                />
                {parseError
                  ? (
                    <Alert
                      type="error"
                      className="mt-2"
                      title={t("dynamicUIManager.parseError")}
                      description={<pre className="whitespace-pre-wrap m-0">{parseError}</pre>}
                    />
                  )
                  : parsedPreview
                  ? <Alert type="success" className="mt-2" title={t("dynamicUIManager.schemaValid")} />
                  : null}
              </>
            )
            : null}
        </Form>
      </Modal>

      {/* 版本历史弹窗 */}
      <Modal
        title={
          <Space>
            <HistoryOutlined />
            <span>{t("dynamicUIManager.versionHistory")}</span>
            {selectedSchema
              ? <Tag color="green">v{selectedSchema.version}</Tag>
              : null}
          </Space>
        }
        open={versionPanelOpen}
        onCancel={() => {
          setVersionPanelOpen(false);
          setVersionPreview(null);
        }}
        width={900}
        footer={null}
      >
        <Spin spinning={versionLoading}>
          {versionList.length === 0
            ? (
              <Empty
                description={t("dynamicUIManager.noVersions")}
                image={Empty.PRESENTED_IMAGE_SIMPLE}
              />
            )
            : (
              <Table
                dataSource={versionList}
                columns={versionColumns}
                rowKey="id"
                size="small"
                pagination={false}
                scroll={{ y: 400 }}
              />
            )}
        </Spin>
      </Modal>

      {/* 导入文件选择器（隐藏） */}
      <input
        type="file"
        accept="application/json,.json"
        style={{ display: "none" }}
        ref={fileInputRef}
        onChange={handleImportFile}
      />
    </div>
  );
}
