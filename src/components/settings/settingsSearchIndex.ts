// SPDX-License-Identifier: AGPL-3.0-only

import i18n from "@/i18n";
import type { SettingsSection } from "@/types";

/**
 * 设置搜索索引 —— 板块级（Phase 1）+ 项级（Phase 2）。
 *
 * Phase 1 只匹配板块标题+keywords；
 * Phase 2 额外匹配板块内各设置项（items），搜索结果显示为独立行，
 * 点击后：①setSettingsSection 切板块；②setSettingsHighlight 触发高亮闪烁。
 */
export interface SettingsSearchItem {
  /** 唯一标识，用于 data-search-key 匹配与高亮定位 */
  itemKey: string;
  /** i18n key = settings.xxx.xxx，用于渲染结果行标题 */
  labelKey: string;
  /** 中/英关键词 */
  keywords: string[];
}

export interface SettingsSearchEntry {
  /** 板块 key，与 SettingsSection 一一对应 */
  section: SettingsSection;
  /** 所属菜单分组（仅用于归类） */
  group: string;
  /** 中/英关键词，直接匹配原始字符串 */
  keywords: string[];
  /** Phase 2 — 项级元数据。非空时搜索结果展开为板块+项两级 */
  items?: SettingsSearchItem[];
}

/**
 * 板块级关键词索引。
 * 覆盖 SECTION_COMPONENTS 中全部可导航板块；其中 localTools / mcpServers
 * 不在 TAB_GROUPS 菜单里，但也登记进来，使搜索能直达这些隐藏板块。
 */
