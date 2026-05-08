/**
 * SkillSandboxContainer — 统一的 Skill iframe 沙箱容器
 *
 * 所有 Skill UI 都在 sandbox iframe 中运行，通过 postMessage RPC 与宿主通信。
 *
 * 安全特性：
 * - iframe sandbox="allow-scripts"（无 same-origin、无表单、无弹窗）
 * - 权限白名单在 Skill 加载时强制执行
 * - 删除 fetch/XHR 防止网络滥用
 */

import i18n from "@/i18n";
import { invoke } from "@/lib/invoke";
import { createHostApiBridge, createHostRpcBridge } from "@/sdk/rpcBridge";
import type { HostApiBridge, HostRpcBridge } from "@/sdk/rpcBridge";
import { generateSandboxHtml } from "@/sdk/sandboxTemplate";
import type { SkillHostApi, SkillHostStore, SkillHostUi, SkillPermissions } from "@/sdk/types";
import { notification, theme as antdTheme } from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { SkillErrorFallback, SkillLoadingSkeleton } from "./SkillErrorFallback";

/** 沙箱加载超时（毫秒） */
const SANDBOX_LOAD_TIMEOUT_MS = 30000;

export interface SkillSandboxContainerProps {
  skillName: string;
  componentId: string;
  componentConfig: Record<string, unknown>;
  permissions?: SkillPermissions;
  style?: React.CSSProperties;
}

