// SPDX-License-Identifier: AGPL-3.0-only

import { Icon } from "@/components/common/Icon";
import { Tooltip } from "@/components/layout/Tooltip";
import { FEATURE_FLAGS } from "@/constants/featureFlags";
import { useResolvedAvatarSrc } from "@/hooks/useResolvedAvatarSrc";
import { invoke, logIpcError } from "@/lib/invoke";
import { BUILTIN_PAGE_PATH } from "@/lib/pageRegistry";
import { getPinnedSchemasByGroup, PIN_GROUPS } from "@/lib/pinned-schemas";
import type { PinnedSchemaMap } from "@/lib/pinned-schemas";
import { formatShortcutForDisplay, getShortcutBinding } from "@/lib/shortcuts";
import type { ShortcutAction } from "@/lib/shortcuts";
import {
  useAgentPanelStore,
  useDynamicUIStore,
  useOnboardingStore,
  useSettingsStore,
  useUIStore,
  useUserProfileStore,
  useWorkspaceStore,
} from "@/stores";
import type { AppSettings, PageKey } from "@/types";
import { AppstoreAddOutlined, MenuFoldOutlined, MenuUnfoldOutlined } from "@ant-design/icons";
import { Avatar } from "antd";
import { GitBranch, Globe, LineChart, Moon, Pin, PinOff, RotateCcw, Settings, Sun, User } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { UserProfileModal } from "./UserProfileModal";
/** 单一路径来源：直接复用 pageRegistry 的权威映射。 */
const pageKeyToPath = BUILTIN_PAGE_PATH as Record<PageKey, string>;

/** path→key 反查表：用于识别多段路径（如 /devtools/trace-explorer）对应的 PageKey。 */
const pathToPageKeyMap: Record<string, PageKey> = Object.fromEntries(
  Object.entries(BUILTIN_PAGE_PATH).map(([key, path]) => [path, key as PageKey]),
) as Record<string, PageKey>;

