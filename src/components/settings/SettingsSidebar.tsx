import { SETTINGS_ICON_COLORS } from "@/lib/iconColors";
import { resolveIconComponent } from "@/lib/skillIcons";
import { useSkillExtensionStore, useUIStore } from "@/stores";
import type { SettingsSection } from "@/types";
import { Menu, Tabs, theme, Tooltip } from "antd";
import {
  ArrowLeft,
  Bell,
  BookOpen,
  Bot,
  Boxes,
  Cable,
  Clock,
  Cloud,
  CloudUpload,
  Database,
  Dna,
  FileText,
  GitBranch,
  Globe,
  HardDrive,
  Info,
  LayoutDashboard,
  MessageSquare,
  Monitor,
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
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  plugins: <Puzzle size={16} color={SETTINGS_ICON_COLORS.Puzzle} />,
  knowledgeSettings: <BookOpen size={16} color={SETTINGS_ICON_COLORS.BookOpen} />,
  dashboardPlugins: <LayoutDashboard size={16} color={SETTINGS_ICON_COLORS.LayoutDashboard} />,
  notificationCenter: <Bell size={16} color={SETTINGS_ICON_COLORS.Bell} />,
  webhooks: <Bell size={16} color={SETTINGS_ICON_COLORS.Bell} />,
  messageChannels: <Send size={16} color={SETTINGS_ICON_COLORS.Send} />,
  advanced: <SlidersHorizontal size={16} color={SETTINGS_ICON_COLORS.Settings} />,
  promptTemplates: <FileText size={16} color={SETTINGS_ICON_COLORS.FileText} />,
  appConfig: <Bot size={16} color={SETTINGS_ICON_COLORS.Bot} />,
  evolution: <Dna size={16} color={SETTINGS_ICON_COLORS.Palette} />,
  cloudWorkspace: <Cloud size={16} color={SETTINGS_ICON_COLORS.Cloud} />,
};

// 分组定义：tab key → 包含的 sections
const TAB_GROUPS: Record<string, SettingsSection[]> = {
  model: ["providers", "defaultModel", "conversationSettings", "promptTemplates", "searchProviders"],
  appearance: ["general", "display", "shortcuts"],
  extensions: [
    "tools",
    "skillsHub",
    "plugins",
    "knowledgeSettings",
    "dashboardPlugins",
    "workflow",
    "appConfig",
    "userProfile",
  ],
  network: ["proxy", "messageChannels", "webhooks", "acp"],
  data: ["data", "storage", "cloudWorkspace", "backup", "scheduler", "notificationCenter"],
  system: ["advanced", "evolution", "about"],
};

// Tab 图标映射
const TAB_ICONS: Record<string, React.ReactNode> = {
  model: <Boxes size={18} />,
  appearance: <Monitor size={18} />,
  extensions: <Puzzle size={18} />,
  network: <Cable size={18} />,
  data: <Database size={18} />,
  system: <SlidersHorizontal size={18} />,
};

function useDraggableWidth(initial: number, min: number, max: number) {
  const [width, setWidth] = useState(initial);
  const dragging = useRef(false);
  const startRef = useRef({ startX: 0, startWidth: 0 });

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!dragging.current) { return; }
      const dx = e.clientX - startRef.current.startX;
      setWidth(Math.max(min, Math.min(max, startRef.current.startWidth + dx)));
    };
    const handleMouseUp = () => {
      dragging.current = false;
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [min, max]);

  const onMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    dragging.current = true;
    startRef.current = { startX: e.clientX, startWidth: width };
  }, [width]);

  return { width, onMouseDown };
}

export function SettingsSidebar() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const settingsSection = useUIStore((s) => s.settingsSection);
  const setSettingsSection = useUIStore((s) => s.setSettingsSection);
  const skillSections = useSkillExtensionStore((s) => s.settingsSections);
  const { width: tabBarWidth, onMouseDown: onTabBarResize } = useDraggableWidth(72, 48, 200);

  // 预构建 section → tab 反向映射，避免循环中调用 includes
  const sectionToTab = useMemo(() => {
    const map = new Map<string, string>();
    for (const [tab, sections] of Object.entries(TAB_GROUPS)) {
      for (const section of sections) {
        map.set(section, tab);
      }
    }
    return map;
  }, []);

  // 根据当前选中的 section 反查所属 tab
  const [activeTab, setActiveTab] = useState(() => {
    return sectionToTab.get(settingsSection) ?? "model";
  });

  // 当 settingsSection 变化时，同步更新 activeTab
  useEffect(() => {
    const tab = sectionToTab.get(settingsSection);
    if (tab) { setActiveTab(tab); }
  }, [settingsSection, sectionToTab]);

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
        label: sec.title,
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

    const tabLabel = t(`settings.tab${key.charAt(0).toUpperCase() + key.slice(1)}`);
    return {
      key,
      label: (
        <Tooltip title={tabLabel} placement="right">
          <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
            {TAB_ICONS[key]}
            {tabBarWidth > 120 && <span style={{ fontSize: 13 }}>{tabLabel}</span>}
          </span>
        </Tooltip>
      ),
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
        role="button"
        tabIndex={0}
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
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") { navigate("/"); }
        }}
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
            fontSize: 12,
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
      <div className="flex-1 pt-1" style={{ overflowY: "auto", display: "flex" }}>
        <Tabs
          activeKey={activeTab}
          onChange={handleTabChange}
          items={tabItems}
          tabPlacement="start"
          tabBarStyle={{ width: tabBarWidth, flexShrink: 0, transition: "width 0.05s" }}
          style={{ height: "100%", flex: 1 }}
        />
        {/* Resize handle */}
        <div
          role="separator"
          tabIndex={0}
          onMouseDown={onTabBarResize}
          style={{
            width: 4,
            cursor: "col-resize",
            flexShrink: 0,
            backgroundColor: "transparent",
            transition: "background-color 0.15s",
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = token.colorPrimary;
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = "transparent";
          }}
        />
      </div>
    </div>
  );
}
