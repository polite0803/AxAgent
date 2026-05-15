import { useHelpStore } from "@/stores/feature/helpStore";
import { theme, Tooltip } from "antd";
import { HelpCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useLocation } from "react-router-dom";

const PAGE_LABELS: Record<string, string> = {
  "/": "nav.chat",
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
  if (pathname.startsWith("/skill/")) { return "settings.skillsHub"; }
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
      className="ax-cyber-border"
      style={{
        height: 40,
        minHeight: 40,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "0 16px",
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
        backgroundColor: "transparent",
        position: "relative",
      }}
    >
      <span
        className={labelKey ? "" : ""}
        style={{
          fontSize: 13,
          fontWeight: 600,
          color: token.colorText,
          letterSpacing: "0.04em",
        }}
      >
        {labelKey ? t(labelKey) : t("nav.app")}
      </span>
      <Tooltip title={t("help.title")}>
        <button
          type="button"
          onClick={toggleHelp}
          className="ax-titlebar-btn"
          aria-label={t("help.title")}
          style={{
            color: token.colorTextQuaternary,
          }}
        >
          <HelpCircle size={16} />
        </button>
      </Tooltip>
    </div>
  );
}
