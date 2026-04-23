import React, { useState } from 'react';
import { Modal, Input, Form, message } from 'antd';
import { useSkillStore } from '@/stores';
import { useTranslation } from 'react-i18next';

interface SkillCreateModalProps {
  open: boolean;
  onClose: () => void;
}

export const SkillCreateModal: React.FC<SkillCreateModalProps> = ({ open, onClose }) => {
  const { t } = useTranslation();
  const createSkill = useSkillStore((s) => s.createSkill);
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);

  const handleOk = async () => {
    try {
      const values = await form.validateFields();
      setLoading(true);
      const result = await createSkill(values.name, values.description || '', values.content);
      message.success(result);
      form.resetFields();
      onClose();
    } catch (e: unknown) {
      if (e instanceof Error) {
        message.error(e.message);
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      title={t('skill.createTitle')}
      open={open}
      onCancel={onClose}
      onOk={handleOk}
      okText={t('skill.create')}
      confirmLoading={loading}
      width={640}
    >
      <Form form={form} layout="vertical">
        <Form.Item
          name="name"
          label={t('skill.name')}
          rules={[{ required: true, message: t('skill.nameRequired') }]}
        >
          <Input placeholder={t('skill.namePlaceholder')} />
        </Form.Item>
        <Form.Item
          name="description"
          label={t('skill.description')}
        >
          <Input placeholder={t('skill.descriptionPlaceholder')} />
        </Form.Item>
        <Form.Item
          name="content"
          label={t('skill.content')}
          rules={[{ required: true, message: t('skill.contentRequired') }]}
        >
          <Input.TextArea
            rows={12}
            placeholder={t('skill.contentPlaceholder')}
          />
        </Form.Item>
      </Form>
    </Modal>
  );
};

interface SkillEditModalProps {
  open: boolean;
  onClose: () => void;
  skillName: string;
  initialContent: string;
  mode: 'edit' | 'patch';
}

export const SkillEditModal: React.FC<SkillEditModalProps> = ({
  open,
  onClose,
  skillName,
  initialContent,
  mode,
}) => {
  const { t } = useTranslation();
  const patchSkill = useSkillStore((s) => s.patchSkill);
  const editSkill = useSkillStore((s) => s.editSkill);
  const [content, setContent] = useState(initialContent);
  const [loading, setLoading] = useState(false);

  React.useEffect(() => {
    setContent(initialContent);
  }, [initialContent]);

  const handleOk = async () => {
    if (!content.trim()) return;
    setLoading(true);
    try {
      const result = mode === 'patch'
        ? await patchSkill(skillName, content)
        : await editSkill(skillName, content);
      message.success(result);
      onClose();
    } catch (e: unknown) {
      if (e instanceof Error) {
        message.error(e.message);
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      title={mode === 'patch'
        ? t('skill.patchTitle', { name: skillName })
        : t('skill.editTitle', { name: skillName })
      }
      open={open}
      onCancel={onClose}
      onOk={handleOk}
      okText={mode === 'patch' ? t('skill.patch') : t('skill.edit')}
      confirmLoading={loading}
      width={640}
    >
      <Input.TextArea
        value={content}
        onChange={(e) => setContent(e.target.value)}
        rows={16}
        placeholder={mode === 'patch'
          ? t('skill.patchPlaceholder')
          : t('skill.editPlaceholder')
        }
      />
    </Modal>
  );
};
