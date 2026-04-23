import React, { useState, useEffect } from 'react';
import { Modal, Select, Tag, Input, Button } from 'antd';
import { useProviderStore } from '@/stores';
import { useTranslation } from 'react-i18next';
import { Route, Plus, Trash2, Brain, Code, FileText, Languages } from 'lucide-react';

interface ModelRoutingConfig {
  primaryModelId: string;
  codeReviewModelId?: string;
  summarizationModelId?: string;
  translationModelId?: string;
  routingRules?: Record<string, string>;
}

interface ModelRoutingConfigProps {
  conversationId: string;
  open: boolean;
  onClose: () => void;
}

const ModelRoutingConfigPanel: React.FC<ModelRoutingConfigProps> = ({
  conversationId,
  open,
  onClose,
}) => {
  const { t } = useTranslation();
  const providers = useProviderStore((s) => s.providers);
  const [config, setConfig] = useState<ModelRoutingConfig>({
    primaryModelId: '',
    codeReviewModelId: undefined,
    summarizationModelId: undefined,
    translationModelId: undefined,
    routingRules: {},
  });
  const [newRulePattern, setNewRulePattern] = useState('');
  const [newRuleModel, setNewRuleModel] = useState('');

  // Build flat model list
  const allModels = React.useMemo(() => {
    const models: Array<{ value: string; label: string; providerName: string }> = [];
    for (const provider of providers) {
      for (const model of provider.models) {
        models.push({
          value: `${provider.id}:${model.model_id}`,
          label: model.name || model.model_id,
          providerName: provider.name,
        });
      }
    }
    return models;
  }, [providers]);

  // Load saved config
  useEffect(() => {
    if (!open) return;
    try {
      const saved = localStorage.getItem(`axagent:model-routing:${conversationId}`);
      if (saved) {
        setConfig(JSON.parse(saved));
      }
    } catch { /* ignore */ }
  }, [open, conversationId]);

  const saveConfig = () => {
    localStorage.setItem(`axagent:model-routing:${conversationId}`, JSON.stringify(config));
    onClose();
  };

  const addRoutingRule = () => {
    if (!newRulePattern || !newRuleModel) return;
    setConfig((prev) => ({
      ...prev,
      routingRules: { ...(prev.routingRules || {}), [newRulePattern]: newRuleModel },
    }));
    setNewRulePattern('');
    setNewRuleModel('');
  };

  const removeRoutingRule = (pattern: string) => {
    setConfig((prev) => {
      const rules = { ...(prev.routingRules || {}) };
      delete rules[pattern];
      return { ...prev, routingRules: rules };
    });
  };

  return (
    <Modal
      title={t('chat.modelRoutingConfig.title')}
      open={open}
      onOk={saveConfig}
      onCancel={onClose}
      width={600}
      okText={t('chat.modelRoutingConfig.save')}
    >
      <div className="space-y-4">
        {/* Primary Model */}
        <div>
          <label className="flex items-center gap-1 text-sm font-medium mb-1">
            <Brain size={14} /> {t('chat.modelRoutingConfig.primaryModel')}
          </label>
          <Select
            value={config.primaryModelId || undefined}
            onChange={(v) => setConfig((prev) => ({ ...prev, primaryModelId: v }))}
            options={allModels.map((m) => ({ value: m.value, label: `${m.label} (${m.providerName})` }))}
            showSearch
            placeholder={t('chat.modelRoutingConfig.selectPrimaryModel')}
            style={{ width: '100%' }}
          />
        </div>

        {/* Code Review Model */}
        <div>
          <label className="flex items-center gap-1 text-sm font-medium mb-1">
            <Code size={14} /> {t('chat.modelRoutingConfig.codeReviewModel')}
          </label>
          <Select
            value={config.codeReviewModelId || undefined}
            onChange={(v) => setConfig((prev) => ({ ...prev, codeReviewModelId: v || undefined }))}
            options={[{ value: '', label: t('chat.modelRoutingConfig.usePrimaryModel') }, ...allModels.map((m) => ({ value: m.value, label: `${m.label} (${m.providerName})` }))]}
            allowClear
            showSearch
            placeholder={t('chat.modelRoutingConfig.codeReviewPlaceholder')}
            style={{ width: '100%' }}
          />
        </div>

        {/* Summarization Model */}
        <div>
          <label className="flex items-center gap-1 text-sm font-medium mb-1">
            <FileText size={14} /> {t('chat.modelRoutingConfig.summarizationModel')}
          </label>
          <Select
            value={config.summarizationModelId || undefined}
            onChange={(v) => setConfig((prev) => ({ ...prev, summarizationModelId: v || undefined }))}
            options={[{ value: '', label: t('chat.modelRoutingConfig.usePrimaryModel') }, ...allModels.map((m) => ({ value: m.value, label: `${m.label} (${m.providerName})` }))]}
            allowClear
            showSearch
            placeholder={t('chat.modelRoutingConfig.summarizationPlaceholder')}
            style={{ width: '100%' }}
          />
        </div>

        {/* Translation Model */}
        <div>
          <label className="flex items-center gap-1 text-sm font-medium mb-1">
            <Languages size={14} /> {t('chat.modelRoutingConfig.translationModel')}
          </label>
          <Select
            value={config.translationModelId || undefined}
            onChange={(v) => setConfig((prev) => ({ ...prev, translationModelId: v || undefined }))}
            options={[{ value: '', label: t('chat.modelRoutingConfig.usePrimaryModel') }, ...allModels.map((m) => ({ value: m.value, label: `${m.label} (${m.providerName})` }))]}
            allowClear
            showSearch
            placeholder={t('chat.modelRoutingConfig.translationPlaceholder')}
            style={{ width: '100%' }}
          />
        </div>

        {/* Custom Routing Rules */}
        <div>
          <label className="flex items-center gap-1 text-sm font-medium mb-2">
            <Route size={14} /> {t('chat.modelRoutingConfig.customRoutingRules')}
          </label>
          <div className="text-xs text-gray-500 mb-2">
            {t('chat.modelRoutingConfig.customRoutingRulesDesc')}
          </div>

          {/* Existing rules */}
          {Object.entries(config.routingRules || {}).map(([pattern, model_id]) => (
            <div key={pattern} className="flex items-center gap-2 mb-1">
              <Tag color="blue">{pattern}</Tag>
              <span className="text-xs text-gray-500">→</span>
              <Tag color="green">{allModels.find((m) => m.value === model_id)?.label || model_id}</Tag>
              <Button
                type="text"
                size="small"
                danger
                icon={<Trash2 size={12} />}
                onClick={() => removeRoutingRule(pattern)}
              />
            </div>
          ))}

          {/* Add new rule */}
          <div className="flex items-center gap-2 mt-2">
            <Input
              size="small"
              placeholder={t('chat.modelRoutingConfig.patternPlaceholder')}
              value={newRulePattern}
              onChange={(e) => setNewRulePattern(e.target.value)}
              style={{ width: 180 }}
            />
            <Select
              size="small"
              value={newRuleModel || undefined}
              onChange={setNewRuleModel}
              options={allModels.map((m) => ({ value: m.value, label: m.label }))}
              showSearch
              placeholder={t('chat.modelRoutingConfig.modelPlaceholder')}
              style={{ width: 180 }}
            />
            <Button
              size="small"
              type="dashed"
              icon={<Plus size={12} />}
              onClick={addRoutingRule}
              disabled={!newRulePattern || !newRuleModel}
            >
              {t('chat.modelRoutingConfig.add')}
            </Button>
          </div>
        </div>
      </div>
    </Modal>
  );
};

export default ModelRoutingConfigPanel;
export type { ModelRoutingConfig };
