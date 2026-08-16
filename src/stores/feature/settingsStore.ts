// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { DEFAULT_SHORTCUT_BINDINGS } from "@/lib/shortcuts";
import type { AppSettings } from "@/types";
import { create } from "zustand";

const DEFAULT_SETTINGS: AppSettings = {
  language: "zh-CN",
  themeMode: "dark",
  themePreset: "deep-dusk",
  primaryColor: "#17A93D",
  borderRadius: 6,
  autoStart: false,
  showOnStart: true,
  minimizeToTray: true,
  fontSize: 14,
  fontWeight: 400,
  fontFamily: "'Geist Variable', 'Inter Variable', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  codeFontFamily: "'JetBrains Mono Variable', ui-monospace, monospace",
  bubbleStyle: "modern",
  codeTheme: "poimandres",
  codeThemeLight: "github-light",
  defaultProviderId: null,
  defaultModelId: null,
  defaultTemperature: null,
  defaultMaxTokens: null,
  defaultTopP: null,
  defaultFrequencyPenalty: null,
  defaultContextCount: null,
  titleSummaryProviderId: null,
  titleSummaryModelId: null,
  titleSummaryTemperature: null,
  titleSummaryMaxTokens: null,
  titleSummaryTopP: null,
  titleSummaryFrequencyPenalty: null,
  titleSummaryContextCount: null,
  titleSummaryPrompt: null,
  compressionProviderId: null,
  compressionModelId: null,
  compressionTemperature: null,
  compressionMaxTokens: null,
  compressionTopP: null,
  compressionFrequencyPenalty: null,
  compressionPrompt: null,
  proxyType: null,
  proxyAddress: null,
  proxyPort: null,
  globalShortcut: DEFAULT_SHORTCUT_BINDINGS.toggleCurrentWindow,
  shortcutToggleCurrentWindow: DEFAULT_SHORTCUT_BINDINGS.toggleCurrentWindow,
  shortcutToggleAllWindows: DEFAULT_SHORTCUT_BINDINGS.toggleAllWindows,
  shortcutCloseWindow: DEFAULT_SHORTCUT_BINDINGS.closeWindow,
  shortcutNewConversation: DEFAULT_SHORTCUT_BINDINGS.newConversation,
  shortcutOpenSettings: DEFAULT_SHORTCUT_BINDINGS.openSettings,
  shortcutToggleModelSelector: DEFAULT_SHORTCUT_BINDINGS.toggleModelSelector,
  shortcutFillLastMessage: DEFAULT_SHORTCUT_BINDINGS.fillLastMessage,
  shortcutClearContext: DEFAULT_SHORTCUT_BINDINGS.clearContext,
  shortcutClearConversationMessages: DEFAULT_SHORTCUT_BINDINGS.clearConversationMessages,
  shortcutToggleGateway: DEFAULT_SHORTCUT_BINDINGS.toggleGateway,
  shortcutToggleMode: DEFAULT_SHORTCUT_BINDINGS.toggleMode,
  shortcutShowQuickBar: DEFAULT_SHORTCUT_BINDINGS.showQuickBar,
  gatewayAutoStart: false,
  gatewayListenAddress: "127.1.0.0",
  gatewayPort: 8080,
  gatewaySslEnabled: false,
  gatewaySslMode: "upload",
  gatewaySslCertPath: null,
  gatewaySslKeyPath: null,
  gatewaySslPort: 8443,
  gatewayForceSsl: false,
  alwaysOnTop: false,
  trayEnabled: true,
  globalShortcutsEnabled: true,
  shortcutRegistrationLogsEnabled: false,
  shortcutTriggerToastEnabled: false,
  notificationsEnabled: true,
  miniWindowEnabled: false,
  startMinimized: false,
  closeToTray: true,
  notifyBackup: true,
  notifyImport: true,
  notifyErrors: true,
  lastSelectedConversationId: null,
  documentsRootOverride: null,
  updateCheckInterval: 60,
  defaultSystemPrompt: null,
  chatMinimapEnabled: false,
  chatMinimapStyle: "faq",
  agentPanelEnabled: true,
  agentPanelCompact: false,
  onboardingCompleted: false,
  onboardingWizardDismissed: false,
  onboardingTutorialCompleted: false,
  onboardingSelectedPreset: null,
  multiModelDisplayMode: "tabs",
  renderUserMarkdown: false,
  defaultWorkspaceDir: null,
  // WebDAV sync settings — must be present so stale saves never omit them
  webdavHost: null,
  webdavUsername: null,
  webdavPath: null,
  webdavAcceptInvalidCerts: false,
  webdavSyncEnabled: false,
  webdavSyncIntervalMinutes: 60,
  webdavMaxRemoteBackups: 10,
  webdavIncludeDocuments: false,
  // Closed-loop nudge scheduler settings
  closedLoopEnabled: true,
  closedLoopIntervalMinutes: 5,
  screenPerceptionEnabled: false,
  rlOptimizerEnabled: false,
  loraFinetuneEnabled: false,
  proactiveNudgeEnabled: true,
  thoughtChainEnabled: true,
  errorRecoveryEnabled: true,
  totEnabled: false,
  showDeveloperTools: true,
  // Cloud workspace settings
  workspaceUri: null,
  cloudBackend: null,
  s3ProviderPreset: null,
  s3SecretAccessKey: null,
  webdavPassword: null,
  cloudSyncEnabled: false,
  s3UsePathStyle: false,
  // RAG pipeline config
  ragPipelineConfig: {
    queryEnhancement: {
      enabled: false,
      strategy: "auto" as const,
      maxVariants: 3,
      combinedCall: true,
    },
    rerank: {
      enabled: true,
      backend: "rule" as const,
      crossEncoderModel: "bge-reranker-v2-m3",
      topN: 5,
      candidateK: 30,
      ruleFilterKeep: 15,
      scoreThreshold: null,
      ollamaEndpoint: "http://localhost:11434",
    },
    selfRag: {
      enabled: false,
      judgeModel: "qwen2.5:0.5b",
      ollamaEndpoint: "http://localhost:11434",
      relevanceThreshold: 0.5,
      qualityThreshold: 0.6,
      maxRetryRounds: 2,
    },
  },
  // Smart Router 智能路由
  smartRouterEnabled: false,
  smartRouterTierMappings: {},
  // RAG 模型自动加载
  autoLoadModels: true,
  // P2-8: ACP 服务端 base URL（null 时使用默认值 http://localhost:9876）
  acpBaseUrl: null,
};

