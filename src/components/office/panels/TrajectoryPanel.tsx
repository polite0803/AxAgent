// SPDX-License-Identifier: AGPL-3.0-only

/**
 * TrajectoryPanel — agent 协作轨迹 + 股票业务时间线合并面板。
 *
 * 数据来源（按时间顺序合并展示）：
 * 1. stockAnalysisStore.timeline — 股票工作流的业务节点时间线
 *    （scan / diagnose / debate / decide 四阶段，反映分析进度）
 * 2. officeStore.dispatchEvents — dispatcher 的协作事件流
 *    （routing / process / agent_status / token_usage）
 *
 * 合并后投研团队可在办公室内一站式查看：
 * - 当前股票分析推进到哪个阶段、哪些节点已完成/失败
 * - dispatcher 同时在做什么路由决策、token 消耗
 *
 * 业务节点支持点击跳转：若 evidenceRefs 指向 market/analyze/execute 侧栏面板，
 * 点击节点会触发 onNavigateToStock(tabKey, panelKey) 回调（可选）。
 */

import { useOfficeStore, useStockAnalysisStore } from "@/stores";
import type { DispatchEvent } from "@/types";
import type { TimelineNode, TimelinePhase } from "@/types/stock-analysis";
import { Empty, Tag, theme, Timeline } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

export interface TrajectoryPanelProps {
  /** 用户点击业务节点跳转到股票分析对应面板 */
  onNavigateToStock?: (
    tabKey: "market" | "analyze" | "execute",
    panelKey: string,
  ) => void;
}

/** 合并后的时间线条目 */
type MergedEntry =
  | {
    kind: "stock";
    ts: number;
    node: TimelineNode;
  }
  | {
    kind: "dispatch";
    ts: number;
    event: DispatchEvent;
  };

const PHASE_COLOR: Record<TimelinePhase, string> = {
  scan: "cyan",
  diagnose: "blue",
  debate: "magenta",
  decide: "gold",
};

const PHASE_LABEL_KEY: Record<TimelinePhase, string> = {
  scan: "stockAnalysis.timeline.phase.scan",
  diagnose: "stockAnalysis.timeline.phase.diagnose",
  debate: "stockAnalysis.timeline.phase.debate",
  decide: "stockAnalysis.timeline.phase.decide",
};

const PHASE_DOT_COLOR: Record<TimelinePhase, string> = {
  scan: "#13c2c2",
  diagnose: "#1677ff",
  debate: "#eb2f96",
  decide: "#fa8c16",
};

/** dispatchEvent 时间戳推断（事件本身无 ts 字段时用回退顺序） */
function dispatchTs(event: DispatchEvent, fallback: number): number {
  // DispatchEvent 当前没有显式 timestamp 字段；用 fallback（数组下标倒推）保持稳定排序
  const anyEvent = event as unknown as { timestamp?: number; ts?: number };
  if (typeof anyEvent.timestamp === "number") { return anyEvent.timestamp; }
  if (typeof anyEvent.ts === "number") { return anyEvent.ts; }
  return fallback;
}

/** 业务节点状态对应颜色 */
function nodeStatusColor(status: TimelineNode["status"]): string {
  switch (status) {
    case "done":
      return "green";
    case "failed":
      return "red";
    case "running":
      return "blue";
    case "pending":
      return "default";
    default:
      return "default";
  }
}

