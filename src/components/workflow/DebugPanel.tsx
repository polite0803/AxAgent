// SPDX-License-Identifier: AGPL-3.0-only

import { countTerminalNodes, isDeadEndNode } from "@/components/workflow/DebugPanel/deadEnd";
import { invoke } from "@/lib/invoke";
import { findCyclicSCCs } from "@/lib/workflowLayout";
import { useWorkflowEditorStore } from "@/stores";
import { useWorkEngineStore } from "@/stores/feature/workEngineStore";
import {
  BugOutlined,
  CaretRightOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  CodeOutlined,
  ExclamationCircleOutlined,
  EyeOutlined,
  FastForwardOutlined,
  PauseOutlined,
  PlayCircleOutlined,
  ReloadOutlined,
  StepForwardOutlined,
  StopOutlined,
  ThunderboltOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { theme } from "antd";
import {
  Badge,
  Button,
  Card,
  Col,
  Collapse,
  Descriptions,
  Divider,
  Empty,
  List,
  Modal,
  Row,
  Space,
  Statistic,
  Switch,
  Table,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import type { ColumnsType } from "antd/es/table";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ExecutionStatusResponse, NodeExecutionRecord } from "../../types";

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
  hasIncoming: boolean;
  hasOutgoing: boolean;
  isOrphan: boolean;
  isDeadEnd: boolean;
  toolMissing?: string;
  promptEmpty?: boolean;
  issueCount: number;
}

