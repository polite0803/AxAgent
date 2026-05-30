import { useWorkflowEditorStore } from "@/stores";
import {
  BugOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  ExclamationCircleOutlined,
  PlayCircleOutlined,
  ReloadOutlined,
  StopOutlined,
  ThunderboltOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import {
  Badge,
  Button,
  Card,
  Col,
  Collapse,
  Empty,
  List,
  Row,
  Space,
  Statistic,
  Table,
  Tag,
  theme,
  Tooltip,
  Typography,
} from "antd";
import type { ColumnsType } from "antd/es/table";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Title, Text, Paragraph } = Typography;
const { Panel } = Collapse;

interface ValidationError {
  error_type: string;
  node_id?: string;
  message: string;
  suggestion?: string;
}

interface ValidationWarning {
  warning_type: string;
  node_id?: string;
  message: string;
}

interface ValidationResult {
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

interface NodeDiagnostic {
  nodeId: string;
  nodeName: string;
  nodeType: string;
  hasSourceEdge: boolean;
  hasTargetEdge: boolean;
  isOrphan: boolean;
  isDeadEnd: boolean;
  toolMissing?: string;
  modelEmpty?: boolean;
  promptEmpty?: boolean;
  issueCount: number;
}

function analyzeNodes(nodes: any[], edges: any[]): NodeDiagnostic[] {
  const sources = new Set(edges.map((e: any) => e.source));
  const targets = new Set(edges.map((e: any) => e.target));

  return nodes.map((n) => {
    const isOrphan = !sources.has(n.id) && !targets.has(n.id);
    const isDeadEnd = sources.has(n.id) && !targets.has(n.id);
    const hasSourceEdge = targets.has(n.id);
    const hasTargetEdge = sources.has(n.id);
    let issueCount = 0;
    let toolMissing: string | undefined;
    let modelEmpty: boolean | undefined;
    let promptEmpty: boolean | undefined;

    if (isOrphan) { issueCount++; }
    if (isDeadEnd) { issueCount++; }

    // 类型特定检查
    const nt = n.type || (n.data?.type) || "";
    if (nt === "tool") {
      const tn = n.config?.tool_name || n.data?.config?.tool_name || n.data?.tool_name;
      if (!tn) {
        issueCount++;
        toolMissing = "(empty)";
      }
    }
    if (nt === "agent" || nt === "llm") {
      const sp = n.config?.system_prompt || n.data?.config?.system_prompt || n.data?.system_prompt;
      if (!sp) {
        issueCount++;
        promptEmpty = true;
      }
    }
    if (nt === "subWorkflow") {
      const sid = n.config?.sub_workflow_id || n.data?.config?.sub_workflow_id || n.data?.subWorkflowId;
      if (!sid) { issueCount++; }
    }

    return {
      nodeId: n.id,
      nodeName: n.title || n.data?.title || n.data?.label || n.id,
      nodeType: nt || "unknown",
      hasSourceEdge,
      hasTargetEdge,
      isOrphan,
      isDeadEnd,
      toolMissing,
      modelEmpty,
      promptEmpty,
      issueCount,
    };
  });
}

function analyzeEdges(edges: any[], nodeIds: Set<string>): { invalidSource: number; invalidTarget: number }[] {
  let invalidSource = 0;
  let invalidTarget = 0;
  for (const e of edges) {
    if (!nodeIds.has(e.source)) { invalidSource++; }
    if (!nodeIds.has(e.target)) { invalidTarget++; }
  }
  return [{ invalidSource, invalidTarget }];
}

function findCycles(edges: any[]): string[][] {
  const adj = new Map<string, string[]>();
  for (const e of edges) {
    if (!adj.has(e.source)) { adj.set(e.source, []); }
    adj.get(e.source)!.push(e.target);
  }

  const cycles: string[][] = [];
  const visited = new Set<string>();
  const stack = new Set<string>();

  function dfs(node: string, path: string[]) {
    visited.add(node);
    stack.add(node);
    for (const next of adj.get(node) || []) {
      if (!visited.has(next)) {
        dfs(next, [...path, next]);
      } else if (stack.has(next)) {
        const cycleStart = path.indexOf(next);
        if (cycleStart >= 0) {
          cycles.push(path.slice(cycleStart));
        }
      }
    }
    stack.delete(node);
  }

  for (const node of adj.keys()) {
    if (!visited.has(node)) { dfs(node, [node]); }
  }
  return cycles;
}

function findUnreachableNodes(nodes: any[], edges: any[]): string[] {
  const reachable = new Set<string>();
  const adj = new Map<string, string[]>();
  for (const e of edges) {
    if (!adj.has(e.source)) { adj.set(e.source, []); }
    adj.get(e.source)!.push(e.target);
  }

  // BFS from trigger nodes
  const queue = nodes.filter((n) => {
    const t = n.type || n.data?.type || "";
    return t === "trigger";
  }).map((n) => n.id);

  for (const q of queue) { reachable.add(q); }
  while (queue.length > 0) {
    const curr = queue.shift()!;
    for (const next of adj.get(curr) || []) {
      if (!reachable.has(next)) {
        reachable.add(next);
        queue.push(next);
      }
    }
  }

  return nodes.filter((n) => !reachable.has(n.id)).map((n) => n.id);
}

interface DebugPanelProps {
  trace?: any;
  workflowId?: string;
}

export function DebugPanel({ trace, workflowId: _workflowId }: DebugPanelProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [activeTab, setActiveTab] = useState<"static" | "runtime">("static");
  const [validationResult, setValidationResult] = useState<ValidationResult | null>(null);
  const [validating, setValidating] = useState(false);

  const nodes = useWorkflowEditorStore((s) => s.nodes);
  const edges = useWorkflowEditorStore((s) => s.edges);
  const validateTemplate = useWorkflowEditorStore((s) => s.validateTemplate);

  const nodeIds = useMemo(() => new Set(nodes.map((n: any) => n.id)), [nodes]);
  const diagnostics = useMemo(() => analyzeNodes(nodes, edges), [nodes, edges]);
  const edgeAnalysis = useMemo(() => analyzeEdges(edges, nodeIds), [edges, nodeIds]);
  const cycles = useMemo(() => findCycles(edges), [edges]);
  const unreachable = useMemo(() => findUnreachableNodes(nodes, edges), [nodes, edges]);

  const issuesWithCode = useMemo(() => {
    const count = diagnostics.reduce((s, d) => s + d.issueCount, 0);
    return count + edgeAnalysis[0].invalidSource + edgeAnalysis[0].invalidTarget + cycles.length + unreachable.length;
  }, [diagnostics, edgeAnalysis, cycles, unreachable]);

  const runValidation = useCallback(async () => {
    setValidating(true);
    try {
      const result = await validateTemplate();
      setValidationResult(result as ValidationResult | null);
    } finally {
      setValidating(false);
    }
  }, [validateTemplate]);

  const nodeDiagnosticColumns: ColumnsType<NodeDiagnostic> = [
    {
      title: t("workflow.debug.colNode"),
      dataIndex: "nodeName",
      key: "nodeName",
      ellipsis: true,
      render: (name: string, r: NodeDiagnostic) => (
        <Space size={4}>
          {r.isOrphan && (
            <Tooltip title={t("workflow.debug.orphan")}>
              <WarningOutlined style={{ color: "#faad14" }} />
            </Tooltip>
          )}
          {r.isDeadEnd && (
            <Tooltip title={t("workflow.debug.deadEnd")}>
              <StopOutlined style={{ color: "#ff4d4f" }} />
            </Tooltip>
          )}
          <Text>{name}</Text>
          <Tag>{r.nodeType}</Tag>
        </Space>
      ),
    },
    {
      title: t("workflow.debug.colIssues"),
      key: "issues",
      width: 150,
      render: (_: any, r: NodeDiagnostic) => (
        <Space size={4} wrap>
          {r.isOrphan && <Tag color="warning">{t("workflow.debug.orphan")}</Tag>}
          {r.isDeadEnd && <Tag color="error">{t("workflow.debug.deadEnd")}</Tag>}
          {r.toolMissing !== undefined && <Tag color="error">Tool: {r.toolMissing}</Tag>}
          {r.promptEmpty && <Tag color="warning">{t("workflow.debug.noPrompt")}</Tag>}
          {r.issueCount === 0 && <Tag color="success">OK</Tag>}
        </Space>
      ),
    },
  ];

  const staticPanel = (
    <div className="flex-1 overflow-y-auto p-4" style={{ minHeight: 0 }}>
      <Row gutter={12} className="mb-4">
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("workflow.debug.totalNodes")}
              value={nodes.length}
              prefix={<ThunderboltOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("workflow.debug.totalEdges")}
              value={edges.length}
              prefix={<PlayCircleOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("workflow.debug.issuesFound")}
              value={issuesWithCode}
              valueStyle={{ color: issuesWithCode > 0 ? token.colorError : token.colorSuccess }}
              prefix={issuesWithCode > 0 ? <CloseCircleOutlined /> : <CheckCircleOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size="small">
            <Statistic
              title={t("workflow.debug.cyclesDetected")}
              value={cycles.length}
              valueStyle={{ color: cycles.length > 0 ? token.colorError : token.colorSuccess }}
              prefix={cycles.length > 0 ? <ExclamationCircleOutlined /> : <CheckCircleOutlined />}
            />
          </Card>
        </Col>
      </Row>

      <Collapse defaultActiveKey={["nodes", "validate"]} className="mb-4">
        <Panel header={`${t("workflow.debug.nodeDiagnostics")} (${diagnostics.length})`} key="nodes">
          <Table
            columns={nodeDiagnosticColumns}
            dataSource={diagnostics}
            rowKey="nodeId"
            size="small"
            pagination={false}
            scroll={{ y: 200 }}
          />
        </Panel>

        <Panel
          header={
            <Space>
              {t("workflow.debug.structuralValidation")}
              {validating && <Badge status="processing" />}
              {!validating && validationResult && (
                <Badge
                  status={validationResult.errors.length > 0 ? "error" : "success"}
                  text={`${validationResult.errors.length} errors, ${validationResult.warnings.length} warnings`}
                />
              )}
            </Space>
          }
          key="validate"
          extra={
            <Button
              size="small"
              icon={<ReloadOutlined />}
              loading={validating}
              onClick={(e) => {
                e.stopPropagation();
                runValidation();
              }}
            >
              {t("workflow.debug.runValidation")}
            </Button>
          }
        >
          {!validationResult
            ? (
              <div className="text-center py-6">
                <Text type="secondary">{t("workflow.debug.clickToValidate")}</Text>
              </div>
            )
            : (
              <Space direction="vertical" className="w-full">
                {validationResult.errors.length > 0 && (
                  <List
                    size="small"
                    header={
                      <Text type="danger" strong>
                        {t("workflow.debug.errors", { count: validationResult.errors.length })}
                      </Text>
                    }
                    dataSource={validationResult.errors}
                    renderItem={(err) => (
                      <List.Item>
                        <Space direction="vertical" size={0} className="w-full">
                          <Space>
                            <CloseCircleOutlined style={{ color: token.colorError }} />
                            <Text>{err.message}</Text>
                            {err.node_id && <Tag>{err.node_id}</Tag>}
                          </Space>
                          {err.suggestion && <Text type="secondary" className="text-xs">{err.suggestion}</Text>}
                        </Space>
                      </List.Item>
                    )}
                  />
                )}
                {validationResult.warnings.length > 0 && (
                  <List
                    size="small"
                    header={
                      <Text type="warning" strong>
                        {t("workflow.debug.warnings", { count: validationResult.warnings.length })}
                      </Text>
                    }
                    dataSource={validationResult.warnings}
                    renderItem={(warn) => (
                      <List.Item>
                        <Space direction="vertical" size={0}>
                          <Space>
                            <WarningOutlined style={{ color: token.colorWarning }} />
                            <Text>{warn.message}</Text>
                            {warn.node_id && <Tag>{warn.node_id}</Tag>}
                          </Space>
                        </Space>
                      </List.Item>
                    )}
                  />
                )}
                {validationResult.errors.length === 0 && validationResult.warnings.length === 0 && (
                  <div className="text-center py-4">
                    <CheckCircleOutlined style={{ color: token.colorSuccess, fontSize: 24 }} />
                    <br />
                    <Text type="success" strong>{t("workflow.debug.allClear")}</Text>
                  </div>
                )}
              </Space>
            )}
        </Panel>

        <Panel
          header={`${t("workflow.debug.topoAnalysis")}${cycles.length > 0 ? ` (⚠ ${cycles.length} cycles)` : ""}${
            unreachable.length > 0 ? ` (⚡ ${unreachable.length} unreachable)` : ""
          }`}
          key="topology"
        >
          {cycles.length > 0 && (
            <Card size="small" type="inner" className="mb-2">
              <Text type="danger" strong>{t("workflow.debug.cyclesDetected")}: {cycles.length}</Text>
              {cycles.map((c, i) => (
                <Paragraph key={i} className="mt-1 mb-0" code>
                  {c.join(" → ")}
                </Paragraph>
              ))}
            </Card>
          )}

          {unreachable.length > 0 && (
            <Card size="small" type="inner" className="mb-2">
              <Text type="warning" strong>
                {t("workflow.debug.unreachableNodesCount", { count: unreachable.length })}
              </Text>
              <div className="flex flex-wrap gap-1 mt-1">
                {unreachable.map((id) => <Tag key={id}>{id}</Tag>)}
              </div>
            </Card>
          )}

          {edgeAnalysis[0].invalidSource === 0 && edgeAnalysis[0].invalidTarget === 0 && cycles.length === 0
            && unreachable.length === 0 && (
            <div className="text-center py-4">
              <CheckCircleOutlined style={{ color: token.colorSuccess, fontSize: 24 }} />
              <br />
              <Text type="success" strong>{t("workflow.debug.topoHealthy")}</Text>
            </div>
          )}
        </Panel>
      </Collapse>

      {/* 启动校验按钮 */}
      <div className="text-center">
        <Button
          type="primary"
          icon={<BugOutlined />}
          onClick={runValidation}
          loading={validating}
        >
          {t("workflow.debug.runFullCheck")}
        </Button>
      </div>
    </div>
  );

  const runtimePanel = trace
    ? (
      <div className="flex-1 overflow-y-auto p-4" style={{ minHeight: 0 }}>
        <Empty description={t("workflow.debug.executionMode")} />
      </div>
    )
    : (
      <div className="flex-1 flex items-center justify-center" style={{ minHeight: 0 }}>
        <Empty description={t("workflow.debug.noRuntimeData")}>
          <Text type="secondary">{t("workflow.debug.staticDebugHint")}</Text>
        </Empty>
      </div>
    );

  return (
    <div
      className="h-full flex flex-col"
      style={{ background: token.colorBgElevated }}
    >
      <div
        className="border-b p-3 flex items-center justify-between shrink-0"
        style={{ borderColor: token.colorBorderSecondary }}
      >
        <Space>
          <BugOutlined />
          <Title level={5} className="m-0">{t("workflow.debug.title")}</Title>
          {issuesWithCode > 0 && <Tag color="error">{issuesWithCode} issues</Tag>}
        </Space>
        <Space>
          <Button
            size="small"
            type={activeTab === "static" ? "primary" : "default"}
            onClick={() => setActiveTab("static")}
          >
            {t("workflow.debug.staticCheck")}
          </Button>
          <Button
            size="small"
            type={activeTab === "runtime" ? "primary" : "default"}
            onClick={() => setActiveTab("runtime")}
            disabled={!trace}
          >
            {t("workflow.debug.runtimeTrace")}
          </Button>
        </Space>
      </div>

      {activeTab === "static" ? staticPanel : runtimePanel}
    </div>
  );
}
