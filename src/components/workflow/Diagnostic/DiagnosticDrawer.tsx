/**
 * 工作流诊断面板
 *
 * 显示 `runWorkflowDiagnose` 返回的 `DiagnosticReport`，支持：
 * - 严重度统计（error / warning / info）
 * - 类别筛选
 * - 单条 issue 跳转、查看详情
 * - 一键自动修复（auto_fixable=true 时显示修复按钮）
 */
import { useWorkflowEditorStore } from "@/stores/feature/workflowEditorStore";
import { App, Button, Drawer, Empty, Segmented, Space, Spin, Statistic, Tag, Tooltip, Typography } from "antd";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronRight,
  CircleAlert,
  Info,
  Loader2,
  Stethoscope,
  Wand2,
} from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useShallow } from "zustand/react/shallow";
import type { DiagnosticCategory, DiagnosticIssue, DiagnosticSeverity } from "../types/workflow.types";

const { Text } = Typography;

type FilterMode = "all" | DiagnosticSeverity;
type CategoryFilter = "all" | DiagnosticCategory;

interface DiagnosticDrawerProps {
  open: boolean;
  onClose: () => void;
  onJumpToNode?: (nodeId: string) => void;
}

const SEVERITY_ORDER: DiagnosticSeverity[] = ["error", "warning", "info"];

function renderMessage(template: string, params?: Record<string, string | number>): string {
  if (!params) { return template; }
  return template.replace(/\{\{\s*(\w+)\s*\}\}/g, (_, key) => {
    const v = params[key];
    return v === undefined || v === null ? `{{${key}}}` : String(v);
  });
}

