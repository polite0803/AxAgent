// SPDX-License-Identifier: AGPL-3.0-only

import { Icon } from "@/components/common/Icon";
import { Tooltip } from "@/components/layout/Tooltip";
import { useResolvedAvatarSrc } from "@/hooks/useResolvedAvatarSrc";
import { NAV_ICON_COLORS } from "@/lib/iconColors";
import { invoke } from "@/lib/invoke";
import { formatShortcutForDisplay, getShortcutBinding } from "@/lib/shortcuts";
import type { ShortcutAction } from "@/lib/shortcuts";
import { resolveIconComponent } from "@/lib/skillIcons";
import {
  useOnboardingStore,
  useSettingsStore,
  useSkillExtensionStore,
  useUIStore,
  useUserProfileStore,
} from "@/stores";
import type { AppSettings, PageKey } from "@/types";
import { MenuFoldOutlined, MenuUnfoldOutlined } from "@ant-design/icons";
import { Avatar } from "antd";
import { Globe, Moon, Pin, PinOff, RotateCcw, Settings, Sun, User } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { UserProfileModal } from "./UserProfileModal";
const pageKeyToPath: Record<PageKey, string> = {
  chat: "/",
  knowledge: "/knowledge",
  memory: "/memory",
  link: "/link",
  gateway: "/gateway",
  files: "/files",
  terminal: "/terminal",
  workflow: "/workflow",
  settings: "/settings",
};

function pathToPageKey(path: string): PageKey {
  if (path === "/" || path === "") {
    return "chat";
  }
  if (path.startsWith("/skill/")) {
    return path;
  }
  const key = path.slice(1);
  if (key in pageKeyToPath) {
    return key as PageKey;
  }
  return "chat";
}

interface NavItem {
  key: string;
  icon: React.ReactNode;
  labelKey: string;
  path: string;
  isPlugin: boolean;
  pluginName?: string;
}

const builtinNavItems: NavItem[] = [
  {
    key: "chat",
    icon: <Icon icon="fluent:chat-20-filled" size={17} />,
    labelKey: "nav.chat",
    path: "/",
    isPlugin: false,
  },
  {
    key: "knowledge",
    icon: <Icon icon="fluent:book-database-20-filled" size={17} />,
    labelKey: "nav.knowledge",
    path: "/knowledge",
    isPlugin: false,
  },
  {
    key: "gateway",
    icon: <Icon icon="fluent:globe-20-filled" size={17} />,
    labelKey: "nav.gateway",
    path: "/gateway",
    isPlugin: false,
  },
  {
    key: "terminal",
    icon: <Icon icon="fluent:prompt-20-filled" size={17} />,
    labelKey: "nav.terminal",
    path: "/terminal",
    isPlugin: false,
  },
  {
    key: "files",
    icon: <Icon icon="fluent:folder-20-filled" size={17} />,
    labelKey: "nav.files",
    path: "/files",
    isPlugin: false,
  },
  {
    key: "workflow",
    icon: <Icon icon="fluent:flow-20-filled" size={17} />,
    labelKey: "nav.workflow",
    path: "/workflow",
    isPlugin: false,
  },
];

interface SidebarSection {
  key: string;
  labelKey: string;
  items: NavItem[];
}

