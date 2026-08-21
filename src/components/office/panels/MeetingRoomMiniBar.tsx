// i18n-exempt: 业务逻辑/格式化/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * MeetingRoomMiniBar — 会议室晨会议题 mini 条。
 *
 * 仅在「投研办公室」场景（investment_office）下显示，置于 Phaser 画布顶部、
 * TradingRoomMiniBar 之上（堆叠时按渲染顺序排列）。
 *
 * 数据源：marketMainlineStore — 拉取最近 7 天主线，默认展示今日（或最近一日）
 * 主线列表。每条主线显示：theme / strengthScore / persistence / themeCategory /
 * 代表性标的 + narrative 摘要。
 *
 * 交互：
 *   - 手动刷新按钮
 *   - 切换日期（前一日 / 后一日，限定最近 7 天）
 *   - 折叠 / 展开议题列表（默认展开）
 */

import { useMarketMainlineStore } from "@/stores";
import type { MarketMainline } from "@/types";
import { parseRepresentativeSymbols } from "@/types/market-mainline";
import { Button, Empty, Spin, Tag, theme, Tooltip, Typography } from "antd";
import { CalendarDays, ChevronLeft, ChevronRight, Clock, RefreshCw } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/** 持久性 → 颜色 */
function persistenceColor(p: string, token: ReturnType<typeof theme.useToken>["token"]): string {
  if (p === "emerging") { return token.colorSuccess; }
  if (p === "fading") { return token.colorWarning; }
  if (p === "1d") { return token.colorTextTertiary; }
  return token.colorPrimary; // 1w / 1m / 默认
}

/** 强度评分 → 颜色 */
function strengthColor(score: number, token: ReturnType<typeof theme.useToken>["token"]): string {
  if (score >= 75) { return token.colorError; }
  if (score >= 50) { return token.colorWarning; }
  return token.colorTextSecondary;
}

/** 主题大类 → Tag color */
function categoryTagColor(cat: string): string {
  switch (cat) {
    case "科技":
      return "geekblue";
    case "消费":
      return "magenta";
    case "周期":
      return "orange";
    case "金融":
      return "gold";
    case "医药":
      return "green";
    case "政策":
      return "purple";
    default:
      return "default";
  }
}

/** 格式化日期为「MM-DD」 */
function shortDate(d: string): string {
  if (!d) { return "—"; }
  const m = d.match(/^\d{4}-(\d{2})-(\d{2})$/);
  return m ? `${m[1]}-${m[2]}` : d;
}

