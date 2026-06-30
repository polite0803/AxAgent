// SPDX-License-Identifier: AGPL-3.0-only

import { getActionRouter } from "@/lib/actionRouter";
import type { ActionContext } from "@/lib/actionRouter";
import type { DynamicAction, EventHandler, UISchema } from "@/types";

/**
 * 事件处理引擎：解析 EventHandler 并执行 DynamicAction。
 *
 * 复用项目中已有的 ActionRouter 体系。
 * 通过 update-schema 动作支持动态更新 UI Schema。
 */

/**
 * 执行一组 DynamicAction（顺序执行）。
 */
export async function executeActions(
  actions: DynamicAction[],
  context?: Record<string, unknown>,
): Promise<void> {
  const router = getActionRouter();
  const actionCtx: ActionContext = {
    skillName: context?.skillName
      ? String(context.skillName)
      : "DynamicUI",
    pageParams: (context?.pageParams as Record<string, string>) || {},
  };

  for (const action of actions) {
    if (action.type === "update-schema") {
      await executeUpdateSchema(action, context);
    } else {
      // 其余动作类型委托给 ActionRouter
      await router.execute(
        {
          mode: "declarative",
          action: {
            type: action.type,
            ...action.config,
          } as Parameters<typeof router.execute>[0] extends { action: infer A }
            ? A
            : never,
        } as Parameters<typeof router.execute>[0],
        actionCtx,
      );
    }
  }
}

/**
 * 处理 update-schema 动作。
 * 通过全局事件通知 DynamicUIRenderer 更新 Schema。
 *
 * config 格式：
 * { schemaId: string, operation: 'replace' | 'append' | 'remove', path?: string, newSchema?: UISchema }
 */
async function executeUpdateSchema(
  action: DynamicAction,
  _context?: Record<string, unknown>,
): Promise<void> {
  const config = action.config as {
    schemaId: string;
    operation: "replace" | "append" | "remove";
    path?: string;
    newSchema?: UISchema;
  };

  // 通过自定义事件通知 DynamicUIRenderer 更新 Schema
  window.dispatchEvent(
    new CustomEvent("dynamic-ui:schema-update", {
      detail: {
        schemaId: config.schemaId,
        operation: config.operation,
        path: config.path,
        newSchema: config.newSchema,
      },
    }),
  );
}

/**
 * 处理事件处理器数组，返回 React 事件绑定对象。
 * 返回 { triggerName: handlerFunction } 格式，可直接展开到组件 props。
 */
export function handleEvents(
  handlers: EventHandler[],
  context?: Record<string, unknown>,
): Record<string, (...args: unknown[]) => void> {
  const bindings: Record<string, (...args: unknown[]) => void> = {};

  for (const handler of handlers) {
    const trigger = handler.trigger;
    // onMount / onUnmount 由 DynamicUIRenderer 在 useEffect 中处理
    if (trigger === "onMount" || trigger === "onUnmount") {
      continue;
    }

    bindings[trigger] = (..._args: unknown[]) => {
      void executeActions([...handler.actions], context);
    };
  }

  return bindings;
}

/**
 * 获取需要执行的 mount / unmount 处理器。
 */
export function getLifecycleHandlers(
  handlers: EventHandler[],
): {
  onMount: DynamicAction[];
  onUnmount: DynamicAction[];
} {
  const onMount: DynamicAction[] = [];
  const onUnmount: DynamicAction[] = [];

  for (const handler of handlers) {
    if (handler.trigger === "onMount") {
      onMount.push(...handler.actions);
    } else if (handler.trigger === "onUnmount") {
      onUnmount.push(...handler.actions);
    }
  }

  return { onMount, onUnmount };
}
