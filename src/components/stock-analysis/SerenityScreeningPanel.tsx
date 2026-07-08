import { invoke } from "@/lib/invoke";
import {
  type AttentionMetrics,
  type ExitSignals,
  type SerenityCandidate,
  type StepLog,
  type StepStage,
  type TrendInfo,
  useSerenityStore,
} from "@/stores/feature/serenityStore";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import {
  AlertOutlined,
  CheckCircleOutlined,
  ClockCircleOutlined,
  DownOutlined,
  HistoryOutlined,
  LoadingOutlined,
  PlayCircleOutlined,
  ReloadOutlined,
  RightOutlined,
  StockOutlined,
} from "@ant-design/icons";
import { listen } from "@tauri-apps/api/event";
import {
  Alert,
  Button,
  Card,
  Checkbox,
  Empty,
  InputNumber,
  message,
  Modal,
  Progress,
  Space,
  Spin,
  Table,
  Tag,
  Typography,
} from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

const { Text, Title } = Typography;

// ── invoke 返回类型 ──
interface SerenityResult {
  status?: string;
  candidates?: unknown;
  trends?: TrendInfo[];
  /**
   * 后端从 a-candidate-mapper 的 arguments.summary 透传出来的"为什么没有候选"。
   * 当上游三个瓶颈节点均返回 data_gaps=true 时，模型反幻觉拒绝编造并在此
   * 字段说明原因；前端在 candidates 为空时把它展示给用户。
   */
  emptyReason?: string | null;
}

/// 从多种可能的 candidates 结构中提取候选数组。
/// 支持的输入形态：
///   - 数组 [...]
///   - { candidates: [...] }
///   - { stocks: [...] } / { list: [...] } / { data: [...] }  常见字段名
///   - Agent 包装 { content: "...", params: { candidates: [...] } }
///   - Agent 包装 { content: "```json\\n{...}\\n```" } （markdown 代码块）
///   - Agent 包装 { content: "{ candidates: [...] }" } （content 是 JSON string）
///   - 任意对象中嵌套的 candidates/stocks 数组（深搜）
function extractCandidatesList(raw: unknown): SerenityCandidate[] {
  if (raw == null) { return []; }
  if (Array.isArray(raw)) {
    return raw as SerenityCandidate[];
  }
  if (typeof raw === "object") {
    const obj = raw as Record<string, unknown>;
    // 常见容器字段
    for (const key of ["candidates", "stocks", "list", "data", "items", "results"]) {
      if (Array.isArray(obj[key])) {
        return obj[key] as SerenityCandidate[];
      }
    }
    // Agent 包装：{ params: { candidates: [...] } }
    if (obj.params && typeof obj.params === "object") {
      const params = obj.params as Record<string, unknown>;
      for (const key of ["candidates", "stocks", "list", "data"]) {
        if (Array.isArray(params[key])) {
          return params[key] as SerenityCandidate[];
        }
      }
      // params 整体就是数组
      if (Array.isArray(params)) {
        return params as unknown as SerenityCandidate[];
      }
    }
    // Agent 包装：{ content: "..." }（content 可能是 JSON string 或 markdown 块）
    if (typeof obj.content === "string") {
      const parsed = parseJsonFromContent(obj.content);
      if (parsed) {
        return extractCandidatesList(parsed);
      }
    }
    // 兜底：深度搜索任何属性里的数组，每个元素形如 { stock_code, ... }
    const fallback = findCandidatesDeep(obj);
    if (fallback.length > 0) {
      return fallback;
    }
  }
  return [];
}

/// 从 content 字符串提取 JSON（支持 markdown 代码块、纯 JSON、有无前缀）
function parseJsonFromContent(content: string): unknown | null {
  // 去除前后空白
  const trimmed = content.trim();
  if (!trimmed) { return null; }
  // 1) 直接是 JSON
  if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
    try {
      return JSON.parse(trimmed);
    } catch {
      // 不是纯 JSON，继续尝试
    }
  }
  // 2) markdown ```json ... ``` 块
  const codeBlock = trimmed.match(/```(?:json)?\s*([\s\S]*?)```/);
  if (codeBlock) {
    try {
      return JSON.parse(codeBlock[1].trim());
    } catch {
      // fall through
    }
  }
  // 3) 提取第一个 {...} 或 [...] 块
  const firstBrace = trimmed.indexOf("{");
  const firstBracket = trimmed.indexOf("[");
  const start = (() => {
    if (firstBrace === -1) { return firstBracket; }
    if (firstBracket === -1) { return firstBrace; }
    return Math.min(firstBrace, firstBracket);
  })();
  if (start < 0) { return null; }
  const openChar = trimmed[start];
  const closeChar = openChar === "{" ? "}" : "]";
  // 括号配对扫描（处理字符串内的括号）
  let depth = 0;
  let inStr = false;
  let escape = false;
  let end = -1;
  for (let i = start; i < trimmed.length; i++) {
    const c = trimmed[i];
    if (escape) {
      escape = false;
      continue;
    }
    if (c === "\\") {
      escape = true;
      continue;
    }
    if (c === '"') {
      inStr = !inStr;
      continue;
    }
    if (inStr) { continue; }
    if (c === openChar) { depth++; }
    else if (c === closeChar) {
      depth--;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }
  if (end > start) {
    const candidate = trimmed.slice(start, end + 1);
    try {
      return JSON.parse(candidate);
    } catch {
      return null;
    }
  }
  return null;
}

/// 深度搜索：返回 obj 任意层级第一个看起来像候选数组的数组
function findCandidatesDeep(obj: Record<string, unknown>, depth = 0): SerenityCandidate[] {
  if (depth > 4) { return []; }
  for (const v of Object.values(obj)) {
    if (Array.isArray(v)) {
      // 数组里第一个元素包含 stock_code 或 stock_name 字段就认为命中
      if (v.length > 0 && typeof v[0] === "object" && v[0] !== null) {
        const first = v[0] as Record<string, unknown>;
        if ("stock_code" in first || "stockCode" in first || "stock_name" in first || "stockName" in first) {
          return v as SerenityCandidate[];
        }
      }
    } else if (v && typeof v === "object") {
      const found = findCandidatesDeep(v as Record<string, unknown>, depth + 1);
      if (found.length > 0) { return found; }
    }
  }
  return [];
}

