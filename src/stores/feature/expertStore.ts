import { BUILTIN_EXPERT_PRESETS } from "@/data/expertPresets";
import { invoke } from "@/lib/invoke";
import type { ExpertRole } from "@/types/expert";
import { EXPERT_CATEGORY_LABELS } from "@/types/expert";
import { create } from "zustand";

const CUSTOM_ROLES_KEY = "axagent_custom_expert_roles";

function loadCustomRoles(): ExpertRole[] {
  try {
    const stored = localStorage.getItem(CUSTOM_ROLES_KEY);
    if (!stored) return [];
    const parsed = JSON.parse(stored);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function saveCustomRoles(roles: ExpertRole[]): void {
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

function agencyRowToRole(row: AgencyExpertRow): ExpertRole {
  const CATEGORY_ICONS: Record<string, string> = {
    development: "\uD83D\uDCBB",
    security: "\uD83D\uDD12",
    data: "\uD83D\uDCCA",
    devops: "\uD83D\uDE80",
    design: "\uD83C\uDFA8",
    writing: "\uD83D\uDCDD",
    business: "\uD83D\uDCBC",
    general: "\uD83E\uDD16",
  };

  const tags = [row.source_dir, row.category];
  if (row.color) tags.push(row.color);

  // Default permission mode based on category
  const PERMISSION_BY_CATEGORY: Record<string, ExpertRole["recommendPermissionMode"]> = {
    security: "default",
    development: "accept_edits",
    devops: "accept_edits",
    data: "default",
    business: "default",
  };

  return {
    id: row.id,
    displayName: row.name,
    description: row.description ?? "",
    category: row.category as ExpertRole["category"],
    icon: CATEGORY_ICONS[row.category] ?? "\uD83E\uDD16",
    systemPrompt: row.system_prompt,
    source: "agency",
    tags,
    recommendPermissionMode: PERMISSION_BY_CATEGORY[row.category] ?? "default",
    recommendedWorkflows: row.recommended_workflows ?? undefined,
    recommendedTools: row.recommended_tools ?? undefined,
  };
}

interface ExpertState {
  builtinRoles: ExpertRole[];
  customRoles: ExpertRole[];
  agencyRoles: ExpertRole[];
  agencyLoaded: boolean;
  agencyLoading: boolean;

  recentSwitch: { conversationId: string; roleId: string; timestamp: number } | null;

  getAllRoles: () => ExpertRole[];
  getRolesByCategory: () => Record<string, ExpertRole[]>;
  getRoleById: (id: string) => ExpertRole | undefined;
  getSystemPrompt: (roleId: string | null) => string | null;
  getCategoryLabel: (roleId: string | null) => string;

  recordSwitch: (conversationId: string, roleId: string) => void;
  consumeSwitch: (conversationId: string) => { roleId: string } | null;

  /** Import from agency-agents-zh repo */
  importAgencyExperts: (path: string) => Promise<{ count: number; workflows_created?: number; tools_matched?: number; errors: string[] }>;
  /** Load agency experts from DB */
  loadAgencyRoles: () => Promise<void>;
  /** Clear agency experts from DB */
  clearAgencyExperts: () => Promise<void>;
  /** Delete a single agency expert */
  deleteAgencyExpert: (id: string) => Promise<void>;
  /** Update an agency expert's fields */
  updateAgencyExpert: (id: string, fields: { name?: string; description?: string; category?: string; system_prompt?: string; is_enabled?: boolean }) => Promise<void>;
  /** Export all agency experts as JSON */
  exportAgencyExperts: () => Promise<string>;

  addCustomRole: (role: ExpertRole) => void;
  updateCustomRole: (role: ExpertRole) => void;
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
    const result: ExpertRole[] = [];
    if (general) result.push(general);
    result.push(...otherBuiltins, ...get().agencyRoles, ...get().customRoles);
    return result;
  },

  getRolesByCategory: () => {
    const grouped: Record<string, ExpertRole[]> = {};
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
    if (!roleId) return null;
    const role = get().getRoleById(roleId);
    return role?.systemPrompt || null;
  },

  getCategoryLabel: (roleId: string | null) => {
    if (!roleId) return "通用";
    const role = get().getRoleById(roleId);
    if (!role) return "通用";
    return EXPERT_CATEGORY_LABELS[role.category] || role.category;
  },

  recordSwitch: (conversationId, roleId) => {
    set({ recentSwitch: { conversationId, roleId, timestamp: Date.now() } });
  },

  consumeSwitch: (conversationId) => {
    const sw = get().recentSwitch;
    if (!sw || sw.conversationId !== conversationId) return null;
    set({ recentSwitch: null });
    return { roleId: sw.roleId };
  },

  importAgencyExperts: async (path: string) => {
    set({ agencyLoading: true });
    try {
      const result = await invoke<{ count: number; workflows_created: number; tools_matched: number; errors: string[] }>("import_agency_experts", {
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
      const roles = rows.map(agencyRowToRole);
      set({ agencyRoles: roles, agencyLoaded: true });
    } catch (e) {
      console.error("[expertStore] loadAgencyRoles failed:", e);
      set({ agencyLoaded: true });
    }
  },

  clearAgencyExperts: async () => {
    try {
      await invoke("clear_agency_experts");
      set({ agencyRoles: [], agencyLoaded: false });
    } catch (e) {
      console.error("[expertStore] clearAgencyExperts failed:", e);
    }
  },

  deleteAgencyExpert: async (id: string) => {
    try {
      await invoke("delete_agency_expert", { request: { id } });
      const roles = get().agencyRoles.filter((r) => r.id !== id);
      set({ agencyRoles: roles });
    } catch (e) {
      console.error("[expertStore] deleteAgencyExpert failed:", e);
    }
  },

  updateAgencyExpert: async (id: string, fields) => {
    try {
      await invoke("update_agency_expert", { request: { id, ...fields } });
      await get().loadAgencyRoles();
    } catch (e) {
      console.error("[expertStore] updateAgencyExpert failed:", e);
    }
  },

  exportAgencyExperts: async () => {
    const json = await invoke<string>("export_agency_experts");
    return json;
  },

  addCustomRole: (role) => {
    const updated = [...get().customRoles, role];
    saveCustomRoles(updated);
    set({ customRoles: updated });
  },

  updateCustomRole: (role) => {
    const existing = get().customRoles.find((r) => r.id === role.id);
    if (!existing) return;
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
      const validRoles: ExpertRole[] = [];
      for (const item of parsed) {
        if (item.id && item.displayName && item.category) {
          validRoles.push(item as ExpertRole);
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
