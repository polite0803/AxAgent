import { useExpertStore } from "@/stores/feature/expertStore";
import { Tooltip } from "antd";
import { ChevronDown } from "lucide-react";

interface ExpertBadgeProps {
  expertRoleId: string | null;
  onClick: () => void;
}

export function ExpertBadge({ expertRoleId, onClick }: ExpertBadgeProps) {
  const getRoleById = useExpertStore((s) => s.getRoleById);

  const role = expertRoleId ? getRoleById(expertRoleId) : null;

  if (!role) {
    return (
      <Tooltip title="选择专家角色">
        <button
          onClick={onClick}
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 4,
            padding: "2px 8px",
            borderRadius: 6,
            border: "1px dashed var(--color-border-tertiary)",
            background: "transparent",
            cursor: "pointer",
            fontSize: 12,
            color: "var(--color-text-secondary)",
            transition: "all 0.15s",
          }}
        >
          <span>{"\uD83E\uDD16"}</span>
          <span>通用助手</span>
          <ChevronDown size={12} />
        </button>
      </Tooltip>
    );
  }

  return (
    <Tooltip title={role.description}>
      <button
        onClick={onClick}
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 4,
          padding: "2px 8px",
          borderRadius: 6,
          border: "1px solid var(--color-border-info)",
          background: "var(--color-background-info)",
          cursor: "pointer",
          fontSize: 12,
          color: "var(--color-text-primary)",
          transition: "all 0.15s",
        }}
      >
        <span>{role.icon}</span>
        <span>{role.displayName}</span>
        <ChevronDown size={12} />
      </button>
    </Tooltip>
  );
}
