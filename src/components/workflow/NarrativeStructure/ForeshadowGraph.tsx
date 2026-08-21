// SPDX-License-Identifier: AGPL-3.0-only

import type { Foreshadow as ForeshadowType, ForeshadowStatus } from "@/types/narrative";
import { Card, Tag, Typography } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface ForeshadowGraphProps {
  foreshadows: ForeshadowType[];
}

const STATUS_COLORS: Record<ForeshadowStatus, string> = {
  setup: "blue",
  payoff: "green",
  abandoned: "default",
};

const STATUS_LABELS: Record<ForeshadowStatus, string> = {
  setup: "workflow.narrative.foreshadowStatusSetup",
  payoff: "workflow.narrative.foreshadowStatusPayoff",
  abandoned: "workflow.narrative.foreshadowStatusAbandoned",
};

export function ForeshadowGraph({ foreshadows }: ForeshadowGraphProps) {
  const { t } = useTranslation();

  const { setupChapters, payoffChapters, statusCounts } = useMemo(() => {
    const setupChapters = foreshadows.map((f) => f.setupChapter);
    const payoffChapters = foreshadows
      .map((f) => f.payoffChapter)
      .filter((c): c is number => c !== undefined && c !== null);

    const statusCounts = foreshadows.reduce(
      (acc, f) => {
        acc[f.status] = (acc[f.status] || 0) + 1;
        return acc;
      },
      {} as Record<ForeshadowStatus, number>,
    );

    return { setupChapters, payoffChapters, statusCounts };
  }, [foreshadows]);

  if (foreshadows.length === 0) {
    return (
      <div className="text-center py-8">
        <Text type="secondary">
          {t("workflow.narrative.noForeshadows")}
        </Text>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* 状态统计 */}
      <div className="flex gap-2 flex-wrap">
        {(["setup", "payoff", "abandoned"] as ForeshadowStatus[]).map((status) => (
          <div
            key={status}
            className="flex items-center gap-1 rounded border px-2 py-1 text-xs"
          >
            <Tag color={STATUS_COLORS[status]}>{t(STATUS_LABELS[status])}</Tag>
            <span className="font-semibold">{statusCounts[status] || 0}</span>
          </div>
        ))}
      </div>

      {/* 伏笔关系可视化 */}
      <div className="relative">
        {/* 时间轴 */}
        <div className="absolute left-12 right-4 top-1/2 h-0.5 bg-gray-200 -translate-y-1/2" />

        {/* 节点列表 */}
        <div className="space-y-2">
          {foreshadows.map((fs) => <ForeshadowNode key={fs.id} foreshadow={fs} />)}
        </div>
      </div>

      {/* 章节分布统计 */}
      <Card size="small" title={t("workflow.narrative.chapterDistribution")}>
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <Text type="secondary" className="text-xs block mb-1">
              {t("workflow.narrative.setupChapters")}
            </Text>
            <div className="flex flex-wrap gap-1">
              {setupChapters.map((ch, i) => (
                <Tag key={i} color="blue" className="text-xs">
                  {t("workflow.narrative.chapterLabel", { chapter: ch })}
                </Tag>
              ))}
            </div>
          </div>
          <div>
            <Text type="secondary" className="text-xs block mb-1">
              {t("workflow.narrative.payoffChapters")}
            </Text>
            <div className="flex flex-wrap gap-1">
              {payoffChapters.length > 0
                ? (
                  payoffChapters.map((ch, i) => (
                    <Tag key={i} color="green" className="text-xs">
                      {t("workflow.narrative.chapterLabel", { chapter: ch })}
                    </Tag>
                  ))
                )
                : (
                  <Text type="secondary" className="text-xs">
                    {t("workflow.narrative.noPayoff")}
                  </Text>
                )}
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}

function ForeshadowNode({ foreshadow }: { foreshadow: ForeshadowType }) {
  const { t } = useTranslation();
  const statusColor = STATUS_COLORS[foreshadow.status];
  const hasPayoff = foreshadow.payoffChapter !== null && foreshadow.payoffChapter !== undefined;

  return (
    <div className="flex items-center gap-3">
      {/* 埋设节点 */}
      <div className="flex items-center gap-2 w-16 shrink-0">
        <div className="w-8 h-8 rounded-full border-2 border-blue-400 bg-blue-50 flex items-center justify-center text-xs font-bold text-blue-600">
          {foreshadow.setupChapter}
        </div>
      </div>

      {/* 连接线 */}
      <div className="flex-1 relative">
        {hasPayoff && (
          <svg className="w-full h-6" viewBox="0 0 100 24" preserveAspectRatio="none">
            <path
              d="M 0 12 Q 50 0 100 12"
              fill="none"
              stroke={foreshadow.status === "payoff" ? "#52c41a" : "#d9d9d9"}
              strokeWidth="2"
              strokeDasharray={foreshadow.status === "setup" ? "4 2" : undefined}
            />
          </svg>
        )}
      </div>

      {/* 回收节点或状态 */}
      <div className="w-16 shrink-0">
        {hasPayoff
          ? (
            <div className="w-8 h-8 rounded-full border-2 border-green-400 bg-green-50 flex items-center justify-center text-xs font-bold text-green-600 mx-auto">
              {foreshadow.payoffChapter}
            </div>
          )
          : (
            <Tag color={statusColor} className="text-xs mx-auto block w-fit">
              {t(STATUS_LABELS[foreshadow.status])}
            </Tag>
          )}
      </div>

      {/* 描述 */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-1 mb-0.5">
          <Tag color={statusColor} className="text-xs">
            {t(STATUS_LABELS[foreshadow.status])}
          </Tag>
          {foreshadow.relatedArcs.length > 0 && (
            <span className="text-xs text-gray-400">
              {t("workflow.narrative.relatedArcs")}: {foreshadow.relatedArcs.length}
            </span>
          )}
        </div>
        <Text type="secondary" className="text-xs truncate block">
          {foreshadow.description}
        </Text>
        {foreshadow.payoffDescription && (
          <Text type="secondary" className="text-xs truncate block text-green-600">
            → {foreshadow.payoffDescription}
          </Text>
        )}
      </div>
    </div>
  );
}
