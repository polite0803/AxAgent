// SPDX-License-Identifier: AGPL-3.0-only

import i18n from "@/i18n";
import type { SkillPermissions } from "@/types";

// ── 权限声明区分 sentinel ─────────────────────────────────────────

/** 区分 "undefined（未声明）" vs "{}（声明了但为空）" 的 sentinel 值 */
export const __NO_PERMISSIONS_DECLARED__ = Symbol("NO_PERMISSIONS_DECLARED");

// ── 权限声明签名校验（P3 #21） ──────────────────────────────────────

/** 存储各 skill 的权限 manifest 哈希 */
const permissionHashStore = new Map<string, string>();

/**
 * 计算权限声明的确定性哈希（SHA-256 截取前 16 字符 hex）。
 * 对 permissions 对象做 JSON 稳定序列化后计算。
 */
async function computePermissionHash(
  permissions: SkillPermissions | undefined,
): Promise<string> {
  const payload = permissions === undefined
    ? `__NO_PERMISSIONS_DECLARED__:${String(__NO_PERMISSIONS_DECLARED__)}`
    : JSON.stringify(permissions, Object.keys(permissions).sort());
  const encoder = new TextEncoder();
  const data = encoder.encode(payload);
  const digest = await crypto.subtle.digest("SHA-256", data);
  const hex = Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
  return hex.slice(0, 16);
}

/**
 * 检查权限声明是否发生变更。若已存储哈希与新哈希不匹配，发出 console.warn。
 * @returns 当前哈希值
 */
export async function checkPermissionIntegrity(
  skillName: string,
  permissions: SkillPermissions | undefined,
): Promise<string> {
  const newHash = await computePermissionHash(permissions);
  const oldHash = permissionHashStore.get(skillName);
  if (oldHash !== undefined && oldHash !== newHash) {
    console.warn(
      `[skillPermissions] Permission manifest changed for "${skillName}" — `
        + `old hash: ${oldHash}, new hash: ${newHash}. `
        + `Review the updated permissions before trusting this skill.`,
    );
  }
  permissionHashStore.set(skillName, newHash);
  return newHash;
}

/** 清除指定 skill 的权限哈希缓存 */
export function clearPermissionHash(skillName: string): void {
  permissionHashStore.delete(skillName);
}

// ── 权限校验（加载时强制执行的白名单） ──────────────────────────────

/** 权限校验结果 */
export interface PermissionValidationResult {
  /** 是否通过 */
  valid: boolean;
  /** 拒绝原因列表 */
  violations: string[];
}

/** P1 #16: 禁止 Skill 通过声明式 action 访问的 Store */
const FORBIDDEN_STORES = new Set(["skill"]);

/** 默认权限：无声明时拒绝所有操作 */
const DEFAULT_PERMISSIONS: Required<SkillPermissions> = {
  commands: [],
  events: [],
  storeRead: [],
  storeWrite: [],
  navigate: [],
  network: [],
  filesystem: { read: [], write: [] },
  tools: [],
};

function parseStorePerm(pattern: string): {
  storeName: string;
  fieldPath?: string;
} {
  const colonIdx = pattern.indexOf(":");
  if (colonIdx === -1) {
    return { storeName: pattern };
  }
  return {
    storeName: pattern.slice(0, colonIdx),
    fieldPath: pattern.slice(colonIdx + 1),
  };
}

function isStorePermCovered(
  storeName: string,
  fieldPath: string | undefined,
  perms: string[],
): boolean {
  return perms.some((pattern) => {
    const parsed = parseStorePerm(pattern);
    if (parsed.storeName !== storeName) {
      return false;
    }
    if (!parsed.fieldPath) {
      return true;
    }
    if (!fieldPath) {
      return false;
    }
    return (
      fieldPath === parsed.fieldPath
      || fieldPath.startsWith(parsed.fieldPath + ".")
      // Support array index notation: "items.0.name" is a child of "items"
      || (parsed.fieldPath.split(".").every((seg) => /^\d+$/.test(seg) || /^[a-zA-Z_]\w*$/.test(seg))
        && fieldPath.startsWith(parsed.fieldPath + "."))
    );
  });
}

export function isStoreReadCovered(
  storeName: string,
  fieldPath: string | undefined,
  perms: string[],
): boolean {
  return isStorePermCovered(storeName, fieldPath, perms);
}

export function isStoreWriteCovered(
  storeName: string,
  fieldPath: string | undefined,
  perms: string[],
): boolean {
  return isStorePermCovered(storeName, fieldPath, perms);
}

