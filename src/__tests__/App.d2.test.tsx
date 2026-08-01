// SPDX-License-Identifier: AGPL-3.0-only

import { render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const enableD2 = vi.fn();
const preloadChatRenderers = vi.fn();
const setDefaultI18nMap = vi.fn();

const settingsState = {
  settings: {
    theme_mode: "dark",
    primary_color: "#17A93D",
    font_size: 14,
    border_radius: 8,
    language: "zh-CN",
    always_on_top: false,
    close_to_tray: true,
    auto_start: false,
    global_shortcuts_enabled: false,
    shortcut_registration_logs_enabled: false,
    shortcut_trigger_toast_enabled: false,
    global_shortcut: "",
  },
  fetchSettings: vi.fn().mockResolvedValue(undefined),
};

const uiState = {
  activePage: "chat",
};

const workspaceState = {
  activeTab: "chat",
  setActiveTab: vi.fn(),
};

vi.mock("antd", () => ({
  ConfigProvider: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  App: Object.assign(
    ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
    {
      useApp: () => ({
        modal: { confirm: vi.fn() },
        message: {
          success: vi.fn(),
          error: vi.fn(),
          info: vi.fn(),
          warning: vi.fn(),
          loading: vi.fn(),
          open: vi.fn(),
          destroy: vi.fn(),
        },
      }),
    },
  ),
  Layout: Object.assign(
    ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
    {
      Sider: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
      Content: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
    },
  ),
  Space: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  Button: ({ children }: { children: React.ReactNode }) => <button>{children}</button>,
  Spin: () => <div data-testid="spin" />,
  Result: () => <div data-testid="result" />,
  Typography: {
    Text: ({ children }: { children: React.ReactNode }) => <span>{children}</span>,
    Paragraph: ({ children }: { children: React.ReactNode }) => <p>{children}</p>,
  },
  message: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    open: vi.fn(),
    destroy: vi.fn(),
    useMessage: () => [vi.fn(), null],
  },
  theme: {
    useToken: () => ({
      token: {
        colorBorderSecondary: "#444",
        colorBgContainer: "#111",
        colorBgElevated: "#1a1a1a",
        colorText: "#f5f5f5",
        colorTextSecondary: "#999",
        colorPrimary: "#1677ff",
      },
    }),
  },
}));

vi.mock("antd/locale/zh_CN", () => ({
  default: {},
}));

vi.mock("react-i18next", () => ({
  initReactI18next: {
    type: "3rdParty",
    init: () => {},
  },
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: {
      language: "zh-CN",
      getFixedT: () => (_key: string) => _key,
      changeLanguage: vi.fn(),
    },
  }),
}));

vi.mock("@/components/layout/Sidebar", () => ({
  Sidebar: () => <div>sidebar</div>,
}));

vi.mock("@/components/layout/WorkspaceSwitcher", () => ({
  WorkspaceSwitcher: () => <div data-testid="workspace-switcher" />,
}));

vi.mock("@/components/layout/TitleBar", () => ({
  TitleBar: () => <div>titlebar</div>,
}));

vi.mock("@/components/layout/ContentArea", () => ({
  ContentArea: () => <div>content</div>,
}));

vi.mock("@/components/layout/CommandPalette", () => ({
  CommandPalette: () => null,
}));

vi.mock("@/hooks/useCommandPalette", () => ({
  useCommandPalette: () => ({
    open: false,
    setOpen: vi.fn(),
  }),
}));

vi.mock("@/stores", () => ({
  useUIStore: (selector: (state: typeof uiState) => unknown) => selector(uiState),
  useWorkspaceStore: (selector: (state: typeof workspaceState) => unknown) => selector(workspaceState),
  useWorkspaceTabStore: (selector: (state: typeof workspaceState) => unknown) => selector(workspaceState),
  useSettingsStore: Object.assign(
    (selector: (state: typeof settingsState) => unknown) => selector(settingsState),
    {
      getState: () => settingsState,
    },
  ),
  useConversationStore: Object.assign(
    (selector: (state: unknown) => unknown) =>
      selector({
        startStreamListening: vi.fn(),
      }),
    {
      getState: () => ({
        startStreamListening: vi.fn(),
        fetchConversations: vi.fn().mockResolvedValue(undefined),
        conversations: [],
      }),
    },
  ),
  useStreamStore: Object.assign(
    (selector: (state: unknown) => unknown) =>
      selector({
        stopStreamListening: vi.fn(),
      }),
    { getState: () => ({ stopStreamListening: vi.fn() }) },
  ),
  useSkillExtensionStore: Object.assign(
    (selector: (state: unknown) => unknown) =>
      selector({
        skills: [],
        loading: false,
        commands: [],
        panels: [],
        settingsSections: [],
        toolbarButtons: [],
        chatCommands: [],
        statusBarItems: [],
        handlers: {},
        fetchSkills: vi.fn().mockResolvedValue(undefined),
        getHandler: vi.fn().mockReturnValue(undefined),
        refreshSkill: vi.fn(),
      }),
    {
      getState: () => ({
        fetchSkills: vi.fn().mockResolvedValue(undefined),
      }),
    },
  ),
  useOnboardingStore: Object.assign(
    (selector: (state: unknown) => unknown) =>
      selector({
        completed: true,
        setCompleted: vi.fn(),
        loadFromSettings: vi.fn(),
      }),
    {
      getState: () => ({
        loadFromSettings: vi.fn(),
      }),
    },
  ),
  useAgentPanelStore: () => ({
    isVisible: false,
    togglePanel: vi.fn(),
    showPanel: vi.fn(),
    hidePanel: vi.fn(),
  }),
  useAppConfigStore: Object.assign(
    () => ({ loadConfig: vi.fn().mockResolvedValue(undefined) }),
    { getState: () => ({ loadConfig: vi.fn().mockResolvedValue(undefined) }) },
  ),
  useTabStore: Object.assign(
    () => ({ pruneInvalidTabs: vi.fn() }),
    { getState: () => ({ pruneInvalidTabs: vi.fn() }) },
  ),
}));

