// SPDX-License-Identifier: AGPL-3.0-only

import type { NarrativeStructureRecord } from "@/lib/narrativeStructure";
import { useWorkflowEditorStore } from "@/stores";
import type {
  ArcType,
  ChapterMeta,
  ConfluencePoint,
  Foreshadow,
  NarrativeArc,
  NarrativeStructure,
  StructureAdjustmentSuggestion,
} from "@/types/narrative";
import { Button, Card, Empty, Input, message, Modal, Segmented, Select, Space, Tag, Typography } from "antd";
import { BookOpen, GitBranch, HardDrive, Network, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArcTimeline } from "./ArcTimeline";
import { ForeshadowGraph } from "./ForeshadowGraph";

const { Text } = Typography;

type ViewMode = "overview" | "arcs" | "foreshadows";

export interface NarrativeStructurePanelProps {
  structure?: NarrativeStructure | null;
  chapters?: ChapterMeta[];
  compact?: boolean;
  onArcSelect?: (arcId: string) => void;
  onForeshadowSelect?: (foreshadowId: string) => void;
  onAdjust?: (suggestion: StructureAdjustmentSuggestion) => void;
}

const ARC_TYPE_COLORS: Record<ArcType, string> = {
  transformative: "purple",
  steadfast: "blue",
  flat: "default",
  tragic: "red",
  comedic: "green",
};

const ARC_TYPE_LABELS: Record<ArcType, string> = {
  transformative: "workflow.narrative.arcTypeTransformative",
  steadfast: "workflow.narrative.arcTypeSteadfast",
  flat: "workflow.narrative.arcTypeFlat",
  tragic: "workflow.narrative.arcTypeTragic",
  comedic: "workflow.narrative.arcTypeComedic",
};

