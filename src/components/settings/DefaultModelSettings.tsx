// SPDX-License-Identifier: AGPL-3.0-only

import { ModelParamSliders } from "@/components/common/ModelParamSliders";
import { Tooltip } from "@/components/layout/Tooltip";
import { ModelSelect, parseModelValue } from "@/components/shared/ModelSelect";
import { getModelSelection } from "@/lib/settingsAdaptor";
import { useProviderStore, useSettingsStore } from "@/stores";
import { ModelSelection } from "@/types";
import type { AppSettings } from "@/types";
import { Button, Divider, Input, InputNumber, Modal, Slider, theme } from "antd";
import { Info, Settings, Undo2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

// 默认 prompt 已移至 i18n locale keys:
// settings.titleSummaryPromptDefault / settings.compressionPromptDefault

// ── Context count slider ───────────────────────────────────

function ContextCountParam({
  label,
  tooltip,
  value,
  onChange,
}: {
  label: string;
  tooltip?: string;
  value: number | null;
  onChange: (v: number | null) => void;
}) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const effectiveValue = value ?? 5;
  const contextMarks: Record<number, string> = {
    0: "0",
    5: "5",
    10: "10",
    15: "15",
    50: t("common.unlimited"),
  };

  return (
    <>
      <div style={{ padding: "12px 0 4px" }}>
        <span
          style={{
            display: "flex",
            alignItems: "center",
            gap: 4,
            fontSize: 14,
          }}
        >
          {label}
          {tooltip && (
            <Tooltip title={tooltip}>
              <Info
                size={12}
                style={{ color: token.colorTextSecondary, cursor: "help" }}
              />
            </Tooltip>
          )}
        </span>
      </div>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          paddingBottom: 8,
        }}
      >
        <Slider
          style={{ flex: 1 }}
          min={0}
          max={50}
          step={1}
          marks={contextMarks}
          value={effectiveValue}
          onChange={(v) => onChange(v)}
        />
        <InputNumber
          id="default-model-settings-inputnumber-51"
          style={{ width: 72 }}
          min={0}
          max={50}
          value={effectiveValue}
          onChange={(v) => onChange(v ?? 5)}
          size="small"
        />
      </div>
      <Divider style={{ margin: 0 }} />
    </>
  );
}

// ── Settings Modal ─────────────────────────────────────────

function ModelParamsModal({
  open,
  onClose,
  title,
  showPrompt,
  showContextCount,
  promptKey,
  temperatureKey,
  topPKey,
  maxTokensKey,
  contextCountKey,
  defaultTemperature,
  defaultTopP,
  defaultMaxTokens,
  defaultPrompt,
  promptPlaceholder,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  showPrompt: boolean;
  showContextCount: boolean;
  promptKey?: keyof AppSettings;
  temperatureKey?: keyof AppSettings;
  topPKey?: keyof AppSettings;
  maxTokensKey?: keyof AppSettings;
  contextCountKey?: keyof AppSettings;
  defaultTemperature?: number;
  defaultTopP?: number;
  defaultMaxTokens?: number;
  defaultPrompt?: string;
  promptPlaceholder?: string;
}) {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);

  const handleReset = useCallback(() => {
    const resetValues: Record<string, unknown> = {};
    if (temperatureKey) { resetValues[temperatureKey] = null; }
    if (topPKey) { resetValues[topPKey] = null; }
    if (maxTokensKey) { resetValues[maxTokensKey] = null; }
    if (contextCountKey) {
      resetValues[contextCountKey] = null;
    }
    if (promptKey) {
      resetValues[promptKey] = null;
    }
    saveSettings(resetValues as Partial<AppSettings>);
  }, [
    saveSettings,
    temperatureKey,
    topPKey,
    maxTokensKey,
    contextCountKey,
    promptKey,
  ]);

  return (
    <Modal
      open={open}
      onCancel={onClose}
      title={title}
      footer={null}
      width={520}
      mask={{ enabled: true, blur: true }}
    >
      {showPrompt && promptKey && (
        <div style={{ marginBottom: 16 }}>
          <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
            {t("settings.promptLabel")}
          </div>
          <Input.TextArea
            rows={4}
            value={(settings[promptKey] as string | null)
              ?? (defaultPrompt || t("settings.titleSummaryPromptDefault"))}
            onChange={(e) =>
              saveSettings({
                [promptKey]: e.target.value || null,
              } as Partial<AppSettings>)}
            placeholder={promptPlaceholder || t("settings.titleSummaryPromptPlaceholder")}
          />
        </div>
      )}

      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          marginBottom: 4,
        }}
      >
        <span style={{ fontSize: 14, fontWeight: 600 }}>
          {t("settings.modelParams")}
        </span>
        <Button
          type="text"
          size="small"
          icon={<Undo2 size={14} />}
          onClick={handleReset}
        />
      </div>
      <Divider style={{ margin: "4px 0 0" }} />

      <ModelParamSliders
        values={{
          temperature: (temperatureKey
            ? (settings[temperatureKey] as number | null)
            : null) ?? defaultTemperature ?? 0.7,
          topP: (topPKey ? (settings[topPKey] as number | null) : null)
            ?? defaultTopP ?? 1.0,
          maxTokens: (maxTokensKey
            ? (settings[maxTokensKey] as number | null)
            : null) ?? defaultMaxTokens ?? 4096,
          frequencyPenalty: null,
        }}
        onChange={(v) => {
          const patch: Record<string, unknown> = {};
          if (temperatureKey && "temperature" in v) {
            patch[temperatureKey] = v.temperature;
          }
          if (topPKey && "topP" in v) {
            patch[topPKey] = v.topP;
          }
          if (maxTokensKey && "maxTokens" in v) {
            patch[maxTokensKey] = v.maxTokens;
          }
          saveSettings(patch as Partial<AppSettings>);
        }}
        defaults={{
          temperature: defaultTemperature ?? 0.7,
          topP: defaultTopP ?? 1.0,
          maxTokens: defaultMaxTokens ?? 4096,
        }}
        visibleParams={["temperature", "topP", "maxTokens"]}
      />

      {showContextCount && contextCountKey && (
        <ContextCountParam
          label={t("settings.contextCount")}
          tooltip={t("settings.contextCountTooltip")}
          value={settings[contextCountKey] as number | null}
          onChange={(v) => saveSettings({ [contextCountKey]: v } as Partial<AppSettings>)}
        />
      )}
    </Modal>
  );
}