const NAV_SHORTCUT_MAP: Partial<Record<string, ShortcutAction>> = {
  gateway: "toggleGateway",
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
  const navigate = useNavigate();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const [pinned, setPinned] = useState(settings.always_on_top ?? false);

  const deviceLayout = useUIStore((s) => s.deviceLayout);
  if (deviceLayout !== "mobile") { return null; }

  const togglePin = async () => {
    const next = !pinned;
    setPinned(next);
    try {
      await invoke("set_always_on_top", { enabled: next });
      saveSettings({ always_on_top: next });
    } catch {
      setPinned(!next);
    }
  };

  const cycleTheme = () => {
    const next = settings.theme_mode === "dark" ? "system" : settings.theme_mode === "system" ? "light" : "dark";
    saveSettings({ theme_mode: next });
  };

  const ThemeIcon = settings.theme_mode === "dark" ? Moon : settings.theme_mode === "light" ? Sun : Globe;
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
        <button style={btnBase} onClick={togglePin}>
          {pinned ? <Pin size={16} /> : <PinOff size={16} />}
        </button>
      </Tooltip>
      <Tooltip title={t("settings.groupTheme")} placement="right">
        <button style={btnBase} onClick={cycleTheme}>
          <ThemeIcon size={16} />
        </button>
      </Tooltip>
      <Tooltip title={t("desktop.reloadPage")} placement="right">
        <button style={btnBase} onClick={() => window.location.reload()}>
          <RotateCcw size={16} />
        </button>
      </Tooltip>
      <Tooltip title={t("settings.openSettings")} placement="right">
        <button style={btnBase} onClick={() => navigate("/settings")}>
          <Settings size={16} />
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
  const skillNavItems = useSkillExtensionStore((s) => s.navItems);
  const sidebarCollapsed = useUIStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useUIStore((s) => s.toggleSidebar);
  const toggleHelp = useOnboardingStore((s) => s.toggle);

  const sections = useMemo<SidebarSection[]>(() => {
    const pluginItems: NavItem[] = [];
    for (const item of skillNavItems) {
      const IconComp = resolveIconComponent(item.icon);
      pluginItems.push({
        key: `plugin:${item.id}`,
        // eslint-disable-next-line react-hooks/static-components
        icon: <IconComp size={18} color={NAV_ICON_COLORS.Router} />,
        labelKey: item.label,
        path: `/skill/${item.skillName}/${item.pageId}`,
        isPlugin: true,
        pluginName: item.skillName,
      });
    }

    const topPlugins = pluginItems.filter((i) => {
      const orig = skillNavItems.find((n) => `plugin:${n.id}` === i.key);
      return (orig?.position ?? 1) === 0;
    });
    const bottomPlugins = pluginItems.filter((i) => {
      const orig = skillNavItems.find((n) => `plugin:${n.id}` === i.key);
      return (orig?.position ?? 1) !== 0;
    });

    const sections: SidebarSection[] = [];

    if (topPlugins.length > 0) {
      sections.push({
        key: "work",
        labelKey: "sidebar.sectionWork",
        items: [
          ...topPlugins,
          ...builtinNavItems.filter((n) => n.key === "chat"),
        ],
      });
    } else {
      sections.push({
        key: "work",
        labelKey: "sidebar.sectionWork",
        items: [builtinNavItems.find((n) => n.key === "chat")!],
      });
    }

    sections.push({
      key: "tools",
      labelKey: "sidebar.sectionTools",
      items: builtinNavItems.filter((n) => n.key === "knowledge"),
    });

    sections.push({
      key: "infrastructure",
      labelKey: "sidebar.sectionInfrastructure",
      items: builtinNavItems.filter((n) =>
        n.key === "gateway" || n.key === "terminal" || n.key === "files" || n.key === "workflow"
      ),
    });

    if (bottomPlugins.length > 0) {
      sections.push({
        key: "plugins",
        labelKey: "sidebar.sectionPlugins",
        items: bottomPlugins,
      });
    }

    return sections.filter((s) => s.items.length > 0);
  }, [skillNavItems]);

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
            <div className="ax-sidebar-section-header">
              {t(section.labelKey)}
            </div>
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
        </div>
      ))}

      <div className="flex-1" />

      {/* Settings — lower group, above plugins in prototype */}
      <Tooltip title={sidebarCollapsed ? t("settings.openSettings") : ""} placement="right">
        <button
          type="button"
          className={`nav-item${activePage === "settings" ? " active" : ""}`}
          onClick={() => navigate("/settings")}
          aria-label={t("settings.openSettings")}
        >
          <Icon icon="fluent:settings-20-filled" size={17} />
        </button>
      </Tooltip>

      {/* Mobile action buttons (TitleBar actions on Android) */}
      <MobileActions />

      {/* Help button */}
      <Tooltip title={t("help.title")} placement="right">
        <button
          type="button"
          className="nav-item"
          onClick={toggleHelp}
          aria-label={t("help.title")}
        >
          <Icon icon="fluent:question-circle-20-filled" size={17} />
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
