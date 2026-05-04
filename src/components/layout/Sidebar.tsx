import { useResolvedAvatarSrc } from "@/hooks/useResolvedAvatarSrc";
import { NAV_ICON_COLORS } from "@/lib/iconColors";
import { formatShortcutForDisplay, getShortcutBinding } from "@/lib/shortcuts";
import type { ShortcutAction } from "@/lib/shortcuts";
import { resolveIconComponent } from "@/lib/skillIcons";
import { useSettingsStore, useSkillExtensionStore, useUserProfileStore } from "@/stores";
import type { PageKey } from "@/types";
import { Avatar, theme, Tooltip } from "antd";
import { Database, MessageSquare, Router, Sparkles, User } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { UserProfileModal } from "./UserProfileModal";

const pageKeyToPath: Record<PageKey, string> = {
  chat: "/",
  skills: "/skills",
  knowledge: "/knowledge",
  memory: "/memory",
  link: "/link",
  gateway: "/gateway",
  files: "/files",
  settings: "/settings",
};

function pathToPageKey(path: string): PageKey {
  if (path === "/" || path === "") { return "chat"; }
  if (path.startsWith("/skill/")) { return path; }
  const key = path.slice(1);
  if (key in pageKeyToPath) { return key as PageKey; }
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
    icon: <MessageSquare size={18} color={NAV_ICON_COLORS.MessageSquare} />,
    labelKey: "nav.chat",
    path: "/",
    isPlugin: false,
  },
  {
    key: "skills",
    icon: <Sparkles size={18} color={NAV_ICON_COLORS.Sparkles} />,
    labelKey: "nav.skills",
    path: "/skills",
    isPlugin: false,
  },
  {
    key: "knowledge",
    icon: <Database size={18} color={NAV_ICON_COLORS.Database} />,
    labelKey: "nav.knowledge",
    path: "/knowledge",
    isPlugin: false,
  },
  {
    key: "gateway",
    icon: <Router size={18} color={NAV_ICON_COLORS.Router} />,
    labelKey: "nav.gateway",
    path: "/gateway",
    isPlugin: false,
  },
];

interface SidebarSection {
  key: string;
  labelKey: string;
  items: NavItem[];
}

const SIDEBAR_WIDTH = 240;

