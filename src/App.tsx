import { BuddyWidget } from "@/components/chat/BuddyWidget";
import { HelpPanel } from "@/components/help/HelpPanel";
import { CommandPalette } from "@/components/layout/CommandPalette";
import { ContentArea } from "@/components/layout/ContentArea";
import { GlobalCopyMenu } from "@/components/layout/GlobalCopyMenu";
import { GlobalErrorBoundary } from "@/components/layout/GlobalErrorBoundary";
import { MobileBottomNav } from "@/components/layout/MobileBottomNav";
import { ModuleErrorBoundary } from "@/components/layout/ModuleErrorBoundary";
import { Sidebar } from "@/components/layout/Sidebar";
import { TitleBar } from "@/components/layout/TitleBar";
import { InteractiveTutorial } from "@/components/onboarding/InteractiveTutorial";
import { WelcomeWizard } from "@/components/onboarding/WelcomeWizard";
import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { SkillPanels } from "@/components/skill/SkillPanels";
import { SkillStatusBar } from "@/components/skill/SkillStatusBar";
import { useCommandPalette } from "@/hooks/useCommandPalette";
import { useGlobalOverlayScrollbars } from "@/hooks/useGlobalOverlayScrollbars";
import { useGlobalShortcutManager } from "@/hooks/useGlobalShortcutManager";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { useResolvedDarkMode } from "@/hooks/useResolvedDarkMode";
import { useResponsive } from "@/hooks/useResponsive";
import { useUpdateChecker } from "@/hooks/useUpdateChecker";
import { checkIpcHealth, invoke, isTauri, listen } from "@/lib/invoke";
import { preloadChatRenderers } from "@/lib/preloadChatRenderers";
import {
  useConversationStore,
  useOnboardingStore,
  useSettingsStore,
  useSkillExtensionStore,
  useStreamStore,
  useUIStore,
} from "@/stores";
import { useShadcnTheme } from "@/theme/shadcnTheme";
import type { ThemePreset } from "@/theme/shadcnTheme";
import { App as AntdApp, ConfigProvider, Drawer, Layout, theme } from "antd";
import { enableD2, setDefaultI18nMap } from "markstream-react";
import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { BrowserRouter, useLocation, useNavigate } from "react-router-dom";
import "./i18n";
// antd 语言包 — 静态导入确保 Rolldown 正确打包
import antdArEG from "antd/locale/ar_EG";
import antdDeDE from "antd/locale/de_DE";
import antdEnUS from "antd/locale/en_US";
import antdEsES from "antd/locale/es_ES";
import antdFrFR from "antd/locale/fr_FR";
import antdHiIN from "antd/locale/hi_IN";
import antdJaJP from "antd/locale/ja_JP";
import antdKoKR from "antd/locale/ko_KR";
import antdRuRU from "antd/locale/ru_RU";
import antdZhCN from "antd/locale/zh_CN";
import antdZhTW from "antd/locale/zh_TW";

const LazyQuickBarPage = lazy(() => import("@/pages/QuickBarPage").then((m) => ({ default: m.QuickBarPage })));

const { Content } = Layout;
const { useToken } = theme;

/** Show the main window (it starts hidden to avoid white flash). */
async function showWindow() {
  try {
    const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    await getCurrentWebviewWindow().show();
  } catch (e) {
    console.warn("Failed to show window:", e);
  }
}

/** 仅当技能扩展注册了状态栏项时才渲染 */
function ConditionalSkillStatusBar() {
  const count = useSkillExtensionStore((s) => s.statusBarItems.length);
  if (count === 0) { return null; }
  return (
    <ModuleErrorBoundary moduleName="SkillStatusBar">
      <SkillStatusBar alignment="right" />
    </ModuleErrorBoundary>
  );
}