export function TrajectoryPanel({ onNavigateToStock }: TrajectoryPanelProps) {
  const { t } = useTranslation();
  const { token: themeToken } = theme.useToken();
  const dispatchEvents = useOfficeStore((s) => s.dispatchEvents);
  const timeline = useStockAnalysisStore((s) => s.timeline);
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const analysisStatus = useStockAnalysisStore((s) => s.status);
  const [hideDispatch, setHideDispatch] = useState(false);

  // 进入面板时若 timeline 为空但分析状态在 running/loading，说明跨页面切换丢失，
  // 这里不主动拉取（避免与办公室 tab 冲突），只展示当前 store 状态。

  // 合并：先业务节点（按 finishedAt/startedAt 排序），再 dispatch 事件（按到达顺序）
  const merged: MergedEntry[] = useMemo(() => {
    const stockEntries: MergedEntry[] = timeline.map((node, idx) => ({
      kind: "stock" as const,
      ts: node.finishedAt ?? node.startedAt ?? Date.now() - (timeline.length - idx) * 1000,
      node,
    }));
    const dispatchEntries: MergedEntry[] = dispatchEvents
      .filter((e) =>
        e.type === "routing" || e.type === "process" || e.type === "agent_status"
        || e.type === "token_usage"
      )
      .map((event, idx) => ({
        kind: "dispatch" as const,
        ts: dispatchTs(event, Date.now() - (dispatchEvents.length - idx) * 500),
        event,
      } as MergedEntry));

    const combined = [...stockEntries, ...dispatchEntries];
    // 按 ts 升序合并（早→晚），保证业务节点与协作事件交错呈现真实时序
    combined.sort((a, b) => a.ts - b.ts);
    return combined;
  }, [timeline, dispatchEvents]);

  // 单独统计业务节点数（用于 header 显示）
  const stockCount = timeline.length;
  const dispatchCount = merged.filter((m) => m.kind === "dispatch").length;

  // 没有任何数据时显示空态
  if (merged.length === 0) {
    return (
      <div style={{ padding: 24, height: "100%" }}>
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("office.trajectory.empty")}
          styles={{ description: { fontSize: 12, color: themeToken.colorTextQuaternary } }}
        />
      </div>
    );
  }

  const items = merged.map((entry, i) => {
    if (entry.kind === "stock") {
      const node = entry.node;
      const color = PHASE_DOT_COLOR[node.phase];
      return {
        key: `stock-${node.id}-${i}`,
        children: <StockTimelineItem node={node} onNavigate={onNavigateToStock} />,
        color,
      };
    }
    const event = entry.event;
    return {
      key: `dispatch-${i}`,
      children: <DispatchItem event={event} />,
      color: getDispatchColor(event),
    };
  });

  return (
    <div style={{ padding: 12, height: "100%", overflow: "auto", display: "flex", flexDirection: "column", gap: 8 }}>
      {/* 头部统计条 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "6px 8px",
          fontSize: 11,
          color: themeToken.colorTextSecondary,
          background: themeToken.colorBgLayout,
          borderRadius: 4,
          border: `1px solid ${themeToken.colorBorderSecondary}`,
        }}
      >
        <Tag color="blue" style={{ fontSize: 10, margin: 0, padding: "0 6px" }}>
          {t("office.trajectory.businessCount", { count: stockCount })}
        </Tag>
        <Tag color="purple" style={{ fontSize: 10, margin: 0, padding: "0 6px" }}>
          {t("office.trajectory.dispatchCount", { count: dispatchCount })}
        </Tag>
        {stockCode && (
          <span style={{ color: themeToken.colorTextTertiary, fontFamily: "monospace" }}>
            {stockCode}
          </span>
        )}
        {analysisStatus === "running" && (
          <Tag color="processing" style={{ fontSize: 10, margin: 0, padding: "0 6px" }}>
            {t("office.trajectory.analysisRunning")}
          </Tag>
        )}
        {/* 折叠/展开 dispatch 事件 */}
        <a
          style={{ marginLeft: "auto", fontSize: 10, color: themeToken.colorPrimary }}
          onClick={() =>
            setHideDispatch((v) => !v)}
        >
          {hideDispatch
            ? t("office.trajectory.showDispatch")
            : t("office.trajectory.hideDispatch")}
        </a>
      </div>

      {/* 时间线主体（hideDispatch=true 时只显示 stock 节点） */}
      <Timeline
        items={hideDispatch ? items.filter((_, i) => merged[i]?.kind === "stock") : items}
      />
    </div>
  );
}