export function DiagnosticDrawer({ open, onClose, onJumpToNode }: DiagnosticDrawerProps): React.JSX.Element {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const [filter, setFilter] = useState<FilterMode>("all");
  const [category, setCategory] = useState<CategoryFilter>("all");

  const { report, loading, applying, applyFix } = useWorkflowEditorStore(
    useShallow((s) => ({
      report: s.diagnoseReport,
      loading: s.diagnoseLoading,
      applying: false as boolean,
      applyFix: s.applyDiagnoseFix,
    })),
  );

  const issues = report?.issues ?? [];
  const summary = report?.summary ?? { error: 0, warning: 0, info: 0 };

  const filtered = useMemo(() => {
    return issues.filter((iss) => {
      if (filter !== "all" && iss.severity !== filter) { return false; }
      if (category !== "all" && iss.category !== category) { return false; }
      return true;
    });
  }, [issues, filter, category]);

  const groupedBySeverity = useMemo(() => {
    const m: Record<DiagnosticSeverity, DiagnosticIssue[]> = { error: [], warning: [], info: [] };
    for (const iss of filtered) { m[iss.severity].push(iss); }
    return m;
  }, [filtered]);

  const handleFix = (issue: DiagnosticIssue) => {
    if (!issue.auto_fixable) { return; }
    const ok = applyFix(issue.id);
    if (ok) {
      message.success(t("workflow.diagnostic.fix.applied"));
    } else {
      message.error(t("workflow.diagnostic.fix.failed"));
    }
  };

  const handleJump = (issue: DiagnosticIssue) => {
    const first = issue.node_ids?.[0];
    if (first && onJumpToNode) { onJumpToNode(first); }
  };

  const totalCount = issues.length;
  const totalLabel = t("workflow.diagnostic.summary.total", { count: totalCount });

  return (
    <Drawer
      title={
        <Space>
          <Stethoscope size={18} />
          <span>{t("workflow.diagnostic.title")}</span>
          {totalCount > 0 && <Tag color="default">{totalLabel}</Tag>}
        </Space>
      }
      placement="right"
      size={520}
      open={open}
      onClose={onClose}
      destroyOnHidden={false}
    >
      {loading
        ? (
          <div className="flex flex-col items-center justify-center gap-2 py-16 text-gray-500">
            <Spin indicator={<Loader2 className="animate-spin" size={28} />} />
            <Text type="secondary">{t("workflow.diagnostic.loading")}</Text>
          </div>
        )
        : report === null
        ? <Empty description={t("workflow.diagnostic.empty")} />
        : totalCount === 0
        ? (
          <div className="flex flex-col items-center justify-center gap-2 py-16">
            <CheckCircle2 size={36} className="text-green-500" />
            <Text type="secondary">{t("workflow.diagnostic.allGood")}</Text>
          </div>
        )
        : (
          <div className="flex flex-col gap-3">
            <div className="grid grid-cols-3 gap-2">
              {SEVERITY_ORDER.map((sev) => (
                <div
                  key={sev}
                  className="rounded-md border border-gray-200 px-3 py-2 dark:border-gray-700"
                >
                  <Statistic
                    title={
                      <span className="flex items-center gap-1 text-xs">
                        {sev === "error" && <CircleAlert size={12} className="text-red-500" />}
                        {sev === "warning" && <AlertTriangle size={12} className="text-amber-500" />}
                        {sev === "info" && <Info size={12} className="text-blue-500" />}
                        {t(`workflow.diagnostic.severity.${sev}`)}
                      </span>
                    }
                    value={summary[sev]}
                    valueStyle={{
                      fontSize: 18,
                      color: sev === "error" ? "#ef4444" : sev === "warning" ? "#f59e0b" : "#3b82f6",
                    }}
                  />
                </div>
              ))}
            </div>

            <Space direction="vertical" size={4} className="w-full">
              <Text type="secondary" className="text-xs">
                {t("workflow.diagnostic.severity.error")}/{t("workflow.diagnostic.severity.warning")}/{t(
                  "workflow.diagnostic.severity.info",
                )}
              </Text>
              <Segmented<FilterMode>
                value={filter}
                onChange={setFilter}
                block
                options={[
                  { label: t("workflow.diagnostic.summary.title"), value: "all" },
                  { label: t("workflow.diagnostic.severity.error"), value: "error" },
                  { label: t("workflow.diagnostic.severity.warning"), value: "warning" },
                  { label: t("workflow.diagnostic.severity.info"), value: "info" },
                ]}
              />
              <Segmented<CategoryFilter>
                value={category}
                onChange={setCategory}
                block
                options={[
                  { label: t("workflow.diagnostic.summary.title"), value: "all" },
                  { label: t("workflow.diagnostic.category.structure"), value: "structure" },
                  { label: t("workflow.diagnostic.category.configuration"), value: "configuration" },
                  { label: t("workflow.diagnostic.category.prompt"), value: "prompt" },
                  { label: t("workflow.diagnostic.category.prompt_quality"), value: "prompt_quality" },
                  { label: t("workflow.diagnostic.category.performance"), value: "performance" },
                  { label: t("workflow.diagnostic.category.cost"), value: "cost" },
                  { label: t("workflow.diagnostic.category.security"), value: "security" },
                  { label: t("workflow.diagnostic.category.reference"), value: "reference" },
                  { label: t("workflow.diagnostic.category.best_practice"), value: "best_practice" },
                ]}
              />
            </Space>

            {filtered.length === 0
              ? <Empty description={t("workflow.diagnostic.allGood")} />
              : SEVERITY_ORDER.map((sev) => {
                const list = groupedBySeverity[sev];
                if (list.length === 0) { return null; }
                return (
                  <div key={sev} className="flex flex-col gap-2">
                    {list.map((iss) => {
                      const title = iss.title_override
                        || t(`workflow.diagnostic.issues.${iss.id}.title`, { defaultValue: iss.id });
                      const msgTpl = iss.detail_override
                        || t(`workflow.diagnostic.issues.${iss.id}.message`, { defaultValue: "" });
                      const msg = msgTpl ? renderMessage(msgTpl, iss.message_params) : "";
                      const sevColor = sev === "error" ? "red" : sev === "warning" ? "gold" : "blue";
                      return (
                        <div
                          key={iss.id + ":" + (iss.node_ids?.[0] ?? "")}
                          className="rounded-md border border-gray-200 p-2 dark:border-gray-700"
                        >
                          <div className="flex items-start justify-between gap-2">
                            <Space size={4} align="start">
                              <Tag color={sevColor} className="!m-0">
                                {t(`workflow.diagnostic.severity.${iss.severity}`)}
                              </Tag>
                              <Tag color="default" className="!m-0">
                                {t(`workflow.diagnostic.category.${iss.category}`)}
                              </Tag>
                            </Space>
                            <Space size={4}>
                              {iss.node_ids?.[0] && (
                                <Tooltip title={t("workflow.diagnostic.jump")}>
                                  <Button
                                    size="small"
                                    type="text"
                                    icon={<ChevronRight size={14} />}
                                    onClick={() => handleJump(iss)}
                                  />
                                </Tooltip>
                              )}
                              {iss.auto_fixable && (
                                <Tooltip title={t("workflow.diagnostic.fix.apply")}>
                                  <Button
                                    size="small"
                                    type="primary"
                                    icon={<Wand2 size={14} />}
                                    loading={applying}
                                    onClick={() => handleFix(iss)}
                                  >
                                    {t("workflow.diagnostic.fix.apply")}
                                  </Button>
                                </Tooltip>
                              )}
                            </Space>
                          </div>
                          <div className="mt-1 text-sm font-medium">{title}</div>
                          {msg && <div className="text-xs text-gray-500 dark:text-gray-400">{msg}</div>}
                          {iss.suggestion_override && (
                            <div className="text-xs text-blue-500 dark:text-blue-400 mt-1 flex items-start gap-1">
                              <Info size={12} aria-hidden="true" className="mt-0.5 shrink-0" />
                              <span>{iss.suggestion_override}</span>
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                );
              })}
          </div>
        )}
    </Drawer>
  );
}