export interface GlobalShortcutDiagnostic {
  timestamp: string;
  phase: "env" | "register" | "cleanup";
  level: "info" | "warn" | "error";
  message: string;
  action?: string;
  shortcut?: string;
  reason?: string;
}

export interface GlobalShortcutStatus {
  enabled: boolean;
  registered: string[];
  failed: Array<{ shortcut: string; reason: string }>;
  diagnostics: GlobalShortcutDiagnostic[];
}

interface SettingsState {
  settings: AppSettings;
  loading: boolean;
  /** Set once after the first successful fetchSettings; guards saveSettings from writing stale data. */
  _loaded: boolean;
  error: string | null;
  globalShortcutStatus: GlobalShortcutStatus;
  fetchSettings: () => Promise<void>;
  saveSettings: (settings: Partial<AppSettings>) => Promise<void>;
  setGlobalShortcutStatus: (status: GlobalShortcutStatus) => void;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  loading: true,
  _loaded: false,
  error: null,
  globalShortcutStatus: {
    enabled: false,
    registered: [],
    failed: [],
    diagnostics: [],
  },

  fetchSettings: async () => {
    set({ loading: true });
    try {
      const fetched = await invoke<Partial<AppSettings>>("get_settings");
      set({
        settings: { ...DEFAULT_SETTINGS, ...fetched },
        loading: false,
        _loaded: true,
        error: null,
      });
    } catch (e) {
      set({ error: String(e), loading: false, _loaded: true });
    }
  },

  saveSettings: async (partial) => {
    if (!get()._loaded) {
      // Settings haven't been hydrated from the DB yet; dropping silently would
      // make the change look "persisted" in the UI but vanish on next launch.
      console.warn(
        "[settingsStore] saveSettings called before fetchSettings finished — skipping",
        { keys: Object.keys(partial) },
      );
      return;
    }
    set((s) => ({ settings: { ...s.settings, ...partial }, error: null }));
    try {
      await invoke("save_settings", { settings: get().settings });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setGlobalShortcutStatus: (status) => {
    set({ globalShortcutStatus: status });
  },
}));
