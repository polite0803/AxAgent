// SPDX-License-Identifier: AGPL-3.0-only

import { TabBar } from "@/components/chat/TabBar";
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
import { WorkspaceSwitcher } from "@/components/layout/WorkspaceSwitcher";
import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";

// 非首屏核心组件改为 lazy，避免 dev 模式下 Vite 逐个编译阻塞首帧。
// 这些组件仅在特定条件下渲染（onboarding/帮助/agent面板/skill浮动面板），
// 不在首屏关键路径上，用 Suspense fallback={null} 包裹即可。
const LazyAgentPanel = lazy(() => import("@/components/agent/AgentPanel").then((m) => ({ default: m.AgentPanel })));
const LazyAgentEntryPoint = lazy(() =>
  import("@/components/agent/AgentEntryPoint").then((m) => ({ default: m.AgentEntryPoint }))
);
const LazyAgentMiniPanel = lazy(() =>
  import("@/components/agent/AgentMiniPanel").then((m) => ({ default: m.AgentMiniPanel }))
);
const LazyBuddyWidget = lazy(() => import("@/components/chat/BuddyWidget").then((m) => ({ default: m.BuddyWidget })));
const LazyHelpPanel = lazy(() => import("@/components/help/HelpPanel").then((m) => ({ default: m.HelpPanel })));
const LazyInteractiveTutorial = lazy(() =>
  import("@/components/onboarding/InteractiveTutorial").then((m) => ({ default: m.InteractiveTutorial }))
);
const LazyWelcomeWizard = lazy(() =>
  import("@/components/onboarding/WelcomeWizard").then((m) => ({ default: m.WelcomeWizard }))
);
const LazySkillPanels = lazy(() => import("@/components/skill/SkillPanels").then((m) => ({ default: m.SkillPanels })));
const LazyPermissionGate = lazy(() =>
  import("@/components/agent/PermissionGate").then((m) => ({ default: m.PermissionGate }))
);
import { FEATURE_FLAGS } from "@/constants/featureFlags";
import { useCommandPalette } from "@/hooks/useCommandPalette";
import { useGlobalOverlayScrollbars } from "@/hooks/useGlobalOverlayScrollbars";
import { useGlobalShortcutManager } from "@/hooks/useGlobalShortcutManager";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { useResolvedDarkMode } from "@/hooks/useResolvedDarkMode";
import { useResponsive } from "@/hooks/useResponsive";
import { useUpdateChecker } from "@/hooks/useUpdateChecker";
import { setDefaultNavigate } from "@/lib/actionRouter";
import { invoke, isTauri, listen } from "@/lib/invoke";
import { message, setMessageInstance } from "@/lib/toast";
import {
  useAgentPanelStore,
  useSettingsStore,
  useSkillStore,
  useStreamStore,
  useUIStore,
  useWorkspaceStore,
} from "@/stores";
import { useShadcnTheme } from "@/theme/shadcnTheme";
import type { ThemePreset } from "@/theme/shadcnTheme";
import type { SkillProposal } from "@/types";
import { App as AntdApp, ConfigProvider, theme } from "antd";
import type { Locale } from "antd/es/locale";
import { setDefaultI18nMap } from "markstream-react";
import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { BrowserRouter, useLocation, useNavigate } from "react-router-dom";
import "./i18n";

// antd locale 懒加载：dev 模式不阻塞 10 个未使用的 locale 模块加载。
// 仅在语言切换时动态 import 对应 locale。
const LOCAL_MODULE_MAP: Record<string, () => Promise<Record<string, unknown>>> = {
  zh_CN: () => import("antd/locale/zh_CN"),
  zh_TW: () => import("antd/locale/zh_TW"),
  en_US: () => import("antd/locale/en_US"),
  ja_JP: () => import("antd/locale/ja_JP"),
  ko_KR: () => import("antd/locale/ko_KR"),
  de_DE: () => import("antd/locale/de_DE"),
  fr_FR: () => import("antd/locale/fr_FR"),
  es_ES: () => import("antd/locale/es_ES"),
  ru_RU: () => import("antd/locale/ru_RU"),
  hi_IN: () => import("antd/locale/hi_IN"),
  ar_EG: () => import("antd/locale/ar_EG"),
};

const LazyQuickBarPage = lazy(() => import("@/pages/QuickBarPage").then((m) => ({ default: m.QuickBarPage })));

const { useToken } = theme;

