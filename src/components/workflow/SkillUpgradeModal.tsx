import React, { useState } from 'react';
import { Modal, Button, Tag, Descriptions, Spin, Alert, Divider } from 'antd';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import type { SkillUpgradeSuggestion, AtomicSkillInfo } from '@/components/workflow/types';

interface SkillUpgradeModalProps {
  open: boolean;
  onClose: () => void;
  existingSkill: AtomicSkillInfo;
  generatedSkillName: string;
  generatedSkillDescription: string;
  onConfirm: (suggestion: SkillUpgradeSuggestion) => void;
}

export const SkillUpgradeModal: React.FC<SkillUpgradeModalProps> = ({
  open,
  onClose,
  existingSkill,
  generatedSkillName,
  generatedSkillDescription,
  onConfirm,
}) => {
  const { t } = useTranslation('chat');
  const [suggestion, setSuggestion] = useState<SkillUpgradeSuggestion | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  React.useEffect(() => {
    if (open && !suggestion && !loading) {
      fetchUpgradeSuggestion();
    }
  }, [open]);

  const fetchUpgradeSuggestion = async () => {
    setLoading(true);
    setError(null);

    try {
      const result = await invoke<{ suggestion: SkillUpgradeSuggestion }>('upgrade_skill_with_llm', {
        request: {
          existing_skill_id: existingSkill.id,
          generated_name: generatedSkillName,
          generated_description: generatedSkillDescription,
          generated_input_schema: null,
          generated_output_schema: null,
        },
      });

      setSuggestion(result.suggestion);
    } catch (err) {
      console.error('Failed to get upgrade suggestion:', err);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleClose = () => {
    setSuggestion(null);
    setError(null);
    onClose();
  };

  const handleConfirm = () => {
    if (suggestion) {
      onConfirm(suggestion);
      handleClose();
    }
  };

  const renderJsonSchema = (schema: Record<string, unknown> | null) => {
    if (!schema) return <Tag>无</Tag>;
    try {
      return <pre style={{ margin: 0, fontSize: 12 }}>{JSON.stringify(schema, null, 2)}</pre>;
    } catch {
      return <Tag>无效的Schema</Tag>;
    }
  };

  return (
    <Modal
      title={t('workflow.upgradeSkillTitle')}
      open={open}
      onCancel={handleClose}
      width={800}
      footer={[
        <Button key="cancel" onClick={handleClose}>
          {t('common.cancel')}
        </Button>,
        <Button key="confirm" type="primary" onClick={handleConfirm} disabled={!suggestion}>
          {t('workflow.applyUpgrade')}
        </Button>,
      ]}
    >
      <div style={{ maxHeight: 500, overflowY: 'auto' }}>
        <Descriptions column={2} bordered size="small" style={{ marginBottom: 16 }}>
          <Descriptions.Item label={t('workflow.existingSkill')} span={2}>
            <Tag color="blue">{existingSkill.name}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label={t('workflow.generatedSkill')} span={2}>
            <Tag color="purple">{generatedSkillName}</Tag>
          </Descriptions.Item>
        </Descriptions>

        <Divider>{t('workflow.upgradeSuggestion')}</Divider>

        {loading && (
          <div style={{ textAlign: 'center', padding: 40 }}>
            <Spin size="large" />
            <p>{t('workflow.generatingUpgrade')}</p>
          </div>
        )}

        {error && (
          <Alert
            message={t('workflow.upgradeError')}
            description={error}
            type="error"
            showIcon
          />
        )}

        {suggestion && !loading && (
          <div style={{ backgroundColor: '#f5f5f5', padding: 16, borderRadius: 8 }}>
            <Descriptions column={1} size="small">
              <Descriptions.Item label={t('workflow.upgradedName')}>
                <Tag color="green">{suggestion.name}</Tag>
              </Descriptions.Item>
              <Descriptions.Item label={t('workflow.description')}>
                {suggestion.description}
              </Descriptions.Item>
              <Descriptions.Item label={t('workflow.inputSchema')}>
                {renderJsonSchema(suggestion.input_schema)}
              </Descriptions.Item>
              <Descriptions.Item label={t('workflow.outputSchema')}>
                {renderJsonSchema(suggestion.output_schema)}
              </Descriptions.Item>
              <Descriptions.Item label={t('workflow.upgradeReasoning')}>
                <Alert message={suggestion.reasoning} type="info" showIcon />
              </Descriptions.Item>
            </Descriptions>
          </div>
        )}
      </div>
    </Modal>
  );
};