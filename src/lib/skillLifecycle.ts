import { invoke } from "@/lib/invoke";
import type { SkillCommandAction, SkillLifecycleHooks, SkillManifestMeta } from "@/types";
import { getActionRouter } from "./actionRouter";

/** 从 skill 目录读取 manifest.json 并提取生命周期钩子 */
async function readLifecycleHooks(skillName: string): Promise<SkillLifecycleHooks | null> {
  try {
    const detail = await invoke<{ manifest?: SkillManifestMeta }>("get_skill", {
      name: skillName,
    });
    return detail?.manifest?.lifecycle ?? null;
  } catch {
    return null;
  }
}

/** 执行生命周期钩子 */
async function executeHooks(actions: SkillCommandAction[], skillName: string): Promise<void> {
  if (!actions || actions.length === 0) { return; }
  const router = getActionRouter();
  for (const action of actions) {
    try {
      await router.execute(action, { skillName });
    } catch (e) {
      console.error(`[Lifecycle] Hook execution failed for ${skillName}:`, e);
    }
  }
}

/** 技能安装后触发 onInstall */
export async function triggerOnInstall(skillName: string): Promise<void> {
  const hooks = await readLifecycleHooks(skillName);
  if (hooks?.onInstall) {
    await executeHooks(hooks.onInstall, skillName);
  }
}

/** 技能启用时触发 onEnable */
export async function triggerOnEnable(skillName: string): Promise<void> {
  const hooks = await readLifecycleHooks(skillName);
  if (hooks?.onEnable) {
    await executeHooks(hooks.onEnable, skillName);
  }
}

/** 技能禁用时触发 onDisable */
export async function triggerOnDisable(skillName: string): Promise<void> {
  const hooks = await readLifecycleHooks(skillName);
  if (hooks?.onDisable) {
    await executeHooks(hooks.onDisable, skillName);
  }
}

/** 技能卸载前触发 onUninstall */
export async function triggerOnUninstall(skillName: string): Promise<void> {
  const hooks = await readLifecycleHooks(skillName);
  if (hooks?.onUninstall) {
    await executeHooks(hooks.onUninstall, skillName);
  }
}

/** 刷新技能扩展（技能文件变更时） */
export async function triggerSkillReload(skillName: string): Promise<void> {
  const { useSkillExtensionStore } = await import("@/stores");
  useSkillExtensionStore.getState().refreshSkill(skillName);
}
