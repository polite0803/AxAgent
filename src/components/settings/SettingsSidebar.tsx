import {
  Bell,
  Bot,
  Clock,
  Cloud,
  CloudUpload,
  Database,
  Dna,
  FileText,
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
  Wrench,
  Zap,
} from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import { resolveIconComponent } from "@/lib/skillIcons";
import { useSkillExtensionStore, useUIStore } from "@/stores";
import type { SettingsSection } from "@/types";

// 菜单图标 — 不设 color, 由 CSS .st-item / .st-item.active 通过 currentColor 控制
const MENU_ICONS: Partial<Record<SettingsSection, React.ReactNode>> = {
  providers: <Cloud size={14} />,
  conversationSettings: <MessageSquare size={14} />,
  defaultModel: <Bot size={14} />,
  general: <Settings size={14} />,
  display: <Palette size={14} />,
  proxy: <Globe size={14} />,
  shortcuts: <Zap size={14} />,
  data: <Database size={14} />,
  storage: <HardDrive size={14} />,
  about: <Info size={14} />,
  searchProviders: <Search size={14} />,
  tools: <Wrench size={14} />,
  scheduler: <Clock size={14} />,
  backup: <CloudUpload size={14} />,
  acp: <Network size={14} />,
  skillsHub: <ShoppingBag size={14} />,
  plugins: <Puzzle size={14} />,
  dashboardPlugins: <LayoutDashboard size={14} />,
  notificationCenter: <Bell size={14} />,
  webhooks: <Bell size={14} />,
  messageChannels: <Send size={14} />,
  advanced: <SlidersHorizontal size={14} />,
  promptTemplates: <FileText size={14} />,
  appConfig: <Bot size={14} />,
  evolution: <Dna size={14} />,
  cloudWorkspace: <Cloud size={14} />,
  theme: <PaintBucket size={14} />,
  imageGen: <Image size={14} />,
  cron: <Timer size={14} />,
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
    "dashboardPlugins",
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

export function SettingsSidebar() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const settingsSection = useUIStore((s) => s.settingsSection);
  const setSettingsSection = useUIStore((s) => s.setSettingsSection);
  const deviceLayout = useUIStore((s) => s.deviceLayout);
  const skillSections = useSkillExtensionStore((s) => s.settingsSections);
  const isSmall = deviceLayout === "mobile" || deviceLayout === "tablet";

  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(() => {
    // 小屏默认折叠所有分组，节省垂直空间
    if (isSmall) {
      return new Set(Object.keys(TAB_GROUPS));
    }
    return new Set();
  });

  const toggleGroup = useCallback((key: string) => {
    setCollapsedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(key)) { next.delete(key); }
      else { next.add(key); }
      return next;
    });
  }, []);

  const skillItems = useMemo(() => {
    return skillSections.map((sec) => {
      const IconComp = sec.icon ? resolveIconComponent(sec.icon) : Puzzle;
      return {
        key: `skill:${sec.skillName}:${sec.id}` as string,
        icon: <IconComp size={14} />,
        label: sec.title,
      };
    });
  }, [skillSections]);

  const groupConfigs = useMemo(() => {
    const groups = [];
    for (const [key, sections] of Object.entries(TAB_GROUPS)) {
      const items: Array<
        { key: string; icon: React.ReactNode; label: string }
      > = sections.map((sec) => ({
        key: sec,
        icon: MENU_ICONS[sec],
        label: t([`settings.${sec}.title`, `settings.${sec}`]),
      }));
      if (key === "extensions") {
        items.push(...skillItems);
      }
      groups.push({
        key,
        label: t(
          `settings.tab${key.charAt(0).toUpperCase() + key.slice(1)}`,
        ),
        items,
      });
    }
    return groups;
  }, [t, skillItems]);

  return (
    <div className="h-full flex flex-col" data-testid="settings-sidebar">
      <button
        className="settings-back-btn"
        onClick={() => navigate("/")}
      >
        {/* ArrowLeft as inline SVG to avoid extra import */}
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <line x1="19" y1="12" x2="5" y2="12" />
          <polyline points="12 19 5 12 12 5" />
        </svg>
        <span>{t("common.back")}</span>
        {!isSmall && <kbd className="settings-back-kbd">Esc</kbd>}
      </button>

      <div style={{ flex: 1, overflowY: "auto" }}>
        {groupConfigs.map((group) => (
          <div
            key={group.key}
            className={`st-group${collapsedGroups.has(group.key) ? " collapsed" : ""}`}
          >
            <div
              className="st-group-header"
              onClick={() => toggleGroup(group.key)}
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
              >
                <circle
                  cx="12"
                  cy="12"
                  r="3"
                  fill="currentColor"
                  fillOpacity=".12"
                />
              </svg>
              <span>{group.label}</span>
              <svg
                className="arrow"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
              >
                <polyline points="9 18 15 12 9 6" />
              </svg>
            </div>
            <div className="st-items">
              {group.items.map((item) => (
                <div
                  key={item.key}
                  className={`st-item${settingsSection === item.key ? " active" : ""}`}
                  onClick={() => setSettingsSection(item.key as SettingsSection)}
                >
                  {item.icon}
                  <span className="st-item-text">{item.label}</span>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
