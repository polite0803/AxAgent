/**
 * P2-2: 分析运行时 Debug 面板
 *
 * 展示每个分析节点的状态、报告大小、错误信息等，
 * 帮助定位断流问题。
 *
 * 数据来源：store（analystReports / failedNodes / failedNodeErrors），
 * 不依赖独立的 Tauri 事件监听（与 store 重复且不可靠）。
 */

import { useStockAnalysisStore } from "@/stores";
import { Card, Table, Tag, Tooltip } from "antd";
import { useMemo } from "react";

interface NodeInfo {
  nodeId: string;
  status: "completed" | "failed" | "pending";
  reportSize: number;
  errorMessage?: string;
}

/** 节点类型分组颜色 */
const NODE_COLORS: Record<string, string> = {
  "a-": "#3b82f6", // 分析师 → 蓝色
  "risk-": "#ef4444", // 风控 → 红色
  "p-risk-assess": "#ef4444",
  "agg-risk": "#ef4444",
  "risk-convergence": "#ef4444",
  "bull-": "#22c55e", // 多方 → 绿色
  "bear-": "#a855f7", // 空方 → 紫色
  "t-": "#f97316", // 工具 → 橙色
  default: "#6b7280", // 默认 → 灰色
};

function getNodeColor(nodeId: string): string {
  for (const [prefix, color] of Object.entries(NODE_COLORS)) {
    if (nodeId === prefix || nodeId.startsWith(prefix)) { return color; }
  }
  return NODE_COLORS.default;
}

function getNodeTypeLabel(nodeId: string): string {
  if (nodeId.startsWith("a-")) { return "分析师"; }
  if (
    nodeId.startsWith("risk-") || nodeId === "p-risk-assess" || nodeId === "agg-risk" || nodeId === "risk-convergence"
  ) { return "风控"; }
  if (nodeId.startsWith("bull-")) { return "多方辩论"; }
  if (nodeId.startsWith("bear-")) { return "空方辩论"; }
  if (nodeId.startsWith("t-")) { return "数据工具"; }
  if (nodeId === "debate-convergence") { return "辩论收敛"; }
  if (nodeId.includes("mgr") || nodeId.includes("manager")) { return "决策引擎"; }
  if (nodeId.includes("checker") || nodeId === "rule-check") { return "规则检查"; }
  return "其他";
}

