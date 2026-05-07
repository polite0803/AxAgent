import { invoke } from "@/lib/invoke";
import type { SkillCommandAction, SkillManifestMeta, SkillPermissions, SkillPermissionsV2 } from "@/types";

// ── V1 权限 ──────────────────────────────────────────────────────────

/** 读取 skill 的权限声明 */
async function loadPermissions(skillName: string): Promise<SkillPermissions | null> {
  try {
    const detail = await invoke<{ manifest?: SkillManifestMeta }>("get_skill", { name: skillName });
    return detail?.manifest?.permissions ?? null;
  } catch {
    return null;
  }
}

/** 检查声明式 action 是否被 skill 权限允许 */
export async function checkDeclarativeAction(
  skillName: string,
  action: SkillCommandAction,
): Promise<{ allowed: boolean; reason?: string }> {
  if (action.mode !== "declarative") {
    return { allowed: true }; // agentic actions 由后端权限系统控制
  }

  const perms = await loadPermissions(skillName);
  if (!perms) { return { allowed: true }; // 无权限声明 = 默认允许
   }

  const act = action.action;

  // 检查 Tauri 命令调用权限
  if (act.type === "invoke" && perms.commands) {
    if (!perms.commands.includes(act.command)) {
      return {
        allowed: false,
        reason: `Skill "${skillName}" 无权调用命令 "${act.command}"`,
      };
    }
  }

  // 检查事件发送权限
  if (act.type === "emit" && perms.events) {
    const eventAllowed = perms.events.some((pattern) => {
      if (pattern.endsWith("*")) {
        return act.event.startsWith(pattern.slice(0, -1));
      }
      return act.event === pattern;
    });
    if (!eventAllowed) {
      return {
        allowed: false,
        reason: `Skill "${skillName}" 无权发送事件 "${act.event}"`,
      };
    }
  }

  return { allowed: true };
}

/** 检查整个 action 链 */
export async function checkActionChain(
  skillName: string,
  actions: SkillCommandAction[],
): Promise<{ allowed: boolean; reason?: string }> {
  for (const action of actions) {
    const result = await checkDeclarativeAction(skillName, action);
    if (!result.allowed) { return result; }
  }
  return { allowed: true };
}

/** 检查所有技能工具权限（供后端参考） */
export async function getAllowedTools(skillName: string): Promise<string[]> {
  const perms = await loadPermissions(skillName);
  return perms?.tools ?? [];
}

// ── V2 权限（前置白名单，加载时强制执行） ──────────────────────────

/** V2 权限校验结果 */
export interface PermissionValidationResult {
  /** 是否通过 */
  valid: boolean;
  /** 拒绝原因列表 */
  violations: string[];
}

/** 默认权限：无声明时拒绝所有 */
const DEFAULT_V2_PERMISSIONS: Required<SkillPermissionsV2> = {
  commands: [],
  events: [],
  storeRead: [],
  storeWrite: [],
  navigate: [],
  network: [],
  filesystem: { read: [], write: [] },
  tools: [],
};

/**
 * 在 Skill 加载时校验权限声明（前置白名单）。
 *
 * V2 架构的核心安全机制：
 * - 未声明权限 = 拒绝所有操作
 * - 支持通配符 "read_*" 匹配
 * - 返回完整的违规列表，而非遇错即停
 *
 * @param permissions Skill 声明的权限
 * @param requiredCommands Skill 实际需要的命令列表（从 manifest.capabilities 提取）
 * @returns 校验结果
 */
export function validateSkillPermissionsAtLoad(
  permissions: SkillPermissionsV2 | undefined,
  requiredCommands: string[],
): PermissionValidationResult {
  const violations: string[] = [];

  if (!permissions) {
    // 无权限声明：若是新 Skill 则拒绝所有
    violations.push("Skill 未声明 permissions 字段，拒绝加载");
    return { valid: false, violations };
  }

  const perms = { ...DEFAULT_V2_PERMISSIONS, ...permissions };

  // 校验命令：Skill 需要的每个命令都必须在白名单中
  for (const cmd of requiredCommands) {
    if (!isWildcardMatch(cmd, perms.commands)) {
      violations.push(`未授权命令: "${cmd}"`);
    }
  }

  // 校验 store 写入者不能读取（最小权限原则提示）
  if (perms.storeWrite.length > 0 && perms.storeRead.length === 0) {
    violations.push("声明了 storeWrite 但未声明 storeRead（可能导致写入后无法验证）");
  }

  return {
    valid: violations.every((v) => !v.startsWith("未授权")),
    violations,
  };
}

/**
 * 通配符匹配
 * @param target 待匹配字符串
 * @param patterns 模式列表，支持 "read_*" 通配符
 */
function isWildcardMatch(target: string, patterns: string[]): boolean {
  return patterns.some((pattern) => {
    if (pattern.endsWith("*")) {
      return target.startsWith(pattern.slice(0, -1));
    }
    return target === pattern;
  });
}

/**
 * 从 V2 SkillManifest 的 capabilities 中提取所有需要的命令。
 */
export function extractRequiredCommands(
  capabilities: unknown[] | undefined,
): string[] {
  if (!capabilities) { return []; }
  const commands = new Set<string>();

  function walk(obj: unknown): void {
    if (!obj || typeof obj !== "object") { return; }
    if (Array.isArray(obj)) {
      for (const item of obj) { walk(item); }
      return;
    }
    const record = obj as Record<string, unknown>;
    // 提取 invoke 命令
    if (record.type === "invoke" && typeof record.command === "string") {
      commands.add(record.command);
    }
    // 提取 dynamicText 数据源
    if (typeof record.command === "string" && record.refreshIntervalMs !== undefined) {
      commands.add(record.command);
    }
    for (const value of Object.values(record)) {
      walk(value);
    }
  }

  walk(capabilities);
  return [...commands];
}