export function NarrativeStructurePanel({
  structure,
  compact = false,
}: NarrativeStructurePanelProps) {
  const { t } = useTranslation();
  const [viewMode, setViewMode] = useState<ViewMode>("overview");
  const [saveModalOpen, setSaveModalOpen] = useState(false);
  const [saveName, setSaveName] = useState("");
  const [saveDesc, setSaveDesc] = useState("");
  const [saveGenre, setSaveGenre] = useState("novel");

  const {
    narrativeRecords,
    loadNarrativeRecords,
    saveNarrativeStructure,
    loadNarrativeStructure,
    deleteNarrativeStructure,
  } = useWorkflowEditorStore();

  useEffect(() => {
    loadNarrativeRecords();
  }, [loadNarrativeRecords]);

  const stats = useMemo(() => {
    if (!structure) {
      return { totalArcs: 0, totalForeshadows: 0, totalConfluences: 0, avgProgress: 0 };
    }
    const avgProgress = structure.arcs.length > 0
      ? structure.arcs.reduce((sum, a) => sum + a.currentProgress, 0) / structure.arcs.length
      : 0;
    return {
      totalArcs: structure.arcs.length,
      totalForeshadows: structure.foreshadows.length,
      totalConfluences: structure.confluences.length,
      avgProgress: Math.round(avgProgress),
    };
  }, [structure]);

  const handleSave = async () => {
    if (!saveName.trim()) {
      message.warning(t("workflow.narrative.saveNameRequired"));
      return;
    }
    const id = await saveNarrativeStructure(saveName, saveDesc || undefined, saveGenre);
    if (id) {
      message.success(t("workflow.narrative.saveSuccess"));
      setSaveModalOpen(false);
      setSaveName("");
      setSaveDesc("");
    } else {
      message.error(t("workflow.narrative.saveFailed"));
    }
  };

  const handleLoad = async (record: NarrativeStructureRecord) => {
    await loadNarrativeStructure(record.id);
    message.success(t("workflow.narrative.loadSuccess"));
  };

  const handleDelete = async (id: string) => {
    Modal.confirm({
      title: t("workflow.narrative.confirmDelete"),
      content: t("workflow.narrative.confirmDeleteDesc"),
      okText: t("common.confirm"),
      cancelText: t("common.cancel"),
      onOk: () => deleteNarrativeStructure(id),
    });
  };

  if (!structure) {
    return (
      <div className={compact ? "p-2" : "p-4"}>
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={
            <Text type="secondary">
              {t("workflow.narrative.noStructure")}
            </Text>
          }
        />
        {narrativeRecords.length > 0 && (
          <div className="mt-4">
            <Text type="secondary" className="text-xs mb-2 block">
              {t("workflow.narrative.savedRecords")}
            </Text>
            <div className="space-y-2 max-h-40 overflow-auto">
              {narrativeRecords.map((record) => (
                <div
                  key={record.id}
                  className="flex items-center justify-between p-2 rounded border hover:bg-gray-50 cursor-pointer"
                  onClick={() => handleLoad(record)}
                >
                  <div className="flex-1 min-w-0">
                    <Text strong className="truncate block">{record.name}</Text>
                    <Text type="secondary" className="text-xs">{record.genre}</Text>
                  </div>
                  <Button
                    size="small"
                    danger
                    icon={<Trash2 size={12} />}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDelete(record.id);
                    }}
                  />
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    );
  }

  return (
    <div className={compact ? "p-2" : "p-4"}>
      {/* 操作栏 */}
      <div className="flex justify-between items-center mb-3">
        <Text strong>{t("workflow.narrative.title")}</Text>
        <Space size="small">
          <Button
            size="small"
            icon={<HardDrive size={14} />}
            onClick={() => setSaveModalOpen(true)}
          >
            {t("workflow.narrative.save")}
          </Button>
        </Space>
      </div>

      {/* 已保存记录选择 */}
      {narrativeRecords.length > 0 && (
        <div className="mb-3">
          <Select
            placeholder={t("workflow.narrative.loadFromHistory")}
            allowClear
            size="small"
            className="w-full"
            options={narrativeRecords.map((r) => ({
              value: r.id,
              label: `${r.name} (${r.genre})`,
            }))}
            onChange={(id) => {
              if (id) {
                const record = narrativeRecords.find((r) => r.id === id);
                if (record) { handleLoad(record); }
              }
            }}
          />
        </div>
      )}

      {/* 概览统计 */}
      <div className="grid grid-cols-2 gap-3 mb-4">
        <StatCard
          icon={<GitBranch size={16} />}
          label={t("workflow.narrative.arcs")}
          value={stats.totalArcs}
          color="purple"
        />
        <StatCard
          icon={<Network size={16} />}
          label={t("workflow.narrative.foreshadows")}
          value={stats.totalForeshadows}
          color="blue"
        />
        <StatCard
          icon={<BookOpen size={16} />}
          label={t("workflow.narrative.confluences")}
          value={stats.totalConfluences}
          color="orange"
        />
        <StatCard
          icon={<GitBranch size={16} />}
          label={t("workflow.narrative.progress")}
          value={`${stats.avgProgress}%`}
          color="green"
        />
      </div>

      {/* 视图切换 */}
      <Segmented
        value={viewMode}
        onChange={(val) => setViewMode(val as ViewMode)}
        options={[
          { label: t("workflow.narrative.viewOverview"), value: "overview" },
          { label: t("workflow.narrative.viewArcs"), value: "arcs" },
          { label: t("workflow.narrative.viewForeshadows"), value: "foreshadows" },
        ]}
        className="mb-3 w-full"
      />

      {/* 视图内容 */}
      {viewMode === "overview" && (
        <OverviewView
          arcs={structure.arcs}
          confluences={structure.confluences}
          foreshadows={structure.foreshadows}
        />
      )}
      {viewMode === "arcs" && <ArcTimeline arcs={structure.arcs} />}
      {viewMode === "foreshadows" && <ForeshadowGraph foreshadows={structure.foreshadows} />}

      {/* 保存对话框 */}
      <Modal
        title={t("workflow.narrative.saveTitle")}
        open={saveModalOpen}
        onOk={handleSave}
        onCancel={() => setSaveModalOpen(false)}
        okText={t("common.confirm")}
        cancelText={t("common.cancel")}
      >
        <div className="space-y-3">
          <div>
            <Text type="secondary" className="text-xs block mb-1">
              {t("workflow.narrative.saveName")}
            </Text>
            <Input
              value={saveName}
              onChange={(e) => setSaveName(e.target.value)}
              placeholder={t("workflow.narrative.saveNamePlaceholder")}
            />
          </div>
          <div>
            <Text type="secondary" className="text-xs block mb-1">
              {t("workflow.narrative.saveDesc")}
            </Text>
            <Input
              value={saveDesc}
              onChange={(e) => setSaveDesc(e.target.value)}
              placeholder={t("workflow.narrative.saveDescPlaceholder")}
            />
          </div>
          <div>
            <Text type="secondary" className="text-xs block mb-1">
              {t("workflow.narrative.saveGenre")}
            </Text>
            <Select
              value={saveGenre}
              onChange={setSaveGenre}
              className="w-full"
              options={[
                { value: "novel", label: t("workflow.narrative.genreNovel") },
                { value: "short_story", label: t("workflow.narrative.genreShortStory") },
                { value: "screenplay", label: t("workflow.narrative.genreScreenplay") },
                { value: "poetry", label: t("workflow.narrative.genrePoetry") },
                { value: "other", label: t("workflow.narrative.genreOther") },
              ]}
            />
          </div>
        </div>
      </Modal>
    </div>
  );
}

// ── 子组件 ──

function StatCard({
  icon,
  label,
  value,
  color,
}: {
  icon: React.ReactNode;
  label: string;
  value: string | number;
  color: string;
}) {
  const colorMap: Record<string, string> = {
    purple: "bg-purple-50 border-purple-200 text-purple-700",
    blue: "bg-blue-50 border-blue-200 text-blue-700",
    orange: "bg-orange-50 border-orange-200 text-orange-700",
    green: "bg-green-50 border-green-200 text-green-700",
  };
  return (
    <div
      className={`rounded-lg border px-3 py-2 ${colorMap[color] ?? colorMap.blue}`}
    >
      <div className="flex items-center gap-1 text-xs opacity-75 mb-1">
        {icon}
        <span>{label}</span>
      </div>
      <div className="text-lg font-semibold">{value}</div>
    </div>
  );
}

function OverviewView({
  arcs,
  confluences,
  foreshadows,
}: {
  arcs: NarrativeArc[];
  confluences: ConfluencePoint[];
  foreshadows: Foreshadow[];
}) {
  const { t } = useTranslation();

  return (
    <Space direction="vertical" size="small" className="w-full">
      {/* 弧线列表 */}
      <Card
        size="small"
        title={
          <span className="flex items-center gap-2">
            <GitBranch size={14} />
            {t("workflow.narrative.arcList")}
          </span>
        }
        className="narrative-card"
      >
        {arcs.length === 0
          ? (
            <Text type="secondary" className="text-sm">
              {t("workflow.narrative.noArcs")}
            </Text>
          )
          : (
            <Space direction="vertical" size={8}>
              {arcs.map((arc) => <ArcListItem key={arc.id} arc={arc} />)}
            </Space>
          )}
      </Card>

      {/* 交汇点列表 */}
      {confluences.length > 0 && (
        <Card
          size="small"
          title={
            <span className="flex items-center gap-2">
              <BookOpen size={14} />
              {t("workflow.narrative.confluenceList")}
            </span>
          }
          className="narrative-card"
        >
          <Space wrap size={[8, 8]}>
            {confluences.map((cp) => (
              <Tag key={cp.id} color="orange">
                {t("workflow.narrative.confluenceLabel", { chapter: cp.triggerChapter, type: cp.confluenceType })}
              </Tag>
            ))}
          </Space>
        </Card>
      )}

      {/* 伏笔列表 */}
      {foreshadows.length > 0 && (
        <Card
          size="small"
          title={
            <span className="flex items-center gap-2">
              <Network size={14} />
              {t("workflow.narrative.foreshadowList")}
            </span>
          }
          className="narrative-card"
        >
          <Space direction="vertical" size={4}>
            {foreshadows.map((fs) => (
              <div key={fs.id} className="flex items-center gap-2 text-sm">
                <Tag
                  color={fs.status === "payoff" ? "green" : fs.status === "abandoned" ? "default" : "blue"}
                >
                  {fs.status}
                </Tag>
                <span className="text-xs text-gray-500">
                  {t("workflow.narrative.chapterLabel", { chapter: fs.setupChapter })}
                  {fs.payoffChapter ? `→${t("workflow.narrative.chapterLabel", { chapter: fs.payoffChapter })}` : ""}
                </span>
                <span className="truncate">{fs.description}</span>
              </div>
            ))}
          </Space>
        </Card>
      )}
    </Space>
  );
}

function ArcListItem({ arc }: { arc: NarrativeArc }) {
  const { t } = useTranslation();
  return (
    <div className="rounded border border-gray-200 p-2 hover:border-purple-300 transition-colors">
      <div className="flex items-center justify-between mb-1">
        <div className="flex items-center gap-2">
          <Tag color={ARC_TYPE_COLORS[arc.arcType]}>
            {t(ARC_TYPE_LABELS[arc.arcType])}
          </Tag>
          <Text strong>{arc.subject}</Text>
        </div>
        <Text type="secondary" className="text-xs">
          {t("workflow.narrative.progress")}: {arc.currentProgress}%
        </Text>
      </div>
      {arc.want && (
        <div className="text-xs text-gray-500">
          <span className="text-blue-500">Want:</span> {arc.want}
        </div>
      )}
      {arc.need && (
        <div className="text-xs text-gray-500">
          <span className="text-amber-500">Need:</span> {arc.need}
        </div>
      )}
    </div>
  );
}