function pathToPageKey(path: string): PageKey {
  if (path === "/" || path === "") {
    return "dashboard";
  }
  // 优先按完整路径反查（覆盖 /devtools/xxx 这类多段路径）
  if (path in pathToPageKeyMap) {
    return pathToPageKeyMap[path];
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
    path: BUILTIN_PAGE_PATH.chat,
    isPlugin: false,
  },
  {
    key: "dashboard",
    icon: <Icon icon="fluent:grid-20-filled" size={17} />,
    labelKey: "nav.dashboard",
    path: BUILTIN_PAGE_PATH.dashboard,
    isPlugin: false,
  },
  {
    key: "knowledge",
    icon: <Icon icon="fluent:book-database-20-filled" size={17} />,
    labelKey: "nav.knowledge",
    path: BUILTIN_PAGE_PATH.knowledge,
    isPlugin: false,
  },
  {
    key: "gateway",
    icon: <Icon icon="fluent:globe-20-filled" size={17} />,
    labelKey: "nav.gateway",
    path: BUILTIN_PAGE_PATH.gateway,
    isPlugin: false,
  },
  {
    key: "terminal",
    icon: <Icon icon="fluent:prompt-20-filled" size={17} />,
    labelKey: "nav.terminal",
    path: BUILTIN_PAGE_PATH.terminal,
    isPlugin: false,
  },
  {
    key: "files",
    icon: <Icon icon="fluent:folder-20-filled" size={17} />,
    labelKey: "nav.files",
    path: BUILTIN_PAGE_PATH.files,
    isPlugin: false,
  },
  {
    key: "workflow",
    icon: <Icon icon="fluent:flow-20-filled" size={17} />,
    labelKey: "nav.workflow",
    path: BUILTIN_PAGE_PATH.workflow,
    isPlugin: false,
  },
  {
    key: "marketplace",
    icon: <Icon icon="fluent:store-microsoft-20-filled" size={17} />,
    labelKey: "nav.marketplace",
    path: BUILTIN_PAGE_PATH.marketplace,
    isPlugin: false,
  },
  {
    key: "dynamic-ui",
    icon: <Icon icon="fluent:apps-20-filled" size={17} />,
    labelKey: "nav.dynamicUI",
    path: BUILTIN_PAGE_PATH["dynamic-ui"],
    isPlugin: false,
  },
  {
<<<<<<< HEAD
    key: "workspace",
    icon: <Icon icon="fluent:target-20-filled" size={17} />,
    labelKey: "nav.workspace",
    path: BUILTIN_PAGE_PATH.workspace,
    isPlugin: false,
  },
  {
    key: "screener",
    icon: <Icon icon="fluent:search-20-filled" size={17} />,
    labelKey: "nav.screener",
    path: BUILTIN_PAGE_PATH.screener,
    isPlugin: false,
  },
  {
    key: "paper-portfolio",
    icon: <Icon icon="fluent:notebook-20-filled" size={17} />,
    labelKey: "nav.paperPortfolio",
    path: BUILTIN_PAGE_PATH["paper-portfolio"],
    isPlugin: false,
  },
  {
    key: "market-mainline",
    icon: <Icon icon="fluent:trending-20-filled" size={17} />,
    labelKey: "nav.marketMainline",
    path: BUILTIN_PAGE_PATH["market-mainline"],
    isPlugin: false,
  },
  {
    key: "screenshot-diagnosis",
    icon: <Icon icon="fluent:image-20-filled" size={17} />,
    labelKey: "nav.screenshotDiagnosis",
    path: BUILTIN_PAGE_PATH["screenshot-diagnosis"],
    isPlugin: false,
  },
  {
    key: "multi-agent",
    icon: <Icon icon="fluent:people-team-20-filled" size={17} />,
    labelKey: "nav.multiAgent",
    path: BUILTIN_PAGE_PATH["multi-agent"],
    isPlugin: false,
  },
  {
    key: "quant",
    icon: <Icon icon="fluent:brain-20-filled" size={17} />,
    labelKey: "nav.quant",
    path: BUILTIN_PAGE_PATH.quant,
    isPlugin: false,
  },
  {
    key: "pipeline",
    icon: <GitBranch size={17} />,
    labelKey: "nav.pipeline",
    path: BUILTIN_PAGE_PATH.pipeline,
    isPlugin: false,
  },
  {
    key: "wiki",
    icon: <Icon icon="fluent:book-globe-20-filled" size={17} />,
    labelKey: "nav.wiki",
    path: BUILTIN_PAGE_PATH.wiki,
    isPlugin: false,
  },
];
    icon: <Icon icon="fluent:book-globe-20-filled" size={17} />,
    labelKey: "nav.wiki",
    path: BUILTIN_PAGE_PATH.wiki,
    isPlugin: false,
  },
];

