// SPDX-License-Identifier: AGPL-3.0-only
/* eslint-disable react-refresh/only-export-components */

/**
 * MultiLlmVotePanel.tsx — 多 LLM 决策投票/集成面板
 *
 * 借鉴 TradingAgents-CN 多 LLM 投票思想:在多模型并行场景下,
 * 从每个模型的回复中提取结构化决策(workflow-decision 卡片),
 * 按用户选择的策略(majority / weighted / consensus)聚合为最终决策。
 *
 * 设计目标:纯前端、零后端改动、不修改 multiModelStore,
 * 从多模型回复的 workflow-decision 卡片(<!-- workflow-decision:JSON -->)中解析。
 *
 * 入参:多模型版本列表(同一 parent_message_id 的若干 Message)
 * 出参:聚合后的最终决策 + 各模型票数明细 + 平均信心度
 */

import type { Message } from "@/types";
import { Card, Segmented, Space, Statistic, Tag, theme, Tooltip, Typography } from "antd";
import { CheckCircle2, Scale, Vote } from "lucide-react";
import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

export type VoteStrategy = "majority" | "weighted" | "consensus";

export interface ModelVote {
  modelId: string;
  providerId: string | null;
  action: string;
  positionPct: number | null;
  targetPrice: number | null;
  stopLoss: number | null;
  riskLevel: string | null;
  confidence: number;
  reasoning: string | null;
}

export interface VoteResult {
  strategy: VoteStrategy;
  finalAction: string | null;
  winnerModelId: string | null;
  winnerProviderId: string | null;
  avgConfidence: number;
  /** action -> count(按模型数) */
  breakdown: Record<string, number>;
  /** action -> weighted score(按 confidence) */
  weightedBreakdown: Record<string, number>;
  total: number;
  allAgree: boolean;
}

const DEFAULT_RISK = "medium";
void DEFAULT_RISK; // 保留以备扩展风险维度

function extractDecision(msg: Message): ModelVote | null {
  // 解析最后一条 content(若有多张卡片,优先 decision)
  // workflow-decision 卡片以 <!-- workflow-decision:JSON --> 开头
  if (msg.role !== "assistant") { return null; }
  const content = msg.content ?? "";
  // 多卡片时 split by <!-- workflow-
  const segments = content.split(/<!-- workflow-decision:/);
  for (let i = segments.length - 1; i >= 1; i--) {
    const tail = segments[i];
    const end = tail.indexOf("-->");
    if (end < 0) { continue; }
    const json = tail.slice(0, end);
    try {
      const data = JSON.parse(json);
      // 必须含 action 字段
      if (typeof data.action !== "string") { continue; }
      return {
        modelId: msg.model_id ?? "__unknown__",
        providerId: msg.provider_id,
        action: data.action,
        positionPct: typeof data.positionPct === "number" ? data.positionPct : null,
        targetPrice: typeof data.targetPrice === "number" ? data.targetPrice : null,
        stopLoss: typeof data.stopLoss === "number" ? data.stopLoss : null,
        riskLevel: typeof data.riskLevel === "string" ? data.riskLevel : null,
        confidence: typeof data.confidence === "number" ? data.confidence : 0,
        reasoning: typeof data.reasoning === "string" ? data.reasoning : null,
      };
    } catch {
      // 继续尝试上一段
    }
  }
  return null;
}

function dedupeByModel(votes: ModelVote[]): ModelVote[] {
  // 同一 model_id 取 confidence 最高(或最后一条)
  const map = new Map<string, ModelVote>();
  for (const v of votes) {
    const prev = map.get(v.modelId);
    if (!prev || v.confidence >= prev.confidence) {
      map.set(v.modelId, v);
    }
  }
  return Array.from(map.values());
}

