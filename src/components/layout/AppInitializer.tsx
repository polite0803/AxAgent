import i18n from "@/i18n";
import { checkIpcHealth, invoke, isTauri, logIpcError } from "@/lib/invoke";
import { preloadChatRenderers, preloadCommonPages } from "@/lib/preloadChatRenderers";
import { useConversationStore, useOnboardingStore, useSettingsStore, useSkillExtensionStore } from "@/stores";
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
      setPhase("healthCheck");
      if (isTauri()) {
        const health = await checkIpcHealth();
        if (!health.ok) {
          logIpcError("IPC 健康检查")(health.detail);
          await new Promise((r) => setTimeout(r, 2000));
          const retry = await checkIpcHealth();
          if (!retry.ok) {
            logIpcError("IPC 重试")(retry.detail);
          }
        }
      }

      setPhase("loadSettings");
      try {
        await useSettingsStore.getState().fetchSettings();
      } catch (e) {
        logIpcError("get_settings")(e);
      }

      setPhase("applyConfig");
      if (isTauri()) {
        const settings = useSettingsStore.getState().settings;
        try {
          await invoke("apply_startup_settings", {
            alwaysOnTop: settings.always_on_top ?? false,
            closeToTray: settings.minimize_to_tray ?? false,
          });
        } catch (e) {
          logIpcError("apply_startup_settings")(e);
        }

        if (!import.meta.env.DEV) {
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
        }
      }

      setPhase("startServices");
      const settings = useSettingsStore.getState().settings;

      if (settings.language && i18n.language !== settings.language) {
        await i18n.changeLanguage(settings.language);
      }

      useConversationStore.getState().startStreamListening();

      useSkillExtensionStore.getState().fetchSkills().catch(logIpcError("list_skills"));

      useOnboardingStore.getState().loadFromSettings();

      await enableD2AndPreload();

      if (isTauri()) {
        await showWindow();
      }

      setPhase("ready");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setFailed(true);
    }
  }, []);

  useEffect(() => {
    runInit();
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
  } catch {}
  void preloadChatRenderers();
  preloadCommonPages();
}
