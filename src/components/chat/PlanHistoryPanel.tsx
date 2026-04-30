import { usePlanStore } from "@/stores";
import type { Plan } from "@/types";
import { CheckCircleFilled, CloseCircleFilled } from "@ant-design/icons";
import { Badge, Button, Drawer, Tag, theme, Tooltip } from "antd";
import { ClipboardList, History, RotateCcw } from "lucide-react";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface PlanHistoryPanelProps {
  conversationId: string;
}

const statusConfig: Record<string, { color: string; labelKey: string }> = {
  reviewing: { color: "purple", labelKey: "plan.status.reviewing" },
  executing: { color: "blue", labelKey: "plan.status.executing" },
  completed: { color: "green", labelKey: "plan.status.completed" },
  cancelled: { color: "default", labelKey: "plan.status.cancelled" },
};

export function PlanHistoryPanel({ conversationId }: PlanHistoryPanelProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [open, setOpen] = useState(false);

  const activePlan = usePlanStore((s) => s.activePlans[conversationId]);
  const history = usePlanStore((s) => s.planHistory[conversationId] || []);
  const loadPlanHistory = usePlanStore((s) => s.loadPlanHistory);
  const resumePlan = usePlanStore((s) => s.resumePlan);

  // Merge active plan with history for display
  const allPlans: Plan[] = React.useMemo(() => {
    const seen = new Set<string>();
    const plans: Plan[] = [];
    if (activePlan) {
      seen.add(activePlan.id);
      plans.push(activePlan);
    }
    for (const p of history) {
      if (!seen.has(p.id)) {
        seen.add(p.id);
        plans.push(p);
      }
    }
    return plans;
  }, [activePlan, history]);

  useEffect(() => {
    if (open) {
      void loadPlanHistory(conversationId);
    }
  }, [open, conversationId, loadPlanHistory]);

  const handleResume = async (planId: string) => {
    await resumePlan(conversationId, planId);
  };

  const formatTime = (ts: number) => {
    const d = new Date(ts);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 1) return t("plan.justNow", "just now");
    if (diffMin < 60) return t("plan.minutesAgo", "{{n}}m ago").replace("{{n}}", String(diffMin));
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return t("plan.hoursAgo", "{{n}}h ago").replace("{{n}}", String(diffHr));
    return d.toLocaleDateString();
  };

  const activeCount = allPlans.filter(
    (p) => p.status === "reviewing" || p.status === "executing" || p.status === "draft",
  ).length;

  return (
    <>
      <Tooltip title={t("plan.historyTitle", "Plan History")}>
        <Button
          type="text"
          size="small"
          icon={
            <Badge count={activeCount} size="small" offset={[-4, 4]} color="#722ed1">
              <ClipboardList size={14} />
            </Badge>
          }
          onClick={() => setOpen(true)}
        />
      </Tooltip>

      <Drawer
        title={
          <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <History size={16} />
            {t("plan.historyTitle", "Plan History")}
          </span>
        }
        placement="right"
        width={380}
        open={open}
        onClose={() => setOpen(false)}
        styles={{ body: { padding: 0 } }}
      >
        <div style={{ padding: "12px 16px" }}>
          {allPlans.length === 0 ? (
            <div
              style={{
                textAlign: "center",
                padding: "40px 0",
                color: token.colorTextSecondary,
                fontSize: 13,
              }}
            >
              <ClipboardList size={32} style={{ opacity: 0.3, marginBottom: 12 }} />
              <div>{t("plan.noPlans", "No plans yet")}</div>
              <div style={{ fontSize: 12, marginTop: 4, color: token.colorTextQuaternary }}>
                {t("plan.noPlansHint", "Switch to Plan First strategy to create one")}
              </div>
            </div>
          ) : (
            allPlans.map((plan) => {
              const config = statusConfig[plan.status] || statusConfig.completed;
              const canResume = plan.status === "reviewing" || plan.status === "draft";

              return (
                <div
                  key={plan.id}
                  style={{
                    border: `1px solid ${token.colorBorderSecondary}`,
                    borderRadius: 8,
                    padding: "12px",
                    marginBottom: 8,
                    backgroundColor: token.colorBgElevated,
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 6 }}>
                    <span style={{ fontWeight: 500, fontSize: 13, color: token.colorText, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {plan.title}
                    </span>
                    <div style={{ display: "flex", alignItems: "center", gap: 6, flexShrink: 0, marginLeft: 8 }}>
                      <Tag color={config.color} style={{ fontSize: 10, lineHeight: "18px", padding: "0 4px", margin: 0 }}>
                        {t(config.labelKey)}
                      </Tag>
                      {canResume && (
                        <Tooltip title={t("plan.resume", "Resume")}>
                          <Button
                            type="text"
                            size="small"
                            icon={<RotateCcw size={12} />}
                            onClick={() => handleResume(plan.id)}
                          />
                        </Tooltip>
                      )}
                    </div>
                  </div>

                  <div style={{ fontSize: 11, color: token.colorTextQuaternary, marginBottom: 4 }}>
                    {plan.steps.length} {t("plan.stepsApproved", "steps")} · {formatTime(plan.created_at)}
                  </div>

                  {plan.steps.length > 0 && (
                    <div style={{ fontSize: 11, color: token.colorTextSecondary }}>
                      {plan.steps.slice(0, 3).map((step) => (
                        <div
                          key={step.id}
                          style={{
                            display: "flex",
                            alignItems: "center",
                            gap: 4,
                            marginTop: 2,
                          }}
                        >
                          {step.status === "completed"
                            ? <CheckCircleFilled style={{ color: "#52c41a", fontSize: 10 }} />
                            : step.status === "error"
                            ? <CloseCircleFilled style={{ color: "#ff4d4f", fontSize: 10 }} />
                            : <span style={{ width: 10, height: 10, borderRadius: "50%", border: "1px solid #d9d9d9", display: "inline-block" }} />}
                          <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                            {step.title}
                          </span>
                        </div>
                      ))}
                      {plan.steps.length > 3 && (
                        <div style={{ color: token.colorTextQuaternary, marginTop: 2 }}>
                          +{plan.steps.length - 3} more
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })
          )}
        </div>
      </Drawer>
    </>
  );
}
