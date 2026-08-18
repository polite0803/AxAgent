// SPDX-License-Identifier: AGPL-3.0-only

import { Icon } from "@/components/common/Icon";
import { Tooltip } from "@/components/layout/Tooltip";
import { FEATURE_FLAGS } from "@/constants/featureFlags";
import { useResolvedAvatarSrc } from "@/hooks/useResolvedAvatarSrc";
import { CAPABILITY_DOMAIN_META } from "@/lib/domainMeta";
import { invoke, logIpcError } from "@/lib/invoke";
import { type NavItem, navItemsByDomain } from "@/lib/navRegistry";
import { BUILTIN_PAGE_PATH } from "@/lib/pageRegistry";
import { formatShortcutForDisplay, getShortcutBinding } from "@/lib/shortcuts";
import type { ShortcutAction } from "@/lib/shortcuts";
import {
  useAgentPanelStore,
  useOnboardingStore,
  useSettingsStore,
  useUIStore,
  useUserProfileStore,
  useWorkspaceStore,
} from "@/stores";
import type { AppSettings, PageKey } from "@/types";
import { MenuFoldOutlined, MenuUnfoldOutlined } from "@ant-design/icons";
import { Avatar } from "antd";
import { Globe, LineChart, Moon, Pin, PinOff, RotateCcw, Sun, User } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { UserProfileModal } from "./UserProfileModal";
/** 单一路径来源：直接复用 pageRegistry 的权威映射。 */
const pageKeyToPath = BUILTIN_PAGE_PATH as Record<PageKey, string>;

/** path→key 反查表：用于识别多段路径（如 /devtools/trace-explorer）对应的 PageKey。 */
const pathToPageKeyMap: Record<string, PageKey> = Object.fromEntries(
  Object.entries(BUILTIN_PAGE_PATH).map(([key, path]) => [path, key as PageKey]),
) as Record<string, PageKey>;

function pathToPageKey(path: string): PageKey {
  if (path === "/" || path === "") {
    return "dashboard";
  }
  // 优先按完整路径反查（覆盖 /devtools/xxx 这类多段路径）
  if (path in pathToPageKeyMap) {
    return pathToPageKeyMap[path];
  }
  const key = path.slice(1);
  if (key in pageKeyToPath) {
    return key as PageKey;
  }
  return "chat";
}

interface SidebarSection {
  key: string;
  labelKey: string;
  /** 域主题色（用于分组标题高亮点） */
  color?: string;
  /** 域聚合入口路径（点击分组标题跳转），无则不可点击 */
  domainPath?: string;
  items: NavItem[];
}

const NAV_SHORTCUT_MAP: Partial<Record<string, ShortcutAction>> = {
  // 网关已迁入设置页，侧栏不再保留快捷键映射
};

/**
 * Extracted component for rendering a navigation button.
 * Fixes react-doctor/no-render-in-render by moving renderNavButton() out of Sidebar.
 */
function NavItemButton({
  item,
  activePage,
  sidebarCollapsed,
  settings,
  onNavigate,
}: {
  item: NavItem;
  activePage: string;
  sidebarCollapsed: boolean;
  settings: AppSettings;
  onNavigate: (path: string) => void;
}) {
  const { t } = useTranslation();
  const location = useLocation();

  const isActive = item.isPlugin
    ? location.pathname === item.path
      || location.pathname.startsWith(item.path + "/")
    : activePage === item.key;
  const label = item.isPlugin ? item.labelKey : t(item.labelKey);
  const tooltipText = item.isPlugin ? `${label} (${item.pluginName})` : label;
  const action = !item.isPlugin && item.key in NAV_SHORTCUT_MAP
    ? NAV_SHORTCUT_MAP[item.key]
    : undefined;
  const shortcutLabel = action
    ? formatShortcutForDisplay(getShortcutBinding(settings, action))
    : "";
  const title = shortcutLabel
    ? `${tooltipText} (${shortcutLabel})`
    : tooltipText;

  const navClass = sidebarCollapsed
    ? `nav-item${isActive ? " active" : ""}`
    : `nav-item-expanded${isActive ? " active" : ""}`;

  return (
    <button
      type="button"
      onClick={() => onNavigate(item.path)}
      className={navClass}
      data-tutorial={item.key === "knowledge" ? "knowledge-nav" : undefined}
      aria-label={title}
      aria-current={isActive ? "page" : undefined}
    >
      {item.icon}
      {!sidebarCollapsed && (
        <span className="nav-label">
          {label}
        </span>
      )}
      {!sidebarCollapsed && shortcutLabel && (
        <span
          style={{
            marginLeft: "auto",
            fontSize: 10,
            color: "var(--color-text-secondary)",
            flexShrink: 0,
          }}
        >
          {shortcutLabel}
        </span>
      )}
    </button>
  );
}

