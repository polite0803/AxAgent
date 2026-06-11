// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Skill RPC Bridge — 宿主侧
 *
 * 负责管理宿主与 Skill iframe 沙箱之间的 postMessage 通信。
 * 提供请求-响应模式（Promise-based）的 RPC 调用。
 *
 * @module sdk/rpcBridge
 */

import { logIpcError } from "@/lib/invoke";
import { isStoreReadCovered, isStoreWriteCovered, isWildcardMatch } from "@/lib/skillPermissions";
import type { HostToSkillMessage, SkillHostApi, SkillHostStore, SkillHostUi, SkillToHostMessage } from "./types";

/** RPC 调用超时时间（毫秒） */
const RPC_TIMEOUT_MS = 15000;

/** 允许的 postMessage 来源（Tauri 自定义协议下仅自身） */
function defaultAllowedOrigin(): string {
  if (typeof window !== "undefined" && window.location.protocol === "tauri:") {
    return window.location.origin;
  }
  return "*";
}

// ── 宿主侧 RPC 注册表 ──

/** 宿主侧 RPC 管理器 */
export interface HostRpcBridge {
  /** 发送消息到 Skill 沙箱 */
  sendMessage(message: HostToSkillMessage): void;
  /** 调用 Skill 内注册的 RPC 方法 */
  callSkillMethod(
    method: string,
    args?: Record<string, unknown>,
  ): Promise<unknown>;
  /** 发送事件到 Skill 沙箱 */
  emitEvent(event: string, payload?: unknown): void;
  /** 通知 Skill 沙箱生命周期 */
  sendLifecycle(
    phase: "mount" | "unmount",
    props?: Record<string, unknown>,
  ): void;
  /** 销毁桥接 */
  destroy(): void;
}

/**
 * 创建宿主侧 RPC 桥接
 *
 * @param contentWindow Skill iframe 的 contentWindow
 * @param origin 允许的来源，生产环境用 Tauri 自定义协议
 * @returns RPC 桥接实例
 */
export function createHostRpcBridge(
  contentWindow: Window | null,
  origin?: string,
): HostRpcBridge {
  const resolvedOrigin = origin ?? defaultAllowedOrigin();
  const pendingCalls = new Map<
    string,
    {
      resolve: (value: unknown) => void;
      reject: (error: Error) => void;
      timer: ReturnType<typeof setTimeout>;
    }
  >();
  let callIdCounter = 0;
  let destroyed = false;

  function handleMessage(event: MessageEvent<SkillToHostMessage>) {
    if (destroyed) {
      return;
    }

    const msg = event.data;
    if (!msg || typeof msg.type !== "string") {
      return;
    }

    if (msg.type === "skill:ready") {
      // no-op in production; DEV-only logging removed
    } else if (msg.type === "skill:error") {
      logIpcError("HostRpcBridge skill error")(msg.error);
    }
  }

  // 仅在非测试环境添加监听器
  if (typeof window !== "undefined") {
    window.addEventListener("message", handleMessage);
  }

  return {
    sendMessage(message: HostToSkillMessage): void {
      if (destroyed || !contentWindow) {
        return;
      }
      try {
        contentWindow.postMessage(message, resolvedOrigin);
      } catch (e) {
        logIpcError("HostRpcBridge send message failed")(e);
      }
    },

    async callSkillMethod(
      method: string,
      args?: Record<string, unknown>,
    ): Promise<unknown> {
      if (destroyed) {
        throw new Error("Bridge is destroyed");
      }
      if (!contentWindow) {
        throw new Error("No content window");
      }

      const callId = `host_${++callIdCounter}_${Date.now()}`;

      return new Promise<unknown>((resolve, reject) => {
        const timer = setTimeout(() => {
          pendingCalls.delete(callId);
          reject(
            new Error(
              `RPC call "${method}" timed out after ${RPC_TIMEOUT_MS}ms`,
            ),
          );
        }, RPC_TIMEOUT_MS);

        pendingCalls.set(callId, { resolve, reject, timer });

        const responseHandler = (event: MessageEvent<HostToSkillMessage>) => {
          const msg = event.data;
          if (msg?.type === "rpc:response" && msg.callId === callId) {
            window.removeEventListener("message", responseHandler);
            const pending = pendingCalls.get(callId);
            if (!pending) {
              return;
            }
            clearTimeout(pending.timer);
            pendingCalls.delete(callId);
            if (msg.error) {
              pending.reject(new Error(msg.error));
            } else {
              pending.resolve(msg.result);
            }
          }
        };

        window.addEventListener("message", responseHandler);

        // 发送 rpc:request 到 Skill
        const requestMsg: SkillToHostMessage = {
          type: "rpc:request",
          callId,
          method,
          args,
        };
        try {
          contentWindow.postMessage(requestMsg, resolvedOrigin);
        } catch (e) {
          window.removeEventListener("message", responseHandler);
          clearTimeout(timer);
          pendingCalls.delete(callId);
          reject(e);
        }
      });
    },

    emitEvent(event: string, payload?: unknown): void {
      this.sendMessage({ type: "host:event", event, payload });
    },

    sendLifecycle(
      phase: "mount" | "unmount",
      props?: Record<string, unknown>,
    ): void {
      this.sendMessage({ type: "host:lifecycle", phase, props });
    },

    destroy(): void {
      destroyed = true;
      // 清理待处理请求
      for (const [, pending] of pendingCalls) {
        clearTimeout(pending.timer);
        pending.reject(new Error("Bridge destroyed"));
      }
      pendingCalls.clear();
      if (typeof window !== "undefined") {
        window.removeEventListener("message", handleMessage);
      }
    },
  };
}

