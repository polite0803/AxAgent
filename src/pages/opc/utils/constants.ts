// SPDX-License-Identifier: AGPL-3.0-only

// ── 类型定义 ─────────────────────────────────────────────────

export interface InvoiceLineItem {
  description: string;
  quantity: number;
  unit_price: number;
  tax_rate: number;
  total: number;
}

export interface Invoice {
  id: string;
  customer_id: string;
  invoice_number: string;
  status: string;
  line_items?: InvoiceLineItem[];
  subtotal: number;
  tax_total: number;
  total: number;
  currency: string;
  notes: string;
  due_at: number | null;
  paid_at: number | null;
  issued_at: number | null;
  created_at: number;
  updated_at: number;
}

export interface Customer {
  id: string;
  name: string;
  email: string;
  phone: string | null;
  company: string | null;
  status: string;
  source: string | null;
  tags: string[];
  notes: string;
  total_revenue: number;
  invoice_count: number;
  created_at: number;
  updated_at: number;
}

export interface Milestone {
  id: string;
  title: string;
  description: string;
  due_at: number | null;
  completed_at: number | null;
  status: string;
}

export interface Project {
  id: string;
  customer_id: string | null;
  title: string;
  description: string;
  status: string;
  milestones: Milestone[];
  budget: number | null;
  currency: string;
  started_at: number | null;
  deadline: number | null;
  completed_at: number | null;
  notes: string;
  created_at: number;
  updated_at: number;
}

export interface KanbanItem {
  id: string;
  title: string;
  phase: string;
  owner_role_id: string | null;
  assignee_agent_id: string | null;
  manager_role_id: string | null;
  last_error: string | null;
  deps: string[];
  created_at: number;
  updated_at: number;
}

export type KanbanBoard = Record<string, KanbanItem[]>;

export interface SirResult {
  text: string;
  totalRounds: number;
  finalScore: number;
  confidence: number;
  strengths: string[];
  gaps: string[];
}

export interface MarketPack {
  id: string;
  name: string;
  icon: string;
  version: number;
  enabled: boolean;
  installed: boolean;
  path: string;
}

// ── 状态键映射 ───────────────────────────────────────────────

export function getInvoiceStatusKey(status: string): string {
  return `opc.invoiceStatus.${status}`;
}

export function getCustomerStatusKey(status: string): string {
  return `opc.customerStatus.${status}`;
}

export function getProjectStatusKey(status: string): string {
  return `opc.projectStatus.${status}`;
}

export function getSourceKey(source: string): string {
  return `opc.source.${source}`;
}

// ── 状态颜色映射 ─────────────────────────────────────────────

export const STATUS_COLOR_MAP: Record<string, string> = {
  draft: "default",
  sent: "blue",
  paid: "green",
  overdue: "red",
  cancelled: "default",
  refunded: "orange",
};

export const CUST_STATUS_COLOR_MAP: Record<string, string> = {
  lead: "default",
  prospect: "blue",
  active: "green",
  inactive: "default",
  churned: "red",
};

export const PROJ_STATUS_COLOR_MAP: Record<string, string> = {
  planning: "blue",
  active: "green",
  paused: "orange",
  completed: "default",
  cancelled: "red",
};

// ── 看板列 ──────────────────────────────────────────────────

export const KANBAN_COLUMNS = [
  "opc.kanban.colTodo",
  "opc.kanban.colInProgress",
  "opc.kanban.colBlocked",
  "opc.kanban.colReview",
  "opc.kanban.colDone",
  "opc.kanban.colCancelled",
];

// ── 需求发现类型 ───────────────────────────────────────────────

export interface CapabilityEntry {
  id: string;
  name: string;
  description: string;
  source: string;
  source_id: string;
  capability_type: string;
  applicable_scenarios: string[];
  example_deliverables: string[];
  metadata: Record<string, unknown>;
}

