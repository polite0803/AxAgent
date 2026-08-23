// SPDX-License-Identifier: AGPL-3.0-only

import { ModelParamSliders, type ModelParamValues } from "@/components/common/ModelParamSliders";
import { IconEditor } from "@/components/shared/IconEditor";
import { ModelSelect, parseModelValue } from "@/components/shared/ModelSelect";
import { safeJoinIds } from "@/lib/validators";
import { useSettingsStore } from "@/stores";
import { Avatar, Divider, Input, Modal, theme, Typography } from "antd";
import { FolderOpen } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

export interface CategoryEditFormData {
  name: string;
  iconType: string | null;
  iconValue: string | null;
  systemPrompt: string | null;
  defaultProviderId: string | null;
  defaultModelId: string | null;
  defaultTemperature: number | null;
  defaultMaxTokens: number | null;
  defaultTopP: number | null;
  defaultFrequencyPenalty: number | null;
}

interface CategoryEditModalProps {
  open: boolean;
  onClose: () => void;
  onOk: (data: CategoryEditFormData) => void;
  initialName?: string;
  initialIconType?: string | null;
  initialIconValue?: string | null;
  initialSystemPrompt?: string | null;
  initialDefaultProviderId?: string | null;
  initialDefaultModelId?: string | null;
  initialDefaultTemperature?: number | null;
  initialDefaultMaxTokens?: number | null;
  initialDefaultTopP?: number | null;
  initialDefaultFrequencyPenalty?: number | null;
  title?: string;
  confirmLoading?: boolean;
}