export function Sidebar() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const location = useLocation();
  const activePage = pathToPageKey(location.pathname);
  const profile = useUserProfileStore((s) => s.profile);
  const [profileModalOpen, setProfileModalOpen] = useState(false);
  const resolvedAvatarSrc = useResolvedAvatarSrc(profile.avatarType, profile.avatarValue);
  const settings = useSettingsStore((s) => s.settings);
  const skillNavItems = useSkillExtensionStore((s) => s.navItems);

  const sections = useMemo<SidebarSection[]>(() => {
    const pluginItems: NavItem[] = [];
    for (const item of skillNavItems) {
      const IconComp = resolveIconComponent(item.icon);
      pluginItems.push({
        key: `plugin:${item.id}`,
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
        items: [...topPlugins, ...builtinNavItems.filter((n) => n.key === "chat")],
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
      items: builtinNavItems.filter((n) => n.key === "skills" || n.key === "knowledge"),
    });

    sections.push({
      key: "infrastructure",
      labelKey: "sidebar.sectionInfrastructure",
      items: builtinNavItems.filter((n) => n.key === "gateway"),
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

  const NAV_SHORTCUT_MAP: Partial<Record<string, ShortcutAction>> = {
    gateway: "toggleGateway",
  };

  const renderNavButton = (item: NavItem) => {
    const isActive = item.isPlugin
      ? location.pathname === item.path || location.pathname.startsWith(item.path + "/")
      : activePage === item.key;
    const label = item.isPlugin ? item.labelKey : t(item.labelKey);
    const tooltipText = item.isPlugin ? `${label} (${item.pluginName})` : label;
    const action = !item.isPlugin && item.key in NAV_SHORTCUT_MAP
      ? NAV_SHORTCUT_MAP[item.key]
      : undefined;
    const shortcutLabel = action
      ? formatShortcutForDisplay(getShortcutBinding(settings, action))
      : "";
    const title = shortcutLabel ? `${tooltipText} (${shortcutLabel})` : tooltipText;

    return (
      <Tooltip key={item.key} title={title} placement="right">
        <div
          onClick={() => navigate(item.path)}
          className={isActive ? "ax-nav-item-active" : ""}
          style={{
            display: "flex",
            alignItems: "center",
            height: 32,
            width: "100%",
            borderRadius: 6,
            cursor: "pointer",
            position: "relative",
            backgroundColor: isActive ? token.colorPrimaryBg : "transparent",
            paddingLeft: 12,
            paddingRight: 12,
            gap: 10,
            transition: "background-color 0.12s ease-in-out",
          }}
          onMouseEnter={(e) => {
            if (!isActive) {
              e.currentTarget.style.backgroundColor = token.colorFillSecondary;
            }
          }}
          onMouseLeave={(e) => {
            if (!isActive) {
              e.currentTarget.style.backgroundColor = "transparent";
            }
          }}
        >
          <div className="ax-nav-indicator" />
          <span style={{ display: "flex", alignItems: "center", flexShrink: 0, width: 20, justifyContent: "center" }}>
            {item.icon}
          </span>
          <span
            className="ax-nav-label"
            style={{
              fontSize: 13,
              fontWeight: isActive ? 500 : 400,
              color: isActive ? token.colorPrimary : token.colorText,
            }}
          >
            {label}
          </span>
          {shortcutLabel && (
            <span style={{ marginLeft: "auto", fontSize: 10, color: token.colorTextQuaternary, flexShrink: 0 }}>
              {shortcutLabel}
            </span>
          )}
        </div>
      </Tooltip>
    );
  };

  const renderUserAvatar = () => {
    const size = 28;
    if (profile.avatarType === "emoji" && profile.avatarValue) {
      return (
        <div
          style={{
            width: size,
            height: size,
            borderRadius: "50%",
            backgroundColor: token.colorFillSecondary,
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
    if ((profile.avatarType === "url" || profile.avatarType === "file") && profile.avatarValue) {
      const src = profile.avatarType === "file" ? resolvedAvatarSrc : profile.avatarValue;
      return <Avatar size={size} src={src} style={{ cursor: "pointer" }} />;
    }
    return (
      <Avatar
        size={size}
        icon={<User size={14} />}
        style={{ cursor: "pointer", backgroundColor: token.colorPrimary }}
      />
    );
  };

  return (
    <div
      className="ax-sidebar"
      style={{
        height: "100%",
        display: "flex",
        flexDirection: "column",
        alignItems: "stretch",
        width: SIDEBAR_WIDTH,
        padding: "12px 8px 12px",
        overflow: "hidden",
      }}
    >
      <nav style={{ flexShrink: 0, display: "flex", flexDirection: "column", gap: 4 }}>
        {sections.map((section) => (
          <div key={section.key} style={{ marginBottom: 4 }}>
            <div className="ax-sidebar-section-header">
              {t(section.labelKey)}
            </div>
            {section.items.map(renderNavButton)}
          </div>
        ))}
      </nav>

      <div className="flex-1" />

      <Tooltip title={profile.name || t("userProfile.title")} placement="right">
        <div
          onClick={() => setProfileModalOpen(true)}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "6px 12px",
            borderRadius: 6,
            cursor: "pointer",
            flexShrink: 0,
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = token.colorFillSecondary;
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = "transparent";
          }}
        >
          {renderUserAvatar()}
          <span
            className="ax-nav-label"
            style={{
              fontSize: 13,
              color: token.colorTextSecondary,
            }}
          >
            {profile.name || t("userProfile.title")}
          </span>
        </div>
      </Tooltip>

      <UserProfileModal open={profileModalOpen} onClose={() => setProfileModalOpen(false)} />
    </div>
  );
}
