// SPDX-License-Identifier: AGPL-3.0-only

import i18n from "@/i18n";
import { invoke, logIpcError } from "@/lib/invoke";
import { extractRequiredCommands, validateSkillPermissions } from "@/lib/skillPermissions";
import type {
  DeclarativeActionType,
  Skill,
  SkillCapability,
  SkillCommandAction,
  SkillHandler,
  SkillToolbarCapability,
  UISchema,
} from "@/types";
import { create } from "zustand";

// 注意：此处静态导入 skillStore 与 skillStore 中的静态导入
// skillExtensionStore 构成循环依赖，但双方仅在函数体内（运行时）引用对方，
// 不会在模块初始化阶段访问，因此安全。
import { useSkillStore } from "./skillStore";

export interface MergedCommand {
  id: string;
  label: string;
  description?: string;
  category?: string;
  icon?: string;
  shortcut?: string;
  actions: SkillCommandAction[];
  skillName: string;
}

export interface MergedPanel {
  id: string;
  title: string;
  componentType: string;
  componentConfig: Record<string, unknown>;
  position: string;
  size: string;
  collapsible: boolean;
  defaultCollapsed: boolean;
  skillName: string;
  sourcePath: string;
  /** DynamicUI Schema（Phase 2），Agent 动态生成的 UI Schema */
  uiSchema?: UISchema;
}

export interface MergedSettingsSection {
  id: string;
  title: string;
  icon?: string;
  settingsGroup: string;
  componentType: string;
  componentConfig: Record<string, unknown>;
  skillName: string;
  sourcePath: string;
}

export interface MergedToolbarButton {
  id: string;
  icon: string;
  tooltip: string;
  position: "left" | "right";
  priority: number;
  onClick: SkillCommandAction[];
  menu?: { label: string; actions: SkillCommandAction[] }[];
  skillName: string;
}

export interface MergedChatCommand {
  name: string;
  description: string;
  icon?: string;
  mode: "declarative" | "agentic";
  actions?: SkillCommandAction[];
  skillName: string;
}

export interface MergedStatusBarItem {
  id: string;
  alignment: "left" | "right";
  priority: number;
  text?: string;
  icon?: string;
  dynamicText?: {
    command: string;
    args?: Record<string, unknown>;
    refreshIntervalMs: number;
    template?: string;
  };
  onClick?: SkillCommandAction[];
  skillName: string;
}

interface SkillExtensionState {
  skills: Skill[];
  loading: boolean;

  commands: MergedCommand[];
  panels: MergedPanel[];
  settingsSections: MergedSettingsSection[];
  toolbarButtons: MergedToolbarButton[];
  chatCommands: MergedChatCommand[];
  statusBarItems: MergedStatusBarItem[];
  handlers: Record<string, SkillHandler>;

  fetchSkills: () => Promise<void>;
  /** 从内存中的 skills 数组直接合并扩展，消除重复 IPC 调用。
   *  由 skillStore 在加载完技能后调用。 */
  syncFromSkills: (skills: Skill[]) => void;
  getHandler: (name: string) => SkillHandler | undefined;
  refreshSkill: (skillName: string) => Promise<void>;
}

function namespaceId(skillName: string, id: string): string {
  return `${skillName}::${id}`;
}

function rewriteDeclarativeAction(
  action: DeclarativeActionType,
  skillName: string,
): DeclarativeActionType {
  if (action.type === "handler") {
    return { ...action, name: `${skillName}::${action.name}` };
  }
  if (action.type === "chain") {
    return {
      ...action,
      actions: action.actions.map((a) => rewriteDeclarativeAction(a, skillName)),
    };
  }
  return action;
}

function rewriteHandlerActions(
  actions: SkillCommandAction[],
  skillName: string,
): SkillCommandAction[] {
  return actions.map((action) => {
    if (action.mode === "declarative") {
      return {
        ...action,
        action: rewriteDeclarativeAction(action.action, skillName),
      };
    }
    return action;
  });
}

