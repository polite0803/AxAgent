// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { type AdaptResult, adaptSettings } from "@/lib/settingsAdaptor";
import { fromDto, toDto, validateDtoConsistency } from "@/lib/settingsDtoConverter";
import { DEFAULT_SHORTCUT_BINDINGS } from "@/lib/shortcuts";
import type { AppSettings, ProviderConfig } from "@/types";
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

  // === 使用 NullableModelRef 保证结构一致性 ===
  defaultModel: null,
  defaultTemperature: null,
  defaultMaxTokens: null,
  defaultTopP: null,
  defaultFrequencyPenalty: null,
  defaultContextCount: null,

  titleSummaryModel: null,
  titleSummaryTemperature: null,
  titleSummaryMaxTokens: null,
  titleSummaryTopP: null,
  titleSummaryFrequencyPenalty: null,
  titleSummaryContextCount: null,
  titleSummaryPrompt: null,

  compressionModel: null,
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
  sandboxMode: "danger-full-access" as "read-only" | "workspace-write" | "danger-full-access",
  approvalPolicy: "on-request" as "untrusted" | "on-failure" | "on-request" | "never",
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
  _loaded: boolean;
  error: string | null;
  globalShortcutStatus: GlobalShortcutStatus;
  _fetchPromise: Promise<void> | null;
  fetchSettings: () => Promise<void>;
  saveSettings: (settings: Partial<AppSettings>) => Promise<void>;
  validateAndCleanModels: (providers: readonly ProviderConfig[]) => Promise<AdaptResult>;
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

  // 用于防止并发调用 fetchSettings
  _fetchPromise: null as Promise<void> | null,

  /**
   * 从后端加载设置（防重复调用：并发请求复用同一个 Promise）
   */
  fetchSettings: async () => {
    // 如果已经有正在进行的请求，直接复用
    const state = get();
    if (state._fetchPromise) {
      return state._fetchPromise;
    }

    const promise = (async () => {
      set({ loading: true });
      try {
        const rawDto = await invoke<Record<string, unknown>>("get_settings");

        validateDtoConsistency(rawDto as never);

        const appSettings = fromDto(rawDto as never);

        set({
          settings: { ...DEFAULT_SETTINGS, ...appSettings },
          loading: false,
          _loaded: true,
          error: null,
        });
      } catch (e) {
        console.error("[settingsStore.fetchSettings] Failed to fetch settings", e);
        set({ error: String(e), loading: false, _loaded: true });
      } finally {
        // 清除 promise 引用
        set((s) => ({ ...s, _fetchPromise: null }));
      }
    })();

    // 存储 promise 以供复用
    set({ _fetchPromise: promise });
    return promise;
  },

  /**
   * 保存设置到后端
   *
   * 关键变化：
   * 1. 前端内部使用 NullableModelRef（类型安全）
   * 2. 使用 toDto() 转换为后端需要的分离字段格式
   * 3. 转换层保证：如果 NullableModelRef 有效，拆分后的字段一定有效
   */
  saveSettings: async (partial) => {
    if (!get()._loaded) {
      console.warn("[settingsStore] saveSettings called before fetchSettings finished — skipping");
      return;
    }

    // 更新本地状态
    set((s) => ({ settings: { ...s.settings, ...partial }, error: null }));

    const currentSettings = get().settings;

    try {
      // 转换为后端 DTO 格式并保存
      const dto = toDto(currentSettings);

      await invoke<unknown>("save_settings", { settings: dto });
    } catch (e) {
      console.error("[settingsStore.saveSettings] invoke failed", e);
      set({ error: String(e) });
    }
  },

  /**
   * 验证并清理无效的模型引用
   *
   * 这是类型驱动设计的核心：
   * - 类型系统保证结构一致性
   * - 运行时验证数据有效性（provider/model 是否真实存在）
   * - 任何无效引用在此阶段就被清除
   */
  validateAndCleanModels: async (providers) => {
    const currentSettings = get().settings;
    const result = adaptSettings(currentSettings, providers);

    if (result.changed) {
      console.warn("[settingsStore] invalid model references cleaned", {
        invalidFields: result.invalidFields,
      });

      // 更新本地状态为清理后的设置
      set({ settings: result.cleanedSettings });

      // 持久化清理后的设置到后端
      try {
        const dto = toDto(result.cleanedSettings);
        await invoke("save_settings", { settings: dto });
      } catch (e) {
        console.error("[settingsStore] failed to save cleaned settings", e);
        set({ error: String(e) });
      }
    }

    return result;
  },

  setGlobalShortcutStatus: (status) => {
    set({ globalShortcutStatus: status });
  },
}));
