import { BUILTIN_EXPERT_PRESETS } from "@/data/expertPresets";
import i18n from "@/i18n";
import { invoke } from "@/lib/invoke";
import type { AgentBehaviorMode, AgentProfile, ExpertCategory } from "@/types";
import { message } from "antd";
import { create } from "zustand";

const CUSTOM_ROLES_KEY = "axagent_custom_expert_roles";
const BUILTIN_IMPORTED_KEY = "axagent_builtin_experts_imported";

/** 将内置预设中的 nameKey/descKey 解析为 name/description */
function resolvePreset(
  preset: (typeof BUILTIN_EXPERT_PRESETS)[number],
): AgentProfile {
  // i18n 模块可能因打包顺序尚未初始化，安全访问 .t()
  const _i18n = i18n as { t?: (key: string) => string } | undefined;
  const name = preset.nameKey && _i18n?.t ? _i18n.t(preset.nameKey) : preset.name;
  const desc = preset.descKey && _i18n?.t ? _i18n.t(preset.descKey) : undefined;
  return {
    ...preset,
    name,
    description: desc || preset.description || null,
    nameKey: preset.nameKey,
    descKey: preset.descKey,
  } as AgentProfile;
}

// 默认仅载入通用助手，完整的 12 个开发专家预设不强制加载。
// 用户可通过专家管理页"导入内置专家"按钮，一次性导入全部 12 个预设。
// resolvePreset 调用 i18n.t()，必须在运行时延迟调用，不能在模块顶层执行。
function getMinimalBuiltin(): AgentProfile[] {
  return BUILTIN_EXPERT_PRESETS.flatMap((p) => p.id === "general-assistant" ? [resolvePreset(p)] : []);
}

function loadBuiltinRoles(): AgentProfile[] {
  try {
    const imported = localStorage.getItem(BUILTIN_IMPORTED_KEY);
    if (imported === "true") {
      return BUILTIN_EXPERT_PRESETS.map(resolvePreset);
    }
  } catch {
    /* ignore */
  }
  return getMinimalBuiltin();
}

function loadCustomRoles(): AgentProfile[] {
  try {
    const stored = localStorage.getItem(CUSTOM_ROLES_KEY);
    if (!stored) {
      return [];
    }
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) {
      return [];
    }
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
  if (row.color) {
    tags.push(row.color);
  }

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
  _builtinLoaded: boolean;
  customRoles: AgentProfile[];
  agencyRoles: AgentProfile[];
  agencyLoaded: boolean;
  agencyLoading: boolean;

  recentSwitch: {
    conversationId: string;
    roleId: string;
    timestamp: number;
  } | null;

  getAllRoles: () => AgentProfile[];
  getRolesByCategory: () => Record<string, AgentProfile[]>;
  getRoleById: (id: string) => AgentProfile | undefined;
  getSystemPrompt: (roleId: string | null) => string | null;
  getCategoryLabel: (roleId: string | null) => string;

  recordSwitch: (conversationId: string, roleId: string) => void;
  consumeSwitch: (conversationId: string) => { roleId: string } | null;

  importAgencyExperts: (
    path: string,
  ) => Promise<{
    count: number;
    workflows_created?: number;
    tools_matched?: number;
    errors: string[];
  }>;
  loadAgencyRoles: () => Promise<void>;
  clearAgencyExperts: () => Promise<void>;
  deleteAgencyExpert: (id: string) => Promise<void>;
  updateAgencyExpert: (
    id: string,
    fields: {
      name?: string;
      description?: string;
      category?: string;
      system_prompt?: string;
      is_enabled?: boolean;
    },
  ) => Promise<void>;
  exportAgencyExperts: () => Promise<string>;
  extractStructure: (text: string) => Promise<AgentProfile | null>;

  addCustomRole: (role: AgentProfile) => void;
  updateCustomRole: (role: AgentProfile) => void;
  removeCustomRole: (id: string) => void;
  exportCustomRoles: () => string;
  importCustomRoles: (json: string) => { count: number; errors: string[] };

  /** 是否已导入全部内置专家 */
  hasFullBuiltinPresets: () => boolean;
  /** 导入全部 12 个内置专家预设 */
  importBuiltinPresets: () => void;
  /** 移除除通用助手外的内置专家 */
  removeBuiltinPresets: () => void;
}

