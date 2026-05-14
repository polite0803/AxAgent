import { useEvaluatorStore } from "@/stores/devtools/evaluatorStore";
import type { BenchmarkReport } from "@/types";
import { formatDuration, formatScore, getDifficultyLabel } from "@/types";
import { Button, Card, Col, Row, Statistic, Table, Tabs, Tag } from "antd";
import { useTranslation } from "react-i18next";

interface BenchmarkReportViewProps {
  report: BenchmarkReport;
}

export function BenchmarkReportView({ report }: BenchmarkReportViewProps) {
  const { exportReport } = useEvaluatorStore();
  const { t } = useTranslation();

  const columns = [
    { title: t("benchmark.task"), dataIndex: "task_name", key: "task_name" },
    { title: t("benchmark.difficulty"), dataIndex: "difficulty", key: "difficulty", render: getDifficultyLabel },
    {
      title: t("benchmark.status"),
      dataIndex: "success",
      key: "success",
      render: (success: boolean) => <Tag color={success ? "green" : "red"}>{success ? t("benchmark.passed") : t("benchmark.failed")}</Tag>,
    },
    { title: t("benchmark.score"), dataIndex: "score", key: "score", render: formatScore },
    { title: t("benchmark.duration"), dataIndex: "duration_ms", key: "duration_ms", render: formatDuration },
  ];

  const criteriaColumns = [
    { title: t("benchmark.criteria"), dataIndex: "name", key: "name" },
    { title: t("benchmark.score"), dataIndex: "score", key: "score", render: formatScore },
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
        <h3 className="text-lg font-bold">{t("benchmark.reportTitle")}</h3>
        <div className="flex gap-2">
          <Button onClick={() => exportReport("json")}>{t("benchmark.exportJson")}</Button>
          <Button onClick={() => exportReport("markdown")}>{t("benchmark.exportMarkdown")}</Button>
        </div>
      </div>

      <Row gutter={16} className="mb-4">
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("benchmark.passRate")}
              value={report.summary.pass_rate * 100}
              suffix="%"
              precision={1}
              valueStyle={{ color: report.summary.pass_rate >= 0.7 ? "#52c41a" : "#ff4d4f" }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("benchmark.overallScore")}
              value={report.summary.overall_score * 100}
              suffix="%"
              precision={1}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("benchmark.taskCount")}
              value={report.summary.total_tasks}
              suffix={`/ ${report.summary.passed_tasks} ${t("benchmark.passed")}`}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("benchmark.totalDuration")}
              value={report.summary.total_duration_ms}
              formatter={(val) => formatDuration(Number(val))}
            />
          </Card>
        </Col>
      </Row>

      <Tabs defaultActiveKey="tasks">
        <Tabs.TabPane tab={t("benchmark.taskDetails")} key="tasks">
          <Table
            dataSource={report.task_breakdown}
            columns={columns}
            rowKey="task_id"
            size="small"
            pagination={false}
            expandable={{
              expandedRowRender: (record) => (
                <div className="p-2">
                  <h4 className="font-medium mb-2">{t("benchmark.scoreDetails")}</h4>
                  <Table
                    dataSource={record.criteria_scores}
                    columns={criteriaColumns}
                    rowKey="name"
                    size="small"
                    pagination={false}
                  />
                </div>
              ),
            }}
          />
        </Tabs.TabPane>

        <Tabs.TabPane tab={t("benchmark.recommendations")} key="recommendations">
          <Card>
            <ul className="list-disc pl-5">
              {report.recommendations.map((rec, idx) => <li key={idx} className="mb-2">{rec}</li>)}
            </ul>
          </Card>
        </Tabs.TabPane>
      </Tabs>
    </div>
  );
}
