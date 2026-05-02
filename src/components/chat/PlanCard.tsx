import { usePlanStore } from "@/stores";
import type { Plan, PlanStep, PlanStepStatus } from "@/types";
import {
  CheckCircleFilled,
  CloseCircleFilled,
  LoadingOutlined,
  PlayCircleOutlined,
  RightOutlined,
} from "@ant-design/icons";
import { Button, Progress, Tag, theme, Tooltip } from "antd";
import { AlertTriangle, ClipboardList, Play, RotateCcw, X } from "lucide-react";
import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import "./PlanCard.css";

// ── Status Icons ──────────────────────────────────────────────────────

const statusConfig: Record<PlanStepStatus, { icon: React.ReactNode; color: string; labelKey: string }> = {
  pending: {
    icon: <span className="plan-step-dot" style={{ background: "#d9d9d9" }} />,
    color: "#8c8c8c",
    labelKey: "plan.status.pending",
  },
  approved: {
    icon: <CheckCircleFilled style={{ color: "#52c41a", fontSize: 14 }} />,
    color: "#52c41a",
    labelKey: "plan.status.approved",
  },
  rejected: {
    icon: <CloseCircleFilled style={{ color: "#ff4d4f", fontSize: 14 }} />,
    color: "#ff4d4f",
    labelKey: "plan.status.rejected",
  },
  running: {
    icon: <LoadingOutlined style={{ color: "#1890ff", fontSize: 14 }} />,
    color: "#1890ff",
    labelKey: "plan.status.running",
  },
  completed: {
    icon: <CheckCircleFilled style={{ color: "#52c41a", fontSize: 14 }} />,
    color: "#52c41a",
    labelKey: "plan.status.completed",
  },
  error: {
    icon: <CloseCircleFilled style={{ color: "#ff4d4f", fontSize: 14 }} />,
    color: "#ff4d4f",
    labelKey: "plan.status.error",
  },
};

// ── Progress calculation ──────────────────────────────────────────────

function calcProgress(steps: PlanStep[]): { completed: number; total: number; percent: number } {
  const total = steps.length;
  const completed = steps.filter((s) => s.status === "completed").length;
  const percent = total > 0 ? Math.round((completed / total) * 100) : 0;
  return { completed, total, percent };
}

// ── Component ──────────────────────────────────────────────────────────

interface PlanCardProps {
  plan: Plan;
  conversationId: string;
  /** Whether the plan is in a "resumed from history" state (view only unless explicitly resumed) */
  isHistorical?: boolean;
  /** Callback when the plan execution completes */
  onExecutionComplete?: () => void;
}

