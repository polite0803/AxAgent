import { invoke } from "@/lib/invoke";
import { extractRequiredCommands, validateSkillPermissionsAtLoad } from "@/lib/skillPermissions";
import type {
  Skill,
  SkillCapabilityV2,
  SkillChatCommand,
  SkillFrontendExtension,
  SkillHandler,
  SkillNavItem,
  SkillPage,
  SkillSettingsSection,
  SkillStatusBarItem,
  SkillToolbarButton,
  SkillUICommand,
  SkillUIPanel,
} from "@/types";
import { create } from "zustand";

export interface MergedNavItem extends SkillNavItem {
  skillName: string;
}

export interface MergedPage extends SkillPage {
  skillName: string;
  sourcePath: string;
}

export interface MergedCommand extends SkillUICommand {
  skillName: string;
}

export interface MergedPanel extends SkillUIPanel {
  skillName: string;
  sourcePath: string;
}

export interface MergedSettingsSection extends SkillSettingsSection {
  skillName: string;
  sourcePath: string;
}

export interface MergedToolbarButton extends SkillToolbarButton {
  skillName: string;
}

export interface MergedChatCommand extends SkillChatCommand {
  skillName: string;
}

export interface MergedStatusBarItem extends SkillStatusBarItem {
  skillName: string;
}

interface SkillExtensionState {
  skills: Skill[];
  loading: boolean;

  extensions: Skill[];
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
  setSkillFrontend: (name: string, frontend: SkillFrontendExtension) => Promise<void>;
  getHandler: (name: string) => SkillHandler | undefined;
  refreshSkill: (skillName: string) => Promise<void>;
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

  for (const skill of skills) {
    // ── V2: 优先处理 capabilities（新架构） ──
    const capabilitiesV2 = skill.manifest?.capabilities;
    if (capabilitiesV2 && capabilitiesV2.length > 0) {
      // V2 权限前置校验
      const permsV2 = skill.manifest?.permissionsV2;
      const required = extractRequiredCommands(capabilitiesV2);
      const permResult = validateSkillPermissionsAtLoad(permsV2, required);
      if (!permResult.valid) {
        console.warn(
          `[SkillExtension] Skill "${skill.name}" 权限校验失败:`,
          permResult.violations,
        );
        // 部分加载：只跳过未授权的 capabilities
      }

      for (const cap of capabilitiesV2) {
        mergeV2Capability(cap, skill, {
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
      continue; // V2 的 skill 跳过 V1 处理
    }

    // ── V1: 回退到 frontend 扩展（旧架构，保持向后兼容） ──
    if (!skill.frontend) { continue; }
    const f = skill.frontend;

    for (const nav of f.navigation) {
      navItems.push({ ...nav, skillName: skill.name });
    }
    for (const page of f.pages) {
      pages.push({ ...page, skillName: skill.name, sourcePath: skill.sourcePath });
    }
    for (const cmd of f.commands) {
      commands.push({ ...cmd, skillName: skill.name });
    }
    for (const panel of f.panels) {
      panels.push({ ...panel, skillName: skill.name, sourcePath: skill.sourcePath });
    }
    for (const section of f.settingsSections) {
      settingsSections.push({ ...section, skillName: skill.name, sourcePath: skill.sourcePath });
    }
    for (const btn of f.toolbar) {
      toolbarButtons.push({ ...btn, skillName: skill.name });
    }
    for (const cc of f.chatCommand) {
      chatCommands.push({ ...cc, skillName: skill.name });
    }
    for (const sb of f.statusBar) {
      statusBarItems.push({ ...sb, skillName: skill.name });
    }

    // V1 handlers
    if (skill.manifest?.handlers) {
      for (const [hName, hDef] of Object.entries(skill.manifest.handlers)) {
        handlers[`${skill.name}:${hName}`] = hDef;
        handlers[hName] = hDef;
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

/**
 * 将单个 V2 capability 合并到对应的扩展列表。
 * V2 → V1 格式转换，保持下游组件透明。
 */
function mergeV2Capability(
  cap: SkillCapabilityV2,
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
        id: cap.id,
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
        id: cap.id,
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
        id: cap.id,
        label: cap.title,
        icon: cap.icon,
        pageId: cap.pageId,
        position: cap.position ?? 0,
        skillName: skill.name,
      });
      break;
    case "toolbar":
      target.toolbarButtons.push({
        id: cap.id,
        icon: cap.icon,
        tooltip: cap.tooltip || cap.title || "",
        position: cap.position,
        priority: cap.priority ?? 10,
        onClick: cap.onClick,
        menu: cap.menu,
        skillName: skill.name,
      });
      break;
    case "chatCommand":
      target.chatCommands.push({
        name: cap.commandName,
        description: cap.description,
        icon: cap.icon,
        mode: cap.mode,
        actions: cap.actions,
        skillName: skill.name,
      });
      break;
    case "statusBar":
      target.statusBarItems.push({
        id: cap.id,
        alignment: cap.alignment,
        priority: cap.priority ?? 10,
        text: cap.text,
        icon: cap.icon,
        dynamicText: cap.dynamicText,
        onClick: cap.onClick,
        skillName: skill.name,
      });
      break;
    case "settings":
      target.settingsSections.push({
        id: cap.id,
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
      console.warn(`[SkillExtension] Unknown V2 capability type: "${(cap as any).type}"`);
  }
}

export const useSkillExtensionStore = create<SkillExtensionState>((set, get) => ({
  skills: [],
  loading: false,
  extensions: [],
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
      const extensions = skills.filter((s) => s.frontend);
      const merged = mergeExtensions(skills);
      set({ skills, extensions, ...merged, loading: false });
    } catch (e) {
      console.error("Failed to fetch skill extensions:", e);
      set({ loading: false });
    }
  },

  setSkillFrontend: async (name: string, frontend: SkillFrontendExtension) => {
    try {
      await invoke("skill_set_frontend", { name, frontend });
      await get().fetchSkills();
    } catch (e) {
      console.error("Failed to set skill frontend:", e);
    }
  },

  getHandler: (name: string) => get().handlers[name],

  refreshSkill: async (_skillName: string) => {
    const skills = await invoke<Skill[]>("list_skills");
    const merged = mergeExtensions(skills);
    const extensions = skills.filter((s) => s.frontend);
    set({ skills, extensions, ...merged });
  },
}));

// 注册热重载监听（模块加载时执行一次）
let _hotReloadRegistered = false;
export function ensureHotReloadRegistered() {
  if (_hotReloadRegistered) { return; }
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
  if (_pollingTimer) { return; }
  // 仅在开发模式启用
  if (!import.meta.env.DEV) { return; }

  let lastHash = "";
  _pollingTimer = setInterval(async () => {
    try {
      const { invoke } = await import("@/lib/invoke");
      const skills = await invoke<Array<{ name: string; enabled: boolean }>>("list_skills");
      const currentHash = JSON.stringify(skills.map((s) => `${s.name}:${s.enabled}`).sort());
      if (currentHash !== lastHash && lastHash !== "") {
        console.log("[SkillHotReload] Skill list changed, refreshing...");
        useSkillExtensionStore.getState().fetchSkills();
        await import("@/stores").then((s) => s.useSkillStore.getState().loadSkills());
      }
      lastHash = currentHash;
    } catch {
      // 浏览器模式下 list_skills 可能不存在，静默忽略
    }
  }, 5000);
}
