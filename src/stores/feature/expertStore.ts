import { BUILTIN_EXPERT_PRESETS } from "@/data/expertPresets";
import { invoke } from "@/lib/invoke";
import type { AgentBehaviorMode, AgentProfile, ExpertCategory } from "@/types";
import { EXPERT_CATEGORY_LABELS } from "@/types";
import { message } from "antd";
import { create } from "zustand";

const CUSTOM_ROLES_KEY = "axagent_custom_expert_roles";

function loadCustomRoles(): AgentProfile[] {
  try {
    const stored = localStorage.getItem(CUSTOM_ROLES_KEY);
    if (!stored) { return []; }
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) { return []; }
    // 迁移旧格式 (ExpertRole 有 displayName) → 新格式 (AgentProfile 有 name)
    return parsed.map((item: Record<string, unknown>) => {
      if (item.displayName && !item.name) {
        return {
          ...item,
          name: item.displayName,
          agentRole: (item as Record<string, unknown>).agentRole ?? null,
          sortOrder: (item as Record<string, unknown>).sortOrder ?? 0,
          isEnabled: (item as Record<string, unknown>).isEnabled ?? true,
          createdAt: (item as Record<string, unknown>).createdAt ?? 0,
          updatedAt: (item as Record<string, unknown>).updatedAt ?? 0,
        };
      }
      return item;
    }) as unknown as AgentProfile[];
  } catch {
    return [];
  }
}

function saveCustomRoles(roles: AgentProfile[]): void {
  localStorage.setItem(CUSTOM_ROLES_KEY, JSON.stringify(roles));
}

interface AgencyExpertRow {
  id: string;
  name: string;
  description: string | null;
  category: string;
  system_prompt: string;
  color: string | null;
  source_dir: string;
  is_enabled: boolean;
  recommended_workflows: string[] | null;
  recommended_tools: string[] | null;
}

function agencyRowToRole(row: AgencyExpertRow): AgentProfile {
  const CATEGORY_ICONS: Record<string, string> = {
    development: "💻",
    security: "🔒",
    data: "📊",
    devops: "🚀",
    design: "🎨",
    writing: "📝",
    business: "💼",
    general: "🤖",
  };

  const tags = [row.source_dir, row.category];
  if (row.color) { tags.push(row.color); }

  const PERMISSION_BY_CATEGORY: Record<string, AgentBehaviorMode> = {
    security: "default",
    development: "accept_edits",
    devops: "accept_edits",
    data: "default",
    business: "default",
  };

  return {
    id: row.id,
    name: row.name,
    description: row.description,
    category: row.category as ExpertCategory,
    icon: CATEGORY_ICONS[row.category] ?? "🤖",
    systemPrompt: row.system_prompt,
    source: "agency",
    agentRole: null,
    tags,
    recommendPermissionMode: PERMISSION_BY_CATEGORY[row.category] ?? "default",
    recommendedWorkflows: row.recommended_workflows ?? undefined,
    recommendedTools: row.recommended_tools ?? undefined,
    sortOrder: 0,
    isEnabled: row.is_enabled,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  };
}

interface ExpertState {
  builtinRoles: AgentProfile[];
  customRoles: AgentProfile[];
  agencyRoles: AgentProfile[];
  agencyLoaded: boolean;
  agencyLoading: boolean;

  recentSwitch: { conversationId: string; roleId: string; timestamp: number } | null;

  getAllRoles: () => AgentProfile[];
  getRolesByCategory: () => Record<string, AgentProfile[]>;
  getRoleById: (id: string) => AgentProfile | undefined;
  getSystemPrompt: (roleId: string | null) => string | null;
  getCategoryLabel: (roleId: string | null) => string;

  recordSwitch: (conversationId: string, roleId: string) => void;
  consumeSwitch: (conversationId: string) => { roleId: string } | null;

  importAgencyExperts: (
    path: string,
  ) => Promise<{ count: number; workflows_created?: number; tools_matched?: number; errors: string[] }>;
  loadAgencyRoles: () => Promise<void>;
  clearAgencyExperts: () => Promise<void>;
  deleteAgencyExpert: (id: string) => Promise<void>;
  updateAgencyExpert: (
    id: string,
    fields: { name?: string; description?: string; category?: string; system_prompt?: string; is_enabled?: boolean },
  ) => Promise<void>;
  exportAgencyExperts: () => Promise<string>;
  extractStructure: (text: string) => Promise<AgentProfile | null>;

  addCustomRole: (role: AgentProfile) => void;
  updateCustomRole: (role: AgentProfile) => void;
  removeCustomRole: (id: string) => void;
  exportCustomRoles: () => string;
  importCustomRoles: (json: string) => { count: number; errors: string[] };
}

