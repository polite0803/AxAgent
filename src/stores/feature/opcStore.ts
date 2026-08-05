// SPDX-License-Identifier: AGPL-3.0-only

import { create } from "zustand";

// ── 类型定义 ──────────────────────────────────────────────────

export interface OpcDashboardSummary {
  total_revenue: number;
  total_invoices: number;
  active_projects: number;
  total_customers: number;
  recent_kpis: Array<{ name: string; value: number; unit: string; period: string }>;
}

export interface OpcInvoice {
  id: string;
  customer_id: string;
  invoice_number: string;
  status: string;
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

export interface OpcCustomer {
  id: string;
  name: string;
  email: string;
  phone: string | null;
  company: string | null;
  status: string;
  source: string | null;
  total_revenue: number;
  invoice_count: number;
  created_at: number;
  updated_at: number;
}

export interface OpcProject {
  id: string;
  customer_id: string | null;
  title: string;
  description: string;
  status: string;
  budget: number | null;
  currency: string;
  started_at: number | null;
  deadline: number | null;
  completed_at: number | null;
  notes: string;
  created_at: number;
  updated_at: number;
}

export interface OpcState {
  // 数据状态
  dashboard: OpcDashboardSummary | null;
  invoices: OpcInvoice[];
  customers: OpcCustomer[];
  projects: OpcProject[];

  // UI 状态
  activeTab: string;
  loadingMap: Record<string, boolean>;
  errorMap: Record<string, string | null>;

  // 缓存时间戳
  lastUpdated: {
    dashboard: number | null;
    invoices: number | null;
    customers: number | null;
    projects: number | null;
  };
}

export interface OpcActions {
  // 设置数据
  setDashboard: (data: OpcDashboardSummary | null) => void;
  setInvoices: (data: OpcInvoice[]) => void;
  setCustomers: (data: OpcCustomer[]) => void;
  setProjects: (data: OpcProject[]) => void;

  // 加载状态管理
  setLoading: (key: string, loading: boolean) => void;
  setError: (key: string, error: string | null) => void;

  // Tab 切换
  setActiveTab: (tab: string) => void;

  // 工具方法
  isLoading: (key: string) => boolean;
  getError: (key: string) => string | null;
  isDataStale: (key: keyof OpcState["lastUpdated"], maxAgeMs?: number) => boolean;
  reset: () => void;
}

const INITIAL_STATE: OpcState = {
  dashboard: null,
  invoices: [],
  customers: [],
  projects: [],
  activeTab: "dashboard",
  loadingMap: {},
  errorMap: {},
  lastUpdated: {
    dashboard: null,
    invoices: null,
    customers: null,
    projects: null,
  },
};

export const useOpcStore = create<OpcState & OpcActions>((set, get) => ({
  ...INITIAL_STATE,

  setDashboard: (data) =>
    set({
      dashboard: data,
      lastUpdated: { ...get().lastUpdated, dashboard: Date.now() },
    }),

  setInvoices: (data) =>
    set({
      invoices: data,
      lastUpdated: { ...get().lastUpdated, invoices: Date.now() },
    }),

  setCustomers: (data) =>
    set({
      customers: data,
      lastUpdated: { ...get().lastUpdated, customers: Date.now() },
    }),

  setProjects: (data) =>
    set({
      projects: data,
      lastUpdated: { ...get().lastUpdated, projects: Date.now() },
    }),

  setLoading: (key, loading) =>
    set({
      loadingMap: { ...get().loadingMap, [key]: loading },
    }),

  setError: (key, error) =>
    set({
      errorMap: { ...get().errorMap, [key]: error },
    }),

  setActiveTab: (tab) => set({ activeTab: tab }),

  isLoading: (key) => get().loadingMap[key] ?? false,

  getError: (key) => get().errorMap[key] ?? null,

  isDataStale: (key, maxAgeMs = 5 * 60 * 1000) => {
    const timestamp = get().lastUpdated[key];
    if (timestamp === null) { return true; }
    return Date.now() - timestamp > maxAgeMs;
  },

  reset: () => set({ ...INITIAL_STATE }),
}));