/**
 * 在 Skill 加载时校验权限声明（前置白名单）。
 *
 * 核心安全机制：
 * - 未声明权限 = 拒绝所有操作
 * - 支持通配符 "read_*" 匹配
 * - 返回完整的违规列表
 *
 * 职责边界：此函数为声明时静态校验，仅能检查 manifest 中显式声明的内容（commands）。
 * storeRead/storeWrite/navigate/network 的操作目标在声明时不可知，需在运行时
 * per-call 白名单检查中强制。实现在 RPC 桥接层
 * （SkillSandboxContainer → createHostApiBridge / actionRouter.ts）。
 *
 * @param permissions Skill 声明的权限
 * @param requiredCommands Skill 实际需要的命令列表（从 manifest.capabilities 提取）
 * @returns 校验结果
 */
/** 授权违规的前缀标记 */
const UNAUTHORIZED_PREFIX = "UNAUTH:";

export function validateSkillPermissions(
  permissions: SkillPermissions | undefined,
  requiredCommands: string[],
): PermissionValidationResult {
  const violations: string[] = [];

  if (permissions === undefined) {
    violations.push(
      `${UNAUTHORIZED_PREFIX}${i18n.t("skillPermissions.undeclaredPermissions")} (${
        String(__NO_PERMISSIONS_DECLARED__)
      })`,
    );
    return { valid: false, violations };
  }

  const perms = { ...DEFAULT_PERMISSIONS, ...permissions };

  // P1 #16: 硬性拒绝访问 forbidden store
  for (const perm of perms.storeRead) {
    const { storeName } = parseStorePerm(perm);
    if (FORBIDDEN_STORES.has(storeName)) {
      violations.push(
        `${UNAUTHORIZED_PREFIX}storeRead "${perm}" is forbidden: skill store access is restricted`,
      );
    }
  }
  for (const perm of perms.storeWrite) {
    const { storeName } = parseStorePerm(perm);
    if (FORBIDDEN_STORES.has(storeName)) {
      violations.push(
        `${UNAUTHORIZED_PREFIX}storeWrite "${perm}" is forbidden: skill store access is restricted`,
      );
    }
  }

  for (const cmd of requiredCommands) {
    if (!isWildcardMatch(cmd, perms.commands)) {
      violations.push(
        `${UNAUTHORIZED_PREFIX}${i18n.t("skillPermissions.commandNotInWhitelist", { cmd })}`,
      );
    }
  }

  // 最小权限原则提示（非阻塞警告）
  if (perms.storeWrite.length > 0 && perms.storeRead.length === 0) {
    violations.push(i18n.t("skillPermissions.writeWithoutRead"));
  }

  const writeStores = new Set(
    perms.storeWrite.map((p) => parseStorePerm(p).storeName),
  );
  const readStores = new Set(
    perms.storeRead.map((p) => parseStorePerm(p).storeName),
  );
  for (const ws of writeStores) {
    if (!readStores.has(ws)) {
      violations.push(
        i18n.t("skillPermissions.writeWithoutReadDetail", { ws }),
      );
    }
  }

  // 校验 network 权限：若声明了 network 但沙箱已删除 fetch/XHR，给出提示
  if (perms.network.length > 0) {
    violations.push(i18n.t("skillPermissions.networkDisabled"));
  }

  // 校验 tools 权限：必须至少声明一个工具才能加载
  // tools 权限由 LLM 工具注册系统校验，此处仅做声明检查

  return {
    valid: !violations.some((v) => v.startsWith(UNAUTHORIZED_PREFIX)),
    violations,
  };
}

/**
 * 通配符匹配
 * @param target 待匹配字符串
 * @param patterns 模式列表，支持 "read_*" 后缀通配符
 */
export function isWildcardMatch(target: string, patterns: string[]): boolean {
  return patterns.some((pattern) => {
    if (pattern.endsWith("*")) {
      return target.startsWith(pattern.slice(0, -1));
    }
    return target === pattern;
  });
}

/**
 * 从 capabilities 中提取所有需要的 Tauri 命令。
 * 只匹配声明式 action 中 type === "invoke" 的 command 字段。
 */
export function extractRequiredCommands(
  capabilities: unknown[] | undefined,
): string[] {
  if (!capabilities) {
    return [];
  }
  const commands = new Set<string>();

  function walk(obj: unknown): void {
    if (!obj || typeof obj !== "object") {
      return;
    }
    if (Array.isArray(obj)) {
      for (const item of obj) {
        walk(item);
      }
      return;
    }
    const record = obj as Record<string, unknown>;
    // 只提取声明式 invoke action 的命令
    if (record.type === "invoke" && typeof record.command === "string") {
      commands.add(record.command);
    }
    // 提取 dynamicText 轮询数据源命令
    if (
      typeof record.command === "string"
      && typeof record.refreshIntervalMs === "number"
    ) {
      commands.add(record.command);
    }
    for (const value of Object.values(record)) {
      walk(value);
    }
  }

  walk(capabilities);
  return [...commands];
}
