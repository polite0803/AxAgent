// i18n-exempt: 业务逻辑/API 描述/日志字符串，非 UI 展示文本
// SPDX-License-Identifier: AGPL-3.0-only

import { PageErrorBoundary } from "@/components/shared/ErrorBoundary";
import { invoke } from "@/lib/invoke";
import { listen } from "@/lib/invoke";
import { usePipelineStore } from "@/stores";
import type { PipelineRun, PipelineStepEvent } from "@/types";
import { Button, Card, Col, Empty, message, Row, Statistic, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { Pause, Play } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const STATUS_TAG_COLOR: Record<string, string> = {
  completed: "green",
  success: "green",
  running: "blue",
  failed: "red",
  error: "red",
  cancelled: "default",
};

function StatusTag({ status }: { status: string }) {
  const color = STATUS_TAG_COLOR[status?.toLowerCase()] ?? "default";
  return <Tag color={color}>{status}</Tag>;
}

function PipelinePageInner() {
  const { t } = useTranslation();
  const pipelineRuns = usePipelineStore((s) => s.pipelineRuns);
  const isRunning = usePipelineStore((s) => s.isRunning);
  const stepEvents = usePipelineStore((s) => s.stepEvents);
  const fetchHistory = usePipelineStore((s) => s.fetchHistory);
  const runPipeline = usePipelineStore((s) => s.runPipeline);
  const clearStepEvents = usePipelineStore((s) => s.clearStepEvents);
  const addStepEvent = usePipelineStore((s) => s.addStepEvent);
  const [pausing, setPausing] = useState(false);
  const [resuming, setResuming] = useState(false);
  const [isPaused, setIsPaused] = useState(false);

  const WORKFLOW_ID = "stock-analysis";

  const handlePause = useCallback(async () => {
    setPausing(true);
    try {
      await invoke("workflow_pause", { workflowId: WORKFLOW_ID });
      message.success(t("chat.workflow.paused"));
      setIsPaused(true);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      message.error(t("chat.workflow.pauseFailed") + ": " + msg);
    } finally {
      setPausing(false);
    }
  }, [t]);

  const handleResume = useCallback(async () => {
    setResuming(true);
    try {
      await invoke("workflow_resume", { workflowId: WORKFLOW_ID });
      message.success(t("chat.workflow.resumed"));
      setIsPaused(false);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      message.error(t("chat.workflow.resumeFailed") + ": " + msg);
    } finally {
      setResuming(false);
    }
  }, [t]);

  useEffect(() => {
    fetchHistory();
    const unlisten = listen<PipelineStepEvent>("pipeline-step", (event) => {
      addStepEvent(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchHistory, addStepEvent]);

  const handleRun = async () => {
    clearStepEvents();
    setIsPaused(false);
    try {
      await runPipeline();
    } catch (e) {
      console.error("[PipelinePage] 执行管道失败:", e);
    }
  };

  const { totalRuns, successRate } = useMemo(() => {
    const total = pipelineRuns.length;
    const success = pipelineRuns.filter((r) =>
      r.status?.toLowerCase() === "completed" || r.status?.toLowerCase() === "success"
    ).length;
    const rate = total > 0 ? Math.round((success / total) * 100) : 0;
    return { totalRuns: total, successRate: rate };
  }, [pipelineRuns]);

  const columns: ColumnsType<PipelineRun> = [
    {
      title: t("pipeline.date"),
      dataIndex: "runDate",
      key: "runDate",
      render: (value: string) => value ?? "-",
    },
    {
      title: t("pipeline.status"),
      dataIndex: "status",
      key: "status",
      render: (status: string) => <StatusTag status={status} />,
    },
    {
      title: t("pipeline.candidates"),
      key: "candidates",
      render: (_: unknown, record: PipelineRun) => record.summary?.discovery?.candidatesFound ?? 0,
    },
    {
      title: t("pipeline.analyzed"),
      key: "analyzed",
      render: (_: unknown, record: PipelineRun) => {
        if (!record.summary) { return 0; }
        return record.summary.analysis.newAnalyzed + record.summary.analysis.reassessed;
      },
    },
    {
      title: t("pipeline.reflectionScheduled"),
      key: "reflectionScheduled",
      render: (_: unknown, record: PipelineRun) => record.summary?.reflectionScheduled ?? 0,
    },
  ];

  return (
    <div className="flex h-full flex-col">
      <div className="flex-1 overflow-auto p-4">
        <Row gutter={[16, 16]}>
          <Col xs={24} sm={8}>
            <Card>
              <Statistic title={t("pipeline.totalRuns")} value={totalRuns} />
            </Card>
          </Col>
          <Col xs={24} sm={8}>
            <Card>
              <Statistic
                title={t("pipeline.successRate")}
                value={successRate}
                suffix="%"
              />
            </Card>
          </Col>
          <Col xs={24} sm={8}>
            <Card
              title={t("pipeline.title")}
              extra={
                <div className="flex items-center gap-1">
                  {isRunning && (
                    <Button
                      size="small"
                      icon={<Pause size={12} />}
                      loading={pausing}
                      onClick={handlePause}
                    >
                      {t("chat.workflow.pause")}
                    </Button>
                  )}
                  {isPaused && (
                    <Button
                      size="small"
                      icon={<Play size={12} />}
                      loading={resuming}
                      onClick={handleResume}
                      className="text-green-500"
                    >
                      {t("chat.workflow.resume")}
                    </Button>
                  )}
                  <Button
                    type="primary"
                    loading={isRunning}
                    onClick={handleRun}
                  >
                    {isRunning ? t("pipeline.running") : t("pipeline.runNow")}
                  </Button>
                </div>
              }
            >
              <Typography.Text type="secondary">
                {isRunning ? t("pipeline.running") : t("pipeline.runNow")}
              </Typography.Text>
            </Card>
          </Col>
        </Row>

        {isRunning && (
          <Card title={t("pipeline.progress")} style={{ marginTop: 16 }}>
            {stepEvents.length === 0
              ? (
                <Empty
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                  description={t("pipeline.running")}
                />
              )
              : (
                <ul style={{ margin: 0, paddingLeft: 20 }}>
                  {stepEvents.map((ev, idx) => (
                    <li key={`${ev.timestamp}-${idx}`}>
                      <Typography.Text strong>{ev.step}</Typography.Text>
                      <Typography.Text type="secondary">
                        {" — "}
                        {ev.detail}
                      </Typography.Text>
                    </li>
                  ))}
                </ul>
              )}
          </Card>
        )}

        <Card title={t("pipeline.history")} style={{ marginTop: 16 }}>
          {pipelineRuns.length === 0
            ? (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={t("pipeline.noHistory")}
              />
            )
            : (
              <Table<PipelineRun>
                rowKey="id"
                columns={columns}
                dataSource={pipelineRuns}
                pagination={{ pageSize: 10 }}
                size="small"
              />
            )}
        </Card>
      </div>
    </div>
  );
}

export function PipelinePage() {
  const { t } = useTranslation();
  return (
    <PageErrorBoundary title={t("pipeline.title")}>
      <PipelinePageInner />
    </PageErrorBoundary>
  );
}
