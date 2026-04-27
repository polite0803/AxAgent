import { ModelParamSliders } from "@/components/common/ModelParamSliders";
import { IconEditor } from "@/components/shared/IconEditor";
import { ModelSelect, parseModelValue } from "@/components/shared/ModelSelect";
import { useSettingsStore } from "@/stores";
import { Avatar, Divider, Input, Modal, theme, Typography } from "antd";
import { FolderOpen } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { TextArea } = Input;

export interface CategoryEditFormData {
  name: string;
  icon_type: string | null;
  icon_value: string | null;
  system_prompt: string | null;
  default_provider_id: string | null;
  default_model_id: string | null;
  default_temperature: number | null;
  default_max_tokens: number | null;
  default_top_p: number | null;
  default_frequency_penalty: number | null;
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
}: CategoryEditModalProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const settings = useSettingsStore((s) => s.settings);
  const [name, setName] = useState(initialName);
  const [iconType, setIconType] = useState<string | null>(initialIconType);
  const [iconValue, setIconValue] = useState<string | null>(initialIconValue);
  const [systemPrompt, setSystemPrompt] = useState(initialSystemPrompt ?? "");
  const [defaultProviderId, setDefaultProviderId] = useState<string | null>(initialDefaultProviderId);
  const [defaultModelId, setDefaultModelId] = useState<string | null>(initialDefaultModelId);
  const [defaultTemperature, setDefaultTemperature] = useState<number | null>(initialDefaultTemperature);
  const [defaultMaxTokens, setDefaultMaxTokens] = useState<number | null>(initialDefaultMaxTokens);
  const [defaultTopP, setDefaultTopP] = useState<number | null>(initialDefaultTopP);
  const [defaultFrequencyPenalty, setDefaultFrequencyPenalty] = useState<number | null>(initialDefaultFrequencyPenalty);

  useEffect(() => {
    if (open) {
      setName(initialName);
      setIconType(initialIconType ?? null);
      setIconValue(initialIconValue ?? null);
      setSystemPrompt(initialSystemPrompt ?? "");
      setDefaultProviderId(initialDefaultProviderId ?? null);
      setDefaultModelId(initialDefaultModelId ?? null);
      setDefaultTemperature(initialDefaultTemperature ?? null);
      setDefaultMaxTokens(initialDefaultMaxTokens ?? null);
      setDefaultTopP(initialDefaultTopP ?? null);
      setDefaultFrequencyPenalty(initialDefaultFrequencyPenalty ?? null);
    }
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
    ? `${defaultProviderId}::${defaultModelId}`
    : undefined;

  const handleDefaultModelChange = (value: string | undefined) => {
    const parsed = parseModelValue(value);
    setDefaultProviderId(parsed?.providerId ?? null);
    setDefaultModelId(parsed?.model_id ?? null);
  };

  const handleOk = () => {
    if (!name.trim()) { return; }
    onOk({
      name: name.trim(),
      icon_type: iconType,
      icon_value: iconValue,
      system_prompt: systemPrompt.trim() || null,
      default_provider_id: defaultProviderId,
      default_model_id: defaultModelId,
      default_temperature: defaultTemperature,
      default_max_tokens: defaultMaxTokens,
      default_top_p: defaultTopP,
      default_frequency_penalty: defaultFrequencyPenalty,
    });
    onClose();
  };

  return (
    <Modal
      title={title ?? t("chat.createCategory")}
      open={open}
      onCancel={onClose}
      onOk={handleOk}
      okButtonProps={{ disabled: !name.trim() }}
      destroyOnHidden
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
              style={{ cursor: "pointer", backgroundColor: token.colorFillSecondary, color: token.colorTextSecondary }}
            />
          }
        />

        <Input
          placeholder={t("chat.categoryNamePlaceholder")}
          value={name}
          onChange={(e) => setName(e.target.value)}
          onPressEnter={handleOk}
          autoFocus
          style={{ maxWidth: 340 }}
        />

        <TextArea
          placeholder={t("chat.categorySystemPromptPlaceholder", "System Prompt（分类下的对话将继承此提示词）")}
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
            onChange={(values) => {
              if ("temperature" in values) { setDefaultTemperature(values.temperature ?? null); }
              if ("topP" in values) { setDefaultTopP(values.topP ?? null); }
              if ("maxTokens" in values) { setDefaultMaxTokens(values.maxTokens ?? null); }
              if ("frequencyPenalty" in values) { setDefaultFrequencyPenalty(values.frequencyPenalty ?? null); }
            }}
            defaults={{
              temperature: settings.default_temperature ?? 0.7,
              topP: settings.default_top_p ?? 1,
              maxTokens: settings.default_max_tokens ?? 4096,
              frequencyPenalty: settings.default_frequency_penalty ?? 0,
            }}
          />
        </div>
      </div>
    </Modal>
  );
}