export interface CapabilityInventory {
  tools: CapabilityEntry[];
  skills: CapabilityEntry[];
  mcp_tools: CapabilityEntry[];
  workflows: CapabilityEntry[];
  agents: CapabilityEntry[];
  scanned_at: number;
  total_count: number;
}

export interface DemandLead {
  id: string;
  platform: string;
  title: string;
  description: string;
  status: string;
  priority: number;
  budget_min: number | null;
  budget_max: number | null;
  contact_name: string | null;
  contact_email: string | null;
  contact_phone: string | null;
  source_url: string | null;
  raw_snapshot: Record<string, unknown>;
  ai_analysis: Record<string, unknown>;
  matched_capabilities: Array<{ id: string; name: string; source: string; score: number }>;
  recommended_workflow: string | null;
  confidence_score: number;
  // 后端实体原始字段（_json 为 JSON 字符串，由 mapLead 解析到上面的对象字段）
  confidence: number | null;
  raw_snapshot_json: string | null;
  ai_analysis_json: string | null;
  matched_capabilities_json: string | null;
  recommended_workflow_id: string | null;
  // 需求价值评估字段
  pain_score: number | null;
  market_gap_score: number | null;
  commercial_value_score: number | null;
  opportunity_level: string | null;
  demand_type: string | null;
  evaluated_at: number | null;
  created_at: number;
  updated_at: number;
}

export interface Delivery {
  id: string;
  lead_id: string;
  title: string;
  workflow_template_id: string;
  status: string;
  progress: number;
  started_at: number | null;
  completed_at: number | null;
  result_summary: string | null;
  deliverables: Array<Record<string, unknown>>;
  errors: Array<Record<string, unknown>>;
  metadata: Record<string, unknown>;
  created_at: number;
  updated_at: number;
}

export const LEAD_STATUS_COLOR_MAP: Record<string, string> = {
  new: "default",
  qualified: "blue",
  executing: "orange",
  running: "cyan",
  delivered: "green",
  failed: "red",
  cancelled: "default",
  expired: "red",
  claimed: "purple",
};

export const DELIVERY_STATUS_COLOR_MAP: Record<string, string> = {
  pending: "default",
  running: "blue",
  completed: "green",
  failed: "red",
  cancelled: "default",
};

// ── 平台配置 / 能力缺口类型 ──────────────────────────────────

export interface MarketPlatform {
  id: string;
  name: string;
  platform_type: string;
  enabled: number;
  base_url: string | null;
  config: Record<string, unknown>;
  last_sync_at: number | null;
  status: string;
  created_at: number;
  updated_at: number;
}

export interface CapabilityGap {
  id: string;
  lead_id: string | null;
  title: string;
  description: string;
  missing_capability: string;
  gap_type: string;
  suggested_action: string;
  priority: number;
  status: string;
  created_at: number;
  updated_at: number;
  closed_at: number | null;
}

// ── 定时任务类型 ──────────────────────────────────────────────

export interface CronJobData {
  id: string;
  name: string;
  schedule: string;
  description: string;
  prompt: string;
  status: string;
  workflow_id: string | null;
  task_type: string | null;
  platform: string | null;
  enabled_toolsets: string[];
  last_run_at: number | null;
  next_run_at: number | null;
  created_at: number;
  updated_at: number;
}

export const CRON_STATUS_COLOR_MAP: Record<string, string> = {
  active: "green",
  paused: "orange",
  completed: "default",
  failed: "red",
};

// ── 人才市场常量 ──────────────────────────────────────────────

export interface TalentRole {
  id: string;
  nameKey: string;
  descriptionKey: string;
  category: string;
  icon: string;
}

