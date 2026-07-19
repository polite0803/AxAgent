// SPDX-License-Identifier: AGPL-3.0-only

// BusinessRole Store —— 业务岗位状态管理
//
// 业务岗位（CEO/CTO/产品经理 等）表达「在组织里担什么责」，
// 与 AgentRole（executor/planner/researcher，表达「怎么干活」）正交。
// AgentProfile 通过 businessRoleId + expertId 将三者融合。
//
// 后端 BusinessRoleDto 中 responsibilities / decisionAuthority /
// managedExpertIds / requiredCertifications / activeDomains 均以 JSON 字符串存储，
// 本 store 负责接收时 parse 为结构化数据，保存时序列化回 JSON 字符串。

import i18n from "@/i18n";
import { invoke, logIpcError } from "@/lib/invoke";
import { message } from "@/lib/toast";
import type { BusinessRole, SaveBusinessRoleInput } from "@/types";
import { create } from "zustand";

/**
 * 后端 BusinessRoleDto 的原始形态（JSON 字符串字段未 parse）。
 * 字段对齐 `src-tauri/crates/harness/src/repo_dtos.rs::BusinessRoleDto`。
 * 后端 serde 使用 rename_all = "camelCase"，前端接收即 camelCase。
 */
interface BusinessRoleDtoRow {
  id: string;
  name: string;
  description: string | null;
  responsibilities: string | null;
  decisionAuthority: string | null;
  reportsTo: string | null;
  managedExpertIds: string | null;
  requiredCertifications: string | null;
  activeDomains: string | null;
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

/** 将后端 DTO Row 转换为前端结构化 BusinessRole */
function dtoToBusinessRole(row: BusinessRoleDtoRow): BusinessRole {
  return {
    id: row.id,
    name: row.name,
    description: row.description,
    responsibilities: parseStringArray(row.responsibilities),
    decisionAuthority: parseObject(row.decisionAuthority),
    reportsTo: row.reportsTo,
    managedExpertIds: parseStringArray(row.managedExpertIds),
    requiredCertifications: parseStringArray(row.requiredCertifications),
    activeDomains: parseStringArray(row.activeDomains),
    systemPrompt: row.systemPrompt,
    icon: row.icon,
    color: row.color,
    source: row.source as BusinessRole["source"],
    sortOrder: row.sortOrder,
    isEnabled: row.isEnabled,
    createdAt: row.createdAt,
    updatedAt: row.updatedAt,
  };
}

/**
 * 将前端 SaveBusinessRoleInput 转换为后端可接收的格式。
 * decisionAuthority 对象会被序列化为 JSON 字符串。
 */
function serializeSaveInput(input: SaveBusinessRoleInput): Record<string, unknown> {
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

interface BusinessRoleState {
  roles: BusinessRole[];
  loading: boolean;
  loaded: boolean;

  fetchRoles: (source?: "builtin" | "custom") => Promise<void>;
  getRoleById: (id: string) => BusinessRole | undefined;
  /** 构造岗位树（按 reportsTo 引用），返回顶层节点列表 */
  getRoleTree: () => Array<BusinessRole & { children: BusinessRole[] }>;
  saveRole: (input: SaveBusinessRoleInput) => Promise<BusinessRole>;
  deleteRole: (id: string) => Promise<void>;
}

export const useBusinessRoleStore = create<BusinessRoleState>((set, get) => ({
  roles: [],
  loading: false,
  loaded: false,

  fetchRoles: async (source) => {
    set({ loading: true });
    try {
      const rows = await invoke<BusinessRoleDtoRow[]>("list_business_roles", {
        source: source ?? null,
      });
      const roles = (rows ?? []).map(dtoToBusinessRole);
      set({ roles, loading: false, loaded: true });
    } catch (e) {
      logIpcError("businessRoleStore.fetchRoles")(e);
      message.error(i18n.t("businessRoleStore.loadFailed", { error: String(e) }));
      set({ loading: false, loaded: true });
    }
  },

  getRoleById: (id) => get().roles.find((r) => r.id === id),

  getRoleTree: () => {
    const roles = get().roles;
    const childrenMap = new Map<string, BusinessRole[]>();
    const roots: BusinessRole[] = [];
    for (const role of roles) {
      if (role.reportsTo) {
        const siblings = childrenMap.get(role.reportsTo) ?? [];
        siblings.push(role);
        childrenMap.set(role.reportsTo, siblings);
      } else {
        roots.push(role);
      }
    }
    const buildNode = (role: BusinessRole): BusinessRole & { children: BusinessRole[] } => ({
      ...role,
      children: (childrenMap.get(role.id) ?? []).map(buildNode),
    });
    return roots.map(buildNode);
  },

  saveRole: async (input) => {
    const payload = serializeSaveInput(input);
    const row = await invoke<BusinessRoleDtoRow>("save_business_role", { input: payload });
    const role = dtoToBusinessRole(row);
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
    await invoke("delete_business_role", { id });
    set((s) => ({ roles: s.roles.filter((r) => r.id !== id) }));
  },
}));
