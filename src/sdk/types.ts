// SPDX-License-Identifier: AGPL-3.0-only

/**
 * AxAgent Skill SDK — 公共类型定义
 *
 * Skill 开发者使用这些类型来声明 Skill 的能力（capability）、
 * 编写 Skill 前端组件代码。宿主应用通过 postMessage RPC 协议
 * 向 Skill 沙箱暴露 ctx 对象。
 *
 * @module sdk/types
 */

// ── 基础类型 ─────────────────────────────────────────────────────────

/** Skill 清单（skill.json 顶层结构） */
export interface SkillManifest {
  /** 技能名称（唯一标识，kebab-case） */
  name: string;
  /** 语义化版本号 */
  version: string;
  /** 技能描述（会注入 LLM 上下文） */
  description: string;
  /** 作者信息 */
  author?: string;
  /** 图标标识符，如 "lucide:FolderOpen" */
  icon?: string;
  /** 依赖的其他 Skill（名称 → 版本约束） */
  dependencies?: Record<string, string>;
  /** 权限声明（白名单，宿主强制执行） */
  permissions: SkillPermissions;
  /** 能力声明列表 */
  capabilities: SkillCapability[];
  /** 生命周期钩子 */
  lifecycle?: SkillLifecycleHooks;
  /** Skill SKILL.md 的主内容文件路径，默认 "SKILL.md" */
  entryPoint?: string;
}

// ── 权限声明（加载时强制执行的白名单） ─────────────────────────────

export interface SkillPermissions {
  commands?: string[];
  events?: string[];
  storeRead?: string[];
  storeWrite?: string[];
  navigate?: string[];
  network?: string[];
  filesystem?: {
    read?: string[];
    write?: string[];
  };
  tools?: string[];
}

// ── 能力声明（Capability-based Manifest） ──────────────────────────

export type SkillCapability =
  | SkillPageCapability
  | SkillCommandCapability
  | SkillPanelCapability
  | SkillToolbarCapability
  | SkillChatCommandCapability
  | SkillStatusBarCapability
  | SkillNavigationCapability
  | SkillSettingsCapability
  | SkillToolCapability;

/** 基础能力字段 */
interface BaseCapability {
  /** 能力唯一 ID */
  id: string;
  /** 能力标题 */
  title: string;
  /** 能力描述 */
  description?: string;
}

// ── 页面能力 ─

export interface SkillPageCapability extends BaseCapability {
  type: "page";
  /** 页面布局模式 */
  layout?: "default" | "fullscreen" | "sidebar";
  /** 页面路由路径，如 "/skills/my-skill/dashboard" */
  route?: string;
  /** 图标标识符 */
  icon?: string;
  /** 渲染源：skill 目录下的 HTML 文件路径 */
  entry: string;
  /** 传递给 Skill 的初始 props */
  props?: Record<string, unknown>;
  /** 页面加载时调用的 RPC 方法名 */
  onLoad?: string;
  /** 页面卸载时调用的 RPC 方法名 */
  onUnload?: string;
}

// ── 命令面板能力 ─

export interface SkillCommandCapability extends BaseCapability {
  type: "command";
  /** 命令分类 */
  category?: string;
  /** 图标标识符 */
  icon?: string;
  /** 键盘快捷键 */
  shortcut?: string;
  /** 命令触发时调用的 RPC 方法名 或 声明式动作 */
  action?: SkillActionDeclarative | string;
}

// ── 面板能力 ─

export interface SkillPanelCapability extends BaseCapability {
  type: "panel";
  /** 面板位置 */
  position: "main" | "sidebar" | "header" | "footer";
  /** 面板尺寸 */
  size?: "small" | "medium" | "large" | "full";
  /** 是否可折叠 */
  collapsible?: boolean;
  /** 默认是否折叠 */
  defaultCollapsed?: boolean;
  /** 渲染源 */
  entry: string;
  /** 初始 props */
  props?: Record<string, unknown>;
  /** 面板加载/卸载钩子 */
  onLoad?: string;
  onUnload?: string;
}

// ── 工具栏能力 ─

export interface SkillToolbarCapability extends BaseCapability {
  type: "toolbar";
  /** 图标 */
  icon: string;
  /** 提示文本 */
  tooltip?: string;
  /** 位置 */
  position: "left" | "right";
  /** 优先级（越小越靠前） */
  priority?: number;
  /** 点击时调用的 RPC 方法名 或 声明式动作 */
  action?: SkillActionDeclarative | string;
  /** 下拉菜单项 */
  menu?: {
    label: string;
    action: SkillActionDeclarative | string;
  }[];
}

// ── 聊天命令能力 ─

export interface SkillChatCommandCapability extends BaseCapability {
  type: "chatCommand";
  /** 命令名称（/xxx） */
  commandName: string;
  /** 图标 */
  icon?: string;
  /** 执行模式 */
  mode: "declarative" | "agentic";
  /** 声明式动作（mode=declarative 时使用） */
  action?: SkillActionDeclarative;
  /** LLM 提示模板（mode=agentic 时使用） */
  promptTemplate?: string;
  /** 上下文收集配置 */
  contextGatherer?: {
    includeConversation?: boolean;
    includeFiles?: boolean;
    includeSelection?: boolean;
  };
  /** 命令参数定义 */
  args?: {
    name: string;
    description: string;
    required?: boolean;
    type: "string" | "number" | "boolean" | "file";
  }[];
}

