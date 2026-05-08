import { useExecutionStore } from "@/stores/feature/executionStore";
import type { TrajectoryDetail, TrajectoryStep } from "@/types";
import { theme } from "antd";
import { ChevronLeft, ChevronRight, Loader2, Pause, Play, SkipBack, SkipForward } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import "./TrajectoryReplay.css";

interface TrajectoryReplayProps {
  conversationId: string;
}

const SPEED_OPTIONS = [0.5, 1, 2, 4] as const;

const _EMPTY_TRAJECTORIES: never[] = [];

export function TrajectoryReplay({ conversationId }: TrajectoryReplayProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const fetchList = useExecutionStore((s) => s.fetchTrajectoryList);
  const fetchDetail = useExecutionStore((s) => s.fetchTrajectoryDetail);
  const trajectories = useExecutionStore((s) => s.trajectoriesByConversation[conversationId] || _EMPTY_TRAJECTORIES);
  const loadingList = useExecutionStore((s) => s.loadingTrajectories);
  const details = useExecutionStore((s) => s.trajectoryDetails);

  // 选择状态
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [trajectory, setTrajectory] = useState<TrajectoryDetail | null>(null);
  const [loadingDetail, setLoadingDetail] = useState(false);

  // 播放状态
  const [currentStep, setCurrentStep] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [speed, setSpeed] = useState<number>(1);
  const rafRef = useRef<number | null>(null);
  const lastStepTimeRef = useRef<number>(0);
  const mountedRef = useRef(true);
  const loadingIdRef = useRef<string | null>(null);

  // 卸载标记
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    if (conversationId) {
      fetchList(conversationId);
    }
  }, [conversationId]);

  const handleSelectTrajectory = useCallback(async (id: string) => {
    setSelectedId(id);
    setCurrentStep(0);
    setIsPlaying(false);
    setLoadingDetail(true);
    loadingIdRef.current = id;
    try {
      const detail = details[id] ?? await fetchDetail(id);
      // 防止快速切换时旧结果覆盖新选择
      if (loadingIdRef.current === id) {
        setTrajectory(detail);
      }
    } finally {
      if (loadingIdRef.current === id) {
        setLoadingDetail(false);
      }
    }
  }, [details, fetchDetail]);

  // 播放动画
  const playFrame = useCallback(() => {
    if (!trajectory || !mountedRef.current) { return; }
    const now = performance.now();
    const interval = 1000 / speed;
    if (now - lastStepTimeRef.current >= interval) {
      lastStepTimeRef.current = now;
      setCurrentStep((prev) => {
        if (prev >= trajectory.steps.length - 1) {
          setIsPlaying(false);
          return prev;
        }
        return prev + 1;
      });
    }
    if (mountedRef.current) {
      rafRef.current = requestAnimationFrame(playFrame);
    }
  }, [trajectory, speed]);

  useEffect(() => {
    if (isPlaying) {
      lastStepTimeRef.current = performance.now();
      rafRef.current = requestAnimationFrame(playFrame);
    }
    return () => {
      if (rafRef.current) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
    };
  }, [isPlaying, playFrame]);

  const handlePlayPause = () => {
    if (!trajectory) { return; }
    if (currentStep >= trajectory.steps.length - 1) {
      setCurrentStep(0);
      setIsPlaying(true);
    } else {
      setIsPlaying(!isPlaying);
    }
  };

  const handlePrevStep = () => setCurrentStep((p) => Math.max(0, p - 1));
  const handleNextStep = () => {
    if (!trajectory) { return; }
    setCurrentStep((p) => Math.min(trajectory.steps.length - 1, p + 1));
  };
  const handleGoToStart = () => setCurrentStep(0);
  const handleGoToEnd = () => trajectory && setCurrentStep(trajectory.steps.length - 1);

  // 是否没有数据
  const isEmpty = !loadingList && trajectories.length === 0 && !selectedId;

  // 当前步骤数据
  const step: TrajectoryStep | null = trajectory?.steps[currentStep] ?? null;

  return (
    <div
      className="trajectory-replay"
      style={{ padding: "8px 0", display: "flex", flexDirection: "column", height: "100%" }}
    >
      {/* 轨迹选择器 */}
      <div style={{ padding: "0 8px", flexShrink: 0 }}>
        {loadingList
          ? (
            <span style={{ fontSize: 12, color: token.colorTextQuaternary }}>
              <Loader2 size={12} style={{ display: "inline", marginRight: 4 }} />
              {t("chat.agentPanel.loadingTrajectory")}
            </span>
          )
          : isEmpty
          ? (
            <span style={{ fontSize: 12, color: token.colorTextQuaternary }}>
              {t("chat.agentPanel.noTrajectory")}
            </span>
          )
          : (
            <select
              value={selectedId ?? ""}
              onChange={(e) => {
                if (e.target.value) { handleSelectTrajectory(e.target.value); }
              }}
              style={{
                width: "100%",
                padding: "4px 6px",
                fontSize: 12,
                borderRadius: 4,
                border: `1px solid ${token.colorBorderSecondary}`,
                backgroundColor: token.colorBgContainer,
                color: token.colorText,
              }}
            >
              <option value="">{t("chat.agentPanel.selectTrajectory")}</option>
              {trajectories.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.topic || t.id.slice(0, 8)} — {t.outcome}
                </option>
              ))}
            </select>
          )}
      </div>

      {/* 回放区域 */}
      {loadingDetail
        ? (
          <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
            <Loader2 size={18} style={{ animation: "spin 1s linear infinite", color: token.colorTextQuaternary }} />
          </div>
        )
        : trajectory && step
        ? (
          <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden", marginTop: 8 }}>
            {/* 质量摘要 */}
            {trajectory.quality && (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  padding: "4px 12px",
                  fontSize: 11,
                  color: token.colorTextSecondary,
                }}
              >
                <span>
                  {t("chat.agentPanel.quality")}: {(trajectory.quality.overall * 100).toFixed(0)}%
                </span>
                <span>
                  {t("chat.timeline.completed", "耗时")}: {(trajectory.duration_ms / 1000).toFixed(1)}s
                </span>
                <span>
                  {t("chat.agentPanel.steps")}: {trajectory.steps.length}
                </span>
              </div>
            )}

            {/* 步骤内容 */}
            <div style={{ flex: 1, overflow: "auto", padding: "4px 12px" }}>
              <div
                style={{
                  fontSize: 10,
                  color: token.colorTextQuaternary,
                  marginBottom: 4,
                }}
              >
                {t("chat.agentPanel.step")} {currentStep + 1} / {trajectory.steps.length}
                {step.timestamp_ms > 0 && (
                  <span style={{ marginLeft: 8 }}>
                    +{(step.timestamp_ms / 1000).toFixed(1)}s
                  </span>
                )}
              </div>
              <div
                style={{
                  fontSize: 11,
                  color: token.colorPrimary,
                  fontWeight: 600,
                  marginBottom: 4,
                }}
              >
                {step.role}
              </div>
              {step.reasoning && (
                <div
                  style={{
                    fontSize: 11,
                    color: token.colorTextSecondary,
                    fontStyle: "italic",
                    marginBottom: 6,
                    padding: "4px 8px",
                    backgroundColor: token.colorFillQuaternary,
                    borderRadius: 4,
                  }}
                >
                  {step.reasoning}
                </div>
              )}
              <div
                style={{
                  fontSize: 12,
                  color: token.colorText,
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-word",
                  lineHeight: 1.5,
                }}
              >
                {step.content}
              </div>

              {/* 工具调用 */}
              {step.tool_calls && step.tool_calls.length > 0 && (
                <div style={{ marginTop: 8 }}>
                  {step.tool_calls.map((tc, i) => (
                    <div
                      key={tc.id || i}
                      style={{
                        padding: "6px 8px",
                        marginBottom: 4,
                        backgroundColor: token.colorFillQuaternary,
                        borderRadius: 4,
                        fontSize: 11,
                      }}
                    >
                      <span style={{ color: token.colorWarning, fontWeight: 600 }}>
                        🔧 {tc.name}
                      </span>
                      {tc.input && Object.keys(tc.input).length > 0 && (
                        <pre
                          style={{
                            margin: "4px 0 0",
                            fontSize: 10,
                            color: token.colorTextSecondary,
                            whiteSpace: "pre-wrap",
                            wordBreak: "break-all",
                          }}
                        >
                          {JSON.stringify(tc.input, null, 1)}
                        </pre>
                      )}
                    </div>
                  ))}
                </div>
              )}

              {/* 工具结果 */}
              {step.tool_results && step.tool_results.length > 0 && (
                <div style={{ marginTop: 8 }}>
                  {step.tool_results.map((tr, i) => (
                    <div
                      key={tr.id || i}
                      style={{
                        padding: "6px 8px",
                        marginBottom: 4,
                        backgroundColor: tr.error
                          ? token.colorErrorBg
                          : token.colorSuccessBg,
                        borderRadius: 4,
                        fontSize: 11,
                        color: tr.error ? token.colorError : token.colorTextSecondary,
                      }}
                    >
                      {tr.error ? `❌ ${tr.error}` : tr.output?.slice(0, 300)}
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        )
        : (
          <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
            <span style={{ fontSize: 12, color: token.colorTextQuaternary }}>
              {selectedId ? t("chat.agentPanel.unableToLoad") : t("chat.agentPanel.selectTrajectory")}
            </span>
          </div>
        )}

      {/* 播放控制栏 */}
      {trajectory && (
        <div
          className="trajectory-replay__controls"
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            padding: "6px 12px",
            borderTop: `1px solid ${token.colorBorderSecondary}`,
            flexShrink: 0,
          }}
        >
          <button
            type="button"
            className="trajectory-replay__btn"
            onClick={handleGoToStart}
            title={t("chat.agentPanel.goToStart", "回到开始")}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              color: token.colorTextSecondary,
              padding: 2,
              borderRadius: 4,
              display: "flex",
            }}
          >
            <SkipBack size={14} />
          </button>
          <button
            type="button"
            className="trajectory-replay__btn"
            onClick={handlePrevStep}
            title={t("chat.agentPanel.prevStep", "上一步")}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              color: token.colorTextSecondary,
              padding: 2,
              borderRadius: 4,
              display: "flex",
            }}
          >
            <ChevronLeft size={16} />
          </button>
          <button
            type="button"
            className="trajectory-replay__btn trajectory-replay__btn--play"
            onClick={handlePlayPause}
            title={isPlaying ? t("chat.agentPanel.pause") : t("chat.agentPanel.play")}
            style={{
              border: "none",
              background: token.colorPrimary,
              cursor: "pointer",
              color: "#fff",
              padding: "4px 8px",
              borderRadius: 4,
              display: "flex",
              alignItems: "center",
              gap: 2,
              fontSize: 11,
            }}
          >
            {isPlaying ? <Pause size={14} /> : <Play size={14} />}
            {isPlaying ? t("chat.agentPanel.pause") : t("chat.agentPanel.play")}
          </button>
          <button
            type="button"
            className="trajectory-replay__btn"
            onClick={handleNextStep}
            title={t("chat.agentPanel.nextStep", "下一步")}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              color: token.colorTextSecondary,
              padding: 2,
              borderRadius: 4,
              display: "flex",
            }}
          >
            <ChevronRight size={16} />
          </button>
          <button
            type="button"
            className="trajectory-replay__btn"
            onClick={handleGoToEnd}
            title={t("chat.agentPanel.goToEnd", "跳到结尾")}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              color: token.colorTextSecondary,
              padding: 2,
              borderRadius: 4,
              display: "flex",
            }}
          >
            <SkipForward size={14} />
          </button>

          {/* 速度选择 */}
          <div style={{ flex: 1 }} />
          <div style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 11, color: token.colorTextSecondary }}>
            {t("chat.agentPanel.speed")}:
            {SPEED_OPTIONS.map((s) => (
              <button
                key={s}
                type="button"
                onClick={() => setSpeed(s)}
                style={{
                  border: `1px solid ${speed === s ? token.colorPrimary : token.colorBorderSecondary}`,
                  background: speed === s ? token.colorPrimaryBg : "transparent",
                  cursor: "pointer",
                  color: speed === s ? token.colorPrimary : token.colorTextSecondary,
                  padding: "1px 5px",
                  borderRadius: 3,
                  fontSize: 10,
                }}
              >
                {s}x
              </button>
            ))}
          </div>

          {/* 进度文本 */}
          <span style={{ fontSize: 11, color: token.colorTextQuaternary, marginLeft: 8, flexShrink: 0 }}>
            {currentStep + 1}/{trajectory.steps.length}
          </span>
        </div>
      )}
    </div>
  );
}
