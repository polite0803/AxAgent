import { DeleteOutlined, EditOutlined, HistoryOutlined, PlusOutlined, SearchOutlined } from "@ant-design/icons";
import { Button, Form, Input, List, message, Modal, Space, Tag, Typography } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { Title } = Typography;

interface PromptTemplate {
  id: string;
  name: string;
  description?: string;
  content: string;
  variables_schema?: string;
  version: number;
  is_active: boolean;
  ab_test_enabled: boolean;
  created_at: number;
  updated_at: number;
}

const mockTemplates: PromptTemplate[] = [
  {
    id: "1",
    name: "Summarization Prompt",
    description: "Used for summarizing long documents",
    content: "Summarize the following text in {style} style:\n\n{text}",
    variables_schema: '{"style": "string", "text": "string"}',
    version: 2,
    is_active: true,
    ab_test_enabled: false,
    created_at: Date.now() - 86400000 * 7,
    updated_at: Date.now() - 86400000 * 2,
  },
  {
    id: "2",
    name: "Code Review Prompt",
    description: "For AI-assisted code reviews",
    content: "Review the following {language} code:\n\n```{language}\n{code}\n```\n\nFocus on: {focus_areas}",
    version: 1,
    is_active: true,
    ab_test_enabled: true,
    created_at: Date.now() - 86400000 * 14,
    updated_at: Date.now() - 86400000 * 14,
  },
];

export function PromptTemplatesPage() {
  const { t } = useTranslation();
  const [templates] = useState<PromptTemplate[]>(mockTemplates);
  const [searchText, setSearchText] = useState("");
  const [isEditorOpen, setIsEditorOpen] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<PromptTemplate | null>(null);
  const [isVersionHistoryOpen, setIsVersionHistoryOpen] = useState(false);
  const [versionHistoryTemplate, setVersionHistoryTemplate] = useState<PromptTemplate | null>(null);
  const [form] = Form.useForm();

  const filteredTemplates = templates.filter(
    (t) =>
      t.name.toLowerCase().includes(searchText.toLowerCase())
      || t.description?.toLowerCase().includes(searchText.toLowerCase()),
  );

  const handleCreate = () => {
    setEditingTemplate(null);
    form.resetFields();
    setIsEditorOpen(true);
  };

  const handleEdit = (template: PromptTemplate) => {
    setEditingTemplate(template);
    form.setFieldsValue(template);
    setIsEditorOpen(true);
  };

  const handleSave = () => {
    form.validateFields().then((values) => {
      console.log("Saving template:", values);
      message.success(editingTemplate ? t("promptTemplates.templateUpdated") : t("promptTemplates.templateCreated"));
      setIsEditorOpen(false);
    });
  };

  const handleDelete = (template: PromptTemplate) => {
    Modal.confirm({
      title: t("promptTemplates.deleteTemplate"),
      content: t("promptTemplates.deleteTemplateConfirm", { name: template.name }),
      okText: t("common.delete"),
      okType: "danger",
      onOk: () => {
        message.success(t("promptTemplates.templateDeleted"));
      },
    });
  };

  const handleViewHistory = (template: PromptTemplate) => {
    setVersionHistoryTemplate(template);
    setIsVersionHistoryOpen(true);
  };

  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-6">
        <Title level={4} className="m-0">
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
                  {template.is_active && <Tag color="green">{t("common.active")}</Tag>}
                  {template.ab_test_enabled && <Tag color="blue">{t("promptTemplates.abTest")}</Tag>}
                  <Tag>v{template.version}</Tag>
                </Space>
              }
              description={template.description || template.content.slice(0, 100) + "..."}
            />
          </List.Item>
        )}
      />

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
            <Input.TextArea
              placeholder={t("promptTemplates.contentPlaceholder")}
              rows={6}
            />
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
            dataSource={[
              { id: "v2", version: 2, changelog: "Added style parameter", created_at: Date.now() - 86400000 * 2 },
              { id: "v1", version: 1, changelog: "Initial version", created_at: Date.now() - 86400000 * 7 },
            ]}
            renderItem={(item) => (
              <List.Item
                actions={[
                  <Button key="view" size="small" onClick={() => handleEdit(versionHistoryTemplate!)}>
                    {t("common.view")}
                  </Button>,
                  <Button
                    key="rollback"
                    size="small"
                    onClick={() => message.info(t("promptTemplates.rollbackNotImplemented"))}
                  >
                    {t("promptTemplates.rollback")}
                  </Button>,
                ]}
              >
                <List.Item.Meta
                  title={
                    <Space>
                      <Tag>{t("promptTemplates.version")} {item.version}</Tag>
                      <span className="text-gray-500 text-sm">
                        {new Date(item.created_at).toLocaleDateString()}
                      </span>
                    </Space>
                  }
                  description={item.changelog}
                />
              </List.Item>
            )}
          />
        </div>
      </Modal>
    </div>
  );
}