function AppInner() {
  const { token } = useToken();
  const { t } = useTranslation();
  const { modal } = AntdApp.useApp();
  const location = useLocation();
  const navigate = useNavigate();
  const { open: cmdOpen, setOpen: setCmdOpen } = useCommandPalette();
  const isInSettings = location.pathname === "/settings"
    || location.pathname.startsWith("/settings/");
  const deviceLayout = useUIStore((s) => s.deviceLayout);
  const mobileNavOpen = useUIStore((s) => s.mobileNavOpen);
  const setMobileNavOpen = useUIStore((s) => s.setMobileNavOpen);

  // 同步检测 QuickBar 窗口（在首次渲染前），避免 ChatPage 先渲染导致崩溃
  const [isQuickBarWindow] = useState(() => {
    const params = new URLSearchParams(window.location.search);
    return params.get("__route") === "quickbar";
  });
  const isQuickBar = isQuickBarWindow || location.pathname === "/quickbar";

  // Navigate to /quickbar if the app is loaded in the quickbar window
  useEffect(() => {
    if (isQuickBarWindow) {
      navigate("/quickbar", { replace: true });
      return;
    }
    if (isTauri()) {
      import("@tauri-apps/api/webviewWindow").then(
        ({ getCurrentWebviewWindow }) => {
          try {
            const label = getCurrentWebviewWindow().label;
            if (label === "quickbar") {
              navigate("/quickbar", { replace: true });
            }
          } catch {
            /* not a Tauri webview window */
          }
        },
      );
    }
  }, [navigate]);

  // These hooks use useNavigate() and must be inside BrowserRouter
  useKeyboardShortcuts();
  useGlobalShortcutManager();
  useGlobalOverlayScrollbars();
  // 自动检测桌面分辨率，设置 deviceLayout
  useResponsive();

  // Handle app close confirmation from backend
  const handleCloseRequested = useCallback(() => {
    modal.confirm({
      title: t("desktop.closeConfirmTitle"),
      content: t("desktop.closeConfirmContent"),
      okText: t("desktop.closeConfirmOk"),
      cancelText: t("desktop.closeConfirmCancel"),
      okButtonProps: { danger: true },
      onOk: () => invoke("force_quit"),
    });
  }, [modal, t]);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }
    const unlisten = listen("app-close-requested", handleCloseRequested);
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [handleCloseRequested]);

  // Sync Ant Design tokens to CSS custom properties for global usage
  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty("--border-color", token.colorBorderSecondary);
    root.style.setProperty("--color-bg-container", token.colorBgContainer);
    root.style.setProperty("--color-bg-elevated", token.colorBgElevated);
    root.style.setProperty("--color-text", token.colorText);
    root.style.setProperty("--color-text-secondary", token.colorTextSecondary);
    root.style.setProperty("--color-primary", token.colorPrimary);
    root.style.setProperty("--color-fill-alter", token.colorFillAlter);
    // Markdown renderer (markstream-react) CSS variables
    root.style.setProperty("--table-border", token.colorBorderSecondary);
    root.style.setProperty("--hr-border-color", token.colorBorderSecondary);
    root.style.setProperty(
      "--blockquote-border-color",
      token.colorBorderSecondary,
    );
  }, [token]);

  // Global stream event listeners — persist across page navigation
  const startStreamListening = useConversationStore(
    (s) => s.startStreamListening,
  );
  const stopStreamListening = useStreamStore((s) => s.stopStreamListening);
  useEffect(() => {
    startStreamListening();
    return () => stopStreamListening();
  }, [startStreamListening, stopStreamListening]);

  // 加载技能前端扩展
  const fetchSkills = useSkillExtensionStore((s) => s.fetchSkills);
  useEffect(() => {
    fetchSkills().catch((e: unknown) => {
      console.warn("[启动] list_skills 失败:", e);
    });
  }, [fetchSkills]);

  // 加载引导状态
  const loadOnboarding = useOnboardingStore((s) => s.loadFromSettings);
  useEffect(() => {
    loadOnboarding();
  }, []);

  // Auto-check for updates on startup and periodically
  const { checkForUpdate } = useUpdateChecker();
  const updateCheckInterval = useSettingsStore(
    (s) => s.settings.update_check_interval ?? 60,
  );
  const updateIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }
    // Initial check after 3s delay
    const timer = setTimeout(() => checkForUpdate({ silent: true }), 3000);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!isTauri() || !updateCheckInterval) {
      return;
    }
    if (updateIntervalRef.current) {
      clearInterval(updateIntervalRef.current);
    }
    const intervalMs = Math.max(updateCheckInterval, 1) * 60 * 1000;
    updateIntervalRef.current = setInterval(
      () => checkForUpdate({ silent: true }),
      intervalMs,
    );
    return () => {
      if (updateIntervalRef.current) {
        clearInterval(updateIntervalRef.current);
      }
    };
  }, [updateCheckInterval, checkForUpdate]);

  return (
    <>
      <div
        className="flex flex-col h-screen"
        style={{ backgroundColor: token.colorBgContainer }}
      >
        {isQuickBar
          ? (
            isQuickBarWindow && location.pathname !== "/quickbar"
              ? (
                <Suspense
                  fallback={
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        height: "100%",
                      }}
                    />
                  }
                >
                  <PageErrorBoundary title="QuickBar">
                    <LazyQuickBarPage />
                  </PageErrorBoundary>
                </Suspense>
              )
              : <ContentArea />
          )
          : (
            <>
              <SkillPanels />
              <ModuleErrorBoundary moduleName="TitleBar">
                <TitleBar />
              </ModuleErrorBoundary>
              <ConditionalSkillStatusBar />
              <CommandPalette open={cmdOpen} onClose={() => setCmdOpen(false)} />
              <GlobalCopyMenu />
              {/* 移动端：滑出式导航抽屉 + 全宽内容区 + 底部导航栏 */}
              {deviceLayout === "mobile" && (
                <>
                  <Drawer
                    open={mobileNavOpen}
                    onClose={() => setMobileNavOpen(false)}
                    placement="left"
                    width={280}
                    styles={{ body: { padding: 0 } }}
                    closeIcon={null}
                  >
                    <ModuleErrorBoundary moduleName="Sidebar">
                      <Sidebar />
                    </ModuleErrorBoundary>
                  </Drawer>
                  <div className="flex-1 overflow-hidden">
                    <div
                      className="ax-page-transition"
                      style={{ height: "100%" }}
                      key={location.key}
                    >
                      <ContentArea />
                    </div>
                  </div>
                  {/* 浮动导航 — position:fixed，不占布局空间 */}
                  <MobileBottomNav />
                </>
              )}
              {/* 平板/桌面：固定侧边栏 + 内容区 */}
              {deviceLayout !== "mobile" && (
                <Layout
                  hasSider={!isInSettings}
                  className="flex-1 overflow-hidden"
                  style={{ backgroundColor: "transparent" }}
                >
                  {!isInSettings && (
                    <div
                      style={{
                        backgroundColor: "transparent",
                        borderRight: "1px solid var(--border-color)",
                        flexShrink: 0,
                      }}
                    >
                      <ModuleErrorBoundary moduleName="Sidebar">
                        <Sidebar />
                      </ModuleErrorBoundary>
                    </div>
                  )}
                  <Content className="overflow-hidden">
                    <div
                      className="ax-page-transition"
                      style={{ height: "100%" }}
                      key={location.key}
                    >
                      <ContentArea />
                    </div>
                  </Content>
                </Layout>
              )}
            </>
          )}
      </div>
      <WelcomeWizard />
      <InteractiveTutorial />
      <HelpPanel />
      <BuddyWidget />
    </>
  );
}

