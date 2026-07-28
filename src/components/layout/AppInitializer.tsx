// SPDX-License-Identifier: AGPL-3.0-only

import i18n from "@/i18n";
import { invoke, isTauri, logIpcError } from "@/lib/invoke";
import { preloadChatRenderers, preloadCommonPages } from "@/lib/preloadChatRenderers";
import {
  useAppConfigStore,
  useConversationStore,
  useOnboardingStore,
  useSettingsStore,
  useSkillExtensionStore,
  useTabStore,
} from "@/stores";
import { Button, Result, Spin, theme, Typography } from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

export type InitPhase =
  | "idle"
  | "healthCheck"
  | "loadSettings"
  | "applyConfig"
  | "startServices"
  | "ready";

export interface InitPhaseInfo {
  phase: InitPhase;
  error: string | null;
  failed: boolean;
}

interface AppInitializerProps {
  children: React.ReactNode;
}

async function showWindow() {
  try {
    const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    await getCurrentWebviewWindow().show();
  } catch (e) {
    logIpcError("显示窗口")(e);
  }
}

export function AppInitializer({ children }: AppInitializerProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [phase, setPhase] = useState<InitPhase>("idle");
  const [error, setError] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const initRan = useRef(false);

  const runInit = useCallback(async () => {
    if (initRan.current) { return; }
    initRan.current = true;
    setFailed(false);
    setError(null);

    try {
      // healthCheck 与 loadSettings 合并：fetchSettings 成功即代表 IPC 健康。
      // 移除原 2 秒阻塞重试 — IPC 不可用时后续每个调用都有独立 catch 降级，
      // 不应让首屏为单一健康检查等待。
      setPhase("loadSettings");
      const settingsPromise = useSettingsStore.getState().fetchSettings().catch((e) => {
        logIpcError("get_settings")(e);
      });

      // 尽早显示窗口，不等其他初始化完成
      if (isTauri()) {
        void showWindow();
      }

      await settingsPromise;

      setPhase("applyConfig");
      const settings = useSettingsStore.getState().settings;

      // 并行执行无依赖的初始化任务（原为串行 await）
      const parallelTasks: Promise<unknown>[] = [];

      if (isTauri()) {
        parallelTasks.push(
          invoke("apply_startup_settings", {
            alwaysOnTop: settings.always_on_top ?? false,
            closeToTray: settings.minimize_to_tray ?? false,
          }).catch((e) => logIpcError("apply_startup_settings")(e)),
        );

        if (!import.meta.env.DEV) {
          parallelTasks.push(
            (async () => {
              try {
                const { enable, disable } = await import("@tauri-apps/plugin-autostart");
                if (settings.auto_start) {
                  await enable();
                } else {
                  await disable();
                }
              } catch (e) {
                const errorStr = String(e);
                if (!errorStr.includes("os error 2")) {
                  logIpcError("设置自启动")(e);
                }
              }
            })(),
          );
        }
      }

      if (settings.language && i18n.language !== settings.language) {
        parallelTasks.push(i18n.changeLanguage(settings.language));
      }

      // P1-4: 启动时加载 AppConfigStore（model / permissionMode / features 等），
      // 失败不阻塞首屏，降级为默认配置。
      parallelTasks.push(
        useAppConfigStore.getState().loadConfig().catch((e) => {
          logIpcError("appConfigStore: loadConfig")(e);
        }),
      );

      await Promise.all(parallelTasks);

      setPhase("startServices");
      useConversationStore.getState().startStreamListening();
      useSkillExtensionStore.getState().fetchSkills().catch(logIpcError("list_skills"));
      useOnboardingStore.getState().loadFromSettings();

      // P1-6: 启动后加载 conversations 并清理失效 tab（持久化的 conversationId 可能已被删除）
      void (async () => {
        try {
          await useConversationStore.getState().fetchConversations();
          const validIds = new Set(
            useConversationStore.getState().conversations.map((c) => c.id),
          );
          useTabStore.getState().pruneInvalidTabs(validIds);
        } catch (e) {
          logIpcError("fetchConversations + pruneInvalidTabs")(e);
        }
      })();

      // D2（WASM）/ Monaco / 页面预加载改为 fire-and-forget，不阻塞首屏。
      // 这些是重型依赖，idle 时间加载即可，首屏渲染不应等待。
      void enableD2AndPreload();

      setPhase("ready");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setFailed(true);
    }
  }, []);

  useEffect(() => {
    setTimeout(() => runInit(), 0);
  }, [runInit]);

  if (phase === "ready") {
    return <>{children}</>;
  }

  if (failed) {
    return (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          minHeight: "100vh",
          padding: "48px 24px",
          backgroundColor: token.colorBgContainer,
        }}
      >
        <Result
          status="error"
          title={t("appInit.failedTitle")}
          subTitle={error || t("appInit.failedSubtitle")}
          extra={
            <Button
              type="primary"
              onClick={() => {
                initRan.current = false;
                runInit();
              }}
            >
              {t("appInit.retry")}
            </Button>
          }
        />
      </div>
    );
  }

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        minHeight: "100vh",
        backgroundColor: token.colorBgContainer,
      }}
    >
      <Spin size="large" />
      <Text
        type="secondary"
        style={{ marginTop: 16, fontSize: 14 }}
      >
        {t(`appInit.${phase}`)}
      </Text>
    </div>
  );
}

async function enableD2AndPreload() {
  try {
    const { enableD2 } = await import("markstream-react");
    enableD2(() => import("@terrastruct/d2"));
  } catch {
    // D2 may not be available
  }
  void preloadChatRenderers();
  preloadCommonPages();
}
