import { makeWorkflowContent, type WorkflowCardData } from "@/components/chat/WorkflowAgentCard";
import { extractContent, extractDecision } from "@/lib/agentOutput";
import { invoke, listen } from "@/lib/invoke";
import type { UnlistenFn } from "@/lib/invoke";
import { useConversationStore } from "@/stores/domain/conversationStore";
import type { Message } from "@/types";
import type { StockDecision } from "@/types/stock-analysis";
import i18next from "i18next";

const activeBridges = new Map<string, UnlistenFn[]>();

const ANALYST_NODE_TO_NAME: Record<string, string> = {
  "a-market-analyst": "market-analyst",
  "a-sentiment": "sentiment",
  "a-news": "news",
  "a-fundamentals": "fundamentals",
  "a-policy": "policy",
  "a-hot-money": "hot-money",
  "a-lockup": "lockup",
  "a-research": "research",
  "a-sector": "sector",
  "a-catalyst": "catalyst",
};

const TOOL_NODE_TO_LABEL: Record<string, string> = {
  "t-market-data": i18next.t("stockAnalysis.tool.marketData"),
  "t-kline-data": i18next.t("stockAnalysis.tool.klineData"),
  "t-financial-data": i18next.t("stockAnalysis.tool.financialData"),
  "t-news-data": i18next.t("stockAnalysis.tool.newsData"),
  "t-money-flow": i18next.t("stockAnalysis.tool.moneyFlow"),
  "t-sentiment-data": i18next.t("stockAnalysis.tool.sentimentData"),
  "t-policy-data": i18next.t("stockAnalysis.tool.policyData"),
  "t-fundamentals-data": i18next.t("stockAnalysis.tool.fundamentalsData"),
  "t-hotmoney-data": i18next.t("stockAnalysis.tool.hotMoneyData"),
  "t-lockup-data": i18next.t("stockAnalysis.tool.lockupData"),
  "t-research-data": i18next.t("stockAnalysis.tool.researchData"),
  "t-sector-data": i18next.t("stockAnalysis.tool.sectorData"),
  "t-scoring": i18next.t("stockAnalysis.tool.scoring"),
  "t-valuation": i18next.t("stockAnalysis.tool.valuation"),
  "t-portfolio-risk": i18next.t("stockAnalysis.tool.portfolioRisk"),
  "t-peers-data": i18next.t("stockAnalysis.tool.peersData"),
  "t-option-data": i18next.t("stockAnalysis.tool.optionData"),
  "t-index-data": i18next.t("stockAnalysis.tool.indexData"),
  "t-announcement-data": i18next.t("stockAnalysis.tool.announcement"),
  "t-northbound-data": i18next.t("stockAnalysis.tool.northbound"),
  "t-dragon-tiger-data": i18next.t("stockAnalysis.tool.dragonTiger"),
  "t-cls-flash-data": i18next.t("stockAnalysis.tool.clsFlash"),
  "t-block-trade-data": i18next.t("stockAnalysis.tool.blockTrade"),
  "t-institutional-data": i18next.t("stockAnalysis.tool.institutional"),
  "t-consensus-data": i18next.t("stockAnalysis.tool.consensus"),
  "t-concept-data": i18next.t("stockAnalysis.tool.concept"),
};

function wf(type: string, data: Record<string, unknown>, fallback: string): string {
  return makeWorkflowContent(type, data, fallback);
}

function appendMessageToStore(conversationId: string, msg: Message) {
  const store = useConversationStore.getState();
  if (store.activeConversationId !== conversationId) {
    return;
  }
  const exists = store.messages.some((m) => m.id === msg.id);
  if (exists) {
    return;
  }
  useConversationStore.setState((s) => ({
    messages: [...s.messages, msg],
  }));
}

function updateMessageInStore(messageId: string, content: string) {
  const store = useConversationStore.getState();
  const exists = store.messages.some((m) => m.id === messageId);
  if (!exists) {
    return;
  }
  useConversationStore.setState((s) => ({
    messages: s.messages.map((m) => m.id === messageId ? { ...m, content } : m),
  }));
}

function summarizeToolResult(result: unknown): string {
  if (result == null) { return ""; }
  if (typeof result === "string") {
    try {
      return summarizeParsed(JSON.parse(result));
    } catch {
      return result.length > 120 ? result.slice(0, 120) + "..." : result;
    }
  }
  return summarizeParsed(result);
}

