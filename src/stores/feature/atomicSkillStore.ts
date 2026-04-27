import { create } from "zustand";
import { invoke } from "../../lib/invoke";
import type {
  AtomicSkill,
  AtomicSkillFilter,
  CreateAtomicSkillParams,
  SkillReference,
  UpdateAtomicSkillParams,
} from "../../types";

interface AtomicSkillState {
  skills: AtomicSkill[];
  selectedSkill: AtomicSkill | null;
  loading: boolean;
  filter: AtomicSkillFilter;

  loadSkills: (filter?: AtomicSkillFilter) => Promise<void>;
  getSkill: (id: string) => Promise<void>;
  createSkill: (params: CreateAtomicSkillParams) => Promise<string>;
  updateSkill: (id: string, params: UpdateAtomicSkillParams) => Promise<boolean>;
  deleteSkill: (id: string) => Promise<boolean>;
  toggleSkill: (id: string, enabled: boolean) => Promise<void>;
  checkSemanticUniqueness: (
    entry_type: string,
    entry_ref: string,
    input_schema?: Record<string, unknown>,
    output_schema?: Record<string, unknown>,
  ) => Promise<AtomicSkill | null>;
  getReferences: (skillId: string) => Promise<SkillReference[]>;
  setFilter: (filter: AtomicSkillFilter) => void;
}

export const useAtomicSkillStore = create<AtomicSkillState>((set, get) => ({
  skills: [],
  selectedSkill: null,
  loading: false,
  filter: {},

  loadSkills: async (filter?: AtomicSkillFilter) => {
    set({ loading: true });
    try {
      const f = filter ?? get().filter;
      const skills = await invoke<AtomicSkill[]>("list_atomic_skills", { filter: f });
      set({ skills, filter: f });
    } finally {
      set({ loading: false });
    }
  },

  getSkill: async (id: string) => {
    const skill = await invoke<AtomicSkill | null>("get_atomic_skill", { id });
    set({ selectedSkill: skill });
  },

  createSkill: async (params: CreateAtomicSkillParams) => {
    const id = await invoke<string>("create_atomic_skill", { params });
    await get().loadSkills();
    return id;
  },

  updateSkill: async (id: string, params: UpdateAtomicSkillParams) => {
    const success = await invoke<boolean>("update_atomic_skill", { id, params });
    if (success) { await get().loadSkills(); }
    return success;
  },

  deleteSkill: async (id: string) => {
    const success = await invoke<boolean>("delete_atomic_skill", { id });
    if (success) { await get().loadSkills(); }
    return success;
  },

  toggleSkill: async (id: string, enabled: boolean) => {
    await invoke<boolean>("toggle_atomic_skill", { id, enabled });
    await get().loadSkills();
  },

  checkSemanticUniqueness: async (
    entry_type: string,
    entry_ref: string,
    input_schema?: Record<string, unknown>,
    output_schema?: Record<string, unknown>,
  ) => {
    return invoke<AtomicSkill | null>("check_semantic_uniqueness", {
      entry_type,
      entry_ref,
      input_schema,
      output_schema,
    });
  },

  getReferences: async (skillId: string) => {
    return invoke<SkillReference[]>("get_skill_references", { skill_id: skillId });
  },

  setFilter: (filter: AtomicSkillFilter) => {
    set({ filter });
  },
}));
