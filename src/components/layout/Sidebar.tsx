import { useResolvedAvatarSrc } from "@/hooks/useResolvedAvatarSrc";
import { NAV_ICON_COLORS } from "@/lib/iconColors";
import { formatShortcutForDisplay, getShortcutBinding } from "@/lib/shortcuts";
import type { ShortcutAction } from "@/lib/shortcuts";
import { useSettingsStore, useUserProfileStore } from "@/stores";
import type { PageKey } from "@/types";
import { Avatar, theme, Tooltip } from "antd";
import { BookOpen, Brain, FileText, FolderOpen, Link2, MessageSquare, Router, Sparkles, Store, User } from "lucide-react";
import { useState } from "react";
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
  { key: "memory", icon: <Brain size={18} color={NAV_ICON_COLORS.Brain} />, labelKey: "nav.memory" },
  { key: "link", icon: <Link2 size={18} color={NAV_ICON_COLORS.Link2} />, labelKey: "nav.link" },
  { key: "gateway", icon: <Router size={18} color={NAV_ICON_COLORS.Router} />, labelKey: "nav.gateway" },
  { key: "files", icon: <FolderOpen size={18} color={NAV_ICON_COLORS.FolderOpen} />, labelKey: "nav.files" },
];

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

  const NAV_SHORTCUT_MAP: Partial<Record<PageKey, ShortcutAction>> = {
    gateway: "toggleGateway",
  };

  const renderNavButton = (item: { key: PageKey; icon: React.ReactNode; labelKey: string }) => {
    const isActive = activePage === item.key;
    const label = t(item.labelKey);
    const action = NAV_SHORTCUT_MAP[item.key];
    const title = action
      ? `${label} (${formatShortcutForDisplay(getShortcutBinding(settings, action))})`
      : label;
    return (
      <Tooltip key={item.key} title={title} placement="right">
        <button
          onClick={() => navigate(pageKeyToPath[item.key])}
          className="flex items-center justify-center text-base transition-colors"
          style={{
            width: 36,
            height: 36,
            borderRadius: "50%",
            backgroundColor: isActive ? token.colorPrimaryBg : "transparent",
            // When active, let the icon's own color show (which is the semantic color);
            // when inactive, also let the icon's own color show.
            // The icon already has its color set via the color prop.
            color: undefined,
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
          {item.icon}
        </button>
      </Tooltip>
    );
  };

  const renderUserAvatar = () => {
    const size = 32;
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
            fontSize: 16,
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
        icon={<User size={16} />}
        style={{ cursor: "pointer", backgroundColor: token.colorPrimary }}
      />
    );
  };

  return (
    <div className="flex flex-col items-center h-full" style={{ paddingTop: 8, paddingBottom: 12 }}>
      <nav className="flex flex-col gap-2">
        {mainNavItems.map(renderNavButton)}
      </nav>

      <div className="flex-1" />

      {/* User Avatar */}
      <Tooltip title={profile.name || t("userProfile.title")} placement="right">
        <button
          onClick={() => setProfileModalOpen(true)}
          style={{ background: "none", border: "none", padding: 0 }}
        >
          {renderUserAvatar()}
        </button>
      </Tooltip>

      <UserProfileModal open={profileModalOpen} onClose={() => setProfileModalOpen(false)} />
    </div>
  );
}
