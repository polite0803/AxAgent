import React, { useState } from 'react';
import { Input, Select, Button, Table, Typography } from 'antd';
import { PlusOutlined, DeleteOutlined } from '@ant-design/icons';

const { Text } = Typography;

interface SchemaProperty {
  name: string;
  type: string;
  description: string;
  required: boolean;
}

interface JsonSchemaEditorProps {
  value?: Record<string, unknown>;
  onChange?: (schema: Record<string, unknown>) => void;
}

const TYPE_OPTIONS = [
  { label: 'string', value: 'string' },
  { label: 'number', value: 'number' },
  { label: 'integer', value: 'integer' },
  { label: 'boolean', value: 'boolean' },
  { label: 'array', value: 'array' },
  { label: 'object', value: 'object' },
];

function schemaToProperties(schema: Record<string, unknown>): SchemaProperty[] {
  const props: SchemaProperty[] = [];
  const properties = (schema.properties || {}) as Record<string, Record<string, unknown>>;
  const required = (schema.required || []) as string[];
  for (const [name, def] of Object.entries(properties)) {
    props.push({
      name,
      type: (def.type as string) || 'string',
      description: (def.description as string) || '',
      required: required.includes(name),
    });
  }
  return props;
}

function propertiesToSchema(properties: SchemaProperty[]): Record<string, unknown> {
  const schemaProps: Record<string, unknown> = {};
  const required: string[] = [];
  for (const p of properties) {
    schemaProps[p.name] = { type: p.type, description: p.description };
    if (p.required) required.push(p.name);
  }
  return {
    type: 'object',
    properties: schemaProps,
    required,
  };
}

export const JsonSchemaEditor: React.FC<JsonSchemaEditorProps> = ({ value, onChange }) => {
  const [properties, setProperties] = useState<SchemaProperty[]>(
    value ? schemaToProperties(value) : []
  );

  const updateProperties = (newProps: SchemaProperty[]) => {
    setProperties(newProps);
    onChange?.(propertiesToSchema(newProps));
  };

  const addProperty = () => {
    updateProperties([
      ...properties,
      { name: `param_${properties.length + 1}`, type: 'string', description: '', required: false },
    ]);
  };

  const removeProperty = (index: number) => {
    updateProperties(properties.filter((_, i) => i !== index));
  };

  const updateProperty = (index: number, field: keyof SchemaProperty, val: string | boolean) => {
    const newProps = [...properties];
    newProps[index] = { ...newProps[index], [field]: val };
    updateProperties(newProps);
  };

  const columns = [
    {
      title: '参数名',
      dataIndex: 'name',
      key: 'name',
      width: 120,
      render: (name: string, _: SchemaProperty, index: number) => (
        <Input
          value={name}
          onChange={(e) => updateProperty(index, 'name', e.target.value)}
          size="small"
        />
      ),
    },
    {
      title: '类型',
      dataIndex: 'type',
      key: 'type',
      width: 100,
      render: (type: string, _: SchemaProperty, index: number) => (
        <Select
          value={type}
          onChange={(v) => updateProperty(index, 'type', v)}
          options={TYPE_OPTIONS}
          size="small"
          style={{ width: '100%' }}
        />
      ),
    },
    {
      title: '描述',
      dataIndex: 'description',
      key: 'description',
      render: (desc: string, _: SchemaProperty, index: number) => (
        <Input
          value={desc}
          onChange={(e) => updateProperty(index, 'description', e.target.value)}
          size="small"
        />
      ),
    },
    {
      title: '必填',
      dataIndex: 'required',
      key: 'required',
      width: 50,
      render: (required: boolean, _: SchemaProperty, index: number) => (
        <input
          type="checkbox"
          checked={required}
          onChange={(e) => updateProperty(index, 'required', e.target.checked)}
        />
      ),
    },
    {
      title: '',
      key: 'action',
      width: 40,
      render: (_: unknown, __: SchemaProperty, index: number) => (
        <Button type="text" danger size="small" icon={<DeleteOutlined />} onClick={() => removeProperty(index)} />
      ),
    },
  ];

  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}>
        <Text type="secondary" style={{ fontSize: 12 }}>JSON Schema 参数定义</Text>
        <Button size="small" type="dashed" icon={<PlusOutlined />} onClick={addProperty}>
          添加参数
        </Button>
      </div>
      <Table
        dataSource={properties}
        columns={columns}
        rowKey={(_, i) => String(i)}
        size="small"
        pagination={false}
      />
      {properties.length === 0 && (
        <Text type="secondary" style={{ fontSize: 12, display: 'block', textAlign: 'center', padding: 8 }}>
          暂无参数，点击"添加参数"开始定义
        </Text>
      )}
    </div>
  );
};
