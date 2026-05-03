import { invoke } from "@/lib/invoke";
import type {
  Skill,
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

    // 合并 handlers（后加载的 skill 同名 handler 会被覆盖）
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

  import("@/lib/invoke").then(({ listen }) => {
    listen<{ skillName: string }>("skill:file-changed", (event) => {
      const { skillName } = event.payload;
      useSkillExtensionStore.getState().refreshSkill(skillName);
    }).catch(() => {
      // 非 Tauri 环境静默忽略
    });
  });
}
