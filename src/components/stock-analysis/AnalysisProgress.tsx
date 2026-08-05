import { invoke } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores";
import { Button, message, Progress, Steps, Tag } from "antd";
import { Pause, Play } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const STAGES = [
  "stage.dataLoading",
  "stage.analysis",
  "stage.debate",
  "stage.risk",
  "stage.decision",
];

// 已知的分析师节点 ID（与 workflowChatBridge ANALYST_NODE_TO_NAME 同步）
const ANALYST_NODE_IDS = [
  "a-market-analyst",
  "a-sentiment",
  "a-news",
  "a-fundamentals",
  "a-policy",
  "a-hot-money",
  "a-lockup",
  "a-research",
  "a-sector",
  "a-catalyst",
];
// 最大辩论轮数（bull/bear 配对数），与 workflow template 中的 maxDebateRounds 同步
const TOTAL_DEBATE_ROUNDS = 3;

export function AnalysisProgress() {
  const { t } = useTranslation();
  const status = useStockAnalysisStore((s) => s.status);
  const currentStage = useStockAnalysisStore((s) => s.currentStage);
  const analystReports = useStockAnalysisStore((s) => s.analystReports);
  const debateRounds = useStockAnalysisStore((s) => s.debateRounds);
  const riskAssessments = useStockAnalysisStore((s) => s.riskAssessments);
  const error = useStockAnalysisStore((s) => s.error);
  const llmStatus = useStockAnalysisStore((s) => s.llmStatus);
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const progressMessage = useStockAnalysisStore((s) => s.progressMessage);
  const progressPct = useStockAnalysisStore((s) => s.progressPct);
  const dataWarnings = useStockAnalysisStore((s) => s.dataWarnings);
  const startAnalysis = useStockAnalysisStore((s) => s.startAnalysis);
  const workflowId = useStockAnalysisStore((s) => s.workflowId);
  const [pausing, setPausing] = useState(false);
  const [resuming, setResuming] = useState(false);
  const [localPaused, setLocalPaused] = useState(false);

  // Hooks 必须在 early return 之前 — 保持顺序稳定
  const subProgress = useMemo(() => {
    if (status === "idle") { return null; }
    // 动态计算分析师总数：已知 analyst node ID 中正在被使用的个数
    const analystKeys = Object.keys(analystReports).filter((k) =>
      k !== "investment-plan" && k !== "bull-researcher" && k !== "bear-researcher"
    );
    const activeAnalysts = ANALYST_NODE_IDS.filter((id) => analystKeys.includes(id));
    const totalAnalysts = Math.max(activeAnalysts.length, ANALYST_NODE_IDS.length);
    switch (currentStage) {
      case 1:
        return analystKeys.length > 0
          ? t("stockAnalysis.analystCount", { current: analystKeys.length, total: totalAnalysts })
          : null;
      case 2:
        return debateRounds.length > 0
          ? t("stockAnalysis.debateRounds", { current: debateRounds.length, total: TOTAL_DEBATE_ROUNDS })
          : null;
      case 3:
        return Object.keys(riskAssessments).length > 0
          ? t("stockAnalysis.riskAssessCount", { count: Object.keys(riskAssessments).length })
          : null;
      default:
        return null;
    }
  }, [status, currentStage, analystReports, debateRounds, riskAssessments, t]);

  const handlePause = useCallback(async () => {
    if (!workflowId) { return; }
    setPausing(true);
    try {
      await invoke("workflow_pause", { workflowId });
      message.success(t("chat.workflow.paused"));
      setLocalPaused(true);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      message.error(t("chat.workflow.pauseFailed") + ": " + msg);
    } finally {
      setPausing(false);
    }
  }, [workflowId, t]);

  const handleResume = useCallback(async () => {
    if (!workflowId) { return; }
    setResuming(true);
    try {
      await invoke("workflow_resume", { workflowId });
      message.success(t("chat.workflow.resumed"));
      setLocalPaused(false);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      message.error(t("chat.workflow.resumeFailed") + ": " + msg);
    } finally {
      setResuming(false);
    }
  }, [workflowId, t]);

  // 工作流状态变化时重置暂停状态
  useEffect(() => {
    if (status !== "running" && status !== "loading") {
      setLocalPaused(false);
    }
  }, [status]);

  if (status === "idle") { return null; }

  const currentStep = status === "completed" ? 4 : currentStage;

  return (
    <div>
      {status === "cancelled" && (
        <div
          className="mb-1 p-1.5 rounded flex justify-between items-center text-xs"
          style={{ color: "var(--sa-amber)", background: "rgba(245,158,11,0.08)" }}
        >
          <span>{t("stockAnalysis.cancelled")}</span>
          {stockCode && (
            <Button size="small" onClick={() => startAnalysis(stockCode)}>
              {t("stockAnalysis.retry")}
            </Button>
          )}
        </div>
      )}
      {error && (
        <div
          className="mb-1 p-1.5 rounded flex justify-between items-center text-xs"
          style={{ color: "var(--sa-red)", background: "var(--sa-red-bg)" }}
        >
          <span style={{ flex: 1 }}>{error}</span>
          {stockCode && status === "error" && (
            <Button
              size="small"
              danger
              onClick={() => startAnalysis(stockCode)}
            >
              {t("stockAnalysis.retry")}
            </Button>
          )}
        </div>
      )}
      {llmStatus === "placeholder" && (
        <Tag color="orange" style={{ marginBottom: 8 }}>
          ⚠️ {t("stockAnalysis.offlineMode")}
        </Tag>
      )}

      {/* 数据源降级警告 */}
      {dataWarnings.length > 0 && (
        <div style={{ marginBottom: 8, display: "flex", flexDirection: "column", gap: 4 }}>
          {dataWarnings.map((w, i) => (
            <Tag
              key={i}
              color="orange"
              style={{ fontSize: 11, whiteSpace: "normal", height: "auto", padding: "2px 6px" }}
            >
              {w}
            </Tag>
          ))}
        </div>
      )}
      <Steps
        size="small"
        current={currentStep}
        status={status === "error" ? "error" : "process"}
        items={STAGES.map((s, i) => ({
          title: (
            <span>
              {t(`stockAnalysis.${s}`)}
              {i === currentStep && subProgress && <Tag style={{ marginLeft: 6, fontSize: 10 }}>{subProgress}</Tag>}
            </span>
          ),
        }))}
      />

      {/* 实时进度提示 */}
      {(status === "running" || status === "loading") && progressMessage && (
        <div className="mt-1 flex items-center gap-1.5 text-xs" style={{ color: "var(--muted)" }}>
          <span className="inline-block w-2 h-2 rounded-full bg-blue-500 animate-pulse" />
          <span>{progressMessage}</span>
        </div>
      )}
      {status === "completed" && progressMessage && (
        <div className="mt-1 text-xs" style={{ color: "var(--sa-green)" }}>
          {progressMessage}
        </div>
      )}
      {status === "error" && progressMessage && (
        <div className="mt-1 text-xs" style={{ color: "var(--sa-red)" }}>
          {progressMessage}
        </div>
      )}

      {/* 进度条 */}
      {(status === "running" || status === "loading") && progressPct > 0 && (
        <Progress percent={progressPct} size="small" showInfo={false} className="mt-1" strokeColor="var(--accent)" />
      )}

      {/* 暂停/恢复按钮 */}
      {(status === "running" || status === "loading") && !localPaused && (
        <div className="mt-2 flex items-center gap-1">
          <Button
            size="small"
            icon={<Pause size={12} />}
            loading={pausing}
            onClick={handlePause}
            className="text-xs text-amber-500 hover:text-amber-600"
          >
            {t("chat.workflow.pause")}
          </Button>
        </div>
      )}
      {localPaused && (
        <div className="mt-2 flex items-center gap-1">
          <Button
            size="small"
            icon={<Play size={12} />}
            loading={resuming}
            onClick={handleResume}
            className="text-xs text-green-500 hover:text-green-600"
          >
            {t("chat.workflow.resume")}
          </Button>
        </div>
      )}
    </div>
  );
}
