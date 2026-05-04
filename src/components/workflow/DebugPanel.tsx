import {
  BugOutlined,
  CheckCircleOutlined,
  ClockCircleOutlined,
  CloseCircleOutlined,
  LoadingOutlined,
  PauseCircleOutlined,
  ThunderboltOutlined,
} from "@ant-design/icons";
import {
  Badge,
  Card,
  Col,
  Collapse,
  Progress,
  Row,
  Space,
  Statistic,
  Switch,
  Table,
  Tag,
  Timeline,
  Typography,
} from "antd";
import type { ColumnsType } from "antd/es/table";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Title, Text } = Typography;
const { Panel } = Collapse;

interface NodeExecution {
  nodeId: string;
  nodeName: string;
  nodeType: string;
  status: "running" | "completed" | "failed" | "skipped";
  startTime: number;
  endTime?: number;
  duration?: number;
  input: Record<string, unknown>;
  output?: Record<string, unknown>;
  error?: string;
  retryCount: number;
}

interface VariableSnapshot {
  timestamp: number;
  variables: Record<string, unknown>;
}

interface ExecutionTrace {
  id: string;
  workflowId: string;
  workflowName: string;
  status: "running" | "completed" | "failed" | "terminated";
  startTime: number;
  endTime?: number;
  duration?: number;
  nodeExecutions: NodeExecution[];
  variableSnapshots: VariableSnapshot[];
  triggerType: string;
  userId?: string;
}

interface PerformanceMetrics {
  totalDuration: number;
  nodeCount: number;
  successfulNodes: number;
  failedNodes: number;
  skippedNodes: number;
  avgNodeDuration: number;
  maxNodeDuration: { nodeName: string; duration: number };
  minNodeDuration: { nodeName: string; duration: number };
}

const mockNodeExecutions: NodeExecution[] = [
  {
    nodeId: "node-1",
    nodeName: "Trigger",
    nodeType: "trigger",
    status: "completed",
    startTime: Date.now() - 5000,
    endTime: Date.now() - 4900,
    duration: 100,
    input: { type: "webhook" },
    output: { result: "triggered" },
    retryCount: 0,
  },
  {
    nodeId: "node-2",
    nodeName: "Fetch User Input",
    nodeType: "agent",
    status: "completed",
    startTime: Date.now() - 4900,
    endTime: Date.now() - 4200,
    duration: 700,
    input: { query: "weather" },
    output: { data: { location: "Beijing", temp: 25 } },
    retryCount: 0,
  },
  {
    nodeId: "node-3",
    nodeName: "Call LLM",
    nodeType: "llm",
    status: "completed",
    startTime: Date.now() - 4200,
    endTime: Date.now() - 2000,
    duration: 2200,
    input: { prompt: "What is the weather?" },
    output: { response: "The weather is sunny." },
    retryCount: 0,
  },
  {
    nodeId: "node-4",
    nodeName: "Format Response",
    nodeType: "code",
    status: "failed",
    startTime: Date.now() - 2000,
    endTime: Date.now() - 1800,
    duration: 200,
    input: { data: "{}" },
    error: "JSON parse error: unexpected token",
    retryCount: 2,
  },
];

const mockSnapshots: VariableSnapshot[] = [
  {
    timestamp: Date.now() - 5000,
    variables: { triggerType: "webhook", requestId: "req-123" },
  },
  {
    timestamp: Date.now() - 4900,
    variables: {
      triggerType: "webhook",
      requestId: "req-123",
      userQuery: "weather",
    },
  },
  {
    timestamp: Date.now() - 4200,
    variables: {
      triggerType: "webhook",
      requestId: "req-123",
      userQuery: "weather",
      weatherData: { location: "Beijing", temp: 25 },
    },
  },
];

const mockTrace: ExecutionTrace = {
  id: "trace-001",
  workflowId: "wf-001",
  workflowName: "Weather Query Workflow",
  status: "failed",
  startTime: Date.now() - 5000,
  endTime: Date.now(),
  duration: 5000,
  nodeExecutions: mockNodeExecutions,
  variableSnapshots: mockSnapshots,
  triggerType: "webhook",
  userId: "user-001",
};