function PageTransitionWrapper({ children }: { children: React.ReactNode }) {
  const location = useLocation();
  const ref = useRef<HTMLDivElement>(null);
  const prevPathname = useRef(location.pathname);

  useEffect(() => {
    if (location.pathname !== prevPathname.current) {
      prevPathname.current = location.pathname;
      const el = ref.current;
      if (el) {
        el.classList.remove("ax-page-transition");
        void el.offsetWidth;
        el.classList.add("ax-page-transition");
      }
    }
  }, [location.pathname]);

  return (
    <div
      ref={ref}
      className="ax-page-transition"
      style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column", overflow: "hidden" }}
    >
      {children}
    </div>
  );
}

function GlobalStatusBarWrapper() {
  return (
    <ModuleErrorBoundary moduleName="GlobalStatusBar">
      <GlobalStatusBar />
    </ModuleErrorBoundary>
  );
}

function GlobalTabBar() {
  const location = useLocation();
  const isChatPage = location.pathname === "/chat" || location.pathname === "/" || location.pathname === "";
  const workspaceTab = useWorkspaceStore((s) => s.activeTab);
  // 仅在 /chat 路由且功能 Tab 为「对话」时显示会话 TabBar
  if (!isChatPage || workspaceTab !== "chat") {
    return null;
  }
  return (
    <ModuleErrorBoundary moduleName="TabBar">
      <TabBar />
    </ModuleErrorBoundary>
  );
}

/**
 * 工作台切换栏：仅在 /chat 路由下显示功能切换（对话/仪表盘/工作流/终端/知识源）。
 */
function WorkspaceSwitcherBar() {
  const location = useLocation();
  const isChatPage = location.pathname === "/chat" || location.pathname === "/" || location.pathname === "";
  if (!isChatPage) {
    return null;
  }
  return (
    <ModuleErrorBoundary moduleName="WorkspaceSwitcher">
      <WorkspaceSwitcher />
    </ModuleErrorBoundary>
  );
}

