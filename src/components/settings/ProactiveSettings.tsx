// SPDX-License-Identifier: AGPL-3.0-only

import { useSettingsStore } from "@/stores";
import { Button, Divider, InputNumber, Space, Switch, Typography } from "antd";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const { Text } = Typography;

/**
 * 主动能力设置 — 暴露两个已定义但此前从未接入 UI 的"幽灵开关"：
 * - proactive_nudge_enabled：基于对话上下文主动给出提示/建议
 * - closed_loop_enabled / closed_loop_interval_minutes：闭环任务的开关与轮询间隔
 */
export function ProactiveSettings() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);

  const proactiveNudge = settings.proactiveNudgeEnabled ?? true;
  const closedLoop = settings.closedLoopEnabled ?? true;
  const closedLoopInterval = settings.closedLoopIntervalMinutes ?? 5;

  const rowStyle = { padding: "4px 0" };

  return (
    <div className="p-6 pb-12">
      <SettingsGroup title={t("settings.proactiveBehavior")}>
        <div
          style={rowStyle}
          className="flex items-center justify-between"
          data-search-key="proactive:nudge"
        >
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            <span>{t("settings.proactiveNudge")}</span>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("settings.proactiveNudgeDesc")}
            </Text>
          </div>
          <Switch
            checked={proactiveNudge}
            onChange={(next) => saveSettings({ proactiveNudgeEnabled: next })}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div
          style={rowStyle}
          className="flex items-center justify-between"
          data-search-key="proactive:closedLoop"
        >
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            <span>{t("settings.closedLoopEnabled")}</span>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("settings.closedLoopEnabledDesc")}
            </Text>
          </div>
          <Switch
            checked={closedLoop}
            onChange={(next) => saveSettings({ closedLoopEnabled: next })}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div
          style={rowStyle}
          className="flex items-center justify-between"
          data-search-key="proactive:interval"
        >
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            <span>{t("settings.closedLoopInterval")}</span>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("settings.closedLoopIntervalDesc")}
            </Text>
          </div>
          <Space.Compact style={{ width: 100 }}>
            <InputNumber
              min={1}
              max={120}
              value={closedLoopInterval}
              onChange={(val) => val != null && saveSettings({ closedLoopIntervalMinutes: val })}
              style={{ width: 72 }}
            />
            <Button disabled>{t("settings.minutes")}</Button>
          </Space.Compact>
        </div>
      </SettingsGroup>
    </div>
  );
}