// ── Model Card ─────────────────────────────────────────────

function ModelCard({
  title,
  description,
  modelKey,
  placeholder,
  modalTitle,
  showPrompt,
  showContextCount,
  promptKey,
  temperatureKey,
  topPKey,
  maxTokensKey,
  contextCountKey,
  defaultTemperature,
  defaultTopP,
  defaultMaxTokens,
  defaultPrompt,
  promptPlaceholder,
}: {
  title: string;
  description: string;
  modelKey: keyof AppSettings;
  placeholder: string;
  modalTitle: string;
  showPrompt: boolean;
  showContextCount: boolean;
  promptKey?: keyof AppSettings;
  // 参数键全部可选：不传 temperatureKey 表示该模型卡片不提供参数弹窗
  // （如回退/图片/视频/语音等扩展模型，暂不需要独立采样参数）
  temperatureKey?: keyof AppSettings;
  topPKey?: keyof AppSettings;
  maxTokensKey?: keyof AppSettings;
  contextCountKey?: keyof AppSettings;
  defaultTemperature?: number;
  defaultTopP?: number;
  defaultMaxTokens?: number;
  defaultPrompt?: string;
  promptPlaceholder?: string;
}) {
  const { token } = theme.useToken();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const [modalOpen, setModalOpen] = useState(false);
  const hasParams = temperatureKey !== undefined;

  // 使用 getModelSelection 获取强类型的模型选择
  // 如果返回 null，说明未设置或设置无效（后者已在数据层被清理）
  const modelSelection = getModelSelection(settings, modelKey);

  // 转换为 ModelSelect 组件需要的字符串格式
  const currentValue = modelSelection
    ? ModelSelection.toValue(modelSelection)
    : undefined;

  const handleChange = useCallback(
    (value: string | undefined) => {
      if (!value) {
        saveSettings({
          [modelKey]: null,
        } as Partial<AppSettings>);
        return;
      }
      const parsed = parseModelValue(value);
      if (parsed) {
        const modelRef = ModelSelection.from(parsed.providerId, parsed.modelId);
        saveSettings({
          [modelKey]: modelRef,
        } as Partial<AppSettings>);
      }
    },
    [saveSettings, modelKey],
  );

  return (
    <>
      <SettingsGroup title={title}>
        <div
          style={{
            fontSize: 12,
            color: token.colorTextDescription,
            marginBottom: 12,
          }}
        >
          {description}
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <ModelSelect
            style={{ flex: 1 }}
            value={currentValue}
            onChange={handleChange}
            placeholder={placeholder}
          />
          {hasParams && (
            <Tooltip title={modalTitle}>
              <Button
                icon={<Settings size={16} />}
                onClick={() => setModalOpen(true)}
              />
            </Tooltip>
          )}
        </div>
      </SettingsGroup>

      {hasParams && (
        <ModelParamsModal
          open={modalOpen}
          onClose={() => setModalOpen(false)}
          title={modalTitle}
          showPrompt={showPrompt}
          showContextCount={showContextCount}
          promptKey={promptKey}
          temperatureKey={temperatureKey}
          topPKey={topPKey}
          maxTokensKey={maxTokensKey}
          contextCountKey={contextCountKey}
          defaultTemperature={defaultTemperature}
          defaultTopP={defaultTopP}
          defaultMaxTokens={defaultMaxTokens}
          defaultPrompt={defaultPrompt}
          promptPlaceholder={promptPlaceholder}
        />
      )}
    </>
  );
}