vi.mock("@/hooks/useKeyboardShortcuts", () => ({
  useKeyboardShortcuts: vi.fn(),
}));

vi.mock("@/hooks/useResolvedDarkMode", () => ({
  useResolvedDarkMode: () => true,
}));

vi.mock("@/theme/shadcnTheme", () => ({
  useShadcnTheme: () => ({}),
}));

vi.mock("@/lib/invoke", () => ({
  isTauri: () => false,
  invoke: vi.fn().mockResolvedValue(undefined),
  checkIpcHealth: vi.fn().mockResolvedValue({ ok: true }),
  listen: vi.fn().mockResolvedValue(() => {}),
  logIpcError: vi.fn(),
}));

vi.mock("@/lib/toast", () => ({
  message: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    open: vi.fn(),
    destroy: vi.fn(),
    useMessage: () => [vi.fn(), null],
  },
  setMessageInstance: vi.fn(),
}));

vi.mock("@/i18n", () => ({
  default: {
    language: "zh-CN",
    changeLanguage: vi.fn(),
    t: (key: string) => key,
  },
}));

vi.mock("react-router-dom", () => ({
  BrowserRouter: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  useLocation: () => ({ pathname: "/" }),
  useNavigate: () => vi.fn(),
}));

vi.mock("@/components/agent/AgentEntryPoint", () => ({
  AgentEntryPoint: () => null,
}));

vi.mock("@/components/agent/AgentMiniPanel", () => ({
  AgentMiniPanel: () => null,
}));

vi.mock("@/components/agent/AgentPanel", () => ({
  AgentPanel: () => null,
}));

vi.mock("@/components/chat/BuddyWidget", () => ({
  BuddyWidget: () => null,
}));

vi.mock("@/components/chat/TabBar", () => ({
  TabBar: () => null,
}));

vi.mock("@/components/help/HelpPanel", () => ({
  HelpPanel: () => null,
}));

vi.mock("@/components/layout/GlobalCopyMenu", () => ({
  GlobalCopyMenu: () => null,
}));

vi.mock("@/components/layout/GlobalErrorBoundary", () => ({
  GlobalErrorBoundary: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock("@/components/layout/GlobalStatusBar", () => ({
  GlobalStatusBar: () => null,
}));

vi.mock("@/components/layout/ModuleErrorBoundary", () => ({
  ModuleErrorBoundary: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock("@/components/onboarding/InteractiveTutorial", () => ({
  InteractiveTutorial: () => null,
}));

vi.mock("@/components/onboarding/WelcomeWizard", () => ({
  WelcomeWizard: () => null,
}));

vi.mock("@/components/shared/ErrorBoundary", () => ({
  PageErrorBoundary: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock("@/components/skill/SkillPanels", () => ({
  SkillPanels: () => null,
}));

vi.mock("@/hooks/useResponsive", () => ({
  useResponsive: vi.fn(),
}));

vi.mock("@/hooks/useUpdateChecker", () => ({
  useUpdateChecker: () => ({ checkForUpdate: vi.fn() }),
}));

vi.mock("@/hooks/useGlobalOverlayScrollbars", () => ({
  useGlobalOverlayScrollbars: vi.fn(),
}));

vi.mock("@ant-design/icons", () =>
  new Proxy({}, {
    get: (_target, prop) => () => <span data-icon={String(prop)} />,
  }));

vi.mock("@/lib/preloadChatRenderers", () => ({
  preloadChatRenderers,
  preloadCommonPages: vi.fn(),
}));

vi.mock("@lobehub/ui", () => ({}));

vi.mock("@lobehub/icons", () => ({
  Icon: () => null,
}));

vi.mock("@terrastruct/d2", () => ({}));

vi.mock("markstream-react", () => ({
  enableD2,
  setDefaultI18nMap,
}));

vi.mock("@/hooks/useGlobalShortcutManager", () => ({
  useGlobalShortcutManager: () => ({
    register: vi.fn(),
    unregister: vi.fn(),
    registerAll: vi.fn(),
  }),
}));

describe("AppRoot D2 setup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("enables the markstream D2 loader during startup", async () => {
    const { AppRoot } = await import("../App");

    render(<AppRoot />);

    await vi.waitFor(
      () => {
        expect(enableD2).toHaveBeenCalledTimes(1);
      },
      { timeout: 5000 },
    );
    expect(preloadChatRenderers).toHaveBeenCalledTimes(1);
  }, 10000);
});
