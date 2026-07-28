// SPDX-License-Identifier: AGPL-3.0-only

/**
 * StrategyRoomMiniBar — 策略室策略列表 mini 条。
 *
 * 仅在「投研办公室」场景（investment_office）下显示，置于 Phaser 画布顶部、
 * 其他房间 mini 条之上。
 *
 * 数据源：strategyStore（quant） — 拉取已注册策略列表。
 * 每条策略展示：name / version / strategyType / walkForwardEnabled / 更新时间。
 * 支持折叠 / 刷新 / 跳转到 quant 页面。
 */

import { useStrategyStore } from "@/stores";
import type { StrategyMeta, StrategyType } from "@/types";
import { Button, Empty, Spin, Tag, theme, Tooltip, Typography } from "antd";
import { ChevronRight, RefreshCw, Target } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

const { Text } = Typography;

/** 策略类型 → Tag color */
function strategyTypeColor(t: StrategyType | string): string {
  switch (t) {
    case "trend":
      return "blue";
    case "mean_reversion":
      return "green";
    case "momentum":
      return "orange";
    case "value":
      return "purple";
    case "rhai":
      return "magenta";
    case "custom":
      return "gold";
    default:
      return "default";
  }
}

export interface StrategyRoomMiniBarProps {
  sceneTemplateSlug?: string;
}

export function StrategyRoomMiniBar({ sceneTemplateSlug }: StrategyRoomMiniBarProps) {
  // === 所有 hook 前先做条件返回，遵守 Rules of Hooks ===
  if (sceneTemplateSlug !== "investment_office") {
    return null;
  }

  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();

  const strategies = useStrategyStore((s) => s.strategies);
  const isLoading = useStrategyStore((s) => s.isLoading);
  const error = useStrategyStore((s) => s.error);
  const loadStrategies = useStrategyStore((s) => s.loadStrategies);

  const [collapsed, setCollapsed] = useState(false);

  // 首次挂载拉取
  useEffect(() => {
    void loadStrategies(false);
  }, [loadStrategies]);

  const handleRefresh = () => {
    void loadStrategies(true);
  };

  const handleJumpToQuant = () => {
    void navigate("/quant");
  };

  const wfEnabledCount = strategies.filter((s) => s.walkForwardEnabled).length;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 4,
        padding: "6px 8px",
        background: token.colorBgLayout,
        borderRadius: 6,
        border: `1px solid ${token.colorBorderSecondary}`,
        fontSize: 12,
      }}
    >
      {/* 标题栏 */}
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <Target size={14} color={token.colorPrimary} />
        <Text strong style={{ fontSize: 12 }}>
          {t("office.roomMiniBar.strategy.title")}
        </Text>
        <Tag
          color="default"
          style={{ fontSize: 10, margin: 0, padding: "0 4px", lineHeight: "16px" }}
        >
          {t("office.roomMiniBar.strategy.total", { count: strategies.length })}
        </Tag>
        {wfEnabledCount > 0 && (
          <Tag
            color="green"
            style={{ fontSize: 10, margin: 0, padding: "0 4px", lineHeight: "16px" }}
          >
            {t("office.roomMiniBar.strategy.wfCount", { count: wfEnabledCount })}
          </Tag>
        )}

        <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 4 }}>
          <Tooltip title={t("office.roomMiniBar.strategy.jumpQuant")}>
            <Button
              size="small"
              type="text"
              icon={<ChevronRight size={12} />}
              onClick={handleJumpToQuant}
              style={{ padding: "0 4px" }}
            />
          </Tooltip>
          <Tooltip title={t("office.roomMiniBar.strategy.refresh")}>
            <Button
              size="small"
              type="text"
              icon={<RefreshCw size={12} />}
              loading={isLoading}
              onClick={handleRefresh}
              style={{ padding: "0 4px" }}
            />
          </Tooltip>
          <Tooltip
            title={collapsed
              ? t("office.roomMiniBar.strategy.expand")
              : t("office.roomMiniBar.strategy.collapse")}
          >
            <Button
              size="small"
              type="text"
              onClick={() => setCollapsed((v) => !v)}
              style={{ padding: "0 4px", fontSize: 11 }}
            >
              {collapsed ? "▾" : "▴"}
            </Button>
          </Tooltip>
        </div>
      </div>

      {/* 内容区 */}
      {!collapsed && (
        <>
          {isLoading && strategies.length === 0
            ? (
              <div style={{ textAlign: "center", padding: 12 }}>
                <Spin size="small" />
              </div>
            )
            : strategies.length === 0
            ? (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={error ?? t("office.roomMiniBar.strategy.empty")}
                styles={{ description: { fontSize: 11, color: token.colorTextQuaternary } }}
                style={{ margin: "8px 0" }}
              />
            )
            : (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: 3,
                  maxHeight: 160,
                  overflowY: "auto",
                  paddingRight: 4,
                }}
              >
                {strategies.slice(0, 10).map((s) => <StrategyItem key={s.id} strategy={s} />)}
                {strategies.length > 10 && (
                  <Text
                    type="secondary"
                    style={{ fontSize: 10, textAlign: "center", paddingTop: 4 }}
                  >
                    {t("office.roomMiniBar.strategy.moreCount", { count: strategies.length - 10 })}
                  </Text>
                )}
              </div>
            )}
        </>
      )}
    </div>
  );
}

function StrategyItem({ strategy }: { strategy: StrategyMeta }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const updatedStr = strategy.updatedAt
    ? new Date(strategy.updatedAt).toLocaleDateString()
    : "—";

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 6,
        padding: "4px 8px",
        background: token.colorBgContainer,
        borderRadius: 4,
        border: `1px solid ${token.colorBorderSecondary}`,
        fontSize: 11,
      }}
    >
      <Text strong style={{ fontSize: 12, color: token.colorText, flex: "0 1 auto" }}>
        {strategy.name}
      </Text>
      <Tag
        color={strategyTypeColor(strategy.strategyType)}
        style={{ fontSize: 9, margin: 0, padding: "0 4px", lineHeight: "16px" }}
      >
        {strategy.strategyType}
      </Tag>
      <Text type="secondary" style={{ fontSize: 10 }}>
        v{strategy.version}
      </Text>
      {strategy.walkForwardEnabled && (
        <Tag
          color="green"
          style={{ fontSize: 9, margin: 0, padding: "0 4px", lineHeight: "16px" }}
        >
          {t("office.roomMiniBar.strategy.wfEnabled")}
        </Tag>
      )}
      <Text type="secondary" style={{ fontSize: 10, marginLeft: "auto" }}>
        {updatedStr}
      </Text>
    </div>
  );
}
