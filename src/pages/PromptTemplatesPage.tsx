import { List, Button, Space, Tag, Typography, Input, Modal, Form, message } from "antd";
import { PlusOutlined, SearchOutlined, EditOutlined, DeleteOutlined, HistoryOutlined } from "@ant-design/icons";
import { useState } from "react";

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
  const [templates] = useState<PromptTemplate[]>(mockTemplates);
  const [searchText, setSearchText] = useState("");
  const [isEditorOpen, setIsEditorOpen] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<PromptTemplate | null>(null);
  const [isVersionHistoryOpen, setIsVersionHistoryOpen] = useState(false);
  const [versionHistoryTemplate, setVersionHistoryTemplate] = useState<PromptTemplate | null>(null);
  const [form] = Form.useForm();

  const filteredTemplates = templates.filter(
    (t) =>
      t.name.toLowerCase().includes(searchText.toLowerCase()) ||
      t.description?.toLowerCase().includes(searchText.toLowerCase())
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
      message.success(editingTemplate ? "Template updated" : "Template created");
      setIsEditorOpen(false);
    });
  };

  const handleDelete = (template: PromptTemplate) => {
    Modal.confirm({
      title: "Delete Template",
      content: `Are you sure you want to delete "${template.name}"?`,
      okText: "Delete",
      okType: "danger",
      onOk: () => {
        message.success("Template deleted");
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
          Prompt Templates
        </Title>
        <Button type="primary" icon={<PlusOutlined />} onClick={handleCreate}>
          New Template
        </Button>
      </div>

      <div className="mb-4">
        <Input
          placeholder="Search templates..."
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
                History
              </Button>,
              <Button
                key="edit"
                type="text"
                icon={<EditOutlined />}
                onClick={() => handleEdit(template)}
              >
                Edit
              </Button>,
              <Button
                key="delete"
                type="text"
                danger
                icon={<DeleteOutlined />}
                onClick={() => handleDelete(template)}
              >
                Delete
              </Button>,
            ]}
          >
            <List.Item.Meta
              title={
                <Space>
                  <span>{template.name}</span>
                  {template.is_active && <Tag color="green">Active</Tag>}
                  {template.ab_test_enabled && <Tag color="blue">A/B Test</Tag>}
                  <Tag>v{template.version}</Tag>
                </Space>
              }
              description={template.description || template.content.slice(0, 100) + "..."}
            />
          </List.Item>
        )}
      />

      <Modal
        title={editingTemplate ? "Edit Template" : "New Template"}
        open={isEditorOpen}
        onOk={handleSave}
        onCancel={() => setIsEditorOpen(false)}
        width={700}
      >
        <Form form={form} layout="vertical" className="mt-4">
          <Form.Item
            name="name"
            label="Name"
            rules={[{ required: true, message: "Please enter a name" }]}
          >
            <Input placeholder="Template name" />
          </Form.Item>
          <Form.Item name="description" label="Description">
            <Input.TextArea placeholder="Template description" rows={2} />
          </Form.Item>
          <Form.Item
            name="content"
            label="Content"
            rules={[{ required: true, message: "Please enter template content" }]}
          >
            <Input.TextArea
              placeholder="Template content with {variable} placeholders"
              rows={6}
            />
          </Form.Item>
          <Form.Item name="variables_schema" label="Variables Schema (JSON)">
            <Input.TextArea placeholder='{"variable": "type"}' rows={3} />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title={`Version History: ${versionHistoryTemplate?.name}`}
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
                    View
                  </Button>,
                  <Button key="rollback" size="small" onClick={() => message.info("Rollback not implemented")}>
                    Rollback
                  </Button>,
                ]}
              >
                <List.Item.Meta
                  title={
                    <Space>
                      <Tag>Version {item.version}</Tag>
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