export const SETTINGS_SEARCH_INDEX: SettingsSearchEntry[] = [
  {
    section: "providers",
    group: "model",
    keywords: [
      i18n.t("settingsSearch.providers.modelProvider"),
      i18n.t("settingsSearch.providers.serviceProvider"),
      "provider",
      "api",
      i18n.t("settingsSearch.providers.apiKey"),
      "key",
      "llm",
      i18n.t("settingsSearch.providers.access"),
      "openai",
      i18n.t("settingsSearch.providers.modelManagement"),
    ],
  },
  {
    section: "defaultModel",
    group: "model",
    keywords: [
      i18n.t("settingsSearch.defaultModel.defaultModel"),
      "default model",
      i18n.t("settingsSearch.defaultModel.modelSelection"),
      i18n.t("settingsSearch.defaultModel.currentModel"),
      i18n.t("settingsSearch.defaultModel.preferredModel"),
    ],
  },
  {
    section: "conversationSettings",
    group: "model",
    keywords: [
      i18n.t("settingsSearch.chatSettings"),
      "conversation",
      i18n.t("settingsSearch.context"),
      "context",
      i18n.t("settingsSearch.history"),
      i18n.t("settingsSearch.temperature"),
      "temperature",
      i18n.t("settingsSearch.maxToken"),
      "max token",
    ],
    items: [
      {
        itemKey: "conversation:bubbleStyle",
        labelKey: "settings.bubbleStyle",
        keywords: [
          i18n.t("settingsSearch.conversationSettings.bubbleStyle"),
          i18n.t("settingsSearch.conversationSettings.messageBubble"),
          "bubble style",
          "bubble",
        ],
      },
      {
        itemKey: "conversation:renderUserMarkdown",
        labelKey: "settings.renderUserMarkdown",
        keywords: [
          i18n.t("settingsSearch.conversationSettings.renderUser"),
          i18n.t("settingsSearch.conversationSettings.userMessage"),
          "markdown",
          "render user",
        ],
      },
      {
        itemKey: "conversation:multiModelDisplayMode",
        labelKey: "settings.multiModelDisplayMode",
        keywords: [
          i18n.t("settingsSearch.conversationSettings.multiModel"),
          i18n.t("settingsSearch.conversationSettings.displayMode"),
          "multi model",
          "display mode",
          "tabs",
          i18n.t("settingsSearch.conversationSettings.sideBySide"),
          i18n.t("settingsSearch.conversationSettings.stacked"),
        ],
      },
      {
        itemKey: "conversation:chatMinimapEnabled",
        labelKey: "settings.chatMinimapEnabled",
        keywords: [
          i18n.t("settingsSearch.conversationSettings.chatMinimap"),
          i18n.t("settingsSearch.conversationSettings.chatThumbnail"),
          "minimap",
          i18n.t("settingsSearch.conversationSettings.dialogMinimap"),
        ],
      },
      {
        itemKey: "conversation:chatMinimapStyle",
        labelKey: "settings.chatMinimapStyle",
        keywords: [
          i18n.t("settingsSearch.conversationSettings.navStyle"),
          i18n.t("settingsSearch.conversationSettings.thumbnailStyle"),
          "minimap style",
          i18n.t("settingsSearch.conversationSettings.questionIndex"),
          i18n.t("settingsSearch.conversationSettings.floatingIndicator"),
        ],
      },
    ],
  },
  {
    section: "promptTemplates",
    group: "model",
    keywords: [
      i18n.t("settingsSearch.promptTemplates.promptTemplate"),
      "prompt",
      i18n.t("settingsSearch.promptTemplates.template"),
      "template",
      i18n.t("settingsSearch.promptTemplates.systemPrompt"),
      "system prompt",
    ],
  },
  {
    section: "searchProviders",
    group: "model",
    keywords: [
      i18n.t("settingsSearch.searchProviders.searchProvider"),
      "search",
      i18n.t("settingsSearch.searchProviders.searchEngine"),
      i18n.t("settingsSearch.searchProviders.webSearch"),
      "tavily",
      "web search",
    ],
    items: [
      {
        itemKey: "searchProviders:name",
        labelKey: "settings.searchProviders.name",
        keywords: [i18n.t("settingsSearch.searchProviders.name"), "name", "provider name"],
      },
      {
        itemKey: "searchProviders:type",
        labelKey: "settings.searchProviders.type",
        keywords: [i18n.t("settingsSearch.searchProviders.type"), "type", "provider type", "tavily", "brave", "bing"],
      },
      {
        itemKey: "searchProviders:endpoint",
        labelKey: "settings.searchProviders.endpoint",
        keywords: [
          i18n.t("settingsSearch.searchProviders.endpoint"),
          "endpoint",
          i18n.t("settingsSearch.searchProviders.apiAddress"),
        ],
      },
      {
        itemKey: "searchProviders:apiKey",
        labelKey: "settings.searchProviders.apiKeySet",
        keywords: ["api key", i18n.t("settingsSearch.searchProviders.apiKey"), "apikey"],
      },
      {
        itemKey: "searchProviders:resultLimit",
        labelKey: "settings.searchProviders.resultLimit",
        keywords: [i18n.t("settingsSearch.searchProviders.resultLimit"), "result limit", "result count"],
      },
      {
        itemKey: "searchProviders:timeout",
        labelKey: "settings.searchProviders.timeout",
        keywords: [i18n.t("settingsSearch.searchProviders.timeout"), "timeout", "ms"],
      },
      {
        itemKey: "searchProviders:enabled",
        labelKey: "common.enabled",
        keywords: [
          i18n.t("settingsSearch.searchProviders.enabled"),
          "enabled",
          i18n.t("settingsSearch.searchProviders.enable"),
        ],
      },
    ],
  },
  {
    section: "general",
    group: "appearance",
    keywords: [
      i18n.t("settingsSearch.general"),
      "general",
      i18n.t("settingsSearch.language"),
      "language",
      i18n.t("settingsSearch.startup"),
      i18n.t("settingsSearch.boot"),
      i18n.t("settingsSearch.general.autoStart"),
      i18n.t("settingsSearch.general.bootStart"),
      i18n.t("settingsSearch.general.tray"),
      "tray",
      i18n.t("settingsSearch.general.alwaysOnTop"),
      i18n.t("settingsSearch.general.minimize"),
      "startup",
      "autostart",
      "always on top",
    ],
    items: [
      {
        itemKey: "general:language",
        labelKey: "settings.language",
        keywords: [
          i18n.t("settingsSearch.general.language"),
          "language",
          "locale",
          i18n.t("settingsSearch.general.i18n"),
          "i18n",
          i18n.t("settingsSearch.general.multiLanguage"),
        ],
      },
      {
        itemKey: "general:autoStart",
        labelKey: "settings.autoStart",
        keywords: [
          i18n.t("settingsSearch.general.autoStart"),
          i18n.t("settingsSearch.general.bootStart"),
          "auto start",
          "autostart",
          i18n.t("settingsSearch.general.bootStart2"),
        ],
      },
      {
        itemKey: "general:showOnStart",
        labelKey: "settings.showOnStart",
        keywords: [
          i18n.t("settingsSearch.general.showOnStart"),
          i18n.t("settingsSearch.general.startupDisplay"),
          "show on start",
          i18n.t("settingsSearch.general.bootDisplay"),
        ],
      },
      {
        itemKey: "general:alwaysOnTop",
        labelKey: "desktop.alwaysOnTop",
        keywords: [
          i18n.t("settingsSearch.general.alwaysOnTop"),
          "always on top",
          i18n.t("settingsSearch.general.windowTop"),
          i18n.t("settingsSearch.general.frontMost"),
        ],
      },
      {
        itemKey: "general:startMinimized",
        labelKey: "desktop.startMinimized",
        keywords: [
          i18n.t("settingsSearch.general.startMinimized"),
          i18n.t("settingsSearch.general.minimizeStart"),
          "start minimized",
          i18n.t("settingsSearch.general.minimize"),
        ],
      },
      {
        itemKey: "general:minimizeToTray",
        labelKey: "settings.minimizeToTray",
        keywords: [
          i18n.t("settingsSearch.general.tray"),
          "tray",
          i18n.t("settingsSearch.general.minimizeToTray"),
          i18n.t("settingsSearch.general.systemTray"),
          i18n.t("settingsSearch.general.background"),
        ],
      },
      {
        itemKey: "general:defaultWorkspaceDir",
        labelKey: "settings.defaultWorkspaceDir",
        keywords: [
          i18n.t("settingsSearch.general.workspace"),
          "workspace",
          i18n.t("settingsSearch.general.defaultDir"),
          i18n.t("settingsSearch.general.workDir"),
          i18n.t("settingsSearch.general.path"),
        ],
      },
    ],
  },
  {
    section: "display",
    group: "appearance",
    keywords: [
      i18n.t("settingsSearch.display.displaySettings"),
      "display",
      i18n.t("settingsSearch.display.font"),
      "font",
      i18n.t("settingsSearch.display.fontSize"),
      i18n.t("settingsSearch.display.zoom"),
      "zoom",
      i18n.t("settingsSearch.display.layout"),
      i18n.t("settingsSearch.display.density"),
      "density",
      i18n.t("settingsSearch.display.display"),
    ],
    items: [
      {
        itemKey: "display:themeMode",
        labelKey: "settings.theme.label",
        keywords: [
          i18n.t("settingsSearch.display.themeMode"),
          "theme",
          i18n.t("settingsSearch.display.dark"),
          i18n.t("settingsSearch.display.light"),
          "dark",
          "light",
          i18n.t("settingsSearch.display.followSystem"),
          "system",
        ],
      },
      {
        itemKey: "display:themePreset",
        labelKey: "settings.themePreset",
        keywords: [
          i18n.t("settingsSearch.display.themePreset"),
          "theme preset",
          i18n.t("settingsSearch.display.preset"),
          i18n.t("settingsSearch.display.colorScheme"),
          i18n.t("settingsSearch.display.colorTheme"),
        ],
      },
      {
        itemKey: "display:primaryColor",
        labelKey: "settings.primaryColor",
        keywords: [
          i18n.t("settingsSearch.display.accent"),
          i18n.t("settingsSearch.display.primaryColor"),
          "primary color",
          "accent",
          i18n.t("settingsSearch.display.themeColor"),
          i18n.t("settingsSearch.display.brandColor"),
        ],
      },
      {
        itemKey: "display:fontSize",
        labelKey: "settings.fontSize",
        keywords: [
          i18n.t("settingsSearch.display.fontSize"),
          i18n.t("settingsSearch.display.fontSize2"),
          "font size",
          i18n.t("settingsSearch.display.fontDimension"),
        ],
      },
      {
        itemKey: "display:fontWeight",
        labelKey: "settings.fontWeight",
        keywords: [
          i18n.t("settingsSearch.display.fontWeight"),
          "font weight",
          i18n.t("settingsSearch.display.thickness"),
          i18n.t("settingsSearch.display.bold"),
          i18n.t("settingsSearch.display.fontThickness"),
        ],
      },
      {
        itemKey: "display:fontFamily",
        labelKey: "settings.fontFamily",
        keywords: [
          i18n.t("settingsSearch.display.font"),
          "font",
          "font family",
          i18n.t("settingsSearch.display.uiFont"),
          i18n.t("settingsSearch.display.systemFont"),
        ],
      },
      {
        itemKey: "display:codeFontFamily",
        labelKey: "settings.codeFontFamily",
        keywords: [
          i18n.t("settingsSearch.display.codeFont"),
          "code font",
          i18n.t("settingsSearch.display.monoFont"),
          "monospace",
          i18n.t("settingsSearch.display.programmingFont"),
        ],
      },
      {
        itemKey: "display:codeThemeLight",
        labelKey: "settings.codeThemeLight",
        keywords: [
          i18n.t("settingsSearch.display.codeTheme"),
          "code theme",
          i18n.t("settingsSearch.display.lightCode"),
          i18n.t("settingsSearch.display.codeHighlight"),
          "syntax highlight",
        ],
      },
      {
        itemKey: "display:codeThemeDark",
        labelKey: "settings.codeThemeDark",
        keywords: [
          i18n.t("settingsSearch.display.codeTheme"),
          "code theme",
          i18n.t("settingsSearch.display.darkCode"),
          i18n.t("settingsSearch.display.codeHighlight"),
          "syntax highlight",
        ],
      },
      {
        itemKey: "display:borderRadius",
        labelKey: "settings.borderRadius",
        keywords: [
          i18n.t("settingsSearch.display.borderRadius"),
          "border radius",
          i18n.t("settingsSearch.display.radiusSize"),
          i18n.t("settingsSearch.display.radiusValue"),
        ],
      },
    ],
  },
  {
    section: "theme",
    group: "appearance",
    keywords: [
      i18n.t("settingsSearch.theme.theme"),
      "theme",
      i18n.t("settingsSearch.theme.dark"),
      i18n.t("settingsSearch.theme.light"),
      "dark",
      "light",
      i18n.t("settingsSearch.theme.appearance"),
      i18n.t("settingsSearch.theme.color"),
      i18n.t("settingsSearch.theme.accent"),
      "accent",
      i18n.t("settingsSearch.theme.colorScheme"),
    ],
  },
  {
    section: "animations",
    group: "appearance",
    keywords: [
      i18n.t("settingsSearch.animations.animation"),
      "animations",
      i18n.t("settingsSearch.animations.motion"),
      "motion",
      i18n.t("settingsSearch.animations.reducedMotion"),
      "reduced motion",
      "prefers-reduced-motion",
      i18n.t("settingsSearch.animations.transition"),
      "transition",
      i18n.t("settingsSearch.animations.animationEffect"),
    ],
    items: [
      {
        itemKey: "animations:mode",
        labelKey: "settings.animations.modeLabel",
        keywords: [
          i18n.t("settingsSearch.animations.animationMode"),
          "animation mode",
          i18n.t("settingsSearch.animations.followSystem"),
          "system",
          i18n.t("settingsSearch.animations.enable"),
          i18n.t("settingsSearch.animations.disable"),
        ],
      },
      {
        itemKey: "animations:preview",
        labelKey: "settings.animations.previewTitle",
        keywords: [
          i18n.t("settingsSearch.animations.animationPreview"),
          "preview",
          i18n.t("settingsSearch.animations.preview"),
          i18n.t("settingsSearch.animations.effectPreview"),
        ],
      },
    ],
  },
  {
    section: "shortcuts",
    group: "appearance",
    keywords: [
      i18n.t("settingsSearch.shortcuts.shortcut"),
      "shortcut",
      i18n.t("settingsSearch.shortcuts.hotkey"),
      "hotkey",
      i18n.t("settingsSearch.shortcuts.keyBinding"),
      i18n.t("settingsSearch.shortcuts.binding"),
      "keybinding",
    ],
    items: [
      {
        itemKey: "shortcuts:enableGlobalShortcuts",
        labelKey: "settings.enableGlobalShortcuts",
        keywords: [
          i18n.t("settingsSearch.shortcuts.globalShortcut"),
          "global shortcut",
          i18n.t("settingsSearch.shortcuts.enable"),
        ],
      },
      {
        itemKey: "shortcuts:enableShortcutRegistrationLogs",
        labelKey: "settings.enableShortcutRegistrationLogs",
        keywords: [
          i18n.t("settingsSearch.shortcuts.shortcutLog"),
          i18n.t("settingsSearch.shortcuts.registrationLog"),
          "shortcut log",
          "diagnostic",
        ],
      },
      {
        itemKey: "shortcuts:enableShortcutTriggerToast",
        labelKey: "settings.enableShortcutTriggerToast",
        keywords: [i18n.t("settingsSearch.shortcuts.triggerToast"), "toast", "shortcut trigger"],
      },
    ],
  },
  {
    section: "tools",
    group: "extensions",
    keywords: [
      i18n.t("settingsSearch.tools.tool"),
      "tool",
      i18n.t("settingsSearch.tools.localTool"),
      "local tool",
      "mcp",
      i18n.t("settingsSearch.tools.functionCall"),
      "function calling",
      i18n.t("settingsSearch.tools.toolManagement"),
    ],
  },
  {
    section: "skillsHub",
    group: "extensions",
    keywords: [
      i18n.t("settingsSearch.skillsHub.skillCenter"),
      "skill",
      i18n.t("settingsSearch.skillsHub.skill"),
      i18n.t("settingsSearch.skillsHub.marketplace"),
      "marketplace",
      i18n.t("settingsSearch.skillsHub.skillMarket"),
    ],
  },
  {
    section: "plugins",
    group: "extensions",
    keywords: [
      i18n.t("settingsSearch.plugins.plugin"),
      "plugin",
      i18n.t("settingsSearch.plugins.extension"),
      "extension",
    ],
  },
  {
    section: "dashboardPlugins",
    group: "extensions",
    keywords: [
      i18n.t("settingsSearch.dashboardPlugins.dashboardPlugin"),
      "dashboard",
      i18n.t("settingsSearch.dashboardPlugins.dashboard"),
      "widget",
      i18n.t("settingsSearch.dashboardPlugins.widget"),
    ],
  },
  {
    section: "dynamicPages",
    group: "extensions",
    keywords: [
      i18n.t("settingsSearch.dynamicPages.dynamicPage"),
      "dynamic page",
      i18n.t("settingsSearch.dynamicPages.customPage"),
      "custom page",
      i18n.t("settingsSearch.dynamicPages.page"),
    ],
  },
  {
    section: "appConfig",
    group: "extensions",
    keywords: [
      i18n.t("settingsSearch.appConfig.appConfig"),
      "app config",
      i18n.t("settingsSearch.appConfig.agentControl"),
      i18n.t("settingsSearch.appConfig.permission"),
      "permission",
      i18n.t("settingsSearch.appConfig.iteration"),
      "iteration",
      i18n.t("settingsSearch.appConfig.featureFlag"),
      "feature flag",
      "hook",
    ],
    items: [
      {
        itemKey: "agent:maxIterations",
        labelKey: "settings.agent.maxIterations",
        keywords: [i18n.t("settingsSearch.appConfig.maxIterations"), "max iterations", "max iterations"],
      },
      {
        itemKey: "agent:permissionMode",
        labelKey: "settings.agent.permissionMode",
        keywords: [
          i18n.t("settingsSearch.appConfig.permissionMode"),
          "permission mode",
          i18n.t("settingsSearch.appConfig.readonly"),
          i18n.t("settingsSearch.appConfig.fullAccess"),
          i18n.t("settingsSearch.appConfig.writeAccess"),
        ],
      },
      {
        itemKey: "agent:forkSubagent",
        labelKey: "settings.agent.featureFlags.forkSubagent",
        keywords: [
          "fork",
          i18n.t("settingsSearch.appConfig.subagent"),
          "subagent",
          i18n.t("settingsSearch.appConfig.parallel"),
        ],
      },
      {
        itemKey: "agent:coordinatorMode",
        labelKey: "settings.agent.featureFlags.coordinatorMode",
        keywords: [
          i18n.t("settingsSearch.appConfig.coordinator"),
          "coordinator",
          i18n.t("settingsSearch.appConfig.scheduling"),
        ],
      },
      {
        itemKey: "agent:proactiveMode",
        labelKey: "settings.agent.featureFlags.proactiveMode",
        keywords: [
          i18n.t("settingsSearch.appConfig.proactiveMode"),
          "proactive",
          i18n.t("settingsSearch.appConfig.prediction"),
        ],
      },
      {
        itemKey: "agent:swarmMode",
        labelKey: "settings.agent.featureFlags.swarmMode",
        keywords: [
          i18n.t("settingsSearch.appConfig.swarmMode"),
          "swarm",
          i18n.t("settingsSearch.appConfig.swarmCollab"),
        ],
      },
      {
        itemKey: "agent:toolConcurrency",
        labelKey: "settings.agent.featureFlags.toolConcurrency",
        keywords: [
          i18n.t("settingsSearch.appConfig.toolConcurrency"),
          "tool concurrency",
          i18n.t("settingsSearch.appConfig.parallel"),
        ],
      },
      {
        itemKey: "agent:verificationAgent",
        labelKey: "settings.agent.featureFlags.verificationAgent",
        keywords: [
          i18n.t("settingsSearch.appConfig.verification"),
          "verification agent",
          i18n.t("settingsSearch.appConfig.review"),
        ],
      },
      {
        itemKey: "agent:dreamTask",
        labelKey: "settings.agent.featureFlags.dreamTask",
        keywords: [
          i18n.t("settingsSearch.appConfig.dream"),
          "dream task",
          i18n.t("settingsSearch.appConfig.backgroundOpt"),
        ],
      },
    ],
  },
  {
    section: "imageGen",
    group: "extensions",
    keywords: [
      i18n.t("settingsSearch.imageGen.imageGen"),
      "image gen",
      i18n.t("settingsSearch.imageGen.textToImage"),
      i18n.t("settingsSearch.imageGen.draw"),
      i18n.t("settingsSearch.imageGen.painting"),
      "draw",
      "stable diffusion",
      "dall",
    ],
  },
  {
    section: "proxy",
    group: "network",
    keywords: [
      i18n.t("settingsSearch.proxy.proxy"),
      "proxy",
      i18n.t("settingsSearch.proxy.network"),
      "network",
      i18n.t("settingsSearch.proxy.bypass"),
      i18n.t("settingsSearch.proxy.scientific"),
      i18n.t("settingsSearch.proxy.httpProxy"),
    ],
    items: [
      {
        itemKey: "proxy:proxyType",
        labelKey: "settings.proxyType",
        keywords: [i18n.t("settingsSearch.proxy.proxyType"), "proxy type", "http", "socks5", "system", "none"],
      },
      {
        itemKey: "proxy:proxyAddress",
        labelKey: "settings.proxyAddress",
        keywords: [i18n.t("settingsSearch.proxy.proxyAddress"), "proxy address", "proxy host"],
      },
      {
        itemKey: "proxy:proxyPort",
        labelKey: "settings.proxyPort",
        keywords: [i18n.t("settingsSearch.proxy.proxyPort"), "proxy port", "port"],
      },
    ],
  },
  {
    section: "messageChannels",
    group: "network",
    keywords: [
      i18n.t("settingsSearch.messageChannels.messageChannel"),
      "message channel",
      i18n.t("settingsSearch.messageChannels.notificationChannel"),
      i18n.t("settingsSearch.messageChannels.platform"),
      "telegram",
      i18n.t("settingsSearch.messageChannels.dingtalk"),
      i18n.t("settingsSearch.messageChannels.wechat"),
      i18n.t("settingsSearch.messageChannels.feishu"),
      "discord",
    ],
  },
  {
    section: "webhooks",
    group: "network",
    keywords: [
      "webhook",
      i18n.t("settingsSearch.webhooks.callback"),
      "callback",
      i18n.t("settingsSearch.webhooks.hook"),
      i18n.t("settingsSearch.webhooks.notificationCallback"),
    ],
  },
  {
    section: "acp",
    group: "network",
    keywords: ["acp", i18n.t("settingsSearch.acp.acpProtocol"), "agent client protocol"],
    items: [
      {
        itemKey: "acp:serverAddress",
        labelKey: "acp.serverAddress",
        keywords: [i18n.t("settingsSearch.acp.serverAddress"), "server address", "acp", "base url"],
      },
      {
        itemKey: "acp:connectionStatus",
        labelKey: "acp.connectionStatus",
        keywords: [i18n.t("settingsSearch.acp.connectionStatus"), "connection status", "connected"],
      },
      {
        itemKey: "acp:workdir",
        labelKey: "acp.workdir",
        keywords: [
          i18n.t("settingsSearch.acp.workDir"),
          "workdir",
          "working directory",
          i18n.t("settingsSearch.acp.session"),
        ],
      },
    ],
  },
  {
    section: "data",
    group: "data",
    keywords: [
      i18n.t("settingsSearch.data.data"),
      "data",
      i18n.t("settingsSearch.data.dataManagement"),
      i18n.t("settingsSearch.data.export"),
      i18n.t("settingsSearch.data.import"),
      i18n.t("settingsSearch.data.clear"),
      i18n.t("settingsSearch.data.reset"),
      "data manager",
    ],
    items: [
      {
        itemKey: "data:exportData",
        labelKey: "settings.exportData",
        keywords: [i18n.t("settingsSearch.data.exportData"), "export", i18n.t("settingsSearch.data.dataExport")],
      },
      {
        itemKey: "data:importData",
        labelKey: "settings.importData",
        keywords: [i18n.t("settingsSearch.data.importData"), "import", i18n.t("settingsSearch.data.dataImport")],
      },
      {
        itemKey: "data:clearData",
        labelKey: "settings.clearData",
        keywords: [
          i18n.t("settingsSearch.data.clearData"),
          i18n.t("settingsSearch.data.clearConversation"),
          "clear",
          "delete",
          "danger",
        ],
      },
    ],
  },
  {
    section: "database",
    group: "data",
    keywords: [
      i18n.t("settingsSearch.database.database"),
      "database",
      i18n.t("settingsSearch.database.storage"),
      "sqlite",
      i18n.t("settingsSearch.database.connection"),
      "db",
    ],
  },
  {
    section: "storage",
    group: "data",
    keywords: [
      i18n.t("settingsSearch.storage.storage"),
      "storage",
      i18n.t("settingsSearch.storage.space"),
      i18n.t("settingsSearch.storage.disk"),
      i18n.t("settingsSearch.storage.cache"),
      "cache",
      i18n.t("settingsSearch.storage.usage"),
      i18n.t("settingsSearch.storage.cleanup"),
    ],
  },
  {
    section: "readingList",
    group: "data",
    keywords: [
      i18n.t("settingsSearch.readingList.readingList"),
      "reading list",
      i18n.t("settingsSearch.readingList.toRead"),
      i18n.t("settingsSearch.readingList.queue"),
      "queue",
      "papers",
      i18n.t("settingsSearch.readingList.paperQueue"),
      i18n.t("settingsSearch.readingList.readingQueue"),
    ],
    items: [
      {
        itemKey: "readingList:create",
        labelKey: "readingList.create",
        keywords: [
          i18n.t("settingsSearch.readingList.createList"),
          "create list",
          i18n.t("settingsSearch.readingList.createReadingList"),
        ],
      },
      {
        itemKey: "readingList:items",
        labelKey: "readingList.items",
        keywords: [
          i18n.t("settingsSearch.readingList.items"),
          "items",
          i18n.t("settingsSearch.readingList.readingItems"),
        ],
      },
    ],
  },
  {
    section: "paperOverview",
    group: "data",
    keywords: [
      i18n.t("settingsSearch.paperOverview.paperOverview"),
      "paper overview",
      i18n.t("settingsSearch.paperOverview.paperAbstract"),
      i18n.t("settingsSearch.paperOverview.structuredOverview"),
      "TLDR",
      "key concepts",
      i18n.t("settingsSearch.paperOverview.coreConcepts"),
    ],
    items: [
      {
        itemKey: "paperOverview:generatePrompt",
        labelKey: "paper.generatePrompt",
        keywords: [
          i18n.t("settingsSearch.paperOverview.generatePrompt"),
          "generate prompt",
          i18n.t("settingsSearch.paperOverview.overviewPrompt"),
        ],
      },
      {
        itemKey: "paperOverview:regenerate",
        labelKey: "paper.regenerate",
        keywords: [
          i18n.t("settingsSearch.paperOverview.regenerate"),
          "regenerate",
          i18n.t("settingsSearch.paperOverview.regenerateOverview"),
        ],
      },
    ],
  },
  {
    section: "knowledgeGraph",
    group: "data",
    keywords: [
      i18n.t("settingsSearch.knowledgeGraph.knowledgeGraph"),
      "knowledge graph",
      "lightrag",
      i18n.t("settingsSearch.knowledgeGraph.entity"),
      i18n.t("settingsSearch.knowledgeGraph.relation"),
      "entity",
      "relation",
      i18n.t("settingsSearch.knowledgeGraph.graphQuery"),
      "graph search",
    ],
    items: [
      {
        itemKey: "knowledgeGraph:search",
        labelKey: "knowledgeGraph.search",
        keywords: [
          i18n.t("settingsSearch.knowledgeGraph.graphEnhancedSearch"),
          "graph enhanced search",
          i18n.t("settingsSearch.knowledgeGraph.search"),
        ],
      },
      {
        itemKey: "knowledgeGraph:extract",
        labelKey: "knowledgeGraph.extract",
        keywords: [
          i18n.t("settingsSearch.knowledgeGraph.crossDocExtraction"),
          "extract entities",
          i18n.t("settingsSearch.knowledgeGraph.entityExtraction"),
        ],
      },
    ],
  },
  {
    section: "cloudWorkspace",
    group: "data",
    keywords: [
      i18n.t("settingsSearch.cloudWorkspace.cloudWorkspace"),
      "cloud workspace",
      i18n.t("settingsSearch.cloudWorkspace.sync"),
      "sync",
      i18n.t("settingsSearch.cloudWorkspace.cloudWorkarea"),
    ],
  },
  {
    section: "backup",
    group: "data",
    keywords: [
      i18n.t("settingsSearch.backup.backup"),
      "backup",
      i18n.t("settingsSearch.backup.restore"),
      "restore",
      i18n.t("settingsSearch.backup.snapshot"),
      "snapshot",
    ],
  },
  {
    section: "scheduler",
    group: "data",
    keywords: [
      i18n.t("settingsSearch.scheduler.scheduler"),
      "scheduler",
      i18n.t("settingsSearch.scheduler.plan"),
      i18n.t("settingsSearch.scheduler.task"),
      "task",
      i18n.t("settingsSearch.scheduler.scheduling"),
    ],
    items: [
      {
        itemKey: "scheduler:autoBackupEnabled",
        labelKey: "settings.scheduler.enabled",
        keywords: [
          i18n.t("settingsSearch.scheduler.autoBackup"),
          "auto backup",
          i18n.t("settingsSearch.scheduler.enabled"),
        ],
      },
      {
        itemKey: "scheduler:backupInterval",
        labelKey: "settings.scheduler.backupInterval",
        keywords: [
          i18n.t("settingsSearch.scheduler.backupInterval"),
          "backup interval",
          i18n.t("settingsSearch.scheduler.hour"),
        ],
      },
      {
        itemKey: "scheduler:maxBackupCount",
        labelKey: "settings.scheduler.maxCount",
        keywords: [
          i18n.t("settingsSearch.scheduler.backupRetention"),
          "max count",
          i18n.t("settingsSearch.scheduler.maxCount"),
        ],
      },
      {
        itemKey: "scheduler:webdavSyncEnabled",
        labelKey: "settings.scheduler.enabled",
        keywords: [
          i18n.t("settingsSearch.scheduler.webdavSync"),
          i18n.t("settingsSearch.scheduler.autoSync"),
          "webdav sync",
        ],
      },
      {
        itemKey: "scheduler:syncInterval",
        labelKey: "settings.scheduler.syncInterval",
        keywords: [
          i18n.t("settingsSearch.scheduler.syncInterval"),
          "sync interval",
          i18n.t("settingsSearch.scheduler.minute"),
        ],
      },
      {
        itemKey: "scheduler:maxRemoteBackups",
        labelKey: "settings.scheduler.maxRemoteBackups",
        keywords: [i18n.t("settingsSearch.scheduler.remoteRetention"), "remote backup", "max"],
      },
      {
        itemKey: "scheduler:closedLoopEnabled",
        labelKey: "settings.scheduler.enabled",
        keywords: [
          i18n.t("settingsSearch.scheduler.closedLoopLearning"),
          "closed loop",
          i18n.t("settingsSearch.scheduler.learningTip"),
        ],
      },
      {
        itemKey: "scheduler:nudgeInterval",
        labelKey: "settings.scheduler.nudgeInterval",
        keywords: [
          i18n.t("settingsSearch.scheduler.nudgeInterval"),
          "nudge interval",
          "nudge",
          i18n.t("settingsSearch.scheduler.closedLoop"),
        ],
      },
    ],
  },
  {
    section: "cron",
    group: "data",
    keywords: [
      "cron",
      i18n.t("settingsSearch.cron.schedule"),
      i18n.t("settingsSearch.cron.expression"),
      "expression",
      i18n.t("settingsSearch.cron.periodicTask"),
      i18n.t("settingsSearch.cron.scheduledTask"),
    ],
  },
  {
    section: "notificationCenter",
    group: "data",
    keywords: [
      i18n.t("settingsSearch.notificationCenter.notificationCenter"),
      "notification",
      i18n.t("settingsSearch.notificationCenter.notification"),
      i18n.t("settingsSearch.notificationCenter.reminder"),
      "alert",
      i18n.t("settingsSearch.notificationCenter.messageCenter"),
    ],
  },
  {
    section: "advanced",
    group: "system",
    keywords: [
      i18n.t("settingsSearch.advanced.advanced"),
      "advanced",
      i18n.t("settingsSearch.advanced.developer"),
      "developer",
      i18n.t("settingsSearch.advanced.experimental"),
      "experimental",
      i18n.t("settingsSearch.advanced.debug"),
      "debug",
    ],
    items: [
      {
        itemKey: "advanced:dangerousCmdDetect",
        labelKey: "advanced.dangerousCmdDetect",
        keywords: [
          i18n.t("settingsSearch.advanced.dangerousCmd"),
          "command detect",
          i18n.t("settingsSearch.advanced.safetyVerify"),
          "bash",
        ],
      },
      {
        itemKey: "advanced:networkCmdDetect",
        labelKey: "advancedSettings.networkCmdDetect",
        keywords: [
          i18n.t("settingsSearch.advanced.networkCmd"),
          "network detect",
          i18n.t("settingsSearch.advanced.networkDetect"),
        ],
      },
      {
        itemKey: "advanced:cmdTimeout",
        labelKey: "advanced.cmdTimeout",
        keywords: [i18n.t("settingsSearch.advanced.cmdTimeout"), "cmd timeout", "timeout", "bash"],
      },
      {
        itemKey: "advanced:defaultPermission",
        labelKey: "advancedSettings.defaultPermission",
        keywords: [
          i18n.t("settingsSearch.advanced.defaultPermission"),
          "permission mode",
          i18n.t("settingsSearch.advanced.ask"),
          i18n.t("settingsSearch.advanced.edit"),
          i18n.t("settingsSearch.advanced.full"),
        ],
      },
      {
        itemKey: "advanced:fileWriteConfirm",
        labelKey: "advanced.fileWriteConfirm",
        keywords: [
          i18n.t("settingsSearch.advanced.fileWrite"),
          "write confirm",
          i18n.t("settingsSearch.advanced.fileConfirm"),
        ],
      },
      {
        itemKey: "advanced:networkConfirm",
        labelKey: "advancedSettings.networkConfirm",
        keywords: [
          i18n.t("settingsSearch.advanced.networkConfirm"),
          "network confirm",
          i18n.t("settingsSearch.advanced.networkRequest"),
        ],
      },
      {
        itemKey: "advanced:shellConfirm",
        labelKey: "advancedSettings.shellConfirm",
        keywords: [
          i18n.t("settingsSearch.advanced.shellConfirm"),
          "shell confirm",
          i18n.t("settingsSearch.advanced.cmdExecution"),
        ],
      },
      {
        itemKey: "advanced:defaultMode",
        labelKey: "advancedSettings.defaultMode",
        keywords: [
          i18n.t("settingsSearch.advanced.defaultMode"),
          "default mode",
          i18n.t("settingsSearch.advanced.general"),
          i18n.t("settingsSearch.advanced.fast"),
          i18n.t("settingsSearch.advanced.deep"),
          i18n.t("settingsSearch.advanced.plan"),
        ],
      },
      {
        itemKey: "advanced:tokenBudgetLimit",
        labelKey: "advancedSettings.tokenBudgetLimit",
        keywords: [
          i18n.t("settingsSearch.advanced.tokenBudget"),
          "budget",
          i18n.t("settingsSearch.advanced.budget"),
          "token limit",
        ],
      },
      {
        itemKey: "advanced:enableTokenBudget",
        labelKey: "advancedSettings.enableTokenBudget",
        keywords: [i18n.t("settingsSearch.advanced.budgetDetect"), "token budget", "budget enable"],
      },
      {
        itemKey: "advanced:autoRetry",
        labelKey: "advancedSettings.autoRetry",
        keywords: [i18n.t("settingsSearch.advanced.autoRetry"), "auto retry", i18n.t("settingsSearch.advanced.retry")],
      },
      {
        itemKey: "advanced:maxRetries",
        labelKey: "advancedSettings.maxRetries",
        keywords: [i18n.t("settingsSearch.advanced.retryCount"), "max retries", "retry"],
      },
      {
        itemKey: "advanced:retryDelay",
        labelKey: "advancedSettings.retryDelay",
        keywords: [
          i18n.t("settingsSearch.advanced.retryDelay"),
          "retry delay",
          i18n.t("settingsSearch.advanced.delay"),
        ],
      },
      {
        itemKey: "advanced:modelFallback",
        labelKey: "advancedSettings.modelFallback",
        keywords: [
          i18n.t("settingsSearch.advanced.modelFallback"),
          "fallback",
          i18n.t("settingsSearch.advanced.fallback"),
        ],
      },
      {
        itemKey: "advanced:cpuLimit",
        labelKey: "advancedSettings.cpuLimit",
        keywords: [i18n.t("settingsSearch.advanced.cpuLimit"), "cpu limit", "cpu"],
      },
      {
        itemKey: "advanced:memoryLimit",
        labelKey: "advancedSettings.memoryLimit",
        keywords: [
          i18n.t("settingsSearch.advanced.memoryLimit"),
          "memory limit",
          i18n.t("settingsSearch.advanced.memory"),
        ],
      },
      {
        itemKey: "advanced:enableIdleDetection",
        labelKey: "advancedSettings.enableIdleDetection",
        keywords: [i18n.t("settingsSearch.advanced.idleDetect"), "idle detect", "idle"],
      },
      {
        itemKey: "advanced:idleTimeout",
        labelKey: "advancedSettings.idleTimeout",
        keywords: [i18n.t("settingsSearch.advanced.idleTimeout"), "idle timeout", "timeout"],
      },
      {
        itemKey: "advanced:autoCompressThreshold",
        labelKey: "advancedSettings.autoCompressThreshold",
        keywords: [i18n.t("settingsSearch.advanced.compressThreshold"), "auto compress", "context compress"],
      },
      {
        itemKey: "advanced:warningBuffer",
        labelKey: "advancedSettings.warningBuffer",
        keywords: [i18n.t("settingsSearch.advanced.warningBuffer"), "warning buffer", "threshold"],
      },
      {
        itemKey: "advanced:maxConsecutiveFailures",
        labelKey: "advancedSettings.maxConsecutiveFailures",
        keywords: [i18n.t("settingsSearch.advanced.compactFail"), "compact failure", "max failures"],
      },
      {
        itemKey: "advanced:enableMemoryCompression",
        labelKey: "advancedSettings.enableMemoryCompression",
        keywords: [i18n.t("settingsSearch.advanced.memoryCompact"), "memory compact", "session compress"],
      },
      {
        itemKey: "advanced:enableDream",
        labelKey: "advancedSettings.enableDream",
        keywords: [
          "dream",
          i18n.t("settingsSearch.advanced.consolidation"),
          i18n.t("settingsSearch.advanced.bgConsolidation"),
          "consolidation",
        ],
      },
      {
        itemKey: "advanced:dreamMinInterval",
        labelKey: "advancedSettings.minInterval",
        keywords: [
          i18n.t("settingsSearch.advanced.dreamInterval"),
          "min interval",
          i18n.t("settingsSearch.advanced.hour"),
        ],
      },
      {
        itemKey: "advanced:dreamMinSessions",
        labelKey: "advancedSettings.minNewSessions",
        keywords: [i18n.t("settingsSearch.advanced.newSession"), "min sessions", "dream"],
      },
      {
        itemKey: "advanced:dreamMaxDuration",
        labelKey: "advancedSettings.maxDuration",
        keywords: [i18n.t("settingsSearch.advanced.duration"), "max duration", "dream"],
      },
      {
        itemKey: "advanced:enableLspDiagnostics",
        labelKey: "advancedSettings.enableLspDiagnostics",
        keywords: [
          i18n.t("settingsSearch.advanced.lspDiagnostics"),
          "lsp diagnostics",
          i18n.t("settingsSearch.advanced.languageServer"),
        ],
      },
      {
        itemKey: "advanced:diagnosticLevel",
        labelKey: "advancedSettings.diagnosticLevelLabel",
        keywords: [i18n.t("settingsSearch.advanced.diagnosticLevel"), "diagnostic level", "error", "warning"],
      },
    ],
  },
  {
    section: "evolution",
    group: "system",
    keywords: [
      i18n.t("settingsSearch.evolution.evolution"),
      "evolution",
      i18n.t("settingsSearch.evolution.skillEvolution"),
      i18n.t("settingsSearch.evolution.genetic"),
      "genetic",
      i18n.t("settingsSearch.evolution.learning"),
      "learning",
      i18n.t("settingsSearch.evolution.adaptive"),
    ],
  },
  {
    section: "persona",
    group: "system",
    keywords: [
      i18n.t("settingsSearch.persona.persona"),
      "persona",
      i18n.t("settingsSearch.persona.identity"),
      i18n.t("settingsSearch.persona.profile"),
      "identity",
      "profile",
      i18n.t("settingsSearch.persona.styleTransfer"),
    ],
  },
  {
    section: "proactiveBehavior",
    group: "system",
    keywords: [
      i18n.t("settingsSearch.proactiveBehavior.proactive"),
      "proactive",
      i18n.t("settingsSearch.proactiveBehavior.suggestion"),
      i18n.t("settingsSearch.proactiveBehavior.closedLoop"),
      "closed loop",
      i18n.t("settingsSearch.proactiveBehavior.prediction"),
      "nudge",
      "self-learning",
    ],
    items: [
      {
        itemKey: "proactive:nudge",
        labelKey: "settings.proactiveNudge",
        keywords: [
          i18n.t("settingsSearch.proactiveBehavior.proactiveSuggestion"),
          "nudge",
          "proactive",
          i18n.t("settingsSearch.proactiveBehavior.tip"),
        ],
      },
      {
        itemKey: "proactive:closedLoop",
        labelKey: "settings.closedLoopEnabled",
        keywords: [
          i18n.t("settingsSearch.proactiveBehavior.closedLoop"),
          "closed loop",
          i18n.t("settingsSearch.proactiveBehavior.autoExecute"),
        ],
      },
      {
        itemKey: "proactive:interval",
        labelKey: "settings.closedLoopInterval",
        keywords: [
          i18n.t("settingsSearch.proactiveBehavior.interval"),
          "interval",
          i18n.t("settingsSearch.proactiveBehavior.minute"),
          i18n.t("settingsSearch.proactiveBehavior.polling"),
        ],
      },
    ],
  },
  {
    section: "about",
    group: "system",
    keywords: [
      i18n.t("settingsSearch.about.about"),
      "about",
      i18n.t("settingsSearch.about.version"),
      "version",
      i18n.t("settingsSearch.about.license"),
      "license",
      i18n.t("settingsSearch.about.changelog"),
      "changelog",
    ],
    items: [
      {
        itemKey: "about:appVersion",
        labelKey: "settings.version",
        keywords: [i18n.t("settingsSearch.about.version"), "version", "app version"],
      },
      {
        itemKey: "about:openSource",
        labelKey: "settings.openSource",
        keywords: [
          i18n.t("settingsSearch.about.openSourceAgreement"),
          i18n.t("settingsSearch.about.openSource"),
          "license",
          "AGPL",
        ],
      },
      {
        itemKey: "about:overallStatus",
        labelKey: "settings.overallStatus",
        keywords: [
          i18n.t("settingsSearch.about.overallStatus"),
          i18n.t("settingsSearch.about.serviceHealth"),
          "health",
          "service health",
        ],
      },
      {
        itemKey: "about:checkUpdate",
        labelKey: "settings.checkUpdate",
        keywords: [
          i18n.t("settingsSearch.about.checkUpdate"),
          "update",
          "check update",
          i18n.t("settingsSearch.about.versionUpdate"),
        ],
      },
      {
        itemKey: "about:updateCheckInterval",
        labelKey: "settings.updateCheckInterval",
        keywords: [i18n.t("settingsSearch.about.updateInterval"), "update interval", "check interval"],
      },
      {
        itemKey: "about:developerTools",
        labelKey: "settings.developerTools",
        keywords: [i18n.t("settingsSearch.about.developerTools"), "devtools", i18n.t("settingsSearch.about.devTools")],
      },
      {
        itemKey: "about:loraFinetune",
        labelKey: "settings.loraFinetuneEnabled",
        keywords: ["LoRA", i18n.t("settingsSearch.about.finetune"), "fine-tune", "finetune", "lora"],
      },
      {
        itemKey: "about:replayTutorial",
        labelKey: "help.onboardingReplayDesc",
        keywords: [
          i18n.t("settingsSearch.about.guide"),
          "tutorial",
          "onboarding",
          i18n.t("settingsSearch.about.onboarding"),
        ],
      },
    ],
  },
  {
    section: "localTools",
    group: "other",
    keywords: [
      i18n.t("settingsSearch.localTools.localTool"),
      "local tool",
      i18n.t("settingsSearch.localTools.localCommand"),
      "command",
      i18n.t("settingsSearch.localTools.scriptTool"),
    ],
  },
  {
    section: "mcpServers",
    group: "other",
    keywords: [
      "mcp",
      i18n.t("settingsSearch.mcpServers.mcpServer"),
      "mcp server",
      "model context protocol",
      i18n.t("settingsSearch.mcpServers.serverConfig"),
    ],
    items: [
      {
        itemKey: "mcpServers:name",
        labelKey: "settings.mcpServers.name",
        keywords: [i18n.t("settingsSearch.mcpServers.name"), "name", "server name"],
      },
      {
        itemKey: "mcpServers:transport",
        labelKey: "settings.mcpServers.transport",
        keywords: [i18n.t("settingsSearch.mcpServers.transport"), "transport", "stdio", "sse", "http"],
      },
      {
        itemKey: "mcpServers:command",
        labelKey: "settings.mcpServers.command",
        keywords: [i18n.t("settingsSearch.mcpServers.command"), "command", "stdio command"],
      },
      {
        itemKey: "mcpServers:args",
        labelKey: "settings.mcpServers.args",
        keywords: [i18n.t("settingsSearch.mcpServers.args"), "args", "arguments"],
      },
      {
        itemKey: "mcpServers:endpoint",
        labelKey: "settings.mcpServers.endpoint",
        keywords: [i18n.t("settingsSearch.mcpServers.endpoint"), "endpoint", "url"],
      },
      {
        itemKey: "mcpServers:customHeaders",
        labelKey: "settings.mcpServers.customHeaders",
        keywords: [i18n.t("settingsSearch.mcpServers.requestHeaders"), "custom headers", "headers"],
      },
      {
        itemKey: "mcpServers:envVars",
        labelKey: "settings.mcpServers.envVars",
        keywords: [i18n.t("settingsSearch.mcpServers.envVars"), "env", "environment"],
      },
      {
        itemKey: "mcpServers:discoverTimeout",
        labelKey: "settings.mcpServers.discoverTimeout",
        keywords: [i18n.t("settingsSearch.mcpServers.discoverTimeout"), "discover timeout", "timeout"],
      },
      {
        itemKey: "mcpServers:executeTimeout",
        labelKey: "settings.mcpServers.executeTimeout",
        keywords: [i18n.t("settingsSearch.mcpServers.executeTimeout"), "execute timeout", "timeout"],
      },
      {
        itemKey: "mcpServers:enabled",
        labelKey: "common.enabled",
        keywords: [i18n.t("settingsSearch.mcpServers.enabled"), "enabled", i18n.t("settingsSearch.mcpServers.enable")],
      },
    ],
  },
];