export const TALENT_CATEGORIES: Record<string, { labelKey: string; icon: string }> = {
  engineering: { labelKey: "opc.talent.catEngineering", icon: "💻" },
  design: { labelKey: "opc.talent.catDesign", icon: "🎨" },
  finance: { labelKey: "opc.talent.catFinance", icon: "💰" },
  marketing: { labelKey: "opc.talent.catMarketing", icon: "📢" },
  sales: { labelKey: "opc.talent.catSales", icon: "🤝" },
  product: { labelKey: "opc.talent.catProduct", icon: "📋" },
  security: { labelKey: "opc.talent.catSecurity", icon: "🔒" },
  data: { labelKey: "opc.talent.catData", icon: "📊" },
  devops: { labelKey: "opc.talent.catDevops", icon: "🚀" },
  testing: { labelKey: "opc.talent.catTesting", icon: "🧪" },
  support: { labelKey: "opc.talent.catSupport", icon: "🎧" },
  academic: { labelKey: "opc.talent.catAcademic", icon: "🎓" },
};

export const TALENT_ROLES: TalentRole[] = [
  {
    id: "ai-engineer",
    nameKey: "opc.talent.roleAiEngineer",
    descriptionKey: "opc.talent.roleAiEngineerDesc",
    category: "engineering",
    icon: "🤖",
  },
  {
    id: "backend-architect",
    nameKey: "opc.talent.roleBackendArchitect",
    descriptionKey: "opc.talent.roleBackendArchitectDesc",
    category: "engineering",
    icon: "🏗️",
  },
  {
    id: "frontend-developer",
    nameKey: "opc.talent.roleFrontendDev",
    descriptionKey: "opc.talent.roleFrontendDevDesc",
    category: "engineering",
    icon: "🖥️",
  },
  {
    id: "devops-engineer",
    nameKey: "opc.talent.roleDevops",
    descriptionKey: "opc.talent.roleDevopsDesc",
    category: "engineering",
    icon: "🚀",
  },
  {
    id: "code-reviewer",
    nameKey: "opc.talent.roleCodeReviewer",
    descriptionKey: "opc.talent.roleCodeReviewerDesc",
    category: "engineering",
    icon: "👀",
  },
  {
    id: "financial-analyst",
    nameKey: "opc.talent.roleFinancialAnalyst",
    descriptionKey: "opc.talent.roleFinancialAnalystDesc",
    category: "finance",
    icon: "📈",
  },
  {
    id: "accountant",
    nameKey: "opc.talent.roleAccountant",
    descriptionKey: "opc.talent.roleAccountantDesc",
    category: "finance",
    icon: "🧾",
  },
  {
    id: "security-expert",
    nameKey: "opc.talent.roleSecurityExpert",
    descriptionKey: "opc.talent.roleSecurityExpertDesc",
    category: "security",
    icon: "🛡️",
  },
  {
    id: "data-scientist",
    nameKey: "opc.talent.roleDataScientist",
    descriptionKey: "opc.talent.roleDataScientistDesc",
    category: "data",
    icon: "📊",
  },
  {
    id: "seo-specialist",
    nameKey: "opc.talent.roleSeoSpecialist",
    descriptionKey: "opc.talent.roleSeoSpecialistDesc",
    category: "marketing",
    icon: "🔍",
  },
  {
    id: "sales-engineer",
    nameKey: "opc.talent.roleSalesEngineer",
    descriptionKey: "opc.talent.roleSalesEngineerDesc",
    category: "sales",
    icon: "🤝",
  },
  {
    id: "product-manager",
    nameKey: "opc.talent.roleProductManager",
    descriptionKey: "opc.talent.roleProductManagerDesc",
    category: "product",
    icon: "📋",
  },
  {
    id: "ux-designer",
    nameKey: "opc.talent.roleUxDesigner",
    descriptionKey: "opc.talent.roleUxDesignerDesc",
    category: "design",
    icon: "🎨",
  },
  {
    id: "qa-engineer",
    nameKey: "opc.talent.roleQaEngineer",
    descriptionKey: "opc.talent.roleQaEngineerDesc",
    category: "testing",
    icon: "🧪",
  },
  {
    id: "tech-support",
    nameKey: "opc.talent.roleTechSupport",
    descriptionKey: "opc.talent.roleTechSupportDesc",
    category: "support",
    icon: "🎧",
  },
];
