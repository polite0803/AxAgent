// SPDX-License-Identifier: AGPL-3.0-only

import { Input } from "antd";
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
  Server,
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
import { SETTINGS_SEARCH_INDEX } from "./settingsSearchIndex";

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
  database: <Server size={14} />,
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
  dynamicPages: <LayoutDashboard size={14} />,
  localTools: <Wrench size={14} />,
  mcpServers: <Network size={14} />,
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
    "dynamicPages",
    "appConfig",
    "imageGen",
  ],
  network: ["proxy", "messageChannels", "webhooks", "acp"],
  data: [
    "data",
    "database",
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

  // === 设置搜索（板块级 + 项级，Phase 1 + 2）===
  // 合并「静态索引板块」、「索引内项级条目」、「技能动态注入板块」作为可搜索全集。
  const [query, setQuery] = useState("");

  interface SearchResult {
    kind: "section" | "item";
    key: string;
    icon: React.ReactNode;
    label: string;
    /** 副标题 — 仅项级结果显示（父板块名） */
    subLabel?: string;
    keywords: string[];
    /** 点击目标板块（项级结果 = 父 section，板块级结果 = 自身 section） */
    targetSection: string;
    /** 项级结果 → 高亮 itemKey */
    itemKey?: string;
  }

  const allSearchable = useMemo(() => {
    const results: SearchResult[] = [];
    // 板块级
    for (const entry of SETTINGS_SEARCH_INDEX) {
      results.push({
        kind: "section",
        key: entry.section,
        icon: MENU_ICONS[entry.section],
        label: t([`settings.${entry.section}.title`, `settings.${entry.section}`]),
        keywords: entry.keywords,
        targetSection: entry.section,
      });
      // 项级
      for (const item of entry.items ?? []) {
        results.push({
          kind: "item",
          key: `${entry.section}:${item.itemKey}`,
          icon: MENU_ICONS[entry.section],
          label: t(item.labelKey),
          subLabel: t([`settings.${entry.section}.title`, `settings.${entry.section}`]),
          keywords: item.keywords,
          targetSection: entry.section,
          itemKey: item.itemKey,
        });
      }
    }
    // 技能动态
    for (const it of skillItems) {
      results.push({
        kind: "item",
        key: it.key,
        icon: it.icon,
        label: it.label,
        keywords: [],
        targetSection: it.key,
      });
    }
    return results;
  }, [t, skillItems]);

  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) {
      return [];
    }
    return allSearchable.filter((entry) => {
      if (entry.label.toLowerCase().includes(q)) {
        return true;
      }
      return entry.keywords.some((k) => {
        const kw = k.toLowerCase();
        return kw.includes(q) || q.includes(kw);
      });
    });
  }, [query, allSearchable]);

  const setSettingsHighlight = useUIStore((s) => s.setSettingsHighlight);

  const handleSelect = useCallback(
    (result: SearchResult) => {
      setSettingsSection(result.targetSection as SettingsSection);
      setSettingsHighlight(result.itemKey ?? null);
      setQuery("");
    },
    [setSettingsSection, setSettingsHighlight],
  );

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

      <div className="st-search-box">
        <Input
          allowClear
          prefix={<Search size={14} />}
          placeholder={t("settings.searchPlaceholder")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onPressEnter={() => {
            if (results.length > 0) {
              handleSelect(results[0]);
            }
          }}
        />
      </div>

      <div style={{ flex: 1, overflowY: "auto" }}>
        {query.trim()
          ? (
            <div className="st-search-results">
              {results.length === 0
                ? <div className="st-search-empty">{t("settings.searchNoResults")}</div>
                : (
                  <>
                    <div className="st-search-header">
                      {t("settings.searchResults", { count: results.length })}
                    </div>
                    {results.map((r) => (
                      <div
                        key={r.key}
                        className={`st-item${settingsSection === r.key ? " active" : ""}`}
                        onClick={() => handleSelect(r)}
                      >
                        {r.icon}
                        <span className="st-item-text">
                          <span>{r.label}</span>
                          {r.subLabel && <span className="st-search-sub">{r.subLabel}</span>}
                        </span>
                      </div>
                    ))}
                  </>
                )}
            </div>
          )
          : groupConfigs.map((group) => (
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