export const useExpertStore = create<ExpertState>((set, get) => ({
  builtinRoles: BUILTIN_EXPERT_PRESETS,
  customRoles: loadCustomRoles(),
  agencyRoles: [],
  agencyLoaded: false,
  agencyLoading: false,
  recentSwitch: null,

  getAllRoles: () => {
    const general = get().builtinRoles.find((r) => r.id === "general-assistant");
    const otherBuiltins = get().builtinRoles.filter((r) => r.id !== "general-assistant");
    const result: AgentProfile[] = [];
    if (general) { result.push(general); }
    result.push(...otherBuiltins, ...get().agencyRoles, ...get().customRoles);
    return result;
  },

  getRolesByCategory: () => {
    const grouped: Record<string, AgentProfile[]> = {};
    for (const role of get().getAllRoles()) {
      if (!grouped[role.category]) {
        grouped[role.category] = [];
      }
      grouped[role.category].push(role);
    }
    return grouped;
  },

  getRoleById: (id: string) => {
    return get().getAllRoles().find((r) => r.id === id);
  },

  getSystemPrompt: (roleId: string | null) => {
    if (!roleId) { return null; }
    const role = get().getRoleById(roleId);
    return role?.systemPrompt || null;
  },

  getCategoryLabel: (roleId: string | null) => {
    if (!roleId) { return "通用"; }
    const role = get().getRoleById(roleId);
    if (!role) { return "通用"; }
    return EXPERT_CATEGORY_LABELS[role.category] || role.category;
  },

  recordSwitch: (conversationId, roleId) => {
    set({ recentSwitch: { conversationId, roleId, timestamp: Date.now() } });
  },

  consumeSwitch: (conversationId) => {
    const sw = get().recentSwitch;
    if (!sw || sw.conversationId !== conversationId) { return null; }
    set({ recentSwitch: null });
    return { roleId: sw.roleId };
  },

  importAgencyExperts: async (path: string) => {
    set({ agencyLoading: true });
    try {
      const result = await invoke<
        { count: number; workflows_created: number; tools_matched: number; errors: string[] }
      >("import_agency_experts", {
        request: { path },
      });
      await get().loadAgencyRoles();
      return result;
    } finally {
      set({ agencyLoading: false });
    }
  },

  loadAgencyRoles: async () => {
    try {
      const rows = await invoke<AgencyExpertRow[]>("list_agency_experts");
      if (!rows || !Array.isArray(rows)) {
        set({ agencyLoaded: true });
        return;
      }
      const roles = rows.map(agencyRowToRole);
      set({ agencyRoles: roles, agencyLoaded: true });
    } catch (e) {
      console.error("[expertStore] loadAgencyRoles failed:", e);
      message.error(`加载外部专家失败: ${String(e)}`);
      set({ agencyLoaded: true });
    }
  },

  clearAgencyExperts: async () => {
    try {
      await invoke("clear_agency_experts");
      set({ agencyRoles: [], agencyLoaded: false });
    } catch (e) {
      console.error("[expertStore] clearAgencyExperts failed:", e);
      message.error(`清除外部专家失败: ${String(e)}`);
    }
  },

  deleteAgencyExpert: async (id: string) => {
    try {
      await invoke("delete_agency_expert", { request: { id } });
      const roles = get().agencyRoles.filter((r) => r.id !== id);
      set({ agencyRoles: roles });
    } catch (e) {
      console.error("[expertStore] deleteAgencyExpert failed:", e);
      message.error(`删除外部专家失败: ${String(e)}`);
    }
  },

  updateAgencyExpert: async (id: string, fields) => {
    try {
      await invoke("update_agency_expert", { request: { id, ...fields } });
      await get().loadAgencyRoles();
    } catch (e) {
      console.error("[expertStore] updateAgencyExpert failed:", e);
      message.error(`更新外部专家失败: ${String(e)}`);
    }
  },

  exportAgencyExperts: async () => {
    const json = await invoke<string>("export_agency_experts");
    return json;
  },

  extractStructure: async (expertId: string) => {
    try {
      const row = await invoke<AgencyExpertRow>("extract_expert_structure", { request: { expertId } });
      if (!row) { return null; }
      return agencyRowToRole(row);
    } catch (e) {
      console.error("[expertStore] extractStructure failed:", e);
      message.error(`提取专家结构失败: ${String(e)}`);
      return null;
    }
  },

  addCustomRole: (role) => {
    const updated = [...get().customRoles, role];
    saveCustomRoles(updated);
    set({ customRoles: updated });
  },

  updateCustomRole: (role) => {
    const existing = get().customRoles.find((r) => r.id === role.id);
    if (!existing) { return; }
    const updated = get().customRoles.map((r) => (r.id === role.id ? role : r));
    saveCustomRoles(updated);
    set({ customRoles: updated });
  },

  removeCustomRole: (id) => {
    const updated = get().customRoles.filter((r) => r.id !== id);
    saveCustomRoles(updated);
    set({ customRoles: updated });
  },

  exportCustomRoles: () => {
    const customRoles = get().customRoles;
    return JSON.stringify(customRoles, null, 2);
  },

  importCustomRoles: (json) => {
    const errors: string[] = [];
    try {
      const parsed = JSON.parse(json);
      if (!Array.isArray(parsed)) {
        return { count: 0, errors: ["JSON 格式错误：期望一个数组"] };
      }
      const validRoles: AgentProfile[] = [];
      for (const item of parsed) {
        const hasId = !!item.id;
        const hasName = !!(item.displayName || item.name);
        const hasCategory = !!item.category;
        if (hasId && hasName && hasCategory) {
          validRoles.push(item as AgentProfile);
        } else {
          errors.push(`跳过无效角色: ${JSON.stringify(item).slice(0, 50)}`);
        }
      }
      if (validRoles.length > 0) {
        const existingIds = new Set(get().customRoles.map((r) => r.id));
        const newRoles = validRoles.filter((r) => !existingIds.has(r.id));
        const updated = [...get().customRoles, ...newRoles];
        saveCustomRoles(updated);
        set({ customRoles: updated });
      }
      return { count: validRoles.length, errors };
    } catch (e) {
      return { count: 0, errors: [`JSON 解析失败: ${String(e)}`] };
    }
  },
}));