function AppInner() {
  const { token } = useToken();
  const { t } = useTranslation();
  const { modal, message: antdMessage } = AntdApp.useApp();
  // 将 App.useApp().message 注入全局 toast proxy，使非组件模块（store / 全局快捷键）也能使用上下文感知的 message
  setMessageInstance(antdMessage);
  const location = useLocation();
  const navigate = useNavigate();
  const { open: cmdOpen, setOpen: setCmdOpen } = useCommandPalette();

  // 注册全局默认导航器：供非 React 调用方（如技能生命周期钩子）通过 ActionRouter 导航
  useEffect(() => {
    setDefaultNavigate(navigate);
  }, [navigate]);
  const handleCloseCmdPalette = useCallback(() => setCmdOpen(false), [setCmdOpen]);
  const isInSettings = location.pathname === "/settings"
    || location.pathname.startsWith("/settings/");
  const sidebarCollapsed = useUIStore((s) => s.sidebarCollapsed);
  const deviceLayout = useUIStore((s) => s.deviceLayout);
  const isAgentPanelOpen = useAgentPanelStore((s) => s.isOpen);
  const isAgentMiniMode = useAgentPanelStore((s) => s.isMiniMode);
  const isAgentDragging = useAgentPanelStore((s) => s.isDragging);
  const agentPanelWidth = useAgentPanelStore((s) => s.panelWidth);
  const agentInTheLoopEnabled = FEATURE_FLAGS.AGENT_IN_THE_LOOP;

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
      ).catch(() => {
        /* Tauri API not available in browser mode */
      });
    }
  }, [navigate, isQuickBarWindow]);

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
      onOk: () => {
        invoke("force_quit").catch((err) => {
          console.error("Force quit failed:", err);
        });
      },
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

  // 监听后端子技能提案事件，自动弹出通知
  useEffect(() => {
    if (!isTauri()) {
      return;
    }
    const unlisten = listen<SkillProposal>("skill-proposal", (event) => {
      const proposal = event.payload;
      // 将提案添加到 store，供 SkillsPage 的 SkillProposalPanel 展示
      useSkillStore.getState().addSkillProposal(proposal);
      // 弹出通知，点击跳转到技能页面
      message.info({
        content: t("app.newSkillProposal", { name: proposal.suggested_name }),
        duration: 8,
        onClick: () => {
          navigate("/settings");
          useUIStore.getState().setSettingsSection("skillsHub");
        },
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [navigate, t]);

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
                  <PageErrorBoundary title={t("app.quickBar")}>
                    <LazyQuickBarPage />
                  </PageErrorBoundary>
                </Suspense>
              )
              : <ContentArea />
          )
          : (
            <>
              <Suspense fallback={null}>
                <LazySkillPanels />
              </Suspense>
              <ModuleErrorBoundary moduleName="TitleBar">
                <TitleBar />
              </ModuleErrorBoundary>
              <CommandPalette open={cmdOpen} onClose={handleCloseCmdPalette} />
              <GlobalCopyMenu />
              <ErrorNotificationToast />
              <div className="main-area">
                <nav className={`nav-sidebar${sidebarCollapsed ? "" : " expanded"}`}>
                  <ModuleErrorBoundary moduleName="Sidebar">
                    <Sidebar />
                  </ModuleErrorBoundary>
                </nav>
                <div
                  className="content-col"
                  style={{
                    flex: 1,
                    minWidth: 0,
                    marginRight: (agentInTheLoopEnabled && isAgentPanelOpen && deviceLayout !== "mobile"
                        && deviceLayout !== "tablet")
                      ? agentPanelWidth
                      : 0,
                    transition: isAgentDragging ? "none" : "margin-right 300ms ease-in-out",
                  }}
                >
                  <WorkspaceSwitcherBar />
                  <GlobalTabBar />
                  <div className="page-area">
                    <PageTransitionWrapper>
                      <ContentArea />
                    </PageTransitionWrapper>
                  </div>
                  <GlobalStatusBarWrapper />
                </div>
                {agentInTheLoopEnabled && (
                  <Suspense fallback={null}>
                    {/* 手机/平板 → 覆盖层，不占 flex 空间；桌面 → 内联浮动 */}
                    {(deviceLayout === "mobile" || deviceLayout === "tablet") && isAgentPanelOpen && (
                      <div
                        style={{
                          position: "absolute",
                          inset: 0,
                          zIndex: 49,
                          backgroundColor: "rgba(0,0,0,0.3)",
                        }}
                        onClick={() => useAgentPanelStore.getState().close()}
                      />
                    )}
                    <div
                      style={deviceLayout === "mobile" || deviceLayout === "tablet"
                        ? {
                          position: "absolute",
                          right: 0,
                          top: 0,
                          bottom: 0,
                          zIndex: 50,
                          display: isAgentPanelOpen ? undefined : "none",
                          maxWidth: "100vw",
                        }
                        : { display: "contents" }}
                    >
                      <LazyAgentPanel />
                    </div>
                  </Suspense>
                )}
              </div>
            </>
          )}
      </div>
      <Suspense fallback={null}>
        <LazyWelcomeWizard />
      </Suspense>
      <Suspense fallback={null}>
        <LazyInteractiveTutorial />
      </Suspense>
      <Suspense fallback={null}>
        <LazyHelpPanel />
      </Suspense>
      <Suspense fallback={null}>
        <LazyBuddyWidget />
      </Suspense>
      {agentInTheLoopEnabled && isAgentMiniMode && (
        <Suspense fallback={null}>
          <LazyAgentMiniPanel />
        </Suspense>
      )}
      {agentInTheLoopEnabled && !isAgentMiniMode && (
        <Suspense fallback={null}>
          <LazyAgentEntryPoint />
        </Suspense>
      )}
      <Suspense fallback={null}>
        <LazyPermissionGate />
      </Suspense>
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

  const [antdLocale, setAntdLocale] = useState<Locale | undefined>(undefined);

  // 动态加载当前语言的 antd locale（替代顶层导入所有 11 个 locale）
  useEffect(() => {
    const localeKey = localeMap[language] || (language?.startsWith("zh") ? "zh_CN" : "en_US");
    if (localeKey in LOCAL_MODULE_MAP) {
      LOCAL_MODULE_MAP[localeKey]()
        .then((mod) => setAntdLocale((mod as unknown as { default: Locale }).default))
        .catch(() => {});
    }
  }, [language, localeMap]);

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
    // 21th 预设时默认使用等宽字体，其他预设使用无衬线字体
    const is21th = themePreset?.includes("21th") ?? false;
    const defaultFont = is21th
      ? "'Geist Mono', ui-monospace, monospace"
      : "'Geist Variable', 'Inter Variable', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif";
    const resolvedFont = fontFamily || defaultFont;
    const resolvedCode = codeFontFamily || "'JetBrains Mono Variable', ui-monospace, monospace";
    root.style.setProperty("--font-family", resolvedFont);
    document.body.style.fontFamily = resolvedFont;
    root.style.setProperty("--code-font-family", resolvedCode);
  }, [fontWeight, fontFamily, codeFontFamily, themePreset]);

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