export const useExpertStore = create<ExpertState>((set, get) => ({
  builtinRoles: [] as AgentProfile[], // 延迟到 i18n 就绪后加载
  _builtinLoaded: false,
  customRoles: loadCustomRoles(),
  agencyRoles: [],
  agencyLoaded: false,
  agencyLoading: false,
  recentSwitch: null,

  getAllRoles: () => {
    // 延迟加载：首次访问时加载内置角色（i18n 此时已就绪）
    if (!get()._builtinLoaded) {
      set({ builtinRoles: loadBuiltinRoles(), _builtinLoaded: true });
    }
    const general = get().builtinRoles.find(
      (r) => r.id === "general-assistant",
    );
    const otherBuiltins = get().builtinRoles.filter(
      (r) => r.id !== "general-assistant",
    );
    const result: AgentProfile[] = [];
    if (general) {
      result.push(general);
    }
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
    return get()
      .getAllRoles()
      .find((r) => r.id === id);
  },

  getSystemPrompt: (roleId: string | null) => {
    if (!roleId) {
      return null;
    }
    const role = get().getRoleById(roleId);
    return role?.systemPrompt || null;
  },

  getCategoryLabel: (roleId: string | null) => {
    if (!roleId) {
      return i18n.t("expertCategory.general");
    }
    const role = get().getRoleById(roleId);
    if (!role) {
      return i18n.t("expertCategory.general");
    }
    return i18n.t("expertCategory." + role.category) || role.category;
  },

  recordSwitch: (conversationId, roleId) => {
    set({ recentSwitch: { conversationId, roleId, timestamp: Date.now() } });
  },

  consumeSwitch: (conversationId) => {
    const sw = get().recentSwitch;
    if (!sw || sw.conversationId !== conversationId) {
      return null;
    }
    set({ recentSwitch: null });
    return { roleId: sw.roleId };
  },

  importAgencyExperts: async (path: string) => {
    set({ agencyLoading: true });
    try {
      const result = await invoke<{
        count: number;
        workflows_created: number;
        tools_matched: number;
        errors: string[];
      }>("import_agency_experts", {
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
      message.error(i18n.t("expertStore.loadFailed", { error: String(e) }));
      set({ agencyLoaded: true });
    }
  },

  clearAgencyExperts: async () => {
    try {
      await invoke("clear_agency_experts");
      set({ agencyRoles: [], agencyLoaded: false });
    } catch (e) {
      console.error("[expertStore] clearAgencyExperts failed:", e);
      message.error(i18n.t("expertStore.clearFailed", { error: String(e) }));
    }
  },

  deleteAgencyExpert: async (id: string) => {
    try {
      await invoke("delete_agency_expert", { request: { id } });
      const roles = get().agencyRoles.filter((r) => r.id !== id);
      set({ agencyRoles: roles });
    } catch (e) {
      console.error("[expertStore] deleteAgencyExpert failed:", e);
      message.error(i18n.t("expertStore.deleteFailed", { error: String(e) }));
    }
  },

  updateAgencyExpert: async (id: string, fields) => {
    try {
      await invoke("update_agency_expert", { request: { id, ...fields } });
      await get().loadAgencyRoles();
    } catch (e) {
      console.error("[expertStore] updateAgencyExpert failed:", e);
      message.error(i18n.t("expertStore.updateFailed", { error: String(e) }));
    }
  },

  exportAgencyExperts: async () => {
    const json = await invoke<string>("export_agency_experts");
    return json;
  },

  extractStructure: async (expertId: string) => {
    try {
      const row = await invoke<AgencyExpertRow>("extract_expert_structure", {
        request: { expertId },
      });
      if (!row) {
        return null;
      }
      return agencyRowToRole(row);
    } catch (e) {
      console.error("[expertStore] extractStructure failed:", e);
      message.error(i18n.t("expertStore.extractFailed", { error: String(e) }));
      return null;
    }
  },

  addCustomRole: (role) => {
    set((s) => ({ customRoles: [...s.customRoles, role] }));
    saveCustomRoles(get().customRoles);
  },

  updateCustomRole: (role) => {
    const existing = get().customRoles.find((r) => r.id === role.id);
    if (!existing) {
      return;
    }
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
        return { count: 0, errors: [i18n.t("expertStore.jsonFormatError")] };
      }
      const validRoles: AgentProfile[] = [];
      for (const item of parsed) {
        const hasId = !!item.id;
        const hasName = !!(item.displayName || item.name);
        const hasCategory = !!item.category;
        if (hasId && hasName && hasCategory) {
          validRoles.push(item as AgentProfile);
        } else {
          errors.push(
            i18n.t("expertStore.skippedInvalidRole", {
              role: JSON.stringify(item).slice(0, 50),
            }),
          );
        }
      }
      if (validRoles.length > 0) {
        const existingIds = new Set(get().customRoles.map((r) => r.id));
        const newRoles = validRoles.filter((r) => !existingIds.has(r.id));
        set((s) => ({ customRoles: [...s.customRoles, ...newRoles] }));
        saveCustomRoles(get().customRoles);
      }
      return { count: validRoles.length, errors };
    } catch (e) {
      return {
        count: 0,
        errors: [i18n.t("expertStore.jsonParseError", { error: String(e) })],
      };
    }
  },

  hasFullBuiltinPresets: () => {
    try {
      return localStorage.getItem(BUILTIN_IMPORTED_KEY) === "true";
    } catch {
      return false;
    }
  },

  importBuiltinPresets: () => {
    localStorage.setItem(BUILTIN_IMPORTED_KEY, "true");
    set({ builtinRoles: BUILTIN_EXPERT_PRESETS.map(resolvePreset) });
  },

  removeBuiltinPresets: () => {
    localStorage.removeItem(BUILTIN_IMPORTED_KEY);
    set({ builtinRoles: getMinimalBuiltin() });
  },
}));