/**
 * Extracted component for rendering the user avatar.
 * Fixes react-doctor/no-render-in-render by moving renderUserAvatar() out of Sidebar.
 */
function UserAvatarButton({
  profile,
  resolvedAvatarSrc,
}: {
  profile: { avatarType?: string; avatarValue?: string; name?: string };
  resolvedAvatarSrc: string | undefined;
}) {
  const size = 28;

  if (profile.avatarType === "emoji" && profile.avatarValue) {
    return (
      <div
        style={{
          width: size,
          height: size,
          borderRadius: "50%",
          backgroundColor: "var(--color-fill-secondary)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontSize: 14,
          cursor: "pointer",
        }}
      >
        {profile.avatarValue}
      </div>
    );
  }
  if (
    (profile.avatarType === "url" || profile.avatarType === "file")
    && profile.avatarValue
  ) {
    const src = profile.avatarType === "file" ? resolvedAvatarSrc : profile.avatarValue;
    return <Avatar size={size} src={src} style={{ cursor: "pointer" }} />;
  }
  return (
    <Avatar
      size={size}
      icon={<User size={14} />}
      style={{ cursor: "pointer", backgroundColor: "var(--color-primary)" }}
    />
  );
}

/** Mobile action buttons — mirrors TitleBar actions on Android where they get clipped */
function MobileActions() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const [pinned, setPinned] = useState(settings.alwaysOnTop ?? false);

  const deviceLayout = useUIStore((s) => s.deviceLayout);
  if (deviceLayout !== "mobile") { return null; }

  const togglePin = async () => {
    const next = !pinned;
    setPinned(next);
    try {
      await invoke("set_always_on_top", { enabled: next });
      saveSettings({ alwaysOnTop: next });
    } catch {
      setPinned(!next);
    }
  };

  const cycleTheme = () => {
    const next = settings.themeMode === "dark" ? "system" : settings.themeMode === "system" ? "light" : "dark";
    saveSettings({ themeMode: next }).catch(logIpcError("Sidebar: saveSettings(themeMode)"));
  };

  const ThemeIcon = settings.themeMode === "dark" ? Moon : settings.themeMode === "light" ? Sun : Globe;
  const btnBase: React.CSSProperties = {
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    width: 36,
    height: 36,
    borderRadius: 6,
    border: "none",
    backgroundColor: "transparent",
    cursor: "pointer",
    color: "var(--color-text-secondary)",
    transition: "color 0.15s",
  };

  return (
    <div
      style={{
        display: "flex",
        flexWrap: "wrap",
        gap: 2,
        justifyContent: "center",
        padding: "4px 0",
        borderTop: `1px solid ${"var(--color-border-secondary)"}`,
      }}
    >
      <Tooltip title={t("desktop.alwaysOnTop")} placement="right">
        <button style={btnBase} onClick={togglePin} aria-label={t("desktop.alwaysOnTop")} aria-pressed={pinned}>
          {pinned ? <Pin size={16} /> : <PinOff size={16} />}
        </button>
      </Tooltip>
      <Tooltip title={t("settings.groupTheme")} placement="right">
        <button style={btnBase} onClick={cycleTheme} aria-label={t("settings.groupTheme")}>
          <ThemeIcon size={16} />
        </button>
      </Tooltip>
      <Tooltip title={t("desktop.reloadPage")} placement="right">
        <button style={btnBase} onClick={() => window.location.reload()} aria-label={t("desktop.reloadPage")}>
          <RotateCcw size={16} />
        </button>
      </Tooltip>
    </div>
  );
}

