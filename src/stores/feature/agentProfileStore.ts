import { invoke } from "@/lib/invoke";
import type { AgentProfile, CreateAgentProfileInput, UpdateAgentProfileInput } from "@/types/agentProfile";
import { create } from "zustand";

interface AgentProfileState {
  profiles: AgentProfile[];
  loaded: boolean;

  loadProfiles(): Promise<void>;
  getAllProfiles(): AgentProfile[];
  getProfileById(id: string): AgentProfile | undefined;
  getSystemPrompt(id: string): string | undefined;

  importFromAgency(): Promise<{ count: number; errors: string[] }>;

  createCustomProfile(input: CreateAgentProfileInput): Promise<AgentProfile>;
  updateCustomProfile(id: string, input: UpdateAgentProfileInput): Promise<AgentProfile>;
  deleteCustomProfile(id: string): Promise<void>;
}

export const useAgentProfileStore = create<AgentProfileState>((set, get) => ({
  profiles: [],
  loaded: false,

  async loadProfiles(): Promise<void> {
    try {
      const rows: AgentProfile[] = await invoke("list_agent_profiles");
      set({ profiles: rows, loaded: true });
    } catch {
      set({ loaded: true });
    }
  },

  getAllProfiles(): AgentProfile[] {
    return get().profiles;
  },

  getProfileById(id: string): AgentProfile | undefined {
    return get().profiles.find((p) => p.id === id);
  },

  getSystemPrompt(id: string): string | undefined {
    return get().getProfileById(id)?.systemPrompt || undefined;
  },

  async importFromAgency(): Promise<{ count: number; errors: string[] }> {
    const result: { count: number; errors: string[] } = await invoke(
      "import_agent_profiles_from_agency",
    );
    await get().loadProfiles();
    return result;
  },

  async createCustomProfile(input: CreateAgentProfileInput): Promise<AgentProfile> {
    const profile: AgentProfile = await invoke("create_agent_profile", { input });
    set((s) => ({ profiles: [...s.profiles, profile] }));
    return profile;
  },

  async updateCustomProfile(id: string, input: UpdateAgentProfileInput): Promise<AgentProfile> {
    const profile: AgentProfile = await invoke("update_agent_profile", { id, input });
    set((s) => ({
      profiles: s.profiles.map((p) => (p.id === id ? profile : p)),
    }));
    return profile;
  },

  async deleteCustomProfile(id: string): Promise<void> {
    await invoke("delete_agent_profile", { id });
    set((s) => ({ profiles: s.profiles.filter((p) => p.id !== id) }));
  },
}));