/** 开发者工具导航项 — 由 settings.show_developer_tools 门控 */
const devtoolsNavItems: NavItem[] = [
  {
    key: "devtoolsTraceExplorer",
    icon: <Icon icon="fluent:search-info-20-filled" size={17} />,
    labelKey: "nav.devtools.traceExplorer",
    path: BUILTIN_PAGE_PATH.devtoolsTraceExplorer,
    isPlugin: false,
  },
  {
    key: "devtoolsBenchmark",
    icon: <Icon icon="fluent:gauge-20-filled" size={17} />,
    labelKey: "nav.devtools.benchmark",
    path: BUILTIN_PAGE_PATH.devtoolsBenchmark,
    isPlugin: false,
  },
  {
    key: "devtoolsToolRecommender",
    icon: <Icon icon="fluent:wand-20-filled" size={17} />,
    labelKey: "nav.devtools.toolRecommender",
    path: BUILTIN_PAGE_PATH.devtoolsToolRecommender,
    isPlugin: false,
  },
  {
    key: "devtoolsFineTune",
    icon: <Icon icon="fluent:brain-circuit-20-filled" size={17} />,
    labelKey: "nav.devtools.fineTune",
    path: BUILTIN_PAGE_PATH.devtoolsFineTune,
    isPlugin: false,
  },
  {
    key: "devtoolsRlTraining",
    icon: <Icon icon="fluent:trophy-20-filled" size={17} />,
    labelKey: "nav.devtools.rlTraining",
    path: BUILTIN_PAGE_PATH.devtoolsRlTraining,
>>>>>>> upstream-master
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
    saveSettings({ theme_mode: next }).catch(logIpcError("Sidebar: saveSettings(theme_mode)"));
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
        <button style={btnBase} onClick={togglePin} aria-label={t("desktop.alwaysOnTop")} aria-pressed={pinned}>
          {pinned ? <Pin size={16} /> : <PinOff size={16} />}
        </button>
      </Tooltip>
      <Tooltip title={t("settings.groupTheme")} placement="right">
        <button style={btnBase} onClick={cycleTheme} aria-label={t("settings.groupTheme")}>
          <ThemeIcon size={16} />
        </button>
      </Tooltip>
      <Tooltip title={t("desktop.reloadPage")} placement="right">
        <button style={btnBase} onClick={() => window.location.reload()} aria-label={t("desktop.reloadPage")}>
          <RotateCcw size={16} />
        </button>
      </Tooltip>
      <Tooltip title={t("settings.openSettings")} placement="right">
        <button
          style={btnBase}
          onClick={() => navigate(BUILTIN_PAGE_PATH.settings)}
          aria-label={t("settings.openSettings")}
        >
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
  const sidebarCollapsed = useUIStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useUIStore((s) => s.toggleSidebar);
  const toggleHelp = useOnboardingStore((s) => s.toggle);
  const toggleAgentPanel = useAgentPanelStore((s) => s.toggle);
  const isAgentPanelOpen = useAgentPanelStore((s) => s.isOpen);
  const agentInTheLoopEnabled = FEATURE_FLAGS.AGENT_IN_THE_LOOP;
  const recentStocks = useWorkspaceStore((s) => s.recentStocks);

  const sections = useMemo<SidebarSection[]>(() => {
    const sections: SidebarSection[] = [];

    sections.push({
      key: "overview",
      labelKey: "sidebar.sectionOverview",
      items: builtinNavItems.filter((n) => n.key === "chat" || n.key === "dashboard"),
    });

    sections.push({
      key: "tools",
      labelKey: "sidebar.sectionTools",
      items: builtinNavItems.filter((n) => n.key === "knowledge" || n.key === "wiki"),
    });

    sections.push({
      key: "infrastructure",
      labelKey: "sidebar.sectionInfrastructure",
      items: builtinNavItems.filter((n) =>
        n.key === "gateway" || n.key === "terminal" || n.key === "files" || n.key === "workflow"
        || n.key === "marketplace" || n.key === "dynamic-ui"
      ),
    });

    sections.push({
      key: "invest-workspace",
      labelKey: "sidebar.sectionInvestWorkspace",
      items: builtinNavItems.filter((n) =>
        n.key === "workspace" || n.key === "screener" || n.key === "paper-portfolio"
        || n.key === "market-mainline" || n.key === "screenshot-diagnosis"
        || n.key === "multi-agent"
      ),
    });

    sections.push({
      key: "invest-review",
      labelKey: "sidebar.sectionInvestReview",
      items: builtinNavItems.filter((n) => n.key === "quant" || n.key === "pipeline"),
    });

    // 开发者工具分组 — 仅在设置开启时显示（默认 true）
    if (settings.show_developer_tools !== false) {
      sections.push({
        key: "developer",
        labelKey: "sidebar.sectionDeveloper",
        items: devtoolsNavItems,
      });
    }
>>>>>>> upstream-master

    return sections.filter((s) => s.items.length > 0);
  }, [settings.show_developer_tools]);

  // ── 固定在导航的动态页面 ──
  const dynamicSchemas = useDynamicUIStore((s) => s.schemas);
  const fetchSchemas = useDynamicUIStore((s) => s.fetchSchemas);
  const pins = useDynamicUIStore((s) => s.pins);
  const fetchPins = useDynamicUIStore((s) => s.fetchPins);
  useEffect(() => {
    void fetchSchemas();
    void fetchPins();
  }, [fetchSchemas, fetchPins]);

  const pinnedGroups = useMemo(() => {
    const schemasById = new Map(dynamicSchemas.map((s) => [s.id, s]));
    const pinnedMap: PinnedSchemaMap = {};
    for (const p of pins) {
      pinnedMap[p.schema_id] = {
        schemaId: p.schema_id,
        title: p.title,
        group: p.group_name,
        position: p.position,
      };
    }
    const grouped = getPinnedSchemasByGroup(pinnedMap);
    return grouped
      .map((g) => ({
        groupKey: g.group,
        labelKey: PIN_GROUPS.find((pg) => pg.key === g.group)?.labelKey ?? g.group,
        items: g.items
          .filter((c) => schemasById.has(c.schemaId))
          .map((c) => ({
            key: `dynamic-ui-${c.schemaId}`,
            icon: <AppstoreAddOutlined />,
            // 缺陷 6：优先使用 schema 实时标题，保证改名后导航项同步更新
            labelKey: schemasById.get(c.schemaId)?.title ?? c.title,
            path: `${BUILTIN_PAGE_PATH["dynamic-ui"]}/${c.schemaId}`,
            isPlugin: true,
          })),
      }))
      .filter((g) => g.items.length > 0);
  }, [dynamicSchemas, pins]);

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
          {/* invest-workspace section：展开时显示最近股票快捷列表 */}
          {section.key === "invest-workspace" && !sidebarCollapsed && recentStocks.length > 0 && (
            <div className="mt-0.5 mb-1 space-y-0.5">
              {!sidebarCollapsed && (
                <div
                  className="text-sm px-3 pt-1 pb-0.5"
                  style={{ color: "var(--muted)", fontSize: 11 }}
                >
                  {t("workspace.stockSwitcher.recent")}
                </div>
              )}
              {recentStocks.slice(0, 5).map((stock) => (
                <button
                  key={stock.code}
                  type="button"
                  onClick={() => navigate(`${BUILTIN_PAGE_PATH.workspace}/${stock.code}`)}
                  className="w-full flex items-center gap-1.5 px-3 py-1 rounded text-left transition-colors hover:opacity-70"
                  style={{ color: "var(--color-text-secondary)" }}
                >
                  <LineChart size={12} style={{ color: "var(--muted)", flexShrink: 0 }} />
                  <span className="text-sm truncate flex-1">{stock.name}</span>
                  <span className="text-sm font-mono" style={{ color: "var(--muted)", fontSize: 11 }}>
                    {stock.code}
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      ))}

      {/* 固定在导航的动态页面 */}
      {pinnedGroups.map((group) => (
        <div key={group.groupKey}>
          {!sidebarCollapsed && (
            <div className="ax-sidebar-section-header">
              {t(group.labelKey)}
            </div>
          )}
          {group.items.map((item) => (
            <Tooltip
              key={item.key}
              title={sidebarCollapsed ? item.labelKey : ""}
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
          ))}
        </div>
      ))}

      <div className="flex-1" />

      {/* Agent Panel toggle */}
      {agentInTheLoopEnabled && (
        <Tooltip
          title={sidebarCollapsed ? (isAgentPanelOpen ? t("sidebar.closeAgent") : t("sidebar.openAgent")) : ""}
          placement="right"
        >
          <button
            type="button"
            className={`nav-item${isAgentPanelOpen ? " active" : ""}`}
            onClick={toggleAgentPanel}
            aria-label={t("sidebar.agentPanel")}
            style={isAgentPanelOpen ? { color: "var(--color-primary)" } : undefined}
          >
            <Icon icon="fluent:bot-20-filled" size={17} />
            {!sidebarCollapsed && <span className="nav-label">Agent</span>}
          </button>
        </Tooltip>
      )}

      {/* Settings — lower group, above plugins in prototype */}
      <Tooltip title={sidebarCollapsed ? t("settings.openSettings") : ""} placement="right">
        <button
          type="button"
          className={`nav-item${activePage === "settings" ? " active" : ""}`}
          onClick={() => navigate(BUILTIN_PAGE_PATH.settings)}
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
