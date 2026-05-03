import { invoke } from "@/lib/invoke";
import type { AgenticAction, DeclarativeActionType, SkillCommandAction } from "@/types";

// ── 类型 ──

export interface ActionContext {
  skillName: string;
  skillContent?: string;
  conversationId?: string;
  pageParams?: Record<string, string>;
  triggerEvent?: Event;
}

export interface ActionResult {
  success: boolean;
  data?: unknown;
  error?: string;
  streamChunks?: string[];
  toolCalls?: ToolCallRecord[];
}

export interface ToolCallRecord {
  tool: string;
  args: unknown;
  result: unknown;
}

export type DeclarativeExecutor = (action: DeclarativeActionType, ctx: ActionContext) => Promise<ActionResult>;

// ── Action Router ──

export class ActionRouter {
  private declarativeExecutors = new Map<string, DeclarativeExecutor>();

  constructor() {
    this.registerBuiltinExecutors();
  }

  /** 注册自定义声明式执行器 */
  registerDeclarativeExecutor(type: string, executor: DeclarativeExecutor): void {
    this.declarativeExecutors.set(type, executor);
  }

  /** 执行单个 Action */
  async execute(action: SkillCommandAction, context: ActionContext): Promise<ActionResult> {
    try {
      // 权限检查（声明式 action）
      if (action.mode === "declarative" && context.skillName) {
        const { checkDeclarativeAction } = await import("./skillPermissions");
        const permCheck = await checkDeclarativeAction(context.skillName, action);
        if (!permCheck.allowed) {
          return { success: false, error: permCheck.reason || "Permission denied" };
        }
      }

      if (action.mode === "agentic") {
        return await this.executeAgentic(action, context);
      }
      return await this.executeDeclarative(action.action, context);
    } catch (e) {
      return { success: false, error: String(e) };
    }
  }

  /** 执行 Action 链（顺序执行，前一个的输出合并到上下文） */
  async executeChain(actions: SkillCommandAction[], context: ActionContext): Promise<ActionResult> {
    let lastResult: ActionResult = { success: true };
    for (const action of actions) {
      lastResult = await this.execute(action, {
        ...context,
        pageParams: { ...context.pageParams, ...(lastResult.data as Record<string, string> || {}) },
      });
      if (!lastResult.success) { break; }
    }
    return lastResult;
  }

  /** 执行声明式 Action */
  private async executeDeclarative(action: DeclarativeActionType, ctx: ActionContext): Promise<ActionResult> {
    const executor = this.declarativeExecutors.get(action.type);
    if (!executor) {
      return { success: false, error: `Unknown declarative action type: ${action.type}` };
    }
    return executor(action, ctx);
  }

  /** 执行 Agentic Action */
  private async executeAgentic(action: AgenticAction, ctx: ActionContext): Promise<ActionResult> {
    const { useConversationStore, useProviderStore } = await import("@/stores");
    const convStore = useConversationStore.getState();
    const providerStore = useProviderStore.getState();

    const providers = providerStore.providers;
    const provider = providers.find((p) => p.enabled && p.models.some((m) => m.enabled));
    const model = provider?.models.find((m) => m.enabled);

    if (!provider || !model) {
      return { success: false, error: "No enabled provider/model found" };
    }

    const title = `${ctx.skillName || "Skill"}: ${action.prompt.slice(0, 50)}`;

    try {
      const conv = await convStore.createConversation(title, model.model_id, provider.id);
      return { success: true, data: { conversationId: conv.id } };
    } catch (e) {
      return { success: false, error: String(e) };
    }
  }

  /** 注册内置执行器 */
  private registerBuiltinExecutors(): void {
    // invoke: 调用 Tauri 后端命令
    this.declarativeExecutors.set("invoke", async (action) => {
      if (action.type !== "invoke") { return { success: false, error: "Type mismatch" }; }
      const result = await invoke(action.command, action.args || {});
      return { success: true, data: result };
    });

    // navigate: 前端路由跳转
    this.declarativeExecutors.set("navigate", async (action) => {
      if (action.type !== "navigate") { return { success: false, error: "Type mismatch" }; }
      window.location.hash = action.path;
      return { success: true };
    });

    // emit: 发送事件
    this.declarativeExecutors.set("emit", async (action) => {
      if (action.type !== "emit") { return { success: false, error: "Type mismatch" }; }
      window.dispatchEvent(new CustomEvent(action.event, { detail: action.payload }));
      return { success: true };
    });

    // store: 读写 Zustand Store
    this.declarativeExecutors.set("store", async (action) => {
      if (action.type !== "store") { return { success: false, error: "Type mismatch" }; }
      const { getStoreRegistry } = await import("./storeRegistry");
      const store = getStoreRegistry().get(action.storeName);
      if (!store) { return { success: false, error: `Store ${action.storeName} not registered` }; }
      const result = store[action.operation](action.payload);
      return { success: true, data: result };
    });

    // function: 执行注册的自定义函数
    this.declarativeExecutors.set("function", async (action) => {
      if (action.type !== "function") { return { success: false, error: "Type mismatch" }; }
      const { getCustomFunction } = await import("./skillActionExecutor");
      const fn = getCustomFunction(action.name);
      if (!fn) { return { success: false, error: `Function ${action.name} not registered` }; }
      await fn({ args: action.args }, "");
      return { success: true };
    });

    // handler: 引用 handlers 中定义的处理器
    this.declarativeExecutors.set("handler", async (action) => {
      if (action.type !== "handler") { return { success: false, error: "Type mismatch" }; }
      const { useSkillExtensionStore } = await import("@/stores");
      const handler = useSkillExtensionStore.getState().getHandler(action.name);
      if (!handler) { return { success: false, error: `Handler ${action.name} not found` }; }
      if (handler.mode === "declarative" && handler.actions) {
        return this.executeChain(handler.actions, { skillName: action.name });
      }
      return { success: false, error: `Handler ${action.name} is not declarative` };
    });

    // chain: 嵌套子链
    this.declarativeExecutors.set("chain", async (action) => {
      if (action.type !== "chain") { return { success: false, error: "Type mismatch" }; }
      return this.executeChain(
        action.actions.map((a) => ({ mode: "declarative" as const, action: a })),
        { skillName: "chain" },
      );
    });
  }
}

/** 全局单例 */
let _instance: ActionRouter | null = null;

export function getActionRouter(): ActionRouter {
  if (!_instance) {
    _instance = new ActionRouter();
  }
  return _instance;
}