export function aggregateVotes(
  votes: ModelVote[],
  strategy: VoteStrategy,
): VoteResult {
  const total = votes.length;
  if (total === 0) {
    return {
      strategy,
      finalAction: null,
      winnerModelId: null,
      winnerProviderId: null,
      avgConfidence: 0,
      breakdown: {},
      weightedBreakdown: {},
      total: 0,
      allAgree: false,
    };
  }

  // 票数明细
  const breakdown: Record<string, number> = {};
  const weightedBreakdown: Record<string, number> = {};
  let confSum = 0;
  for (const v of votes) {
    breakdown[v.action] = (breakdown[v.action] ?? 0) + 1;
    weightedBreakdown[v.action] = (weightedBreakdown[v.action] ?? 0) + v.confidence;
    confSum += v.confidence;
  }
  const avgConfidence = confSum / total;

  const allAgree = Object.keys(breakdown).length <= 1;

  let finalAction: string | null = null;

  if (strategy === "consensus") {
    // 全员一致:必须 Object.keys(breakdown).length === 1
    if (allAgree) {
      finalAction = Object.keys(breakdown)[0] ?? null;
    } else {
      // 无共识:回退到 weighted
      const sorted = Object.entries(weightedBreakdown).sort((a, b) => b[1] - a[1]);
      finalAction = sorted[0]?.[0] ?? null;
    }
  } else if (strategy === "majority") {
    // 多数票:取票数最多的 action;并列时取 confidence 加权更高者
    const maxCount = Math.max(...Object.values(breakdown));
    const tied = Object.entries(breakdown)
      .filter(([, c]) => c === maxCount)
      .map(([a]) => a);
    if (tied.length === 1) {
      finalAction = tied[0];
    } else {
      const tiedWeighted = tied
        .map((a) => ({ a, w: weightedBreakdown[a] ?? 0 }))
        .sort((x, y) => y.w - x.w);
      finalAction = tiedWeighted[0]?.a ?? null;
    }
  } else {
    // weighted:按 confidence 加权
    const sorted = Object.entries(weightedBreakdown).sort((a, b) => b[1] - a[1]);
    finalAction = sorted[0]?.[0] ?? null;
  }

  const winner = votes.find((v) => v.action === finalAction) ?? null;

  return {
    strategy,
    finalAction,
    winnerModelId: winner?.modelId ?? null,
    winnerProviderId: winner?.providerId ?? null,
    avgConfidence,
    breakdown,
    weightedBreakdown,
    total,
    allAgree,
  };
}

const ACTION_COLOR: Record<string, string> = {
  BUY: "green",
  INCREASE: "cyan",
  HOLD: "gold",
  REDUCE: "orange",
  SELL: "red",
  UNCERTAIN: "default",
};

function actionColor(action: string): string {
  return ACTION_COLOR[action] ?? "default";
}

export interface MultiLlmVotePanelProps {
  versions: Message[];
  /** 默认策略 */
  defaultStrategy?: VoteStrategy;
}

