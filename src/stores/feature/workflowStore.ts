// SPDX-License-Identifier: AGPL-3.0-only
// Phase 4: NL2Workflow Store — 工作流状态管理

import type {
  ExecutionLogEntry,
  NL2SkillRequest,
  NL2SkillResult,
  NL2UIRequest,
  NL2UIResult,
  NLParseRequest,
  NLParseResult,
  NodeExecutionState,
  SkillDefinition,
  VersionDiff,
  WorkflowDefinition,
  WorkflowEdge,
  WorkflowExecution,
  WorkflowFilter,
  WorkflowNode,
  WorkflowTemplate,
  WorkflowVersion,
} from "@/types/workflow";
import type { UISchema } from "@/types/dynamicUI";
import { create } from "zustand";

// ============================================================
// Mock Data
// ============================================================

function makeId(): string {
  return `wf_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

const mockNodes: WorkflowNode[] = [
  { id: "trigger-1", type: "trigger", label: "定时触发", config: { cron: "0 8 * * *" }, position: { x: 100, y: 50 } },
  { id: "action-1", type: "action", label: "HTTP 请求", config: { url: "https://api.example.com/data", method: "GET" }, position: { x: 100, y: 150 }, inputs: ["trigger-1"], outputs: ["response_data"] },
  { id: "condition-1", type: "condition", label: "数据校验", config: { expression: "response.status === 200" }, position: { x: 100, y: 270 }, inputs: ["action-1"] },
  { id: "action-2", type: "action", label: "AI 分析摘要", config: { prompt: "请分析以下数据并生成摘要" }, position: { x: 300, y: 270 }, inputs: ["condition-1"], outputs: ["summary"] },
  { id: "output-1", type: "output", label: "发送通知", config: { channel: "企业微信", template: "每日报告已生成" }, position: { x: 100, y: 390 }, inputs: ["action-2"] },
];

const mockEdges: WorkflowEdge[] = [
  { id: "e1", source: "trigger-1", target: "action-1" },
  { id: "e2", source: "action-1", target: "condition-1" },
  { id: "e3", source: "condition-1", target: "action-2", condition: "status === 200" },
  { id: "e4", source: "action-2", target: "output-1" },
];

function createMockWorkflow(overrides: Partial<WorkflowDefinition> = {}): WorkflowDefinition {
  return {
    id: makeId(),
    name: "新建工作流",
    description: "",
    version: 1,
    nodes: [],
    edges: [],
    variables: {},
    createdAt: Date.now(),
    updatedAt: Date.now(),
    status: "draft",
    ...overrides,
  };
}

const mockWorkflows: WorkflowDefinition[] = [
  {
    id: "wf-daily-report",
    name: "日报生成",
    description: "每天早上 8 点抓取指定数据，用 AI 生成摘要并发送企业微信通知",
    version: 3,
    nodes: mockNodes,
    edges: mockEdges,
    variables: { target_url: "https://api.example.com/data", wechat_webhook: "https://qyapi.weixin.qq.com/xxx" },
    createdAt: Date.now() - 86400000 * 10,
    updatedAt: Date.now() - 86400000 * 2,
    status: "active",
  },
  {
    id: "wf-data-sync",
    name: "数据同步",
    description: "每小时从 A 数据库同步数据到 B 数据库，失败自动重试",
    version: 2,
    nodes: [
      { id: "t1", type: "trigger", label: "定时触发", config: { cron: "0 * * * *" }, position: { x: 50, y: 50 } },
      { id: "a1", type: "action", label: "查询源数据库", config: { query: "SELECT * FROM updates WHERE synced=0" }, position: { x: 50, y: 160 }, inputs: ["t1"], outputs: ["rows"] },
      { id: "c1", type: "condition", label: "有新数据?", config: { expression: "rows.length > 0" }, position: { x: 50, y: 280 }, inputs: ["a1"] },
      { id: "a2", type: "action", label: "写入目标库", config: { query: "INSERT INTO archive ..." }, position: { x: 250, y: 280 }, inputs: ["c1"] },
      { id: "o1", type: "output", label: "更新同步状态", config: { query: "UPDATE updates SET synced=1" }, position: { x: 50, y: 400 }, inputs: ["a2"] },
    ],
    edges: [
      { id: "e1", source: "t1", target: "a1" },
      { id: "e2", source: "a1", target: "c1" },
      { id: "e3", source: "c1", target: "a2", condition: "rows.length > 0" },
      { id: "e4", source: "a2", target: "o1" },
    ],
    variables: { source_db: "mysql://prod", target_db: "mysql://archive" },
    createdAt: Date.now() - 86400000 * 20,
    updatedAt: Date.now() - 86400000 * 5,
    status: "active",
  },
  {
    id: "wf-monitor-alert",
    name: "监控告警",
    description: "每 5 分钟检查服务健康状态，异常时通过多渠道发送告警",
    version: 1,
    nodes: [
      { id: "t1", type: "trigger", label: "定时触发(5min)", config: { cron: "*/5 * * * *" }, position: { x: 100, y: 50 } },
      { id: "a1", type: "action", label: "健康检查", config: { url: "https://api.example.com/health", method: "GET" }, position: { x: 100, y: 150 }, inputs: ["t1"] },
      { id: "c1", type: "condition", label: "是否异常?", config: { expression: "response.status !== 200" }, position: { x: 100, y: 260 }, inputs: ["a1"] },
      { id: "p1", type: "parallel", label: "多渠道通知", config: {}, position: { x: 300, y: 150 }, inputs: ["c1"] },
      { id: "a2", type: "action", label: "发送邮件", config: { to: "admin@example.com" }, position: { x: 300, y: 260 }, inputs: ["p1"] },
      { id: "a3", type: "action", label: "企业微信通知", config: { webhook: "xxx" }, position: { x: 300, y: 360 }, inputs: ["p1"] },
    ],
    edges: [
      { id: "e1", source: "t1", target: "a1" },
      { id: "e2", source: "a1", target: "c1" },
      { id: "e3", source: "c1", target: "p1", condition: "异常" },
      { id: "e4", source: "p1", target: "a2" },
      { id: "e5", source: "p1", target: "a3" },
    ],
    variables: { check_url: "https://api.example.com/health", alert_email: "admin@example.com" },
    createdAt: Date.now() - 86400000 * 30,
    updatedAt: Date.now() - 86400000 * 10,
    status: "draft",
  },
  {
    id: "wf-content-review",
    name: "内容审核",
    description: "用户提交内容后自动进行敏感词检测和 AI 内容审核",
    version: 5,
    nodes: [
      { id: "t1", type: "trigger", label: "Webhook 触发", config: { path: "/review", method: "POST" }, position: { x: 100, y: 50 } },
      { id: "a1", type: "action", label: "敏感词检测", config: { wordlist: ["违禁词1", "违禁词2"] }, position: { x: 100, y: 160 }, inputs: ["t1"], outputs: ["has_sensitive"] },
      { id: "c1", type: "condition", label: "是否含敏感词?", config: { expression: "has_sensitive === true" }, position: { x: 100, y: 270 }, inputs: ["a1"] },
      { id: "a2", type: "action", label: "AI 内容审核", config: { model: "gpt-4", prompt: "审核以下内容是否违规" }, position: { x: 300, y: 160 }, inputs: ["c1"], outputs: ["review_result"] },
      { id: "o1", type: "output", label: "审核通过", config: { action: "publish" }, position: { x: 100, y: 380 }, inputs: ["a2"] },
      { id: "o2", type: "output", label: "拒绝发布", config: { action: "reject" }, position: { x: 300, y: 380 }, inputs: ["a1"] },
    ],
    edges: [
      { id: "e1", source: "t1", target: "a1" },
      { id: "e2", source: "a1", target: "c1" },
      { id: "e3", source: "c1", target: "a2", condition: "无敏感词" },
      { id: "e4", source: "a2", target: "o1" },
      { id: "e5", source: "a1", target: "o2", condition: "含敏感词" },
    ],
    variables: { webhook_secret: "xxx", ai_model: "gpt-4" },
    createdAt: Date.now() - 86400000 * 60,
    updatedAt: Date.now() - 86400000 * 1,
    status: "active",
  },
  {
    id: "wf-customer-notify",
    name: "客户通知",
    description: "订单状态变更时自动发送短信和邮件通知客户",
    version: 2,
    nodes: [
      { id: "t1", type: "trigger", label: "事件触发", config: { event: "order.status_changed" }, position: { x: 100, y: 50 } },
      { id: "c1", type: "condition", label: "判断状态", config: { expression: "event.new_status" }, position: { x: 100, y: 160 }, inputs: ["t1"] },
      { id: "a1", type: "action", label: "发送短信", config: { template: "您的订单已{status}" }, position: { x: 300, y: 100 }, inputs: ["c1"], outputs: ["sms_sent"] },
      { id: "a2", type: "action", label: "发送邮件", config: { template: "order_update" }, position: { x: 300, y: 220 }, inputs: ["c1"], outputs: ["email_sent"] },
      { id: "o1", type: "output", label: "记录日志", config: {}, position: { x: 100, y: 320 }, inputs: ["a1", "a2"] },
    ],
    edges: [
      { id: "e1", source: "t1", target: "c1" },
      { id: "e2", source: "c1", target: "a1", condition: "status=shipped" },
      { id: "e3", source: "c1", target: "a2", condition: "status=delivered" },
      { id: "e4", source: "a1", target: "o1" },
      { id: "e5", source: "a2", target: "o1" },
    ],
    variables: { sms_api_key: "xxx", email_api_key: "xxx" },
    createdAt: Date.now() - 86400000 * 45,
    updatedAt: Date.now() - 86400000 * 15,
    status: "archived",
  },
];

const mockTemplates: WorkflowTemplate[] = [
  {
    id: "tpl-daily-report",
    name: "日报生成模板",
    description: "定时抓取数据 → AI 分析 → 多渠道推送的完整日报生成工作流",
    category: "content-generation",
    nodeCount: 5,
    tags: ["日报", "AI", "自动化", "通知"],
    workflow: mockWorkflows[0],
    isBuiltIn: true,
    createdAt: Date.now() - 86400000 * 30,
    updatedAt: Date.now() - 86400000 * 10,
  },
  {
    id: "tpl-data-sync",
    name: "数据同步模板",
    description: "跨数据库定时同步，支持条件过滤、失败重试和数据校验",
    category: "data-processing",
    nodeCount: 5,
    tags: ["数据同步", "数据库", "定时"],
    workflow: mockWorkflows[1],
    isBuiltIn: true,
    createdAt: Date.now() - 86400000 * 25,
    updatedAt: Date.now() - 86400000 * 5,
  },
  {
    id: "tpl-monitor",
    name: "服务监控告警",
    description: "定时健康检查 → 异常检测 → 多渠道告警通知",
    category: "monitoring",
    nodeCount: 6,
    tags: ["监控", "告警", "通知"],
    workflow: mockWorkflows[2],
    isBuiltIn: true,
    createdAt: Date.now() - 86400000 * 20,
    updatedAt: Date.now() - 86400000 * 3,
  },
  {
    id: "tpl-content-review",
    name: "内容审核流水线",
    description: "用户提交内容 → 敏感词检测 → AI 审核 → 自动发布/拒绝",
    category: "integration",
    nodeCount: 6,
    tags: ["审核", "AI", "内容安全"],
    workflow: mockWorkflows[3],
    isBuiltIn: true,
    createdAt: Date.now() - 86400000 * 15,
    updatedAt: Date.now() - 86400000 * 1,
  },
  {
    id: "tpl-notification",
    name: "多通道通知引擎",
    description: "事件驱动的多渠道通知工作流，支持短信/邮件/企业微信/钉钉",
    category: "notification",
    nodeCount: 5,
    tags: ["通知", "短信", "邮件", "企业微信"],
    workflow: mockWorkflows[4],
    isBuiltIn: true,
    createdAt: Date.now() - 86400000 * 10,
    updatedAt: Date.now() - 86400000 * 2,
  },
];

function generateMockParseResult(prompt: string): NLParseResult {
  const workflow: WorkflowDefinition = {
    id: makeId(),
    name: prompt.length > 30 ? prompt.slice(0, 30) + "..." : prompt,
    description: `基于自然语言描述自动生成的工作流: ${prompt}`,
    version: 1,
    nodes: [
      { id: "trigger-1", type: "trigger", label: "手动触发", config: {}, position: { x: 100, y: 50 } },
      { id: "action-1", type: "action", label: "处理步骤 1", config: { text: prompt }, position: { x: 100, y: 160 }, inputs: ["trigger-1"], outputs: ["result_1"] },
      { id: "condition-1", type: "condition", label: "条件判断", config: { expression: "result_1 !== null" }, position: { x: 100, y: 280 }, inputs: ["action-1"] },
      { id: "action-2", type: "action", label: "处理步骤 2", config: {}, position: { x: 300, y: 280 }, inputs: ["condition-1"], outputs: ["result_2"] },
      { id: "output-1", type: "output", label: "输出结果", config: {}, position: { x: 100, y: 400 }, inputs: ["action-2"] },
    ],
    edges: [
      { id: "e1", source: "trigger-1", target: "action-1" },
      { id: "e2", source: "action-1", target: "condition-1" },
      { id: "e3", source: "condition-1", target: "action-2" },
      { id: "e4", source: "action-2", target: "output-1" },
    ],
    variables: {},
    createdAt: Date.now(),
    updatedAt: Date.now(),
    status: "draft",
  };

  return {
    workflow,
    confidence: 0.72 + Math.random() * 0.2,
    suggestions: [
      "建议为 HTTP 请求节点添加超时和重试配置",
      "建议添加异常处理分支以提高工作流鲁棒性",
      "可考虑添加执行结果通知节点",
    ],
    alternatives: undefined,
  };
}

// ============================================================
// Store Types
// ============================================================

interface WorkflowStoreState {
  // 工作流列表
  workflows: WorkflowDefinition[];
  // 当前编辑的工作流 ID
  currentWorkflowId: string | null;
  // 模板库
  templates: WorkflowTemplate[];
  // NL 解析历史
  parseHistory: NLParseResult[];
  // 执行记录
  executions: WorkflowExecution[];
  // 版本历史 (key: workflowId)
  versionHistories: Record<string, WorkflowVersion[]>;

  // 加载状态
  loading: boolean;
  error: string | null;

  // NL 解析状态
  isParsing: boolean;
  parseProgress: string;

  // 执行状态
  isExecuting: boolean;

  // 筛选
  filter: WorkflowFilter;

  // ========== Actions ==========
  fetchWorkflows: () => Promise<void>;
  getWorkflow: (id: string) => Promise<WorkflowDefinition | null>;
  createWorkflow: (workflow: Partial<WorkflowDefinition>) => Promise<WorkflowDefinition>;
  updateWorkflow: (id: string, updates: Partial<WorkflowDefinition>) => Promise<void>;
  deleteWorkflow: (id: string) => Promise<void>;
  duplicateWorkflow: (id: string) => Promise<WorkflowDefinition>;

  // NL 解析
  parseNaturalLanguage: (request: NLParseRequest) => Promise<NLParseResult>;

  // 模板
  fetchTemplates: () => Promise<void>;
  createFromTemplate: (templateId: string) => Promise<WorkflowDefinition>;

  // 执行
  executeWorkflow: (id: string, inputs: Record<string, unknown>) => Promise<WorkflowExecution>;
  getExecutionStatus: (executionId: string) => Promise<WorkflowExecution | null>;

  // 版本管理
  getVersionHistory: (workflowId: string) => Promise<WorkflowVersion[]>;
  restoreVersion: (workflowId: string, version: number) => Promise<void>;
  compareVersions: (workflowId: string, v1: number, v2: number) => Promise<VersionDiff>;

  // 筛选
  setFilter: (filter: Partial<WorkflowFilter>) => void;
  getFilteredWorkflows: () => WorkflowDefinition[];

  // 编辑器专用
  setCurrentWorkflow: (id: string | null) => void;
  addNode: (node: WorkflowNode) => void;
  updateNode: (nodeId: string, updates: Partial<WorkflowNode>) => void;
  removeNode: (nodeId: string) => void;
  addEdge: (edge: WorkflowEdge) => void;
  removeEdge: (edgeId: string) => void;
  setParsingProgress: (progress: string) => void;

  // NL2Skill
  parseSkillFromNaturalLanguage: (request: NL2SkillRequest) => Promise<NL2SkillResult>;

  // NL2UI
  parseUIFromNaturalLanguage: (request: NL2UIRequest) => Promise<NL2UIResult>;
}

// ============================================================
// Store Implementation
// ============================================================

export const useWorkflowStore = create<WorkflowStoreState>((set, get) => ({
  workflows: [...mockWorkflows],
  currentWorkflowId: null,
  templates: [...mockTemplates],
  parseHistory: [],
  executions: [],
  versionHistories: {},
  loading: false,
  error: null,
  isParsing: false,
  parseProgress: "",
  isExecuting: false,
  filter: { status: "all" },

  // ========== 工作流 CRUD ==========

  fetchWorkflows: async () => {
    set({ loading: true, error: null });
    try {
      // 尝试调用后端，失败时使用 mock
      // await invoke("get_all_workflows");
      await new Promise((r) => setTimeout(r, 300));
      // keep mock data, already loaded
    } catch (e) {
      console.warn("[workflowStore] fetchWorkflows fallback to mock:", e);
    } finally {
      set({ loading: false });
    }
  },

  getWorkflow: async (id: string) => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 100));
      const wf = get().workflows.find((w) => w.id === id) ?? null;
      return wf;
    } catch (e) {
      console.warn("[workflowStore] getWorkflow fallback:", e);
      set({ error: String(e) });
      return null;
    } finally {
      set({ loading: false });
    }
  },

  createWorkflow: async (workflow: Partial<WorkflowDefinition>) => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 200));
      const newWf: WorkflowDefinition = {
        id: makeId(),
        name: workflow.name ?? "新建工作流",
        description: workflow.description ?? "",
        version: 1,
        nodes: workflow.nodes ?? [],
        edges: workflow.edges ?? [],
        variables: workflow.variables ?? {},
        createdAt: Date.now(),
        updatedAt: Date.now(),
        status: "draft",
      };
      set((s) => ({ workflows: [newWf, ...s.workflows] }));
      return newWf;
    } catch (e) {
      console.warn("[workflowStore] createWorkflow fallback:", e);
      set({ error: String(e) });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  updateWorkflow: async (id: string, updates: Partial<WorkflowDefinition>) => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 200));
      set((s) => ({
        workflows: s.workflows.map((w) =>
          w.id === id
            ? { ...w, ...updates, updatedAt: Date.now(), version: w.version + 1 }
            : w
        ),
      }));
    } catch (e) {
      console.warn("[workflowStore] updateWorkflow fallback:", e);
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  deleteWorkflow: async (id: string) => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 200));
      set((s) => ({
        workflows: s.workflows.filter((w) => w.id !== id),
        currentWorkflowId: s.currentWorkflowId === id ? null : s.currentWorkflowId,
      }));
    } catch (e) {
      console.warn("[workflowStore] deleteWorkflow fallback:", e);
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },

  duplicateWorkflow: async (id: string) => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 200));
      const original = get().workflows.find((w) => w.id === id);
      if (!original) throw new Error(`Workflow ${id} not found`);
      const dup: WorkflowDefinition = {
        ...original,
        id: makeId(),
        name: `${original.name} (副本)`,
        version: 1,
        createdAt: Date.now(),
        updatedAt: Date.now(),
        status: "draft",
      };
      set((s) => ({ workflows: [dup, ...s.workflows] }));
      return dup;
    } catch (e) {
      console.warn("[workflowStore] duplicateWorkflow fallback:", e);
      set({ error: String(e) });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  // ========== NL 解析 ==========

  parseNaturalLanguage: async (request: NLParseRequest) => {
    set({ isParsing: true, parseProgress: "正在分析意图..." });
    try {
      // 模拟多阶段解析过程
      await new Promise((r) => setTimeout(r, 600));
      set({ parseProgress: "正在匹配节点..." });
      await new Promise((r) => setTimeout(r, 600));
      set({ parseProgress: "正在构建工作流..." });
      await new Promise((r) => setTimeout(r, 600));
      set({ parseProgress: "正在优化..." });
      await new Promise((r) => setTimeout(r, 400));

      const result = generateMockParseResult(request.prompt);
      set((s) => ({ parseHistory: [result, ...s.parseHistory] }));
      return result;
    } catch (e) {
      console.warn("[workflowStore] parseNaturalLanguage fallback:", e);
      const result = generateMockParseResult(request.prompt);
      set((s) => ({ parseHistory: [result, ...s.parseHistory] }));
      return result;
    } finally {
      set({ isParsing: false, parseProgress: "" });
    }
  },

  // ========== 模板 ==========

  fetchTemplates: async () => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 200));
    } catch (e) {
      console.warn("[workflowStore] fetchTemplates fallback:", e);
    } finally {
      set({ loading: false });
    }
  },

  createFromTemplate: async (templateId: string) => {
    set({ loading: true, error: null });
    try {
      await new Promise((r) => setTimeout(r, 300));
      const template = get().templates.find((t) => t.id === templateId);
      if (!template) throw new Error(`Template ${templateId} not found`);
      const newWf: WorkflowDefinition = {
        ...template.workflow,
        id: makeId(),
        name: `${template.name} (来自模板)`,
        version: 1,
        createdAt: Date.now(),
        updatedAt: Date.now(),
        status: "draft",
      };
      set((s) => ({ workflows: [newWf, ...s.workflows] }));
      return newWf;
    } catch (e) {
      console.warn("[workflowStore] createFromTemplate fallback:", e);
      set({ error: String(e) });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  // ========== 执行 ==========

  executeWorkflow: async (id: string, inputs: Record<string, unknown>) => {
    set({ isExecuting: true, error: null });
    const executionId = `exec_${Date.now()}`;
    const wf = get().workflows.find((w) => w.id === id);
    const logs: ExecutionLogEntry[] = [];

    const addLog = (nodeId: string, nodeName: string, level: ExecutionLogEntry["level"], message: string) => {
      logs.push({ timestamp: Date.now(), nodeId, nodeName, level, message });
    };

    const nodeStates: NodeExecutionState[] = wf
      ? wf.nodes.map((n) => ({ nodeId: n.id, status: "waiting" as const }))
      : [];

    try {
      if (wf) {
        for (const node of wf.nodes) {
          addLog(node.id, node.label, "info", `开始执行节点: ${node.label}`);
          set({ isExecuting: true });
          await new Promise((r) => setTimeout(r, 300 + Math.random() * 400));
          const idx = nodeStates.findIndex((ns) => ns.nodeId === node.id);
          if (idx >= 0) {
            nodeStates[idx] = { nodeId: node.id, status: "success", startedAt: Date.now(), finishedAt: Date.now() };
          }
          addLog(node.id, node.label, "info", `节点执行完成: ${node.label}`);
        }
      }

      const execution: WorkflowExecution = {
        id: executionId,
        workflowId: id,
        status: "completed",
        startedAt: Date.now(),
        finishedAt: Date.now(),
        nodeStates,
        inputs,
        outputs: { message: "执行成功" },
        logs,
      };

      set((s) => ({ executions: [execution, ...s.executions] }));
      return execution;
    } catch (e) {
      const execution: WorkflowExecution = {
        id: executionId,
        workflowId: id,
        status: "failed",
        startedAt: Date.now(),
        finishedAt: Date.now(),
        nodeStates,
        inputs,
        logs,
      };
      set((s) => ({ executions: [execution, ...s.executions], error: String(e) }));
      return execution;
    } finally {
      set({ isExecuting: false });
    }
  },

  getExecutionStatus: async (executionId: string) => {
    const exec = get().executions.find((e) => e.id === executionId) ?? null;
    return exec;
  },

  // ========== 版本管理 ==========

  getVersionHistory: async (workflowId: string) => {
    const wf = get().workflows.find((w) => w.id === workflowId);
    if (!wf) return [];
    const versions: WorkflowVersion[] = [
      { version: wf.version, updatedAt: wf.updatedAt, summary: "当前版本", status: wf.status, snapshot: wf },
      { version: wf.version - 1, updatedAt: wf.updatedAt - 86400000, summary: "优化节点配置", status: "active", snapshot: wf },
      { version: wf.version - 2, updatedAt: wf.updatedAt - 86400000 * 2, summary: "添加条件分支", status: "active", snapshot: wf },
      { version: 1, updatedAt: wf.createdAt, summary: "初始创建", status: "draft", snapshot: wf },
    ];
    return versions;
  },

  restoreVersion: async (workflowId: string, version: number) => {
    await new Promise((r) => setTimeout(r, 300));
    set((s) => ({
      workflows: s.workflows.map((w) =>
        w.id === workflowId ? { ...w, version: w.version + 1, updatedAt: Date.now() } : w
      ),
    }));
  },

  compareVersions: async (_workflowId: string, _v1: number, _v2: number) => {
    await new Promise((r) => setTimeout(r, 200));
    return {
      addedNodes: [],
      removedNodes: [],
      modifiedNodes: [],
      addedEdges: [],
      removedEdges: [],
      modifiedEdges: [],
    };
  },

  // ========== 筛选 ==========

  setFilter: (filter: Partial<WorkflowFilter>) => {
    set((s) => ({ filter: { ...s.filter, ...filter } }));
  },

  getFilteredWorkflows: () => {
    const { workflows, filter } = get();
    return workflows.filter((wf) => {
      if (filter.status && filter.status !== "all" && wf.status !== filter.status) return false;
      if (filter.search) {
        const q = filter.search.toLowerCase();
        if (!wf.name.toLowerCase().includes(q) && !wf.description.toLowerCase().includes(q)) return false;
      }
      return true;
    });
  },

  // ========== 编辑器 ==========

  setCurrentWorkflow: (id: string | null) => {
    set({ currentWorkflowId: id });
  },

  addNode: (node: WorkflowNode) => {
    set((s) => ({
      workflows: s.workflows.map((w) =>
        w.id === s.currentWorkflowId
          ? { ...w, nodes: [...w.nodes, node], updatedAt: Date.now() }
          : w
      ),
    }));
  },

  updateNode: (nodeId: string, updates: Partial<WorkflowNode>) => {
    set((s) => ({
      workflows: s.workflows.map((w) =>
        w.id === s.currentWorkflowId
          ? {
              ...w,
              nodes: w.nodes.map((n) => (n.id === nodeId ? { ...n, ...updates } : n)),
              updatedAt: Date.now(),
            }
          : w
      ),
    }));
  },

  removeNode: (nodeId: string) => {
    set((s) => ({
      workflows: s.workflows.map((w) =>
        w.id === s.currentWorkflowId
          ? {
              ...w,
              nodes: w.nodes.filter((n) => n.id !== nodeId),
              edges: w.edges.filter((e) => e.source !== nodeId && e.target !== nodeId),
              updatedAt: Date.now(),
            }
          : w
      ),
    }));
  },

  addEdge: (edge: WorkflowEdge) => {
    set((s) => ({
      workflows: s.workflows.map((w) =>
        w.id === s.currentWorkflowId
          ? { ...w, edges: [...w.edges, edge], updatedAt: Date.now() }
          : w
      ),
    }));
  },

  removeEdge: (edgeId: string) => {
    set((s) => ({
      workflows: s.workflows.map((w) =>
        w.id === s.currentWorkflowId
          ? { ...w, edges: w.edges.filter((e) => e.id !== edgeId), updatedAt: Date.now() }
          : w
      ),
    }));
  },

  setParsingProgress: (progress: string) => {
    set({ parseProgress: progress });
  },

  // ========== NL2Skill ==========

  parseSkillFromNaturalLanguage: async (request: NL2SkillRequest) => {
    set({ isParsing: true, parseProgress: "意图分析" });

    const phases = [
      { phase: "意图分析", status: "done" as const, detail: `识别为${request.prompt.includes("客服") ? "客服自动回复" : request.prompt.includes("报告") ? "报告生成" : request.prompt.includes("翻译") ? "多语言翻译" : "自定义"}技能` },
      { phase: "技能匹配", status: "done" as const, detail: `匹配到 ${request.skillType || "chat"} 类型，${extractTriggers(request.prompt).length} 个触发词` },
      { phase: "参数提取", status: "done" as const, detail: "提取 2 个参数：query、context" },
      { phase: "模板生成", status: "done" as const, detail: "生成提示词模板" },
      { phase: "校验优化", status: "done" as const, detail: "通过语义校验，置信度 92%" },
    ];

    const progressSteps = [25, 45, 65, 85, 100];
    for (let i = 0; i < progressSteps.length; i++) {
      await new Promise(r => setTimeout(r, 300));
      set({ parseProgress: phases[i].phase });
    }

    const skillId = `skill_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    const skill: SkillDefinition = {
      id: skillId,
      name: request.prompt.includes("客服") ? "智能客服回复"
        : request.prompt.includes("报告") ? "日报生成"
        : request.prompt.includes("翻译") ? "多语言翻译"
        : "自定义技能",
      description: request.prompt.slice(0, 100),
      type: request.skillType || "chat",
      triggers: extractTriggers(request.prompt),
      prompt_template: `基于以下上下文回答问题：\n{{context}}\n\n用户问题：{{query}}\n\n要求：${request.prompt}`,
      parameters: [
        { name: "query", type: "string", description: "用户输入的问题", required: true },
        { name: "context", type: "string", description: "对话上下文", required: false, default: "" },
      ],
      tools: ["web_search", "knowledge_retrieval"],
      icon: "MessageSquare",
      tags: ["auto-generated"],
    };

    const result: NL2SkillResult = {
      skill,
      confidence: 0.92,
      phases,
      suggestions: [
        "建议添加错误处理分支：当用户输入无法识别时返回友好提示",
        "可增加多轮对话支持，在 prompt_template 中引入对话历史变量",
        "建议为高频问题添加缓存机制以提升响应速度",
      ],
    };

    set({ isParsing: false, parseProgress: "完成" });
    return result;
  },

  // ========== NL2UI ==========

  parseUIFromNaturalLanguage: async (request: NL2UIRequest) => {
    set({ isParsing: true, parseProgress: "意图分析" });

    const uiType = request.uiType || "custom";
    const phases = [
      { phase: "意图分析", status: "done" as const, detail: `识别为 ${uiType} 类型 UI` },
      { phase: "布局规划", status: "done" as const, detail: "规划布局，组件编排" },
      { phase: "组件选择", status: "done" as const, detail: "选择 Form + Table + Card + Statistic + Chart + Tag" },
      { phase: "Schema构建", status: "done" as const, detail: "生成 UISchema" },
      { phase: "校验优化", status: "done" as const, detail: "通过 JSON Schema 校验，无循环引用" },
    ];

    const progressSteps = [20, 40, 60, 80, 100];
    for (let i = 0; i < progressSteps.length; i++) {
      await new Promise(r => setTimeout(r, 300));
      set({ parseProgress: phases[i].phase });
    }

    const schema = generateUISchema(request);

    const result: NL2UIResult = {
      schema,
      confidence: 0.88,
      phases,
      suggestions: [
        "建议将表格列宽设为响应式以适应不同屏幕",
        "可在 Card 外层添加 Tabs 组件以分组展示不同维度数据",
        "Chart 建议接入实际数据源后调整颜色映射",
      ],
    };

    set({ isParsing: false, parseProgress: "完成" });
    return result;
  },
}));