/// 兼容编辑器层 + DAG 原始格式推断节点真实类型
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function resolveNodeType(n: any): string {
  // 1. 编辑器 ReactFlow 节点（优先走 data.type / node.type）
  if (n.type && n.type !== "base") { return n.type; }
  if (n.data?.type) { return n.data.type; }

  // 2. DAG 原始 WorkflowNode 变体：检查特化字段推断类型
  const cfg = n.config || n.data?.config || {};
  if (cfg.trigger_type) { return "trigger"; }
  if (cfg.system_prompt) { return "agent"; }
  if (cfg.prompt && !cfg.system_prompt) { return "llm"; }
  if (cfg.tool_name) { return "tool"; }
  if (cfg.sub_workflow_id) { return "subWorkflow"; }
  if (cfg.target_workflow_id) { return "workflowRef"; }
  if (cfg.conditions) { return "condition"; }
  if (cfg.cases) { return "switch"; }
  // end: output_var 但无其他 config 特化字段
  if (cfg.output_var) { return "end"; }

  // 3. 按 base.id 中的前缀推断（如 ToolNode::xxx）
  const baseId = n.base?.id || n.id || "";
  if (baseId.startsWith("tool_")) { return "tool"; }
  if (baseId.startsWith("agent_")) { return "agent"; }
  if (baseId.startsWith("llm_")) { return "llm"; }
  return "unknown";
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function analyzeNodes(nodes: any[], edges: any[]): NodeDiagnostic[] {
  const sources = new Set(edges.map((e) => e.source));
  const targets = new Set(edges.map((e) => e.target));

  // 一次扫描计算工作流级终端计数，避免 O(n²)
  const summaries = nodes.map((n) => ({
    id: n.id || "",
    nodeType: resolveNodeType(n),
    hasIncoming: targets.has(n.id),
    hasOutgoing: sources.has(n.id),
  }));
  const totalTerminals = countTerminalNodes(summaries);

  return nodes.map((n) => {
    const nt = resolveNodeType(n);
    const hasIncoming = targets.has(n.id);
    const hasOutgoing = sources.has(n.id);
    const isOrphan = !hasOutgoing && !hasIncoming;
    const isStart = nt === "trigger";
    const isDeadEnd = isDeadEndNode(
      { id: n.id, nodeType: nt, hasIncoming, hasOutgoing },
      totalTerminals,
    );
    let issueCount = 0;
    let toolMissing: string | undefined;
    let promptEmpty: boolean | undefined;

    if (isOrphan && !isStart) { issueCount++; }
    if (isDeadEnd) { issueCount++; }

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
      hasIncoming,
      hasOutgoing,
      isOrphan,
      isDeadEnd,
      toolMissing,
      promptEmpty,
      issueCount,
    };
  });
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function analyzeEdges(edges: any[], nodeIds: Set<string>): { invalidSource: number; invalidTarget: number }[] {
  let invalidSource = 0;
  let invalidTarget = 0;
  for (const e of edges) {
    if (!nodeIds.has(e.source)) { invalidSource++; }
    if (!nodeIds.has(e.target)) { invalidTarget++; }
  }
  return [{ invalidSource, invalidTarget }];
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function findCycles(edges: any[]): string[][] {
  // 复用 workflowLayout 的 Tarjan SCC 算法，避免重复实现
  const nodeIds = new Set<string>();
  for (const e of edges) {
    nodeIds.add(e.source);
    nodeIds.add(e.target);
  }
  const nodes = Array.from(nodeIds).map((id) => ({ id }));
  return findCyclicSCCs(nodes, edges);
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function findUnreachableNodes(nodes: any[], edges: any[]): string[] {
  const reachable = new Set<string>();
  const adj = new Map<string, string[]>();
  for (const e of edges) {
    if (!adj.has(e.source)) { adj.set(e.source, []); }
    adj.get(e.source)!.push(e.target);
  }

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

function formatDuration(ms: number | null): string {
  if (ms == null) { return "-"; }
  if (ms < 1000) { return `${ms}ms`; }
  return `${(ms / 1000).toFixed(2)}s`;
}

function statusColor(status: string): string {
  switch (status) {
    case "completed":
      return "success";
    case "partially_completed":
      return "warning";
    case "running":
      return "processing";
    case "failed":
    case "timeout":
      return "error";
    case "skipped":
      return "default";
    case "paused":
      return "warning";
    default:
      return "default";
  }
}

interface DebugPanelProps {
  workflowId?: string;
}

export function DebugPanel({ workflowId }: DebugPanelProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [activeTab, setActiveTab] = useState<"static" | "runtime">("static");
  const [validationResult, setValidationResult] = useState<ValidationResult | null>(null);
  const [validating, setValidating] = useState(false);
  const [detailRecord, setDetailRecord] = useState<NodeExecutionRecord | null>(null);
  const [subExecutionDetail, setSubExecutionDetail] = useState<ExecutionStatusResponse | null>(null);
  const [subExecutionLoading, setSubExecutionLoading] = useState(false);

  const nodes = useWorkflowEditorStore((s) => s.nodes);
  const edges = useWorkflowEditorStore((s) => s.edges);
  const validateTemplate = useWorkflowEditorStore((s) => s.validateTemplate);

  // 用字段级 selector 订阅，避免任何 workEngine store 字段更新都触发重渲染
  const executionId = useWorkEngineStore((s) => s.executionId);
  const status = useWorkEngineStore((s) => s.status);
  const nodeRecords = useWorkEngineStore((s) => s.nodeRecords);
  const variables = useWorkEngineStore((s) => s.variables);
  const breakpoints = useWorkEngineStore((s) => s.breakpoints);
  const loading = useWorkEngineStore((s) => s.loading);
  const dryRun = useWorkEngineStore((s) => s.dryRun);
  const isDebugRunning = useWorkEngineStore((s) => s.isDebugRunning);
  const lastDebugError = useWorkEngineStore((s) => s.lastDebugError);
  const executionHistory = useWorkEngineStore((s) => s.executionHistory);
  // actions：引用稳定，订阅动作不会触发重渲染
  const debugRun = useWorkEngineStore((s) => s.debugRun);
  const cancelRun = useWorkEngineStore((s) => s.cancel);
  const resumeBreakpoint = useWorkEngineStore((s) => s.resumeBreakpoint);
  const stepBreakpoint = useWorkEngineStore((s) => s.stepBreakpoint);
  const getStatus = useWorkEngineStore((s) => s.getStatus);
  const loadHistory = useWorkEngineStore((s) => s.loadHistory);
  const pauseRun = useWorkEngineStore((s) => s.pause);
  const resumeRun = useWorkEngineStore((s) => s.resume);
  const setDryRun = useWorkEngineStore((s) => s.setDryRun);
  const toggleBreakpoint = useWorkEngineStore((s) => s.toggleBreakpoint);

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const nodeIds = useMemo(() => new Set(nodes.map((n: any) => n.id)), [nodes]);
  const diagnostics = useMemo(() => analyzeNodes(nodes, edges), [nodes, edges]);
  const edgeAnalysis = useMemo(() => analyzeEdges(edges, nodeIds), [edges, nodeIds]);
  const cycles = useMemo(() => findCycles(edges), [edges]);
  const unreachable = useMemo(() => findUnreachableNodes(nodes, edges), [nodes, edges]);

  const issuesWithCode = useMemo(() => {
    const count = diagnostics.reduce((s, d) => s + d.issueCount, 0);
    return count + edgeAnalysis[0].invalidSource + edgeAnalysis[0].invalidTarget + cycles.length + unreachable.length;
  }, [diagnostics, edgeAnalysis, cycles, unreachable]);

  const [subDiags, setSubDiags] = useState<
    Record<string, {
      name: string;
      diagnostics: NodeDiagnostic[];
      cycles: number;
      unreachable: number;
      mappingIssues?: string[];
      templateExists?: boolean;
    }>
  >({});
  const [subAnalyzing, setSubAnalyzing] = useState(false);

  const analyzeSubWorkflows = useCallback(async () => {
    setSubAnalyzing(true);
    const subNodes =
      (nodes as unknown as { type?: string; data?: Record<string, unknown>; config?: Record<string, unknown> }[])
        .filter((n) => {
          const t = n.type || n.data?.type || "";
          return t === "subWorkflow";
        });
    const result: Record<string, unknown> = {};
    if (subNodes.length === 0) {
      setSubAnalyzing(false);
      return;
    }
    const recursionErrors: string[] = [];

    function checkRecursiveRef(
      currentId: string,
      path: string[],
      pathSet: Set<string>,
    ): void {
      if (pathSet.has(currentId)) {
        recursionErrors.push([...path, currentId].join(" → "));
        return;
      }
      pathSet.add(currentId);
      const subNode = subNodes.find((n: { config?: Record<string, unknown>; data?: Record<string, unknown> }) => {
        const cfg = n.config as Record<string, unknown> | undefined;
        const d = n.data as Record<string, unknown> | undefined;
        const sid = (cfg?.sub_workflow_id ?? (d?.config as Record<string, unknown> | undefined)?.["sub_workflow_id"]
          ?? d?.subWorkflowId ?? d?.sub_workflow_id) as string | undefined;
        return sid === currentId;
      });
      if (subNode) {
        const cfg = subNode.config as Record<string, unknown> | undefined;
        const d = subNode.data as Record<string, unknown> | undefined;
        const nextId = (cfg?.sub_workflow_id ?? (d?.config as Record<string, unknown> | undefined)?.["sub_workflow_id"]
          ?? d?.subWorkflowId ?? d?.sub_workflow_id) as string | undefined;
        if (nextId) {
          checkRecursiveRef(nextId, [...path, currentId], new Set(pathSet));
        }
      }
    }

    for (const sn of subNodes) {
      const s = sn as unknown as { [key: string]: unknown };
      const sCfg = s["config"] as { [key: string]: unknown } | undefined;
      const sData = s["data"] as { [key: string]: unknown } | undefined;
      const subId =
        (sCfg?.sub_workflow_id || (sData?.config as Record<string, unknown> | undefined)?.["sub_workflow_id"]
          || sData?.subWorkflowId || sData?.sub_workflow_id) as string | undefined;
      if (!subId) { continue; }
      if (subId === workflowId) {
        recursionErrors.push(`${s["title"] || s["id"]} → self`);
        continue;
      }
      checkRecursiveRef(subId, [workflowId || "root"], new Set([workflowId || "root"]));
    }

    for (const sn of subNodes) {
      const s = sn as unknown as { [key: string]: unknown };
      const sCfg = s["config"] as { [key: string]: unknown } | undefined;
      const sData = s["data"] as { [key: string]: unknown } | undefined;
      const subId =
        (sCfg?.sub_workflow_id || (sData?.config as Record<string, unknown> | undefined)?.["sub_workflow_id"]
          || sData?.subWorkflowId || sData?.sub_workflow_id) as string | undefined;
      try {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const tmpl: any = await invoke("get_workflow_template", { id: subId });
        if (!tmpl?.nodes || !Array.isArray(tmpl.nodes)) { continue; }
        const subN = tmpl.nodes;
        const subE = tmpl.edges || [];
        const diags = analyzeNodes(subN, subE);
        const cyc = findCycles(subE).length;
        const unreach = findUnreachableNodes(subN, subE).length;

        const inputMapping = sCfg?.input_mapping
          || (sData?.config as Record<string, unknown> | undefined)?.["input_mapping"] || {};
        const subInputSchema = tmpl.input_schema || {};
        const mappingIssues: string[] = [];
        if (typeof inputMapping === "object" && Object.keys(inputMapping).length > 0) {
          const schemaProps = (subInputSchema as Record<string, unknown>)?.properties as Record<string, unknown> || {};
          for (const key of Object.keys(inputMapping)) {
            if (Object.keys(schemaProps).length > 0 && !schemaProps[key]) {
              mappingIssues.push(`input "${key}" not in sub-workflow schema`);
            }
          }
        }

        result[s["id"] as string] = {
          name: tmpl.name || subId,
          diagnostics: diags,
          cycles: cyc,
          unreachable: unreach,
          mappingIssues,
          templateExists: true,
        };
      } catch {
        result[s["id"] as string] = {
          name: subId,
          diagnostics: [],
          cycles: 0,
          unreachable: 0,
          mappingIssues: ["Template not found or deleted"],
          templateExists: false,
        };
      }
    }

    if (result) {
      (result as Record<string, unknown>)["_recursionErrors"] = recursionErrors;
    }
    setSubDiags(
      result as unknown as Record<
        string,
        {
          name: string;
          diagnostics: NodeDiagnostic[];
          cycles: number;
          unreachable: number;
          mappingIssues?: string[];
          templateExists?: boolean;
        }
      >,
    );
    setSubAnalyzing(false);
  }, [nodes, workflowId]);

  useEffect(() => {
    const subNodes =
      (nodes as unknown as { type?: string; data?: Record<string, unknown>; config?: Record<string, unknown> }[])
        .filter((n) => {
          const t = n.type || n.data?.type || "";
          return t === "subWorkflow";
        });
    if (subNodes.length === 0) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setSubDiags({});
      return;
    }
    const timer = setTimeout(() => {
      analyzeSubWorkflows();
    }, 500);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [nodes]);

  const runValidation = useCallback(async () => {
    setValidating(true);
    try {
      const result = await validateTemplate();
      setValidationResult(result as ValidationResult | null);
    } finally {
      setValidating(false);
    }
  }, [validateTemplate]);

  // effect 中删除了 setupEventListeners 调用, 已迁移到 WorkflowEditor

  useEffect(() => {
    if (!workflowId) { return; }
    loadHistory(workflowId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workflowId]);

  useEffect(() => {
    if (!executionId || !isDebugRunning) { return; }
    const interval = setInterval(() => {
      getStatus(executionId);
    }, 2000);
    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [executionId, isDebugRunning]);

  const handleDebugRun = useCallback(async () => {
    if (!workflowId) { return; }
    await debugRun(workflowId, {
      breakpoints: breakpoints.length > 0 ? breakpoints : undefined,
      dryRun,
    });
    setActiveTab("runtime");
  }, [workflowId, breakpoints, dryRun, debugRun]);

  const handleCancel = useCallback(async () => {
    await cancelRun();
    if (executionId) {
      await getStatus(executionId);
    }
  }, [cancelRun, executionId, getStatus]);

  const handleResumeBreakpoint = useCallback(async () => {
    await resumeBreakpoint();
  }, [resumeBreakpoint]);

  const handleStepBreakpoint = useCallback(async () => {
    await stepBreakpoint();
  }, [stepBreakpoint]);

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
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
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

  const recordColumns: ColumnsType<NodeExecutionRecord> = [
    {
      title: t("workflow.debug.colNode"),
      key: "node",
      ellipsis: true,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      render: (_: any, r: NodeExecutionRecord) => (
        <Space size={4}>
          {r.status === "running" && <Badge status="processing" />}
          {r.status === "completed" && <CheckCircleOutlined style={{ color: token.colorSuccess }} />}
          {r.status === "failed" && <CloseCircleOutlined style={{ color: token.colorError }} />}
          {r.status === "skipped" && <StopOutlined style={{ color: token.colorTextQuaternary }} />}
          <Text>{r.node_name || r.node_id}</Text>
          {r.sub_workflow_id && (
            <Tooltip title={`Sub-Workflow: ${r.sub_workflow_id}`}>
              <Tag color="blue" style={{ fontSize: 10 }}>sub</Tag>
            </Tooltip>
          )}
        </Space>
      ),
    },
    {
      title: t("workflow.debug.colType"),
      dataIndex: "node_type",
      key: "node_type",
      width: 100,
      render: (v: string) => <Tag>{v}</Tag>,
    },
    {
      title: t("workflow.debug.colStatus"),
      dataIndex: "status",
      key: "status",
      width: 90,
      render: (v: string) => <Tag color={statusColor(v)}>{v}</Tag>,
    },
    {
      title: t("workflow.debug.colTime"),
      dataIndex: "execution_time_ms",
      key: "time",
      width: 80,
      render: (v: number | null) => formatDuration(v),
    },
    {
      title: "",
      key: "actions",
      width: 40,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      render: (_: any, r: NodeExecutionRecord) => (
        <Tooltip title={t("workflow.debug.viewDetail")}>
          <Button
            type="text"
            size="small"
            icon={<EyeOutlined />}
            onClick={() => setDetailRecord(r)}
          />
        </Tooltip>
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

        {Object.keys(subDiags).length > 0 && (
          <Panel header={`Sub-Workflows (${Object.keys(subDiags).length})${subAnalyzing ? " ..." : ""}`} key="subs">
            {((subDiags as unknown as { _recursionErrors?: string[] })._recursionErrors?.length ?? 0) > 0 && (
              <Card size="small" type="inner" className="mb-2">
                <Text type="danger" strong>Recursive References Detected</Text>
                {(subDiags as unknown as { _recursionErrors?: string[] })._recursionErrors?.map((path, i) => (
                  <Paragraph key={i} className="mt-1 mb-0" code type="danger">
                    {path}
                  </Paragraph>
                ))}
              </Card>
            )}
            {Object.entries(subDiags).filter(([k]) =>
              k !== "_recursionErrors"
            ).map(([nodeId, info]) => {
              const totalIssues = info.diagnostics.reduce((s: number, d: NodeDiagnostic) => s + d.issueCount, 0)
                + info.cycles + info.unreachable + (info.mappingIssues?.length || 0);
              return (
                <Card key={nodeId} size="small" type="inner" className="mb-2" title={info.name}>
                  <Space size="small" className="mb-2">
                    <Tag>{info.diagnostics.length} nodes</Tag>
                    {totalIssues > 0
                      ? <Tag color="error">{totalIssues} issues</Tag>
                      : <Tag color="success">clean</Tag>}
                    {info.cycles > 0 && <Tag color="error">{info.cycles} cycles</Tag>}
                    {info.unreachable > 0 && <Tag color="warning">{info.unreachable} unreachable</Tag>}
                    {info.templateExists === false && <Tag color="error">NOT FOUND</Tag>}
                  </Space>
                  {(info.mappingIssues && info.mappingIssues.length > 0) && (
                    <div className="mb-2">
                      {info.mappingIssues.map((issue: string, i: number) => (
                        <Tag key={i} color="warning" className="mb-1">{issue}</Tag>
                      ))}
                    </div>
                  )}
                  {info.diagnostics.length > 0 && (
                    <Table
                      columns={nodeDiagnosticColumns}
                      dataSource={info.diagnostics}
                      rowKey="nodeId"
                      size="small"
                      pagination={false}
                      scroll={{ y: 160 }}
                    />
                  )}
                </Card>
              );
            })}
          </Panel>
        )}
      </Collapse>

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

  const runtimePanel = (
    <div className="flex-1 overflow-y-auto p-4" style={{ minHeight: 0 }}>
      <Card size="small" className="mb-3">
        <Row gutter={8} align="middle">
          <Col>
            <Space>
              {!isDebugRunning
                ? (
                  <>
                    <Button
                      type="primary"
                      icon={<CaretRightOutlined />}
                      loading={loading}
                      onClick={handleDebugRun}
                      disabled={!workflowId}
                    >
                      {t("workflow.debug.startDebug")}
                    </Button>
                    {lastDebugError && (
                      <Text type="danger" className="text-xs" style={{ display: "block", marginTop: 4 }}>
                        {lastDebugError}
                      </Text>
                    )}
                  </>
                )
                : (
                  <>
                    <Tooltip title={t("workflow.debug.pause")}>
                      <Button
                        icon={<PauseOutlined />}
                        onClick={() => pauseRun()}
                        disabled={status?.status === "paused"}
                      />
                    </Tooltip>
                    <Tooltip title={t("workflow.debug.resume")}>
                      <Button
                        icon={<PlayCircleOutlined />}
                        onClick={() => resumeRun()}
                        disabled={status?.status !== "paused"}
                      />
                    </Tooltip>
                    <Tooltip title={t("workflow.debug.cancel")}>
                      <Button
                        icon={<StopOutlined />}
                        danger
                        onClick={handleCancel}
                      />
                    </Tooltip>
                    <Divider type="vertical" />
                    <Tooltip title={t("workflow.debug.resumeBreakpoint")}>
                      <Button
                        icon={<FastForwardOutlined />}
                        onClick={handleResumeBreakpoint}
                        disabled={status?.status !== "paused"}
                      >
                        {t("workflow.debug.continue")}
                      </Button>
                    </Tooltip>
                    <Tooltip title={t("workflow.debug.stepBreakpoint")}>
                      <Button
                        icon={<StepForwardOutlined />}
                        onClick={handleStepBreakpoint}
                        disabled={status?.status !== "paused"}
                      >
                        {t("workflow.debug.step")}
                      </Button>
                    </Tooltip>
                  </>
                )}
            </Space>
          </Col>
          <Col flex="auto" />
          <Col>
            <Space size="middle">
              <Space size={4}>
                <Text type="secondary" className="text-xs">Dry Run</Text>
                <Switch
                  size="small"
                  checked={dryRun}
                  onChange={setDryRun}
                  disabled={isDebugRunning}
                />
              </Space>
              {status && (
                <Tag color={statusColor(status.status)} style={{ fontSize: 12 }}>
                  {status.status.toUpperCase()}
                </Tag>
              )}
            </Space>
          </Col>
        </Row>
      </Card>

      {status && (
        <Row gutter={12} className="mb-3">
          <Col span={8}>
            <Card size="small">
              <Statistic
                title={t("workflow.debug.execTime")}
                value={formatDuration(status.total_time_ms)}
                valueStyle={{ fontSize: 16 }}
              />
            </Card>
          </Col>
          <Col span={8}>
            <Card size="small">
              <Statistic
                title={t("workflow.debug.nodesExecuted")}
                value={nodeRecords.length}
                suffix={`/ ${status.node_count || nodes.length}`}
                valueStyle={{ fontSize: 16 }}
              />
            </Card>
          </Col>
          <Col span={8}>
            <Card size="small">
              <Statistic
                title={t("workflow.debug.breakpoints")}
                value={breakpoints.length}
                valueStyle={{ fontSize: 16 }}
              />
            </Card>
          </Col>
        </Row>
      )}

      <Collapse
        defaultActiveKey={["records", "variables"]}
        className="mb-3"
        items={[
          {
            key: "records",
            label: (
              <Space>
                {t("workflow.debug.nodeRecords")}
                <Tag>{nodeRecords.length}</Tag>
                {nodeRecords.filter((r) => r.status === "failed").length > 0 && (
                  <Tag color="error">
                    {nodeRecords.filter((r) => r.status === "failed").length} failed
                  </Tag>
                )}
              </Space>
            ),
            children: nodeRecords.length > 0
              ? (
                <Table
                  columns={recordColumns}
                  dataSource={nodeRecords}
                  rowKey="node_id"
                  size="small"
                  pagination={false}
                  scroll={{ y: 240 }}
                  expandable={{
                    expandedRowRender: (r: NodeExecutionRecord) => (
                      <div className="p-2">
                        <Row gutter={16}>
                          {r.input != null && (
                            <Col span={12}>
                              <Text strong className="text-xs">{t("workflow.debug.input")}</Text>
                              <Paragraph
                                className="mt-1 mb-0"
                                code
                                style={{
                                  fontSize: 11,
                                  maxHeight: 120,
                                  overflow: "auto",
                                  background: token.colorBgLayout,
                                  padding: 8,
                                  borderRadius: 4,
                                }}
                              >
                                {typeof r.input === "string" ? r.input : JSON.stringify(r.input, null, 2)}
                              </Paragraph>
                            </Col>
                          )}
                          {r.output != null && (
                            <Col span={12}>
                              <Text strong className="text-xs">{t("workflow.debug.output")}</Text>
                              <Paragraph
                                className="mt-1 mb-0"
                                code
                                style={{
                                  fontSize: 11,
                                  maxHeight: 120,
                                  overflow: "auto",
                                  background: token.colorBgLayout,
                                  padding: 8,
                                  borderRadius: 4,
                                }}
                              >
                                {typeof r.output === "string" ? r.output : JSON.stringify(r.output, null, 2)}
                              </Paragraph>
                            </Col>
                          )}
                        </Row>
                        {r.error && (
                          <div className="mt-2">
                            <Text type="danger" strong className="text-xs">{t("workflow.debug.error")}</Text>
                            <Paragraph
                              className="mt-1 mb-0"
                              code
                              style={{
                                fontSize: 11,
                                background: "rgba(255,77,79,0.06)",
                                padding: 8,
                                borderRadius: 4,
                              }}
                            >
                              {r.error}
                            </Paragraph>
                          </div>
                        )}
                        {r.sub_workflow_id && (
                          <div className="mt-2">
                            <Tag color="blue">
                              Sub-Workflow: {r.sub_workflow_id}
                            </Tag>
                          </div>
                        )}
                      </div>
                    ),
                  }}
                />
              )
              : (
                <Empty
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                  description={isDebugRunning ? t("workflow.debug.waitingForNodes") : t("workflow.debug.noRuntimeData")}
                />
              ),
          },
          {
            key: "variables",
            label: (
              <Space>
                <CodeOutlined />
                {t("workflow.debug.variables")}
                <Tag>{Object.keys(variables).length}</Tag>
              </Space>
            ),
            children: Object.keys(variables).length > 0
              ? (
                <Descriptions
                  size="small"
                  column={1}
                  bordered
                  contentStyle={{ fontFamily: "monospace", fontSize: 11 }}
                  labelStyle={{ width: 140, fontSize: 11 }}
                >
                  {Object.entries(variables).map(([key, val]) => (
                    <Descriptions.Item key={key} label={key}>
                      {typeof val === "string" ? val : JSON.stringify(val, null, 2)}
                    </Descriptions.Item>
                  ))}
                </Descriptions>
              )
              : (
                <Empty
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                  description={t("workflow.debug.noVariables")}
                />
              ),
          },
          {
            key: "breakpoints",
            label: (
              <Space>
                {t("workflow.debug.breakpointsPanel")}
                <Tag>{breakpoints.length}</Tag>
              </Space>
            ),
            children: breakpoints.length > 0
              ? (
                <div className="flex flex-wrap gap-1">
                  {breakpoints.map((id) => {
                    const node = nodes.find((n: { id: string }) => n.id === id);
                    const name = node?.title || (node as unknown as { data?: { title?: string } })?.data?.title || id;
                    return (
                      <Tag
                        key={id}
                        color="red"
                        closable
                        onClose={() => toggleBreakpoint(id)}
                      >
                        {name}
                      </Tag>
                    );
                  })}
                </div>
              )
              : (
                <Empty
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                  description={t("workflow.debug.noBreakpoints")}
                />
              ),
          },
          {
            key: "history",
            label: (
              <Space>
                {t("workflow.debug.executionHistory")}
                <Tag>{executionHistory.length}</Tag>
              </Space>
            ),
            children: executionHistory.length > 0
              ? (
                <List
                  size="small"
                  dataSource={executionHistory}
                  renderItem={(item) => (
                    <List.Item
                      actions={[
                        <Button
                          key="view"
                          type="link"
                          size="small"
                          onClick={async () => {
                            useWorkEngineStore.setState({ isDebugRunning: false });
                            await getStatus(item.id);
                            useWorkEngineStore.setState({ executionId: item.id });
                          }}
                        >
                          {t("workflow.debug.view")}
                        </Button>,
                      ]}
                    >
                      <Space>
                        <Tag color={statusColor(item.status)}>{item.status}</Tag>
                        <Text type="secondary" className="text-xs">
                          {new Date(item.created_at).toLocaleString()}
                        </Text>
                        {item.total_time_ms != null && (
                          <Text type="secondary" className="text-xs">
                            {formatDuration(item.total_time_ms)}
                          </Text>
                        )}
                      </Space>
                    </List.Item>
                  )}
                />
              )
              : (
                <Empty
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                  description={t("workflow.debug.noHistory")}
                />
              ),
          },
        ]}
      />

      {!status && !isDebugRunning && (
        <div className="text-center py-8">
          <Empty
            description={t("workflow.debug.noRuntimeData")}
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          >
            <Text type="secondary">{t("workflow.debug.staticDebugHint")}</Text>
          </Empty>
        </div>
      )}
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
          {activeTab === "static" && issuesWithCode > 0 && <Tag color="error">{issuesWithCode} issues</Tag>}
          {activeTab === "runtime" && isDebugRunning && (
            <Badge
              status="processing"
              text={t("workflow.debug.running")}
            />
          )}
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
          >
            {t("workflow.debug.runtimeTrace")}
          </Button>
        </Space>
      </div>

      {activeTab === "static" ? staticPanel : runtimePanel}

      <Modal
        title={
          <Space>
            <CodeOutlined />
            {detailRecord?.node_name || detailRecord?.node_id}
            {detailRecord && <Tag color={statusColor(detailRecord.status)}>{detailRecord.status}</Tag>}
          </Space>
        }
        open={detailRecord != null}
        onCancel={() => setDetailRecord(null)}
        footer={null}
        width={640}
      >
        {detailRecord && (
          <div>
            <Descriptions size="small" column={2} bordered className="mb-3">
              <Descriptions.Item label="Node ID">{detailRecord.node_id}</Descriptions.Item>
              <Descriptions.Item label="Type">{detailRecord.node_type}</Descriptions.Item>
              <Descriptions.Item label="Status">
                <Tag color={statusColor(detailRecord.status)}>{detailRecord.status}</Tag>
              </Descriptions.Item>
              <Descriptions.Item label="Duration">
                {formatDuration(detailRecord.execution_time_ms)}
              </Descriptions.Item>
              {detailRecord.sub_workflow_id && (
                <Descriptions.Item label="Sub-Workflow" span={2}>
                  <Space>
                    <Tag color="blue">{String(detailRecord.sub_workflow_id ?? "")}</Tag>
                    {!!detailRecord.output && typeof detailRecord.output === "object"
                      && "_child_execution_id" in detailRecord.output && (
                      <Button
                        type="link"
                        size="small"
                        icon={<EyeOutlined />}
                        loading={subExecutionLoading}
                        onClick={async () => {
                          const childId = (detailRecord.output as Record<string, unknown>)._child_execution_id;
                          if (!childId || typeof childId !== "string") { return; }
                          setSubExecutionLoading(true);
                          try {
                            const result = await invoke<ExecutionStatusResponse>(
                              "get_workflow_execution_status",
                              { execution_id: childId },
                            );
                            setSubExecutionDetail(result);
                          } catch {
                            setSubExecutionDetail(null);
                          } finally {
                            setSubExecutionLoading(false);
                          }
                        }}
                      >
                        {t("workflow.debug.viewSubExecution")}
                      </Button>
                    )}
                  </Space>
                </Descriptions.Item>
              )}
            </Descriptions>

            {detailRecord.input != null && (
              <div className="mb-3">
                <Text strong>{t("workflow.debug.input")}</Text>
                <Paragraph
                  className="mt-1 mb-0"
                  code
                  style={{
                    fontSize: 11,
                    maxHeight: 200,
                    overflow: "auto",
                    background: token.colorBgLayout,
                    padding: 8,
                    borderRadius: 4,
                  }}
                >
                  {typeof detailRecord.input === "string"
                    ? detailRecord.input
                    : JSON.stringify(detailRecord.input, null, 2)}
                </Paragraph>
              </div>
            )}

            {detailRecord.output != null && (
              <div className="mb-3">
                <Text strong>{t("workflow.debug.output")}</Text>
                <Paragraph
                  className="mt-1 mb-0"
                  code
                  style={{
                    fontSize: 11,
                    maxHeight: 200,
                    overflow: "auto",
                    background: token.colorBgLayout,
                    padding: 8,
                    borderRadius: 4,
                  }}
                >
                  {typeof detailRecord.output === "string"
                    ? detailRecord.output
                    : JSON.stringify(detailRecord.output, null, 2)}
                </Paragraph>
              </div>
            )}

            {detailRecord.error && (
              <div>
                <Text type="danger" strong>{t("workflow.debug.error")}</Text>
                <Paragraph
                  className="mt-1 mb-0"
                  code
                  style={{
                    fontSize: 11,
                    background: "rgba(255,77,79,0.06)",
                    padding: 8,
                    borderRadius: 4,
                  }}
                >
                  {detailRecord.error}
                </Paragraph>
              </div>
            )}
          </div>
        )}
      </Modal>

      <Modal
        title={
          <Space>
            <BugOutlined />
            {t("workflow.debug.subExecutionDetail")}
            {subExecutionDetail && <Tag color={statusColor(subExecutionDetail.status)}>{subExecutionDetail.status}
            </Tag>}
          </Space>
        }
        open={subExecutionDetail != null}
        onCancel={() => setSubExecutionDetail(null)}
        footer={null}
        width={720}
      >
        {subExecutionDetail && (
          <div>
            <Descriptions size="small" column={2} bordered className="mb-3">
              <Descriptions.Item label="Execution ID">{subExecutionDetail.execution_id}</Descriptions.Item>
              <Descriptions.Item label="Workflow ID">{subExecutionDetail.workflow_id}</Descriptions.Item>
              <Descriptions.Item label="Status">
                <Tag color={statusColor(subExecutionDetail.status)}>{subExecutionDetail.status}</Tag>
              </Descriptions.Item>
              <Descriptions.Item label="Duration">
                {formatDuration(subExecutionDetail.total_time_ms)}
              </Descriptions.Item>
              {subExecutionDetail.parent_execution_id && (
                <Descriptions.Item label="Parent Execution" span={2}>
                  <Tag color="purple">{subExecutionDetail.parent_execution_id}</Tag>
                </Descriptions.Item>
              )}
            </Descriptions>

            {subExecutionDetail.node_records.length > 0 && (
              <Table
                columns={recordColumns}
                dataSource={subExecutionDetail.node_records}
                rowKey="node_id"
                size="small"
                pagination={false}
                scroll={{ y: 300 }}
                expandable={{
                  expandedRowRender: (r: NodeExecutionRecord) => (
                    <div className="p-2">
                      <Row gutter={16}>
                        {r.input != null && (
                          <Col span={12}>
                            <Text strong className="text-xs">{t("workflow.debug.input")}</Text>
                            <Paragraph
                              className="mt-1 mb-0"
                              code
                              style={{
                                fontSize: 11,
                                maxHeight: 120,
                                overflow: "auto",
                                background: token.colorBgLayout,
                                padding: 8,
                                borderRadius: 4,
                              }}
                            >
                              {typeof r.input === "string" ? r.input : JSON.stringify(r.input, null, 2)}
                            </Paragraph>
                          </Col>
                        )}
                        {r.output != null && (
                          <Col span={12}>
                            <Text strong className="text-xs">{t("workflow.debug.output")}</Text>
                            <Paragraph
                              className="mt-1 mb-0"
                              code
                              style={{
                                fontSize: 11,
                                maxHeight: 120,
                                overflow: "auto",
                                background: token.colorBgLayout,
                                padding: 8,
                                borderRadius: 4,
                              }}
                            >
                              {typeof r.output === "string" ? r.output : JSON.stringify(r.output, null, 2)}
                            </Paragraph>
                          </Col>
                        )}
                      </Row>
                      {r.error && (
                        <div className="mt-2">
                          <Text type="danger" strong className="text-xs">{t("workflow.debug.error")}</Text>
                          <Paragraph
                            className="mt-1 mb-0"
                            code
                            style={{
                              fontSize: 11,
                              background: "rgba(255,77,79,0.06)",
                              padding: 8,
                              borderRadius: 4,
                            }}
                          >
                            {r.error}
                          </Paragraph>
                        </div>
                      )}
                    </div>
                  ),
                }}
              />
            )}
          </div>
        )}
      </Modal>
    </div>
  );
}