/** 业务时间线条目（来自 stockAnalysis.timeline） */
function StockTimelineItem({
  node,
  onNavigate,
}: {
  node: TimelineNode;
  onNavigate?: (tabKey: "market" | "analyze" | "execute", panelKey: string) => void;
}) {
  const { t } = useTranslation();
  const { token: themeToken } = theme.useToken();

  const phaseColor = PHASE_COLOR[node.phase];
  const phaseLabel = t(PHASE_LABEL_KEY[node.phase]);

  // 节点可跳转：evidenceRefs 至少有一条
  const firstRef = node.evidenceRefs?.[0];
  const clickable = !!onNavigate && !!firstRef;

  const handleClick = () => {
    if (!clickable || !firstRef) { return; }
    onNavigate(firstRef.tabKey, firstRef.panelKey);
  };

  return (
    <div
      style={{
        fontSize: 12,
        cursor: clickable ? "pointer" : "default",
        padding: "2px 0",
      }}
      onClick={handleClick}
      onMouseEnter={(e) => {
        if (clickable) {
          (e.currentTarget as HTMLDivElement).style.background = themeToken.colorFillQuaternary;
        }
      }}
      onMouseLeave={(e) => {
        if (clickable) {
          (e.currentTarget as HTMLDivElement).style.background = "transparent";
        }
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 4, flexWrap: "wrap" }}>
        <Tag color={phaseColor} style={{ fontSize: 10, margin: 0, padding: "0 6px" }}>
          {phaseLabel}
        </Tag>
        <span style={{ fontWeight: 600, color: themeToken.colorText }}>
          {node.agentName || node.agentId}
        </span>
        <Tag
          color={nodeStatusColor(node.status)}
          style={{ fontSize: 10, margin: 0, padding: "0 6px", lineHeight: "16px" }}
        >
          {t(`stockAnalysis.timeline.nodeStatus.${node.status}`)}
        </Tag>
        {node.confidence > 0 && (
          <span style={{ fontSize: 10, color: themeToken.colorTextTertiary }}>
            {(node.confidence * 100).toFixed(0)}%
          </span>
        )}
      </div>
      <div style={{ marginTop: 2, color: themeToken.colorTextSecondary, fontSize: 11 }}>
        {node.summary || node.title}
      </div>
      {clickable && (
        <div style={{ fontSize: 10, color: themeToken.colorPrimary, marginTop: 2 }}>
          {t("office.trajectory.jumpToEvidence")} →
        </div>
      )}
    </div>
  );
}

/** dispatch 事件条目（保持原 TrajectoryItem 逻辑，仅迁移样式） */
function DispatchItem({ event }: { event: DispatchEvent }) {
  const { t } = useTranslation();
  const { token: themeToken } = theme.useToken();
  switch (event.type) {
    case "routing":
      return (
        <div style={{ fontSize: 12 }}>
          <Tag color="blue" style={{ fontSize: 10, margin: 0, padding: "0 6px" }}>
            {t("office.trajectory.tagRouting")}
          </Tag>
          <span style={{ color: themeToken.colorText }}>
            {t("office.trajectory.routing", {
              slug: event.agentSlug,
              summary: event.taskSummary.slice(0, 60),
            })}
          </span>
        </div>
      );
    case "process":
      return (
        <div style={{ fontSize: 12 }}>
          <Tag color="purple" style={{ fontSize: 10, margin: 0, padding: "0 6px" }}>
            {t("office.trajectory.tagProcess")}
          </Tag>
          <span>
            {event.agentSlug}: {event.status}
          </span>
        </div>
      );
    case "agent_status":
      return (
        <div style={{ fontSize: 12 }}>
          <Tag color="orange" style={{ fontSize: 10, margin: 0, padding: "0 6px" }}>
            {t("office.trajectory.tagStatus")}
          </Tag>
          <span>
            {event.agentSlug}: {t(`office.memberStatus.${event.status}`)}
          </span>
        </div>
      );
    case "token_usage":
      return (
        <div style={{ fontSize: 12 }}>
          <Tag color="green" style={{ fontSize: 10, margin: 0, padding: "0 6px" }}>
            {t("office.trajectory.tagToken")}
          </Tag>
          <span>
            {event.agentSlug}: +{event.inputTokens}↑ / +{event.outputTokens}↓
          </span>
        </div>
      );
    default:
      return null;
  }
}

function getDispatchColor(event: DispatchEvent): string {
  switch (event.type) {
    case "routing":
      return "blue";
    case "process":
      return "purple";
    case "agent_status":
      return "orange";
    case "token_usage":
      return "green";
    default:
      return "gray";
  }
}
