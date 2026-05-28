import { invoke, logIpcError } from "@/lib/invoke";
import type { SkillCommandAction, SkillLifecycleHooks, SkillManifest, SkillPermissions } from "@/types";
import { getActionRouter } from "./actionRouter";

interface LifecycleCacheEntry {
  hooks: SkillLifecycleHooks | null;
  permissions: SkillPermissions | undefined;
  ts: number;
}

const lifecycleCache = new Map<string, LifecycleCacheEntry>();
const LIFECYCLE_CACHE_TTL_MS = 5 * 60 * 1000;

async function readLifecycleData(
  skillName: string,
): Promise<{
  hooks: SkillLifecycleHooks | null;
  permissions: SkillPermissions | undefined;
}> {
  const cached = lifecycleCache.get(skillName);
  if (cached && Date.now() - cached.ts < LIFECYCLE_CACHE_TTL_MS) {
    return { hooks: cached.hooks, permissions: cached.permissions };
  }

  try {
    const detail = await invoke<{ manifest?: SkillManifest }>("get_skill", {
      name: skillName,
    });
    const hooks = detail?.manifest?.lifecycle ?? null;
    const permissions = detail?.manifest?.permissions;
    lifecycleCache.set(skillName, { hooks, permissions, ts: Date.now() });
    return { hooks, permissions };
  } catch {
    return { hooks: null, permissions: undefined };
  }
}

/** 清除指定 skill 的缓存 */
export function invalidateLifecycleCache(skillName: string): void {
  lifecycleCache.delete(skillName);
}

async function executeHooks(
  actions: SkillCommandAction[],
  skillName: string,
  permissions?: SkillPermissions,
): Promise<void> {
  if (!actions || actions.length === 0) {
    return;
  }
  const router = getActionRouter();
  await Promise.all(
    actions.map((action) =>
      router.execute(action, { skillName, permissions }).catch(logIpcError(`Lifecycle hook failed for ${skillName}`))
    ),
  );
}

export async function triggerOnInstall(skillName: string): Promise<void> {
  const { hooks, permissions } = await readLifecycleData(skillName);
  if (hooks?.onInstall) {
    await executeHooks(hooks.onInstall, skillName, permissions);
  }
}

export async function triggerOnEnable(skillName: string): Promise<void> {
  const { hooks, permissions } = await readLifecycleData(skillName);
  if (hooks?.onEnable) {
    await executeHooks(hooks.onEnable, skillName, permissions);
  }
}

export async function triggerOnDisable(skillName: string): Promise<void> {
  const { hooks, permissions } = await readLifecycleData(skillName);
  if (hooks?.onDisable) {
    await executeHooks(hooks.onDisable, skillName, permissions);
  }
}

export async function triggerOnUninstall(skillName: string): Promise<void> {
  const { hooks, permissions } = await readLifecycleData(skillName);
  if (hooks?.onUninstall) {
    await executeHooks(hooks.onUninstall, skillName, permissions);
  }
}

/** 刷新技能扩展（技能文件变更时） */
export async function triggerSkillReload(skillName: string): Promise<void> {
  const { useSkillExtensionStore } = await import("@/stores");
  useSkillExtensionStore.getState().refreshSkill(skillName);
}
