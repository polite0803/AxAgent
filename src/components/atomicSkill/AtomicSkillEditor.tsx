import { Button, Drawer, Form, Input, message, Space, Switch } from "antd";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAtomicSkillStore } from "../../stores/feature/atomicSkillStore";
import type { AtomicSkill, CreateAtomicSkillParams, UpdateAtomicSkillParams } from "../../types";
import { EntryRefSelector } from "./EntryRefSelector";
import { EntryTypeSelector } from "./EntryTypeSelector";
import { JsonSchemaEditor } from "./JsonSchemaEditor";
import { SemanticConflictAlert } from "./SemanticConflictAlert";

const { TextArea } = Input;

interface AtomicSkillEditorProps {
  visible: boolean;
  skill?: AtomicSkill | null;
  onClose: () => void;
}

export const AtomicSkillEditor: React.FC<AtomicSkillEditorProps> = ({ visible, skill, onClose }) => {
  const [form] = Form.useForm();
  const { createSkill, updateSkill, checkSemanticUniqueness } = useAtomicSkillStore();
  const [semanticConflict, setSemanticConflict] = useState<AtomicSkill | null>(null);
  const [saving, setSaving] = useState(false);
  const [entryType, setEntryType] = useState<string | undefined>(undefined);
  const isEdit = !!skill;

  useEffect(() => {
    if (visible) {
      if (skill) {
        form.setFieldsValue({
          name: skill.name,
          description: skill.description,
          entry_type: skill.entry_type,
          entry_ref: skill.entry_ref,
          category: skill.category,
          tags: skill.tags.join(", "),
          version: skill.version,
          enabled: skill.enabled,
        });
        setEntryType(skill.entry_type);
      } else {
        form.resetFields();
        setEntryType(undefined);
      }
      setSemanticConflict(null);
    }
  }, [visible, skill, form]);

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      setSaving(true);
      setSemanticConflict(null);

      const tags = values.tags
        ? values.tags.split(",").map((t: string) => t.trim()).filter(Boolean)
        : [];

      if (isEdit && skill) {
        const params: UpdateAtomicSkillParams = {
          name: values.name,
          description: values.description,
          entry_type: values.entry_type,
          entry_ref: values.entry_ref,
          category: values.category,
          tags,
          version: values.version,
          enabled: values.enabled,
        };
        const success = await updateSkill(skill.id, params);
        if (success) {
          message.success("原子Skill已更新");
          onClose();
        }
      } else {
        // Check semantic uniqueness before creating
        const conflict = await checkSemanticUniqueness(
          values.entry_type,
          values.entry_ref,
        );
        if (conflict) {
          setSemanticConflict(conflict);
          setSaving(false);
          return;
        }

        const params: CreateAtomicSkillParams = {
          name: values.name,
          description: values.description,
          entry_type: values.entry_type,
          entry_ref: values.entry_ref,
          category: values.category || "general",
          tags,
          version: values.version || "1.0.0",
          enabled: values.enabled ?? true,
        };
        await createSkill(params);
        message.success("原子Skill已创建");
        onClose();
      }
    } catch {
      // Form validation failed
    } finally {
      setSaving(false);
    }
  };

  return (
    <Drawer
      title={isEdit ? "编辑原子Skill" : "新建原子Skill"}
      open={visible}
      onClose={onClose}
      width={560}
      extra={
        <Space>
          <Button onClick={onClose}>取消</Button>
          <Button type="primary" loading={saving} onClick={handleSave}>保存</Button>
        </Space>
      }
    >
      <SemanticConflictAlert
        conflict={semanticConflict}
        onClose={() => setSemanticConflict(null)}
      />

      <Form form={form} layout="vertical">
        <Form.Item name="name" label="名称" rules={[{ required: true, message: "请输入名称" }]}>
          <Input placeholder="atomic_my_skill" />
        </Form.Item>

        <Form.Item name="description" label="描述" rules={[{ required: true, message: "请输入描述" }]}>
          <TextArea rows={3} placeholder="描述此原子Skill的功能" />
        </Form.Item>

        <Form.Item name="entry_type" label="执行入口类型" rules={[{ required: true, message: "请选择入口类型" }]}>
          <EntryTypeSelector
            onChange={(v) => {
              setEntryType(v);
              form.setFieldsValue({ entry_ref: undefined });
            }}
          />
        </Form.Item>

        <Form.Item name="entry_ref" label="执行入口引用" rules={[{ required: true, message: "请选择或输入入口引用" }]}>
          <EntryRefSelector entryType={entryType} />
        </Form.Item>

        <Form.Item name="input_schema" label="输入参数模式">
          <JsonSchemaEditor />
        </Form.Item>

        <Form.Item name="output_schema" label="输出参数模式">
          <JsonSchemaEditor />
        </Form.Item>

        <Form.Item name="category" label="分类">
          <Input placeholder="general" />
        </Form.Item>

        <Form.Item name="tags" label="标签（逗号分隔）">
          <Input placeholder="tag1, tag2" />
        </Form.Item>

        <Form.Item name="version" label="版本">
          <Input placeholder="1.0.0" />
        </Form.Item>

        <Form.Item name="enabled" label="启用" valuePropName="checked">
          <Switch />
        </Form.Item>
      </Form>
    </Drawer>
  );
};