function calculateMetrics(executions: NodeExecution[]): PerformanceMetrics {
  const completedExecutions = executions.filter((e) => e.status === "completed");
  const failedExecutions = executions.filter((e) => e.status === "failed");
  const skippedExecutions = executions.filter((e) => e.status === "skipped");

  const totalDuration = executions.reduce((sum, e) => sum + (e.duration || 0), 0);
  const durations = completedExecutions.map((e) => ({ nodeName: e.nodeName, duration: e.duration || 0 }));

  const maxNode = durations.reduce(
    (max, curr) => (curr.duration > max.duration ? curr : max),
    { nodeName: "", duration: 0 },
  );
  const minNode = durations.reduce(
    (min, curr) => (curr.duration < min.duration ? curr : min),
    { nodeName: "", duration: Infinity },
  );

  return {
    totalDuration,
    nodeCount: executions.length,
    successfulNodes: completedExecutions.length,
    failedNodes: failedExecutions.length,
    skippedNodes: skippedExecutions.length,
    avgNodeDuration: completedExecutions.length > 0 ? totalDuration / completedExecutions.length : 0,
    maxNodeDuration: maxNode,
    minNodeDuration: minNode.duration === Infinity ? { nodeName: "N/A", duration: 0 } : minNode,
  };
}

function formatDuration(ms: number): string {
  if (ms < 1000) { return `${ms}ms`; }
  if (ms < 60000) { return `${(ms / 1000).toFixed(1)}s`; }
  return `${(ms / 60000).toFixed(1)}m`;
}

function getStatusIcon(status: NodeExecution["status"]) {
  switch (status) {
    case "completed":
      return <CheckCircleOutlined className="text-green-500" />;
    case "failed":
      return <CloseCircleOutlined className="text-red-500" />;
    case "running":
      return <LoadingOutlined className="text-blue-500" />;
    case "skipped":
      return <PauseCircleOutlined className="text-gray-400" />;
  }
}

interface DebugPanelProps {
  trace?: ExecutionTrace;
}

