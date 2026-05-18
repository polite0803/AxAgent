import appLogo from "@/assets/image/logo.png";
import { useUpdateChecker } from "@/hooks/useUpdateChecker";
import { TITLEBAR_ICON_COLORS } from "@/lib/iconColors";
import { invoke, isTauri, logIpcError } from "@/lib/invoke";
import { formatShortcutForDisplay, getShortcutBinding } from "@/lib/shortcuts";
import { useBackupStore, useSettingsStore } from "@/stores";
import type { PageKey } from "@/types";
import { App, Divider, Dropdown, Popover, Space, Spin, theme, Tooltip, Typography } from "antd";
import type { MenuProps } from "antd";
import {
  ArrowDownCircle,
  Bug,
  CloudUpload,
  Dna,
  Ellipsis,
  Globe,
  MessageSquarePlus,
  Minus,
  Monitor,
  Moon,
  Pin,
  PinOff,
  RotateCcw,
  Settings,
  Square,
  Star,
  Sun,
  X,
  XCircle,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { NotificationBell } from "./NotificationBell";

const IS_WINDOWS = navigator.userAgent.includes("Windows");

interface EvolutionStatus {
  llm_provider_connected: boolean;
  skill_count: number;
  auto_tools_count: number;
}

function useEvolutionStatus() {
  const [status, setStatus] = useState<EvolutionStatus | null>(null);
  useEffect(() => {
    if (!isTauri()) { return; }
    const fetch = () => {
      invoke<EvolutionStatus>("get_evolution_stats")
        .then((s) =>
          setStatus({
            llm_provider_connected: s.llm_provider_connected,
            skill_count: s.skill_count,
            auto_tools_count: s.auto_tools_count,
          })
        )
        .catch(logIpcError("fetch_backend_status"));
    };
    fetch();
    const id = setInterval(fetch, 60000);
    return () => clearInterval(id);
  }, []);
  return status;
}

/** Standard Windows "restore down" icon: two overlapping rectangles */
const RestoreIcon = () => (
  <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.2">
    <rect x="3" y="5" width="8" height="7" rx="0.5" />
    <path d="M5 5 V3.5 A0.5 0.5 0 0 1 5.5 3 H12 A0.5 0.5 0 0 1 12.5 3.5 V10 A0.5 0.5 0 0 1 12 10.5 H10.5" />
  </svg>
);

const THEME_OPTIONS = [
  { key: "system", icon: <Monitor size={14} color={TITLEBAR_ICON_COLORS.Monitor} />, labelKey: "settings.themeSystem" },
  { key: "light", icon: <Sun size={14} color={TITLEBAR_ICON_COLORS.Sun} />, labelKey: "settings.themeLight" },
  { key: "dark", icon: <Moon size={14} color={TITLEBAR_ICON_COLORS.Moon} />, labelKey: "settings.themeDark" },
] as const;

import { LANG_OPTIONS } from "@/lib/constants";

export function TitleBar() {
  const { t, i18n } = useTranslation();
  const { token } = theme.useToken();
  const { modal, message } = App.useApp();
  const location = useLocation();
  const navigate = useNavigate();
  const activePage = location.pathname === "/settings" || location.pathname.startsWith("/settings/")
    ? "settings"
    : location.pathname === "/"
    ? "chat"
    : location.pathname.slice(1) as PageKey;
  const themeMode = useSettingsStore((s) => s.settings.theme_mode);
  const alwaysOnTop = useSettingsStore((s) => s.settings.always_on_top);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const settings = useSettingsStore((s) => s.settings);

  const isInSettings = activePage === "settings";
  const [pinned, setPinned] = useState(alwaysOnTop ?? false);

  useEffect(() => {
    setPinned(alwaysOnTop ?? false);
  }, [alwaysOnTop]);

  const handlePinToggle = useCallback(async () => {
    const next = !pinned;
    setPinned(next);
    try {
      await invoke("set_always_on_top", { enabled: next });
      saveSettings({ always_on_top: next });
    } catch {
      setPinned(!next);
    }
  }, [pinned, saveSettings]);

  const { checkForUpdate } = useUpdateChecker();

  const handleCheckUpdate = useCallback(async () => {
    await checkForUpdate();
  }, [checkForUpdate]);

  const themeMenuItems: MenuProps["items"] = THEME_OPTIONS.map((opt) => ({
    key: opt.key,
    icon: opt.icon,
    label: t(opt.labelKey),
  }));

  const langMenuItems: MenuProps["items"] = LANG_OPTIONS.map((opt) => ({
    key: opt.key,
    icon: <span>{opt.icon}</span>,
    label: opt.label,
  }));

  const handleSettingsToggle = () => {
    if (isInSettings) {
      navigate("/");
    } else {
      navigate("/settings");
    }
  };

  const handleReload = useCallback(() => {
    modal.confirm({
      title: t("desktop.reloadConfirmTitle"),
      content: t("desktop.reloadConfirmContent"),
      okText: t("desktop.reloadConfirmOk"),
      cancelText: t("desktop.reloadConfirmCancel"),
      onOk: () => {
        window.location.reload();
      },
    });
  }, [modal, t]);

  // Windows window controls
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    if (!IS_WINDOWS || !isTauri()) { return; }
    let unlisten: (() => void) | undefined;
    (async () => {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      setIsMaximized(await win.isMaximized());
      unlisten = await win.onResized(async () => {
        setIsMaximized(await win.isMaximized());
      });
    })();
    return () => {
      unlisten?.();
    };
  }, []);

  const handleWindowMinimize = useCallback(async () => {
    await invoke("minimize_window");
  }, []);

  const handleWindowMaximize = useCallback(async () => {
    await invoke("toggle_maximize_window");
  }, []);

  const handleWindowClose = useCallback(async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  }, []);

  // Quick Backup state
  const [backupPopoverOpen, setBackupPopoverOpen] = useState(false);
  const [backingUp, setBackingUp] = useState<"local" | "webdav" | null>(null);
  const [lastLocalBackup, setLastLocalBackup] = useState<string | null>(null);
  const [lastWebDavSync, setLastWebDavSync] = useState<string | null>(null);
  // Timestamps (ms) for next scheduled backups
  const nextLocalTsRef = useRef<number | null>(null);
  const nextWebDavTsRef = useRef<number | null>(null);
  // Live countdown strings (updated every second)
  const [countdownText, setCountdownText] = useState<string | null>(null);
  const [popoverLocalCountdown, setPopoverLocalCountdown] = useState<string | null>(null);
  const [popoverWebDavCountdown, setPopoverWebDavCountdown] = useState<string | null>(null);

  const { backupSettings, loadBackupSettings } = useBackupStore();
  const evolutionStatus = useEvolutionStatus();

  const fmtCountdown = (ms: number) => {
    if (ms <= 0) { return t("titlebar.now"); }
    const h = Math.floor(ms / 3600000);
    const m = Math.floor((ms % 3600000) / 60000);
    const s = Math.floor((ms % 60000) / 1000);
    if (h > 0) { return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`; }
    return `${m}:${s.toString().padStart(2, "0")}`;
  };

  // Fetch backup info on mount and when popover opens
  useEffect(() => {
    loadBackupSettings();

    invoke<{ lastSyncTime: string | null }>("get_webdav_sync_status")
      .then((s) => {
        if (s.lastSyncTime) {
          const d = new Date(s.lastSyncTime);
          if (!Number.isNaN(d.getTime())) {
            setLastWebDavSync(d.toLocaleString());
          }
        }
      })
      .catch((e: unknown) => {
        console.warn("[IPC]", e);
      });

    invoke<Array<{ createdAt: string }>>("list_backups")
      .then((list) => {
        if (list.length > 0) {
          const raw = list[0].createdAt;
          const d = new Date(raw.includes("T") || raw.includes("Z") ? raw : raw + "Z");
          if (!Number.isNaN(d.getTime())) { setLastLocalBackup(d.toLocaleString()); }
        }
      })
      .catch((e: unknown) => {
        console.warn("[IPC]", e);
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [backupPopoverOpen]);

  // Calculate next WebDAV sync timestamp (re-run when settings or lastWebDavSync change)
  useEffect(() => {
    if (!lastWebDavSync) {
      nextWebDavTsRef.current = null;
      return;
    }
    const d = new Date(lastWebDavSync);
    if (Number.isNaN(d.getTime())) { return; }
    const interval = settings.webdav_sync_interval_minutes ?? 60;
    if (settings.webdav_sync_enabled && interval > 0) {
      const intervalMs = interval * 60000;
      let next = d.getTime() + intervalMs;
      // If overdue, advance to the next future interval
      while (next < Date.now()) {
        next += intervalMs;
      }
      nextWebDavTsRef.current = next;
    } else {
      nextWebDavTsRef.current = null;
    }
  }, [settings.webdav_sync_enabled, settings.webdav_sync_interval_minutes, lastWebDavSync]);

  // Calculate next local backup timestamp from backup settings
  useEffect(() => {
    if (!backupSettings?.enabled) {
      nextLocalTsRef.current = null;
      return;
    }
    const intervalMs = (backupSettings.intervalHours ?? 24) * 3600000;
    if (lastLocalBackup) {
      const lastTime = new Date(lastLocalBackup).getTime();
      if (!Number.isNaN(lastTime)) {
        let next = lastTime + intervalMs;
        // If overdue, advance to the next future interval
        while (next < Date.now()) {
          next += intervalMs;
        }
        nextLocalTsRef.current = next;
        return;
      }
    }
    // No previous backup — next backup at now + interval
    nextLocalTsRef.current = Date.now() + intervalMs;
  }, [backupSettings, lastLocalBackup]);

  // Live countdown on button — tick every second while any backup is scheduled
  useEffect(() => {
    const tick = () => {
      const now = Date.now();
      let soonest: number | null = null;
      if (nextLocalTsRef.current) {
        if (!soonest || nextLocalTsRef.current < soonest) { soonest = nextLocalTsRef.current; }
      }
      if (nextWebDavTsRef.current) {
        if (!soonest || nextWebDavTsRef.current < soonest) { soonest = nextWebDavTsRef.current; }
      }
      if (soonest) {
        setCountdownText(fmtCountdown(soonest - now));
      } else {
        setCountdownText(null);
      }
    };

    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, []);

  // Live countdown in popover — tick every second only when open
  useEffect(() => {
    if (!backupPopoverOpen) { return; }
    const tick = () => {
      const now = Date.now();
      if (nextLocalTsRef.current && nextLocalTsRef.current > now) {
        setPopoverLocalCountdown(
          `${new Date(nextLocalTsRef.current).toLocaleString()} (${fmtCountdown(nextLocalTsRef.current - now)})`,
        );
      } else {
        setPopoverLocalCountdown(null);
      }
      if (nextWebDavTsRef.current && nextWebDavTsRef.current > now) {
        setPopoverWebDavCountdown(
          `${new Date(nextWebDavTsRef.current).toLocaleString()} (${fmtCountdown(nextWebDavTsRef.current - now)})`,
        );
      } else {
        setPopoverWebDavCountdown(null);
      }
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [backupPopoverOpen]);

  const handleQuickBackup = useCallback(async (type: "local" | "webdav") => {
    setBackingUp(type);
    try {
      if (type === "local") {
        await invoke("create_backup", { format: "sqlite" });
      } else {
        await invoke("webdav_backup");
      }
      message.success(t("backup.backupSuccess"));
      setBackupPopoverOpen(false);
    } catch (e) {
      message.error(String(e));
    } finally {
      setBackingUp(null);
    }
  }, [message, t]);

  const GITHUB_REPO = "https://github.com/polite0803/AxAgent";
  const githubMenuItems: MenuProps["items"] = [
    {
      key: "feature",
      icon: <MessageSquarePlus size={14} color={TITLEBAR_ICON_COLORS.MessageSquarePlus} />,
      label: t("titlebar.submitFeature"),
    },
    {
      key: "bug",
      icon: <Bug size={14} color={TITLEBAR_ICON_COLORS.Bug} />,
      label: t("titlebar.submitBug"),
    },
    { type: "divider" },
    {
      key: "star",
      icon: <Star size={14} color={TITLEBAR_ICON_COLORS.Star} />,
      label: t("titlebar.giveStar"),
    },
  ];
  const handleGithubClick = ({ key }: { key: string }) => {
    let url = GITHUB_REPO;
    if (key === "feature") { url = `${GITHUB_REPO}/issues/new?labels=enhancement&template=feature_request.yml`; }
    else if (key === "bug") { url = `${GITHUB_REPO}/issues/new?labels=bug&template=bug_report.yml`; }
    if (isTauri()) {
      import("@tauri-apps/plugin-opener").then(({ openUrl }) => openUrl(url)).catch(() =>
        window.open(url, "_blank", "noopener,noreferrer")
      );
    } else {
      window.open(url, "_blank", "noopener,noreferrer");
    }
  };

  // Pre-load Tauri window module for synchronous drag calls
  const tauriWindowRef = useRef<typeof import("@tauri-apps/api/window") | null>(null);
  useEffect(() => {
    if (isTauri()) {
      import("@tauri-apps/api/window").then((mod) => {
        tauriWindowRef.current = mod;
      });
    }
  }, []);

  const dragTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleDragMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement;
    if (target.closest("button")) { return; }
    const mod = tauriWindowRef.current;
    if (!mod) { return; }
    e.preventDefault();

    if (IS_WINDOWS) {
      // Delay startDragging slightly so double-click can be detected.
      // If a second mousedown arrives within the threshold,
      // the onDoubleClick handler fires and cancels the pending drag.
      if (dragTimerRef.current) { clearTimeout(dragTimerRef.current); }
      dragTimerRef.current = setTimeout(() => {
        mod.getCurrentWindow().startDragging();
      }, 200);
    } else {
      mod.getCurrentWindow().startDragging();
    }
  }, []);

  const handleTitleBarDoubleClick = useCallback(() => {
    if (!IS_WINDOWS) { return; }
    if (dragTimerRef.current) {
      clearTimeout(dragTimerRef.current);
      dragTimerRef.current = null;
    }
    invoke("toggle_maximize_window");
  }, []);

  return (
    <div
      className="title-bar-drag ax-titlebar-compact"
      role="toolbar"
      tabIndex={-1}
      {...(!IS_WINDOWS ? { "data-tauri-drag-region": true } : {})}
      onMouseDown={handleDragMouseDown}
      onDoubleClick={IS_WINDOWS ? handleTitleBarDoubleClick : undefined}
      style={{
        height: 28,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        paddingLeft: IS_WINDOWS ? 12 : 72,
        paddingRight: IS_WINDOWS ? 0 : 12,
        backgroundColor: "transparent",
        flexShrink: 0,
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
        position: "relative",
      }}
    >
      {/* Left: App icon + name (Windows only) */}
      {IS_WINDOWS
        ? (
          <div className="title-bar-nodrag" style={{ display: "flex", alignItems: "center", gap: 6, marginRight: 8 }}>
            <img
              src={appLogo}
              alt={t("app.name")}
              style={{ width: 18, height: 18, filter: "drop-shadow(0 0 4px rgba(0,240,255,0.3))" }}
              draggable={false}
            />
            <span
              style={{
                fontSize: 13,
                fontWeight: 600,
                color: token.colorTextBase,
                userSelect: "none",
                letterSpacing: "0.04em",
              }}
            >
              {t("app.title")}
            </span>
            {evolutionStatus && evolutionStatus.llm_provider_connected && (
              <Tooltip title={t("titlebar.evolutionActive")}>
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    marginLeft: 4,
                    color: token.colorSuccess,
                    animation: "pulse-glow 2s ease-in-out infinite",
                  }}
                >
                  <Dna size={12} />
                </span>
              </Tooltip>
            )}
          </div>
        )
        : <div />}

      <div style={{ display: "flex", alignItems: "center", gap: 0 }}>
        <div className="title-bar-nodrag" style={{ display: "flex", alignItems: "center", gap: 4 }}>
          {/* Pin Toggle */}
          <Tooltip title={t("desktop.alwaysOnTop")}>
            <button
              className="ax-titlebar-btn"
              onClick={handlePinToggle}
              style={{
                color: pinned ? token.colorPrimary : token.colorTextSecondary,
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.color = pinned
                  ? token.colorPrimary
                  : token.colorTextBase;
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.color = pinned
                  ? token.colorPrimary
                  : token.colorTextSecondary;
              }}
            >
              {pinned
                ? <Pin size={12} color={TITLEBAR_ICON_COLORS.Pin} />
                : <PinOff size={12} color={TITLEBAR_ICON_COLORS.PinOff} />}
            </button>
          </Tooltip>

          {/* Appearance: theme + language combined */}
          <Dropdown
            menu={{
              items: [
                { type: "group", label: t("settings.groupTheme"), children: themeMenuItems },
                { type: "divider" },
                ...langMenuItems,
              ],
              onClick: ({ key }) => {
                if (THEME_OPTIONS.some((o) => o.key === key)) {
                  saveSettings({ theme_mode: key });
                } else {
                  i18n.changeLanguage(key);
                  saveSettings({ language: key });
                }
              },
              selectedKeys: [themeMode, i18n.language],
            }}
            trigger={["click"]}
            placement="bottomRight"
            destroyOnHidden
          >
            <button
              className="ax-titlebar-btn"
              style={{ color: token.colorTextSecondary }}
            >
              <Globe size={12} color={TITLEBAR_ICON_COLORS.Globe} />
            </button>
          </Dropdown>

          {/* Quick Backup */}
          <Popover
            open={backupPopoverOpen}
            onOpenChange={setBackupPopoverOpen}
            trigger="click"
            placement="bottomRight"
            destroyOnHidden
            content={
              <div style={{ width: 240 }}>
                <Typography.Text strong style={{ fontSize: 13 }}>
                  {t("titlebar.lastBackup")}
                </Typography.Text>
                <Space orientation="vertical" size={2} style={{ width: "100%", marginTop: 4 }}>
                  {lastLocalBackup && (
                    <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                      {t("titlebar.lastLocal")}: {lastLocalBackup}
                    </Typography.Text>
                  )}
                  {lastWebDavSync && (
                    <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                      WebDAV: {lastWebDavSync}
                    </Typography.Text>
                  )}
                  {!lastLocalBackup && !lastWebDavSync && (
                    <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                      {t("titlebar.noBackupYet")}
                    </Typography.Text>
                  )}
                </Space>

                {(popoverLocalCountdown || popoverWebDavCountdown) && (
                  <>
                    <Divider style={{ margin: "6px 0" }} />
                    <Typography.Text strong style={{ fontSize: 13 }}>
                      {t("titlebar.nextBackup")}
                    </Typography.Text>
                    <Space orientation="vertical" size={2} style={{ width: "100%", marginTop: 4 }}>
                      {popoverLocalCountdown && (
                        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                          {t("titlebar.lastLocal")}: {popoverLocalCountdown}
                        </Typography.Text>
                      )}
                      {popoverWebDavCountdown && (
                        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                          WebDAV: {popoverWebDavCountdown}
                        </Typography.Text>
                      )}
                    </Space>
                  </>
                )}

                <Divider style={{ margin: "6px 0" }} />
                <Space orientation="vertical" size={8} style={{ width: "100%" }}>
                  <button
                    onClick={() => handleQuickBackup("local")}
                    disabled={backingUp !== null}
                    style={{
                      width: "100%",
                      padding: "4px 8px",
                      borderRadius: token.borderRadius,
                      border: `1px solid ${token.colorBorder}`,
                      backgroundColor: "transparent",
                      cursor: backingUp ? "not-allowed" : "pointer",
                      display: "flex",
                      alignItems: "center",
                      gap: 6,
                      color: token.colorText,
                    }}
                  >
                    {backingUp === "local"
                      ? <Spin size="small" />
                      : <CloudUpload size={14} color={TITLEBAR_ICON_COLORS.CloudUpload} />}
                    {t("titlebar.localBackup")}
                  </button>
                  <button
                    onClick={() => handleQuickBackup("webdav")}
                    disabled={backingUp !== null}
                    style={{
                      width: "100%",
                      padding: "4px 8px",
                      borderRadius: token.borderRadius,
                      border: `1px solid ${token.colorBorder}`,
                      backgroundColor: "transparent",
                      cursor: backingUp ? "not-allowed" : "pointer",
                      display: "flex",
                      alignItems: "center",
                      gap: 6,
                      color: token.colorText,
                    }}
                  >
                    {backingUp === "webdav"
                      ? <Spin size="small" />
                      : <CloudUpload size={14} color={TITLEBAR_ICON_COLORS.CloudUpload} />}
                    {t("titlebar.webdavBackup")}
                  </button>
                </Space>
              </div>
            }
          >
            <Tooltip title={t("titlebar.quickBackup")}>
              <button
                className="ax-titlebar-btn"
                style={{
                  color: countdownText ? token.colorPrimary : token.colorTextSecondary,
                  width: countdownText ? "auto" : 28,
                  paddingInline: countdownText ? 4 : 0,
                  gap: 2,
                  fontSize: 12,
                }}
              >
                <CloudUpload size={12} color={TITLEBAR_ICON_COLORS.CloudUpload} />
                {countdownText && <span>({countdownText})</span>}
              </button>
            </Tooltip>
          </Popover>

          {/* More: GitHub, check update, reload */}
          <Dropdown
            menu={{
              items: [
                ...githubMenuItems,
                { type: "divider" },
                ...(isTauri()
                  ? [{
                    key: "checkUpdate",
                    icon: <ArrowDownCircle size={12} color={TITLEBAR_ICON_COLORS.ArrowDownCircle} />,
                    label: t("settings.checkUpdate"),
                  }]
                  : []),
                {
                  key: "reload",
                  icon: <RotateCcw size={12} color={TITLEBAR_ICON_COLORS.RotateCcw} />,
                  label: t("desktop.reloadPage"),
                },
              ] as MenuProps["items"],
              onClick: ({ key }) => {
                if (key === "reload") { handleReload(); }
                else if (key === "checkUpdate") { handleCheckUpdate(); }
                else { handleGithubClick({ key }); }
              },
            }}
            trigger={["click"]}
            placement="bottomRight"
            destroyOnHidden
          >
            <button
              className="ax-titlebar-btn"
              style={{ color: token.colorTextSecondary }}
            >
              <Ellipsis size={12} color={TITLEBAR_ICON_COLORS.GitFork} />
            </button>
          </Dropdown>

          {/* Notification Bell */}
          <NotificationBell />

          {/* Settings Toggle */}
          <Tooltip
            title={`${isInSettings ? t("settings.closeSettings") : t("settings.openSettings")} (${
              formatShortcutForDisplay(getShortcutBinding(settings, "openSettings"))
            })`}
          >
            <button
              className="ax-titlebar-btn"
              data-testid="settings-nav-btn"
              onClick={(e) => {
                handleSettingsToggle();
                e.currentTarget.blur();
              }}
              style={{
                color: isInSettings ? token.colorError : token.colorTextSecondary,
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.color = isInSettings
                  ? token.colorError
                  : token.colorTextBase;
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.color = isInSettings
                  ? token.colorError
                  : token.colorTextSecondary;
              }}
            >
              {isInSettings
                ? <XCircle size={12} color={TITLEBAR_ICON_COLORS.XCircle} />
                : <Settings size={12} color={TITLEBAR_ICON_COLORS.Settings} />}
            </button>
          </Tooltip>
        </div>

        {/* Windows window controls */}
        {IS_WINDOWS && isTauri() && (
          <div className="title-bar-nodrag" style={{ display: "flex", alignItems: "center", marginLeft: 4 }}>
            {/* Minimize */}
            <button
              className="ax-titlebar-winctrl"
              onClick={handleWindowMinimize}
              style={{ color: token.colorTextSecondary }}
            >
              <Minus size={16} />
            </button>
            <button
              className="ax-titlebar-winctrl"
              onClick={handleWindowMaximize}
              style={{ color: token.colorTextSecondary }}
            >
              {isMaximized ? <RestoreIcon /> : <Square size={14} />}
            </button>
            <button
              className="ax-titlebar-winctrl ax-titlebar-winctrl--close"
              onClick={handleWindowClose}
              style={{ color: token.colorTextSecondary, borderRadius: 0 }}
            >
              <X size={16} />
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
