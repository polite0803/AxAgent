import { Tooltip } from "@/components/layout/Tooltip";
import { useExpertStore } from "@/stores/feature/expertStore";
import { ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";

interface ExpertBadgeProps {
  expertRoleId: string | null;
  onClick: () => void;
}

export function ExpertBadge({ expertRoleId, onClick }: ExpertBadgeProps) {
  const getRoleById = useExpertStore((s) => s.getRoleById);
  const { t } = useTranslation();

  const role = expertRoleId ? getRoleById(expertRoleId) : null;

  if (!role) {
    return (
      <Tooltip title={t("expertBadge.selectExpert")}>
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
            transition: "box-shadow 0.15s, transform 0.15s",
          }}
        >
          <span>{"\uD83E\uDD16"}</span>
          <span>{t("expertBadge.generalAssistant")}</span>
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
          transition: "box-shadow 0.15s, transform 0.15s",
        }}
      >
        <span>{role.icon}</span>
        <span>{role.name}</span>
        <ChevronDown size={12} />
      </button>
    </Tooltip>
  );
}
