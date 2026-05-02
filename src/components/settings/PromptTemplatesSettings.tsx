import { usePromptTemplateStore } from "@/stores/feature/promptTemplateStore";
import type { CreatePromptTemplateInput, PromptTemplate, UpdatePromptTemplateInput } from "@/types";
import {
  Button,
  Empty,
  Form,
  Input,
  List,
  message,
  Modal,
  Space,
  Spin,
  Tag,
  Typography,
} from "antd";
import {
  DeleteOutlined,
  EditOutlined,
  HistoryOutlined,
  PlusOutlined,
  SearchOutlined,
} from "@ant-design/icons";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const { Title } = Typography;

export function PromptTemplatesSettings() {
  const { t } = useTranslation();
  const [messageApi, contextHolder] = message.useMessage();
  const [isEditorOpen, setIsEditorOpen] = useState(false);
  const [isVersionHistoryOpen, setIsVersionHistoryOpen] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<PromptTemplate | null>(null);
  const [versionHistoryTemplate, setVersionHistoryTemplate] = useState<PromptTemplate | null>(null);
  const [searchText, setSearchText] = useState("");
  const [form] = Form.useForm();

  const { templates, versions, loading, loadTemplates, createTemplate, updateTemplate, deleteTemplate, loadVersions } =
    usePromptTemplateStore();

  useEffect(() => {
    loadTemplates();
  }, [loadTemplates]);

  const filteredTemplates = templates.filter(
    (template) =>
      template.name.toLowerCase().includes(searchText.toLowerCase()) ||
      (template.description?.toLowerCase().includes(searchText.toLowerCase()) ?? false)
  );

  const handleCreate = useCallback(() => {
    setEditingTemplate(null);
    form.resetFields();
    setIsEditorOpen(true);
  }, [form]);

  const handleEdit = useCallback(
    (template: PromptTemplate) => {
      setEditingTemplate(template);
      form.setFieldsValue({
        name: template.name,
        description: template.description,
        content: template.content,
        variablesSchema: template.variablesSchema,
      });
      setIsEditorOpen(true);
    },
    [form]
  );

  const handleSave = useCallback(async () => {
    try {
      const values = await form.validateFields();
      const input: CreatePromptTemplateInput = {
        name: values.name,
        description: values.description,
        content: values.content,
        variablesSchema: values.variablesSchema,
      };

      if (editingTemplate) {
        const updateInput: UpdatePromptTemplateInput = {
          name: values.name,
          description: values.description,
          content: values.content,
          variablesSchema: values.variablesSchema,
        };
        await updateTemplate(editingTemplate.id, updateInput);
        messageApi.success(t("common.saveSuccess"));
      } else {
        await createTemplate(input);
        messageApi.success(t("common.createSuccess"));
      }
      setIsEditorOpen(false);
      form.resetFields();
    } catch (e) {
      messageApi.error(String(e));
    }
  }, [editingTemplate, form, createTemplate, updateTemplate, messageApi, t]);

  const handleDelete = useCallback(
    async (template: PromptTemplate) => {
      try {
        await deleteTemplate(template.id);
        messageApi.success(t("common.deleteSuccess"));
      } catch (e) {
        messageApi.error(String(e));
      }
    },
    [deleteTemplate, messageApi, t]
  );

  const handleViewHistory = useCallback(
    async (template: PromptTemplate) => {
      setVersionHistoryTemplate(template);
      await loadVersions(template.id);
      setIsVersionHistoryOpen(true);
    },
    [loadVersions]
  );

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
        <div className="flex items-center justify-between mb-4">
          <Title level={5} className="m-0">
            {t("promptTemplates.title")}
          </Title>
          <Button type="primary" icon={<PlusOutlined />} onClick={handleCreate}>
            {t("promptTemplates.newTemplate")}
          </Button>
        </div>

        <div className="mb-4">
          <Input
            placeholder={t("promptTemplates.searchPlaceholder")}
            prefix={<SearchOutlined />}
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            className="max-w-md"
          />
        </div>

        {filteredTemplates.length === 0 ? (
          <Empty description={t("promptTemplates.noTemplates")} />
        ) : (
          <List
            dataSource={filteredTemplates}
            renderItem={(template) => (
              <List.Item
                actions={[
                  <Button
                    key="history"
                    type="text"
                    icon={<HistoryOutlined />}
                    onClick={() => handleViewHistory(template)}
                  >
                    {t("promptTemplates.history")}
                  </Button>,
                  <Button
                    key="edit"
                    type="text"
                    icon={<EditOutlined />}
                    onClick={() => handleEdit(template)}
                  >
                    {t("common.edit")}
                  </Button>,
                  <Button
                    key="delete"
                    type="text"
                    danger
                    icon={<DeleteOutlined />}
                    onClick={() => handleDelete(template)}
                  >
                    {t("common.delete")}
                  </Button>,
                ]}
              >
                <List.Item.Meta
                  title={
                    <Space>
                      <span>{template.name}</span>
                      {template.isActive && <Tag color="green">{t("common.active")}</Tag>}
                      {template.abTestEnabled && <Tag color="blue">{t("promptTemplates.abTest")}</Tag>}
                      <Tag>v{template.version}</Tag>
                    </Space>
                  }
                  description={template.description || template.content.slice(0, 100) + "..."}
                />
              </List.Item>
            )}
          />
        )}
      </div>

      <Modal
        title={editingTemplate ? t("promptTemplates.editTemplate") : t("promptTemplates.newTemplate")}
        open={isEditorOpen}
        onOk={handleSave}
        onCancel={() => setIsEditorOpen(false)}
        width={700}
      >
        <Form form={form} layout="vertical" className="mt-4">
          <Form.Item
            name="name"
            label={t("common.name")}
            rules={[{ required: true, message: t("promptTemplates.nameRequired") }]}
          >
            <Input placeholder={t("promptTemplates.namePlaceholder")} />
          </Form.Item>
          <Form.Item name="description" label={t("common.description")}>
            <Input.TextArea placeholder={t("promptTemplates.descriptionPlaceholder")} rows={2} />
          </Form.Item>
          <Form.Item
            name="content"
            label={t("promptTemplates.content")}
            rules={[{ required: true, message: t("promptTemplates.contentRequired") }]}
          >
            <Input.TextArea placeholder={t("promptTemplates.contentPlaceholder")} rows={6} />
          </Form.Item>
          <Form.Item name="variables_schema" label={t("promptTemplates.variablesSchema") + " (JSON)"}>
            <Input.TextArea placeholder='{"variable": "type"}' rows={3} />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title={t("promptTemplates.versionHistory", { name: versionHistoryTemplate?.name })}
        open={isVersionHistoryOpen}
        onCancel={() => setIsVersionHistoryOpen(false)}
        footer={null}
        width={600}
      >
        <div className="py-4">
          <List
            dataSource={versions}
            renderItem={(item) => (
              <List.Item
                actions={[
                  <Button key="view" size="small">
                    {t("common.view")}
                  </Button>,
                  <Button key="rollback" size="small">
                    {t("promptTemplates.rollback")}
                  </Button>,
                ]}
              >
                <List.Item.Meta
                  title={<Tag>{t("promptTemplates.version")} {item.version}</Tag>}
                  description={item.changelog}
                />
              </List.Item>
            )}
          />
        </div>
      </Modal>
    </SettingsGroup>
  );
}