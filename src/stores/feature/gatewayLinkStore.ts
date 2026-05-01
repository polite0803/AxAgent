import { invoke } from "@/lib/invoke";
import type {
  CreateGatewayLinkInput,
  GatewayLink,
  GatewayLinkActivity,
  GatewayLinkModelSync,
  GatewayLinkPolicy,
  GatewayLinkSkillSync,
} from "@/types";
import { create } from "zustand";

interface GatewayLinkState {
  links: GatewayLink[];
  selectedLinkId: string | null;
  modelSyncs: GatewayLinkModelSync[];
  skillSyncs: GatewayLinkSkillSync[];
  policy: GatewayLinkPolicy | null;
  activities: GatewayLinkActivity[];
  loading: boolean;
  error: string | null;

  fetchLinks: () => Promise<void>;
  selectLink: (id: string | null) => void;
  createLink: (input: CreateGatewayLinkInput) => Promise<GatewayLink>;
  deleteLink: (id: string) => Promise<void>;
  toggleLink: (id: string, enabled: boolean) => Promise<void>;
  connectLink: (id: string) => Promise<void>;
  disconnectLink: (id: string) => Promise<void>;
  fetchModelSyncs: (linkId: string) => Promise<void>;
  pushModels: (linkId: string, modelIds: string[]) => Promise<void>;
  syncAllModels: (linkId: string) => Promise<void>;
  fetchSkillSyncs: (linkId: string) => Promise<void>;
  pushSkills: (linkId: string, skillNames: string[]) => Promise<void>;
  syncAllSkills: (linkId: string) => Promise<void>;
  fetchPolicy: (linkId: string) => Promise<void>;
  savePolicy: (linkId: string, policy: Partial<GatewayLinkPolicy>) => Promise<void>;
  updateSyncSettings: (linkId: string, autoSyncModels: boolean, autoSyncSkills: boolean) => Promise<void>;
  fetchActivities: (linkId: string) => Promise<void>;
  createGatewayConversation: (linkId: string) => Promise<string>;
}

export const useGatewayLinkStore = create<GatewayLinkState>((set, get) => ({
  links: [],
  selectedLinkId: null,
  modelSyncs: [],
  skillSyncs: [],
  policy: null,
  activities: [],
  loading: false,
  error: null,

  fetchLinks: async () => {
    set({ loading: true });
    try {
      const links = await invoke<GatewayLink[]>("list_gateway_links");
      set({ links, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  selectLink: (id) => {
    set({ selectedLinkId: id });
    if (id) {
      const { fetchModelSyncs, fetchSkillSyncs, fetchPolicy, fetchActivities } = get();
      void fetchModelSyncs(id);
      void fetchSkillSyncs(id);
      void fetchPolicy(id);
      void fetchActivities(id);
    } else {
      set({ modelSyncs: [], skillSyncs: [], policy: null, activities: [] });
    }
  },

  createLink: async (input) => {
    try {
      const link = await invoke<GatewayLink>("create_gateway_link", { input });
      set((s) => ({ links: [...s.links, link], error: null }));
      return link;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deleteLink: async (id) => {
    try {
      await invoke("delete_gateway_link", { id });
      set((s) => ({
        links: s.links.filter((l) => l.id !== id),
        selectedLinkId: s.selectedLinkId === id ? null : s.selectedLinkId,
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  toggleLink: async (id, enabled) => {
    try {
      await invoke("toggle_gateway_link", { id, enabled });
      set((s) => ({
        links: s.links.map((l) => (l.id === id ? { ...l, enabled } : l)),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  connectLink: async (id) => {
    try {
      const updatedLink = await invoke<GatewayLink>("connect_gateway_link", { id });
      set((s) => ({
        links: s.links.map((l) => (l.id === id ? updatedLink : l)),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  disconnectLink: async (id) => {
    try {
      const updatedLink = await invoke<GatewayLink>("disconnect_gateway_link", { id });
      set((s) => ({
        links: s.links.map((l) => (l.id === id ? updatedLink : l)),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  fetchModelSyncs: async (linkId) => {
    try {
      const modelSyncs = await invoke<GatewayLinkModelSync[]>("get_gateway_link_model_syncs", { link_id: linkId });
      set({ modelSyncs, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  pushModels: async (linkId, modelIds) => {
    try {
      await invoke("push_gateway_link_models", { link_id: linkId, model_ids: modelIds });
      await get().fetchModelSyncs(linkId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  syncAllModels: async (linkId) => {
    try {
      await invoke("sync_all_gateway_link_models", { link_id: linkId });
      await get().fetchModelSyncs(linkId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  fetchSkillSyncs: async (linkId) => {
    try {
      const skillSyncs = await invoke<GatewayLinkSkillSync[]>("get_gateway_link_skill_syncs", { link_id: linkId });
      set({ skillSyncs, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  pushSkills: async (linkId, skillNames) => {
    try {
      await invoke("push_gateway_link_skills", { link_id: linkId, skill_names: skillNames });
      await get().fetchSkillSyncs(linkId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  syncAllSkills: async (linkId) => {
    try {
      await invoke("sync_all_gateway_link_skills", { link_id: linkId });
      await get().fetchSkillSyncs(linkId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  fetchPolicy: async (linkId) => {
    try {
      const policy = await invoke<GatewayLinkPolicy | null>("get_gateway_link_policy", { link_id: linkId });
      set({ policy, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  savePolicy: async (linkId, policyUpdate) => {
    try {
      const policy = await invoke<GatewayLinkPolicy>("save_gateway_link_policy", { link_id: linkId, input: policyUpdate });
      set({ policy, error: null });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  updateSyncSettings: async (linkId, autoSyncModels, autoSyncSkills) => {
    try {
      const updatedLink = await invoke<GatewayLink>("update_gateway_link_sync_settings", {
        id: linkId,
        auto_sync_models: autoSyncModels,
        auto_sync_skills: autoSyncSkills,
      });
      set((s) => ({
        links: s.links.map((l) => (l.id === linkId ? updatedLink : l)),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  fetchActivities: async (linkId) => {
    try {
      const activities = await invoke<GatewayLinkActivity[]>("get_gateway_link_activities", { link_id: linkId });
      set({ activities, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  createGatewayConversation: async (linkId) => {
    try {
      const conversationId = await invoke<string>("create_gateway_conversation", { link_id: linkId });
      return conversationId;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },
}));