function mergeExtensions(skills: Skill[]) {
  const commands: MergedCommand[] = [];
  const panels: MergedPanel[] = [];
  const settingsSections: MergedSettingsSection[] = [];
  const toolbarButtons: MergedToolbarButton[] = [];
  const chatCommands: MergedChatCommand[] = [];
  const statusBarItems: MergedStatusBarItem[] = [];
  const handlers: Record<string, SkillHandler> = {};
  const seenIds = new Map<string, Set<string>>();
  const toolbarPositionMap = new Map<string, Set<string>>();

  function checkDuplicate(
    type: string,
    id: string,
    skillName: string,
  ): boolean {
    const namespacedId = namespaceId(skillName, id);
    if (!seenIds.has(type)) {
      seenIds.set(type, new Set());
    }
    const ids = seenIds.get(type)!;
    if (ids.has(namespacedId)) {
      return true;
    }
    ids.add(namespacedId);
    return false;
  }

  function checkToolbarPositionConflict(
    position: string,
    skillName: string,
  ): void {
    if (!toolbarPositionMap.has(position)) {
      toolbarPositionMap.set(position, new Set());
    }
    const skillsAtPosition = toolbarPositionMap.get(position)!;
    if (skillsAtPosition.size > 0 && !skillsAtPosition.has(skillName)) {
      // 位置冲突检测：记录已有技能
    }
    skillsAtPosition.add(skillName);
  }

  for (const skill of skills) {
    const capabilities = skill.manifest?.capabilities;
    if (!capabilities || capabilities.length === 0) {
      continue;
    }

    const perms = skill.manifest?.permissions;
    const required = extractRequiredCommands(capabilities);
    const permResult = validateSkillPermissions(perms, required);
    if (!permResult.valid) {
      continue;
    }

    for (const cap of capabilities) {
      const capType = cap.type;
      const capId = cap.id;

      if (capType === "toolbar") {
        checkToolbarPositionConflict(
          (cap as SkillToolbarCapability).position,
          skill.name,
        );
      }

      if (!checkDuplicate(capType, capId, skill.name)) {
        mergeCapability(cap, skill, {
          commands,
          panels,
          settingsSections,
          toolbarButtons,
          chatCommands,
          statusBarItems,
          handlers,
        });
      }
    }
  }

  return {
    commands,
    panels,
    settingsSections,
    toolbarButtons,
    chatCommands,
    statusBarItems,
    handlers,
  };
}

/** 将单个 capability 合并到对应的扩展列表 */
function mergeCapability(
  cap: SkillCapability,
  skill: Skill,
  target: {
    commands: MergedCommand[];
    panels: MergedPanel[];
    settingsSections: MergedSettingsSection[];
    toolbarButtons: MergedToolbarButton[];
    chatCommands: MergedChatCommand[];
    statusBarItems: MergedStatusBarItem[];
    handlers: Record<string, SkillHandler>;
  },
): void {
  switch (cap.type) {
    case "panel":
      target.panels.push({
        id: namespaceId(skill.name, cap.id),
        title: cap.title,
        componentType: cap.componentType,
        componentConfig: cap.componentConfig as Record<string, unknown>,
        position: cap.position,
        size: cap.size || "Medium",
        collapsible: cap.collapsible ?? true,
        defaultCollapsed: cap.defaultCollapsed ?? false,
        skillName: skill.name,
        sourcePath: skill.sourcePath,
      });
      break;
    case "toolbar":
      target.toolbarButtons.push({
        id: namespaceId(skill.name, cap.id),
        icon: cap.icon,
        tooltip: cap.tooltip || cap.title || "",
        position: cap.position,
        priority: cap.priority ?? 10,
        onClick: rewriteHandlerActions(cap.onClick, skill.name),
        menu: cap.menu?.map((m) => ({
          ...m,
          actions: rewriteHandlerActions(m.actions, skill.name),
        })),
        skillName: skill.name,
      });
      break;
    case "chatCommand": {
      const rewrittenActions = rewriteHandlerActions(
        cap.actions || [],
        skill.name,
      );
      const handlerKey = namespaceId(skill.name, cap.commandName);
      target.chatCommands.push({
        name: cap.commandName,
        description: cap.description,
        icon: cap.icon,
        mode: cap.mode,
        actions: rewrittenActions,
        skillName: skill.name,
      });
      target.handlers[handlerKey] = {
        mode: cap.mode,
        description: cap.description,
        actions: rewrittenActions,
      };
      break;
    }
    case "statusBar":
      target.statusBarItems.push({
        id: namespaceId(skill.name, cap.id),
        alignment: cap.alignment,
        priority: cap.priority ?? 10,
        text: cap.text,
        icon: cap.icon,
        dynamicText: cap.dynamicText,
        onClick: cap.onClick
          ? rewriteHandlerActions(cap.onClick, skill.name)
          : undefined,
        skillName: skill.name,
      });
      break;
    case "settings":
      target.settingsSections.push({
        id: namespaceId(skill.name, cap.id),
        title: cap.title,
        icon: cap.icon,
        settingsGroup: cap.settingsGroup,
        componentType: cap.componentType,
        componentConfig: cap.componentConfig as Record<string, unknown>,
        skillName: skill.name,
        sourcePath: skill.sourcePath,
      });
      break;
    default:
      break;
  }
}

