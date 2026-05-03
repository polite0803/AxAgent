import { invoke } from "@/lib/invoke";
import type { SkillCommandAction, SkillManifestMeta, SkillPermissions } from "@/types";

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