/** 今日 YYYY-MM-DD（本地时区） */
function today(): string {
  const d = new Date();
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

export interface MeetingRoomMiniBarProps {
  sceneTemplateSlug?: string;
}

export function MeetingRoomMiniBar({ sceneTemplateSlug }: MeetingRoomMiniBarProps) {
  // === 所有 hook 前先做条件返回，遵守 Rules of Hooks ===
  if (sceneTemplateSlug !== "investment_office") {
    return null;
  }

  const { t } = useTranslation();
  const { token } = theme.useToken();

  const recentMainlines = useMarketMainlineStore((s) => s.recentMainlines);
  const loadingRecent = useMarketMainlineStore((s) => s.loadingRecent);
  const error = useMarketMainlineStore((s) => s.error);
  const fetchRecentMainlines = useMarketMainlineStore((s) => s.fetchRecentMainlines);
  const fetchMainlinesByDate = useMarketMainlineStore((s) => s.fetchMainlinesByDate);
  const dateMainlines = useMarketMainlineStore((s) => s.dateMainlines);

  const [collapsed, setCollapsed] = useState(false);
  const [selectedDate, setSelectedDate] = useState<string>("");

  // 拉取最近 7 天主线（首次挂载）
  useEffect(() => {
    void fetchRecentMainlines(7);
  }, [fetchRecentMainlines]);

  // 选中的日期：默认今日（若今日无主线则取最近一日）
  const availableDates = useMemo(() => {
    const dates = Array.from(new Set(recentMainlines.map((m) => m.mainlineDate)));
    return dates.sort((a, b) => (a < b ? 1 : -1)); // 倒序
  }, [recentMainlines]);

  useEffect(() => {
    if (!selectedDate && availableDates.length > 0) {
      const todayStr = today();
      // 优先选中今日，否则选最近一日
      const target = availableDates.includes(todayStr) ? todayStr : availableDates[0];
      setSelectedDate(target);
    }
  }, [availableDates, selectedDate]);

  // 切换日期时拉取当日主线
  useEffect(() => {
    if (selectedDate) {
      void fetchMainlinesByDate(selectedDate);
    }
  }, [selectedDate, fetchMainlinesByDate]);

  // 当前展示的主线列表（优先用 dateMainlines，回退到 recentMainlines 过滤）
  const displayedMainlines: MarketMainline[] = useMemo(() => {
    if (dateMainlines.length > 0 && dateMainlines[0]?.mainlineDate === selectedDate) {
      return dateMainlines;
    }
    return recentMainlines.filter((m) => m.mainlineDate === selectedDate);
  }, [dateMainlines, recentMainlines, selectedDate]);

  const handleRefresh = () => {
    void fetchRecentMainlines(7);
    if (selectedDate) {
      void fetchMainlinesByDate(selectedDate);
    }
  };

  const handlePrevDay = () => {
    const idx = availableDates.indexOf(selectedDate);
    if (idx >= 0 && idx < availableDates.length - 1) {
      setSelectedDate(availableDates[idx + 1]);
    }
  };

  const handleNextDay = () => {
    const idx = availableDates.indexOf(selectedDate);
    if (idx > 0) {
      setSelectedDate(availableDates[idx - 1]);
    }
  };

  const isToday = selectedDate === today();
  const activeCount = displayedMainlines.filter((m) => m.status === "active").length;

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
        <CalendarDays size={14} color={token.colorPrimary} />
        <Text strong style={{ fontSize: 12 }}>
          {t("office.roomMiniBar.meeting.title")}
        </Text>
        <Tag
          color={isToday ? "green" : "default"}
          style={{ fontSize: 10, margin: 0, padding: "0 4px", lineHeight: "16px" }}
        >
          {isToday
            ? t("office.roomMiniBar.meeting.today")
            : shortDate(selectedDate)}
        </Tag>
        {displayedMainlines.length > 0 && (
          <Text type="secondary" style={{ fontSize: 10 }}>
            {t("office.roomMiniBar.meeting.activeCount", { count: activeCount })}
          </Text>
        )}

        <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 4 }}>
          {/* 日期切换 */}
          <Tooltip title={t("office.roomMiniBar.meeting.prevDay")}>
            <Button
              size="small"
              type="text"
              icon={<ChevronLeft size={12} />}
              onClick={handlePrevDay}
              disabled={availableDates.length === 0 || availableDates.indexOf(selectedDate)
                  === availableDates.length - 1}
              style={{ padding: "0 4px" }}
            />
          </Tooltip>
          <Tooltip title={t("office.roomMiniBar.meeting.nextDay")}>
            <Button
              size="small"
              type="text"
              icon={<ChevronRight size={12} />}
              onClick={handleNextDay}
              disabled={availableDates.length === 0 || availableDates.indexOf(selectedDate) === 0}
              style={{ padding: "0 4px" }}
            />
          </Tooltip>
          <Tooltip title={t("office.roomMiniBar.meeting.refresh")}>
            <Button
              size="small"
              type="text"
              icon={<RefreshCw size={12} />}
              loading={loadingRecent}
              onClick={handleRefresh}
              style={{ padding: "0 4px" }}
            />
          </Tooltip>
          <Tooltip
            title={collapsed
              ? t("office.roomMiniBar.meeting.expand")
              : t("office.roomMiniBar.meeting.collapse")}
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
          {loadingRecent && displayedMainlines.length === 0
            ? (
              <div style={{ textAlign: "center", padding: 12 }}>
                <Spin size="small" />
              </div>
            )
            : displayedMainlines.length === 0
            ? (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={error ?? t("office.roomMiniBar.meeting.empty")}
                styles={{ description: { fontSize: 11, color: token.colorTextQuaternary } }}
                style={{ margin: "8px 0" }}
              />
            )
            : (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: 4,
                  maxHeight: 180,
                  overflowY: "auto",
                  paddingRight: 4,
                }}
              >
                {displayedMainlines.map((m) => <MainlineItem key={m.id} mainline={m} />)}
              </div>
            )}
        </>
      )}
    </div>
  );
}

