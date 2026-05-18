import { Tag } from "antd";

export interface PlatformConfig {
  color: string;
  label: string;
}

export const DEFAULT_PLATFORM_CONFIGS: Record<string, PlatformConfig> = {
  telegram: { color: "#2AABEE", label: "TG" },
  discord: { color: "#5865F2", label: "DC" },
  api_server: { color: "#10B981", label: "API" },
  web: { color: "#F59E0B", label: "Web" },
  local: { color: "#8B5CF6", label: "Local" },
};

const FALLBACK_CONFIG: PlatformConfig = { color: "#6B7280", label: "???" };

interface GatewaySessionBadgeProps {
  platform: string;
  size?: "small" | "default";
  platforms?: Record<string, PlatformConfig>;
}

export function GatewaySessionBadge({
  platform,
  size = "default",
  platforms,
}: GatewaySessionBadgeProps) {
  const merged = { ...DEFAULT_PLATFORM_CONFIGS, ...platforms };
  const config = merged[platform] ?? FALLBACK_CONFIG;

  if (size === "small") {
    return (
      <span
        style={{
          display: "inline-block",
          width: 8,
          height: 8,
          borderRadius: "50%",
          backgroundColor: config.color,
          marginRight: 4,
        }}
        title={platform}
        aria-label={platform}
      />
    );
  }

  return (
    <Tag color={config.color} style={{ margin: 0 }} aria-label={platform}>
      {config.label}
    </Tag>
  );
}
