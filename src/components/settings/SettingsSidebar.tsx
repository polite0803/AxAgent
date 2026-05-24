import { Icon } from "@/components/common/Icon";
import { Tooltip } from "@/components/layout/Tooltip";
import { SettingsMenu } from "@/components/settings/SettingsMenu";
import type { SettingsMenuItem } from "@/components/settings/SettingsMenu";
import { SETTINGS_ICON_COLORS } from "@/lib/iconColors";
import { resolveIconComponent } from "@/lib/skillIcons";
import { useSkillExtensionStore, useUIStore } from "@/stores";
import type { SettingsSection } from "@/types";
import {
  ArrowLeft,
  Bell,
  BookOpen,
  Bot,
  Clock,
  Cloud,
  CloudUpload,
  Database,
  Dna,
  FileText,
  GitBranch,
  Globe,
  HardDrive,
  Image,
  Info,
  LayoutDashboard,
  MessageSquare,
  Network,
  PaintBucket,
  Palette,
  Puzzle,
  Search,
  Send,
  Settings,
  ShoppingBag,
  SlidersHorizontal,
  Timer,
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
  theme: <PaintBucket size={16} color={SETTINGS_ICON_COLORS.Palette} />,
  imageGen: <Image size={16} color={SETTINGS_ICON_COLORS.Palette} />,
  cron: <Timer size={16} color={SETTINGS_ICON_COLORS.Clock} />,
};

const TAB_GROUPS: Record<string, SettingsSection[]> = {
  model: [
    "providers",
    "defaultModel",
    "conversationSettings",
    "promptTemplates",
    "searchProviders",
  ],
  appearance: ["general", "display", "theme", "shortcuts"],
  extensions: [
    "tools",
    "skillsHub",
    "plugins",
    "knowledgeSettings",
    "dashboardPlugins",
    "workflow",
    "appConfig",
    "imageGen",
  ],
  network: ["proxy", "messageChannels", "webhooks", "acp"],
  data: [
    "data",
    "storage",
    "cloudWorkspace",
    "backup",
    "scheduler",
    "cron",
    "notificationCenter",
  ],
  system: ["advanced", "evolution", "about"],
};

const TAB_ICONS: Record<string, React.ReactNode> = {
  model: <Icon icon="fluent:brain-circuit-20-filled" size={20} color="#1677ff" />,
  appearance: <Icon icon="fluent:eye-20-filled" size={20} color="#52c41a" />,
  extensions: <Icon icon="fluent:puzzle-piece-20-filled" size={20} color="#fa8c16" />,
  network: <Icon icon="fluent:globe-20-filled" size={20} color="#13c2c2" />,
  data: <Icon icon="fluent:server-20-filled" size={20} color="#722ed1" />,
  system: <Icon icon="fluent:settings-20-filled" size={20} color="#8c8c8c" />,
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

  const onMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      dragging.current = true;
      startRef.current = { startX: e.clientX, startWidth: width };
    },
    [width],
  );

  return { width, onMouseDown };
}

export function SettingsSidebar() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const settingsSection = useUIStore((s) => s.settingsSection);
  const setSettingsSection = useUIStore((s) => s.setSettingsSection);
  const deviceLayout = useUIStore((s) => s.deviceLayout);
  const skillSections = useSkillExtensionStore((s) => s.settingsSections);
  const { width: tabBarWidth, onMouseDown: onTabBarResize } = useDraggableWidth(72, 48, 200);
  const isMobile = deviceLayout === "mobile";
  const isTablet = deviceLayout === "tablet";

  const sectionToTab = useMemo(() => {
    const map = new Map<string, string>();
    for (const [tab, sections] of Object.entries(TAB_GROUPS)) {
      for (const section of sections) {
        map.set(section, tab);
      }
    }
    return map;
  }, []);

  const [activeTab, setActiveTab] = useState(() => sectionToTab.get(settingsSection) ?? "model");

  useEffect(() => {
    const tab = sectionToTab.get(settingsSection);
    if (tab) { setActiveTab(tab); }
  }, [settingsSection, sectionToTab]);

  const handleTabChange = (key: string) => {
    setActiveTab(key);
    const firstSection = TAB_GROUPS[key]?.[0];
    if (firstSection) { setSettingsSection(firstSection); }
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

  // 构建每个 tab 的菜单项 (纯数据, 不包含 React 节点)
  const tabMenus = useMemo(() => {
    const result: Record<string, SettingsMenuItem[]> = {};
    for (const [key, sections] of Object.entries(TAB_GROUPS)) {
      const builtin: SettingsMenuItem[] = sections.map((sec) => ({
        key: sec,
        icon: MENU_ICONS[sec],
        label: t([`settings.${sec}.title`, `settings.${sec}`]),
      }));
      result[key] = key === "extensions" ? [...builtin, ...skillItems] : builtin;
    }
    return result;
  }, [t, skillItems]);

  const handleMenuClick = ({ key }: { key: string }) => {
    if (key.startsWith("skill:")) {
      setSettingsSection(key as SettingsSection);
    } else {
      setSettingsSection(key as SettingsSection);
    }
  };

  const tabKeys = Object.keys(TAB_GROUPS);

  return (
    <div className="h-full flex flex-col" data-testid="settings-sidebar">
      {/* Back button */}
      <button
        className="settings-back-btn"
        onClick={() => navigate("/")}
      >
        <ArrowLeft size={16} />
        <span>{t("common.back")}</span>
        {!isMobile && <kbd className="settings-back-kbd">Esc</kbd>}
      </button>

      <div className="flex-1 pt-1 settings-tab-area">
        {/* Tab buttons sidebar */}
        <div
          className="settings-tab-buttons"
          style={{ width: isMobile ? "auto" : tabBarWidth }}
        >
          {tabKeys.map((key) => {
            const tabLabel = t(
              `settings.tab${key.charAt(0).toUpperCase() + key.slice(1)}`,
            );
            return (
              <Tooltip key={key} title={tabLabel} placement="right">
                <button
                  className={`settings-tab-btn${activeTab === key ? " active" : ""}`}
                  onClick={() => handleTabChange(key)}
                >
                  {TAB_ICONS[key]}
                  {tabBarWidth > 120 && <span className="settings-tab-btn-label">{tabLabel}</span>}
                </button>
              </Tooltip>
            );
          })}
        </div>

        {/* Resize handle */}
        {!isMobile && !isTablet && (
          <div
            role="separator"
            className="settings-resize-handle"
            onMouseDown={onTabBarResize}
          />
        )}

        {/* Menu panel */}
        <div className="settings-menu-panel">
          <SettingsMenu
            items={tabMenus[activeTab] ?? []}
            selectedKeys={[settingsSection]}
            onClick={handleMenuClick}
          />
        </div>
      </div>
    </div>
  );
}
