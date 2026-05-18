import type { SkillCommandAction } from "@/types";
import { getActionRouter } from "./actionRouter";

// ── 自定义函数注册表（供 "function" 类型声明式 action 使用）─

type CustomHandler = (
  data: Record<string, unknown>,
  skillName: string,
) => Promise<void>;
const customHandlers = new Map<string, CustomHandler>();

export function getCustomFunction(name: string): CustomHandler | undefined {
  return customHandlers.get(name);
}

// ── 便捷方法（向后兼容）──

/** 执行单个 Skill Command Action（根据 mode 分发） */
async function executeSkillAction(
  action: SkillCommandAction,
  navigate: (path: string) => void,
): Promise<void> {
  const router = getActionRouter();
  if (action.mode === "declarative" && action.action.type === "navigate") {
    navigate(action.action.path);
  } else {
    await router.execute(action, { skillName: "" });
  }
}

/** 执行 Action 链 */
export async function executeActionChain(
  actions: SkillCommandAction[],
  navigate: (path: string) => void,
): Promise<void> {
  await Promise.all(
    actions.map((action) => executeSkillAction(action, navigate)),
  );
}
