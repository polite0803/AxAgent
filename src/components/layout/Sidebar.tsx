import { useResolvedAvatarSrc } from "@/hooks/useResolvedAvatarSrc";
import { NAV_ICON_COLORS } from "@/lib/iconColors";
import { formatShortcutForDisplay, getShortcutBinding } from "@/lib/shortcuts";
import type { ShortcutAction } from "@/lib/shortcuts";
import { useSettingsStore, useUserProfileStore } from "@/stores";
import type { PageKey } from "@/types";
import { Avatar, theme, Tooltip } from "antd";
import { BookOpen, Brain, FileText, FolderOpen, Library, Link2, MessageSquare, Router, Sparkles, Store, User } from "lucide-react";
import { useCallback, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { UserProfileModal } from "./UserProfileModal";

const pageKeyToPath: Record<PageKey, string> = {
  chat: "/",
  skills: "/skills",
  marketplace: "/marketplace",
  prompts: "/prompts",
  knowledge: "/knowledge",
  memory: "/memory",
  link: "/link",
  gateway: "/gateway",
  files: "/files",
  wiki: "/wiki",
  settings: "/settings",
};

const pathToPageKey = (path: string): PageKey => {
  if (path === "/" || path === "") { return "chat"; }
  const key = path.slice(1) as PageKey;
  if (key in pageKeyToPath) { return key; }
  return "chat";
};

const mainNavItems: { key: PageKey; icon: React.ReactNode; labelKey: string }[] = [
  { key: "chat", icon: <MessageSquare size={18} color={NAV_ICON_COLORS.MessageSquare} />, labelKey: "nav.chat" },
  { key: "skills", icon: <Sparkles size={18} color={NAV_ICON_COLORS.Sparkles} />, labelKey: "nav.skills" },
  { key: "marketplace", icon: <Store size={18} color={NAV_ICON_COLORS.Router} />, labelKey: "nav.marketplace" },
  { key: "prompts", icon: <FileText size={18} color={NAV_ICON_COLORS.Router} />, labelKey: "nav.prompts" },
  { key: "knowledge", icon: <BookOpen size={18} color={NAV_ICON_COLORS.BookOpen} />, labelKey: "nav.knowledge" },
  { key: "wiki", icon: <Library size={18} color={NAV_ICON_COLORS.Library} />, labelKey: "nav.wiki" },
  { key: "memory", icon: <Brain size={18} color={NAV_ICON_COLORS.Brain} />, labelKey: "nav.memory" },
  { key: "link", icon: <Link2 size={18} color={NAV_ICON_COLORS.Link2} />, labelKey: "nav.link" },
  { key: "gateway", icon: <Router size={18} color={NAV_ICON_COLORS.Router} />, labelKey: "nav.gateway" },
  { key: "files", icon: <FolderOpen size={18} color={NAV_ICON_COLORS.FolderOpen} />, labelKey: "nav.files" },
];

const EXPANDED_WIDTH = 180;
const COLLAPSED_WIDTH = 44;

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
  const [expanded, setExpanded] = useState(false);
  const enterTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const leaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleMouseEnter = useCallback(() => {
    if (leaveTimerRef.current) { clearTimeout(leaveTimerRef.current); }
    enterTimerRef.current = setTimeout(() => setExpanded(true), 150);
  }, []);

  const handleMouseLeave = useCallback(() => {
    if (enterTimerRef.current) { clearTimeout(enterTimerRef.current); }
    leaveTimerRef.current = setTimeout(() => setExpanded(false), 200);
  }, []);

  const NAV_SHORTCUT_MAP: Partial<Record<PageKey, ShortcutAction>> = {
    gateway: "toggleGateway",
  };

  const renderNavButton = (item: { key: PageKey; icon: React.ReactNode; labelKey: string }) => {
    const isActive = activePage === item.key;
    const label = t(item.labelKey);
    const action = NAV_SHORTCUT_MAP[item.key];
    const shortcutLabel = action
      ? formatShortcutForDisplay(getShortcutBinding(settings, action))
      : "";
    const title = action ? `${label} (${shortcutLabel})` : label;

    return (
      <Tooltip key={item.key} title={expanded ? "" : title} placement="right">
        <div
          onClick={() => navigate(pageKeyToPath[item.key])}
          className={isActive ? "ax-nav-item-active" : ""}
          style={{
            display: "flex",
            alignItems: "center",
            height: 36,
            width: "100%",
            borderRadius: 6,
            cursor: "pointer",
            position: "relative",
            backgroundColor: isActive ? token.colorPrimaryBg : "transparent",
            paddingLeft: 8,
            paddingRight: 8,
            gap: 10,
            transition: "background-color 0.15s ease-in-out",
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
              fontSize: 12,
              fontWeight: isActive ? 500 : 400,
              color: isActive ? token.colorPrimary : token.colorText,
              overflow: "hidden",
              whiteSpace: "nowrap",
              textOverflow: "ellipsis",
            }}
          >
            {label}
          </span>
          {shortcutLabel && expanded && (
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
      className={`ax-sidebar ${expanded ? "ax-sidebar-expanded" : ""}`}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      style={{
        height: "100%",
        display: "flex",
        flexDirection: "column",
        alignItems: "stretch",
        width: expanded ? EXPANDED_WIDTH : COLLAPSED_WIDTH,
        padding: "8px 4px 12px",
        overflow: "hidden",
      }}
    >
      <nav className="flex flex-col gap-1" style={{ flexShrink: 0 }}>
        {mainNavItems.map(renderNavButton)}
      </nav>

      <div className="flex-1" />

      {/* User Avatar */}
      <Tooltip title={expanded ? "" : (profile.name || t("userProfile.title"))} placement="right">
        <div
          onClick={() => setProfileModalOpen(true)}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "4px 6px",
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
              fontSize: 12,
              color: token.colorTextSecondary,
              overflow: "hidden",
              whiteSpace: "nowrap",
              textOverflow: "ellipsis",
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
