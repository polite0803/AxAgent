import { useHelpStore } from "@/stores/feature/helpStore";
import { theme, Tooltip } from "antd";
import { HelpCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useLocation } from "react-router-dom";

const PAGE_LABELS: Record<string, string> = {
  "/": "nav.chat",
  "/skills": "nav.skills",
  "/knowledge": "nav.knowledge",
  "/memory": "nav.memory",
  "/gateway": "nav.gateway",
  "/link": "nav.link",
  "/files": "nav.files",
  "/workflow": "nav.workflow",
  "/wiki": "nav.wiki",
};

function resolvePageLabel(pathname: string): string | null {
  if (PAGE_LABELS[pathname]) { return PAGE_LABELS[pathname]; }
  if (pathname.startsWith("/settings")) { return "nav.settings"; }
  if (pathname.startsWith("/skill/")) { return "nav.skills"; }
  if (pathname.startsWith("/devtools/")) { return "nav.devTools"; }
  if (pathname.startsWith("/llm-wiki")) { return "nav.wiki"; }
  if (pathname.startsWith("/wiki/")) { return "nav.wiki"; }
  return null;
}

export function AppHeader() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const location = useLocation();
  const toggleHelp = useHelpStore((s) => s.toggle);

  const labelKey = resolvePageLabel(location.pathname);

  return (
    <div
      style={{
        height: 40,
        minHeight: 40,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "0 16px",
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
        backgroundColor: "transparent",
      }}
    >
      <span
        style={{
          fontSize: 13,
          fontWeight: 500,
          color: token.colorText,
        }}
      >
        {labelKey ? t(labelKey) : t("nav.app")}
      </span>
      <Tooltip title={t("help.title", "帮助中心")}>
        <button
          type="button"
          onClick={toggleHelp}
          style={{
            border: "none",
            background: "transparent",
            cursor: "pointer",
            padding: 4,
            borderRadius: 4,
            display: "inline-flex",
            alignItems: "center",
            color: token.colorTextQuaternary,
          }}
        >
          <HelpCircle size={16} />
        </button>
      </Tooltip>
    </div>
  );
}