function MainlineItem({ mainline }: { mainline: MarketMainline }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const symbols = parseRepresentativeSymbols(mainline.representativeSymbols);

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 3,
        padding: "6px 8px",
        background: token.colorBgContainer,
        borderRadius: 4,
        border: `1px solid ${token.colorBorderSecondary}`,
        fontSize: 11,
      }}
    >
      {/* 第一行：主题 + 评分 + 标签 */}
      <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
        <Text strong style={{ fontSize: 12, color: token.colorText }}>
          {mainline.theme}
        </Text>
        <Tag
          color={categoryTagColor(mainline.themeCategory)}
          style={{ fontSize: 9, margin: 0, padding: "0 4px", lineHeight: "16px" }}
        >
          {mainline.themeCategory}
        </Tag>
        <span
          style={{
            fontSize: 10,
            fontWeight: 700,
            color: strengthColor(mainline.strengthScore, token),
            fontFamily: "ui-monospace, monospace",
          }}
        >
          {t("office.roomMiniBar.meeting.strength")}: {mainline.strengthScore}
        </span>
        <Tag
          style={{
            fontSize: 9,
            margin: 0,
            padding: "0 4px",
            lineHeight: "16px",
            color: persistenceColor(mainline.persistence, token),
            borderColor: persistenceColor(mainline.persistence, token),
          }}
        >
          {mainline.persistence}
        </Tag>
        {mainline.status !== "active" && (
          <Tag
            color={mainline.status === "fading" ? "orange" : "default"}
            style={{ fontSize: 9, margin: 0, padding: "0 4px", lineHeight: "16px" }}
          >
            {mainline.status}
          </Tag>
        )}
      </div>

      {/* 第二行：narrative */}
      {mainline.narrative && (
        <Text
          type="secondary"
          style={{
            fontSize: 11,
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
            color: token.colorTextSecondary,
          }}
        >
          {mainline.narrative}
        </Text>
      )}

      {/* 第三行：代表性标的 */}
      {symbols.length > 0 && (
        <div style={{ display: "flex", alignItems: "center", gap: 4, flexWrap: "wrap" }}>
          <Text type="secondary" style={{ fontSize: 10 }}>
            {t("office.roomMiniBar.meeting.representative")}:
          </Text>
          {symbols.slice(0, 6).map((code) => (
            <Tag
              key={code}
              style={{ fontSize: 9, margin: 0, padding: "0 4px", lineHeight: "16px" }}
            >
              <code style={{ fontFamily: "ui-monospace, monospace" }}>{code}</code>
            </Tag>
          ))}
          {symbols.length > 6 && (
            <Text type="secondary" style={{ fontSize: 10 }}>
              +{symbols.length - 6}
            </Text>
          )}
        </div>
      )}

      {/* 时间戳 */}
      <div style={{ display: "flex", alignItems: "center", gap: 4, color: token.colorTextQuaternary }}>
        <Clock size={10} />
        <span style={{ fontSize: 10 }}>
          {new Date(mainline.updatedAt).toLocaleString()}
        </span>
      </div>
    </div>
  );
}
