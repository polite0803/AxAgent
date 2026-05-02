import { invoke } from "@/lib/invoke";
import type {
  Skill,
  SkillFrontendExtension,
  SkillNavItem,
  SkillPage,
  SkillUICommand,
  SkillUIPanel,
  SkillSettingsSection,
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

interface SkillExtensionState {
  skills: Skill[];
  loading: boolean;

  // 只有 frontend 非空的技能
  extensions: Skill[];
  // 派生合并数据
  navItems: MergedNavItem[];
  pages: MergedPage[];
  commands: MergedCommand[];
  panels: MergedPanel[];
  settingsSections: MergedSettingsSection[];

  fetchSkills: () => Promise<void>;
  setSkillFrontend: (name: string, frontend: SkillFrontendExtension) => Promise<void>;
}

function mergeExtensions(skills: Skill[]): {
  navItems: MergedNavItem[];
  pages: MergedPage[];
  commands: MergedCommand[];
  panels: MergedPanel[];
  settingsSections: MergedSettingsSection[];
} {
  const navItems: MergedNavItem[] = [];
  const pages: MergedPage[] = [];
  const commands: MergedCommand[] = [];
  const panels: MergedPanel[] = [];
  const settingsSections: MergedSettingsSection[] = [];

  for (const skill of skills) {
    if (!skill.frontend) continue;

    for (const nav of skill.frontend.navigation) {
      navItems.push({ ...nav, skillName: skill.name });
    }
    for (const page of skill.frontend.pages) {
      pages.push({ ...page, skillName: skill.name, sourcePath: skill.sourcePath });
    }
    for (const cmd of skill.frontend.commands) {
      commands.push({ ...cmd, skillName: skill.name });
    }
    for (const panel of skill.frontend.panels) {
      panels.push({ ...panel, skillName: skill.name, sourcePath: skill.sourcePath });
    }
    for (const section of skill.frontend.settingsSections) {
      settingsSections.push({ ...section, skillName: skill.name, sourcePath: skill.sourcePath });
    }
  }

  return { navItems, pages, commands, panels, settingsSections };
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

  fetchSkills: async () => {
    set({ loading: true });
    try {
      const skills = await invoke<Skill[]>("list_skills");
      const extensions = skills.filter((s) => s.frontend);
      const merged = mergeExtensions(skills);
      set({
        skills,
        extensions,
        ...merged,
        loading: false,
      });
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
}));
