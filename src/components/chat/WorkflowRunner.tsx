// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, listen } from "@/lib/invoke";
import {
  type SerenityCandidate,
  type StepStage,
  type TrendInfo,
  useSerenityStore,
} from "@/stores/feature/serenityStore";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import { AlertOutlined, CheckCircleOutlined, LoadingOutlined, ReloadOutlined, RobotOutlined } from "@ant-design/icons";
import { Button, Card, Empty, Modal, Progress, Select, Space, Spin, Tag, Typography } from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { SerenityCandidateCard } from "../stock-analysis/SerenityCandidateCard";

const { Text, Title } = Typography;

// 对话页"选工作流即执行"当前仅支持趋势智荐（serenity-screening）：
// 该模板由 run_serenity_screening 专用命令驱动（事件 + 候选提取 + 落库），
// 与通用 workflow_execute 路径不同，故由 WorkflowRunner 单独接管。
// 其他模板（stock-analysis 等）保持原 workflow 会话标记路径，不进 WorkflowRunner。
const STOCK_WORKFLOW_TEMPLATES = [
  "serenity-screening",
];

export function isStockWorkflowTemplate(templateId: string): boolean {
  return STOCK_WORKFLOW_TEMPLATES.some(
    (t) => templateId.includes(t),
  );
}

const NODE_STAGE_MAP: Record<string, StepStage> = {
  trigger: "loading",
  "t-industry-rank": "scanning",
  "t-cls-flash": "scanning",
  "t-northbound": "scanning",
  "t-baseline-semi": "scanning",
  "t-baseline-battery": "scanning",
  "t-baseline-chem": "scanning",
  "t-baseline-med": "scanning",
  "t-baseline-aero": "scanning",
  "t-baseline-consumer-elec": "scanning",
  "t-baseline-auto": "scanning",
  "t-signal-semi": "scanning",
  "t-signal-battery": "scanning",
  "t-signal-chem": "scanning",
  "t-signal-med": "scanning",
  "t-signal-aero": "scanning",
  "t-signal-consumer-elec": "scanning",
  "t-signal-auto": "scanning",
  "a-trend-scanner": "scanning",
  "a-chain-trend1": "decomposing",
  "a-chain-trend2": "decomposing",
  "a-chain-trend3": "decomposing",
  "a-chain-trend4": "decomposing",
  "a-chain-trend5": "decomposing",
  "c-bottleneck-trend1": "identifying",
  "c-bottleneck-trend2": "identifying",
  "c-bottleneck-trend3": "identifying",
  "c-bottleneck-trend4": "identifying",
  "c-bottleneck-trend5": "identifying",
  "c-consistency-check": "identifying",
  "a-candidate-mapper": "mapping",
  "c-data-verifier": "mapping",
  "s-save-candidates": "saving",
};

function nodeTitleKey(nodeId: string): string {
  return `serenityPanel.nodeTitles.${nodeId}`;
}

function extractCandidatesList(raw: unknown): SerenityCandidate[] {
  if (raw == null) { return []; }
  if (Array.isArray(raw)) { return raw as SerenityCandidate[]; }
  if (typeof raw === "object") {
    const obj = raw as Record<string, unknown>;
    for (const key of ["candidates", "stocks", "list", "data", "items", "results"]) {
      if (Array.isArray(obj[key])) {
        return obj[key] as SerenityCandidate[];
      }
    }
    if (obj.params && typeof obj.params === "object") {
      const params = obj.params as Record<string, unknown>;
      for (const key of ["candidates", "stocks", "list", "data"]) {
        if (Array.isArray(params[key])) {
          return params[key] as SerenityCandidate[];
        }
      }
      if (Array.isArray(params)) {
        return params as unknown as SerenityCandidate[];
      }
    }
    if (typeof obj.content === "string") {
      try {
        const parsed = JSON.parse(obj.content);
        if (parsed && typeof parsed === "object") {
          return extractCandidatesList(parsed);
        }
      } catch { /* ignore */ }
    }
  }
  return [];
}

function truncateText(s: string, n: number): string {
  return s.length > n ? s.slice(0, n) + "…" : s;
}

export interface WorkflowRunnerProps {
  templateId: string;
  workflowId?: string;
  conversationId: string;
  onProgress?: (progress: { completed: number; total: number; stage: StepStage }) => void;
  onComplete?: (result: { candidates: SerenityCandidate[]; trends: TrendInfo[] }) => void;
  onClose?: () => void;
}