export function PlanCard({ plan, conversationId, isHistorical = false }: PlanCardProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const approvePlan = usePlanStore((s) => s.approvePlan);
  const rejectPlan = usePlanStore((s) => s.rejectPlan);
  const modifyStep = usePlanStore((s) => s.modifyStep);
  const resumePlan = usePlanStore((s) => s.resumePlan);
  const cancelPlan = usePlanStore((s) => s.cancelPlan);
  const loading = usePlanStore((s) => s.loading[conversationId]);

  const [expandedSteps, setExpandedSteps] = useState<Set<string>>(new Set());
  const [localSteps, setLocalSteps] = useState<PlanStep[]>(plan.steps);

  // Sync with store updates
  useEffect(() => {
    setLocalSteps(plan.steps);
  }, [plan.steps]);

  const isReviewing = plan.status === "reviewing" || plan.status === "draft";
  const isExecuting = plan.status === "executing";
  const isCompleted = plan.status === "completed";

  const progress = calcProgress(localSteps);

  // ── Handlers ──────────────────────────────────────────────────────

  const handleApproveAll = useCallback(async () => {
    // Mark all pending steps as approved
    for (const step of localSteps) {
      if (step.status === "pending") {
        await modifyStep(conversationId, plan.id, step.id, { approved: true });
      }
    }
    // Execute
    await approvePlan(conversationId, plan.id);
  }, [conversationId, plan.id, localSteps, modifyStep, approvePlan]);

  const handleReject = useCallback(async () => {
    await rejectPlan(conversationId, plan.id);
  }, [conversationId, plan.id, rejectPlan]);

  const handleRejectStep = useCallback(
    async (stepId: string) => {
      await modifyStep(conversationId, plan.id, stepId, { approved: false });
    },
    [conversationId, plan.id, modifyStep],
  );

  const handleResume = useCallback(async () => {
    await resumePlan(conversationId, plan.id);
  }, [conversationId, plan.id, resumePlan]);

  const handleCancel = useCallback(async () => {
    await cancelPlan(conversationId, plan.id);
  }, [conversationId, plan.id, cancelPlan]);

  const toggleStep = useCallback((stepId: string) => {
    setExpandedSteps((prev) => {
      const next = new Set(prev);
      if (next.has(stepId)) { next.delete(stepId); }
      else { next.add(stepId); }
      return next;
    });
  }, []);

  // ── Render ────────────────────────────────────────────────────────

  return (
    <div
      className="plan-card"
      style={{
        border: `1px solid ${token.colorBorderSecondary}`,
        borderRadius: 8,
        backgroundColor: token.colorBgElevated,
        margin: "12px 0",
        overflow: "hidden",
      }}
    >
      {/* Header */}
      <div
        className="plan-card-header"
        style={{
          padding: "12px 16px",
          borderBottom: `1px solid ${token.colorBorderSecondary}`,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <ClipboardList size={18} color="#722ed1" />
          <span style={{ fontWeight: 600, fontSize: 14, color: token.colorText }}>
            {plan.title || t("plan.defaultTitle", "Execution Plan")}
          </span>
          <Tag
            color={isReviewing ? "purple" : isExecuting ? "blue" : isCompleted ? "green" : "default"}
            style={{ fontSize: 11, lineHeight: "18px" }}
          >
            {isReviewing
              ? t("plan.status.reviewing", "Reviewing")
              : isExecuting
              ? t("plan.status.executing", "Executing")
              : isCompleted
              ? t("plan.status.completed", "Completed")
              : plan.status}
          </Tag>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
          {isReviewing && !isHistorical && (
            <>
              <Button
                type="primary"
                size="small"
                icon={<Play size={14} />}
                onClick={handleApproveAll}
                loading={loading}
              >
                {t("plan.approveAll", "Approve & Execute")}
              </Button>
              <Tooltip title={t("plan.reject", "Reject Plan")}>
                <Button
                  size="small"
                  danger
                  icon={<X size={14} />}
                  onClick={handleReject}
                />
              </Tooltip>
            </>
          )}
          {isExecuting && (
            <Button
              size="small"
              danger
              icon={<AlertTriangle size={14} />}
              onClick={handleCancel}
            >
              {t("plan.cancel", "Cancel")}
            </Button>
          )}
          {isHistorical && isReviewing && (
            <Button
              size="small"
              icon={<RotateCcw size={14} />}
              onClick={handleResume}
            >
              {t("plan.resume", "Resume")}
            </Button>
          )}
        </div>
      </div>

      {/* Progress bar (during execution) */}
      {isExecuting && (
        <div style={{ padding: "8px 16px 4px" }}>
          <Progress
            percent={progress.percent}
            size="small"
            format={() => `${progress.completed}/${progress.total}`}
            status="active"
          />
        </div>
      )}

      {/* Steps List */}
      <div className="plan-card-steps" style={{ padding: "4px 0" }}>
        {localSteps.map((step, index) => {
          const config = statusConfig[step.status];
          const isExpanded = expandedSteps.has(step.id);

          return (
            <div
              key={step.id}
              className={`plan-step-item ${isExpanded ? "plan-step-item--expanded" : ""}`}
              style={{
                padding: "8px 16px",
                display: "flex",
                alignItems: "flex-start",
                gap: 10,
                cursor: step.description ? "pointer" : "default",
                transition: "background-color 0.15s",
              }}
              onMouseEnter={(e) => {
                if (step.description) {
                  e.currentTarget.style.backgroundColor = token.colorFillSecondary;
                }
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = "transparent";
              }}
              onClick={() => step.description && toggleStep(step.id)}
            >
              {/* Step number + icon */}
              <div
                style={{
                  width: 24,
                  height: 24,
                  borderRadius: "50%",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  flexShrink: 0,
                  marginTop: 1,
                  fontSize: 12,
                  fontWeight: 600,
                  color: step.status === "completed"
                    ? "#fff"
                    : step.status === "running"
                    ? "#fff"
                    : config.color,
                  backgroundColor: step.status === "completed"
                    ? "#52c41a"
                    : step.status === "running"
                    ? "#1890ff"
                    : "transparent",
                  border: step.status === "pending" || step.status === "approved"
                    ? `2px solid ${config.color}`
                    : "none",
                }}
              >
                {step.status === "running"
                  ? <LoadingOutlined style={{ fontSize: 12 }} />
                  : step.status === "completed"
                  ? <CheckCircleFilled style={{ fontSize: 14 }} />
                  : step.status === "error"
                  ? <CloseCircleFilled style={{ fontSize: 14 }} />
                  : index + 1}
              </div>

              {/* Step content */}
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span
                    style={{
                      fontSize: 13,
                      fontWeight: 500,
                      color: token.colorText,
                      textDecoration: step.status === "rejected" ? "line-through" : "none",
                    }}
                  >
                    {step.title}
                  </span>
                  <Tag
                    color={config.color}
                    style={{ fontSize: 10, lineHeight: "16px", padding: "0 4px" }}
                  >
                    {t(config.labelKey)}
                  </Tag>
                  {step.estimated_tools && step.estimated_tools.length > 0 && (
                    <span style={{ fontSize: 11, color: token.colorTextQuaternary }}>
                      {step.estimated_tools.join(", ")}
                    </span>
                  )}
                </div>

                {/* Expanded description */}
                {isExpanded && step.description && (
                  <div
                    style={{
                      marginTop: 4,
                      fontSize: 12,
                      color: token.colorTextSecondary,
                      lineHeight: 1.5,
                    }}
                  >
                    {step.description}
                  </div>
                )}

                {/* Result after execution */}
                {step.result && (step.status === "completed" || step.status === "error") && (
                  <div
                    style={{
                      marginTop: 6,
                      padding: "6px 10px",
                      borderRadius: 4,
                      backgroundColor: step.status === "error"
                        ? "#fff2f0"
                        : "#f6ffed",
                      border: `1px solid ${step.status === "error" ? "#ffccc7" : "#b7eb8f"}`,
                      fontSize: 12,
                      color: token.colorTextSecondary,
                      lineHeight: 1.4,
                    }}
                  >
                    {step.result}
                  </div>
                )}
              </div>

              {/* Step-level approve/reject (reviewing mode) */}
              {isReviewing && !isHistorical && step.status === "pending" && (
                <div style={{ display: "flex", gap: 4, flexShrink: 0 }}>
                  <Tooltip title={t("plan.approveStep", "Approve")}>
                    <Button
                      type="text"
                      size="small"
                      icon={<CheckCircleFilled style={{ color: "#52c41a", fontSize: 16 }} />}
                      onClick={(e) => {
                        e.stopPropagation();
                        modifyStep(conversationId, plan.id, step.id, { approved: true });
                      }}
                    />
                  </Tooltip>
                  <Tooltip title={t("plan.rejectStep", "Reject")}>
                    <Button
                      type="text"
                      size="small"
                      icon={<CloseCircleFilled style={{ color: "#ff4d4f", fontSize: 16 }} />}
                      onClick={(e) => {
                        e.stopPropagation();
                        handleRejectStep(step.id);
                      }}
                    />
                  </Tooltip>
                </div>
              )}

              {/* Expand indicator */}
              {step.description && (
                <RightOutlined
                  style={{
                    fontSize: 10,
                    color: token.colorTextQuaternary,
                    transition: "transform 0.2s",
                    transform: isExpanded ? "rotate(90deg)" : "rotate(0deg)",
                    marginTop: 6,
                  }}
                />
              )}
            </div>
          );
        })}
      </div>

      {/* Footer: compact action row */}
      {isReviewing && !isHistorical && localSteps.some((s) => s.status === "approved") && (
        <div
          style={{
            padding: "8px 16px",
            borderTop: `1px solid ${token.colorBorderSecondary}`,
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <span style={{ fontSize: 12, color: token.colorTextSecondary }}>
            {localSteps.filter((s) => s.status === "approved").length} / {localSteps.length}{" "}
            {t("plan.stepsApproved", "steps approved")}
          </span>
          <Button
            type="primary"
            size="small"
            icon={<PlayCircleOutlined />}
            onClick={() => approvePlan(conversationId, plan.id)}
            loading={loading}
          >
            {t("plan.executeApproved", "Execute Approved Steps")}
          </Button>
        </div>
      )}
    </div>
  );
}
