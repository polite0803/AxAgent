import { invoke } from "@/lib/invoke";
import type { SkillCommandAction } from "@/types";

type CustomHandler = (data: Record<string, unknown>, skillName: string) => Promise<void>;

const customHandlers = new Map<string, CustomHandler>();

/** 注册自定义命令处理器 */
export function registerCustomHandler(handlerId: string, handler: CustomHandler) {
  customHandlers.set(handlerId, handler);
}

/** 注销自定义命令处理器 */
export function unregisterCustomHandler(handlerId: string) {
  customHandlers.delete(handlerId);
}

/** 执行插件命令动作 */
export async function executeSkillAction(
  action: SkillCommandAction,
  navigate: (path: string) => void,
): Promise<void> {
  switch (action.type) {
    case "Navigate":
      navigate(action.path);
      break;
    case "InvokeBackend":
      await invoke(action.command, action.args);
      break;
    case "EmitEvent": {
      const { emit } = await import("@tauri-apps/api/event");
      await emit(action.event, action.payload);
      break;
    }
    case "Custom": {
      const handler = customHandlers.get(action.handlerId);
      if (handler) {
        await handler(action.data, "");
      }
      break;
    }
  }
}
