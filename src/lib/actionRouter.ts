import i18n from "@/i18n";
import { invoke } from "@/lib/invoke";
import { isStoreReadCovered, isStoreWriteCovered, isWildcardMatch } from "@/lib/skillPermissions";
import type { AgenticAction, DeclarativeActionType, SkillCommandAction, SkillPermissions } from "@/types";

export interface ActionContext {
  skillName: string;
  skillContent?: string;
  conversationId?: string;
  pageParams?: Record<string, string>;
  triggerEvent?: Event;
  permissions?: SkillPermissions;
  _chainDepth?: number;
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

export type DeclarativeExecutor = (
  action: DeclarativeActionType,
  ctx: ActionContext,
) => Promise<ActionResult>;

const MAX_CHAIN_DEPTH = 20;

const VALID_ACTION_TYPES = new Set<string>([
  "invoke",
  "navigate",
  "emit",
  "store",
  "function",
  "handler",
  "chain",
]);

interface ActionSchemaRule {
  requiredFields: string[];
  fieldTypes: Record<string, "string" | "object" | "array">;
}

const ACTION_SCHEMAS: Record<string, ActionSchemaRule> = {
  invoke: {
    requiredFields: ["command"],
    fieldTypes: { command: "string", args: "object" },
  },
  navigate: { requiredFields: ["path"], fieldTypes: { path: "string" } },
  emit: {
    requiredFields: ["event"],
    fieldTypes: { event: "string", payload: "object" },
  },
  store: {
    requiredFields: ["storeName", "operation"],
    fieldTypes: { storeName: "string", operation: "string", payload: "object" },
  },
  function: {
    requiredFields: ["name"],
    fieldTypes: { name: "string", args: "array" },
  },
  handler: {
    requiredFields: ["name"],
    fieldTypes: { name: "string", args: "object" },
  },
  chain: { requiredFields: ["actions"], fieldTypes: { actions: "array" } },
};

function validateAction(action: DeclarativeActionType): string | null {
  if (!VALID_ACTION_TYPES.has(action.type)) {
    return i18n.t("actionRouter.unknownType", {
      type: action.type,
      validTypes: [...VALID_ACTION_TYPES].join(", "),
    });
  }
  const schema = ACTION_SCHEMAS[action.type];
  if (!schema) {
    return null;
  }
  for (const field of schema.requiredFields) {
    const value = (action as Record<string, unknown>)[field];
    if (value === undefined || value === null || value === "") {
      return i18n.t("actionRouter.missingField", { type: action.type, field });
    }
  }
  for (const [field, expectedType] of Object.entries(schema.fieldTypes)) {
    const value = (action as Record<string, unknown>)[field];
    if (value === undefined || value === null) {
      continue;
    }
    if (expectedType === "string" && typeof value !== "string") {
      return i18n.t("actionRouter.fieldTypeMismatch", {
        type: action.type,
        field,
        expected: "string",
        actual: typeof value,
      });
    }
    if (expectedType === "object" && typeof value !== "object") {
      return i18n.t("actionRouter.fieldTypeMismatch", {
        type: action.type,
        field,
        expected: "object",
        actual: typeof value,
      });
    }
    if (expectedType === "array" && !Array.isArray(value)) {
      return i18n.t("actionRouter.fieldTypeMismatch", {
        type: action.type,
        field,
        expected: "array",
        actual: typeof value,
      });
    }
  }
  return null;
}

export class ActionRouter {
  private declarativeExecutors = new Map<string, DeclarativeExecutor>();

  constructor() {
    this.registerBuiltinExecutors();
  }

  registerDeclarativeExecutor(
    type: string,
    executor: DeclarativeExecutor,
  ): void {
    if (!VALID_ACTION_TYPES.has(type) && !type.startsWith("custom:")) {
      console.warn(i18n.t("actionRouter.nonStandardType", { type }));
    }
    this.declarativeExecutors.set(type, executor);
  }

  async execute(
    action: SkillCommandAction,
    context: ActionContext,
  ): Promise<ActionResult> {
    try {
      if (action.mode === "agentic") {
        return await this.executeAgentic(action, context);
      }
      return await this.executeDeclarative(action.action, context);
    } catch (e) {
      return { success: false, error: String(e) };
    }
  }

