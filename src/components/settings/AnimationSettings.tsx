// SPDX-License-Identifier: AGPL-3.0-only

import { type AnimationMode, useAnimationStore } from "@/stores";
import { Alert, Radio, theme, Typography } from "antd";
import { lazy, Suspense, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const { Text } = Typography;

/**
 * 动画设置面板 —— 控制界面动画效果的加载与播放方式。
 *
 * 三态模式：
 * - "system"（默认）：跟随系统 prefers-reduced-motion
 * - "on"：始终启用动画
 * - "off"：始终禁用动画（动画组件 lazy 不加载，预览区退化为静态文本）
 */

/** 动态加载的动画组件 —— 仅在动画启用时才会被请求（代码分割 + 按需加载） */
const LazyAnimatedPreview = lazy(() =>
  import("@/components/bits/AnimatedPreview").then((m) => ({
    default: m.AnimatedPreview,
  }))
);

function ModeOptionLabel({ text }: { text: string }) {
  return <span className="whitespace-nowrap">{text}</span>;
}

export function AnimationSettings() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const mode = useAnimationStore((s) => s.mode);
  const setMode = useAnimationStore((s) => s.setMode);
  const systemPrefersReducedMotion = useAnimationStore(
    (s) => s.systemPrefersReducedMotion,
  );
  const animationEnabled = useAnimationStore((s) => s.isAnimationEnabled());

  const systemStatusText = useMemo(() => {
    if (mode !== "system") {
      return null;
    }
    return systemPrefersReducedMotion
      ? t("settings.animations.systemReducedOn")
      : t("settings.animations.systemReducedOff");
  }, [mode, systemPrefersReducedMotion, t]);

  const effectiveText = animationEnabled
    ? t("settings.animations.effectiveEnabled")
    : t("settings.animations.effectiveDisabled");

  return (
    <div className="p-6 pb-12">
      <SettingsGroup title={t("settings.animations.title")}>
        <div
          className="flex items-center justify-between"
          style={{ padding: "4px 0" }}
          data-search-key="animations:mode"
        >
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            <span>{t("settings.animations.modeLabel")}</span>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("settings.animations.modeDesc")}
            </Text>
          </div>
          <Radio.Group
            value={mode}
            onChange={(e) => setMode(e.target.value as AnimationMode)}
            size="small"
            optionType="button"
            buttonStyle="solid"
          >
            <Radio.Button value="system">
              <ModeOptionLabel text={t("settings.animations.followSystem")} />
            </Radio.Button>
            <Radio.Button value="on">
              <ModeOptionLabel text={t("settings.animations.enabled")} />
            </Radio.Button>
            <Radio.Button value="off">
              <ModeOptionLabel text={t("settings.animations.disabled")} />
            </Radio.Button>
          </Radio.Group>
        </div>

        {systemStatusText && (
          <div style={{ marginTop: 8 }}>
            <Alert
              type={systemPrefersReducedMotion ? "info" : "success"}
              showIcon
              message={systemStatusText}
              style={{ fontSize: 12 }}
            />
          </div>
        )}

        <div style={{ marginTop: 12 }}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {effectiveText}
          </Text>
        </div>
      </SettingsGroup>

      <SettingsGroup title={t("settings.animations.previewTitle")}>
        <div
          style={{
            padding: "20px 16px",
            borderRadius: 8,
            textAlign: "center",
            backgroundColor: token.colorFillTertiary,
          }}
          data-search-key="animations:preview"
        >
          {animationEnabled
            ? (
              <Suspense fallback={<span style={{ opacity: 0.5 }}>…</span>}>
                <LazyAnimatedPreview text={t("settings.animations.previewText")} />
              </Suspense>
            )
            : (
              <span style={{ fontSize: 16, opacity: 0.7 }}>
                {t("settings.animations.previewText")}
              </span>
            )}
        </div>
      </SettingsGroup>
    </div>
  );
}
