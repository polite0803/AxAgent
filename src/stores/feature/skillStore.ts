import { invoke } from "@/lib/invoke";
import type { MarketplaceSkill, Skill, SkillDetail, SkillProposal, SkillUpdateInfo } from "@/types";
import { create } from "zustand";

interface SkillState {
  skills: Skill[];
  marketplaceSkills: MarketplaceSkill[];
  skillProposals: SkillProposal[];
  loading: boolean;
  marketplaceLoading: boolean;
  selectedSkill: SkillDetail | null;
  marketplacePage: number;
  marketplaceHasMore: boolean;
  marketplaceQuery: string;
  marketplaceSource: string;
  marketplaceSort: string;

  loadSkills: () => Promise<void>;
  getSkill: (name: string) => Promise<void>;
  toggleSkill: (name: string, enabled: boolean) => Promise<void>;
  installSkill: (source: string, target?: string, scenarios?: string[]) => Promise<string>;
  uninstallSkill: (name: string) => Promise<void>;
  uninstallSkillGroup: (group: string) => Promise<void>;
  openSkillsDir: () => Promise<void>;
  openSkillDir: (path: string) => Promise<void>;
  searchMarketplace: (query: string, source?: string, sort?: string, page?: number) => Promise<void>;
  loadMoreMarketplace: () => Promise<void>;
  checkUpdates: () => Promise<SkillUpdateInfo[]>;
  clearSelectedSkill: () => void;
  // P1: Self-evolution skill management
  createSkill: (name: string, description: string, content: string) => Promise<string>;
  patchSkill: (name: string, content: string) => Promise<string>;
  editSkill: (name: string, content: string) => Promise<string>;
  loadSkillProposals: () => Promise<SkillProposal[]>;
  createSkillFromProposal: (name: string, description: string, content: string) => Promise<string>;
}

export const useSkillStore = create<SkillState>((set, get) => ({
  skills: [],
  skillProposals: [],
  marketplaceSkills: [],
  loading: false,
  marketplaceLoading: false,
  selectedSkill: null,
  marketplacePage: 1,
  marketplaceHasMore: true,
  marketplaceQuery: "",
  marketplaceSource: "skillhub",
  marketplaceSort: "popular",

  loadSkills: async () => {
    set({ loading: true });
    try {
      const skills = await invoke<Skill[]>("list_skills");
      set({ skills, loading: false });
    } catch (e) {
      console.error("Failed to load skills:", e);
      set({ loading: false });
    }
  },

  getSkill: async (name: string) => {
    try {
      const detail = await invoke<SkillDetail>("get_skill", { name });
      set({ selectedSkill: detail });
    } catch (e) {
      console.error("Failed to get skill:", e);
    }
  },

  toggleSkill: async (name: string, enabled: boolean) => {
    set({
      skills: get().skills.map(s => s.name === name ? { ...s, enabled } : s),
    });
    try {
      await invoke("toggle_skill", { name, enabled });
      // 生命周期钩子
      const { triggerOnEnable, triggerOnDisable } = await import("@/lib/skillLifecycle");
      if (enabled) {
        triggerOnEnable(name).catch((e) => console.error("onEnable failed:", e));
      } else {
        triggerOnDisable(name).catch((e) => console.error("onDisable failed:", e));
      }
    } catch (e) {
      console.error("Failed to toggle skill:", e);
      set({
        skills: get().skills.map(s => s.name === name ? { ...s, enabled: !enabled } : s),
      });
    }
  },

  installSkill: async (source: string, target?: string, scenarios?: string[]) => {
    const name = await invoke<string>("install_skill", {
      source,
      target: target ?? null,
      scenarios: scenarios ?? null,
    });
    await get().loadSkills();
    set({
      marketplaceSkills: get().marketplaceSkills.map(s => s.repo === source ? { ...s, installed: true } : s),
    });
    // 生命周期钩子
    const { triggerOnInstall } = await import("@/lib/skillLifecycle");
    triggerOnInstall(name).catch((e) => console.error("onInstall failed:", e));
    return name;
  },

  uninstallSkill: async (name: string) => {
    // 卸载前先触发钩子
    const { triggerOnUninstall } = await import("@/lib/skillLifecycle");
    triggerOnUninstall(name).catch((e) => console.error("onUninstall failed:", e));
    await invoke("uninstall_skill", { name });
    set({ skills: get().skills.filter(s => s.name !== name) });
  },

  uninstallSkillGroup: async (group: string) => {
    await invoke("uninstall_skill_group", { group });
    set({ skills: get().skills.filter(s => s.group !== group) });
  },

  openSkillsDir: async () => {
    await invoke("open_skills_dir");
  },

  openSkillDir: async (path: string) => {
    await invoke("open_skill_dir", { path });
  },

  searchMarketplace: async (query: string, source?: string, sort?: string, page: number = 1) => {
    const currentSource = source ?? "skillhub";
    const currentSort = sort ?? "popular";

    if (page === 1) {
      set({
        marketplaceLoading: true,
        marketplaceSkills: [],
        marketplacePage: 1,
        marketplaceHasMore: true,
        marketplaceQuery: query,
        marketplaceSource: currentSource,
        marketplaceSort: currentSort,
      });
    } else {
      set({ marketplaceLoading: true });
    }

    try {
      const results = await invoke<MarketplaceSkill[]>("search_marketplace", {
        query,
        source: currentSource,
        sort: currentSort,
        page,
        per_page: 20,
      });
      set((state) => ({
        marketplaceSkills: page === 1 ? results : [...state.marketplaceSkills, ...results],
        marketplaceLoading: false,
        marketplacePage: page,
        marketplaceHasMore: results.length >= 20,
      }));
    } catch (e) {
      console.error("Failed to search marketplace:", e);
      set({ marketplaceLoading: false });
    }
  },

  loadMoreMarketplace: async () => {
    const { marketplacePage, marketplaceHasMore, marketplaceLoading } = get();
    if (marketplaceLoading || !marketplaceHasMore) { return; }
    await get().searchMarketplace(
      get().marketplaceQuery,
      get().marketplaceSource,
      get().marketplaceSort,
      marketplacePage + 1,
    );
  },

  checkUpdates: async () => {
    try {
      const updates = await invoke<SkillUpdateInfo[]>("check_skill_updates");
      return updates;
    } catch (e) {
      console.error("Failed to check updates:", e);
      return [];
    }
  },

  clearSelectedSkill: () => set({ selectedSkill: null }),

  // P1: Self-evolution skill management
  createSkill: async (name: string, description: string, content: string) => {
    const result = await invoke<string>("skill_create", { name, description, content });
    await get().loadSkills();
    return result;
  },

  patchSkill: async (name: string, content: string) => {
    const result = await invoke<string>("skill_patch", { name, content });
    await get().getSkill(name);
    return result;
  },

  editSkill: async (name: string, content: string) => {
    const result = await invoke<string>("skill_edit", { name, content });
    await get().getSkill(name);
    return result;
  },

  // P1: Skill proposals from trajectory analysis
  loadSkillProposals: async () => {
    const proposals = await invoke<SkillProposal[]>("get_skill_proposals");
    set({ skillProposals: proposals });
    return proposals;
  },

  createSkillFromProposal: async (name: string, description: string, content: string) => {
    const result = await invoke<string>("create_skill_from_proposal", { name, description, content });
    await get().loadSkills();
    set((s) => ({
      skillProposals: s.skillProposals.filter((p) => p.suggested_name !== name),
    }));
    return result;
  },
}));
