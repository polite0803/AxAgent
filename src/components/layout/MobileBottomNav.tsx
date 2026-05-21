/** 移动端闪现式浮动导航 — 点击展开，选完自动消失 */
import { Icon } from "@/components/common/Icon";
import { useUIStore } from "@/stores";
import { theme } from "antd";
import { Grid3X3 } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";

interface NavItem {
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
  const [expanded, setExpanded] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  // 点击外部关闭
  useEffect(() => {
    if (!expanded) {
      return;
    }
    const handleClick = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setExpanded(false);
      }
    };
    // delay to avoid the triggering click from closing it
    const timer = setTimeout(() => document.addEventListener("click", handleClick), 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("click", handleClick);
    };
  }, [expanded]);

  // 导航后自动关闭
  useEffect(() => {
    setExpanded(false);
  }, [location.pathname]);

  if (deviceLayout !== "mobile" && deviceLayout !== "tablet") { return null; }

  const items: NavItem[] = useMemo(
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

  const handleNavigate = useCallback(
    (path: string) => {
      navigate(path);
    },
    [navigate],
  );

  return (
    <div
      ref={panelRef}
      className="ax-safe-fixed-bottom"
      style={{
        position: "fixed",
        bottom: "calc(12px + env(safe-area-inset-bottom, 0px))",
        left: "50%",
        transform: "translateX(-50%)",
        zIndex: 900,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: 8,
      }}
    >
      {/* 展开面板 */}
      {expanded && (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 4,
            padding: "8px 6px",
            borderRadius: 14,
            backgroundColor: token.colorBgElevated,
            boxShadow: "0 8px 32px rgba(0,0,0,0.22)",
            border: `1px solid ${token.colorBorderSecondary}`,
            minWidth: 140,
            animation: "axFloatIn 0.18s ease-out",
          }}
        >
          {items.map((item) => {
            const active = item.key === activeKey;
            return (
              <button
                key={item.key}
                type="button"
                onClick={() => handleNavigate(item.path)}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 10,
                  padding: "8px 12px",
                  border: "none",
                  borderRadius: 8,
                  backgroundColor: active ? token.colorPrimaryBg : "transparent",
                  color: active ? token.colorPrimary : token.colorText,
                  cursor: "pointer",
                  fontSize: 14,
                  fontWeight: active ? 600 : 400,
                  whiteSpace: "nowrap",
                  transition: "background-color 0.1s",
                }}
              >
                <Icon icon={item.icon} size={18} />
                <span>{item.label}</span>
              </button>
            );
          })}
        </div>
      )}

      {/* 浮动触发按钮 */}
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          setExpanded((v) => !v);
        }}
        aria-label={t("sidebar.toggle")}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          gap: 6,
          height: 36,
          padding: "0 14px",
          border: `1px solid ${token.colorBorderSecondary}`,
          borderRadius: 20,
          backgroundColor: token.colorBgElevated,
          boxShadow: "0 2px 12px rgba(0,0,0,0.14)",
          color: token.colorTextSecondary,
          cursor: "pointer",
          fontSize: 12,
          fontWeight: 500,
        }}
      >
        <Grid3X3 size={16} />
        <span style={{ lineHeight: 1 }}>{t("nav.app")}</span>
      </button>
    </div>
  );
}
