import { SETTINGS_ICON_COLORS } from "@/lib/iconColors";
import { useUIStore } from "@/stores";
import type { SettingsSection } from "@/types";
import { Menu, theme } from "antd";
import {
  ArrowLeft,
  Bell,
  Bot,
  Clock,
  Cloud,
  CloudUpload,
  Database,
  GitBranch,
  Globe,
  HardDrive,
  Info,
  LayoutDashboard,
  MessageSquare,
  Palette,
  Search,
  Send,
  Settings,
  ShoppingBag,
  SlidersHorizontal,
  User,
  Wrench,
  Zap,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

const MENU_ICONS: Partial<Record<SettingsSection, React.ReactNode>> = {
  providers: <Cloud size={16} color={SETTINGS_ICON_COLORS.Cloud} />,
  conversationSettings: <MessageSquare size={16} color={SETTINGS_ICON_COLORS.MessageSquare} />,
  defaultModel: <Bot size={16} color={SETTINGS_ICON_COLORS.Bot} />,
  general: <Settings size={16} color={SETTINGS_ICON_COLORS.Settings} />,
  display: <Palette size={16} color={SETTINGS_ICON_COLORS.Palette} />,
  proxy: <Globe size={16} color={SETTINGS_ICON_COLORS.Globe} />,
  shortcuts: <Zap size={16} color={SETTINGS_ICON_COLORS.Zap} />,
  data: <Database size={16} color={SETTINGS_ICON_COLORS.Database} />,
  storage: <HardDrive size={16} color={SETTINGS_ICON_COLORS.HardDrive} />,
  about: <Info size={16} color={SETTINGS_ICON_COLORS.Info} />,
  searchProviders: <Search size={16} color={SETTINGS_ICON_COLORS.Search} />,
  tools: <Wrench size={16} color={SETTINGS_ICON_COLORS.Wrench} />,
  scheduler: <Clock size={16} color={SETTINGS_ICON_COLORS.Clock} />,
  backup: <CloudUpload size={16} color={SETTINGS_ICON_COLORS.CloudUpload} />,
  workflow: <GitBranch size={16} color={SETTINGS_ICON_COLORS.Workflow} />,
  userProfile: <User size={16} color={SETTINGS_ICON_COLORS.User} />,
  skillsHub: <ShoppingBag size={16} color={SETTINGS_ICON_COLORS.ShoppingBag} />,
  dashboardPlugins: <LayoutDashboard size={16} color={SETTINGS_ICON_COLORS.LayoutDashboard} />,
  webhooks: <Bell size={16} color={SETTINGS_ICON_COLORS.Bell} />,
  messageChannels: <Send size={16} color={SETTINGS_ICON_COLORS.Send} />,
  advanced: <SlidersHorizontal size={16} color={SETTINGS_ICON_COLORS.Settings} />,
};

const SECTION_KEYS: SettingsSection[] = [
  "general",
  "display",
  "providers",
  "conversationSettings",
  "defaultModel",
  "searchProviders",
  "tools",
  "skillsHub",
  "dashboardPlugins",
  "messageChannels",
  "webhooks",
  "proxy",
  "shortcuts",
  "data",
  "storage",
  "scheduler",
  "backup",
  "workflow",
  "userProfile",
  "advanced",
  "about",
];

export function SettingsSidebar() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const settingsSection = useUIStore((s) => s.settingsSection);
  const setSettingsSection = useUIStore((s) => s.setSettingsSection);

  const items = SECTION_KEYS.map((key) => ({
    key,
    icon: MENU_ICONS[key],
    label: t([`settings.${key}.title`, `settings.${key}`]),
  }));

  return (
    <div
      className="h-full flex flex-col"
      data-os-scrollbar
      data-testid="settings-sidebar"
      style={{ backgroundColor: token.colorBgContainer, overflowY: "auto" }}
    >
      {/* Back button */}
      <div
        className="flex items-center gap-2 cursor-pointer"
        style={{
          color: token.colorTextSecondary,
          borderBottom: `1px solid ${token.colorBorderSecondary}`,
          flexShrink: 0,
          paddingLeft: 26,
          paddingRight: 16,
          paddingTop: 12,
          paddingBottom: 12,
        }}
        onClick={() => navigate("/")}
        onMouseEnter={(e) => {
          e.currentTarget.style.color = token.colorText;
          e.currentTarget.style.backgroundColor = token.colorFillSecondary;
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.color = token.colorTextSecondary;
          e.currentTarget.style.backgroundColor = "transparent";
        }}
      >
        <ArrowLeft size={16} />
        <span style={{ fontSize: 14 }}>{t("common.back")}</span>
        <span
          style={{
            fontSize: 11,
            color: token.colorTextQuaternary,
            border: `1px solid ${token.colorBorderSecondary}`,
            borderRadius: 4,
            padding: "1px 6px",
            marginLeft: 4,
            lineHeight: "16px",
          }}
        >
          Esc
        </span>
      </div>
      <div className="flex-1 pt-1" style={{ overflowY: "auto" }}>
        <Menu
          mode="inline"
          selectedKeys={[settingsSection]}
          items={items}
          style={{ borderInlineEnd: "none" }}
          onClick={({ key }) => setSettingsSection(key as SettingsSection)}
        />
      </div>
    </div>
  );
}