// ── Main Component ─────────────────────────────────────────

export function DefaultModelSettings() {
  const { t } = useTranslation();
  const fetchProviders = useProviderStore((s) => s.fetchProviders);
  const providers = useProviderStore((s) => s.providers);
  const providerLoading = useProviderStore((s) => s.loading);
  const validateAndCleanModels = useSettingsStore((s) => s.validateAndCleanModels);
  const fetchSettings = useSettingsStore((s) => s.fetchSettings);

  // 首次加载：获取 settings 和 providers
  useEffect(() => {
    fetchSettings();
    fetchProviders();
  }, [fetchSettings, fetchProviders]);

  // 当 providers 加载完成后，立即验证并清理无效的模型引用
  // 这是类型驱动设计的关键：在数据层就清除无效状态
  useEffect(() => {
    if (!providerLoading && providers.length > 0) {
      validateAndCleanModels(providers).then((result) => {
        if (result.changed) {
          console.info("[DefaultModelSettings] 已清理无效模型引用", result.invalidFields);
        }
      });
    }
  }, [providerLoading, providers, validateAndCleanModels]);

  const placeholderText = t("settings.useActiveModel");

  return (
    <div style={{ padding: 24 }}>
      <ModelCard
        title={t("settings.defaultConversationModel")}
        description={t("settings.defaultConversationModelDesc")}
        modelKey="defaultModel"
        placeholder={placeholderText}
        modalTitle={t("settings.defaultConversationModel")}
        showPrompt={false}
        showContextCount={true}
        temperatureKey="defaultTemperature"
        topPKey="defaultTopP"
        maxTokensKey="defaultMaxTokens"
        contextCountKey="defaultContextCount"
        defaultTemperature={0.7}
        defaultTopP={1.0}
        defaultMaxTokens={4096}
      />

      <ModelCard
        title={t("settings.titleSummaryModel")}
        description={t("settings.titleSummaryModelDesc")}
        modelKey="titleSummaryModel"
        placeholder={placeholderText}
        modalTitle={t("settings.titleSummaryModel")}
        showPrompt={true}
        showContextCount={false}
        promptKey="titleSummaryPrompt"
        temperatureKey="titleSummaryTemperature"
        topPKey="titleSummaryTopP"
        maxTokensKey="titleSummaryMaxTokens"
        defaultTemperature={0.3}
        defaultTopP={1.0}
        defaultMaxTokens={256}
      />

      <ModelCard
        title={t("settings.compressionModel")}
        description={t("settings.compressionModelDesc")}
        modelKey="compressionModel"
        placeholder={placeholderText}
        modalTitle={t("settings.compressionModel")}
        showPrompt={true}
        showContextCount={false}
        promptKey="compressionPrompt"
        temperatureKey="compressionTemperature"
        topPKey="compressionTopP"
        maxTokensKey="compressionMaxTokens"
        defaultTemperature={0.3}
        defaultTopP={1.0}
        defaultMaxTokens={1024}
        defaultPrompt={t("settings.compressionPromptDefault")}
        promptPlaceholder={t("settings.compressionPromptPlaceholder")}
      />

      <ModelCard
        title={t("settings.fallbackModel")}
        description={t("settings.fallbackModelDesc")}
        modelKey="fallbackModel"
        placeholder={placeholderText}
        modalTitle={t("settings.fallbackModel")}
        showPrompt={false}
        showContextCount={false}
      />

      <ModelCard
        title={t("settings.imageModel")}
        description={t("settings.imageModelDesc")}
        modelKey="imageModel"
        placeholder={placeholderText}
        modalTitle={t("settings.imageModel")}
        showPrompt={false}
        showContextCount={false}
      />

      <ModelCard
        title={t("settings.videoModel")}
        description={t("settings.videoModelDesc")}
        modelKey="videoModel"
        placeholder={placeholderText}
        modalTitle={t("settings.videoModel")}
        showPrompt={false}
        showContextCount={false}
      />

      <ModelCard
        title={t("settings.voiceModel")}
        description={t("settings.voiceModelDesc")}
        modelKey="voiceModel"
        placeholder={placeholderText}
        modalTitle={t("settings.voiceModel")}
        showPrompt={false}
        showContextCount={false}
      />
    </div>
  );
}