export function Sidebar() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const activePage = pathToPageKey(location.pathname);
  const profile = useUserProfileStore((s) => s.profile);
  const [profileModalOpen, setProfileModalOpen] = useState(false);
  const resolvedAvatarSrc = useResolvedAvatarSrc(
    profile.avatarType,
    profile.avatarValue,
  );
  const settings = useSettingsStore((s) => s.settings);
  const sidebarCollapsed = useUIStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useUIStore((s) => s.toggleSidebar);
  const toggleHelp = useOnboardingStore((s) => s.toggle);
  const toggleAgentPanel = useAgentPanelStore((s) => s.toggle);
  const isAgentPanelOpen = useAgentPanelStore((s) => s.isOpen);
  const agentInTheLoopEnabled = FEATURE_FLAGS.AGENT_IN_THE_LOOP;
  const recentStocks = useWorkspaceStore((s) => s.recentStocks);

  const sections = useMemo<SidebarSection[]>(() => {
    const sections: SidebarSection[] = [];

    // 侧栏以「能力域」为组织轴：8 个业务域分组，域内导航项按业务本质归域。
    // 每个域分组标题可点击，跳转到该域的聚合入口页（DomainHub）。
    for (const domain of CAPABILITY_DOMAIN_META) {
      const items = navItemsByDomain(domain.id);
      if (items.length === 0) {
        continue;
      }
      sections.push({
        key: domain.id,
        labelKey: domain.labelKey,
        color: domain.color,
        domainPath: domain.path,
        items,
      });
    }

    return sections;
  }, []);

  // 动态固定页面已移至设置/扩展分组管理，不再在侧栏加载和渲染

  return (
    <>
      {/* Collapse toggle */}
      <button
        type="button"
        className="ax-sidebar-toggle"
        onClick={toggleSidebar}
        aria-label={sidebarCollapsed ? t("sidebar.expand") : t("sidebar.collapse")}
        aria-expanded={!sidebarCollapsed}
        style={{ color: "var(--color-text-secondary)" }}
      >
        {sidebarCollapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
      </button>

      {sections.map((section) => (
        <div key={section.key}>
          {!sidebarCollapsed && (
            <button
              type="button"
              className="ax-sidebar-section-header"
              onClick={() => section.domainPath && navigate(section.domainPath)}
              aria-label={t(section.labelKey)}
              style={{
                width: "100%",
                textAlign: "left",
                background: "none",
                border: "none",
                cursor: section.domainPath ? "pointer" : "default",
                padding: "2px 8px",
                fontSize: 11,
                fontWeight: 600,
                letterSpacing: "0.03em",
                color: "var(--muted)",
                borderRadius: 4,
              }}
              onMouseEnter={(e) => {
                if (section.domainPath) { e.currentTarget.style.backgroundColor = "var(--accent-bg)"; }
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = "transparent";
              }}
            >
              <span
                style={{
                  display: "inline-block",
                  width: 6,
                  height: 6,
                  borderRadius: "50%",
                  backgroundColor: section.color,
                  marginRight: 6,
                  verticalAlign: "middle",
                }}
              />
              <span style={{ verticalAlign: "middle" }}>{t(section.labelKey)}</span>
            </button>
          )}
          {section.items.map((item) => {
            const label = item.isPlugin ? item.labelKey : t(item.labelKey);
            const tooltipText = item.isPlugin
              ? `${label} (${item.pluginName})`
              : label;
            return (
              <Tooltip
                key={item.key}
                title={sidebarCollapsed ? tooltipText : ""}
                placement="right"
              >
                <NavItemButton
                  item={item}
                  activePage={activePage}
                  sidebarCollapsed={sidebarCollapsed}
                  settings={settings}
                  onNavigate={navigate}
                />
              </Tooltip>
            );
          })}
          {/* 金融域分组：展开时显示最近股票快捷列表 */}
          {section.key === "finance" && !sidebarCollapsed && recentStocks.length > 0 && (
            <div className="mt-0.5 mb-1 space-y-0.5">
              {!sidebarCollapsed && (
                <div
                  className="text-sm px-3 pt-1 pb-0.5"
                  style={{ color: "var(--muted)", fontSize: 11 }}
                >
                  {t("workspace.stockSwitcher.recent")}
                </div>
              )}
              {recentStocks.slice(0, 5).map((stock) => (
                <button
                  key={stock.code}
                  type="button"
                  onClick={() =>
                    navigate(
                      `${BUILTIN_PAGE_PATH.financeInvestment}?tab=workspace&stockCode=${stock.code}`,
                    )}
                  className="w-full flex items-center gap-1.5 px-3 py-1 rounded text-left transition-colors hover:opacity-70"
                  style={{ color: "var(--color-text-secondary)" }}
                >
                  <LineChart size={12} style={{ color: "var(--muted)", flexShrink: 0 }} />
                  <span className="text-sm truncate flex-1">{stock.name}</span>
                  <span className="text-sm font-mono" style={{ color: "var(--muted)", fontSize: 11 }}>
                    {stock.code}
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      ))}

      {/* 动态固定页面已移至设置/扩展分组管理，不再在侧栏显示 */}

      <div className="flex-1" />

      {/* Agent Panel toggle */}
      {agentInTheLoopEnabled && (
        <Tooltip
          title={sidebarCollapsed ? (isAgentPanelOpen ? t("sidebar.closeAgent") : t("sidebar.openAgent")) : ""}
          placement="right"
        >
          <button
            type="button"
            className={`nav-item${isAgentPanelOpen ? " active" : ""}`}
            onClick={toggleAgentPanel}
            aria-label={t("sidebar.agentPanel")}
            style={isAgentPanelOpen ? { color: "var(--color-primary)" } : undefined}
          >
            <Icon icon="fluent:bot-20-filled" size={17} />
            {!sidebarCollapsed && <span className="nav-label">Agent</span>}
          </button>
        </Tooltip>
      )}

      {/* Settings — lower group, above plugins in prototype */}
      <Tooltip title={sidebarCollapsed ? t("settings.openSettings") : ""} placement="right">
        <button
          type="button"
          className={`nav-item${activePage === "settings" ? " active" : ""}`}
          onClick={() => navigate("/settings")}
          aria-label={t("settings.openSettings")}
        >
          <Icon icon="fluent:settings-20-filled" size={17} />
          {!sidebarCollapsed && <span className="nav-label">{t("settings.openSettings")}</span>}
        </button>
      </Tooltip>
      {/* Mobile action buttons (TitleBar actions on Android) */}
      <MobileActions />

      {/* Help button */}
      <Tooltip title={sidebarCollapsed ? t("help.title") : ""} placement="right">
        <button
          type="button"
          className="nav-item"
          onClick={toggleHelp}
          aria-label={t("help.title")}
        >
          <Icon icon="fluent:question-circle-20-filled" size={17} />
          {!sidebarCollapsed && <span className="nav-label">{t("help.title")}</span>}
        </button>
      </Tooltip>

      <Tooltip
        title={sidebarCollapsed ? profile.name || t("userProfile.title") : ""}
        placement="right"
      >
        <button
          type="button"
          className="ax-sidebar-user"
          onClick={() => setProfileModalOpen(true)}
          aria-label={t("userProfile.title")}
        >
          <UserAvatarButton
            profile={profile}
            resolvedAvatarSrc={resolvedAvatarSrc}
          />
          {!sidebarCollapsed && (
            <span
              className="sidebar-user-name"
              style={{
                fontSize: 13,
                color: "var(--color-text-secondary)",
              }}
            >
              {profile.name || t("userProfile.title")}
            </span>
          )}
        </button>
      </Tooltip>

      <UserProfileModal
        open={profileModalOpen}
        onClose={() => setProfileModalOpen(false)}
      />
    </>
  );
}
