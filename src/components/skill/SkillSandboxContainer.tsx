/**
 * SkillSandboxContainer — 统一的 Skill iframe 沙箱容器
 *
 * 替代旧的 SkillReactPage / SkillWebComponentPage / SkillHtmlPage / SkillIframePage。
 * 所有 Skill UI 都在 sandbox iframe 中运行，通过 postMessage RPC 与宿主通信。
 *
 * 安全特性：
 * - iframe sandbox="allow-scripts"（无 same-origin、无表单、无弹窗）
 * - 权限白名单在 Skill 加载时强制执行
 * - 删除 fetch/XHR 防止网络滥用
 * - 来源校验
 *
 * @module components/skill/SkillSandboxContainer
 */

import i18n from "@/i18n";
import { invoke } from "@/lib/invoke";
import { createHostApiBridge, createHostRpcBridge } from "@/sdk/rpcBridge";
import type { HostApiBridge, HostRpcBridge } from "@/sdk/rpcBridge";
import { generateSandboxHtml } from "@/sdk/sandboxTemplate";
import type { SkillHostApi, SkillHostStore, SkillHostUi, SkillPermissionsV2 } from "@/sdk/types";
import { notification, theme as antdTheme } from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { SkillErrorFallback, SkillLoadingSkeleton } from "./SkillErrorFallback";

export interface SkillSandboxContainerProps {
  /** Skill 名称 */
  skillName: string;
  /** Skill 页面/面板 ID */
  componentId: string;
  /** 组件配置（包含 entry 文件路径、props 等） */
  componentConfig: Record<string, unknown>;
  /** 可选：Skill 的 V2 权限声明（从 skill-manifest-v2.json 读取） */
  permissions?: SkillPermissionsV2;
  /** 可选：高度样式 */
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

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const entry = (componentConfig.entry as string) || "index.html";
  const props = (componentConfig.props as Record<string, unknown>) || {};

  // ── 权限合并：默认权限 + 声明的权限 ──

  const effectivePermissions: SkillPermissionsV2 = {
    commands: permissions?.commands ?? [],
    events: permissions?.events ?? [],
    storeRead: permissions?.storeRead ?? [],
    storeWrite: permissions?.storeWrite ?? [],
    navigate: permissions?.navigate ?? [],
    network: permissions?.network ?? [],
    filesystem: permissions?.filesystem,
    tools: permissions?.tools ?? [],
  };

  // ── 宿主 API 实现 ──

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
    notify: (
      message: string,
      type: "info" | "success" | "warning" | "error" = "info",
    ): void => {
      notification[type]({ message, placement: "bottomRight" });
    },
    getTheme: (): "light" | "dark" => {
      // Ant Design 6 主题获取
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
    read: async <T = unknown>(storeName: string, _selector?: string): Promise<T> => {
      const stores = await import("@/stores");
      const storeMap: Record<string, unknown> = {
        preference: stores.usePreferenceStore?.getState(),
        conversation: stores.useConversationStore?.getState(),
        ui: stores.useUIStore?.getState(),
        skill: stores.useSkillStore?.getState(),
      };
      const store = storeMap[storeName];
      if (!store) { throw new Error(`Store "${storeName}" not found`); }
      return store as T;
    },
    write: async (storeName: string, value: unknown): Promise<void> => {
      const stores = await import("@/stores");
      const storeMap: Record<string, { setState: (partial: unknown) => void }> = {
        preference: stores.usePreferenceStore as unknown as { setState: (partial: unknown) => void },
        ui: stores.useUIStore as unknown as { setState: (partial: unknown) => void },
      };
      const store = storeMap[storeName];
      if (!store?.setState) { throw new Error(`Store "${storeName}" is not writable`); }
      store.setState(value);
    },
  };

  // ── 加载 Skill 内容并设置沙箱 ──

  const loadSandbox = useCallback(async () => {
    setError(null);
    setLoading(true);

    try {
      // 读取 Skill 目录下的 HTML 文件
      const htmlContent = await invoke<string>("skill_read_asset", {
        skillName,
        path: entry,
      });

      // 清理旧桥接
      if (bridgeRef.current) { bridgeRef.current.destroy(); }
      if (apiBridgeRef.current) { apiBridgeRef.current.destroy(); }

      // 生成沙箱 HTML
      const sandboxHtml = generateSandboxHtml({
        skillName,
        skillId: `${skillName}:${componentId}`,
        props,
        permissions: effectivePermissions,
        htmlContent,
        devMode: import.meta.env.DEV,
      });

      // 在 HTML 中注入真实值
      const finalHtml = sandboxHtml
        .replace('"__SKILL_NAME__"', JSON.stringify(skillName))
        .replace('"__SKILL_ID__"', JSON.stringify(`${skillName}:${componentId}`))
        .replace("__INITIAL_PROPS__", JSON.stringify(props))
        .replace("__PERMISSIONS__", JSON.stringify(effectivePermissions));

      // 通过 srcdoc 加载
      if (iframeRef.current) {
        iframeRef.current.srcdoc = finalHtml;
      }

      setLoading(false);
    } catch (e) {
      console.error(`[SkillSandbox] Failed to load skill "${skillName}":`, e);
      setError(String(e));
      setLoading(false);
    }
  }, [skillName, componentId, entry, effectivePermissions, props]);

  // ── iframe onLoad — 建立 RPC 桥接 ──

  const handleIframeLoad = useCallback(() => {
    const iframe = iframeRef.current;
    if (!iframe?.contentWindow) { return; }

    // 创建宿主侧 RPC 桥接
    const bridge = createHostRpcBridge(iframe.contentWindow);
    bridgeRef.current = bridge;

    // 创建宿主 API 桥接（处理来自 Skill 的 RPC 请求）
    const apiBridge = createHostApiBridge({
      api: hostApi,
      ui: hostUi,
      store: hostStore,
      permissions: effectivePermissions,
      contentWindow: iframe.contentWindow,
    });
    apiBridgeRef.current = apiBridge;

    // 监听来自 Skill 的消息
    const messageHandler = (event: MessageEvent) => {
      const msg = event.data;
      if (msg?.type === "rpc:request") {
        apiBridge.handleRpcRequest(msg);
      }
    };
    window.addEventListener("message", messageHandler);

    // 发送 mount 生命周期
    bridge.sendLifecycle("mount", props);

    // 清理函数
    return () => {
      window.removeEventListener("message", messageHandler);
      bridge.destroy();
      apiBridge.destroy();
    };
  }, [effectivePermissions, props]);

  // ── 初始加载 ──

  useEffect(() => {
    loadSandbox();
    return () => {
      if (bridgeRef.current) { bridgeRef.current.destroy(); }
      if (apiBridgeRef.current) { apiBridgeRef.current.destroy(); }
    };
  }, [loadSandbox]);

  // ── 渲染 ──

  if (error) {
    return (
      <SkillErrorFallback
        error={error}
        skillName={skillName}
        onRetry={loadSandbox}
      />
    );
  }

  if (loading) {
    return <SkillLoadingSkeleton />;
  }

  return (
    <iframe
      ref={iframeRef}
      onLoad={handleIframeLoad}
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
