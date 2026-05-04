import { BUILTIN_AGENT_PROFILES } from "@/data/agentProfilePresets";
import { invoke } from "@/lib/invoke";
import type { AgentProfile, CreateAgentProfileInput, UpdateAgentProfileInput } from "@/types/agentProfile";
import { create } from "zustand";

interface AgentProfileState {
  builtinProfiles: AgentProfile[];
  customProfiles: AgentProfile[];
  agencyProfiles: AgentProfile[];
  agencyLoaded: boolean;

  getAllProfiles(): AgentProfile[];
  getProfileById(id: string): AgentProfile | undefined;
  getSystemPrompt(id: string): string | undefined;

  loadAgencyProfiles(): Promise<void>;
  importFromAgency(): Promise<{ count: number; errors: string[] }>;

  createCustomProfile(
    input: CreateAgentProfileInput,
  ): Promise<AgentProfile>;
  updateCustomProfile(
    id: string,
    input: UpdateAgentProfileInput,
  ): Promise<AgentProfile>;
  deleteCustomProfile(id: string): Promise<void>;
}

export const useAgentProfileStore = create<AgentProfileState>(
  (set, get) => ({
    builtinProfiles: BUILTIN_AGENT_PROFILES,
    customProfiles: [],
    agencyProfiles: [],
    agencyLoaded: false,

    getAllProfiles(): AgentProfile[] {
      const { builtinProfiles, agencyProfiles, customProfiles } = get();
      // 内置优先，然后是 agency，最后是自定义
      return [...builtinProfiles, ...agencyProfiles, ...customProfiles];
    },

    getProfileById(id: string): AgentProfile | undefined {
      return get().getAllProfiles().find((p) => p.id === id);
    },

    getSystemPrompt(id: string): string | undefined {
      const profile = get().getProfileById(id);
      return profile?.systemPrompt || undefined;
    },

    async loadAgencyProfiles(): Promise<void> {
      try {
        const rows: AgentProfile[] = await invoke(
          "list_agent_profiles",
          { source: "agency" },
        );
        set({ agencyProfiles: rows, agencyLoaded: true });
      } catch {
        set({ agencyLoaded: true });
      }
    },

    async importFromAgency(): Promise<{
      count: number;
      errors: string[];
    }> {
      const result: { count: number; errors: string[] } = await invoke(
        "import_agent_profiles_from_agency",
      );
      await get().loadAgencyProfiles();
      return result;
    },

    async createCustomProfile(
      input: CreateAgentProfileInput,
    ): Promise<AgentProfile> {
      const profile: AgentProfile = await invoke(
        "create_agent_profile",
        { input },
      );
      set((s) => ({
        customProfiles: [...s.customProfiles, profile],
      }));
      return profile;
    },

    async updateCustomProfile(
      id: string,
      input: UpdateAgentProfileInput,
    ): Promise<AgentProfile> {
      const profile: AgentProfile = await invoke(
        "update_agent_profile",
        { id, input },
      );
      set((s) => ({
        customProfiles: s.customProfiles.map((p) => p.id === id ? profile : p),
      }));
      return profile;
    },

    async deleteCustomProfile(id: string): Promise<void> {
      await invoke("delete_agent_profile", { id });
      set((s) => ({
        customProfiles: s.customProfiles.filter((p) => p.id !== id),
      }));
    },
  }),
);
