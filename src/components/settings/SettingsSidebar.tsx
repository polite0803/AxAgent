import { SETTINGS_ICON_COLORS } from "@/lib/iconColors";
import { resolveIconComponent } from "@/lib/skillIcons";
import { useSkillExtensionStore, useUIStore } from "@/stores";
import type { SettingsSection } from "@/types";
import { Menu, Tabs, theme } from "antd";
import {
  ArrowLeft,
  Bell,
  Bot,
  Clock,
  Cloud,
  CloudUpload,
  Database,
  FileText,
  GitBranch,
  Globe,
  HardDrive,
  Info,
  LayoutDashboard,
  MessageSquare,
  Network,
  Palette,
  Puzzle,
  Search,
  Send,
  Settings,
  ShoppingBag,
  SlidersHorizontal,
  User,
  Wrench,
  Zap,
} from "lucide-react";
import { useMemo, useState } from "react";
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
  acp: <Network size={16} color={SETTINGS_ICON_COLORS.Globe} />,
  skillsHub: <ShoppingBag size={16} color={SETTINGS_ICON_COLORS.ShoppingBag} />,
  dashboardPlugins: <LayoutDashboard size={16} color={SETTINGS_ICON_COLORS.LayoutDashboard} />,
  webhooks: <Bell size={16} color={SETTINGS_ICON_COLORS.Bell} />,
  messageChannels: <Send size={16} color={SETTINGS_ICON_COLORS.Send} />,
  advanced: <SlidersHorizontal size={16} color={SETTINGS_ICON_COLORS.Settings} />,
  promptTemplates: <FileText size={16} color={SETTINGS_ICON_COLORS.FileText} />,
  appConfig: <Bot size={16} color={SETTINGS_ICON_COLORS.Bot} />,
};

// 分组定义：tab key → 包含的 sections
const TAB_GROUPS: Record<string, SettingsSection[]> = {
  model: ["providers", "defaultModel", "conversationSettings", "promptTemplates", "searchProviders"],
  appearance: ["general", "display", "shortcuts"],
  extensions: ["tools", "skillsHub", "dashboardPlugins", "workflow", "appConfig", "userProfile"],
  network: ["proxy", "messageChannels", "webhooks", "acp"],
  data: ["data", "storage", "backup", "scheduler"],
  system: ["advanced", "about"],
};

export function SettingsSidebar() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const settingsSection = useUIStore((s) => s.settingsSection);
  const setSettingsSection = useUIStore((s) => s.setSettingsSection);
  const skillSections = useSkillExtensionStore((s) => s.settingsSections);

  // 根据当前选中的 section 反查所属 tab
  const [activeTab, setActiveTab] = useState(() => {
    for (const [tab, sections] of Object.entries(TAB_GROUPS)) {
      if (sections.includes(settingsSection)) return tab;
    }
    return "model";
  });

  const handleTabChange = (key: string) => {
    setActiveTab(key);
    // 切换到该 tab 的第一个 section
    const firstSection = TAB_GROUPS[key]?.[0];
    if (firstSection) {
      setSettingsSection(firstSection);
    }
  };

  const skillItems = useMemo(() => {
    return skillSections.map((sec) => {
      const IconComp = sec.icon ? resolveIconComponent(sec.icon) : Puzzle;
      return {
        key: `skill:${sec.skillName}:${sec.id}` as string,
        icon: <IconComp size={16} />,
        label: sec.label,
      };
    });
  }, [skillSections]);

  const tabItems = Object.entries(TAB_GROUPS).map(([key, sections]) => {
    const builtin = sections.map((sec) => ({
      key: sec,
      icon: MENU_ICONS[sec],
      label: t([`settings.${sec}.title`, `settings.${sec}`]),
    }));
    // 在最后添加技能扩展项
    const items = key === "extensions" ? [...builtin, ...skillItems] : builtin;

    return {
      key,
      label: t(`settings.tab${key.charAt(0).toUpperCase() + key.slice(1)}`),
      children: (
        <Menu
          mode="inline"
          selectedKeys={[settingsSection]}
          items={items}
          style={{ borderInlineEnd: "none" }}
          onClick={({ key }) => {
            if (typeof key === "string" && key.startsWith("skill:")) {
              setSettingsSection(key as SettingsSection);
            } else {
              setSettingsSection(key as SettingsSection);
            }
          }}
        />
      ),
    };
  });

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
        <Tabs
          activeKey={activeTab}
          onChange={handleTabChange}
          items={tabItems}
          tabPosition="left"
          tabBarStyle={{ width: 72, flexShrink: 0 }}
          style={{ height: "100%" }}
        />
      </div>
    </div>
  );
}
