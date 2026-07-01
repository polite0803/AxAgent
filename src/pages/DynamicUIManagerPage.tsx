// SPDX-License-Identifier: AGPL-3.0-only

import { DynamicUIRenderer } from "@/components/dynamicUI/DynamicUIRenderer";
import { generateUIFromNL } from "@/lib/dynamicUI/nl2ui";
import { validateSchema } from "@/lib/dynamicUI/SchemaValidator";
import { useDynamicUIStore } from "@/stores";
import type { DynamicUISchemaRecord, UISchema } from "@/types";
import {
  AppstoreAddOutlined,
  DeleteOutlined,
  EditOutlined,
  EyeOutlined,
  PlusOutlined,
  RobotOutlined,
  SaveOutlined,
} from "@ant-design/icons";
import {
  Alert,
  Button,
  Card,
  Divider,
  Empty,
  Form,
  Input,
  List,
  message,
  Modal,
  Popconfirm,
  Select,
  Space,
  Spin,
  Tabs,
  Tag,
  Typography,
} from "antd";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

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

  useEffect(() => {
    void fetchSchemas();
  }, [fetchSchemas]);

  /* eslint-disable react-hooks/set-state-in-effect */
  useEffect(() => {
    if (editingRecord) {
      form.setFieldsValue({
        title: editingRecord.title,
        description: editingRecord.description,
        category: editingRecord.category,
        tags: editingRecord.tags,
      });
      setJsonSchemaText(editingRecord.schema_json);
    } else {
      form.resetFields();
      form.setFieldsValue({ category: "custom" });
      setJsonSchemaText("");
    }
  }, [editingRecord, form]);
  /* eslint-enable react-hooks/set-state-in-effect */

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

      if (editingRecord) {
        await updateSchema(editingRecord.id, {
          title: values.title,
          description: values.description,
          category: values.category,
          tags: values.tags,
          schema_json: jsonSchemaText,
        });
        message.success(t("dynamicUIManager.updateSuccess"));
      } else {
        await createSchema({
          title: values.title,
          description: values.description,
          category: values.category,
          tags: values.tags || [],
          schema_json: jsonSchemaText,
        });
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
      const result = await generateUIFromNL(nlPrompt);
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

  const renderPreview = () => {
    if (!selectedSchema) {
      return (
        <Empty
          description={t("dynamicUIManager.selectToPreview")}
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      );
    }
    try {
      const schema = JSON.parse(selectedSchema.schema_json) as UISchema;
      return (
        <div className="p-4">
          <div className="mb-4">
            <Title level={4}>{selectedSchema.title}</Title>
            <Space wrap>
              <Tag color="blue">{selectedSchema.category}</Tag>
              {selectedSchema.tags.map((tag) => <Tag key={tag}>{tag}</Tag>)}
            </Space>
            {selectedSchema.description
              ? <Paragraph type="secondary">{selectedSchema.description}</Paragraph>
              : null}
          </div>
          <div className="border rounded-lg p-4 bg-white dark:bg-gray-900">
            <DynamicUIRenderer schema={schema} />
          </div>
        </div>
      );
    } catch {
      return <Alert type="error" message={t("dynamicUIManager.invalidSchema")} />;
    }
  };

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
          styles={{ body: { flex: 1, overflow: "auto" } }}
        >
          <Spin spinning={loading}>
            <List
              dataSource={schemas}
              locale={{ emptyText: <Empty description={t("dynamicUIManager.noSchemas")} /> }}
              renderItem={(item) => (
                <List.Item
                  actions={[
                    <Button
                      key="preview"
                      type="text"
                      icon={<EyeOutlined />}
                      onClick={() => handlePreview(item)}
                    />,
                    <Button
                      key="edit"
                      type="text"
                      icon={<EditOutlined />}
                      onClick={() => handleEdit(item)}
                      disabled={item.is_builtin}
                    />,
                    <Popconfirm
                      key="delete"
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
                    </Popconfirm>,
                  ]}
                >
                  <List.Item.Meta
                    title={
                      <Space>
                        <Text strong>{item.title}</Text>
                        {item.is_builtin
                          ? <Tag color="purple">{t("dynamicUIManager.builtin")}</Tag>
                          : null}
                      </Space>
                    }
                    description={
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
                    }
                  />
                </List.Item>
              )}
            />
          </Spin>
        </Card>

        <Card
          className="flex flex-col min-h-0"
          title={t("dynamicUIManager.preview")}
          styles={{ body: { flex: 1, overflow: "auto", padding: 0 } }}
        >
          {renderPreview()}
        </Card>
      </div>

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
            <Select options={CATEGORIES.map((c) => ({ label: c, value: c }))} />
          </Form.Item>
          <Form.Item name="tags" label={t("dynamicUIManager.tags")}>
            <Select mode="tags" placeholder={t("dynamicUIManager.tagsPlaceholder")} />
          </Form.Item>

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
                { key: "json", label: t("dynamicUIManager.jsonEditor") },
              ]}
            />
          </div>

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
                      message={t("dynamicUIManager.parseError")}
                      description={<pre className="whitespace-pre-wrap m-0">{parseError}</pre>}
                    />
                  )
                  : parsedPreview
                  ? <Alert type="success" className="mt-2" message={t("dynamicUIManager.schemaValid")} />
                  : null}
              </>
            )
            : null}
        </Form>
      </Modal>
    </div>
  );
}
