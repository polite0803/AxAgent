import { useEffect, useState } from 'react';
import {
  Card,
  Typography,
  Form,
  Input,
  Button,
  Space,
  message,
  Spin,
  Modal,
  Tag,
  Descriptions,
  Empty,
  Popconfirm,
} from 'antd';
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/lib/invoke';
import type { SchemaVersion } from '@/types/llmWiki';

const { Title, Text } = Typography;
const { TextArea } = Input;

interface SchemaEditorProps {
  wikiId: string;
  onSchemaChange?: (schema: Record<string, unknown>) => void;
}

export function SchemaEditor({ wikiId, onSchemaChange }: SchemaEditorProps) {
  const { t } = useTranslation();
  const [schemas, setSchemas] = useState<SchemaVersion[]>([]);
  const [loading, setLoading] = useState(true);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingSchema, setEditingSchema] = useState<SchemaVersion | null>(null);
  const [schemaContent, setSchemaContent] = useState('');
  const [saving, setSaving] = useState(false);
  const [form] = Form.useForm();

  useEffect(() => {
    loadSchemas();
  }, [wikiId]);

  const loadSchemas = async () => {
    setLoading(true);
    try {
      const content = await invoke<string>('llm_wiki_get_schema', { wikiId });
      setSchemas(content ? [{
        version: '1.0',
        created_at: Date.now(),
        content_hash: '',
        note_count: 0,
        description: undefined,
        schema: parseSchemaContent(content),
      } as SchemaVersion] : []);
    } catch (e) {
      message.error(String(e));
    }
    setLoading(false);
  };

  const parseSchemaContent = (content: string): Record<string, unknown> => {
    try {
      const parsed: Record<string, unknown> = {};
      let inFrontmatter = false;
      for (const line of content.split('\n')) {
        if (line.trim() === '---') {
          inFrontmatter = !inFrontmatter;
          continue;
        }
        if (inFrontmatter) {
          const idx = line.indexOf(':');
          if (idx > 0) {
            const key = line.substring(0, idx).trim();
            const val = line.substring(idx + 1).trim();
            parsed[key] = val;
          }
        }
      }
      return parsed;
    } catch {
      return { raw: content };
    }
  };

  const handleCreateSchema = () => {
    setEditingSchema(null);
    setSchemaContent('');
    setIsModalOpen(true);
  };

  const handleEditSchema = (schema: SchemaVersion) => {
    setEditingSchema(schema);
    setSchemaContent(JSON.stringify(schema.schema, null, 2));
    setIsModalOpen(true);
  };

  const handleSaveSchema = async () => {
    setSaving(true);
    try {
      const schemaObj = JSON.parse(schemaContent);
      const version = schemas.length > 0
        ? `${parseInt(schemas[0].version.split('.')[0]) + 1}.0`
        : '1.0';

      if (editingSchema) {
        await invoke('llm_wiki_update_schema', {
          wikiId,
          content: schemaContent,
        });
        message.success(t('wiki.schema.updated', 'Schema updated'));
      } else {
        await invoke('llm_wiki_create_schema_version', {
          wikiId,
          version,
          schema: schemaObj,
          description: form.getFieldValue('description'),
        });
        message.success(t('wiki.schema.created', 'Schema created'));
      }

      setIsModalOpen(false);
      loadSchemas();
      onSchemaChange?.(schemaObj);
    } catch (e) {
      message.error(String(e));
    }
    setSaving(false);
  };

  const handleDeleteSchema = async (_schemaId: string) => {
    try {
      await invoke('llm_wiki_delete_schema', { wikiId });
      message.success(t('wiki.schema.deleted', 'Schema deleted'));
      loadSchemas();
    } catch (e) {
      message.error(String(e));
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Spin size="large" />
      </div>
    );
  }

  return (
    <Space direction="vertical" size="large" style={{ width: '100%' }}>
      <div className="flex items-center justify-between">
        <Title level={4} className="m-0">
          {t('wiki.schema.title', 'Schema Versions')}
        </Title>
        <Button type="primary" icon={<PlusOutlined />} onClick={handleCreateSchema}>
          {t('wiki.schema.create', 'Create Schema')}
        </Button>
      </div>

      {schemas.length === 0 ? (
        <Empty description={t('wiki.schema.noSchemas', 'No schemas defined')} />
      ) : (
        schemas.map((schema) => (
          <Card key={schema.id} size="small">
            <div className="flex items-center justify-between">
              <Descriptions column={2} size="small" className="flex-1">
                <Descriptions.Item label={t('wiki.schema.version', 'Version')}>
                  <Tag color="blue">{schema.version}</Tag>
                </Descriptions.Item>
                <Descriptions.Item label={t('wiki.schema.createdAt', 'Created')}>
                  {new Date(schema.createdAt * 1000).toLocaleString()}
                </Descriptions.Item>
                {schema.description && (
                  <Descriptions.Item label={t('wiki.schema.description', 'Description')}>
                    {schema.description}
                  </Descriptions.Item>
                )}
              </Descriptions>
              <Space>
                <Button
                  type="text"
                  icon={<EditOutlined />}
                  onClick={() => handleEditSchema(schema)}
                />
                <Popconfirm
                  title={t('wiki.schema.deleteConfirm', 'Are you sure to delete this schema?')}
                  onConfirm={() => handleDeleteSchema(schema.id)}
                >
                  <Button type="text" danger icon={<DeleteOutlined />} />
                </Popconfirm>
              </Space>
            </div>
            <pre className="mt-2 p-2 bg-gray-50 rounded text-xs overflow-auto max-h-40">
              {JSON.stringify(schema.schema, null, 2)}
            </pre>
          </Card>
        ))
      )}

      <Modal
        title={editingSchema ? t('wiki.schema.edit', 'Edit Schema') : t('wiki.schema.create', 'Create Schema')}
        open={isModalOpen}
        onCancel={() => setIsModalOpen(false)}
        footer={[
          <Button key="cancel" onClick={() => setIsModalOpen(false)}>
            {t('common.cancel', 'Cancel')}
          </Button>,
          <Button key="save" type="primary" loading={saving} onClick={handleSaveSchema}>
            {t('common.save', 'Save')}
          </Button>,
        ]}
        width={700}
      >
        <Form form={form} layout="vertical">
          {!editingSchema && (
            <Form.Item
              name="description"
              label={t('wiki.schema.description', 'Description')}
              rules={[{ required: true, message: 'Please enter a description' }]}
            >
              <Input placeholder={t('wiki.schema.descriptionPlaceholder', 'e.g., Initial schema')} />
            </Form.Item>
          )}
          <Form.Item
            label={t('wiki.schema.schema', 'Schema Definition')}
            extra={
              <Text type="secondary">
                {t('wiki.schema.schemaHelp', 'Enter a JSON object defining the frontmatter schema')}
              </Text>
            }
          >
            <TextArea
              value={schemaContent}
              onChange={(e) => setSchemaContent(e.target.value)}
              rows={12}
              placeholder='{"type": "object", "properties": {...}}'
              style={{ fontFamily: 'monospace' }}
            />
          </Form.Item>
        </Form>
      </Modal>
    </Space>
  );
}