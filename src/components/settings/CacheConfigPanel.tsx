import { Switch, Typography } from "antd";
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
  return (
    <div className="p-6">
      <SettingsGroup title="Prompt Cache">
        <div className="flex items-center justify-between">
          <span>Enable Cache Breakpoints</span>
          <Switch
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
