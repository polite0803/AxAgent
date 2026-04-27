import { useEvaluatorStore } from "@/stores/devtools/evaluatorStore";
import type { BenchmarkReport } from "@/types/evaluator";
import { Card, Row, Col, Table, Tag, Button, Tabs, Statistic } from "antd";
import { formatScore, formatDuration, getDifficultyLabel } from "@/types/evaluator";

interface BenchmarkReportViewProps {
  report: BenchmarkReport;
}

export function BenchmarkReportView({ report }: BenchmarkReportViewProps) {
  const { exportReport } = useEvaluatorStore();

  const columns = [
    { title: "任务", dataIndex: "task_name", key: "task_name" },
    { title: "难度", dataIndex: "difficulty", key: "difficulty", render: getDifficultyLabel },
    { title: "状态", dataIndex: "success", key: "success", render: (success: boolean) => (
      <Tag color={success ? "green" : "red"}>{success ? "通过" : "失败"}</Tag>
    )},
    { title: "得分", dataIndex: "score", key: "score", render: formatScore },
    { title: "耗时", dataIndex: "duration_ms", key: "duration_ms", render: formatDuration },
  ];

  const criteriaColumns = [
    { title: "评估项", dataIndex: "name", key: "name" },
    { title: "得分", dataIndex: "score", key: "score", render: formatScore },
    { title: "通过", dataIndex: "passed", key: "passed", render: (passed: boolean) => (
      <Tag color={passed ? "green" : "red"}>{passed ? "✅" : "❌"}</Tag>
    )},
  ];

  return (
    <div>
      <div className="flex justify-between items-center mb-4">
        <h3 className="text-lg font-bold">测试报告</h3>
        <div className="flex gap-2">
          <Button onClick={() => exportReport("json")}>导出 JSON</Button>
          <Button onClick={() => exportReport("markdown")}>导出 Markdown</Button>
        </div>
      </div>

      <Row gutter={16} className="mb-4">
        <Col span={6}>
          <Card size="small">
            <Statistic
              title="通过率"
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
              title="总体得分"
              value={report.summary.overall_score * 100}
              suffix="%"
              precision={1}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title="任务数"
              value={report.summary.total_tasks}
              suffix={`/ ${report.summary.passed_tasks} 通过`}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title="总耗时"
              value={report.summary.total_duration_ms}
              formatter={(val) => formatDuration(Number(val))}
            />
          </Card>
        </Col>
      </Row>

      <Tabs defaultActiveKey="tasks">
        <Tabs.TabPane tab="任务详情" key="tasks">
          <Table
            dataSource={report.task_breakdown}
            columns={columns}
            rowKey="task_id"
            size="small"
            pagination={false}
            expandable={{
              expandedRowRender: (record) => (
                <div className="p-2">
                  <h4 className="font-medium mb-2">评分详情</h4>
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

        <Tabs.TabPane tab="建议" key="recommendations">
          <Card>
            <ul className="list-disc pl-5">
              {report.recommendations.map((rec, idx) => (
                <li key={idx} className="mb-2">{rec}</li>
              ))}
            </ul>
          </Card>
        </Tabs.TabPane>
      </Tabs>
    </div>
  );
}
