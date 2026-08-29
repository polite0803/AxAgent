// SPDX-License-Identifier: AGPL-3.0-only

// AgentRole Store —— 角色/岗位状态管理
// 数据源为 agent_roles 表（list_agent_roles / save_agent_role / delete_agent_role）。
// responsibilities / decisionAuthority / managedExpertIds /
// requiredCertifications 以 JSON 字符串存储（parse/stringify 转换）。

import i18n from "@/i18n";
import { invoke, logIpcError } from "@/lib/invoke";
import { message } from "@/lib/toast";
import type { AgentRole, SaveAgentRoleInput } from "@/types";
import { create } from "zustand";

/**
 * 后端 AgentRoleDto 的原始形态。
 * 字段对齐 `src-tauri/crates/harness/src/repo_dtos.rs::AgentRoleDto`。
 * 后端 serde 使用 rename_all = "camelCase"，前端接收即 camelCase。
 */
interface AgentRoleDtoRow {
  id: string;
  name: string;
  description: string | null;
  responsibilities: string | null;
  decisionAuthority: string | null;
  reportsTo: string | null;
  managedExpertIds: string | null;
  requiredCertifications: string | null;
  activeDomains: string[] | null;
  systemPrompt: string;
  icon: string | null;
  color: string | null;
  source: string;
  sortOrder: number;
  isEnabled: boolean;
  createdAt: number;
  updatedAt: number;
}

/** 安全 parse JSON 字符串为字符串数组；失败返回空数组 */
function parseStringArray(raw: string | null): string[] {
  if (!raw) {
    return [];
  }
  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((x) => typeof x === "string") : [];
  } catch {
    return [];
  }
}

/** 安全 parse JSON 字符串为对象；失败返回 null */
function parseObject(raw: string | null): Record<string, unknown> | null {
  if (!raw) {
    return null;
  }
  try {
    const parsed = JSON.parse(raw);
    return parsed !== null && typeof parsed === "object" && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

/** 将后端 DTO Row 转换为前端结构化 AgentRole */
function dtoToAgentRole(row: AgentRoleDtoRow): AgentRole {
  return {
    id: row.id,
    name: row.name,
    description: row.description,
    responsibilities: parseStringArray(row.responsibilities),
    decisionAuthority: parseObject(row.decisionAuthority),
    reportsTo: row.reportsTo,
    managedExpertIds: parseStringArray(row.managedExpertIds),
    requiredCertifications: parseStringArray(row.requiredCertifications),
    activeDomains: row.activeDomains ?? [],
    systemPrompt: row.systemPrompt,
    icon: row.icon,
    color: row.color,
    source: row.source as AgentRole["source"],
    sortOrder: row.sortOrder,
    isEnabled: row.isEnabled,
    createdAt: row.createdAt,
    updatedAt: row.updatedAt,
  };
}

/** 将前端 SaveAgentRoleInput 转换为后端可接收的格式 */
function serializeSaveInput(input: SaveAgentRoleInput): Record<string, unknown> {
  return {
    id: input.id,
    name: input.name,
    description: input.description ?? null,
    responsibilities: input.responsibilities ?? null,
    decisionAuthority: input.decisionAuthority ?? null,
    reportsTo: input.reportsTo ?? null,
    managedExpertIds: input.managedExpertIds ?? null,
    requiredCertifications: input.requiredCertifications ?? null,
    activeDomains: input.activeDomains ?? null,
    systemPrompt: input.systemPrompt,
    icon: input.icon ?? null,
    color: input.color ?? null,
    source: input.source ?? null,
    sortOrder: input.sortOrder ?? null,
  };
}

interface AgentRoleState {
  roles: AgentRole[];
  loading: boolean;
  loaded: boolean;

  fetchRoles: (source?: "builtin" | "custom") => Promise<void>;
  getRoleById: (id: string) => AgentRole | undefined;
  /** 构造岗位树（按 reportsTo 引用），返回顶层节点列表 */
  getRoleTree: () => Array<AgentRole & { children: AgentRole[] }>;
  saveRole: (input: SaveAgentRoleInput) => Promise<AgentRole>;
  deleteRole: (id: string) => Promise<void>;
}

export const useAgentRoleStore = create<AgentRoleState>((set, get) => ({
  roles: [],
  loading: false,
  loaded: false,

  fetchRoles: async (source) => {
    set({ loading: true });
    try {
      const rows = await invoke<AgentRoleDtoRow[]>("list_agent_roles", {
        source: source ?? null,
      });
      const roles = (rows ?? []).map(dtoToAgentRole);
      set({ roles, loading: false, loaded: true });
    } catch (e) {
      logIpcError("agentRoleStore.fetchRoles")(e);
      message.error(i18n.t("agentRoleStore.loadFailed", { error: String(e) }));
      set({ loading: false, loaded: true });
    }
  },

  getRoleById: (id) => get().roles.find((r) => r.id === id),

  getRoleTree: () => {
    const roles = get().roles;
    const childrenMap = new Map<string, AgentRole[]>();
    const roots: AgentRole[] = [];
    for (const role of roles) {
      if (role.reportsTo) {
        const siblings = childrenMap.get(role.reportsTo) ?? [];
        siblings.push(role);
        childrenMap.set(role.reportsTo, siblings);
      } else {
        roots.push(role);
      }
    }
    const buildNode = (role: AgentRole): AgentRole & { children: AgentRole[] } => ({
      ...role,
      children: (childrenMap.get(role.id) ?? []).map(buildNode),
    });
    return roots.map(buildNode);
  },

  saveRole: async (input) => {
    const payload = serializeSaveInput(input);
    const row = await invoke<AgentRoleDtoRow>("save_agent_role", {
      id: payload.id,
      name: payload.name,
      description: payload.description,
      systemPrompt: payload.systemPrompt,
      activeDomains: payload.activeDomains ?? [],
      source: payload.source ?? "custom",
      responsibilities: payload.responsibilities,
      decisionAuthority: payload.decisionAuthority,
      reportsTo: payload.reportsTo,
      managedExpertIds: payload.managedExpertIds,
      requiredCertifications: payload.requiredCertifications,
      icon: payload.icon,
      color: payload.color,
    });
    const role = dtoToAgentRole(row);
    set((s) => {
      const existing = s.roles.findIndex((r) => r.id === role.id);
      if (existing >= 0) {
        const roles = [...s.roles];
        roles[existing] = role;
        return { roles };
      }
      return { roles: [...s.roles, role] };
    });
    return role;
  },

  deleteRole: async (id) => {
    await invoke("delete_agent_role", { id });
    set((s) => ({ roles: s.roles.filter((r) => r.id !== id) }));
  },
}));