function AppRoot() {
  const { i18n } = useTranslation();
  const themeMode = useSettingsStore((s) => s.settings.theme_mode);
  const primaryColor = useSettingsStore((s) => s.settings.primary_color);
  const themePreset = useSettingsStore((s) => s.settings.theme_preset) as
    | ThemePreset
    | undefined;
  const fontSize = useSettingsStore((s) => s.settings.font_size);
  const fontWeight = useSettingsStore((s) => s.settings.font_weight);
  const fontFamily = useSettingsStore((s) => s.settings.font_family);
  const codeFontFamily = useSettingsStore((s) => s.settings.code_font_family);
  const borderRadius = useSettingsStore((s) => s.settings.border_radius);
  const language = useSettingsStore((s) => s.settings.language);
  const isDark = useResolvedDarkMode(themeMode, themePreset);

  const localeMap = useMemo<Record<string, string>>(
    () => ({
      "zh-CN": "zh_CN",
      "zh-TW": "zh_TW",
      ja: "ja_JP",
      ko: "ko_KR",
      de: "de_DE",
      fr: "fr_FR",
      es: "es_ES",
      ru: "ru_RU",
      hi: "hi_IN",
      ar: "ar_EG",
      "pt-BR": "pt_BR",
    }),
    [],
  );

  const staticLocaleMap = useMemo<Record<string, any>>(
    () => ({
      zh_CN: antdZhCN,
      zh_TW: antdZhTW,
      en_US: antdEnUS,
      ja_JP: antdJaJP,
      ko_KR: antdKoKR,
      de_DE: antdDeDE,
      fr_FR: antdFrFR,
      es_ES: antdEsES,
      ru_RU: antdRuRU,
      hi_IN: antdHiIN,
      ar_EG: antdArEG,
    }),
    [],
  );

  const antdLocale = useMemo(() => {
    if (localeMap[language]) {
      return staticLocaleMap[localeMap[language]] ?? antdZhCN;
    }
    if (language?.startsWith("zh")) {
      return antdZhCN;
    }
    return staticLocaleMap["en_US"];
  }, [language, localeMap, staticLocaleMap]);

  useEffect(() => {
    document.documentElement.dataset.theme = isDark ? "dark" : "light";
  }, [isDark]);

  useEffect(() => {
    enableD2(() => import("@terrastruct/d2"));
    void preloadChatRenderers();
  }, []);

  // Load persisted settings from backend on startup, then apply native settings
  // 已有 cleanup (return () => timers.forEach(clearTimeout))，react-doctor 误报
  // eslint-disable-next-line react-doctor/effect-needs-cleanup
  useEffect(() => {
    const timers: ReturnType<typeof setTimeout>[] = [];
    const init = async () => {
      const t0 = performance.now();

      if (isTauri()) {
        const health = await checkIpcHealth();
        if (!health.ok) {
          console.warn(`[启动] IPC 健康检查失败: ${health.detail}`);
          await new Promise((r) => {
            const t = setTimeout(r, 2000);
            timers.push(t);
          });
          const retry = await checkIpcHealth();
          if (!retry.ok) {
            console.error(`[启动] IPC 重试仍失败: ${retry.detail}`);
          }
        }
      }

      try {
        await useSettingsStore.getState().fetchSettings();
      } catch (e) {
        console.warn(
          `[启动] get_settings 失败 (${Math.round(performance.now() - t0)}ms):`,
          e,
        );
      }

      // 注意：预设工作流模板不再在启动时自动导入。
      // 用户可通过工作流管理页面"从预设导入"按钮按需触发 seed_preset_templates 命令。

      if (!isTauri()) {
        return;
      }
      const settings = useSettingsStore.getState().settings;

      try {
        await invoke("apply_startup_settings", {
          alwaysOnTop: settings.always_on_top ?? false,
          closeToTray: settings.minimize_to_tray ?? false,
        });
      } catch (e) {
        console.warn(
          `[启动] apply_startup_settings 失败 (${Math.round(performance.now() - t0)}ms):`,
          e,
        );
      }

      // Autostart (skip in dev mode — exe path doesn't exist)
      if (!import.meta.env.DEV) {
        try {
          const { enable, disable } = await import("@tauri-apps/plugin-autostart");
          if (settings.auto_start) {
            await enable();
          } else {
            await disable();
          }
        } catch (e) {
          const errorStr = String(e);
          if (errorStr.includes("os error 2")) {
            console.debug(
              "Autostart skipped: executable path not found (may occur in portable mode)",
            );
          } else {
            console.warn("Failed to set autostart:", e);
          }
        }
      }

      // Show window after initialization (window starts hidden to avoid white flash)
      await showWindow();
    };
    init();
    return () => timers.forEach(clearTimeout);
  }, []);

  // Sync i18n language with settings store
  useEffect(() => {
    if (i18n.language !== language) {
      i18n.changeLanguage(language);
    }
  }, [i18n, language]);

  useEffect(() => {
    const t = i18n.getFixedT(i18n.language);
    setDefaultI18nMap({
      "common.close": t("common.close"),
      "common.collapse": t("common.collapse"),
      "common.copied": t("common.copied"),
      "common.copy": t("common.copy"),
      "common.decrease": t("common.decrease"),
      "common.expand": t("common.expand"),
      "common.export": t("common.export"),
      "common.increase": t("common.increase"),
      "common.minimize": t("common.minimize"),
      "common.open": t("common.open"),
      "common.preview": t("common.preview"),
      "common.reset": t("common.reset"),
      "common.resetZoom": t("common.resetZoom"),
      "common.source": t("common.source"),
      "common.zoomIn": t("common.zoomIn"),
      "common.zoomOut": t("common.zoomOut"),
      "image.loadError": t("image.loadError"),
      "image.loading": t("image.loading"),
    });
  }, [i18n, i18n.language]);

  // Sync font settings to CSS custom properties
  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty("--font-weight", String(fontWeight));
    if (fontFamily) {
      root.style.setProperty("--font-family", fontFamily);
      document.body.style.fontFamily = fontFamily;
    } else {
      root.style.removeProperty("--font-family");
      document.body.style.removeProperty("font-family");
    }
    if (codeFontFamily) {
      root.style.setProperty("--code-font-family", codeFontFamily);
    } else {
      root.style.removeProperty("--code-font-family");
    }
  }, [fontWeight, fontFamily, codeFontFamily]);

  const themeConfig = useShadcnTheme(
    isDark,
    primaryColor,
    fontSize,
    borderRadius,
    fontFamily || undefined,
    codeFontFamily || undefined,
    themePreset,
  );

  return (
    <GlobalErrorBoundary>
      <BrowserRouter>
        <ConfigProvider
          locale={antdLocale}
          theme={themeConfig}
          modal={{
            centered: true,
            styles: { mask: { backdropFilter: "blur(4px)" } },
          }}
        >
          <AntdApp>
            <AppInner />
          </AntdApp>
        </ConfigProvider>
      </BrowserRouter>
    </GlobalErrorBoundary>
  );
}

export { AppRoot };