export function CategoryEditModal({
  open,
  onClose,
  onOk,
  initialName = "",
  initialIconType = null,
  initialIconValue = null,
  initialSystemPrompt = null,
  initialDefaultProviderId = null,
  initialDefaultModelId = null,
  initialDefaultTemperature = null,
  initialDefaultMaxTokens = null,
  initialDefaultTopP = null,
  initialDefaultFrequencyPenalty = null,
  title,
  confirmLoading = false,
}: CategoryEditModalProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const settings = useSettingsStore((s) => s.settings);

  const [name, setName] = useState("");
  const [iconType, setIconType] = useState<string | null>(null);
  const [iconValue, setIconValue] = useState<string | null>(null);
  const [systemPrompt, setSystemPrompt] = useState("");
  const [defaultProviderId, setDefaultProviderId] = useState<string | null>(
    null,
  );
  const [defaultModelId, setDefaultModelId] = useState<string | null>(null);
  const [defaultTemperature, setDefaultTemperature] = useState<number | null>(
    null,
  );
  const [defaultMaxTokens, setDefaultMaxTokens] = useState<number | null>(null);
  const [defaultTopP, setDefaultTopP] = useState<number | null>(null);
  const [defaultFrequencyPenalty, setDefaultFrequencyPenalty] = useState<
    number | null
  >(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setName(initialName);
    setIconType(initialIconType);
    setIconValue(initialIconValue);
    setSystemPrompt(initialSystemPrompt ?? "");
    setDefaultProviderId(initialDefaultProviderId);
    setDefaultModelId(initialDefaultModelId);
    setDefaultTemperature(initialDefaultTemperature);
    setDefaultMaxTokens(initialDefaultMaxTokens);
    setDefaultTopP(initialDefaultTopP);
    setDefaultFrequencyPenalty(initialDefaultFrequencyPenalty);
  }, [
    open,
    initialName,
    initialIconType,
    initialIconValue,
    initialSystemPrompt,
    initialDefaultProviderId,
    initialDefaultModelId,
    initialDefaultTemperature,
    initialDefaultMaxTokens,
    initialDefaultTopP,
    initialDefaultFrequencyPenalty,
  ]);

  const selectedModelValue = defaultProviderId && defaultModelId
    ? safeJoinIds([defaultProviderId, defaultModelId], "::")
    : undefined;

  const handleDefaultModelChange = useCallback((value: string | undefined) => {
    const parsed = parseModelValue(value);
    setDefaultProviderId(parsed?.providerId ?? null);
    setDefaultModelId(parsed?.modelId ?? null);
  }, []);

  const handleParamsChange = useCallback(
    (values: Partial<ModelParamValues>) => {
      const { temperature, topP, maxTokens, frequencyPenalty } = values;
      if (temperature !== undefined) {
        setDefaultTemperature(temperature ?? null);
      }
      if (topP !== undefined) {
        setDefaultTopP(topP ?? null);
      }
      if (maxTokens !== undefined) {
        setDefaultMaxTokens(maxTokens ?? null);
      }
      if (frequencyPenalty !== undefined) {
        setDefaultFrequencyPenalty(frequencyPenalty ?? null);
      }
    },
    [],
  );

  const handleOk = useCallback(() => {
    onOk({
      name: name.trim(),
      iconType: iconType,
      iconValue: iconValue,
      systemPrompt: systemPrompt.trim() || null,
      defaultProviderId: defaultProviderId,
      defaultModelId: defaultModelId,
      defaultTemperature: defaultTemperature,
      defaultMaxTokens: defaultMaxTokens,
      defaultTopP: defaultTopP,
      defaultFrequencyPenalty: defaultFrequencyPenalty,
    });
  }, [
    name,
    iconType,
    iconValue,
    systemPrompt,
    defaultProviderId,
    defaultModelId,
    defaultTemperature,
    defaultMaxTokens,
    defaultTopP,
    defaultFrequencyPenalty,
    onOk,
  ]);

  const canSubmit = name.trim().length > 0;

  return (
    <Modal
      title={title ?? t("chat.createCategory")}
      open={open}
      onCancel={onClose}
      onOk={handleOk}
      okButtonProps={{ disabled: !canSubmit || confirmLoading }}
      confirmLoading={confirmLoading}
      width={560}
      mask={{ enabled: true, blur: true }}
    >
      <div className="flex flex-col items-center gap-3 py-3">
        <IconEditor
          iconType={iconType}
          iconValue={iconValue}
          onChange={(type, value) => {
            setIconType(type);
            setIconValue(value);
          }}
          size={40}
          defaultIcon={
            <Avatar
              size={40}
              icon={<FolderOpen size={18} />}
              style={{
                cursor: "pointer",
                backgroundColor: token.colorFillSecondary,
                color: token.colorTextSecondary,
              }}
            />
          }
        />

        <Input
          placeholder={t("chat.categoryNamePlaceholder")}
          value={name}
          onChange={(e) => setName(e.target.value)}
          onPressEnter={handleOk}
          style={{ maxWidth: 340 }}
        />

        <Input.TextArea
          placeholder={t("chat.categorySystemPromptPlaceholder")}
          value={systemPrompt}
          onChange={(e) => setSystemPrompt(e.target.value)}
          autoSize={{ minRows: 5, maxRows: 10 }}
          style={{ maxWidth: 340 }}
        />

        <Divider style={{ margin: "4px 0 0" }} />

        <div style={{ width: "100%", maxWidth: 420 }}>
          <Typography.Text strong style={{ display: "block", marginBottom: 8 }}>
            {t("settings.defaultConversationModel")}
          </Typography.Text>
          <ModelSelect
            value={selectedModelValue}
            onChange={handleDefaultModelChange}
            placeholder={t("settings.useActiveModel")}
            style={{ width: "100%" }}
          />
        </div>

        <div style={{ width: "100%", maxWidth: 420 }}>
          <Typography.Text strong style={{ display: "block", marginBottom: 8 }}>
            {t("settings.modelParams")}
          </Typography.Text>
          <ModelParamSliders
            values={{
              temperature: defaultTemperature,
              topP: defaultTopP,
              maxTokens: defaultMaxTokens,
              frequencyPenalty: defaultFrequencyPenalty,
            }}
            onChange={handleParamsChange}
            defaults={{
              temperature: settings.defaultTemperature ?? 0.7,
              topP: settings.defaultTopP ?? 1,
              maxTokens: settings.defaultMaxTokens ?? 4096,
              frequencyPenalty: settings.defaultFrequencyPenalty ?? 0,
            }}
          />
        </div>
      </div>
    </Modal>
  );
}
