// SPDX-License-Identifier: AGPL-3.0-only

import { useIntentClarification } from "@/hooks/useIntentClarification";
import { useVoiceChat } from "@/hooks/useVoiceChat";
import { useApprovalStore } from "@/stores/feature/approvalStore";
import { useExecutionStore } from "@/stores/feature/executionStore";
import { useTTSChannelStore } from "@/stores/feature/ttsChannelStore";
import type { RealtimeConfig, VoiceSessionState } from "@/types";
import { Alert, Button, Empty, Switch, Tag, theme, Typography } from "antd";
import { AlertCircle, Loader, Mic, MicOff, Phone, Volume2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { ApprovalCard } from "../approval/ApprovalCard";
import { DagProgressCard } from "../voice/DagProgressCard";
import type { DagNodeProgress } from "../voice/DagProgressCard";
import { IntentClarificationPanel } from "../voice/IntentClarificationPanel";
import { VoiceWaveform } from "../voice/VoiceWaveform";

interface VoiceCallProps {
  visible: boolean;
  onClose: () => void;
  port?: number;
  host?: string;
  config: RealtimeConfig;
  apiKey: string;
}

function StateBadge({ state }: { state: VoiceSessionState }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const stateConfig: Record<
    VoiceSessionState,
    { color: string; icon: React.ReactNode; label: string }
  > = {
    Idle: { color: token.colorBorder, icon: <Mic size={16} />, label: "idle" },
    Connecting: {
      color: token.colorWarning,
      icon: <Loader size={16} className="animate-spin" />,
      label: "connecting",
    },
    Connected: {
      color: token.colorSuccess,
      icon: <Mic size={16} />,
      label: "connected",
    },
    Speaking: {
      color: token.colorPrimary,
      icon: <Volume2 size={16} className="animate-pulse" />,
      label: "speaking",
    },
    Listening: {
      color: token.colorInfo,
      icon: <Volume2 size={16} />,
      label: "listening",
    },
    Disconnecting: {
      color: token.colorWarning,
      icon: <Loader size={16} className="animate-spin" />,
      label: "disconnecting",
    },
    Error: { color: token.colorError, icon: <AlertCircle size={16} />, label: "stateError" },
  };

  const cfg = stateConfig[state];

  return (
    <Tag color={state === "Error" ? "error" : state === "Connected" ? "success" : "processing"}>
      <span className="flex items-center gap-1">
        {cfg.icon}
        <span>{t(`voice.${cfg.label}`)}</span>
      </span>
    </Tag>
  );
}

export function VoiceCall({
  visible,
  onClose,
  port,
  host,
  config,
  apiKey,
}: VoiceCallProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const { state, isMuted, userTranscript, assistantTranscript, start, stop, toggleMute } = useVoiceChat({
    port,
    host,
    config,
    apiKey,
  });

  const {
    clarification,
    isActive: isClarificationActive,
    start: startClarification,
    answerQuestion,
    requestConfirmation,
    confirm,
    cancel,
    reset: resetClarification,
    setWorkflowExecutionId,
    workflowExecutionId,
  } = useIntentClarification();

  const ttsEnabled = useTTSChannelStore((s) => s.enabled);
  const setTtsEnabled = useTTSChannelStore((s) => s.setEnabled);
  const handleProgressBrief = useTTSChannelStore((s) => s.handleProgressBrief);
  const ttsMessages = useTTSChannelStore((s) => s.messages);

  const pendingApprovals = useApprovalStore((s) => s.pendingApprovals);
  const approvalLoading = useApprovalStore((s) => s.loading);
  const fetchPendingApprovals = useApprovalStore((s) => s.fetchPendingApprovals);

  // 从 executionStore 派生 DAG 状态
  const executionPhase = useExecutionStore((s) => {
    return workflowExecutionId ? s.phases[workflowExecutionId] : undefined;
  });
  const agentStatusMessage = useExecutionStore((s) => {
    return workflowExecutionId ? s.agentStatus[workflowExecutionId] : undefined;
  });

  const [dagNodes, setDagNodes] = useState<DagNodeProgress[]>([]);
  const [dagStatus, setDagStatus] = useState<"idle" | "running" | "completed" | "failed" | "waiting_for_approval">(
    "idle",
  );
  const [dagPercent, setDagPercent] = useState(0);
  const transcriptRef = useRef<HTMLDivElement>(null);

  // 根据 executionPhase 派生 DAG 节点和状态
  useEffect(() => {
    if (!executionPhase && !clarification) { return; }

    if (executionPhase) {
      // phaseMap: ExecutionPhase -> DAG 显示状态
      const phaseMap: Record<string, "idle" | "running" | "completed" | "failed" | "waiting_for_approval"> = {
        idle: "idle",
        planning: "running",
        executing: "running",
        waiting_permission: "waiting_for_approval",
        completed: "completed",
        failed: "failed",
        cancelled: "failed",
      };
      setDagStatus(phaseMap[executionPhase] || "running");

      // 构建动态 DAG 节点
      const phaseNodes: DagNodeProgress[] = [];
      phaseNodes.push({ nodeId: "init", nodeName: t("voice.initializing"), status: "completed", durationMs: 0 });

      if (
        ["planning", "executing", "waiting_permission", "completed", "failed", "cancelled"].includes(executionPhase)
      ) {
        phaseNodes.push({
          nodeId: "plan",
          nodeName: t("voice.planningNode"),
          status: executionPhase === "planning" ? "running" : "completed",
          durationMs: 0,
        });
      }

      if (["executing", "waiting_permission", "completed", "failed", "cancelled"].includes(executionPhase)) {
        const isRunning = executionPhase === "executing";
        const isComplete = executionPhase === "completed";
        phaseNodes.push({
          nodeId: "execute",
          nodeName: agentStatusMessage ? agentStatusMessage.slice(0, 30) : t("voice.executingNode"),
          status: isRunning ? "running" : isComplete ? "completed" : "pending",
          durationMs: 0,
        });
      }

      if (executionPhase === "waiting_permission") {
        phaseNodes.push({ nodeId: "wait", nodeName: t("voice.waitingApproval"), status: "running", durationMs: 0 });
      }

      if (["completed", "failed", "cancelled"].includes(executionPhase)) {
        phaseNodes.push({
          nodeId: "final",
          nodeName: executionPhase === "completed" ? t("voice.completed") : t("voice.failed"),
          status: executionPhase === "completed" ? "completed" : "failed",
          durationMs: 0,
        });
      }

      setDagNodes(phaseNodes);
      const completedCount = phaseNodes.filter((n) => n.status === "completed").length;
      const totalCount = phaseNodes.length;
      setDagPercent(totalCount > 0 ? Math.round((completedCount / totalCount) * 100) : 0);
    } else if (clarification?.state === "submitted") {
      setDagStatus("running");
      setDagNodes([
        { nodeId: "n1", nodeName: t("voice.intent.intentSummary"), status: "completed", durationMs: 120 },
        { nodeId: "n2", nodeName: t("voice.planning"), status: "running" },
        { nodeId: "n3", nodeName: t("voice.waitingExecution"), status: "pending" },
      ]);
      setDagPercent(30);
    }
  }, [executionPhase, agentStatusMessage, clarification, t]);

  // Auto-start when overlay becomes visible
  useEffect(() => {
    if (visible && state === "Idle") {
      start();
    }
  }, [visible, state, start]);

  // Auto-scroll transcript
  useEffect(() => {
    if (transcriptRef.current) {
      transcriptRef.current.scrollTop = transcriptRef.current.scrollHeight;
    }
  }, [userTranscript, assistantTranscript]);

  // 语音转写时自动启动意图澄清
  useEffect(() => {
    if (userTranscript && !clarification) {
      startClarification(userTranscript);
    }
  }, [userTranscript, clarification, startClarification]);

  // 意图确认后设置执行 ID（DAG 状态由 executionStore 派生）
  useEffect(() => {
    if (clarification?.state === "submitted" && clarification.confirmed_intent) {
      setWorkflowExecutionId(`exec_${Date.now()}`);
    }
  }, [clarification, setWorkflowExecutionId]);

  useEffect(() => {
    if (dagStatus === "running") {
      handleProgressBrief({
        brief_type: "workflow_start",
        description: t("voice.tts.workflowStart"),
      });
    } else if (dagStatus === "completed") {
      handleProgressBrief({
        brief_type: "workflow_complete",
        description: t("voice.tts.workflowComplete"),
      });
    } else if (dagStatus === "failed") {
      handleProgressBrief({
        brief_type: "workflow_complete",
        description: t("voice.tts.workflowFailed"),
      });
    } else if (dagStatus === "waiting_for_approval") {
      fetchPendingApprovals();
      handleProgressBrief({
        brief_type: "workflow_waiting",
        description: t("voice.tts.waitingForApproval"),
      });
    }
  }, [dagStatus, handleProgressBrief, fetchPendingApprovals, t]);

  const handleApproved = useCallback(() => {
    fetchPendingApprovals();
  }, [fetchPendingApprovals]);

  const handleRejected = useCallback(() => {
    fetchPendingApprovals();
  }, [fetchPendingApprovals]);

  const handleEndCall = () => {
    stop();
    resetClarification();
    setDagStatus("idle");
    setDagNodes([]);
    setDagPercent(0);
    onClose();
  };

  const handleConfirm = () => {
    confirm();
  };

  const handleCancel = () => {
    cancel();
  };

  const handleRephrase = () => {
    resetClarification();
    if (userTranscript) {
      startClarification(userTranscript);
    }
  };

  const handleSkip = () => {
    // 跳过澄清，直接提交
    if (clarification) {
      requestConfirmation(clarification.original_input);
    }
  };

  if (!visible) {
    return null;
  }

  const isListening = state === "Speaking";
  const isSpeaking = state === "Listening";

  return (
    <div
      className="fixed inset-0 z-[1000] flex flex-col bg-gradient-to-b from-slate-900 via-slate-800 to-slate-900"
      style={{
        background:
          `linear-gradient(180deg, ${token.colorBgLayout} 0%, ${token.colorBgElevated} 50%, ${token.colorBgLayout} 100%)`,
      }}
    >
      {/* 顶部导航栏 */}
      <div
        className="flex items-center justify-between px-6 py-4 border-b"
        style={{ borderColor: token.colorBorderSecondary }}
      >
        <div className="flex items-center gap-3">
          <Typography.Title level={4} style={{ color: token.colorText, margin: 0 }}>
            🎙️ {t("voice.startCall")}
          </Typography.Title>
          <StateBadge state={state} />
          {isClarificationActive && clarification && (
            <Tag
              color={clarification.state === "clarifying"
                ? "blue"
                : clarification.state === "needs_confirmation"
                ? "orange"
                : clarification.state === "submitted"
                ? "green"
                : "default"}
            >
              {t(`voice.intent.${clarification.state}`)}
            </Tag>
          )}
        </div>
        <Button
          size="small"
          icon={<Phone size={16} />}
          onClick={handleEndCall}
          danger
        >
          {t("voice.endCall")}
        </Button>
      </div>

      {/* 主体区域：双栏布局 */}
      <div className="flex-1 flex gap-4 p-4 overflow-hidden">
        {/* 左栏：波形 + 字幕 + 控制 */}
        <div className="flex-1 flex flex-col gap-4 min-w-0">
          {/* SVG 波形可视化 */}
          <div
            className="rounded-xl p-4"
            style={{ background: token.colorBgContainer, border: `1px solid ${token.colorBorderSecondary}` }}
          >
            <VoiceWaveform
              isListening={isListening}
              isSpeaking={isSpeaking}
              analyser={null} // TODO: 从 useVoiceChat 暴露 analyser
              height={100}
            />
          </div>

          {/* 字幕区 */}
          <div
            ref={transcriptRef}
            className="flex-1 overflow-y-auto rounded-xl p-4 flex flex-col gap-3"
            style={{ background: token.colorBgContainer, border: `1px solid ${token.colorBorderSecondary}` }}
          >
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              {t("voice.listening")}
            </Typography.Text>
            {userTranscript
              ? (
                <div
                  className="self-end max-w-[80%] rounded-2xl px-4 py-2 text-sm leading-relaxed"
                  style={{ background: token.colorPrimary, color: token.colorWhite }}
                >
                  {userTranscript}
                </div>
              )
              : (
                <Typography.Text type="secondary" style={{ fontSize: 13, fontStyle: "italic" }}>
                  ...
                </Typography.Text>
              )}

            <Typography.Text type="secondary" style={{ fontSize: 12, marginTop: 8 }}>
              AI
            </Typography.Text>
            {assistantTranscript
              ? (
                <div
                  className="self-start max-w-[80%] rounded-2xl px-4 py-2 text-sm leading-relaxed"
                  style={{ background: token.colorFillSecondary, color: token.colorText }}
                >
                  {assistantTranscript}
                </div>
              )
              : (
                <Typography.Text type="secondary" style={{ fontSize: 13, fontStyle: "italic" }}>
                  ...
                </Typography.Text>
              )}
          </div>

          {/* 控制栏 */}
          <div className="flex items-center justify-center gap-6 py-2">
            <Button
              shape="circle"
              size="large"
              icon={isMuted ? <MicOff size={20} /> : <Mic size={20} />}
              onClick={toggleMute}
              style={{
                width: 56,
                height: 56,
                background: isMuted ? token.colorError : token.colorFillTertiary,
                border: "none",
                color: token.colorWhite,
              }}
              title={t("voice.toggleMute")}
            />
            <Button
              shape="circle"
              size="large"
              icon={<Phone size={24} style={{ transform: "rotate(225deg)" }} />}
              onClick={handleEndCall}
              style={{
                width: 72,
                height: 72,
                background: token.colorError,
                border: "none",
                color: token.colorWhite,
                fontSize: 24,
              }}
              title={t("voice.endCall")}
            />
          </div>
        </div>

        {/* 右栏：意图澄清 + DAG 进度 */}
        <div className="w-80 flex flex-col gap-4 overflow-y-auto">
          {/* 意图澄清面板 */}
          {isClarificationActive && clarification && (
            <IntentClarificationPanel
              clarification={clarification}
              onAnswerQuestion={answerQuestion}
              onConfirm={handleConfirm}
              onCancel={handleCancel}
              onRephrase={handleRephrase}
              onSkip={handleSkip}
            />
          )}

          {/* DAG 执行进度卡 */}
          <DagProgressCard
            visible={dagStatus !== "idle" || dagNodes.length > 0}
            nodes={dagNodes}
            overallStatus={dagStatus === "waiting_for_approval" ? "running" : dagStatus}
            progressPercent={dagPercent}
          />

          {/* 人工审批介入卡 */}
          {dagStatus === "waiting_for_approval" && (
            <div
              className="rounded-xl p-3"
              style={{
                background: token.colorBgContainer,
                border: `1px solid ${token.colorWarning}`,
              }}
            >
              <Alert
                type="warning"
                showIcon
                message={t("approval.waitingForApproval")}
                style={{ marginBottom: 12 }}
              />
              {approvalLoading
                ? (
                  <div style={{ textAlign: "center", padding: 12 }}>
                    <Loader size={16} className="animate-spin" />
                  </div>
                )
                : pendingApprovals.length > 0
                ? (
                  <div className="flex flex-col gap-2">
                    {pendingApprovals
                      .filter((a) => a.status === "pending")
                      .map((approval) => (
                        <ApprovalCard
                          key={approval.id}
                          approval={approval}
                          note=""
                          onApproved={handleApproved}
                          onRejected={handleRejected}
                        />
                      ))}
                  </div>
                )
                : (
                  <Empty
                    description={t("approval.noPending")}
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                  />
                )}
            </div>
          )}

          {/* TTS 语音播报状态 */}
          <div
            className="rounded-xl p-3"
            style={{
              background: token.colorBgContainer,
              border: `1px solid ${token.colorBorderSecondary}`,
            }}
          >
            <div className="flex items-center justify-between mb-2">
              <Typography.Text strong style={{ fontSize: 13 }}>
                🔊 {t("voice.tts.title")}
              </Typography.Text>
              <Switch
                size="small"
                checked={ttsEnabled}
                onChange={setTtsEnabled}
                checkedChildren={t("voice.tts.enabled")}
                unCheckedChildren={t("voice.tts.disabled")}
              />
            </div>
            {ttsMessages.length > 0 && (
              <div className="flex flex-col gap-1 max-h-32 overflow-y-auto">
                {ttsMessages.slice(-3).map((msg) => (
                  <div
                    key={msg.id}
                    className="flex items-center gap-2 text-xs"
                    style={{ color: token.colorTextSecondary }}
                  >
                    <Tag
                      color={msg.channel === "final" ? "purple" : "blue"}
                      style={{ margin: 0, fontSize: 10 }}
                    >
                      {msg.channel === "final" ? "final" : "commentary"}
                    </Tag>
                    <span className="truncate flex-1">{msg.text}</span>
                  </div>
                ))}
              </div>
            )}
            {ttsMessages.length === 0 && (
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                {t("voice.tts.noMessages")}
              </Typography.Text>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