export const MultiLlmVotePanel = React.memo(function MultiLlmVotePanel({
  versions,
  defaultStrategy = "weighted",
}: MultiLlmVotePanelProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const [strategy, setStrategy] = useState<VoteStrategy>(defaultStrategy);

  // 提取所有决策(同一模型保留 confidence 最高)
  const votes = useMemo(() => {
    const raw: ModelVote[] = [];
    for (const m of versions) {
      const d = extractDecision(m);
      if (d) { raw.push(d); }
    }
    return dedupeByModel(raw);
  }, [versions]);

  const result = useMemo(() => aggregateVotes(votes, strategy), [votes, strategy]);

  if (votes.length === 0) {
    return (
      <Card
        size="small"
        title={
          <Space>
            <Vote size={16} />
            {t("chat.multiModelVote.title")}
          </Space>
        }
        style={{ margin: "8px 0" }}
      >
        <Typography.Text type="secondary">
          {t("chat.multiModelVote.noDecision")}
        </Typography.Text>
      </Card>
    );
  }

  const breakdownEntries = Object.entries(result.breakdown).sort(
    (a, b) => b[1] - a[1],
  );

  return (
    <Card
      size="small"
      title={
        <Space>
          <Vote size={16} style={{ color: token.colorPrimary }} />
          <span>{t("chat.multiModelVote.title")}</span>
          {result.allAgree
            ? (
              <Tag color="green" icon={<CheckCircle2 size={12} />}>
                {t("chat.multiModelVote.allAgree")}
              </Tag>
            )
            : (
              <Tag color="orange" icon={<Scale size={12} />}>
                {t("chat.multiModelVote.disagreement")}
              </Tag>
            )}
        </Space>
      }
      extra={
        <Segmented
          size="small"
          value={strategy}
          onChange={(v) => setStrategy(v as VoteStrategy)}
          options={[
            {
              value: "majority",
              label: t("chat.multiModelVote.strategyMajority"),
            },
            {
              value: "weighted",
              label: t("chat.multiModelVote.strategyWeighted"),
            },
            {
              value: "consensus",
              label: t("chat.multiModelVote.strategyConsensus"),
            },
          ]}
        />
      }
      style={{ margin: "8px 0" }}
    >
      {/* 最终决策摘要 */}
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: 12,
          padding: 8,
          backgroundColor: token.colorBgLayout,
          borderRadius: token.borderRadiusSM,
          marginBottom: 12,
        }}
      >
        <Statistic
          title={t("chat.multiModelVote.finalAction")}
          value={result.finalAction ?? "—"}
          valueStyle={{
            color: result.finalAction
              ? token[`color${actionColor(result.finalAction).replace(/^./, (c) => c.toUpperCase())}` as "colorPrimary"]
                ?? token.colorPrimary
              : token.colorTextSecondary,
            fontSize: 20,
          }}
        />
        <Statistic
          title={t("chat.multiModelVote.avgConfidence")}
          value={(result.avgConfidence * 100).toFixed(1)}
          suffix="%"
          valueStyle={{ fontSize: 18 }}
        />
        <Statistic
          title={t("chat.multiModelVote.modelCount")}
          value={result.total}
          suffix={` / ${votes.length}`}
          valueStyle={{ fontSize: 18 }}
        />
        {result.winnerModelId && (
          <Statistic
            title={t("chat.multiModelVote.winnerModel")}
            value={result.winnerModelId}
            valueStyle={{ fontSize: 14, color: token.colorPrimary }}
          />
        )}
      </div>

      {/* 票数明细 */}
      <div>
        <Typography.Text strong style={{ fontSize: 13 }}>
          {t("chat.multiModelVote.voteBreakdown")}
        </Typography.Text>
        <div
          style={{
            display: "flex",
            flexWrap: "wrap",
            gap: 8,
            marginTop: 8,
          }}
        >
          {breakdownEntries.map(([action, count]) => {
            const weighted = result.weightedBreakdown[action] ?? 0;
            const pct = result.total > 0 ? (count / result.total) * 100 : 0;
            return (
              <Tooltip
                key={action}
                title={`confidence-weighted: ${weighted.toFixed(2)}`}
              >
                <Tag color={actionColor(action)} style={{ padding: "4px 10px" }}>
                  <Space size={4}>
                    <strong>{action}</strong>
                    <span>
                      {count} · {pct.toFixed(0)}%
                    </span>
                  </Space>
                </Tag>
              </Tooltip>
            );
          })}
        </div>
      </div>

      {/* 各模型原始票 */}
      <div style={{ marginTop: 12 }}>
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          {votes.map((v) => (
            <Tag
              key={v.modelId + v.action}
              color={actionColor(v.action)}
              style={{ marginBottom: 4 }}
            >
              {v.modelId} → {v.action} ({(v.confidence * 100).toFixed(0)}%)
            </Tag>
          ))}
        </Typography.Text>
      </div>
    </Card>
  );
});
