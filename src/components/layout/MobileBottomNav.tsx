/** 移动端底部导航栏 — 仅在 mobile 布局显示 */
import { Icon } from "@/components/common/Icon";
import { useUIStore } from "@/stores";
import { theme } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";

interface BottomNavItem {
  key: string;
  icon: string;
  label: string;
  path: string;
}

export function MobileBottomNav() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const location = useLocation();
  const deviceLayout = useUIStore((s) => s.deviceLayout);

  if (deviceLayout !== "mobile") { return null; }

  const items: BottomNavItem[] = useMemo(
    () => [
      { key: "chat", icon: "fluent:chat-20-filled", label: t("nav.chat"), path: "/" },
      { key: "knowledge", icon: "fluent:book-database-20-filled", label: t("nav.knowledge"), path: "/knowledge" },
      { key: "gateway", icon: "fluent:globe-20-filled", label: t("nav.gateway"), path: "/gateway" },
      { key: "files", icon: "fluent:folder-20-filled", label: t("nav.files"), path: "/files" },
      { key: "settings", icon: "fluent:settings-20-filled", label: t("nav.settings"), path: "/settings" },
    ],
    [t],
  );

  const activeKey = useMemo(() => {
    if (location.pathname === "/" || location.pathname === "") { return "chat"; }
    return items.find((item) => location.pathname.startsWith(item.path))?.key ?? "chat";
  }, [location.pathname, items]);

  return (
    <nav
      style={{
        height: 48,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-around",
        flexShrink: 0,
        borderTop: `1px solid ${token.colorBorderSecondary}`,
        backgroundColor: token.colorBgContainer,
        paddingBottom: "env(safe-area-inset-bottom, 0px)",
      }}
    >
      {items.map((item) => {
        const active = item.key === activeKey;
        return (
          <button
            key={item.key}
            type="button"
            onClick={() => navigate(item.path)}
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: 1,
              padding: "2px 8px",
              border: "none",
              backgroundColor: "transparent",
              cursor: "pointer",
              color: active ? token.colorPrimary : token.colorTextQuaternary,
              minWidth: 0,
              flex: 1,
            }}
            aria-label={item.label}
            aria-current={active ? "page" : undefined}
          >
            <Icon icon={item.icon} size={20} />
            <span style={{ fontSize: 10, lineHeight: 1.2, whiteSpace: "nowrap" }}>
              {item.label}
            </span>
          </button>
        );
      })}
    </nav>
  );
}