// ── 节点 ID → 阶段映射 ──
const NODE_STAGE_MAP: Record<string, StepStage> = {
  trigger: "loading",
  "t-hot-stocks": "scanning",
  "t-industry-rank": "scanning",
  "t-cls-flash": "scanning",
  "t-northbound": "scanning",
  "a-trend-scanner": "scanning",
  "a-chain-trend1": "decomposing",
  "a-chain-trend2": "decomposing",
  "a-chain-trend3": "decomposing",
  "a-chokepoint-trend1": "identifying",
  "a-chokepoint-trend2": "identifying",
  "a-chokepoint-trend3": "identifying",
  "a-candidate-mapper": "mapping",
  "s-save-candidates": "saving",
};

/// 将节点 ID 映射为 i18n 标题 key
function nodeTitleKey(nodeId: string): string {
  return `serenityPanel.nodeTitles.${nodeId}`;
}

/// 从 Agent 节点输出中提取可读文本（content / params / raw）
function summarizeOutput(output: unknown): string {
  if (output == null) { return ""; }
  if (typeof output === "string") {
    const trimmed = output.trim();
    // 超长文本截断预览
    return trimmed.length > 500 ? trimmed.slice(0, 500) + "..." : trimmed;
  }
  if (typeof output === "number" || typeof output === "boolean") {
    return String(output);
  }
  const obj = output as Record<string, unknown>;
  // Agent 包装：{ content, params, thinking, ... }
  if (typeof obj.content === "string" && obj.content.trim().length > 0) {
    const c = obj.content.trim();
    return c.length > 500 ? c.slice(0, 500) + "..." : c;
  }
  if (obj.params && typeof obj.params === "object") {
    const s = JSON.stringify(obj.params, null, 2);
    return s.length > 500 ? s.slice(0, 500) + "..." : s;
  }
  // ToolNode 原始输出：尝试提取有意义的字段
  if (Array.isArray(obj)) {
    const s = JSON.stringify(obj, null, 2);
    return s.length > 500 ? s.slice(0, 500) + "..." : s;
  }
  // 对象：提取前几个有值字段做摘要
  const keys = Object.keys(obj).filter((k) => {
    const v = obj[k];
    return v != null && !(typeof v === "string" && v.trim().length === 0);
  });
  if (keys.length === 0) { return ""; }
  // 少量字段直接 stringify，大量字段只取前几个
  const summary: Record<string, unknown> = {};
  for (const k of keys.slice(0, 8)) {
    summary[k] = obj[k];
  }
  const s = JSON.stringify(summary, null, 2);
  return s.length > 500 ? s.slice(0, 500) + "..." : s;
}

