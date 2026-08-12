// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import { useSkillExtensionStore } from "@/stores/feature/skillExtensionStore";
import type { SkillCommandAction, SkillLifecycleHooks, SkillManifest, SkillPermissions } from "@/types";
import { getActionRouter } from "./actionRouter";

interface LifecycleCacheEntry {
  hooks: SkillLifecycleHooks | null;
  permissions: SkillPermissions | undefined;
  ts: number;
}

const lifecycleCache = new Map<string, LifecycleCacheEntry>();

/** P3-2.19: 生命周期缓存 TTL 可配置，默认 5 分钟 */
let lifecycleCacheTtlMs = 5 * 60 * 1000;

/** 获取当前缓存 TTL（毫秒） */
export function getLifecycleCacheTtl(): number {
  return lifecycleCacheTtlMs;
}

/** 设置缓存 TTL（毫秒），传入 0 或负数将禁用缓存 */
export function setLifecycleCacheTtl(ttlMs: number): void {
  lifecycleCacheTtlMs = ttlMs;
}

async function readLifecycleData(
  skillName: string,
): Promise<{
  hooks: SkillLifecycleHooks | null;
  permissions: SkillPermissions | undefined;
}> {
  const cached = lifecycleCache.get(skillName);
  const ttl = lifecycleCacheTtlMs;
  if (cached && ttl > 0 && Date.now() - cached.ts < ttl) {
    return { hooks: cached.hooks, permissions: cached.permissions };
  }

  // P2 #19: 带退避的重试
  const maxRetries = 3;
  const retryDelays = [1000, 2000, 4000]; // ms

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      const detail = await invoke<{ manifest?: SkillManifest }>("get_skill", {
        name: skillName,
      });
      const hooks = detail?.manifest?.lifecycle ?? null;
      const permissions = detail?.manifest?.permissions;
      lifecycleCache.set(skillName, { hooks, permissions, ts: Date.now() });
      return { hooks, permissions };
    } catch (e) {
      if (attempt < maxRetries) {
        console.warn(
          `[skillLifecycle] get_skill failed for "${skillName}" (attempt ${attempt + 1}/${
            maxRetries + 1
          }), retrying in ${retryDelays[attempt]}ms:`,
          e,
        );
        await new Promise((resolve) => setTimeout(resolve, retryDelays[attempt]));
      } else {
        console.error(
          `[skillLifecycle] get_skill failed for "${skillName}" after ${maxRetries + 1} attempts:`,
          e,
        );
        return { hooks: null, permissions: undefined };
      }
    }
  }
  return { hooks: null, permissions: undefined };
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

/** P3 #22: 顺序执行钩子，消除卸载时的竞态条件 */
async function executeHooksSequential(
  actions: SkillCommandAction[],
  skillName: string,
  permissions?: SkillPermissions,
): Promise<void> {
  if (!actions || actions.length === 0) {
    return;
  }
  const router = getActionRouter();
  for (const action of actions) {
    try {
      await router.execute(action, { skillName, permissions });
    } catch (e) {
      logIpcError(`Lifecycle hook failed for ${skillName}`)(e);
      // P3 #22: 顺序执行时，单个钩子失败不阻断后续钩子
    }
  }
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
    await executeHooksSequential(hooks.onDisable, skillName, permissions);
  }
}

export async function triggerOnUninstall(skillName: string): Promise<void> {
  const { hooks, permissions } = await readLifecycleData(skillName);
  if (hooks?.onUninstall) {
    await executeHooksSequential(hooks.onUninstall, skillName, permissions);
  }
}

/** 刷新技能扩展（技能文件变更时） */
export async function triggerSkillReload(skillName: string): Promise<void> {
  invalidateLifecycleCache(skillName);
  useSkillExtensionStore.getState().refreshSkill(skillName);
}