export const useSkillExtensionStore = create<SkillExtensionState>(
  (set, get) => ({
    skills: [],
    loading: false,
    commands: [],
    panels: [],
    settingsSections: [],
    toolbarButtons: [],
    chatCommands: [],
    statusBarItems: [],
    handlers: {},

    fetchSkills: async () => {
      set({ loading: true });
      try {
        const skills = await invoke<Skill[]>("list_skills");
        const merged = mergeExtensions(skills);
        set({ skills, ...merged, loading: false });
      } catch (e) {
        logIpcError(i18n.t("skillExtension.fetchFailed"))(e);
        set({
          loading: false,
          skills: [],
          commands: [],
          panels: [],
          settingsSections: [],
          toolbarButtons: [],
          chatCommands: [],
          statusBarItems: [],
          handlers: {},
        });
      }
    },

    syncFromSkills: (skills: Skill[]) => {
      const merged = mergeExtensions(skills);
      set({ skills, ...merged, loading: false });
    },

    getHandler: (name: string) => get().handlers[name],

    refreshSkill: async (_skillName: string) => {
      const skills = await invoke<Skill[]>("list_skills");
      // 直接使用后端最新 skills 列表，以 skill ID 为权威数据源
      const merged = mergeExtensions(skills);
      set({ skills, ...merged });
    },
  }),
);

// 注册热重载监听（模块加载时执行一次）
let _hotReloadRegistered = false;
export function ensureHotReloadRegistered() {
  if (_hotReloadRegistered) {
    return;
  }
  _hotReloadRegistered = true;

  // 优先使用 Tauri 事件系统
  import("@/lib/invoke").then(({ listen }) => {
    listen<{ skillName: string }>("skill:file-changed", (event) => {
      const { skillName } = event.payload;
      useSkillExtensionStore.getState().refreshSkill(skillName);
    }).catch(() => {
      // 非 Tauri 环境（浏览器开发模式），使用轮询
      setupBrowserPolling();
    });
  });
}

/**
 * 浏览器开发模式下的 Skill 热重载（轮询方案）。
 * 每 5 秒检测一次 Skill 列表是否有变化。
 * 注意：此方案仅在浏览器模式下工作，生产环境走 Tauri 事件。
 */
let _pollingTimer: ReturnType<typeof setInterval> | null = null;
function setupBrowserPolling(): void {
  if (_pollingTimer) {
    return;
  }
  // 仅在开发模式启用
  if (!import.meta.env.DEV) {
    return;
  }

  let lastHash = "";
  _pollingTimer = setInterval(async () => {
    try {
      const { invoke } = await import("@/lib/invoke");
      const skills = await invoke<Array<{ name: string; enabled: boolean }>>("list_skills");
      const currentHash = JSON.stringify(
        skills.map((s) => `${s.name}:${s.enabled}`).sort(),
      );
      if (currentHash !== lastHash && lastHash !== "") {
        useSkillExtensionStore.getState().fetchSkills();
        useSkillStore.getState().loadSkills();
      }
      lastHash = currentHash;
    } catch {
      // 浏览器模式下 list_skills 可能不存在，静默忽略
    }
  }, 5000);
}

// HMR 清理：开发热更新会重新求值本模块，旧模块的轮询定时器若不显式清除
// 会累积多个 5s 轮询。生产环境走 Tauri 事件，不触发此分支。
if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    if (_pollingTimer !== null) {
      clearInterval(_pollingTimer);
      _pollingTimer = null;
    }
  });
}
