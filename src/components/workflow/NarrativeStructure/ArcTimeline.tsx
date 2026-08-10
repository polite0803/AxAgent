// SPDX-License-Identifier: AGPL-3.0-only

import type { ArcStage, ArcType, NarrativeArc } from "@/types/narrative";
import { Card, Progress, Segmented, Tag, Typography } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

const ARC_TYPE_COLORS: Record<ArcType, string> = {
  transformative: "purple",
  steadfast: "blue",
  flat: "default",
  tragic: "red",
  comedic: "green",
};

const ARC_TYPE_LABELS: Record<ArcType, string> = {
  transformative: "转换型",
  steadfast: "坚定型",
  flat: "扁平型",
  tragic: "悲剧型",
  comedic: "喜剧型",
};

interface ArcTimelineProps {
  arcs: NarrativeArc[];
  selectedArcId?: string;
  onSelectArc?: (arcId: string) => void;
}

export function ArcTimeline({ arcs, selectedArcId, onSelectArc }: ArcTimelineProps) {
  const { t } = useTranslation();
  const [internalSelected, setInternalSelected] = useState<string>(
    selectedArcId ?? arcs[0]?.id ?? "",
  );

  const activeArcId = selectedArcId ?? internalSelected;
  const activeArc = useMemo(
    () => arcs.find((a) => a.id === activeArcId) ?? arcs[0],
    [arcs, activeArcId],
  );

  if (!activeArc) {
    return (
      <div className="text-center py-8">
        <Text type="secondary">{t("workflow.narrative.noArcs", "暂无弧线定义")}</Text>
      </div>
    );
  }

  const handleArcSelect = (arcId: string) => {
    setInternalSelected(arcId);
    onSelectArc?.(arcId);
  };

  // 按章节排序阶段
  const sortedStages = [...activeArc.stages].sort((a, b) => a.chapter - b.chapter);

  // 计算最大章节数用于时间轴标尺
  const maxChapter = sortedStages.length > 0
    ? Math.max(...sortedStages.map((s) => s.chapter))
    : 0;

  return (
    <div className="space-y-4">
      {/* 弧线选择器 */}
      {arcs.length > 1 && (
        <Segmented
          value={activeArc?.id}
          onChange={(val) => handleArcSelect(val as string)}
          options={arcs.map((arc) => ({
            label: (
              <span className="flex items-center gap-1">
                <Tag color={ARC_TYPE_COLORS[arc.arcType]} className="text-xs">
                  {ARC_TYPE_LABELS[arc.arcType]}
                </Tag>
                {arc.subject}
              </span>
            ),
            value: arc.id,
          }))}
          className="w-full overflow-x-auto"
        />
      )}

      {/* 弧线详情 */}
      <Card
        size="small"
        title={
          <div className="flex items-center gap-2">
            <Tag color={ARC_TYPE_COLORS[activeArc.arcType]}>
              {ARC_TYPE_LABELS[activeArc.arcType]}
            </Tag>
            <Title level={5} style={{ margin: 0 }}>
              {activeArc.subject}
            </Title>
          </div>
        }
      >
        {/* 基本信息 */}
        <div className="grid grid-cols-2 gap-3 mb-4">
          <div>
            <Text type="secondary" className="text-xs">Want（外部目标）</Text>
            <div className="text-sm">{activeArc.want || "-"}</div>
          </div>
          <div>
            <Text type="secondary" className="text-xs">Need（内部缺失）</Text>
            <div className="text-sm">{activeArc.need || "-"}</div>
          </div>
        </div>

        {/* 推进度 */}
        <div className="mb-4">
          <div className="flex justify-between text-xs mb-1">
            <Text type="secondary">{t("workflow.narrative.progress", "推进度")}</Text>
            <Text>{activeArc.currentProgress}%</Text>
          </div>
          <Progress
            percent={activeArc.currentProgress}
            strokeColor={{
              "0%": getArcColor(activeArc.arcType, 0),
              "100%": getArcColor(activeArc.arcType, 100),
            }}
          />
        </div>

        {/* 阶段时间轴 */}
        <div>
          <Text strong className="text-sm mb-2 block">
            {t("workflow.narrative.stages", "弧线阶段")}
          </Text>
          {sortedStages.length === 0
            ? (
              <Text type="secondary" className="text-sm">
                {t("workflow.narrative.noStages", "未定义阶段")}
              </Text>
            )
            : (
              <div className="relative pl-6">
                {/* 时间轴竖线 */}
                <div className="absolute left-2 top-0 bottom-0 w-0.5 bg-gray-200" />

                <div className="space-y-4">
                  {sortedStages.map((stage, index) => (
                    <StageNode
                      key={`${stage.name}-${stage.chapter}`}
                      stage={stage}
                      index={index}
                      total={sortedStages.length}
                      progress={activeArc.currentProgress}
                    />
                  ))}
                </div>
              </div>
            )}
        </div>

        {/* 章节标尺 */}
        {maxChapter > 0 && (
          <div className="mt-4 pt-3 border-t border-gray-100">
            <div className="flex justify-between text-xs text-gray-400">
              <span>第1章</span>
              <span>第{maxChapter}章</span>
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}

function StageNode({
  stage,
  index,
  total,
  progress,
}: {
  stage: ArcStage;
  index: number;
  total: number;
  progress: number;
}) {
  const isCompleted = (index / Math.max(total, 1)) * 100 < progress;
  const isCurrent = !isCompleted && (index / Math.max(total, 1)) * 100 >= progress - 20;

  return (
    <div className="relative">
      {/* 节点圆点 */}
      <div
        className={`absolute -left-4 w-4 h-4 rounded-full border-2 flex items-center justify-center ${
          isCompleted
            ? "border-green-500 bg-green-500"
            : isCurrent
            ? "border-blue-500 bg-white"
            : "border-gray-300 bg-white"
        }`}
      >
        {isCompleted && (
          <svg className="w-2 h-2 text-white" viewBox="0 0 12 12" fill="none">
            <path d="M2 6l3 3 5-5" stroke="currentColor" strokeWidth="2" />
          </svg>
        )}
      </div>

      {/* 阶段内容 */}
      <div
        className={`rounded border p-2 transition-all ${
          isCompleted
            ? "border-green-200 bg-green-50"
            : isCurrent
            ? "border-blue-200 bg-blue-50"
            : "border-gray-200 bg-white"
        }`}
      >
        <div className="flex items-center justify-between mb-1">
          <Text strong className="text-sm">
            {stage.name}
          </Text>
          <Tag color={isCompleted ? "green" : isCurrent ? "blue" : "default"} className="text-xs">
            第{stage.chapter}章
          </Tag>
        </div>
        <Text type="secondary" className="text-xs">
          {stage.description}
        </Text>
      </div>
    </div>
  );
}

function getArcColor(type: ArcType, _opacity: number): string {
  const colorMap: Record<ArcType, string> = {
    transformative: "#722ed1",
    steadfast: "#1677ff",
    flat: "#8c8c8c",
    tragic: "#cf1322",
    comedic: "#389e0d",
  };
  return colorMap[type];
}
