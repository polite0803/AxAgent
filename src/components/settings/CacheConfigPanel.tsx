import { Switch, Typography } from "antd";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const { Text } = Typography;

interface CacheConfigPanelProps {
  enableCacheBreakpoints: boolean;
  onToggleCacheBreakpoints: (enabled: boolean) => void;
}

export function CacheConfigPanel({
  enableCacheBreakpoints,
  onToggleCacheBreakpoints,
}: CacheConfigPanelProps) {
  const { t } = useTranslation();
  return (
    <div className="p-6">
      <SettingsGroup title={t("cacheConfig.title")}>
        <div className="flex items-center justify-between">
          <span>{t("cacheConfig.enableBreakpoints")}</span>
          <Switch
            id="cache-config-panel-switch-41"
            checked={enableCacheBreakpoints}
            onChange={onToggleCacheBreakpoints}
          />
        </div>
        <div className="mt-2">
          <Text type="secondary">
            When enabled, the system prompt is cached by the LLM provider to reduce token usage on subsequent turns.
            Changes to skills, tools, or memory are deferred to the next session. Use --now to force immediate
            application.
          </Text>
        </div>
      </SettingsGroup>
    </div>
  );
}