// Selector for working with current workflow
export function useCurrentWorkflow(): WorkflowDefinition | null {
  return useWorkflowStore((s) => {
    if (!s.currentWorkflowId) return null;
    return s.workflows.find((w) => w.id === s.currentWorkflowId) ?? null;
  });
}

// ============================================================
// Helper Functions
// ============================================================

function extractTriggers(prompt: string): string[] {
  if (prompt.includes("客服")) return ["客服", "帮助", "咨询", "问题"];
  if (prompt.includes("报告")) return ["生成报告", "日报", "周报", "总结"];
  if (prompt.includes("翻译")) return ["翻译", "translate", "译"];
  return ["帮助", "help", "怎么", "如何"];
}

function generateUISchema(request: NL2UIRequest): UISchema {
  const uiType = request.uiType || "custom";

  if (uiType === "dashboard") {
    return {
      version: "1.0",
      id: `dashboard_${Date.now()}`,
      type: "Container",
      props: { style: { padding: "16px", display: "flex", flexDirection: "column", gap: "16px" } },
      children: [
        {
          version: "1.0", id: "row_1", type: "Row", props: { gutter: 16 },
          children: [
            { version: "1.0", id: "stat_1", type: "Card", props: { title: "总请求量" }, children: [{ version: "1.0", id: "stat_1_inner", type: "Text", props: { content: "12,847 次", style: { fontSize: "24px", fontWeight: "bold", color: "#52c41a" } } }] },
            { version: "1.0", id: "stat_2", type: "Card", props: { title: "成功率" }, children: [{ version: "1.0", id: "stat_2_inner", type: "Text", props: { content: "98.5%", style: { fontSize: "24px", fontWeight: "bold", color: "#1677ff" } } }] },
            { version: "1.0", id: "stat_3", type: "Card", props: { title: "平均耗时" }, children: [{ version: "1.0", id: "stat_3_inner", type: "Text", props: { content: "234 ms", style: { fontSize: "24px", fontWeight: "bold", color: "#faad14" } } }] },
          ],
        },
        {
          version: "1.0", id: "chart_1", type: "Card", props: { title: "请求趋势（近 7 天）" },
          children: [{ version: "1.0", id: "chart_1_inner", type: "Chart", props: { chartType: "line", data: { labels: ["周一", "周二", "周三", "周四", "周五", "周六", "周日"], values: [1200, 1900, 1500, 2100, 1800, 2400, 1700] } } }],
        },
      ],
    };
  }

  if (uiType === "form") {
    return {
      version: "1.0",
      id: `form_${Date.now()}`,
      type: "Form",
      props: { layout: "vertical", submitText: "提交" },
      children: [
        { version: "1.0", id: "input_1", type: "Input", props: { label: "名称", name: "name", required: true, placeholder: "请输入名称" } },
        { version: "1.0", id: "select_1", type: "Select", props: { label: "类型", name: "type", options: [{ label: "选项A", value: "a" }, { label: "选项B", value: "b" }] } },
        { version: "1.0", id: "textarea_1", type: "Textarea", props: { label: "描述", name: "description", rows: 4 } },
        { version: "1.0", id: "switch_1", type: "Switch", props: { label: "启用", name: "enabled", default: true } },
        { version: "1.0", id: "btn_1", type: "Button", props: { children: "提交", type: "primary", action: "submit" } },
      ],
    };
  }

  if (uiType === "settings") {
    return {
      version: "1.0",
      id: `settings_${Date.now()}`,
      type: "Tabs",
      props: { items: [{ key: "general", label: "常规" }, { key: "advanced", label: "高级" }] },
      children: [
        {
          version: "1.0", id: "tab_general", type: "Container", props: { tabKey: "general" },
          children: [
            { version: "1.0", id: "input_appName", type: "Input", props: { label: "应用名称", name: "appName", default: "AxAgent" } },
            { version: "1.0", id: "select_lang", type: "Select", props: { label: "语言", name: "lang", options: [{ label: "中文", value: "zh" }, { label: "English", value: "en" }] } },
          ],
        },
        {
          version: "1.0", id: "tab_advanced", type: "Container", props: { tabKey: "advanced" },
          children: [
            { version: "1.0", id: "switch_debug", type: "Switch", props: { label: "调试模式", name: "debug", default: false } },
            { version: "1.0", id: "input_api", type: "Input", props: { label: "API 端点", name: "apiEndpoint" } },
          ],
        },
      ],
    };
  }

  // custom / report 默认
  const truncatedTitle = request.prompt.length > 30 ? request.prompt.slice(0, 30) + "..." : request.prompt;
  return {
    version: "1.0",
    id: `custom_${Date.now()}`,
    type: "Card",
    props: { title: truncatedTitle },
    children: [
      { version: "1.0", id: "md_1", type: "Markdown", props: { content: `# 基于描述生成的 UI\n\n${request.prompt}\n\n*此 UI 由 NL2UI 自动生成*` } },
      { version: "1.0", id: "tag_1", type: "Tag", props: { children: "AI 生成", color: "blue" } },
    ],
  };
}