export function DebugPanel({ trace = mockTrace }: DebugPanelProps) {
  const { t } = useTranslation();
  const [showVariables, setShowVariables] = useState(true);
  const [showPerformance, setShowPerformance] = useState(true);
  const [selectedNode, setSelectedNode] = useState<string | null>(null);

  const metrics = useMemo(() => calculateMetrics(trace.nodeExecutions), [trace.nodeExecutions]);

  const nodeColumns: ColumnsType<NodeExecution> = [
    {
      title: "Node",
      dataIndex: "nodeName",
      key: "nodeName",
      render: (name: string, record) => (
        <Space>
          {getStatusIcon(record.status)}
          <Text strong={selectedNode === record.nodeId}>{name}</Text>
          <Tag>{record.nodeType}</Tag>
        </Space>
      ),
    },
    {
      title: "Duration",
      key: "duration",
      render: (_, record) => <Text type="secondary">{record.duration ? formatDuration(record.duration) : "-"}</Text>,
    },
    {
      title: "Retries",
      dataIndex: "retryCount",
      key: "retryCount",
      render: (count) => (count > 0 ? <Tag color="orange">{count}</Tag> : <Text type="secondary">-</Text>),
    },
    {
      title: "Status",
      dataIndex: "status",
      key: "status",
      render: (status) => (
        <Tag color={status === "completed" ? "success" : status === "failed" ? "error" : "default"}>
          {status.toUpperCase()}
        </Tag>
      ),
    },
  ];

  return (
    <div className="h-full flex flex-col" style={{ background: "#252525" }}>
      <div
        className="border-b p-4"
        style={{ borderColor: "#333" }}
      >
        <div className="flex items-center justify-between">
          <Space>
            <BugOutlined />
            <Title level={5} className="m-0">
              Debug Panel
            </Title>
            <Badge
              status={trace.status === "completed" ? "success" : trace.status === "failed" ? "error" : "processing"}
              text={trace.status}
            />
          </Space>
          <Space>
            <Switch size="small" checked={showVariables} onChange={setShowVariables} />
            <Text type="secondary">Variables</Text>
            <Switch size="small" checked={showPerformance} onChange={setShowPerformance} />
            <Text type="secondary">Performance</Text>
          </Space>
        </div>
        <Text type="secondary" className="text-sm">
          Execution: {trace.id} | Workflow: {trace.workflowName} | Trigger: {trace.triggerType}
        </Text>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        <Row gutter={16} className="mb-4">
          <Col span={6}>
            <Card size="small">
              <Statistic
                title={t("workflow.debug.totalDuration")}
                value={metrics.totalDuration}
                formatter={(val) => formatDuration(Number(val))}
                prefix={<ClockCircleOutlined />}
              />
            </Card>
          </Col>
          <Col span={6}>
            <Card size="small">
              <Statistic
                title={t("workflow.debug.successful")}
                value={metrics.successfulNodes}
                valueStyle={{ color: "#3f8600" }}
                prefix={<CheckCircleOutlined />}
              />
            </Card>
          </Col>
          <Col span={6}>
            <Card size="small">
              <Statistic
                title={t("workflow.debug.failed")}
                value={metrics.failedNodes}
                valueStyle={{ color: "#cf1322" }}
                prefix={<CloseCircleOutlined />}
              />
            </Card>
          </Col>
          <Col span={6}>
            <Card size="small">
              <Statistic
                title={t("workflow.debug.avgNodeTime")}
                value={metrics.avgNodeDuration}
                formatter={(val) => formatDuration(Number(val))}
                prefix={<ThunderboltOutlined />}
              />
            </Card>
          </Col>
        </Row>

        <Collapse defaultActiveKey={["execution", "performance"]} className="mb-4">
          <Panel header={t("workflow.debug.nodeExecutions")} key="execution">
            <Table
              columns={nodeColumns}
              dataSource={trace.nodeExecutions}
              rowKey="nodeId"
              size="small"
              pagination={false}
              onRow={(record) => ({
                onClick: () => setSelectedNode(record.nodeId),
                style: { cursor: "pointer" },
              })}
            />
          </Panel>
        </Collapse>

        {showVariables && (
          <Card size="small" className="mb-4">
            <Title level={5} className="mb-3">
              Variable Snapshots
            </Title>
            <Timeline
              items={trace.variableSnapshots.map((snapshot, index) => ({
                color: index === trace.variableSnapshots.length - 1 ? "blue" : "gray",
                children: (
                  <div key={snapshot.timestamp}>
                    <Text type="secondary" className="text-xs">
                      {new Date(snapshot.timestamp).toLocaleTimeString()}
                    </Text>
                    <pre className="text-xs bg-gray-50 p-2 rounded mt-1 overflow-x-auto">
                      {JSON.stringify(snapshot.variables, null, 2)}
                    </pre>
                  </div>
                ),
              }))}
            />
          </Card>
        )}

        {showPerformance && (
          <Card size="small">
            <Title level={5} className="mb-3">
              Performance Analysis
            </Title>
            <div className="space-y-3">
              <div>
                <Text type="secondary">Slowest Node</Text>
                <div className="flex items-center gap-2 mt-1">
                  <Progress
                    percent={Math.min(100, (metrics.maxNodeDuration.duration / metrics.totalDuration) * 100)}
                    size="small"
                    format={() => formatDuration(metrics.maxNodeDuration.duration)}
                  />
                  <Text>{metrics.maxNodeDuration.nodeName}</Text>
                </div>
              </div>
              <div>
                <Text type="secondary">Fastest Node</Text>
                <div className="flex items-center gap-2 mt-1">
                  <Progress
                    percent={metrics.minNodeDuration.duration > 0
                      ? Math.min(
                        100,
                        (metrics.minNodeDuration.duration / metrics.totalDuration) * 100,
                      )
                      : 0}
                    size="small"
                    format={() => formatDuration(metrics.minNodeDuration.duration)}
                  />
                  <Text>{metrics.minNodeDuration.nodeName}</Text>
                </div>
              </div>
            </div>
          </Card>
        )}

        {selectedNode && (
          <Card size="small" className="mt-4" title={t("workflow.debug.selectedNodeDetails")}>
            {(() => {
              const node = trace.nodeExecutions.find((n) => n.nodeId === selectedNode);
              if (!node) { return null; }
              return (
                <div className="space-y-3">
                  <div>
                    <Text type="secondary">Node:</Text>
                    <Text strong>{node.nodeName}</Text>
                  </div>
                  <div>
                    <Text type="secondary">Type:</Text>
                    <Tag>{node.nodeType}</Tag>
                  </div>
                  {node.error && (
                    <div>
                      <Text type="secondary">Error:</Text>
                      <Text type="danger">{node.error}</Text>
                    </div>
                  )}
                  <div>
                    <Text type="secondary">Input:</Text>
                    <pre className="text-xs bg-gray-50 p-2 rounded mt-1">
                      {JSON.stringify(node.input, null, 2)}
                    </pre>
                  </div>
                  {node.output && (
                    <div>
                      <Text type="secondary">Output:</Text>
                      <pre className="text-xs bg-gray-50 p-2 rounded mt-1">
                        {JSON.stringify(node.output, null, 2)}
                      </pre>
                    </div>
                  )}
                </div>
              );
            })()}
          </Card>
        )}
      </div>
    </div>
  );
}
