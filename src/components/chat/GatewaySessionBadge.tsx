import { Tag } from "antd";

const platformColors: Record<string, string> = {
  telegram: "#2AABEE",
  discord: "#5865F2",
  api_server: "#10B981",
  web: "#F59E0B",
  local: "#8B5CF6",
};

const platformLabels: Record<string, string> = {
  telegram: "TG",
  discord: "DC",
  api_server: "API",
  web: "Web",
  local: "Local",
};

interface GatewaySessionBadgeProps {
  platform: string;
  size?: "small" | "default";
}

export function GatewaySessionBadge({ platform, size = "default" }: GatewaySessionBadgeProps) {
  const color = platformColors[platform] ?? "#6B7280";
  const label = platformLabels[platform] ?? platform;

  if (size === "small") {
    return (
      <span
        style={{
          display: "inline-block",
          width: 8,
          height: 8,
          borderRadius: "50%",
          backgroundColor: color,
          marginRight: 4,
        }}
        title={platform}
      />
    );
  }

  return (
    <Tag color={color} style={{ margin: 0 }}>
      {label}
    </Tag>
  );
}
