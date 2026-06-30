// SPDX-License-Identifier: AGPL-3.0-only

import { Button, Form, Input, InputNumber, Select, Switch } from "antd";
import { useEffect } from "react";

export interface ConfigField {
  key: string;
  label: string;
  type: "text" | "number" | "select" | "switch" | "slider";
  options?: { label: string; value: string }[];
  min?: number;
  max?: number;
  step?: number;
}

interface EngineConfigFormProps {
  config: Record<string, unknown>;
  fields: ConfigField[];
  onSave: (config: Record<string, unknown>) => void;
}

export default function EngineConfigForm({ config, fields, onSave }: EngineConfigFormProps) {
  const [form] = Form.useForm();

  useEffect(() => {
    form.setFieldsValue(config);
  }, [config, form]);

  const handleFinish = (values: Record<string, unknown>) => {
    onSave(values);
  };

  return (
    <Form
      form={form}
      layout="vertical"
      size="small"
      onFinish={handleFinish}
      initialValues={config}
    >
      {fields.map((field) => (
        <Form.Item
          key={field.key}
          name={field.key}
          label={field.label}
          valuePropName={field.type === "switch" ? "checked" : "value"}
        >
          {field.type === "switch" ? (
            <Switch />
          ) : field.type === "number" ? (
            <InputNumber
              style={{ width: "100%" }}
              min={field.min}
              max={field.max}
              step={field.step}
            />
          ) : field.type === "select" ? (
            <Select
              options={field.options?.map((o) => ({ label: o.label, value: o.value }))}
            />
          ) : field.type === "slider" ? (
            <InputNumber
              style={{ width: "100%" }}
              min={field.min ?? 0}
              max={field.max ?? 100}
              step={field.step ?? 1}
            />
          ) : (
            <Input />
          )}
        </Form.Item>
      ))}

      <Form.Item>
        <Button type="primary" htmlType="submit">
          保存配置
        </Button>
      </Form.Item>
    </Form>
  );
}