export function SerenityScreeningPanel() {
  const {
    running,
    setRunning,
    candidates,
    setCandidates,
    trends,
    setTrends,
    error,
    setError,
    stage,
    setStage,
    completedNodes,
    setCompletedNodes,
    totalNodes,
    setTotalNodes,
    steps,
    addStep,
    currentNodeId,
    setCurrentNode,
    clearSteps,
    emptyReason,
    setEmptyReason,
  } = useSerenityStore();
  const { t } = useTranslation();
  const navigate = useNavigate();

  // 用于在 handleRun 启动前注册监听器，确保不漏事件
  const unlistenStepRef = useRef<(() => void) | null>(null);
  const unlistenDoneRef = useRef<(() => void) | null>(null);
  // 跟踪 completed/failed 事件是否已处理（用于避免 invoke 错误路径覆盖事件已设置的结果）
  const eventHandledRef = useRef<boolean>(false);
  const [expandedSteps, setExpandedSteps] = useState<Set<number>>(new Set());
  // 回馈闭环状态
  const [feedbackData, setFeedbackData] = useState<
    {
      total: number;
      profitable_count: number;
      win_rate: number;
      avg_return_pct: number;
      performances: Array<{
        id: string;
        stock_code: string;
        stock_name: string;
        return_pct: number;
        is_profitable: boolean;
        recommend_date: string;
        catalysts: { total: number; verified: number };
      }>;
    } | null
  >(null);
  const [feedbackLoading, setFeedbackLoading] = useState(false);

  // 瓶颈掘金历史（多选 + 批量删除）
  const [serenityHistoryOpen, setSerenityHistoryOpen] = useState(false);
  const [serenityHistory, setSerenityHistory] = useState<
    Array<{
      generatedAt: string;
      stockCount: number;
      createdAt: string;
    }>
  >([]);
  const [serenityHistoryLoading, setSerenityHistoryLoading] = useState(false);
  const [serenitySelected, setSerenitySelected] = useState<string[]>([]);
  const [serenityDeleting, setSerenityDeleting] = useState(false);

  // 瓶颈掘金历史详情
  const [serenityDetailOpen, setSerenityDetailOpen] = useState(false);
  const [serenityDetailLoading, setSerenityDetailLoading] = useState(false);
  const [serenityDetailItems, setSerenityDetailItems] = useState<
    Array<{
      stockCode: string;
      stockName: string;
      confidence: number;
      generatedAt: string;
    }>
  >([]);
  const [serenityDetailRow, setSerenityDetailRow] = useState<
    {
      generatedAt: string;
      stockCount: number;
      createdAt: string;
    } | null
  >(null);

  // ── 估值过滤设置 ──
  const [serenitySettingsOpen, setSerenitySettingsOpen] = useState(false);
  const [serenityVars, setSerenityVars] = useState<Record<string, number>>({});
  useEffect(() => {
    invoke<{ variables: Array<{ name: string; value: unknown }> }>(
      "get_template_by_version",
      { id: "serenity-screening", version: 6 },
    ).then((tpl) => {
      if (!tpl) { return; }
      const map: Record<string, number> = {};
      for (const v of tpl.variables) {
        if (v.name.startsWith("serenity_")) {
          map[v.name.replace("serenity_", "")] = Number(v.value) || 0;
        }
      }
      setSerenityVars(map);
    }).catch(() => {});
  }, []);
  const handleSerenityVarChange = useCallback(async (key: string, value: number) => {
    setSerenityVars((prev) => ({ ...prev, [key]: value }));
    try {
      await invoke("apply_update_variable", {
        templateId: "serenity-screening",
        name: `serenity_${key}`,
        value,
      });
    } catch { /* ignore */ }
  }, []);

  // 组件卸载时清理监听
  useEffect(() => {
    return () => {
      unlistenStepRef.current?.();
      unlistenDoneRef.current?.();
    };
  }, []);

  const toggleStep = useCallback((idx: number) => {
    setExpandedSteps((prev) => {
      const next = new Set(prev);
      if (next.has(idx)) {
        next.delete(idx);
      } else {
        next.add(idx);
      }
      return next;
    });
  }, []);

  const handleRun = useCallback(async () => {
    // 清理上一次结果
    clearSteps();
    setCandidates([]);
    setTrends([]);
    setError(null);
    setEmptyReason(null);
    setStage("loading");
    setCompletedNodes(0);
    setTotalNodes(0);
    setExpandedSteps(new Set());
    eventHandledRef.current = false;

    // 先注册事件监听，再启动 invoke，避免漏掉早期事件
    unlistenStepRef.current?.();
    unlistenDoneRef.current?.();

    try {
      unlistenStepRef.current = await listen<{
        nodeId: string;
        status: string;
        totalNodes: number;
        completedNodes: number;
        output?: unknown;
        error?: string;
        elapsedMs?: number;
      }>("serenity-screening-step", (event) => {
        const p = event.payload;
        const nodeStage = NODE_STAGE_MAP[p.nodeId] ?? "loading";
        setStage(nodeStage);
        setTotalNodes(p.totalNodes ?? 0);
        setCompletedNodes(p.completedNodes ?? 0);
        setCurrentNode(p.nodeId);
        const log: StepLog = {
          nodeId: p.nodeId,
          status: p.status,
          output: p.output,
          error: p.error,
          elapsedMs: p.elapsedMs,
          totalNodes: p.totalNodes,
          completedNodes: p.completedNodes,
          timestamp: Date.now(),
        };
        addStep(log);
      });

      unlistenDoneRef.current = await listen<{
        status: string;
        result?: unknown;
        candidates?: unknown[];
        trends?: TrendInfo[];
        error?: string;
        emptyReason?: string | null;
      }>("serenity-screening-completed", (event) => {
        const p = event.payload;
        eventHandledRef.current = true;
        if (p.status === "failed") {
          setError(p.error ?? t("serenityPanel.errorUnknown"));
          setStage("error");
          setRunning(false);
          setCurrentNode(null);
        } else if (p.status === "completed") {
          console.log(
            "[Serenity] done payload candidates:",
            p.candidates?.length ?? 0,
            "trends:",
            Array.isArray(p.trends) ? p.trends.length : typeof p.trends,
            "result type:",
            Array.isArray(p.result)
              ? "array"
              : typeof p.result === "object" && p.result != null
              ? `object keys=${Object.keys(p.result as Record<string, unknown>).join(",")}`
              : typeof p.result,
          );
          // 优先使用事件 payload 中直接的 candidates 数组
          // 如果 candidates 为空数组但 result 有数据，回退到从 result 提取
          const directCandidates = Array.isArray(p.candidates)
            ? (p.candidates.filter((c: unknown) => c != null) as SerenityCandidate[])
            : null;
          const list = directCandidates && directCandidates.length > 0
            ? directCandidates
            : extractCandidatesList(p.result);
          if (list.length > 0) {
            setCandidates(list);
          } else {
            // 最终兜底：打印完整 payload 帮助诊断
            console.warn(
              "[Serenity] ⚠️ 无法提取任何候选！完整payload:",
              JSON.stringify(p).slice(0, 1000),
            );
          }
          // trends 来自事件
          if (Array.isArray(p.trends)) {
            setTrends(p.trends);
          }
          // 接收后端透传的"为什么没有候选"原因（来自 a-candidate-mapper
          // 的 arguments.summary），在 candidates 为空时展示
          if (typeof p.emptyReason === "string" && p.emptyReason.trim().length > 0) {
            setEmptyReason(p.emptyReason.trim());
          }
          setStage("done");
          setRunning(false);
          setCurrentNode(null);
        }
      });
    } catch {
      // 非 Tauri 环境下 listen 不可用，静默忽略
    }

    setRunning(true);
    try {
      // 读取时间旅行上下文
      const anchorState = useTimeAnchorStore.getState();
      const asOfDate = anchorState.mode === "replay" || anchorState.mode === "backtest_sweep"
        ? anchorState.asOfDate
        : null;
      // invoke 作为兜底：如果 completed 事件已设置结果，这里的重复 set 是无害的；
      // 如果事件未到达（如非 Tauri 环境），invoke 返回值是唯一来源。
      const r = await invoke<SerenityResult>("run_serenity_screening", { asOfDate });
      // 如果事件已经处理过（覆盖了 candidates/trends），这里不要重复 set
      // 但仍要确保 running 状态被关闭
      if (!eventHandledRef.current) {
        const list = extractCandidatesList(r?.candidates);
        if (list.length > 0) {
          setCandidates(list);
        }
        if (Array.isArray(r?.trends) && r.trends.length > 0) {
          setTrends(r.trends);
        }
        if (typeof r?.emptyReason === "string" && r.emptyReason.trim().length > 0) {
          setEmptyReason(r.emptyReason.trim());
        }
        setStage("done");
      }
    } catch (err: unknown) {
      // 仅在 completed 事件未已经处理时才显示错误
      if (!eventHandledRef.current) {
        setError(err instanceof Error ? err.message : String(err));
        setStage("error");
      }
    } finally {
      setRunning(false);
      setCurrentNode(null);
    }
  }, [
    addStep,
    clearSteps,
    setCandidates,
    setEmptyReason,
    setError,
    setRunning,
    setStage,
    setTrends,
    setCompletedNodes,
    setTotalNodes,
    setCurrentNode,
    t,
  ]);

  /** 打开瓶颈掘金历史详情 */
  const openSerenityDetail = useCallback(
    async (row: { generatedAt: string; stockCount: number; createdAt: string }) => {
      setSerenityDetailRow(row);
      setSerenityDetailOpen(true);
      setSerenityDetailLoading(true);
      try {
        const items = await invoke<
          Array<{ stockCode: string; stockName: string; confidence: number; generatedAt: string }>
        >("get_reco_detail", {
          generatedAt: row.generatedAt,
          styleFilter: "serenity",
        });
        setSerenityDetailItems(items ?? []);
      } catch (e) {
        console.error("加载瓶颈掘金详情失败", e);
        setSerenityDetailItems([]);
      }
      setSerenityDetailLoading(false);
    },
    [],
  );

  // 候选标签颜色
  const relevanceColor = (rel: string) => {
    if (rel === "direct") { return "green"; }
    if (rel === "indirect") { return "blue"; }
    return "default";
  };
  const relevanceLabel = (rel: string) => {
    if (rel === "direct") { return t("serenityPanel.directBenefit"); }
    if (rel === "indirect") { return t("serenityPanel.indirectBenefit"); }
    return t("serenityPanel.themeRelated");
  };

  const catalystColor = (type: string) => {
    if (type === "earnings") { return "green"; }
    if (type === "production_ramp") { return "blue"; }
    if (type === "policy") { return "orange"; }
    if (type === "supply_shock") { return "red"; }
    if (type === "capacity_release") { return "purple"; }
    if (type === "contract_award") { return "cyan"; }
    return "default";
  };
  const catalystLabel = (type: string) => {
    const map: Record<string, string> = {
      earnings: t("serenityPanel.catalystEarnings"),
      production_ramp: t("serenityPanel.catalystProdRamp"),
      policy: t("serenityPanel.catalystPolicy"),
      supply_shock: t("serenityPanel.catalystSupplyShock"),
      capacity_release: t("serenityPanel.catalystCapacityRelease"),
      contract_award: t("serenityPanel.catalystContractAward"),
    };
    return map[type] ?? type;
  };
  const timeframeLabel = (tf: string) => {
    if (tf === "short_term") { return t("serenityPanel.timeframeShort"); }
    if (tf === "mid_term") { return t("serenityPanel.timeframeMid"); }
    return t("serenityPanel.timeframeLong");
  };
  const exitUrgencyColor = (urgency?: string) => {
    if (urgency === "exit_now") { return "red"; }
    if (urgency === "caution") { return "orange"; }
    if (urgency === "watch") { return "blue"; }
    return "default";
  };
  const attentionColor = (score?: number) => {
    if (score == null) { return "default"; }
    if (score <= 30) { return "green"; }
    if (score <= 60) { return "blue"; }
    return "red";
  };

  // 当前阶段文案
  const stageLabel = (() => {
    if (!running && stage === "done") { return t("serenityPanel.stage_done"); }
    if (!running && stage === "error") { return t("serenityPanel.stage_error"); }
    switch (stage) {
      case "loading":
        return t("serenityPanel.stage_loading");
      case "scanning":
        return t("serenityPanel.stage_scanning");
      case "decomposing":
        return t("serenityPanel.stage_decomposing");
      case "identifying":
        return t("serenityPanel.stage_identifying");
      case "mapping":
        return t("serenityPanel.stage_mapping");
      case "saving":
        return t("serenityPanel.stage_saving");
      default:
        return t("serenityPanel.running");
    }
  })();

  const progressPct = totalNodes > 0 ? Math.round((completedNodes / totalNodes) * 100) : 0;

  return (
    <div className="flex flex-col gap-3">
      {/* 操作栏 */}
      <div className="flex items-center justify-between">
        <Text type="secondary" className="text-xs">
          {t("serenityPanel.desc")}
        </Text>
        <div className="flex items-center gap-2">
          <Button
            size="small"
            icon={<HistoryOutlined />}
            onClick={async () => {
              setSerenityHistoryOpen(true);
              setSerenityHistoryLoading(true);
              try {
                const list = await invoke<typeof serenityHistory>("list_reco_history", {
                  styleFilter: "serenity",
                  limit: 50,
                });
                setSerenityHistory(list ?? []);
              } catch { /* */ }
              setSerenityHistoryLoading(false);
            }}
          >
            {t("serenityPanel.serenityHistory.viewHistory")}
          </Button>
          <Button
            size="small"
            icon={<AlertOutlined />}
            loading={feedbackLoading}
            onClick={async () => {
              setFeedbackLoading(true);
              try {
                const anchorState = useTimeAnchorStore.getState();
                const asOfDate = anchorState.mode === "replay"
                  ? anchorState.asOfDate
                  : null;
                const r = await invoke<typeof feedbackData>(
                  "refresh_serenity_feedback",
                  { asOfDate },
                );
                setFeedbackData(r);
              } catch (e) {
                console.error("回馈闭环分析失败", e);
              } finally {
                setFeedbackLoading(false);
              }
            }}
          >
            {t("serenityPanel.feedbackButton")}
          </Button>
          <Button
            type="primary"
            icon={running ? <ReloadOutlined spin /> : <PlayCircleOutlined />}
            loading={running}
            onClick={handleRun}
          >
            {running ? t("serenityPanel.running") : t("serenityPanel.run")}
          </Button>
        </div>
      </div>

      {/* 过滤参数设置 */}
      <Card
        size="small"
        className="w-full"
        title={
          <div
            className="flex items-center gap-2 cursor-pointer text-sm"
            onClick={() => setSerenitySettingsOpen(!serenitySettingsOpen)}
          >
            <span>{serenitySettingsOpen ? "▼" : "▶"} {t("serenityPanel.settings")}</span>
          </div>
        }
      >
        {serenitySettingsOpen && (
          <div className="grid grid-cols-2 gap-3">
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">PE 上限</Text>
              <InputNumber
                size="small"
                min={0}
                max={1000}
                value={serenityVars.max_pe ?? 100}
                onChange={(v) => handleSerenityVarChange("max_pe", v ?? 100)}
                className="w-24"
                suffix="倍"
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">PB 上限</Text>
              <InputNumber
                size="small"
                min={0}
                max={100}
                value={serenityVars.max_pb ?? 10}
                onChange={(v) => handleSerenityVarChange("max_pb", v ?? 10)}
                className="w-24"
                suffix="倍"
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">近3月涨幅上限</Text>
              <InputNumber
                size="small"
                min={0}
                max={500}
                value={serenityVars.max_3m_gain_pct ?? 30}
                onChange={(v) => handleSerenityVarChange("max_3m_gain_pct", v ?? 30)}
                className="w-24"
                suffix="%"
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">近12月涨幅上限</Text>
              <InputNumber
                size="small"
                min={0}
                max={500}
                value={serenityVars.max_12m_gain_pct ?? 100}
                onChange={(v) => handleSerenityVarChange("max_12m_gain_pct", v ?? 100)}
                className="w-24"
                suffix="%"
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">毛利率下限</Text>
              <InputNumber
                size="small"
                min={0}
                max={100}
                value={serenityVars.min_gross_margin ?? 25}
                onChange={(v) => handleSerenityVarChange("min_gross_margin", v ?? 25)}
                className="w-24"
                suffix="%"
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">负债率上限</Text>
              <InputNumber
                size="small"
                min={0}
                max={100}
                value={serenityVars.max_debt_ratio ?? 60}
                onChange={(v) => handleSerenityVarChange("max_debt_ratio", v ?? 60)}
                className="w-24"
                suffix="%"
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">高增长豁免阈值</Text>
              <InputNumber
                size="small"
                min={10}
                max={200}
                value={serenityVars.growth_exempt_pct ?? 50}
                onChange={(v) => handleSerenityVarChange("growth_exempt_pct", v ?? 50)}
                className="w-24"
                suffix="%"
              />
            </div>
            <div className="col-span-2 text-xs text-gray-400 mt-1">
              PE超上限但营收增速≥此值时豁免，保护高增长标的不被误杀
            </div>
          </div>
        )}
      </Card>

      {/* 进度状态 */}
      {running && (
        <Card size="small" className="w-full">
          <div className="flex flex-col gap-2">
            <div className="flex items-center gap-2 text-sm">
              <Spin indicator={<LoadingOutlined spin />} size="small" />
              <span className="font-medium">{stageLabel}</span>
              {currentNodeId && (
                <Text type="secondary" className="text-xs">
                  {t(nodeTitleKey(currentNodeId))}
                </Text>
              )}
            </div>
            {totalNodes > 0 && (
              <div className="flex items-center gap-2">
                <Progress
                  percent={progressPct}
                  size="small"
                  className="flex-1"
                  format={() => `${completedNodes}/${totalNodes}`}
                />
              </div>
            )}
          </div>
        </Card>
      )}

      {/* 执行日志 */}
      {steps.length > 0 && (
        <Card
          size="small"
          title={
            <div className="flex items-center gap-2 text-sm">
              <ClockCircleOutlined />
              <span>{t("serenityPanel.stepLogTitle")}</span>
              <Tag className="text-xs">{steps.length}</Tag>
            </div>
          }
          className="w-full"
          styles={{ body: { maxHeight: 360, overflowY: "auto" } }}
        >
          <Space direction="vertical" className="w-full" size={4}>
            {steps.map((s, i) => {
              const isExpanded = expandedSteps.has(i);
              const statusColor = s.status === "completed"
                ? "green"
                : s.status === "failed"
                ? "red"
                : "blue";
              const statusIcon = s.status === "completed"
                ? <CheckCircleOutlined style={{ color: "#52c41a" }} />
                : s.status === "failed"
                ? <span style={{ color: "#ff4d4f" }}>✕</span>
                : <LoadingOutlined style={{ color: "#1677ff" }} />;
              const outputText = summarizeOutput(s.output);
              return (
                <div
                  key={`${s.nodeId}-${i}`}
                  className="rounded border border-gray-100 px-2 py-1 text-xs"
                >
                  <div
                    className="flex items-center gap-2 cursor-pointer"
                    onClick={() => toggleStep(i)}
                  >
                    {statusIcon}
                    <Text strong className="text-xs">
                      {t(nodeTitleKey(s.nodeId))}
                    </Text>
                    <Tag color={statusColor} className="text-xs">
                      {s.status}
                    </Tag>
                    {s.elapsedMs != null && (
                      <Text type="secondary" className="text-xs">
                        {(s.elapsedMs / 1000).toFixed(1)}s
                      </Text>
                    )}
                    <div className="flex-1" />
                    <Text
                      type={outputText ? "secondary" : undefined}
                      className="text-xs cursor-pointer"
                      onClick={(e) => {
                        e.stopPropagation();
                        toggleStep(i);
                      }}
                    >
                      {isExpanded
                        ? <DownOutlined />
                        : <RightOutlined />}
                    </Text>
                  </div>
                  {isExpanded && (
                    outputText
                      ? (
                        <pre
                          className="mt-1 max-h-48 overflow-auto rounded p-2 text-xs whitespace-pre-wrap break-all"
                          style={{
                            backgroundColor: "rgba(255,255,255,0.04)",
                            border: "1px solid rgba(255,255,255,0.08)",
                            color: "rgba(230,230,230,0.9)",
                          }}
                        >
                        {outputText.length > 2000
                          ? outputText.slice(0, 2000) + "..."
                          : outputText}
                        </pre>
                      )
                      : s.error
                      ? <div className="mt-1 text-xs text-red-500">{s.error}</div>
                      : (
                        <div className="mt-1 text-xs text-gray-400 italic">
                          {s.status === "completed"
                            ? "(执行完成，无输出内容)"
                            : s.status === "running"
                            ? "(正在执行...)"
                            : "(无详细输出)"}
                        </div>
                      )
                  )}
                </div>
              );
            })}
          </Space>
        </Card>
      )}

      {/* 错误 */}
      {error && !running && (
        <div
          className="rounded border border-red-500/30 p-2 text-sm text-red-400"
          style={{ backgroundColor: "rgba(255,77,79,0.08)" }}
        >
          {error}
        </div>
      )}

      {/* 趋势摘要 */}
      {trends.length > 0 && !running && (
        <Card size="small" title={t("serenityPanel.trendTitle")} className="w-full">
          <Space direction="vertical" className="w-full">
            {trends.map((tr, i) => (
              <div key={i} className="flex items-center gap-2 text-sm">
                <Tag color="purple">{tr.confidence ?? "?"}%</Tag>
                <Text strong>{tr.trend_name ?? tr.trendName}</Text>
                {tr.bottleneck_candidate && (
                  <Text type="secondary" className="text-xs">
                    {t("serenityPanel.bottleneckLink")}
                    {tr.bottleneck_candidate}
                  </Text>
                )}
              </div>
            ))}
          </Space>
        </Card>
      )}

      {/* 候选股列表 */}
      {candidates.length > 0 && (
        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <Title level={5} className="m-0">
              {t("serenityPanel.candidateTitle")} ({candidates.length})
            </Title>
            <Button
              size="small"
              icon={<AlertOutlined />}
              onClick={async () => {
                try {
                  const anchorState = useTimeAnchorStore.getState();
                  const asOfDate = anchorState.mode === "replay" || anchorState.mode === "backtest_sweep"
                    ? anchorState.asOfDate
                    : null;
                  const r = await invoke<{
                    status: string;
                    checked_count: number;
                    exit_now_count: number;
                    caution_count: number;
                    candidates: Array<{
                      stock_code: string;
                      stock_name: string;
                      exit_urgency: string;
                      has_disruption_news: boolean;
                      margin_declining: boolean;
                    }>;
                  }>("refresh_serenity_exit_signals", { asOfDate });
                  if (r.exit_now_count > 0 || r.caution_count > 0) {
                    alert(
                      `退出信号扫描完成:\n检查 ${r.checked_count} 只\n- 立即退出: ${r.exit_now_count} 只\n- 谨慎关注: ${r.caution_count} 只`,
                    );
                  } else {
                    alert(`退出信号扫描完成: ${r.checked_count} 只，暂无异常`);
                  }
                } catch (e) {
                  console.error("刷新退出信号失败", e);
                }
              }}
            >
              {t("serenityPanel.refreshExitButton")}
            </Button>
          </div>
          {candidates.map((c, i) => {
            const code = c.stock_code ?? c.stockCode ?? "";
            const name = c.stockName ?? c.stock_name ?? "";
            return (
              <Card
                key={`${code}-${i}`}
                size="small"
                hoverable
                className="w-full"
                onClick={() => navigate(`/stock-analysis?code=${code}`, { replace: true })}
              >
                <div className="flex items-start justify-between">
                  <div className="flex flex-col gap-1">
                    <div className="flex items-center gap-2">
                      <Text strong className="text-sm">
                        {name}
                      </Text>
                      <Text type="secondary" className="text-xs font-mono">
                        {code}
                      </Text>
                      {c.relevance && (
                        <Tag color={relevanceColor(c.relevance)} className="text-xs">
                          {relevanceLabel(c.relevance)}
                        </Tag>
                      )}
                    </div>
                    {c.bottleneckProduct ?? c.bottleneck_product
                      ? (
                        <Text type="secondary" className="text-xs">
                          {t("serenityPanel.bottleneckProduct")}
                          {c.bottleneckProduct ?? c.bottleneck_product}
                        </Text>
                      )
                      : null}
                    {c.primaryRisk ?? c.primary_risk
                      ? (
                        <Text type="danger" className="text-xs">
                          {t("serenityPanel.riskPrefix")}
                          {c.primaryRisk ?? c.primary_risk}
                        </Text>
                      )
                      : null}

                    {/* 催化剂 */}
                    {(c.catalysts ?? []).length > 0 && (
                      <div className="flex flex-wrap items-center gap-1 mt-1">
                        <Text type="secondary" className="text-xs mr-1">
                          {t("serenityPanel.catalystLabelPrefix")}
                        </Text>
                        {(c.catalysts ?? []).slice(0, 2).map((cat, ci) => (
                          <Tag
                            key={ci}
                            color={catalystColor(cat.type)}
                            className="text-xs"
                            title={cat.description}
                          >
                            {catalystLabel(cat.type)} {timeframeLabel(cat.expected_timeframe)} {cat.confidence}%
                          </Tag>
                        ))}
                      </div>
                    )}

                    {/* 退出信号 */}
                    {(() => {
                      const es: ExitSignals | undefined = c.exit_signals ?? c.exitSignals;
                      if (!es) { return null; }
                      return (
                        <div className="flex items-center gap-2 mt-1">
                          {es.overall_exit_urgency && (
                            <Tag color={exitUrgencyColor(es.overall_exit_urgency)} className="text-xs">
                              {t("serenityPanel.exitLabelPrefix")}
                              {es.overall_exit_urgency === "exit_now"
                                ? t("serenityPanel.exitNow")
                                : es.overall_exit_urgency === "caution"
                                ? t("serenityPanel.exitCaution")
                                : es.overall_exit_urgency === "watch"
                                ? t("serenityPanel.exitWatch")
                                : t("serenityPanel.exitNone")}
                            </Tag>
                          )}
                        </div>
                      );
                    })()}

                    {/* 关注度 */}
                    {(() => {
                      const am: AttentionMetrics | undefined = c.attention_metrics ?? c.attentionMetrics;
                      if (!am) { return null; }
                      return (
                        <div className="flex items-center gap-2 mt-1">
                          {am.attention_score != null && (
                            <Tag color={attentionColor(am.attention_score)} className="text-xs">
                              {t("serenityPanel.attentionLabelPrefix")}
                              {am.attention_score}
                            </Tag>
                          )}
                          {am.search_heat && (
                            <Text type="secondary" className="text-xs">
                              {t("serenityPanel.heatLabelPrefix")}
                              {am.search_heat}
                            </Text>
                          )}
                        </div>
                      );
                    })()}
                  </div>
                  <div className="flex flex-col items-end gap-1">
                    <Tag color="purple" className="text-xs font-bold">
                      {c.serenityScore ?? c.serenity_score ?? 0}
                      {t("serenityPanel.scoreSuffix")}
                    </Tag>
                    {c.confidence
                      ? (
                        <Text type="secondary" className="text-xs">
                          {t("serenityPanel.confidencePrefix")}
                          {c.confidence}%
                        </Text>
                      )
                      : null}
                  </div>
                </div>
              </Card>
            );
          })}
        </div>
      )}

      {/* 空状态 / 解释无候选原因 */}
      {!running && !error && candidates.length === 0 && trends.length === 0 && (
        emptyReason
          ? (
            <Alert
              type="info"
              showIcon
              message={t("serenityPanel.noCandidateTitle")}
              description={emptyReason}
              className="w-full"
            />
          )
          : (
            <Empty
              image={<StockOutlined style={{ fontSize: 48, opacity: 0.3 }} />}
              description={t("serenityPanel.emptyHint")}
            />
          )
      )}
      {!running && !error && candidates.length === 0 && trends.length > 0 && emptyReason && (
        <Alert
          type="info"
          showIcon
          message={t("serenityPanel.noCandidateTitle")}
          description={emptyReason}
          className="w-full"
        />
      )}

      {/* 回馈闭环结果 */}
      {feedbackLoading && (
        <Card size="small" className="w-full">
          <div className="flex gap-4 mb-3">
            <div
              className="text-center flex-1 rounded p-3 animate-pulse"
              style={{ backgroundColor: "rgba(255,255,255,0.04)" }}
            />
            <div
              className="text-center flex-1 rounded p-3 animate-pulse"
              style={{ backgroundColor: "rgba(255,255,255,0.04)" }}
            />
            <div
              className="text-center flex-1 rounded p-3 animate-pulse"
              style={{ backgroundColor: "rgba(255,255,255,0.04)" }}
            />
          </div>
        </Card>
      )}
      {feedbackData && (
        <Card
          size="small"
          title={t("serenityPanel.feedbackTitle", { count: feedbackData.total })}
          className="w-full"
          extra={
            <Text type="secondary" className="text-xs">
              {new Date().toLocaleDateString()}
            </Text>
          }
        >
          <div className="flex gap-4 mb-3">
            <div className="text-center flex-1 rounded p-2" style={{ backgroundColor: "rgba(255,255,255,0.04)" }}>
              <div
                className="text-lg font-bold"
                style={{ color: feedbackData.win_rate >= 0.5 ? "#52c41a" : "#ff4d4f" }}
              >
                {(feedbackData.win_rate * 100).toFixed(0)}%
              </div>
              <div className="text-xs opacity-50">{t("serenityPanel.feedbackWinRate")}</div>
            </div>
            <div className="text-center flex-1 rounded p-2" style={{ backgroundColor: "rgba(255,255,255,0.04)" }}>
              <div
                className="text-lg font-bold"
                style={{ color: feedbackData.avg_return_pct >= 0 ? "#52c41a" : "#ff4d4f" }}
              >
                {feedbackData.avg_return_pct.toFixed(1)}%
              </div>
              <div className="text-xs opacity-50">{t("serenityPanel.feedbackAvgReturn")}</div>
            </div>
            <div className="text-center flex-1 rounded p-2" style={{ backgroundColor: "rgba(255,255,255,0.04)" }}>
              <div className="text-lg font-bold">{feedbackData.profitable_count}/{feedbackData.total}</div>
              <div className="text-xs opacity-50">{t("serenityPanel.feedbackProfitable")}</div>
            </div>
          </div>
          {/* 个股表现列表 */}
          <div className="max-h-48 overflow-y-auto">
            {feedbackData.performances.slice(0, 50).map((p, i) => (
              <div
                key={p.id ?? i}
                className="flex items-center justify-between py-1 text-xs border-b border-white/5 last:border-0"
              >
                <div className="flex items-center gap-2">
                  <span className="font-mono">{p.stock_code}</span>
                  <span className="text-gray-500">{p.stock_name}</span>
                  <span className="text-gray-400 text-2xs">{p.recommend_date}</span>
                </div>
                <div className="flex items-center gap-3">
                  {p.catalysts && (
                    <Tag className="text-2xs" color={p.catalysts.verified > 0 ? "green" : "default"}>
                      {t("serenityPanel.feedbackCatalyst", {
                        verified: p.catalysts.verified,
                        total: p.catalysts.total,
                      })}
                    </Tag>
                  )}
                  <span style={{ color: p.return_pct >= 0 ? "#52c41a" : "#ff4d4f" }}>
                    {p.return_pct > 0 ? "+" : ""}
                    {p.return_pct.toFixed(1)}%
                  </span>
                  <Button
                    type="text"
                    size="small"
                    danger
                    className="text-xs opacity-40 hover:opacity-100"
                    onClick={async (e) => {
                      e.stopPropagation();
                      try {
                        await invoke("delete_serenity_pick", { id: p.id });
                        setFeedbackData((prev) => {
                          if (!prev) { return prev; }
                          const perf = prev.performances.filter((x) => x.id !== p.id);
                          const total = perf.length;
                          const profitable = perf.filter((x) => x.is_profitable).length;
                          const avg_return = total > 0
                            ? perf.reduce((s, x) => s + x.return_pct, 0) / total
                            : 0;
                          return {
                            ...prev,
                            total,
                            profitable_count: profitable,
                            win_rate: total > 0 ? profitable / total : 0,
                            avg_return_pct: Number(avg_return.toFixed(2)),
                            performances: perf,
                          };
                        });
                      } catch (e) {
                        console.error("删除失败", e);
                      }
                    }}
                  >
                    ✕
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}

      {/* 瓶颈掘金历史（多选 + 批量删除） */}
      <Modal
        title={t("serenityPanel.serenityHistory.title")}
        open={serenityHistoryOpen}
        onCancel={() => {
          setSerenityHistoryOpen(false);
          setSerenitySelected([]);
        }}
        footer={serenitySelected.length > 0
          ? (
            <div className="flex items-center gap-2">
              <span className="text-xs text-gray-400">
                {t("serenityPanel.serenityHistory.selectedCount", { count: serenitySelected.length })}
              </span>
              <Button size="small" onClick={() => setSerenitySelected([])}>
                {t("serenityPanel.serenityHistory.exitSelect")}
              </Button>
              <Button
                size="small"
                danger
                loading={serenityDeleting}
                onClick={async () => {
                  setSerenityDeleting(true);
                  try {
                    await invoke("batch_delete_reco_history", { generatedAts: serenitySelected });
                    message.success(
                      t("serenityPanel.serenityHistory.deleteSuccess", { count: serenitySelected.length }),
                    );
                    setSerenityHistory((prev) => prev.filter((r) => !serenitySelected.includes(r.generatedAt)));
                    setSerenitySelected([]);
                  } catch (e) {
                    message.error(String(e));
                  }
                  setSerenityDeleting(false);
                }}
              >
                {t("serenityPanel.serenityHistory.batchDelete", { count: serenitySelected.length })}
              </Button>
            </div>
          )
          : null}
        width={560}
      >
        <Table
          size="small"
          loading={serenityHistoryLoading}
          dataSource={serenityHistory}
          rowKey="generatedAt"
          pagination={false}
          onRow={(record) => ({
            className: "cursor-pointer",
            onClick: () => openSerenityDetail(record),
          })}
          columns={[
            {
              title: (
                <Checkbox
                  checked={serenityHistory.length > 0 && serenitySelected.length === serenityHistory.length}
                  indeterminate={serenitySelected.length > 0 && serenitySelected.length < serenityHistory.length}
                  onChange={(e) => {
                    setSerenitySelected(e.target.checked ? serenityHistory.map((r) => r.generatedAt) : []);
                  }}
                />
              ),
              key: "select",
              width: 40,
              render: (_, r) => (
                <Checkbox
                  checked={serenitySelected.includes(r.generatedAt)}
                  onClick={(e) => e.stopPropagation()}
                  onChange={(e) => {
                    setSerenitySelected(
                      e.target.checked
                        ? [...serenitySelected, r.generatedAt]
                        : serenitySelected.filter((g) => g !== r.generatedAt),
                    );
                  }}
                />
              ),
            },
            {
              title: t("serenityPanel.serenityHistory.generatedAt"),
              dataIndex: "generatedAt",
              key: "generatedAt",
              render: (v: string) => (
                <span className="text-xs font-mono">
                  {new Date(v).toLocaleString()}
                </span>
              ),
            },
            {
              title: t("serenityPanel.serenityHistory.candidateCount"),
              dataIndex: "stockCount",
              key: "stockCount",
              render: (v: number) => <span className="text-xs">{v} 只</span>,
            },
          ]}
        />
      </Modal>

      {/* 瓶颈掘金历史详情 */}
      <Modal
        title={serenityDetailRow
          ? `${t("serenityPanel.serenityHistory.title")} — ${new Date(serenityDetailRow.generatedAt).toLocaleString()}`
          : ""}
        open={serenityDetailOpen}
        onCancel={() => {
          setSerenityDetailOpen(false);
          setSerenityDetailItems([]);
          setSerenityDetailRow(null);
        }}
        footer={null}
        width={600}
      >
        {serenityDetailLoading
          ? (
            <div className="py-8 text-center text-sm text-gray-400">
              {t("common.loading")}
            </div>
          )
          : serenityDetailItems.length === 0
          ? <Empty description={t("serenityPanel.serenityHistory.detailEmpty")} />
          : (
            <div className="flex flex-col gap-2">
              <div className="text-xs text-gray-500 mb-1">
                {t("serenityPanel.serenityHistory.candidateCount")}: {serenityDetailItems.length} 只
              </div>
              {serenityDetailItems.map((item, i) => (
                <Card
                  key={`${item.stockCode}-${i}`}
                  size="small"
                  hoverable
                  className="w-full"
                  onClick={() => {
                    setSerenityDetailOpen(false);
                    navigate(`/stock-analysis?code=${item.stockCode}`, { replace: true });
                  }}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Text strong className="text-sm">{item.stockName}</Text>
                      <Text type="secondary" className="text-xs font-mono">{item.stockCode}</Text>
                    </div>
                    <Tag color="purple" className="text-xs font-bold">
                      {t("serenityPanel.confidencePrefix")} {item.confidence}
                    </Tag>
                  </div>
                  <div className="mt-1 text-[10px] text-gray-500">
                    {new Date(item.generatedAt).toLocaleString()}
                  </div>
                </Card>
              ))}
            </div>
          )}
      </Modal>
    </div>
  );
}