function summarizeParsed(v: unknown): string {
  if (Array.isArray(v)) {
    if (v.length === 0) { return i18next.t("stockAnalysis.summary.records", { count: 0 }); }
    const first = v[0];
    if (first && typeof first === "object") {
      const keys = Object.keys(first as Record<string, unknown>);
      return i18next.t("stockAnalysis.summary.recordsWithKeys", {
        count: v.length,
        keys: keys.slice(0, 3).join(", ") + (keys.length > 3 ? "..." : ""),
      });
    }
    return i18next.t("stockAnalysis.summary.records", { count: v.length });
  }
  if (v && typeof v === "object") {
    const obj = v as Record<string, unknown>;
    // 解包 {"content": "..."} 包装
    if (typeof obj.content === "string" && Object.keys(obj).length <= 3) {
      const inner = obj.content;
      if (inner === "null" || inner === "[]" || inner === "") { return i18next.t("stockAnalysis.summary.noData"); }
      try {
        return summarizeParsed(JSON.parse(inner));
      } catch {
        return inner.length > 120 ? inner.slice(0, 120) + "..." : inner;
      }
    }
    if (obj.stockName && obj.price) { return `${obj.stockName} ¥${obj.price}`; }
    if (obj.stockCode && obj.totalScore) { return i18next.t("stockAnalysis.summary.score", { score: obj.totalScore }); }
    if (obj.stockCode && obj.dcf) { return `DCF ¥${(obj.dcf as Record<string, unknown>)?.intrinsicValue ?? "N/A"}`; }
    if (obj.items && Array.isArray(obj.items)) {
      return i18next.t("stockAnalysis.summary.records", { count: (obj.items as unknown[]).length });
    }
    if (obj.news && Array.isArray(obj.news)) {
      return i18next.t("stockAnalysis.summary.news", { count: (obj.news as unknown[]).length });
    }
    if (obj.data && Array.isArray(obj.data)) {
      return i18next.t("stockAnalysis.summary.dataItems", { count: (obj.data as unknown[]).length });
    }
    const keys = Object.keys(obj);
    if (keys.length <= 4) { return keys.map((k) => `${k}: ${JSON.stringify(obj[k]).slice(0, 30)}`).join(", "); }
    return i18next.t("stockAnalysis.summary.fields", { count: keys.length });
  }
  return String(v).slice(0, 80);
}

interface DataSourceEntry {
  nodeId: string;
  toolName: string;
  label: string;
  status: "pending" | "fetching" | "success" | "failed";
  error?: string;
  summary?: string;
}

