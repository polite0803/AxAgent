// SPDX-License-Identifier: AGPL-3.0-only

import { useEvaluatorStore } from "@/stores/devtools/evaluatorStore";
import type { BenchmarkReport, Difficulty } from "@/types";
import { formatDuration, formatScore, getDifficultyKey } from "@/types";
import { Button, Card, Col, Row, Statistic, Table, Tabs, Tag } from "antd";
import { useTranslation } from "react-i18next";

interface BenchmarkReportViewProps {
  report: BenchmarkReport;
}

export function BenchmarkReportView({ report }: BenchmarkReportViewProps) {
  const { exportReport } = useEvaluatorStore();
  const { t } = useTranslation();

  const columns = [
    { title: t("benchmark.task"), dataIndex: "taskName", key: "taskName" },
    {
      title: t("benchmark.difficulty"),
      dataIndex: "difficulty",
      key: "difficulty",
      render: (d: Difficulty) => t(getDifficultyKey(d)),
    },
    {
      title: t("benchmark.status"),
      dataIndex: "success",
      key: "success",
      render: (success: boolean) => (
        <Tag color={success ? "green" : "red"}>
          {success ? t("benchmark.passed") : t("benchmark.failed")}
        </Tag>
      ),
    },
    {
      title: t("benchmark.score"),
      dataIndex: "score",
      key: "score",
      render: formatScore,
    },
    {
      title: t("benchmark.duration"),
      dataIndex: "durationMs",
      key: "durationMs",
      render: formatDuration,
    },
  ];

  const criteriaColumns = [
    { title: t("benchmark.criteria"), dataIndex: "name", key: "name" },
    {
      title: t("benchmark.score"),
      dataIndex: "score",
      key: "score",
      render: formatScore,
    },
    {
      title: t("benchmark.passed"),
      dataIndex: "passed",
      key: "passed",
      render: (passed: boolean) => <Tag color={passed ? "green" : "red"}>{passed ? "✅" : "❌"}</Tag>,
    },
  ];

  return (
    <div>
      <div className="flex justify-between items-center mb-4">
        <h3 className="text-lg font-semibold">{t("benchmark.reportTitle")}</h3>
        <div className="flex gap-2">
          <Button onClick={() => exportReport("json")}>
            {t("benchmark.exportJson")}
          </Button>
          <Button onClick={() => exportReport("markdown")}>
            {t("benchmark.exportMarkdown")}
          </Button>
        </div>
      </div>

      <Row gutter={16} className="mb-4">
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("benchmark.passRate")}
              value={report.summary.passRate * 100}
              suffix="%"
              precision={1}
              styles={{ content: { color: report.summary.passRate >= 0.7 ? "#52c41a" : "#ff4d4f" } }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("benchmark.overallScore")}
              value={report.summary.overallScore * 100}
              suffix="%"
              precision={1}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("benchmark.taskCount")}
              value={report.summary.totalTasks}
              suffix={`/ ${report.summary.passedTasks} ${t("benchmark.passed")}`}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("benchmark.totalDuration")}
              value={report.summary.totalDurationMs}
              formatter={(val) => formatDuration(Number(val))}
            />
          </Card>
        </Col>
      </Row>

      <Tabs
        defaultActiveKey="tasks"
        items={[
          {
            key: "tasks",
            label: t("benchmark.taskDetails"),
            children: (
              <Table
                dataSource={report.taskBreakdown}
                columns={columns}
                rowKey="taskId"
                size="small"
                pagination={false}
                expandable={{
                  expandedRowRender: (record) => (
                    <div className="p-2">
                      <h4 className="font-medium mb-2">
                        {t("benchmark.scoreDetails")}
                      </h4>
                      <Table
                        dataSource={record.criteriaScores}
                        columns={criteriaColumns}
                        rowKey="name"
                        size="small"
                        pagination={false}
                      />
                    </div>
                  ),
                }}
              />
            ),
          },
          {
            key: "recommendations",
            label: t("benchmark.recommendations"),
            children: (
              <Card>
                <ul className="list-disc pl-5">
                  {report.recommendations.map((rec, _idx) => (
                    <li key={rec} className="mb-2">
                      {rec}
                    </li>
                  ))}
                </ul>
              </Card>
            ),
          },
        ]}
      />
    </div>
  );
}