  async executeChain(
    actions: SkillCommandAction[],
    context: ActionContext,
    depth = 0,
  ): Promise<ActionResult> {
    if (depth > MAX_CHAIN_DEPTH) {
      return {
        success: false,
        error: i18n.t("actionRouter.chainDepthExceeded", {
          depth: MAX_CHAIN_DEPTH,
        }),
      };
    }
    // 操作链顺序执行：每一步将上一步的 lastResult.data 合并到 pageParams
    // 传给下一步，且链在任一失败时中断 (break)，必须顺序执行，不能并行。
    let lastResult: ActionResult = { success: true };
    for (const action of actions) {
      lastResult = await this.execute(action, {
        ...context,
        pageParams: {
          ...context.pageParams,
          ...((lastResult.data as Record<string, string>) || {}),
        },
      });
      if (!lastResult.success) {
        break;
      }
    }
    return lastResult;
  }

  private async executeDeclarative(
    action: DeclarativeActionType,
    ctx: ActionContext,
  ): Promise<ActionResult> {
    const validationError = validateAction(action);
    if (validationError) {
      return { success: false, error: validationError };
    }
    const executor = this.declarativeExecutors.get(action.type);
    if (!executor) {
      return {
        success: false,
        error: i18n.t("actionRouter.unregisteredAction", { type: action.type }),
      };
    }
    return executor(action, ctx);
  }

  private async executeAgentic(
    action: AgenticAction,
    ctx: ActionContext,
  ): Promise<ActionResult> {
    if (!action.prompt || action.prompt.trim().length === 0) {
      return {
        success: false,
        error: i18n.t("actionRouter.agenticMissingPrompt"),
      };
    }
    const { useConversationStore, useProviderStore, useSettingsStore } = await import("@/stores");
    const convStore = useConversationStore.getState();
    const providerStore = useProviderStore.getState();
    const settingsStore = useSettingsStore.getState().settings;

    const providers = providerStore.providers;
    let provider = settingsStore.default_provider_id
      ? providers.find((p) => p.id === settingsStore.default_provider_id && p.enabled)
      : undefined;
    let model = provider?.models.find(
      (m) => m.model_id === settingsStore.default_model_id && m.enabled,
    );
    if (!provider || !model) {
      provider = providers.find(
        (p) => p.enabled && p.models.some((m) => m.enabled),
      );
      model = provider?.models.find((m) => m.enabled);
    }

    if (!provider || !model) {
      return { success: false, error: i18n.t("actionRouter.noLlmAvailable") };
    }

    const title = `${ctx.skillName || "Skill"}: ${action.prompt.slice(0, 50)}`;

    try {
      const conv = await convStore.createConversation(
        title,
        model.model_id,
        provider.id,
      );
      await convStore.sendMessage(action.prompt);
      return { success: true, data: { conversationId: conv.id } };
    } catch (e) {
      return { success: false, error: String(e) };
    }
  }