export function SkillSandboxContainer({
  skillName,
  componentId,
  componentConfig,
  permissions,
  style,
}: SkillSandboxContainerProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const bridgeRef = useRef<HostRpcBridge | null>(null);
  const apiBridgeRef = useRef<HostApiBridge | null>(null);
  const loadTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const retryCountRef = useRef(0);

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const { token: themeToken } = antdTheme.useToken();
  const currentTheme: "light" | "dark" = themeToken.colorBgBase === "#ffffff" ? "light" : "dark";

  const entry = (componentConfig.entry as string) || "index.html";
  const props = (componentConfig.props as Record<string, unknown>) || {};

  const effectivePermissions: SkillPermissions = {
    commands: permissions?.commands ?? [],
    events: permissions?.events ?? [],
    storeRead: permissions?.storeRead ?? [],
    storeWrite: permissions?.storeWrite ?? [],
    navigate: permissions?.navigate ?? [],
    network: permissions?.network ?? [],
    filesystem: permissions?.filesystem,
    tools: permissions?.tools ?? [],
  };

  const hostApi: SkillHostApi = {
    invoke: async <T = unknown>(command: string, args?: Record<string, unknown>): Promise<T> => {
      return invoke<T>(command, args || {});
    },
    emit: (event: string, payload?: unknown): void => {
      window.dispatchEvent(new CustomEvent(`skill:${skillName}:${event}`, { detail: payload }));
    },
  };

  const hostUi: SkillHostUi = {
    navigate: (path: string): void => {
      window.location.hash = path;
    },
    notify: (message: string, type: "info" | "success" | "warning" | "error" = "info"): void => {
      notification[type]({ message, placement: "bottomRight" });
    },
    getTheme: (): "light" | "dark" => {
      try {
        const token = antdTheme.getDesignToken?.();
        return token?.colorBgBase === "#ffffff" ? "light" : "dark";
      } catch {
        return "light";
      }
    },
    getLocale: (): string => {
      return i18n.language || "zh-CN";
    },
  };

  const hostStore: SkillHostStore = {
    read: async <T = unknown>(storeName: string, selector?: string): Promise<T> => {
      const { getStoreRegistry, initStoreRegistry } = await import("@/lib/storeRegistry");
      await initStoreRegistry();
      const registry = getStoreRegistry();
      const accessor = registry.get(storeName);
      if (!accessor) { throw new Error(`Store "${storeName}" 未找到`); }
      const state = accessor.get() as Record<string, unknown>;
      if (selector) {
        const parts = selector.split(".");
        let result: unknown = state;
        for (const part of parts) {
          if (result && typeof result === "object" && part in (result as Record<string, unknown>)) {
            result = (result as Record<string, unknown>)[part];
          } else {
            return undefined as T;
          }
        }
        return structuredClone(result) as T;
      }
      return structuredClone(state) as T;
    },
    write: async (storeName: string, value: unknown, selector?: string): Promise<void> => {
      const { getStoreRegistry, initStoreRegistry } = await import("@/lib/storeRegistry");
      await initStoreRegistry();
      const registry = getStoreRegistry();
      const accessor = registry.get(storeName);
      if (!accessor) { throw new Error(`Store "${storeName}" 不可写`); }
      if (selector && typeof value === "object" && value !== null) {
        const partial: Record<string, unknown> = {};
        const parts = selector.split(".");
        let current = partial;
        for (let i = 0; i < parts.length - 1; i++) {
          current[parts[i]] = {};
          current = current[parts[i]] as Record<string, unknown>;
        }
        current[parts[parts.length - 1]] = value;
        accessor.set(partial);
      } else {
        accessor.set(value);
      }
    },
  };

  const loadSandbox = useCallback(async () => {
    setError(null);
    setLoading(true);

    // 加载超时保护
    if (loadTimerRef.current) { clearTimeout(loadTimerRef.current); }
    loadTimerRef.current = setTimeout(() => {
      setError("沙箱加载超时，请重试");
      setLoading(false);
    }, SANDBOX_LOAD_TIMEOUT_MS);

    try {
      const htmlContent = await invoke<string>("skill_read_asset", {
        skillName,
        path: entry,
      });

      if (!htmlContent || htmlContent.trim().length === 0) {
        throw new Error(`入口文件 "${entry}" 为空或不存在`);
      }

      if (bridgeRef.current) { bridgeRef.current.destroy(); }
      if (apiBridgeRef.current) { apiBridgeRef.current.destroy(); }

      const skillId = `${skillName}:${componentId}`;
      const propsJson = JSON.stringify(props);
      const permsJson = JSON.stringify(effectivePermissions);

      const sandboxHtml = generateSandboxHtml({
        skillName,
        skillId,
        props,
        permissions: effectivePermissions,
        htmlContent,
        devMode: import.meta.env.DEV,
      });

      const finalHtml = sandboxHtml
        .replace('"__SKILL_NAME__"', JSON.stringify(skillName))
        .replace('"__SKILL_ID__"', JSON.stringify(skillId))
        .replace("__INITIAL_PROPS__", propsJson)
        .replace("__PERMISSIONS__", permsJson);

      // 先建立 message 监听器（防止 skill:ready 竞态），再设置 srcdoc
      const iframe = iframeRef.current;
      if (!iframe) {
        throw new Error("iframe ref 未就绪");
      }

      // 注册 message 监听器
      const messageHandler = (event: MessageEvent) => {
        const msg = event.data;
        if (msg?.type === "skill:ready") {
          // iframe 加载完成，发送 mount 生命周期
          const bridge = createHostRpcBridge(iframe.contentWindow!);
          bridgeRef.current = bridge;

          const apiBridge = createHostApiBridge({
            api: hostApi,
            ui: hostUi,
            store: hostStore,
            permissions: effectivePermissions,
            contentWindow: iframe.contentWindow!,
          });
          apiBridgeRef.current = apiBridge;

          bridge.sendLifecycle("mount", props);
          bridge.emitEvent("theme-change", { theme: hostUi.getTheme() });

          if (loadTimerRef.current) {
            clearTimeout(loadTimerRef.current);
            loadTimerRef.current = null;
          }
          setLoading(false);
          retryCountRef.current = 0;

          // 切换到处理 rpc:request 的模式
          window.removeEventListener("message", messageHandler);
          window.addEventListener("message", handleRpc);
        } else if (msg?.type === "skill:error") {
          console.error(`[SkillSandbox] Skill "${skillName}" 运行时错误:`, msg.error);
        }
      };

      const handleRpc = (event: MessageEvent) => {
        const msg = event.data;
        if (msg?.type === "rpc:request" && apiBridgeRef.current) {
          apiBridgeRef.current.handleRpcRequest(msg);
        } else if (msg?.type === "skill:error") {
          console.error(`[SkillSandbox] Skill "${skillName}" 运行时错误:`, msg.error);
        }
      };

      window.addEventListener("message", messageHandler);
      iframe.srcdoc = finalHtml;
    } catch (e) {
      console.error(`[SkillSandbox] 加载 Skill "${skillName}" 失败:`, e);
      setError(String(e));
      setLoading(false);
      if (loadTimerRef.current) {
        clearTimeout(loadTimerRef.current);
        loadTimerRef.current = null;
      }
    }
  }, [skillName, componentId, entry, effectivePermissions, props, hostApi, hostUi, hostStore]);

  useEffect(() => {
    loadSandbox();
    return () => {
      if (bridgeRef.current) { bridgeRef.current.destroy(); }
      if (apiBridgeRef.current) { apiBridgeRef.current.destroy(); }
      if (loadTimerRef.current) { clearTimeout(loadTimerRef.current); }
    };
  }, [loadSandbox]);

  useEffect(() => {
    if (bridgeRef.current) {
      bridgeRef.current.emitEvent("theme-change", { theme: currentTheme });
    }
  }, [currentTheme]);

  if (error) {
    return (
      <SkillErrorFallback
        error={error}
        skillName={skillName}
        onRetry={() => {
          retryCountRef.current += 1;
          loadSandbox();
        }}
      />
    );
  }

  if (loading) {
    return <SkillLoadingSkeleton />;
  }

  return (
    <iframe
      ref={iframeRef}
      title={`Skill: ${skillName}`}
      sandbox="allow-scripts"
      style={{
        width: "100%",
        height: "100%",
        minHeight: 400,
        border: "none",
        backgroundColor: "transparent",
        ...style,
      }}
    />
  );
}