export function AnalysisDebugPanel() {
  const analystReports = useStockAnalysisStore((s) => s.analystReports);
  const status = useStockAnalysisStore((s) => s.status);
  const failedNodes = useStockAnalysisStore((s) => s.failedNodes);
  const failedNodeErrors = useStockAnalysisStore((s) => s.failedNodeErrors);
  const progressPct = useStockAnalysisStore((s) => s.progressPct);
  const currentStage = useStockAnalysisStore((s) => s.currentStage);

  // 从 store 数据构建完整节点列表
  const allNodes = useMemo((): NodeInfo[] => {
    const nodeMap = new Map<string, NodeInfo>();

    // 1. 已完成的节点（有 analystReport）
    if (analystReports) {
      for (const [nodeId, text] of Object.entries(analystReports)) {
        nodeMap.set(nodeId, {
          nodeId,
          status: "completed",
          reportSize: text?.length ?? 0,
        });
      }
    }

    // 2. 失败的节点（从 failedNodes 补充）
    for (const nodeId of failedNodes) {
      const existing = nodeMap.get(nodeId);
      if (existing) {
        existing.status = "failed";
        existing.errorMessage = failedNodeErrors[nodeId];
      } else {
        nodeMap.set(nodeId, {
          nodeId,
          status: "failed",
          reportSize: 0,
          errorMessage: failedNodeErrors[nodeId],
        });
      }
    }

    return Array.from(nodeMap.values()).sort((a, b) => a.nodeId.localeCompare(b.nodeId));
  }, [analystReports, failedNodes, failedNodeErrors]);

  // 统计
  const completedCount = allNodes.filter((n) => n.status === "completed").length;
  const failedCount = allNodes.filter((n) => n.status === "failed").length;
  const totalSize = allNodes.reduce((sum, n) => sum + n.reportSize, 0);

  const columns = [
    {
      title: "节点",
      dataIndex: "nodeId",
      key: "nodeId",
      width: 180,
      render: (id: string, record: NodeInfo) => (
        <span className="font-mono text-xs" style={{ color: getNodeColor(id) }}>
          <span
            className="inline-block w-2 h-2 rounded-full mr-1.5"
            style={{ background: record.status === "failed" ? "#ef4444" : getNodeColor(id) }}
          />
          {id}
        </span>
      ),
    },
    {
      title: "类型",
      dataIndex: "nodeId",
      key: "nodeType",
      width: 80,
      render: (id: string) => <span className="text-xs text-gray-400">{getNodeTypeLabel(id)}</span>,
    },
    {
      title: "状态",
      dataIndex: "status",
      key: "status",
      width: 90,
      render: (st: string) => (
        <Tag
          color={st === "completed" ? "success" : st === "failed" ? "error" : "default"}
          className="text-[11px]"
        >
          {st === "completed" ? "✅ 完成" : st === "failed" ? "❌ 失败" : "⬜ 待执行"}
        </Tag>
      ),
    },
    {
      title: "报告大小",
      dataIndex: "reportSize",
      key: "size",
      width: 95,
      sorter: (a: NodeInfo, b: NodeInfo) => a.reportSize - b.reportSize,
      render: (size: number) =>
        size > 0
          ? (
            size >= 1000 ? `${(size / 1000).toFixed(1)}KB` : `${size}B`
          )
          : <span className="text-gray-600">-</span>,
    },
    {
      title: "估计 Tokens",
      dataIndex: "reportSize",
      key: "tokens",
      width: 90,
      render: (size: number) => (size > 0 ? Math.round(size / 4) : "-"),
    },
    {
      title: "错误信息",
      dataIndex: "errorMessage",
      key: "error",
      width: 220,
      ellipsis: true,
      render: (err: string | undefined) =>
        err
          ? (
            <Tooltip title={err}>
              <span className="text-red-400 text-xs cursor-help">{err}</span>
            </Tooltip>
          )
          : <span className="text-gray-600">-</span>,
    },
  ];

  return (
    <Card
      size="small"
      title={
        <div className="flex items-center gap-3">
          <span>🔍 Debug 面板</span>
          <Tag color="default" className="text-[10px]">
            进度 {progressPct}%
          </Tag>
          <Tag color="default" className="text-[10px]">
            阶段 {currentStage}/5
          </Tag>
        </div>
      }
      className="mb-3"
      styles={{ body: { padding: "8px 12px" } }}
    >
      {/* 概要行 */}
      <div className="flex gap-4 text-xs mb-2" style={{ color: "var(--muted)" }}>
        <span>总节点: {allNodes.length}</span>
        <span className="text-green-400">完成: {completedCount}</span>
        <span className="text-red-400">失败: {failedCount}</span>
        <span>总报告: {totalSize >= 1000 ? `${(totalSize / 1000).toFixed(1)}KB` : `${totalSize}B`}</span>
        <span className="text-gray-500">状态: {status}</span>
      </div>

      {/* 失败节点高亮提示 */}
      {failedCount > 0 && (
        <div className="mb-2 px-3 py-1.5 rounded text-xs bg-red-500/10 border border-red-500/20">
          <span className="text-red-400 font-medium">
            ⚠️ {failedCount} 个节点失败，可能影响决策质量：
          </span>{" "}
          {allNodes
            .filter((n) => n.status === "failed")
            .map((n) => n.nodeId)
            .join(", ")}
        </div>
      )}

      {/* 节点详细表格 */}
      <Table
        dataSource={allNodes}
        columns={columns}
        pagination={false}
        size="small"
        className="mt-2"
        rowClassName={(record: NodeInfo) => `debug-row text-[11px] ${record.status === "failed" ? "bg-red-500/5" : ""}`}
        scroll={{ x: 750 }}
      />

      {/* 决策诊断提示 */}
      {status === "completed" && failedCount > 0 && (
        <div className="mt-2 px-3 py-1.5 rounded text-xs bg-yellow-500/10 border border-yellow-500/20">
          <span className="text-yellow-400">
            💡 上游节点失败会导致决策引擎收到默认值（score=50），从而产生相同结论。
            请重点排查上方红色标记节点的失败原因。
          </span>
        </div>
      )}

      {status !== "running" && status !== "loading" && status !== "completed" && (
        <div className="text-center py-4 text-xs text-gray-500">等待分析开始...</div>
      )}

      <style>
        {`
        .debug-row td {
          padding: 3px 8px !important;
          font-size: 11px;
        }
        .debug-row:hover td {
          background: rgba(255,255,255,0.03) !important;
        }
      `}
      </style>
    </Card>
  );
}