  private registerBuiltinExecutors(): void {
    this.declarativeExecutors.set("invoke", async (action, ctx) => {
      if (action.type !== "invoke") {
        return { success: false, error: i18n.t("actionRouter.typeMismatch") };
      }
      if (!action.command) {
        return {
          success: false,
          error: i18n.t("actionRouter.invokeMissingCommand"),
        };
      }
      if (ctx.permissions) {
        const allowed = isWildcardMatch(
          action.command,
          ctx.permissions.commands ?? [],
        );
        if (!allowed) {
          return {
            success: false,
            error: i18n.t("actionRouter.commandPermissionDenied", {
              command: action.command,
            }),
          };
        }
      }
      const result = await invoke(action.command, action.args || {});
      return { success: true, data: result };
    });

    this.declarativeExecutors.set("navigate", async (action, ctx) => {
      if (action.type !== "navigate") {
        return { success: false, error: i18n.t("actionRouter.typeMismatch") };
      }
      if (ctx.permissions) {
        const allowed = isWildcardMatch(
          action.path,
          ctx.permissions.navigate ?? [],
        );
        if (!allowed) {
          return {
            success: false,
            error: i18n.t("actionRouter.navigatePermissionDenied", {
              path: action.path,
            }),
          };
        }
      }
      window.location.hash = action.path;
      return { success: true };
    });

    this.declarativeExecutors.set("emit", async (action, ctx) => {
      if (action.type !== "emit") {
        return { success: false, error: i18n.t("actionRouter.typeMismatch") };
      }
      if (ctx.permissions) {
        const allowed = isWildcardMatch(
          action.event,
          ctx.permissions.events ?? [],
        );
        if (!allowed) {
          return {
            success: false,
            error: i18n.t("actionRouter.emitPermissionDenied", {
              event: action.event,
            }),
          };
        }
      }
      window.dispatchEvent(
        new CustomEvent(action.event, { detail: action.payload }),
      );
      return { success: true };
    });

    this.declarativeExecutors.set("store", async (action, ctx) => {
      if (action.type !== "store") {
        return { success: false, error: i18n.t("actionRouter.typeMismatch") };
      }
      const operation = action.operation;
      if (
        operation !== "get"
        && operation !== "set"
        && operation !== "update"
      ) {
        return {
          success: false,
          error: i18n.t("actionRouter.unknownStoreOp", { operation }),
        };
      }
      if (ctx.permissions) {
        const isRead = operation === "get";
        const isWrite = operation === "set" || operation === "update";
        const selector = action.payload
            && typeof action.payload === "object"
            && "selector" in (action.payload as Record<string, unknown>)
          ? String((action.payload as Record<string, unknown>).selector)
          : undefined;
        const fieldSuffix = selector ? ` "${selector}"` : "";
        if (
          isRead
          && !isStoreReadCovered(
            action.storeName,
            selector,
            ctx.permissions.storeRead ?? [],
          )
        ) {
          return {
            success: false,
            error: i18n.t("actionRouter.storeReadPermissionDenied", {
              storeName: action.storeName,
              field: fieldSuffix,
            }),
          };
        }
        if (
          isWrite
          && !isStoreWriteCovered(
            action.storeName,
            selector,
            ctx.permissions.storeWrite ?? [],
          )
        ) {
          return {
            success: false,
            error: i18n.t("actionRouter.storeWritePermissionDenied", {
              storeName: action.storeName,
              field: fieldSuffix,
            }),
          };
        }
      }
      const { getStoreRegistry } = await import("./storeRegistry");
      const store = getStoreRegistry().get(action.storeName);
      if (!store) {
        return {
          success: false,
          error: i18n.t("actionRouter.storeNotRegistered", {
            storeName: action.storeName,
          }),
        };
      }
      const result = store[operation](action.payload);
      return { success: true, data: result };
    });

    this.declarativeExecutors.set("function", async (action, ctx) => {
      if (action.type !== "function") {
        return { success: false, error: i18n.t("actionRouter.typeMismatch") };
      }
      const { getCustomFunction } = await import("./skillActionExecutor");
      const fn = getCustomFunction(action.name);
      if (!fn) {
        return {
          success: false,
          error: i18n.t("actionRouter.functionNotRegistered", {
            name: action.name,
          }),
        };
      }
      await fn({ args: action.args }, ctx.skillName);
      return { success: true };
    });

    this.declarativeExecutors.set("handler", async (action, ctx) => {
      if (action.type !== "handler") {
        return { success: false, error: i18n.t("actionRouter.typeMismatch") };
      }
      const { useSkillExtensionStore } = await import("@/stores");
      const handler = useSkillExtensionStore.getState().getHandler(action.name);
      if (!handler) {
        return {
          success: false,
          error: i18n.t("actionRouter.handlerNotFound", { name: action.name }),
        };
      }
      if (handler.mode === "declarative" && handler.actions) {
        return this.executeChain(handler.actions, ctx);
      }
      return {
        success: false,
        error: i18n.t("actionRouter.handlerNotDeclarative", {
          name: action.name,
        }),
      };
    });

    this.declarativeExecutors.set("chain", async (action, ctx) => {
      if (action.type !== "chain") {
        return { success: false, error: i18n.t("actionRouter.typeMismatch") };
      }
      const depth = (ctx._chainDepth || 0) + 1;
      if (depth > MAX_CHAIN_DEPTH) {
        return {
          success: false,
          error: i18n.t("actionRouter.chainDepthExceeded", {
            depth: MAX_CHAIN_DEPTH,
          }),
        };
      }
      return this.executeChain(
        action.actions.map((a) => ({
          mode: "declarative" as const,
          action: a,
        })),
        { ...ctx, _chainDepth: depth },
      );
    });
  }
}

let _instance: ActionRouter | null = null;

export function getActionRouter(): ActionRouter {
  if (!_instance) {
    _instance = new ActionRouter();
  }
  return _instance;
}
