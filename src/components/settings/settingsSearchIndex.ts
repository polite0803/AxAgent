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
    keywords: ["模型提供商", "服务商", "provider", "api", "密钥", "key", "llm", "接入", "openai", "模型管理"],
  },
  {
    section: "defaultModel",
    group: "model",
    keywords: ["默认模型", "default model", "模型选择", "当前模型", "首选模型"],
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
        keywords: ["气泡样式", "消息气泡", "bubble style", "bubble"],
      },
      {
        itemKey: "conversation:renderUserMarkdown",
        labelKey: "settings.renderUserMarkdown",
        keywords: ["渲染用户", "用户消息", "markdown", "render user"],
      },
      {
        itemKey: "conversation:multiModelDisplayMode",
        labelKey: "settings.multiModelDisplayMode",
        keywords: ["多模型", "显示模式", "multi model", "display mode", "tabs", "并排对比", "堆叠"],
      },
      {
        itemKey: "conversation:chatMinimapEnabled",
        labelKey: "settings.chatMinimapEnabled",
        keywords: ["对话导航", "聊天缩略图", "minimap", "对话缩略图"],
      },
      {
        itemKey: "conversation:chatMinimapStyle",
        labelKey: "settings.chatMinimapStyle",
        keywords: ["导航样式", "缩略图样式", "minimap style", "问题索引", "浮动指示器"],
      },
    ],
  },
  {
    section: "promptTemplates",
    group: "model",
    keywords: ["提示词模板", "prompt", "模板", "template", "系统提示", "system prompt"],
  },
  {
    section: "searchProviders",
    group: "model",
    keywords: ["搜索提供商", "search", "搜索引擎", "联网搜索", "tavily", "web search"],
    items: [
      {
        itemKey: "searchProviders:name",
        labelKey: "settings.searchProviders.name",
        keywords: ["名称", "name", "provider name"],
      },
      {
        itemKey: "searchProviders:type",
        labelKey: "settings.searchProviders.type",
        keywords: ["类型", "type", "provider type", "tavily", "brave", "bing"],
      },
      {
        itemKey: "searchProviders:endpoint",
        labelKey: "settings.searchProviders.endpoint",
        keywords: ["端点", "endpoint", "api地址"],
      },
      {
        itemKey: "searchProviders:apiKey",
        labelKey: "settings.searchProviders.apiKeySet",
        keywords: ["api key", "密钥", "apikey"],
      },
      {
        itemKey: "searchProviders:resultLimit",
        labelKey: "settings.searchProviders.resultLimit",
        keywords: ["结果限制", "result limit", "result count"],
      },
      {
        itemKey: "searchProviders:timeout",
        labelKey: "settings.searchProviders.timeout",
        keywords: ["超时", "timeout", "ms"],
      },
      {
        itemKey: "searchProviders:enabled",
        labelKey: "common.enabled",
        keywords: ["启用", "enabled", "开启"],
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
      "自启动",
      "开机自启",
      "托盘",
      "tray",
      "置顶",
      "最小化",
      "startup",
      "autostart",
      "always on top",
    ],
    items: [
      {
        itemKey: "general:language",
        labelKey: "settings.language",
        keywords: ["语言", "language", "locale", "国际化", "i18n", "多语言"],
      },
      {
        itemKey: "general:autoStart",
        labelKey: "settings.autoStart",
        keywords: ["自启动", "开机自启", "auto start", "autostart", "开机启动"],
      },
      {
        itemKey: "general:showOnStart",
        labelKey: "settings.showOnStart",
        keywords: ["启动时显示", "启动显示", "show on start", "开机显示"],
      },
      {
        itemKey: "general:alwaysOnTop",
        labelKey: "desktop.alwaysOnTop",
        keywords: ["置顶", "always on top", "窗口置顶", "最前"],
      },
      {
        itemKey: "general:startMinimized",
        labelKey: "desktop.startMinimized",
        keywords: ["启动最小化", "最小化启动", "start minimized", "最小化"],
      },
      {
        itemKey: "general:minimizeToTray",
        labelKey: "settings.minimizeToTray",
        keywords: ["托盘", "tray", "最小化到托盘", "系统托盘", "后台"],
      },
      {
        itemKey: "general:defaultWorkspaceDir",
        labelKey: "settings.defaultWorkspaceDir",
        keywords: ["工作区", "workspace", "默认目录", "工作目录", "路径"],
      },
    ],
  },
  {
    section: "display",
    group: "appearance",
    keywords: ["显示设置", "display", "字体", "font", "字号", "缩放", "zoom", "布局", "密度", "density", "显示"],
    items: [
      {
        itemKey: "display:themeMode",
        labelKey: "settings.theme.label",
        keywords: ["主题模式", "theme", "深色", "浅色", "dark", "light", "跟随系统", "system"],
      },
      {
        itemKey: "display:themePreset",
        labelKey: "settings.themePreset",
        keywords: ["主题预设", "theme preset", "预设", "配色方案", "颜色主题"],
      },
      {
        itemKey: "display:primaryColor",
        labelKey: "settings.primaryColor",
        keywords: ["强调色", "主色", "primary color", "accent", "主题色", "品牌色"],
      },
      {
        itemKey: "display:fontSize",
        labelKey: "settings.fontSize",
        keywords: ["字号", "字体大小", "font size", "字体尺寸"],
      },
      {
        itemKey: "display:fontWeight",
        labelKey: "settings.fontWeight",
        keywords: ["字重", "font weight", "粗细", "加粗", "字体粗细"],
      },
      {
        itemKey: "display:fontFamily",
        labelKey: "settings.fontFamily",
        keywords: ["字体", "font", "font family", "界面字体", "系统字体"],
      },
      {
        itemKey: "display:codeFontFamily",
        labelKey: "settings.codeFontFamily",
        keywords: ["代码字体", "code font", "等宽字体", "monospace", "编程字体"],
      },
      {
        itemKey: "display:codeThemeLight",
        labelKey: "settings.codeThemeLight",
        keywords: ["代码主题", "code theme", "浅色代码", "代码高亮", "syntax highlight"],
      },
      {
        itemKey: "display:codeThemeDark",
        labelKey: "settings.codeThemeDark",
        keywords: ["代码主题", "code theme", "深色代码", "代码高亮", "syntax highlight"],
      },
      {
        itemKey: "display:borderRadius",
        labelKey: "settings.borderRadius",
        keywords: ["圆角", "border radius", "圆角半径", "圆角大小"],
      },
    ],
  },
  {
    section: "theme",
    group: "appearance",
    keywords: ["主题", "theme", "深色", "浅色", "dark", "light", "外观", "颜色", "强调色", "accent", "配色"],
  },
  {
    section: "shortcuts",
    group: "appearance",
    keywords: ["快捷键", "shortcut", "热键", "hotkey", "键位", "绑定", "keybinding"],
    items: [
      {
        itemKey: "shortcuts:enableGlobalShortcuts",
        labelKey: "settings.enableGlobalShortcuts",
        keywords: ["全局快捷键", "global shortcut", "启用"],
      },
      {
        itemKey: "shortcuts:enableShortcutRegistrationLogs",
        labelKey: "settings.enableShortcutRegistrationLogs",
        keywords: ["快捷键日志", "注册日志", "shortcut log", "diagnostic"],
      },
      {
        itemKey: "shortcuts:enableShortcutTriggerToast",
        labelKey: "settings.enableShortcutTriggerToast",
        keywords: ["触发提示", "toast", "shortcut trigger"],
      },
    ],
  },
  {
    section: "tools",
    group: "extensions",
    keywords: ["工具", "tool", "本地工具", "local tool", "mcp", "函数调用", "function calling", "工具管理"],
  },
  {
    section: "skillsHub",
    group: "extensions",
    keywords: ["技能中心", "skill", "技能", "插件市场", "marketplace", "技能市场"],
  },
  {
    section: "plugins",
    group: "extensions",
    keywords: ["插件", "plugin", "扩展", "extension"],
  },
  {
    section: "dashboardPlugins",
    group: "extensions",
    keywords: ["仪表盘插件", "dashboard", "看板", "widget", "组件"],
  },
  {
    section: "dynamicPages",
    group: "extensions",
    keywords: ["动态页面", "dynamic page", "自定义页面", "custom page", "页面"],
  },
  {
    section: "appConfig",
    group: "extensions",
    keywords: [
      "应用配置",
      "app config",
      "agent 控制面板",
      "权限",
      "permission",
      "迭代",
      "iteration",
      "功能开关",
      "feature flag",
      "hook",
    ],
    items: [
      {
        itemKey: "agent:maxIterations",
        labelKey: "settings.agent.maxIterations",
        keywords: ["迭代次数", "max iterations", "max iterations"],
      },
      {
        itemKey: "agent:permissionMode",
        labelKey: "settings.agent.permissionMode",
        keywords: ["权限模式", "permission mode", "只读", "完全访问", "写入"],
      },
      {
        itemKey: "agent:forkSubagent",
        labelKey: "settings.agent.featureFlags.forkSubagent",
        keywords: ["fork", "子agent", "subagent", "并行"],
      },
      {
        itemKey: "agent:coordinatorMode",
        labelKey: "settings.agent.featureFlags.coordinatorMode",
        keywords: ["协调者", "coordinator", "调度"],
      },
      {
        itemKey: "agent:proactiveMode",
        labelKey: "settings.agent.featureFlags.proactiveMode",
        keywords: ["主动模式", "proactive", "预测"],
      },
      {
        itemKey: "agent:swarmMode",
        labelKey: "settings.agent.featureFlags.swarmMode",
        keywords: ["集群模式", "swarm", "集群协作"],
      },
      {
        itemKey: "agent:toolConcurrency",
        labelKey: "settings.agent.featureFlags.toolConcurrency",
        keywords: ["工具并发", "tool concurrency", "并行"],
      },
      {
        itemKey: "agent:verificationAgent",
        labelKey: "settings.agent.featureFlags.verificationAgent",
        keywords: ["验证", "verification agent", "审查"],
      },
      {
        itemKey: "agent:dreamTask",
        labelKey: "settings.agent.featureFlags.dreamTask",
        keywords: ["梦境", "dream task", "后台优化"],
      },
    ],
  },
  {
    section: "imageGen",
    group: "extensions",
    keywords: ["图像生成", "image gen", "文生图", "画图", "绘画", "draw", "stable diffusion", "dall"],
  },
  {
    section: "proxy",
    group: "network",
    keywords: ["代理", "proxy", "网络", "network", "翻墙", "科学上网", "http 代理"],
    items: [
      {
        itemKey: "proxy:proxyType",
        labelKey: "settings.proxyType",
        keywords: ["代理类型", "proxy type", "http", "socks5", "system", "none"],
      },
      {
        itemKey: "proxy:proxyAddress",
        labelKey: "settings.proxyAddress",
        keywords: ["代理地址", "proxy address", "proxy host"],
      },
      {
        itemKey: "proxy:proxyPort",
        labelKey: "settings.proxyPort",
        keywords: ["代理端口", "proxy port", "port"],
      },
    ],
  },
  {
    section: "messageChannels",
    group: "network",
    keywords: ["消息渠道", "message channel", "通知渠道", "平台", "telegram", "钉钉", "微信", "飞书", "discord"],
  },
  {
    section: "webhooks",
    group: "network",
    keywords: ["webhook", "回调", "callback", "钩子", "通知回调"],
  },
  {
    section: "acp",
    group: "network",
    keywords: ["acp", "代理通信协议", "agent client protocol"],
    items: [
      {
        itemKey: "acp:serverAddress",
        labelKey: "acp.serverAddress",
        keywords: ["服务器地址", "server address", "acp", "base url"],
      },
      {
        itemKey: "acp:connectionStatus",
        labelKey: "acp.connectionStatus",
        keywords: ["连接状态", "connection status", "connected"],
      },
      {
        itemKey: "acp:workdir",
        labelKey: "acp.workdir",
        keywords: ["工作目录", "workdir", "working directory", "会话"],
      },
    ],
  },
  {
    section: "data",
    group: "data",
    keywords: ["数据", "data", "数据管理", "导出", "导入", "清空", "重置", "data manager"],
    items: [
      {
        itemKey: "data:exportData",
        labelKey: "settings.exportData",
        keywords: ["导出数据", "export", "数据导出"],
      },
      {
        itemKey: "data:importData",
        labelKey: "settings.importData",
        keywords: ["导入数据", "import", "数据导入"],
      },
      {
        itemKey: "data:clearData",
        labelKey: "settings.clearData",
        keywords: ["清除数据", "清除对话", "clear", "delete", "danger"],
      },
    ],
  },
  {
    section: "database",
    group: "data",
    keywords: ["数据库", "database", "存储", "sqlite", "连接", "db"],
  },
  {
    section: "storage",
    group: "data",
    keywords: ["存储", "storage", "空间", "磁盘", "缓存", "cache", "占用", "清理"],
  },
  {
    section: "cloudWorkspace",
    group: "data",
    keywords: ["云端工作区", "cloud workspace", "同步", "sync", "云工作区"],
  },
  {
    section: "backup",
    group: "data",
    keywords: ["备份", "backup", "恢复", "restore", "快照", "snapshot"],
  },
  {
    section: "scheduler",
    group: "data",
    keywords: ["定时任务", "scheduler", "计划", "任务", "task", "调度"],
    items: [
      {
        itemKey: "scheduler:autoBackupEnabled",
        labelKey: "settings.scheduler.enabled",
        keywords: ["自动备份", "auto backup", "启用"],
      },
      {
        itemKey: "scheduler:backupInterval",
        labelKey: "settings.scheduler.backupInterval",
        keywords: ["备份间隔", "backup interval", "小时"],
      },
      {
        itemKey: "scheduler:maxBackupCount",
        labelKey: "settings.scheduler.maxCount",
        keywords: ["备份保留", "max count", "最大数量"],
      },
      {
        itemKey: "scheduler:webdavSyncEnabled",
        labelKey: "settings.scheduler.enabled",
        keywords: ["webdav同步", "自动同步", "webdav sync"],
      },
      {
        itemKey: "scheduler:syncInterval",
        labelKey: "settings.scheduler.syncInterval",
        keywords: ["同步间隔", "sync interval", "分钟"],
      },
      {
        itemKey: "scheduler:maxRemoteBackups",
        labelKey: "settings.scheduler.maxRemoteBackups",
        keywords: ["远程保留", "remote backup", "max"],
      },
      {
        itemKey: "scheduler:closedLoopEnabled",
        labelKey: "settings.scheduler.enabled",
        keywords: ["闭环学习", "closed loop", "学习提示"],
      },
      {
        itemKey: "scheduler:nudgeInterval",
        labelKey: "settings.scheduler.nudgeInterval",
        keywords: ["提示间隔", "nudge interval", "nudge", "闭环"],
      },
    ],
  },
  {
    section: "cron",
    group: "data",
    keywords: ["cron", "定时", "表达式", "expression", "周期任务", "计划任务"],
  },
  {
    section: "notificationCenter",
    group: "data",
    keywords: ["通知中心", "notification", "通知", "提醒", "alert", "消息中心"],
  },
  {
    section: "advanced",
    group: "system",
    keywords: ["高级", "advanced", "开发者", "developer", "实验", "experimental", "调试", "debug"],
    items: [
      {
        itemKey: "advanced:dangerousCmdDetect",
        labelKey: "advanced.dangerousCmdDetect",
        keywords: ["危险命令", "command detect", "安全验证", "bash"],
      },
      {
        itemKey: "advanced:networkCmdDetect",
        labelKey: "advancedSettings.networkCmdDetect",
        keywords: ["网络命令", "network detect", "网络检测"],
      },
      {
        itemKey: "advanced:cmdTimeout",
        labelKey: "advanced.cmdTimeout",
        keywords: ["命令超时", "cmd timeout", "timeout", "bash"],
      },
      {
        itemKey: "advanced:defaultPermission",
        labelKey: "advancedSettings.defaultPermission",
        keywords: ["默认权限", "permission mode", "询问", "编辑", "完全"],
      },
      {
        itemKey: "advanced:fileWriteConfirm",
        labelKey: "advanced.fileWriteConfirm",
        keywords: ["文件写入", "write confirm", "文件确认"],
      },
      {
        itemKey: "advanced:networkConfirm",
        labelKey: "advancedSettings.networkConfirm",
        keywords: ["网络确认", "network confirm", "网络请求"],
      },
      {
        itemKey: "advanced:shellConfirm",
        labelKey: "advancedSettings.shellConfirm",
        keywords: ["shell确认", "shell confirm", "命令执行"],
      },
      {
        itemKey: "advanced:defaultMode",
        labelKey: "advancedSettings.defaultMode",
        keywords: ["默认模式", "default mode", "通用", "快速", "深度", "计划"],
      },
      {
        itemKey: "advanced:tokenBudgetLimit",
        labelKey: "advancedSettings.tokenBudgetLimit",
        keywords: ["token预算", "budget", "预算", "token limit"],
      },
      {
        itemKey: "advanced:enableTokenBudget",
        labelKey: "advancedSettings.enableTokenBudget",
        keywords: ["预算检测", "token budget", "budget enable"],
      },
      {
        itemKey: "advanced:autoRetry",
        labelKey: "advancedSettings.autoRetry",
        keywords: ["自动重试", "auto retry", "重试"],
      },
      {
        itemKey: "advanced:maxRetries",
        labelKey: "advancedSettings.maxRetries",
        keywords: ["重试次数", "max retries", "retry"],
      },
      {
        itemKey: "advanced:retryDelay",
        labelKey: "advancedSettings.retryDelay",
        keywords: ["重试延迟", "retry delay", "延迟"],
      },
      {
        itemKey: "advanced:modelFallback",
        labelKey: "advancedSettings.modelFallback",
        keywords: ["模型降级", "fallback", "回退"],
      },
      {
        itemKey: "advanced:cpuLimit",
        labelKey: "advancedSettings.cpuLimit",
        keywords: ["cpu上限", "cpu limit", "cpu"],
      },
      {
        itemKey: "advanced:memoryLimit",
        labelKey: "advancedSettings.memoryLimit",
        keywords: ["内存上限", "memory limit", "内存"],
      },
      {
        itemKey: "advanced:enableIdleDetection",
        labelKey: "advancedSettings.enableIdleDetection",
        keywords: ["空转检测", "idle detect", "idle"],
      },
      {
        itemKey: "advanced:idleTimeout",
        labelKey: "advancedSettings.idleTimeout",
        keywords: ["空闲超时", "idle timeout", "timeout"],
      },
      {
        itemKey: "advanced:autoCompressThreshold",
        labelKey: "advancedSettings.autoCompressThreshold",
        keywords: ["压缩阈值", "auto compress", "context compress"],
      },
      {
        itemKey: "advanced:warningBuffer",
        labelKey: "advancedSettings.warningBuffer",
        keywords: ["警告缓冲", "warning buffer", "threshold"],
      },
      {
        itemKey: "advanced:maxConsecutiveFailures",
        labelKey: "advancedSettings.maxConsecutiveFailures",
        keywords: ["压缩失败", "compact failure", "max failures"],
      },
      {
        itemKey: "advanced:enableMemoryCompression",
        labelKey: "advancedSettings.enableMemoryCompression",
        keywords: ["记忆压缩", "memory compact", "session compress"],
      },
      {
        itemKey: "advanced:enableDream",
        labelKey: "advancedSettings.enableDream",
        keywords: ["dream", "巩固", "背景巩固", "consolidation"],
      },
      {
        itemKey: "advanced:dreamMinInterval",
        labelKey: "advancedSettings.minInterval",
        keywords: ["dream间隔", "min interval", "小时"],
      },
      {
        itemKey: "advanced:dreamMinSessions",
        labelKey: "advancedSettings.minNewSessions",
        keywords: ["新会话", "min sessions", "dream"],
      },
      {
        itemKey: "advanced:dreamMaxDuration",
        labelKey: "advancedSettings.maxDuration",
        keywords: ["持续时间", "max duration", "dream"],
      },
      {
        itemKey: "advanced:enableLspDiagnostics",
        labelKey: "advancedSettings.enableLspDiagnostics",
        keywords: ["lsp诊断", "lsp diagnostics", "语言服务器"],
      },
      {
        itemKey: "advanced:diagnosticLevel",
        labelKey: "advancedSettings.diagnosticLevelLabel",
        keywords: ["诊断级别", "diagnostic level", "error", "warning"],
      },
    ],
  },
  {
    section: "evolution",
    group: "system",
    keywords: ["进化", "evolution", "技能进化", "遗传", "genetic", "学习", "learning", "自适应"],
  },
  {
    section: "about",
    group: "system",
    keywords: ["关于", "about", "版本", "version", "许可证", "license", "更新日志", "changelog"],
    items: [
      {
        itemKey: "about:appVersion",
        labelKey: "settings.version",
        keywords: ["版本", "version", "app version"],
      },
      {
        itemKey: "about:openSource",
        labelKey: "settings.openSource",
        keywords: ["开源协议", "开源", "license", "AGPL"],
      },
      {
        itemKey: "about:overallStatus",
        labelKey: "settings.overallStatus",
        keywords: ["总体状态", "服务健康", "health", "service health"],
      },
      {
        itemKey: "about:checkUpdate",
        labelKey: "settings.checkUpdate",
        keywords: ["检查更新", "update", "check update", "版本更新"],
      },
      {
        itemKey: "about:updateCheckInterval",
        labelKey: "settings.updateCheckInterval",
        keywords: ["更新间隔", "update interval", "check interval"],
      },
      {
        itemKey: "about:developerTools",
        labelKey: "settings.developerTools",
        keywords: ["开发者工具", "devtools", "开发工具"],
      },
      {
        itemKey: "about:replayTutorial",
        labelKey: "help.onboardingReplayDesc",
        keywords: ["引导", "tutorial", "onboarding", "新手指引"],
      },
    ],
  },
  {
    section: "localTools",
    group: "other",
    keywords: ["本地工具", "local tool", "本地命令", "command", "脚本工具"],
  },
  {
    section: "mcpServers",
    group: "other",
    keywords: ["mcp", "mcp 服务器", "mcp server", "model context protocol", "服务器配置"],
    items: [
      {
        itemKey: "mcpServers:name",
        labelKey: "settings.mcpServers.name",
        keywords: ["名称", "name", "server name"],
      },
      {
        itemKey: "mcpServers:transport",
        labelKey: "settings.mcpServers.transport",
        keywords: ["传输协议", "transport", "stdio", "sse", "http"],
      },
      {
        itemKey: "mcpServers:command",
        labelKey: "settings.mcpServers.command",
        keywords: ["命令", "command", "stdio command"],
      },
      {
        itemKey: "mcpServers:args",
        labelKey: "settings.mcpServers.args",
        keywords: ["参数", "args", "arguments"],
      },
      {
        itemKey: "mcpServers:endpoint",
        labelKey: "settings.mcpServers.endpoint",
        keywords: ["端点", "endpoint", "url"],
      },
      {
        itemKey: "mcpServers:customHeaders",
        labelKey: "settings.mcpServers.customHeaders",
        keywords: ["请求头", "custom headers", "headers"],
      },
      {
        itemKey: "mcpServers:envVars",
        labelKey: "settings.mcpServers.envVars",
        keywords: ["环境变量", "env", "environment"],
      },
      {
        itemKey: "mcpServers:discoverTimeout",
        labelKey: "settings.mcpServers.discoverTimeout",
        keywords: ["发现超时", "discover timeout", "timeout"],
      },
      {
        itemKey: "mcpServers:executeTimeout",
        labelKey: "settings.mcpServers.executeTimeout",
        keywords: ["执行超时", "execute timeout", "timeout"],
      },
      {
        itemKey: "mcpServers:enabled",
        labelKey: "common.enabled",
        keywords: ["启用", "enabled", "开启"],
      },
    ],
  },
];
