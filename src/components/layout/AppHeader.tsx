import { useHelpStore } from "@/stores/feature/helpStore";
import { theme, Tooltip } from "antd";
import { ArrowLeft, HelpCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";

const PAGE_LABELS: Record<string, string> = {
  "/": "nav.chat",
  "/knowledge": "nav.knowledge",
  "/memory": "nav.memory",
  "/gateway": "nav.gateway",
  "/link": "nav.link",
  "/files": "nav.files",
  "/workflow": "nav.workflow",
  "/wiki": "nav.wiki",
  "/terminal": "nav.terminal",
};

function resolvePageLabel(pathname: string): string | null {
  if (PAGE_LABELS[pathname]) {
    return PAGE_LABELS[pathname];
  }
  if (pathname.startsWith("/settings")) {
    return "nav.settings";
  }
  if (pathname.startsWith("/skill/")) {
    return "settings.skillsHub";
  }
  if (pathname.startsWith("/devtools/")) {
    return "nav.devTools";
  }
  if (pathname.startsWith("/llm-wiki")) {
    return "nav.wiki";
  }
  if (pathname.startsWith("/wiki/")) {
    return "nav.wiki";
  }
  return null;
}

/** 非对话页面的上下文摘要 — 后续通过 store 动态注入实际数值 */
function getPageContext(pathname: string, t: (k: string) => string): string | null {
  if (pathname.startsWith("/knowledge")) { return t("appHeader.knowledgeContext"); }
  if (pathname.startsWith("/gateway")) { return t("appHeader.gatewayContext"); }
  if (pathname.startsWith("/files")) { return t("appHeader.filesContext"); }
  if (pathname.startsWith("/terminal")) { return t("appHeader.terminalContext"); }
  return null;
}

export function AppHeader() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const location = useLocation();
  const navigate = useNavigate();
  const toggleHelp = useHelpStore((s) => s.toggle);

  const isChatPage = location.pathname === "/" || location.pathname === "";
  const labelKey = resolvePageLabel(location.pathname);
  const contextSummary = getPageContext(location.pathname, t);

  // 对话页不显示 AppHeader（对话页有自己的 TabBar）
  if (isChatPage) { return null; }

  return (
    <div
      className="ax-cyber-border"
      style={{
        height: 40,
        minHeight: 40,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "0 12px",
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
        backgroundColor: "transparent",
        position: "relative",
        gap: 8,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8, flex: 1, minWidth: 0 }}>
        {/* 返回对话 */}
        <Tooltip title={t("common.back")}>
          <button
            type="button"
            onClick={() => navigate("/")}
            className="ax-titlebar-btn"
            aria-label={t("common.back")}
            style={{ color: token.colorTextQuaternary, flexShrink: 0 }}
          >
            <ArrowLeft size={15} />
          </button>
        </Tooltip>

        {/* 页面名 */}
        <span
          style={{
            fontSize: 13,
            fontWeight: 600,
            color: token.colorText,
            letterSpacing: "0.03em",
            whiteSpace: "nowrap",
          }}
        >
          {labelKey ? t(labelKey) : t("nav.app")}
        </span>

        {/* 上下文摘要 */}
        {contextSummary && (
          <span
            style={{
              fontSize: 11,
              color: token.colorTextQuaternary,
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            {contextSummary}
          </span>
        )}
      </div>

      <Tooltip title={t("help.title")}>
        <button
          type="button"
          onClick={toggleHelp}
          className="ax-titlebar-btn"
          aria-label={t("help.title")}
          style={{ color: token.colorTextQuaternary, flexShrink: 0 }}
        >
          <HelpCircle size={16} />
        </button>
      </Tooltip>
    </div>
  );
}