export function WorkflowRunner({
  templateId,
  conversationId: _conversationId,
  onProgress,
  onComplete,
  onClose,
}: WorkflowRunnerProps) {
  const { t } = useTranslation();

  const [modalOpen, setModalOpen] = useState(true);
  const [themeTags, setThemeTags] = useState<string[]>([]);
  const [configuring, setConfiguring] = useState(false);

  const running = useSerenityStore((s) => s.running);
  const candidates = useSerenityStore((s) => s.candidates);
  const trends = useSerenityStore((s) => s.trends);
  const error = useSerenityStore((s) => s.error);
  const stage = useSerenityStore((s) => s.stage);
  const completedNodes = useSerenityStore((s) => s.completedNodes);
  const totalNodes = useSerenityStore((s) => s.totalNodes);
  const steps = useSerenityStore((s) => s.steps);
  const currentNodeId = useSerenityStore((s) => s.currentNodeId);
  const emptyReason = useSerenityStore((s) => s.emptyReason);

  const setRunning = useSerenityStore((s) => s.setRunning);
  const setCandidates = useSerenityStore((s) => s.setCandidates);
  const setTrends = useSerenityStore((s) => s.setTrends);
  const setError = useSerenityStore((s) => s.setError);
  const setStage = useSerenityStore((s) => s.setStage);
  const setCompletedNodes = useSerenityStore((s) => s.setCompletedNodes);
  const setTotalNodes = useSerenityStore((s) => s.setTotalNodes);
  const addStep = useSerenityStore((s) => s.addStep);
  const setCurrentNode = useSerenityStore((s) => s.setCurrentNode);
  const clearSteps = useSerenityStore((s) => s.clearSteps);
  const setEmptyReason = useSerenityStore((s) => s.setEmptyReason);

  const unlistenStepRef = useRef<(() => void) | null>(null);
  const unlistenDoneRef = useRef<(() => void) | null>(null);
  const eventHandledRef = useRef(false);

  useEffect(() => {
    return () => {
      unlistenStepRef.current?.();
      unlistenDoneRef.current?.();
    };
  }, []);

  const stageLabel = (() => {
    if (!running && stage === "done") { return t("serenityPanel.stage_done"); }
    if (!running && stage === "error") { return t("serenityPanel.stage_error"); }
    switch (stage) {
      case "loading":
        return t("serenityPanel.stage_loading");
      case "scanning":
        return t("serenityPanel.stage_scanning");
      case "decomposing":
        return t("serenityPanel.stage_decomposing");
      case "identifying":
        return t("serenityPanel.stage_identifying");
      case "mapping":
        return t("serenityPanel.stage_mapping");
      case "saving":
        return t("serenityPanel.stage_saving");
      default:
        return t("serenityPanel.running");
    }
  })();

  const progressPct = totalNodes > 0 ? Math.round((completedNodes / totalNodes) * 100) : 0;

  const handleStart = useCallback(async () => {
    setModalOpen(false);
    setConfiguring(true);

    clearSteps();
    setCandidates([]);
    setTrends([]);
    setError(null);
    setEmptyReason(null);
    setStage("loading");
    setCompletedNodes(0);
    setTotalNodes(0);
    eventHandledRef.current = false;

    try {
      unlistenStepRef.current?.();
      unlistenDoneRef.current?.();

      unlistenStepRef.current = await listen<{
        nodeId: string;
        status: string;
        totalNodes: number;
        completedNodes: number;
        output?: unknown;
        error?: string;
        elapsedMs?: number;
      }>("serenity-screening-step", (event) => {
        const p = event.payload;
        const nodeStage = NODE_STAGE_MAP[p.nodeId] ?? "loading";
        setStage(nodeStage);
        setTotalNodes(p.totalNodes ?? 0);
        setCompletedNodes(p.completedNodes ?? 0);
        setCurrentNode(p.nodeId);
        addStep({
          nodeId: p.nodeId,
          status: p.status,
          output: p.output,
          error: p.error,
          elapsedMs: p.elapsedMs,
          totalNodes: p.totalNodes,
          completedNodes: p.completedNodes,
          timestamp: Date.now(),
        });
        onProgress?.({
          completed: p.completedNodes ?? 0,
          total: p.totalNodes ?? 0,
          stage: nodeStage,
        });
      });

      unlistenDoneRef.current = await listen<{
        status: string;
        result?: unknown;
        candidates?: unknown[];
        trends?: TrendInfo[];
        error?: string;
        emptyReason?: string | null;
      }>("serenity-screening-completed", (event) => {
        const p = event.payload;
        eventHandledRef.current = true;
        if (p.status === "failed") {
          setError(p.error ?? t("serenityPanel.errorUnknown"));
          setStage("error");
          setRunning(false);
          setCurrentNode(null);
        } else if (p.status === "completed") {
          const directCandidates = Array.isArray(p.candidates)
            ? (p.candidates.filter((c: unknown) => c != null) as SerenityCandidate[])
            : null;
          const list = directCandidates && directCandidates.length > 0
            ? directCandidates
            : extractCandidatesList(p.result);
          if (list.length > 0) {
            setCandidates(list);
          }
          if (Array.isArray(p.trends)) {
            setTrends(p.trends);
          }
          if (typeof p.emptyReason === "string" && p.emptyReason.trim().length > 0) {
            setEmptyReason(p.emptyReason.trim());
          }
          setStage("done");
          setRunning(false);
          setCurrentNode(null);
          onComplete?.({
            candidates: list,
            trends: Array.isArray(p.trends) ? p.trends : [],
          });
        }
      });
    } catch {
      // non-Tauri environment
    }

    setRunning(true);
    try {
      const anchorState = useTimeAnchorStore.getState();
      const asOfDate = anchorState.mode === "replay" || anchorState.mode === "backtest_sweep"
        ? anchorState.asOfDate
        : null;
      const SERENITY_TIMEOUT_MS = 30 * 60 * 1000;
      const r = await invoke<
        { status?: string; candidates?: unknown; trends?: TrendInfo[]; emptyReason?: string | null }
      >(
        "run_serenity_screening",
        { asOfDate, themes: themeTags.length > 0 ? themeTags : null },
        SERENITY_TIMEOUT_MS,
      );
      if (!eventHandledRef.current) {
        const list = extractCandidatesList(r?.candidates);
        if (list.length > 0) { setCandidates(list); }
        if (Array.isArray(r?.trends) && r.trends.length > 0) { setTrends(r.trends); }
        if (typeof r?.emptyReason === "string" && r.emptyReason.trim().length > 0) {
          setEmptyReason(r.emptyReason.trim());
        }
        setStage("done");
      }
    } catch (err: unknown) {
      if (!eventHandledRef.current) {
        setError(err instanceof Error ? err.message : String(err));
        setStage("error");
      }
    } finally {
      setRunning(false);
      setCurrentNode(null);
      setConfiguring(false);
    }
  }, [
    addStep,
    clearSteps,
    setCandidates,
    setEmptyReason,
    setError,
    setRunning,
    setStage,
    setTrends,
    setCompletedNodes,
    setTotalNodes,
    setCurrentNode,
    themeTags,
    onProgress,
    onComplete,
    t,
  ]);

  const handleCancel = useCallback(() => {
    setModalOpen(false);
    onClose?.();
  }, [onClose]);

  return (
    <>
      <Modal
        title={t("chat.workflowRunner.inputTheme")}
        open={modalOpen}
        onCancel={handleCancel}
        onOk={handleStart}
        confirmLoading={configuring}
        okText={t("chat.workflowRunner.run")}
        cancelText={t("chat.workflowRunner.cancel")}
      >
        <div className="flex flex-col gap-3 py-2">
          <Text type="secondary" className="text-sm">
            {t("chat.workflowRunner.themeHint")}
          </Text>
          <Select
            mode="tags"
            style={{ width: "100%" }}
            placeholder={t("chat.workflowRunner.themePlaceholder")}
            value={themeTags}
            onChange={setThemeTags as (val: string[]) => void}
            tokenSeparators={[",", "，"]}
            open={false}
          />
          {themeTags.length > 0 && (
            <Tag color="blue" className="w-fit">
              {t("chat.workflowRunner.sourceUser")}: {themeTags.join(", ")}
            </Tag>
          )}
        </div>
      </Modal>

      {configuring && (
        <Card
          size="small"
          className="w-full"
          title={
            <div className="flex items-center gap-2 text-sm">
              <RobotOutlined />
              <span>{t("chat.workflowRunner.title")}</span>
              <Tag color="blue">{templateId}</Tag>
            </div>
          }
          extra={
            <Button
              size="small"
              icon={<ReloadOutlined spin={running} />}
              onClick={handleStart}
              disabled={running}
            >
              {t("chat.workflowRunner.rerun")}
            </Button>
          }
        >
          <div className="flex flex-col gap-3">
            {running && (
              <div className="flex flex-col gap-2">
                <div className="flex items-center gap-2 text-sm">
                  <Spin indicator={<LoadingOutlined spin />} size="small" />
                  <span className="font-medium">{stageLabel}</span>
                  {currentNodeId && (
                    <Text type="secondary" className="text-xs">
                      {t(nodeTitleKey(currentNodeId))}
                    </Text>
                  )}
                </div>
                {totalNodes > 0 && (
                  <Progress
                    percent={progressPct}
                    size="small"
                    format={() => `${completedNodes}/${totalNodes}`}
                  />
                )}
              </div>
            )}

            {!running && stage === "error" && error && (
              <div
                className="rounded border border-red-500/30 p-2 text-sm text-red-400"
                style={{ backgroundColor: "rgba(255,77,79,0.08)" }}
              >
                <AlertOutlined className="mr-1" />
                {error}
              </div>
            )}

            {steps.length > 0 && !running && (
              <div className="flex flex-col gap-1">
                <Text type="secondary" className="text-xs">
                  {t("chat.workflowRunner.progress")}
                </Text>
                <Space orientation="vertical" className="w-full" size={4}>
                  {steps.map((s, i) => {
                    const isFailed = s.status === "failed" || s.status === "timeout";
                    const statusColor = s.status === "completed"
                      ? "green"
                      : isFailed
                      ? "red"
                      : "blue";
                    return (
                      <div
                        key={`${s.nodeId}-${i}`}
                        className="flex items-center gap-2 text-xs rounded border border-gray-100 px-2 py-1"
                      >
                        {s.status === "completed"
                          ? <CheckCircleOutlined style={{ color: "#52c41a" }} />
                          : isFailed
                          ? <span style={{ color: "#ff4d4f" }}>✕</span>
                          : <LoadingOutlined style={{ color: "#1677ff" }} />}
                        <Text strong className="shrink-0">
                          {t(nodeTitleKey(s.nodeId))}
                        </Text>
                        <Tag color={statusColor} className="text-xs shrink-0">
                          {s.status}
                        </Tag>
                        {s.elapsedMs != null && (
                          <Text type="secondary" className="shrink-0">
                            {(s.elapsedMs / 1000).toFixed(1)}s
                          </Text>
                        )}
                        {s.error && isFailed && (
                          <Text type="danger" className="truncate flex-1" title={s.error}>
                            {truncateText(s.error, 60)}
                          </Text>
                        )}
                      </div>
                    );
                  })}
                </Space>
              </div>
            )}

            {trends.length > 0 && !running && (
              <div className="flex flex-col gap-1">
                <Text type="secondary" className="text-xs">
                  {t("serenityPanel.trendTitle")}
                </Text>
                <Space orientation="vertical" className="w-full" size={4}>
                  {trends.map((tr, i) => (
                    <div key={i} className="flex items-center gap-2 text-sm">
                      <Tag color="purple">{tr.confidence ?? "?"}%</Tag>
                      <Text strong>{tr.trend_name ?? tr.trendName}</Text>
                    </div>
                  ))}
                </Space>
              </div>
            )}

            {candidates.length > 0 && !running && (
              <div className="flex flex-col gap-2">
                <div className="flex items-center justify-between">
                  <Title level={5} className="m-0">
                    {t("serenityPanel.candidateTitle")} ({candidates.length})
                  </Title>
                </div>
                <div className="flex flex-col gap-2">
                  {candidates.map((c, i) => {
                    const code = c.stock_code ?? c.stockCode ?? "";
                    return (
                      <SerenityCandidateCard
                        key={`${code}-${i}`}
                        candidate={c}
                      />
                    );
                  })}
                </div>
              </div>
            )}

            {!running && !error && candidates.length === 0 && trends.length === 0 && (
              emptyReason
                ? (
                  <div className="rounded border border-blue-500/30 p-2 text-sm text-blue-400">
                    <AlertOutlined className="mr-1" />
                    {emptyReason}
                  </div>
                )
                : (
                  <Empty
                    image={<RobotOutlined style={{ fontSize: 48, opacity: 0.3 }} />}
                    description={t("chat.workflowRunner.noResult")}
                  />
                )
            )}

            {!running && stage === "done" && (
              <div className="flex items-center justify-between pt-1 border-t border-white/5">
                <Text type="secondary" className="text-xs">
                  {t("chat.workflowRunner.completed")}
                </Text>
                <Button
                  size="small"
                  type="text"
                  onClick={onClose}
                >
                  {t("chat.workflowRunner.close")}
                </Button>
              </div>
            )}
          </div>
        </Card>
      )}
    </>
  );
}
