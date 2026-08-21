import { invoke, listen, TimeoutError as InvokeTimeoutError } from "@/lib/invoke";
import {
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
import {
  Alert,
  App,
  Button,
  Card,
  Checkbox,
  Empty,
  InputNumber,
  Modal,
  Progress,
  Select,
  Space,
  Spin,
  Table,
  Tag,
  Typography,
} from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate, useSearchParams } from "react-router-dom";
import { SerenityCandidateCard } from "./SerenityCandidateCard";

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

/// get_reco_detail 返回的单条候选记录（serde camelCase）
interface RecoDetailItem {
  id: string;
  generatedAt: string;
  period: string;
  stockCode: string;
  stockName: string;
  style: string;
  confidence: number;
  synthetic: number;
  seedPoolJson?: string | null;
  pickData?: string | null;
  createdAt: string;
}

/// 从 reco_picks 落库数据还原 SerenityCandidate：
/// 优先解析 seed_pool_json（serenity-screening 工作流写的 candidate 对象），
/// 失败时用基础列构造兜底候选（智能荐股 bottleneck 行的 seed_pool_json 是
/// 推荐池 [[code,name]] 数组，无法解析为单个候选，必须走 fallback）。
function restoreCandidate(item: RecoDetailItem): SerenityCandidate | null {
  if (item.seedPoolJson) {
    try {
      const parsed = JSON.parse(item.seedPoolJson) as unknown;
      // 只接受单个对象（serenity workflow 写的 candidate）—— 数组（推荐池快照）
      // 和其它 shape 一律走 fallback，否则 SerenityCandidate 字段全是 undefined
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
        return parsed as SerenityCandidate;
      }
    } catch {
      // seed_pool_json 损坏时降级到基础字段
    }
  }
  if (!item.stockCode) { return null; }
  return {
    stockCode: item.stockCode,
    stockName: item.stockName,
    confidence: item.confidence,
  };
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
function extractCandidatesList(raw: unknown, depth = 0): SerenityCandidate[] {
  // 防御：LLM 可能返回多层 Agent 包装（content 套 content），限制递归深度避免栈溢出
  if (depth > 10) { return []; }
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
        return extractCandidatesList(parsed, depth + 1);
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
// 与 seed_serenity.rs 中实际节点 ID 同步（v4 模板）
const NODE_STAGE_MAP: Record<string, StepStage> = {
  trigger: "loading",
  // Phase 0: 市场扫描
  "t-industry-rank": "scanning",
  "t-cls-flash": "scanning",
  "t-northbound": "scanning",
  "t-baseline-semi": "scanning",
  "t-baseline-battery": "scanning",
  "t-baseline-chem": "scanning",
  "t-baseline-med": "scanning",
  "t-baseline-aero": "scanning",
  "t-baseline-consumer-elec": "scanning",
  "t-baseline-auto": "scanning",
  "t-signal-semi": "scanning",
  "t-signal-battery": "scanning",
  "t-signal-chem": "scanning",
  "t-signal-med": "scanning",
  "t-signal-aero": "scanning",
  "t-signal-consumer-elec": "scanning",
  "t-signal-auto": "scanning",
  "a-trend-scanner": "scanning",
  // Phase 1: 产业链拆解
  "a-chain-trend1": "decomposing",
  "a-chain-trend2": "decomposing",
  "a-chain-trend3": "decomposing",
  "a-chain-trend4": "decomposing",
  "a-chain-trend5": "decomposing",
  // Phase 2: 瓶颈指标计算
  "c-bottleneck-trend1": "identifying",
  "c-bottleneck-trend2": "identifying",
  "c-bottleneck-trend3": "identifying",
  "c-bottleneck-trend4": "identifying",
  "c-bottleneck-trend5": "identifying",
  "c-consistency-check": "identifying",
  // Phase 3: 候选公司映射
  "a-candidate-mapper": "mapping",
  "c-data-verifier": "mapping",
  // Phase 4: 保存
  "s-save-candidates": "saving",
};

/// 将节点 ID 映射为 i18n 标题 key
function nodeTitleKey(nodeId: string): string {
  return `serenityPanel.nodeTitles.${nodeId}`;
}

// ═══ 节点输出语义化渲染 ═══
// 引擎 executor 输出形态（已实锤）：
//   ToolNode  → { tool_name, result(JSON字符串), truncated, is_error, node_id }
//   CodeNode  → { status:"executed", language, result(JSON), params, input_params, node_id }
//   AgentNode → { content, thinking, tool_calls, usage, iterations, stopped_by_limit }

/// 常见数据字段 → 中文列名（数据 schema 翻译，非 UI 文案，不走 i18n）
const FIELD_LABEL_MAP: Record<string, string> = {
  // 通用
  stock_code: "serenityPanel.colStockCode",
  stockCode: "serenityPanel.colStockCode",
  stock_name: "serenityPanel.colName",
  stockName: "serenityPanel.colName",
  name: "serenityPanel.colName",
  code: "serenityPanel.colCode",
  price: "serenityPanel.colPrice",
  change_pct: "serenityPanel.colChangePct",
  changePct: "serenityPanel.colChangePct",
  pct_chg: "serenityPanel.colChangePct",
  status: "serenityPanel.colStatus",
  result: "serenityPanel.colResult",
  params: "serenityPanel.colParams",
  // 行业排名
  industry: "serenityPanel.colIndustry",
  industry_name: "serenityPanel.colIndustry",
  industryName: "serenityPanel.colIndustry",
  rank: "serenityPanel.colRank",
  change_pct_3m: "serenityPanel.colChangePct3m",
  changePct3m: "serenityPanel.colChangePct3m",
  change_pct_1m: "serenityPanel.colChangePct1m",
  changePct1m: "serenityPanel.colChangePct1m",
  leading_stocks: "serenityPanel.colLeadingStocks",
  leadingStocks: "serenityPanel.colLeadingStocks",
  turnover: "serenityPanel.colTurnover",
  volume: "serenityPanel.colVolume",
  avg_price: "serenityPanel.colAvgPrice",
  // 北向资金
  northbound_hold: "serenityPanel.colNorthboundHold",
  northboundHold: "serenityPanel.colNorthboundHold",
  hold_value: "serenityPanel.colHoldValue",
  holdValue: "serenityPanel.colHoldValue",
  // 趋势/信号
  trend_name: "serenityPanel.colTrend",
  trendName: "serenityPanel.colTrend",
  confidence: "serenityPanel.colConfidence",
  score: "serenityPanel.colScore",
  total_score: "serenityPanel.colTotalScore",
  totalScore: "serenityPanel.colTotalScore",
  // 财报
  revenue: "serenityPanel.colRevenue",
  net_profit: "serenityPanel.colNetProfit",
  netProfit: "serenityPanel.colNetProfit",
  gross_margin: "serenityPanel.colGrossMargin",
  grossMargin: "serenityPanel.colGrossMargin",
  pe: "PE",
  pb: "PB",
  roe: "ROE",
};

/** 节点输出分析结果：语义化渲染所需的最小结构化数据 */
interface NodeOutputView {
  kind: "tool" | "code" | "agent" | "json" | "text" | "empty";
  /** 数组数据（表格渲染） */
  table?: { columns: string[]; rows: Array<Record<string, unknown>> };
  /** 展开态完整展示文本（JSON 美化或原文） */
  jsonText: string;
  /** 文本预览（agent 节点取 content 摘要） */
  textPreview?: string;
  /** 数组条数（折叠态摘要用） */
  count?: number;
  /** 对象有值字段数（折叠态摘要用） */
  fieldCount?: number;
}

/** 宽松解析：字符串 → JSON；非 JSON 字符串或解析失败返回原文 */
function looseParse(v: unknown): unknown {
  if (typeof v !== "string") { return v; }
  const t = v.trim();
  if (!t.startsWith("{") && !t.startsWith("[")) { return v; }
  try {
    return JSON.parse(t);
  } catch {
    return v;
  }
}

/** 提取数组行的列：保持首元素字段顺序，补充后续行的新字段 */
function collectColumns(rows: Array<Record<string, unknown>>): string[] {
  const cols: string[] = [];
  const seen = new Set<string>();
  for (const r of rows) {
    for (const k of Object.keys(r)) {
      if (!seen.has(k)) {
        seen.add(k);
        cols.push(k);
      }
    }
  }
  return cols;
}

/** 单元格清洗：null → "—"；number → 2 位小数；对象/数组 → 精简 JSON */
function cellText(v: unknown, translate?: (key: string) => string): string {
  if (v == null) { return "—"; }
  if (typeof v === "number") {
    return Number.isFinite(v) ? (Math.round(v * 100) / 100).toString() : String(v);
  }
  if (typeof v === "boolean") {
    return translate
      ? (v ? translate("serenityPanel.boolYes") : translate("serenityPanel.boolNo"))
      : (v ? "Yes" : "No");
  }
  if (typeof v === "string") { return v.length > 60 ? v.slice(0, 60) + "…" : v; }
  const s = JSON.stringify(v);
  return s && s.length > 60 ? s.slice(0, 60) + "…" : (s ?? "—");
}

/** 分析节点输出 → 语义化视图（纯函数，不含 i18n 文案，文案在渲染处拼装） */
function buildNodeOutputView(_nodeId: string, output: unknown): NodeOutputView {
  const empty: NodeOutputView = { kind: "empty", jsonText: "" };
  if (output == null) { return empty; }
  if (typeof output === "string" && output.trim().length === 0) { return empty; }

  // 1. 按包装结构解包 → payload + 节点类型
  let kind: NodeOutputView["kind"] = "text";
  let payload: unknown = output;
  if (typeof output === "object" && !Array.isArray(output)) {
    const o = output as Record<string, unknown>;
    if (typeof o.tool_name === "string" && "result" in o) {
      // ToolNode：result 可能是 ToolResult 形态 {content, truncated, is_error, ...}，
      // content 才是工具数据（JSON 字符串）。再解一层。
      kind = "tool";
      let r = o.result;
      if (r && typeof r === "object" && !Array.isArray(r)) {
        const rr = r as Record<string, unknown>;
        if (typeof rr.content === "string") { r = rr.content; }
      }
      payload = looseParse(r);
    } else if (o.status === "executed" && "result" in o) {
      // CodeNode：result 为 Rhai 脚本返回值
      kind = "code";
      payload = looseParse(o.result);
    } else if (typeof o.content === "string") {
      // AgentNode：content 为 LLM 输出（可能为 JSON 字符串）
      kind = "agent";
      payload = looseParse(o.content);
    } else if ("result" in o) {
      payload = looseParse(o.result);
    }
  } else if (typeof output === "string") {
    const parsed = looseParse(output);
    if (parsed !== output) {
      kind = "json";
      payload = parsed;
    }
  }

  // 1.5 解包后为 null（如北向资金返回 "null"）→ 空数据语义
  if (payload === null) {
    return { kind, jsonText: "null", count: 0 };
  }

  // 2. 数组 → 表格（元素为对象时）
  if (Array.isArray(payload)) {
    const rows = payload.filter(
      (r): r is Record<string, unknown> => !!r && typeof r === "object" && !Array.isArray(r),
    );
    if (rows.length > 0) {
      return {
        kind,
        table: { columns: collectColumns(rows), rows },
        jsonText: JSON.stringify(payload, null, 2),
        count: payload.length,
      };
    }
    return { kind, jsonText: JSON.stringify(payload, null, 2), count: payload.length };
  }

  // 3. 对象 → 键值/JSON 展示
  if (payload && typeof payload === "object") {
    const obj = payload as Record<string, unknown>;
    const keys = Object.keys(obj).filter((k) => {
      const v = obj[k];
      return v != null && !(typeof v === "string" && v.trim().length === 0);
    });
    // Agent 节点常输出 { summary, trends, ... }：摘要直接展示 summary 结论文本
    let preview: string | undefined;
    if (typeof obj.summary === "string" && obj.summary.trim().length > 0) {
      preview = obj.summary.trim();
    }
    return {
      kind,
      jsonText: JSON.stringify(payload, null, 2),
      fieldCount: keys.length,
      textPreview: preview,
    };
  }

  // 4. 纯文本
  const text = String(payload).trim();
  if (text.length === 0) { return empty; }
  return { kind, jsonText: text, textPreview: text };
}

/** 截断辅助 */
function truncateText(s: string, n: number): string {
  return s.length > n ? s.slice(0, n) + "…" : s;
}

export function SerenityScreeningPanel() {
  const { message: messageApi } = App.useApp();
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
  const location = useLocation();
  const [searchParams, setSearchParams] = useSearchParams();
  const isInInvestHub = location.pathname.startsWith("/invest");

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

  // 挂载时恢复上次运行候选的加载状态（避免 Empty 闪烁）
  const [lastRunLoading, setLastRunLoading] = useState(false);

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

  // ── 主题输入（对话式主题荐股 v47）──
  const [themeTags, setThemeTags] = useState<string[]>([]);

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

  // ── 挂载时恢复最近一次工作流运行产生的候选 ──
  // tab 打开（destroyOnHidden 下每次切换都会重新 mount）默认展示上一次
  // 趋势智选产物。styleFilter 同时认 'serenity'（serenity-screening 工作流
  // 落库 style='serenity'）和 'bottleneck'（智能荐股内置 SerenityStrategy
  // 落库 style='bottleneck'）——业务上两类都是"趋势智选"，让面板都能显示。
  useEffect(() => {
    let cancelled = false;
    (async () => {
      // 若工作流正在运行，不打扰运行态（completed 事件会覆盖结果）
      if (useSerenityStore.getState().running) { return; }
      setLastRunLoading(true);
      try {
        const list = await invoke<Array<{ generatedAt: string; stockCount: number; createdAt: string }>>(
          "list_reco_history",
          { styleFilter: "serenity,bottleneck", limit: 1 },
        );
        if (cancelled || !list || list.length === 0) { return; }
        const detail = await invoke<RecoDetailItem[]>("get_reco_detail", {
          generatedAt: list[0].generatedAt,
          styleFilter: "serenity,bottleneck",
        });
        if (cancelled) { return; }
        const restored = (detail ?? [])
          .map(restoreCandidate)
          .filter((c): c is SerenityCandidate => c != null);
        if (restored.length > 0) {
          setCandidates(restored);
        }
      } catch (e) {
        // 历史为空/查询失败不阻塞面板，保持默认空状态
        console.error("[Serenity] Failed to load last run candidates", e);
      } finally {
        if (!cancelled) { setLastRunLoading(false); }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [setCandidates]);

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
              "[Serenity] ⚠️ No candidates could be extracted! Full payload:",
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
      // Serenity 筛选涉及多个 LLM 调用，超时时间设为 30 分钟
      const SERENITY_TIMEOUT_MS = 30 * 60 * 1000;
      const r = await invoke<SerenityResult>(
        "run_serenity_screening",
        {
          asOfDate,
          themes: themeTags.length > 0 ? themeTags : null,
        },
        SERENITY_TIMEOUT_MS,
      );
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
        // 超时错误特殊处理：如果后端仍在运行，给用户友好提示
        if (err instanceof InvokeTimeoutError) {
          console.warn(
            `[Serenity] invoke timed out (${
              (err.timeoutMs / 1000).toFixed(0)
            }s), backend workflow may still be running...`,
          );
          setError(
            t("serenityPanel.timeoutHint", {
              seconds: (err.timeoutMs / 1000).toFixed(0),
            }),
          );
        } else {
          setError(err instanceof Error ? err.message : String(err));
        }
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
    themeTags,
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
        console.error("Failed to load bottleneck detail", e);
        setSerenityDetailItems([]);
      }
      setSerenityDetailLoading(false);
    },
    [],
  );

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
      {/* v47: 主题输入区（对话式主题荐股） */}
      <div className="flex items-center gap-2">
        <Text type="secondary" className="text-xs whitespace-nowrap">
          {t("serenityPanel.themeInput")}
        </Text>
        <Select
          mode="tags"
          style={{ flex: 1 }}
          placeholder={t("serenityPanel.themePlaceholder")}
          value={themeTags}
          onChange={setThemeTags as (val: string[]) => void}
          disabled={running}
          tokenSeparators={[",", "，"]}
          open={false}
        />
        {themeTags.length > 0 && (
          <Tag color="blue">
            {t("serenityPanel.sourceUser")}: {themeTags.join(", ")}
          </Tag>
        )}
      </div>

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
                // 同时认 'serenity'（serenity-screening 工作流）和 'bottleneck'
                // （智能荐股内置 SerenityStrategy）——业务上都是"趋势智选"。
                const list = await invoke<typeof serenityHistory>("list_reco_history", {
                  styleFilter: "serenity,bottleneck",
                  limit: 50,
                });
                console.log("[SerenityHistory] list_reco_history returned:", list?.length, list);
                setSerenityHistory(list ?? []);
                if (!list || list.length === 0) {
                  messageApi.warning(
                    t("serenityPanel.emptyHistoryWarning"),
                  );
                }
              } catch (e) {
                console.error("[SerenityHistory] list_reco_history call failed", e);
                messageApi.error(t("serenityPanel.backendCallFailed", { error: String(e) }));
              } finally {
                setSerenityHistoryLoading(false);
              }
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
                console.error("Feedback loop analysis failed", e);
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
              <Text type="secondary">{t("serenityPanel.filterPeUpper")}</Text>
              <InputNumber
                size="small"
                min={0}
                max={1000}
                value={serenityVars.max_pe ?? 100}
                onChange={(v) => handleSerenityVarChange("max_pe", v ?? 100)}
                className="w-24"
                suffix={t("serenityPanel.filterSuffixMultiplier")}
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">{t("serenityPanel.filterPbUpper")}</Text>
              <InputNumber
                size="small"
                min={0}
                max={100}
                value={serenityVars.max_pb ?? 10}
                onChange={(v) => handleSerenityVarChange("max_pb", v ?? 10)}
                className="w-24"
                suffix={t("serenityPanel.filterSuffixMultiplier")}
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">{t("serenityPanel.filter3mGainUpper")}</Text>
              <InputNumber
                size="small"
                min={0}
                max={500}
                value={serenityVars.max_3m_gain_pct ?? 30}
                onChange={(v) => handleSerenityVarChange("max_3m_gain_pct", v ?? 30)}
                className="w-24"
                suffix={t("serenityPanel.filterSuffixPercent")}
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">{t("serenityPanel.filter12mGainUpper")}</Text>
              <InputNumber
                size="small"
                min={0}
                max={500}
                value={serenityVars.max_12m_gain_pct ?? 100}
                onChange={(v) => handleSerenityVarChange("max_12m_gain_pct", v ?? 100)}
                className="w-24"
                suffix={t("serenityPanel.filterSuffixPercent")}
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">{t("serenityPanel.filterGrossMarginLower")}</Text>
              <InputNumber
                size="small"
                min={0}
                max={100}
                value={serenityVars.min_gross_margin ?? 25}
                onChange={(v) => handleSerenityVarChange("min_gross_margin", v ?? 25)}
                className="w-24"
                suffix={t("serenityPanel.filterSuffixPercent")}
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">{t("serenityPanel.filterDebtRatioUpper")}</Text>
              <InputNumber
                size="small"
                min={0}
                max={100}
                value={serenityVars.max_debt_ratio ?? 60}
                onChange={(v) => handleSerenityVarChange("max_debt_ratio", v ?? 60)}
                className="w-24"
                suffix={t("serenityPanel.filterSuffixPercent")}
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <Text type="secondary">{t("serenityPanel.filterGrowthExemptThreshold")}</Text>
              <InputNumber
                size="small"
                min={10}
                max={200}
                value={serenityVars.growth_exempt_pct ?? 50}
                onChange={(v) => handleSerenityVarChange("growth_exempt_pct", v ?? 50)}
                className="w-24"
                suffix={t("serenityPanel.filterSuffixPercent")}
              />
            </div>
            <div className="col-span-2 text-xs text-gray-400 mt-1">
              {t("serenityPanel.filterGrowthExemptHint")}
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
          <Space orientation="vertical" className="w-full" size={4}>
            {steps.map((s, i) => {
              const isExpanded = expandedSteps.has(i);
              // timeout 与 failed 同属失败类（此前 timeout 被渲染成蓝色 loading 图标）
              const isFailed = s.status === "failed" || s.status === "timeout";
              const statusColor = s.status === "completed"
                ? "green"
                : isFailed
                ? "red"
                : "blue";
              const statusIcon = s.status === "completed"
                ? <CheckCircleOutlined style={{ color: "#52c41a" }} />
                : isFailed
                ? <span style={{ color: "#ff4d4f" }}>✕</span>
                : <LoadingOutlined style={{ color: "#1677ff" }} />;
              // 节点输出语义化分析（ToolNode 表格 / CodeNode 计算 / AgentNode 文本）
              const view = buildNodeOutputView(s.nodeId, s.output);
              // 折叠态单行摘要：失败 → 错误信息；成功 → "类型 · 数据规模"
              let summary = "";
              if (isFailed) {
                summary = s.error ? truncateText(s.error, 60) : "";
              } else if (s.status === "completed" && view.kind !== "empty") {
                const typeLabel = t(`serenityPanel.stepLogType.${view.kind}`);
                // 摘要优先级：结论文本（summary）> 数组条数（空→"空数据"）> 字段数
                const detail = view.textPreview
                  ? truncateText(view.textPreview, 42)
                  : view.count != null
                  ? (view.count === 0
                    ? t("serenityPanel.stepLogCountEmpty")
                    : t("serenityPanel.stepLogCountSuffix", { count: view.count }))
                  : view.fieldCount != null
                  ? t("serenityPanel.stepLogFieldSuffix", { count: view.fieldCount })
                  : "";
                summary = detail ? `${typeLabel} · ${detail}` : typeLabel;
              }
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
                    <Text strong className="text-xs shrink-0">
                      {t(nodeTitleKey(s.nodeId))}
                    </Text>
                    <Tag color={statusColor} className="text-xs shrink-0">
                      {s.status}
                    </Tag>
                    {s.elapsedMs != null && (
                      <Text type="secondary" className="text-xs shrink-0">
                        {(s.elapsedMs / 1000).toFixed(1)}s
                      </Text>
                    )}
                    {summary && (
                      <Text
                        type={isFailed ? "danger" : "secondary"}
                        className="text-xs truncate flex-1 min-w-0"
                        title={summary}
                      >
                        {summary}
                      </Text>
                    )}
                    <div className="flex-1" />
                    <Text
                      type={summary ? "secondary" : undefined}
                      className="text-xs cursor-pointer shrink-0"
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
                    isFailed && s.error
                      ? <div className="mt-1 text-xs text-red-500 whitespace-pre-wrap break-all">{s.error}</div>
                      : view.kind === "empty"
                      ? (
                        <div className="mt-1 text-xs text-gray-400 italic">
                          {s.status === "completed"
                            ? t("serenityPanel.stepLogCompletedNoOutput")
                            : s.status === "running"
                            ? t("serenityPanel.stepLogRunning")
                            : t("serenityPanel.stepLogNoDetail")}
                        </div>
                      )
                      : view.table
                      ? (
                        <div
                          className="mt-1 max-h-64 overflow-auto rounded p-1"
                          style={{
                            backgroundColor: "rgba(255,255,255,0.04)",
                            border: "1px solid rgba(255,255,255,0.08)",
                          }}
                        >
                          <Table
                            size="small"
                            pagination={false}
                            rowKey={(_, idx) => String(idx ?? 0)}
                            scroll={{ x: "max-content" }}
                            dataSource={view.table.rows}
                            columns={view.table.columns.map((c) => ({
                              title: (() => {
                                const label = FIELD_LABEL_MAP[c] ?? c;
                                return label.startsWith("serenityPanel.") ? t(label) : label;
                              })(),
                              dataIndex: c,
                              key: c,
                              ellipsis: true,
                              render: (v: unknown) => (
                                <span title={typeof v === "string" ? v : undefined}>{cellText(v, t)}</span>
                              ),
                            }))}
                          />
                        </div>
                      )
                      : (
                        <pre
                          className="mt-1 max-h-48 overflow-auto rounded p-2 text-xs whitespace-pre-wrap break-all"
                          style={{
                            backgroundColor: "rgba(255,255,255,0.04)",
                            border: "1px solid rgba(255,255,255,0.08)",
                            color: "rgba(230,230,230,0.9)",
                          }}
                        >
                        {view.jsonText.length > 2000
                          ? view.jsonText.slice(0, 2000) + "..."
                          : view.jsonText}
                        </pre>
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
          <Space orientation="vertical" className="w-full">
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

      {/* 挂载恢复上次候选的加载态 */}
      {lastRunLoading && (
        <Card size="small" className="w-full">
          <div className="py-6 flex items-center justify-center gap-2 text-sm text-gray-400">
            <Spin size="small" />
            <span>{t("common.loading")}</span>
          </div>
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
                    const msg = t("serenityPanel.exitSignalAlert", {
                      checked: r.checked_count,
                      exitNow: r.exit_now_count,
                      caution: r.caution_count,
                    });
                    alert(msg);
                  } else {
                    alert(t("serenityPanel.exitSignalAlertNone", { checked: r.checked_count }));
                  }
                } catch (e) {
                  console.error("Failed to refresh exit signal", e);
                }
              }}
            >
              {t("serenityPanel.refreshExitButton")}
            </Button>
          </div>
          {candidates.map((c, i) => {
            const code = c.stock_code ?? c.stockCode ?? "";
            return (
              <SerenityCandidateCard
                key={`${code}-${i}`}
                candidate={c}
              />
            );
          })}
        </div>
      )}

      {/* 空状态 / 解释无候选原因 */}
      {!running && !error && !lastRunLoading && candidates.length === 0 && trends.length === 0 && (
        emptyReason
          ? (
            <Alert
              type="info"
              showIcon
              title={t("serenityPanel.noCandidateTitle")}
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
      {!running && !error && !lastRunLoading && candidates.length === 0 && trends.length > 0 && emptyReason && (
        <Alert
          type="info"
          showIcon
          title={t("serenityPanel.noCandidateTitle")}
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
                        console.error("Delete failed", e);
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
                    messageApi.success(
                      t("serenityPanel.serenityHistory.deleteSuccess", { count: serenitySelected.length }),
                    );
                    setSerenityHistory((prev) => prev.filter((r) => !serenitySelected.includes(r.generatedAt)));
                    setSerenitySelected([]);
                  } catch (e) {
                    messageApi.error(String(e));
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
              render: (v: number) => <span className="text-xs">{v}{t("serenityPanel.filterSuffixCount")}</span>,
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
                {t("serenityPanel.serenityHistory.candidateCount")}: {serenityDetailItems.length}
                {t("serenityPanel.filterSuffixCount")}
              </div>
              {serenityDetailItems.map((item, i) => (
                <Card
                  key={`${item.stockCode}-${i}`}
                  size="small"
                  hoverable
                  className="w-full"
                  onClick={() => {
                    setSerenityDetailOpen(false);
                    if (isInInvestHub) {
                      // 在 InvestHub 内部：使用 URL 参数切换到 workspace tab，自动输入股票代码
                      const next = new URLSearchParams(searchParams);
                      next.set("tab", "workspace");
                      next.set("stockCode", item.stockCode);
                      next.set("view", "analysis");
                      setSearchParams(next, { replace: true });
                    } else {
                      // 独立页面：跳转到股票分析页面
                      navigate(`/stock-analysis?code=${item.stockCode}`, { replace: true });
                    }
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
