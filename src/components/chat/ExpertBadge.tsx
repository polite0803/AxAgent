import { Tooltip } from "@/components/layout/Tooltip";
import { useAgentProfileStore } from "@/stores/feature/agentProfileStore";
import { ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";

interface ExpertBadgeProps {
  agentProfileId: string | null;
  onClick: () => void;
}

export function ExpertBadge({ agentProfileId, onClick }: ExpertBadgeProps) {
  const getProfileById = useAgentProfileStore((s) => s.getProfileById);
  const { t } = useTranslation();

  const profile = agentProfileId ? getProfileById(agentProfileId) : null;

  if (!profile) {
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
          <span>{"🤖"}</span>
          <span>{t("expertBadge.generalAssistant")}</span>
          <ChevronDown size={12} />
        </button>
      </Tooltip>
    );
  }

  return (
    <Tooltip title={profile.description || ""}>
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
        <span>{profile.icon}</span>
        <span>{profile.name}</span>
        <ChevronDown size={12} />
      </button>
    </Tooltip>
  );
}