export async function startStockWorkflowChatBridge(conversationId: string): Promise<void> {
  stopStockWorkflowChatBridge(conversationId);

  const unlisteners: UnlistenFn[] = [];
  let aggregateMsgId: string | null = null;

  // ── 节流：每 200ms 最多 flush 一次聚合卡片更新，避免每节点都打 IPC ──
  let updateTimer: ReturnType<typeof setTimeout> | null = null;
  let pendingContent: string | null = null;
  const flushUpdate = () => {
    if (updateTimer) {
      clearTimeout(updateTimer);
      updateTimer = null;
    }
    if (!aggregateMsgId || pendingContent == null) { return; }
    invoke("update_message_content", { id: aggregateMsgId, content: pendingContent }).catch(() => {});
    updateMessageInStore(aggregateMsgId, pendingContent);
    pendingContent = null;
  };
  const scheduleUpdate = (content: string) => {
    pendingContent = content;
    if (updateTimer) { return; }
    updateTimer = setTimeout(flushUpdate, 200);
  };

  const analystsMap = new Map(
    Object.entries(ANALYST_NODE_TO_NAME).map(([nodeId, name]) => [
      nodeId,
      {
        nodeId,
        name,
        status: "pending" as "pending" | "running" | "done" | "failed",
        report: undefined as string | undefined,
      },
    ]),
  );

  const DEBATE_ROUNDS = 3;
  const debatesMap = new Map<
    string,
    { key: string; label: string; status: "pending" | "running" | "done" | "failed"; rounds: string[] }
  >();
  for (let r = 1; r <= DEBATE_ROUNDS; r++) {
    debatesMap.set(`bull-r${r}`, {
      key: `bull-r${r}`,
      label: `${i18next.t("stockAnalysis.workflow.bullAnalyst")}·${r}`,
      status: "pending",
      rounds: [],
    });
    debatesMap.set(`bear-r${r}`, {
      key: `bear-r${r}`,
      label: `${i18next.t("stockAnalysis.workflow.bearAnalyst")}·${r}`,
      status: "pending",
      rounds: [],
    });
  }

  const risksMap = new Map([
    ["risk-agg", {
      key: "risk-agg",
      label: i18next.t("stockAnalysis.workflow.riskAggressive"),
      status: "pending" as "pending" | "running" | "done" | "failed",
      content: undefined as string | undefined,
    }],
    ["risk-con", {
      key: "risk-con",
      label: i18next.t("stockAnalysis.workflow.riskConservative"),
      status: "pending" as "pending" | "running" | "done" | "failed",
      content: undefined as string | undefined,
    }],
    ["risk-neu", {
      key: "risk-neu",
      label: i18next.t("stockAnalysis.workflow.riskNeutral"),
      status: "pending" as "pending" | "running" | "done" | "failed",
      content: undefined as string | undefined,
    }],
  ]);

  const extraNodesMap = new Map([
    ["agg-risk", {
      key: "agg-risk",
      label: i18next.t("stockAnalysis.workflow.riskAggregation"),
      status: "pending" as "pending" | "running" | "done" | "failed",
    }],
    ["cls-risk-level", {
      key: "cls-risk-level",
      label: i18next.t("stockAnalysis.workflow.riskClassification"),
      status: "pending" as "pending" | "running" | "done" | "failed",
    }],
    ["notify-result", {
      key: "notify-result",
      label: i18next.t("stockAnalysis.workflow.notification"),
      status: "pending" as "pending" | "running" | "done" | "failed",
    }],
  ]);

  const dataSourcesMap = new Map<string, DataSourceEntry>();
  const completedNodes = new Set<string>();
  let workflowTotalNodes = 30; // 默认值，会从首次 step-done 事件更新
  let finalDecision: WorkflowCardData | null = null;

  const buildAggregateContent = (
    phase: string,
    status: "running" | "done" | "error",
    totalNodes: number,
    error?: string,
  ) => {
    const analysts = Array.from(analystsMap.values());
    // 转换 debatesMap 为 WorkflowCardData 期望的 round/bull/bear/status 格式
    const debates: { round: number; bull?: string; bear?: string; status: "running" | "done" | "pending" }[] = [];
    for (let r = 1; r <= DEBATE_ROUNDS; r++) {
      const bullEntry = debatesMap.get(`bull-r${r}`);
      const bearEntry = debatesMap.get(`bear-r${r}`);
      debates.push({
        round: r,
        bull: bullEntry?.rounds[0],
        bear: bearEntry?.rounds[0],
        status: (bullEntry?.rounds[0] && bearEntry?.rounds[0])
          ? "done" as const
          : (bullEntry?.status === "running" || bearEntry?.status === "running")
          ? "running" as const
          : "pending" as const,
      });
    }
    const risks = Array.from(risksMap.values());
    const dataSources = Array.from(dataSourcesMap.values());
    const extraNodes = Array.from(extraNodesMap.values());
    const count = completedNodes.size;
    return wf(
      "aggregate",
      {
        phase,
        completed: count,
        total: totalNodes,
        analysts,
        debates,
        risks,
        extraNodes,
        dataSources,
        status,
        error,
        decision: finalDecision,
      },
      `${i18next.t("stockAnalysis.workflow.title")} (${count}/${totalNodes})`,
    );
  };

  const tryCreateAggregateMsg = async (): Promise<string | null> => {
    try {
      const m = await invoke<Message>("send_system_message", {
        conversationId,
        content: buildAggregateContent("trigger", "running", 30),
      });
      appendMessageToStore(conversationId, m);
      return m.id;
    } catch (e) {
      console.error("[StockWorkflowChatBridge] 创建聚合消息失败，将在首个节点完成时重试:", e);
      return null;
    }
  };
  aggregateMsgId = await tryCreateAggregateMsg();

  const u1 = await listen<{
    workflowId: string;
    nodeId: string;
    status: string;
    totalNodes: number;
    completedNodes: number;
    output?: unknown;
  }>("workflow-step-done", async (event) => {
    const { nodeId, totalNodes, status, output } = event.payload;
    // 从首次事件更新实际总节点数
    if (totalNodes > 0) { workflowTotalNodes = totalNodes; }
    if (status === "completed" || status === "failed") {
      completedNodes.add(nodeId);
    }

    // ── Tool 节点 (t- 前缀)：数据源信息 ──
    if (nodeId.startsWith("t-")) {
      if (!dataSourcesMap.has(nodeId)) {
        let toolName = "";
        const label = TOOL_NODE_TO_LABEL[nodeId] ?? nodeId.slice(2);

        if (output != null && typeof output === "object") {
          const out = output as Record<string, unknown>;
          if (out.tool_name && typeof out.tool_name === "string") {
            toolName = out.tool_name;
          }
        }

        dataSourcesMap.set(nodeId, { nodeId, toolName, label, status: "pending" });
      }

      const ds = dataSourcesMap.get(nodeId)!;
      if (status === "running") {
        ds.status = "fetching";
      } else if (status === "completed") {
        ds.status = "success";
        if (output != null && typeof output === "object") {
          const out = output as Record<string, unknown>;
          if (out.tool_name && typeof out.tool_name === "string" && !ds.toolName) {
            ds.toolName = out.tool_name;
          }
          if (out.result != null) {
            ds.summary = summarizeToolResult(out.result);
          } else {
            ds.summary = summarizeToolResult(output);
          }
        } else if (output != null) {
          ds.summary = summarizeToolResult(output);
        }
      } else if (status === "failed") {
        ds.status = "failed";
        if (output != null) {
          ds.error = typeof output === "string" ? output : JSON.stringify(output);
        } else {
          ds.error = i18next.t("stockAnalysis.workflow.fetchFailed");
        }
      }
    }

    // ── Agent 节点 (a- 前缀)：分析师报告 ──
    if (analystsMap.has(nodeId)) {
      const analyst = analystsMap.get(nodeId)!;
      if (status === "running") {
        analyst.status = "running";
      } else if (status === "completed") {
        analyst.status = "done";
        if (output != null) {
          analyst.report = extractContent(output);
        }
      } else if (status === "failed") {
        analyst.status = "failed";
      }
    }

    // ── 辩论节点 (DebateNode 子节点: bull-researcher / bear-researcher / bull-r* / bear-r*) ──
    const debateNodeId = nodeId === "bull-researcher"
      ? "bull-r1"
      : nodeId === "bear-researcher"
      ? "bear-r1"
      : nodeId;

    if (debatesMap.has(debateNodeId)) {
      const debater = debatesMap.get(debateNodeId)!;
      if (status === "running") {
        debater.status = "running";
      } else if (status === "completed") {
        debater.status = "done";
        if (output != null) {
          debater.rounds.push(extractContent(output));
        }
      } else if (status === "failed") {
        debater.status = "failed";
      }
    }

    // ── 风险评估节点 (ParallelNode 子节点) ──
    if (risksMap.has(nodeId)) {
      const risk = risksMap.get(nodeId)!;
      if (status === "running") {
        risk.status = "running";
      } else if (status === "completed") {
        risk.status = "done";
        if (output != null) {
          risk.content = extractContent(output);
        }
      } else if (status === "failed") {
        risk.status = "failed";
      }
    }

    // ── 新增节点 (Aggregator / LlmClassifier / Notification) ──
    if (extraNodesMap.has(nodeId)) {
      const node = extraNodesMap.get(nodeId)!;
      if (status === "running") {
        node.status = "running";
      } else if (status === "completed") {
        node.status = "done";
      } else if (status === "failed") {
        node.status = "failed";
      }
    }

    // ── 更新聚合卡片 ──
    if (!aggregateMsgId && status === "completed") {
      aggregateMsgId = await tryCreateAggregateMsg();
    }
    if (aggregateMsgId) {
      scheduleUpdate(buildAggregateContent(nodeId, "running", totalNodes));
    }
  });
  unlisteners.push(u1);

  const u2 = await listen<{
    workflowId: string;
    results: Record<string, unknown>;
    output?: Record<string, unknown> | null;
  }>("workflow-completed", async (event) => {
    const { output, results } = event.payload;

    // 优先从 portfolio-mgr 节点结果中提取决策（最可靠的来源）
    const pmRaw = results["portfolio-mgr"];
    let parsedDecision: StockDecision | null = null;

    if (pmRaw) {
      parsedDecision = extractDecision(pmRaw);
    }

    // 回退到 output
    if (!parsedDecision && output !== undefined) {
      parsedDecision = extractDecision(output);
    }

    finalDecision = parsedDecision
      ? {
        type: "decision" as const,
        action: parsedDecision.action,
        positionPct: parsedDecision.positionPct,
        targetPrice: parsedDecision.targetPrice ?? 0,
        stopLoss: parsedDecision.stopLoss ?? 0,
        reasoning: parsedDecision.reasoning,
        riskLevel: parsedDecision.riskLevel,
        confidence: parsedDecision.confidence,
      }
      : null;

    if (aggregateMsgId) {
      scheduleUpdate(buildAggregateContent("done", "done", workflowTotalNodes));
    }

    stopStockWorkflowChatBridge(conversationId);
  });
  unlisteners.push(u2);

  const u3 = await listen<{ workflowId: string; error: string }>(
    "workflow-error",
    async (event) => {
      if (aggregateMsgId) {
        scheduleUpdate(buildAggregateContent("error", "error", workflowTotalNodes, event.payload.error));
      }
      stopStockWorkflowChatBridge(conversationId);
    },
  );
  unlisteners.push(u3);

  // 清理：bridge 停止时 flush 一次 pending，并清理 timer
  unlisteners.push(() => {
    flushUpdate();
    if (updateTimer) {
      clearTimeout(updateTimer);
      updateTimer = null;
      pendingContent = null;
    }
  });

  activeBridges.set(conversationId, unlisteners);
}

export function stopStockWorkflowChatBridge(conversationId: string): void {
  const unlisteners = activeBridges.get(conversationId);
  if (unlisteners) {
    for (const u of unlisteners) {
      try {
        u();
      } catch { /* silent */ }
    }
    activeBridges.delete(conversationId);
  }
}