// ── 宿主 API 桥接（Skill 调用宿主方法时使用） ──

export interface HostApiBridge {
  /** 处理来自 Skill 的 RPC 方法调用 */
  handleRpcRequest(msg: SkillToHostMessage): void;
  /** 获取上下文对象（传给沙箱模板使用） */
  getContext(): {
    api: SkillHostApi;
    ui: SkillHostUi;
    store: SkillHostStore;
  };
  /** 销毁 */
  destroy(): void;
}

export interface HostApiBridgeOptions {
  /** 宿主提供的 API 实现 */
  api: SkillHostApi;
  /** 宿主提供的 UI 实现 */
  ui: SkillHostUi;
  /** 宿主提供的 Store 实现 */
  store: SkillHostStore;
  /** Skill 的权限声明 */
  permissions: {
    commands?: string[];
    events?: string[];
    storeRead?: string[];
    storeWrite?: string[];
    navigate?: string[];
  };
  /** Skill 的 contentWindow（用于发送响应） */
  contentWindow: Window | null;
  /** 允许的来源 */
  allowedOrigin?: string;
}

/**
 * 创建宿主 API 桥接（Skill 侧调用宿主方法）
 */
export function createHostApiBridge(
  options: HostApiBridgeOptions,
): HostApiBridge {
  const { api, ui, store, permissions, contentWindow } = options;
  const resolvedOrigin = options.allowedOrigin ?? defaultAllowedOrigin();
  let destroyed = false;

  function sendResponse(
    callId: string,
    result?: unknown,
    error?: string,
  ): void {
    if (destroyed || !contentWindow) {
      return;
    }
    contentWindow.postMessage(
      {
        type: "rpc:response",
        callId,
        result,
        error,
      } satisfies HostToSkillMessage,
      resolvedOrigin,
    );
  }

  function checkCommand(command: string): boolean {
    return isWildcardMatch(command, permissions.commands ?? []);
  }

  function checkEvent(event: string): boolean {
    return isWildcardMatch(event, permissions.events ?? []);
  }

  function checkStoreRead(storeName: string, selector?: string): boolean {
    return isStoreReadCovered(storeName, selector, permissions.storeRead ?? []);
  }

  function checkStoreWrite(storeName: string, selector?: string): boolean {
    return isStoreWriteCovered(
      storeName,
      selector,
      permissions.storeWrite ?? [],
    );
  }

  function checkNavigate(path: string): boolean {
    return isWildcardMatch(path, permissions.navigate ?? []);
  }

  const rpcHandlers: Record<
    string,
    (args?: Record<string, unknown>) => Promise<unknown>
  > = {
    // ctx.api
    "api.invoke": async (args) => {
      const command = args?.command as string;
      if (!command) {
        throw new Error("Command name is required");
      }
      if (!checkCommand(command)) {
        throw new Error(
          `Permission denied: command "${command}" is not allowed`,
        );
      }
      return api.invoke(
        command,
        args?.args as Record<string, unknown> | undefined,
      );
    },
    "api.emit": async (args) => {
      const event = args?.event as string;
      if (!event) {
        throw new Error("Event name is required");
      }
      if (!checkEvent(event)) {
        throw new Error(`Permission denied: event "${event}" is not allowed`);
      }
      api.emit(event, args?.payload);
      return undefined;
    },

    // ctx.ui
    "ui.navigate": async (args) => {
      const path = args?.path as string;
      if (!path) {
        throw new Error("Path is required");
      }
      if (!checkNavigate(path)) {
        throw new Error(`Permission denied: navigate "${path}" is not allowed`);
      }
      ui.navigate(path);
      return undefined;
    },
    "ui.notify": async (args) => {
      ui.notify(
        (args?.message as string) ?? "",
        (args?.type as "info" | "success" | "warning" | "error") ?? "info",
      );
      return undefined;
    },
    "ui.getTheme": async () => {
      return ui.getTheme();
    },
    "ui.getLocale": async () => {
      return ui.getLocale();
    },

    // ctx.store
    "store.read": async (args) => {
      const storeName = args?.storeName as string;
      if (!storeName) {
        throw new Error("Store name is required");
      }
      const selector = args?.selector as string | undefined;
      if (!checkStoreRead(storeName, selector)) {
        const hint = selector
          ? `cannot read store "${storeName}" field "${selector}"`
          : `cannot read store "${storeName}" (field-level permission required)`;
        throw new Error(`Permission denied: ${hint}`);
      }
      return store.read(storeName, selector);
    },
    "store.write": async (args) => {
      const storeName = args?.storeName as string;
      if (!storeName) {
        throw new Error("Store name is required");
      }
      const selector = args?.selector as string | undefined;
      if (!checkStoreWrite(storeName, selector)) {
        const hint = selector
          ? `cannot write store "${storeName}" field "${selector}"`
          : `cannot write store "${storeName}" (field-level permission required)`;
        throw new Error(`Permission denied: ${hint}`);
      }
      await store.write(storeName, args?.value, selector);
      return undefined;
    },
  };

  return {
    handleRpcRequest(msg: SkillToHostMessage): void {
      if (destroyed) {
        return;
      }
      // 类型收窄：只处理 rpc:request 消息
      if (msg.type !== "rpc:request") {
        return;
      }
      const handler = rpcHandlers[msg.method];
      if (!handler) {
        sendResponse(msg.callId, undefined, `Unknown method: ${msg.method}`);
        return;
      }
      handler(msg.args)
        .then((result) => sendResponse(msg.callId, result))
        .catch((e) => sendResponse(msg.callId, undefined, String(e)));
    },

    getContext() {
      return { api, ui, store };
    },

    destroy() {
      destroyed = true;
    },
  };
}
