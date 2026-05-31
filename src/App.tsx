import { BuddyWidget } from "@/components/chat/BuddyWidget";
import { TabBar } from "@/components/chat/TabBar";
import { HelpPanel } from "@/components/help/HelpPanel";
import { AppInitializer } from "@/components/layout/AppInitializer";
import { CommandPalette } from "@/components/layout/CommandPalette";
import { ContentArea } from "@/components/layout/ContentArea";
import { ErrorNotificationToast } from "@/components/layout/ErrorNotificationToast";
import { GlobalCopyMenu } from "@/components/layout/GlobalCopyMenu";
import { GlobalErrorBoundary } from "@/components/layout/GlobalErrorBoundary";
import { GlobalStatusBar } from "@/components/layout/GlobalStatusBar";
import { ModuleErrorBoundary } from "@/components/layout/ModuleErrorBoundary";
import { Sidebar } from "@/components/layout/Sidebar";
import { TitleBar } from "@/components/layout/TitleBar";
import { InteractiveTutorial } from "@/components/onboarding/InteractiveTutorial";
import { WelcomeWizard } from "@/components/onboarding/WelcomeWizard";
import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { SkillPanels } from "@/components/skill/SkillPanels";
import { useCommandPalette } from "@/hooks/useCommandPalette";
import { useGlobalOverlayScrollbars } from "@/hooks/useGlobalOverlayScrollbars";
import { useGlobalShortcutManager } from "@/hooks/useGlobalShortcutManager";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { useResolvedDarkMode } from "@/hooks/useResolvedDarkMode";
import { useResponsive } from "@/hooks/useResponsive";
import { useUpdateChecker } from "@/hooks/useUpdateChecker";
import { invoke, isTauri, listen } from "@/lib/invoke";
import { useSettingsStore, useStreamStore, useUIStore } from "@/stores";
import { useShadcnTheme } from "@/theme/shadcnTheme";
import type { ThemePreset } from "@/theme/shadcnTheme";
import { App as AntdApp, ConfigProvider, theme } from "antd";
import { setDefaultI18nMap } from "markstream-react";
import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { BrowserRouter, useLocation, useNavigate } from "react-router-dom";
import "./i18n";
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

const { useToken } = theme;

function GlobalStatusBarWrapper() {
  return (
    <ModuleErrorBoundary moduleName="GlobalStatusBar">
      <GlobalStatusBar />
    </ModuleErrorBoundary>
  );
}

function GlobalTabBar() {
  const location = useLocation();
  const isChatPage = location.pathname === "/" || location.pathname === "";
  if (!isChatPage) { return null; }
  return (
    <ModuleErrorBoundary moduleName="TabBar">
      <TabBar />
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
  const sidebarCollapsed = useUIStore((s) => s.sidebarCollapsed);

  const [isQuickBarWindow] = useState(() => {
    const params = new URLSearchParams(window.location.search);
    return params.get("__route") === "quickbar";
  });
  const isQuickBar = isQuickBarWindow || location.pathname === "/quickbar";

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

  useKeyboardShortcuts();
  useGlobalShortcutManager();
  useGlobalOverlayScrollbars();
  useResponsive();

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

  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty("--border-color", token.colorBorderSecondary);
    root.style.setProperty("--color-bg-container", token.colorBgContainer);
    root.style.setProperty("--color-bg-elevated", token.colorBgElevated);
    root.style.setProperty("--color-text", token.colorText);
    root.style.setProperty("--color-text-secondary", token.colorTextSecondary);
    root.style.setProperty("--color-primary", token.colorPrimary);
    root.style.setProperty("--color-fill-alter", token.colorFillAlter);
    root.style.setProperty("--table-border", token.colorBorderSecondary);
    root.style.setProperty("--hr-border-color", token.colorBorderSecondary);
    root.style.setProperty(
      "--blockquote-border-color",
      token.colorBorderSecondary,
    );
  }, [token]);

  const stopStreamListening = useStreamStore((s) => s.stopStreamListening);
  useEffect(() => {
    return () => stopStreamListening();
  }, [stopStreamListening]);

  const { checkForUpdate } = useUpdateChecker();
  const updateCheckInterval = useSettingsStore(
    (s) => s.settings.update_check_interval ?? 60,
  );
  const updateIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }
    const timer = setTimeout(() => checkForUpdate({ silent: true }), 3000);
    return () => clearTimeout(timer);
  }, [checkForUpdate]);

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

  const shellClass = [
    "app-shell",
    "ax-safe-top",
    "ax-safe-bottom",
    isInSettings ? "page-mode" : "",
  ].filter(Boolean).join(" ");

  return (
    <>
      <div className={shellClass}>
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
              <CommandPalette open={cmdOpen} onClose={() => setCmdOpen(false)} />
              <GlobalCopyMenu />
              <ErrorNotificationToast />
              <div className="main-area">
                <nav className={`nav-sidebar${sidebarCollapsed ? "" : " expanded"}`}>
                  <ModuleErrorBoundary moduleName="Sidebar">
                    <Sidebar />
                  </ModuleErrorBoundary>
                </nav>
                <div className="content-col">
                  <GlobalTabBar />
                  <div className="page-area">
                    <div
                      className="ax-page-transition"
                      style={{ flex: 1, display: "flex", overflow: "hidden" }}
                    >
                      <ContentArea />
                    </div>
                  </div>
                  <GlobalStatusBarWrapper />
                </div>
              </div>
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
    document.documentElement.dataset.themePreset = themePreset ?? "";
  }, [themePreset]);

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
            <AppInitializer>
              <AppInner />
            </AppInitializer>
          </AntdApp>
        </ConfigProvider>
      </BrowserRouter>
    </GlobalErrorBoundary>
  );
}

export { AppRoot };