// ── 状态栏能力 ─

export interface SkillStatusBarCapability extends BaseCapability {
  type: "statusBar";
  /** 对齐方向 */
  alignment: "left" | "right";
  /** 优先级 */
  priority?: number;
  /** 静态显示文本 或 动态数据源 */
  text?: string;
  /** 图标 */
  icon?: string;
  /** 动态文本轮询配置 */
  dynamicText?: {
    /** 调用的 Tauri 命令 */
    command: string;
    /** 命令参数 */
    args?: Record<string, unknown>;
    /** 刷新间隔（毫秒） */
    refreshIntervalMs: number;
    /** 模板字符串，{{value}} 会被替换 */
    template?: string;
  };
  /** 点击时调用的 RPC 方法名 或 声明式动作 */
  action?: SkillActionDeclarative | string;
}

// ── 导航能力 ─

export interface SkillNavigationCapability extends BaseCapability {
  type: "navigation";
  /** 图标 */
  icon: string;
  /** 关联的页面 ID */
  pageId: string;
  /** 排序位置 */
  position?: number;
  /** 徽章 */
  badge?: {
    text?: string;
    /** 动态徽章数据源（Tauri 命令） */
    command: string;
    refreshIntervalMs?: number;
  };
  /** 可见性条件（Tauri 命令，返回 boolean） */
  visibleCondition?: string;
}

// ── 设置面板能力 ─

export interface SkillSettingsCapability extends BaseCapability {
  type: "settings";
  /** 设置分组 */
  settingsGroup: string;
  /** 图标 */
  icon?: string;
  /** 渲染源 */
  entry: string;
  /** 初始 props */
  props?: Record<string, unknown>;
}

// ── 工具能力（同时注册为 LLM Function Call） ─

export interface SkillToolCapability extends BaseCapability {
  type: "tool";
  /** 工具参数 Schema（JSON Schema） */
  parameters: Record<string, unknown>;
  /** 工具执行处理器（RPC 方法名 或 声明式动作） */
  handler: SkillActionDeclarative | string;
  /** 是否需要用户确认 */
  requiresApproval?: boolean;
}

// ── 声明式动作（与旧版兼容，增强版） ─────────────────────────────

export type SkillActionDeclarative =
  | { type: "invoke"; command: string; args?: Record<string, unknown> }
  | { type: "navigate"; path: string }
  | { type: "emit"; event: string; payload?: unknown }
  | { type: "storeRead"; storeName: string }
  | { type: "storeWrite"; storeName: string; payload: unknown }
  | { type: "storeUpdate"; storeName: string; payload: unknown }
  | { type: "rpc"; method: string; args?: Record<string, unknown> }
  | { type: "chain"; actions: SkillActionDeclarative[] };

// ── 生命周期钩子 ───────────────────────────────────────────────────

export interface SkillLifecycleHooks {
  /** 安装后触发 */
  onInstall?: SkillActionDeclarative[];
  /** 启用时触发 */
  onEnable?: SkillActionDeclarative[];
  /** 禁用时触发 */
  onDisable?: SkillActionDeclarative[];
  /** 卸载前触发 */
  onUninstall?: SkillActionDeclarative[];
}

// ── RPC 协议消息类型 ──────────────────────────────────────────────

/** 宿主 → Skill（请求或响应） */
export type HostToSkillMessage =
  | { type: "rpc:response"; callId: string; result?: unknown; error?: string }
  | { type: "host:event"; event: string; payload?: unknown }
  | {
    type: "host:lifecycle";
    phase: "mount" | "unmount";
    props?: Record<string, unknown>;
  };

/** Skill → 宿主（请求或事件） */
export type SkillToHostMessage =
  | {
    type: "rpc:request";
    callId: string;
    method: string;
    args?: Record<string, unknown>;
  }
  | { type: "skill:ready" }
  | { type: "skill:error"; error: string };

// ── 宿主 API 类型（ctx 对象的类型约束） ─────────────────────────

/** ctx.api — 后端通信 */
export interface SkillHostApi {
  /** 调用 Tauri 后端命令 */
  invoke<T = unknown>(
    command: string,
    args?: Record<string, unknown>,
  ): Promise<T>;
  /** 发送事件 */
  emit(event: string, payload?: unknown): void;
}

/** ctx.ui — UI 操作 */
export interface SkillHostUi {
  /** 路由导航 */
  navigate(path: string): void;
  /** 显示通知 */
  notify(
    message: string,
    type?: "info" | "success" | "warning" | "error",
  ): void;
  /** 获取当前主题模式 */
  getTheme(): "light" | "dark";
  /** 获取当前语言 */
  getLocale(): string;
}

/** ctx.store — 状态读写 */
export interface SkillHostStore {
  read<T = unknown>(storeName: string, selector?: string): Promise<T>;
  write(storeName: string, value: unknown, selector?: string): Promise<void>;
}

/** Skill 沙箱内可用的全局 ctx 对象 */
export interface SkillContext {
  /** Skill 名称 */
  readonly skillName: string;
  /** Skill ID */
  readonly skillId: string;
  /** 宿主注入的初始 props */
  readonly props: Record<string, unknown>;
  /** 后端通信 API */
  readonly api: SkillHostApi;
  /** UI 操作 API */
  readonly ui: SkillHostUi;
  /** 状态读写 API */
  readonly store: SkillHostStore;
}
