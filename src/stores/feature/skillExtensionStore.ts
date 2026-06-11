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
} from "@/types";
import { create } from "zustand";

export interface MergedNavItem {
  id: string;
  label: string;
  icon: string;
  pageId: string;
  position: number;
  skillName: string;
}

export interface MergedPage {
  id: string;
  title: string;
  componentType: string;
  componentConfig: Record<string, unknown>;
  layout?: string;
  icon?: string;
  skillName: string;
  sourcePath: string;
}

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

  navItems: MergedNavItem[];
  pages: MergedPage[];
  commands: MergedCommand[];
  panels: MergedPanel[];
  settingsSections: MergedSettingsSection[];
  toolbarButtons: MergedToolbarButton[];
  chatCommands: MergedChatCommand[];
  statusBarItems: MergedStatusBarItem[];
  handlers: Record<string, SkillHandler>;

  fetchSkills: () => Promise<void>;
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
  const navItems: MergedNavItem[] = [];
  const pages: MergedPage[] = [];
  const commands: MergedCommand[] = [];
  const panels: MergedPanel[] = [];
  const settingsSections: MergedSettingsSection[] = [];
  const toolbarButtons: MergedToolbarButton[] = [];
  const chatCommands: MergedChatCommand[] = [];
  const statusBarItems: MergedStatusBarItem[] = [];
  const handlers: Record<string, SkillHandler> = {};
  const seenIds = new Map<string, Set<string>>();
  const toolbarPositionMap = new Map<string, Set<string>>();
  const pageRouteMap = new Map<string, Set<string>>();

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

  function checkPageRouteConflict(routeId: string, skillName: string): void {
    if (!pageRouteMap.has(routeId)) {
      pageRouteMap.set(routeId, new Set());
    }
    const skillsAtRoute = pageRouteMap.get(routeId)!;
    if (skillsAtRoute.size > 0 && !skillsAtRoute.has(skillName)) {
      // 路由冲突检测：记录已有技能
    }
    skillsAtRoute.add(skillName);
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
      if (capType === "page") {
        checkPageRouteConflict(capId, skill.name);
      }

      if (!checkDuplicate(capType, capId, skill.name)) {
        mergeCapability(cap, skill, {
          navItems,
          pages,
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
    navItems,
    pages,
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
    navItems: MergedNavItem[];
    pages: MergedPage[];
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
    case "page":
      target.pages.push({
        id: namespaceId(skill.name, cap.id),
        title: cap.title,
        componentType: cap.componentType,
        componentConfig: cap.componentConfig as Record<string, unknown>,
        layout: cap.componentConfig.layout,
        icon: cap.icon,
        skillName: skill.name,
        sourcePath: skill.sourcePath,
      });
      break;
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
    case "navigation":
      target.navItems.push({
        id: namespaceId(skill.name, cap.id),
        label: cap.title,
        icon: cap.icon,
        pageId: namespaceId(skill.name, cap.pageId),
        position: cap.position ?? 0,
        skillName: skill.name,
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
    navItems: [],
    pages: [],
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
        // 重置为空白状态，避免 UI 与真实状态不同步
        set({
          loading: false,
          skills: [],
          navItems: [],
          pages: [],
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

    getHandler: (name: string) => get().handlers[name],

    refreshSkill: async (skillName: string) => {
      const skills = await invoke<Skill[]>("list_skills");
      // 增量更新：只合并变化的 skill，保留其他 skill 的扩展数据
      const currentSkills = get().skills;
      const skillMap = new Map(currentSkills.map((cs) => [cs.name, cs]));

      // 使用最新数据覆盖匹配的 skill，保留不匹配的旧数据
      const updatedSkills = skills.map((s) => {
        const existing = skillMap.get(s.name);
        return existing && s.name !== skillName ? existing : s;
      });
      if (!updatedSkills.some((s) => s.name === skillName)) {
        const newSkill = skills.find((s) => s.name === skillName);
        if (newSkill) {
          updatedSkills.push(newSkill);
        }
      }
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
        await import("@/stores").then((s) => s.useSkillStore.getState().loadSkills());
      }
      lastHash = currentHash;
    } catch {
      // 浏览器模式下 list_skills 可能不存在，静默忽略
    }
  }, 5000);
}
