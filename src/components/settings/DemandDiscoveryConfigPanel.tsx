// i18n-exempt: 配置映射表/业务数据字符串，非用户可见 UI 文案
// SPDX-License-Identifier: AGPL-3.0-only

import type { Variable, WorkflowTemplateInput, WorkflowTemplateResponse } from "@/components/workflow/types";
import { invoke } from "@/lib/invoke";
import { App, Button, Input, InputNumber, Select, Slider, Space, Switch, Tag, theme } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const TEMPLATE_ID = "demand-discovery";

/**
 * 需求发现工作流默认配置参数
 * 命名规范: snake_case, 必须与后端 seed 一致
 */
function getDefaultVariables(): Variable[] {
  const vars: Variable[] = [];
  const b = (name: string, val: unknown, desc: string, type: string) =>
    vars.push({ name, varType: type, value: val, description: desc, isSecret: false });

  // ── 领域关键词匹配 ──
  b("domain_tech", "科技/AI/软件/编程/算法", "demandDiscovery.configDescriptions.domainTech", "string");
  b("domain_design", "设计/UI/UX/Logo/品牌/视觉", "demandDiscovery.configDescriptions.domainDesign", "string");
  b("domain_marketing", "营销/推广/SEO/广告/内容运营", "demandDiscovery.configDescriptions.domainMarketing", "string");
  b("domain_business", "商业/咨询/财务/法务/HR", "demandDiscovery.configDescriptions.domainBusiness", "string");
  b("domain_content", "写作/翻译/视频/音频/播客", "demandDiscovery.configDescriptions.domainContent", "string");
  b("domain_edu", "教育/培训/课程/教学", "demandDiscovery.configDescriptions.domainEdu", "string");
  b("domain_medical", "医疗/健康/药品/器械", "demandDiscovery.configDescriptions.domainMedical", "string");
  b("domain_finance", "金融/投资/股票/基金/保险", "demandDiscovery.configDescriptions.domainFinance", "string");

  // ── 预算区间分类 ──
  b("budget_low_max", 1000, "demandDiscovery.configDescriptions.budgetLowMax", "number");
  b("budget_mid_max", 10000, "demandDiscovery.configDescriptions.budgetMidMax", "number");
  b("budget_high_max", 100000, "demandDiscovery.configDescriptions.budgetHighMax", "number");
  b("budget_weight_low", 1.0, "demandDiscovery.configDescriptions.budgetWeightLow", "number");
  b("budget_weight_mid", 1.2, "demandDiscovery.configDescriptions.budgetWeightMid", "number");
  b("budget_weight_high", 1.5, "demandDiscovery.configDescriptions.budgetWeightHigh", "number");
  b("budget_weight_enterprise", 2.0, "demandDiscovery.configDescriptions.budgetWeightEnterprise", "number");

  // ── 能力匹配配置 ──
  b("capability_source_skill_weight", 1.2, "demandDiscovery.configDescriptions.capabilitySourceSkillWeight", "number");
  b("capability_source_mcp_weight", 1.5, "demandDiscovery.configDescriptions.capabilitySourceMcpWeight", "number");
  b(
    "capability_source_workflow_weight",
    1.0,
    "demandDiscovery.configDescriptions.capabilitySourceWorkflowWeight",
    "number",
  );
  b("capability_source_tool_weight", 0.8, "demandDiscovery.configDescriptions.capabilitySourceToolWeight", "number");
  b("capability_gap_auto_create", true, "demandDiscovery.configDescriptions.capabilityGapAutoCreate", "boolean");
  b(
    "capability_sufficient_threshold",
    0.7,
    "demandDiscovery.configDescriptions.capabilitySufficientThreshold",
    "number",
  );

  // ── 平台扫描参数 ──
  b("scan_timeout_secs", 30, "demandDiscovery.configDescriptions.scanTimeoutSecs", "number");
  b("scan_retry_max", 3, "demandDiscovery.configDescriptions.scanRetryMax", "number");
  b("scan_concurrency", 5, "demandDiscovery.configDescriptions.scanConcurrency", "number");
  b("scan_rate_limit", 10, "demandDiscovery.configDescriptions.scanRateLimit", "number");
  b("scan_auto_sync_interval_min", 60, "demandDiscovery.configDescriptions.scanAutoSyncIntervalMin", "number");
  b("scan_max_leads_per_sync", 50, "demandDiscovery.configDescriptions.scanMaxLeadsPerSync", "number");
  b("scan_deduplicate_window_hours", 24, "demandDiscovery.configDescriptions.scanDeduplicateWindowHours", "number");

  // ── 线索处理配置 ──
  b("lead_auto_match", true, "demandDiscovery.configDescriptions.leadAutoMatch", "boolean");
  b("lead_auto_confirm_threshold", 0.8, "demandDiscovery.configDescriptions.leadAutoConfirmThreshold", "number");
  b("lead_min_budget_to_confirm", 500, "demandDiscovery.configDescriptions.leadMinBudgetToConfirm", "number");
  b("lead_expiry_days", 30, "demandDiscovery.configDescriptions.leadExpiryDays", "number");
  b("lead_priority_by_budget", true, "demandDiscovery.configDescriptions.leadPriorityByBudget", "boolean");

  // ── 交付工作流参数 ──
  b("delivery_timeout_secs", 600, "demandDiscovery.configDescriptions.deliveryTimeoutSecs", "number");
  b("delivery_retry_max", 2, "demandDiscovery.configDescriptions.deliveryRetryMax", "number");
  b(
    "delivery_progress_update_interval_secs",
    10,
    "demandDiscovery.configDescriptions.deliveryProgressUpdateIntervalSecs",
    "number",
  );
  b("delivery_auto_start", false, "demandDiscovery.configDescriptions.deliveryAutoStart", "boolean");
  b("delivery_parallel_workflows", 3, "demandDiscovery.configDescriptions.deliveryParallelWorkflows", "number");

  // ── Agent 参数 ──
  b("agent_temperature", 0.3, "demandDiscovery.configDescriptions.agentTemperature", "number");
  b("agent_max_tokens", 16384, "demandDiscovery.configDescriptions.agentMaxTokens", "number");
  b("agent_timeout_secs", 300, "demandDiscovery.configDescriptions.agentTimeoutSecs", "number");
  b("agent_retry_max", 2, "demandDiscovery.configDescriptions.agentRetryMax", "number");

  // ── 通知规则 ──
  b("notify_new_lead", true, "demandDiscovery.configDescriptions.notifyNewLead", "boolean");
  b("notify_high_value_lead", true, "demandDiscovery.configDescriptions.notifyHighValueLead", "boolean");
  b("notify_capability_gap", true, "demandDiscovery.configDescriptions.notifyCapabilityGap", "boolean");
  b("notify_delivery_status_change", true, "demandDiscovery.configDescriptions.notifyDeliveryStatusChange", "boolean");
  b("notify_delivery_failed", true, "demandDiscovery.configDescriptions.notifyDeliveryFailed", "boolean");
  b("notify_min_priority", 2, "demandDiscovery.configDescriptions.notifyMinPriority", "number");

  // ── 系统参数 ──
  b("max_leads_cache", 1000, "demandDiscovery.configDescriptions.maxLeadsCache", "number");
  b("max_capabilities_cache", 500, "demandDiscovery.configDescriptions.maxCapabilitiesCache", "number");
  b("db_cleanup_days", 90, "demandDiscovery.configDescriptions.dbCleanupDays", "number");
  b("log_level", "info", "demandDiscovery.configDescriptions.logLevel", "enum");
  b("enable_analytics", true, "demandDiscovery.configDescriptions.enableAnalytics", "boolean");

  return vars;
}

function parseEnumOptions(desc?: string): string[] {
  if (!desc) { return []; }
  const match = desc.match(/: (.+)/);
  if (match) { return match[1].split(/\s*\/\s*/).map((s) => s.trim()); }
  return [];
}

function inferStep(v: Variable): number {
  if (v.name === "agent_temperature" || v.name.includes("weight")) { return 0.01; }
  return 1;
}

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
interface Props {}

/** number control — 垂直布局 */
function NumberControl({ v, value, onChange }: {
  v: Variable;
  value: unknown;
  onChange: (name: string, val: unknown) => void;
}) {
  const { t } = useTranslation();
  const desc = t(v.description ?? "");
  const hasPct = desc.includes("%");
  const val = Number(value ?? 0);
  return (
    <span className="sacp-number">
      <Slider
        min={0}
        max={v.varType === "number" && v.name.includes("weight") ? 2 : 100}
        step={inferStep(v)}
        className="sacp-number-slider"
        value={val}
        onChange={(v2) => onChange(v.name, v2)}
      />
      <InputNumber
        size="small"
        className="sacp-number-input"
        value={val}
        suffix={hasPct ? "%" : undefined}
        onChange={(v2) => v2 != null && onChange(v.name, v2)}
      />
    </span>
  );
}

function VariableControl({ v, value, onChange }: {
  v: Variable;
  value: unknown;
  onChange: (name: string, val: unknown) => void;
}) {
  const { t } = useTranslation();
  const desc = t(v.description ?? "");
  switch (v.varType) {
    case "boolean":
      return <Switch checked={!!value} onChange={(c) => onChange(v.name, c)} />;
    case "enum": {
      const options = parseEnumOptions(desc);
      return (
        <Select
          size="small"
          style={{ width: 140 }}
          value={String(value ?? "")}
          onChange={(val) => onChange(v.name, val)}
          options={options.map((o) => ({ value: o, label: o }))}
        />
      );
    }
    case "number":
      return <NumberControl v={v} value={value} onChange={onChange} />;
    default:
      return (
        <Input
          size="small"
          style={{ maxWidth: 220 }}
          value={String(value ?? "")}
          onChange={(e) => onChange(v.name, e.target.value)}
        />
      );
  }
}

export function DemandDiscoveryConfigPanel(_props: Props) {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [template, setTemplate] = useState<WorkflowTemplateResponse | null>(null);
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    invoke<WorkflowTemplateResponse | null>("get_workflow_template", { id: TEMPLATE_ID })
      .then((rsp) => {
        if (cancelled) { return; }
        if (rsp && (!rsp.variables || rsp.variables.length === 0)) {
          const defaults = getDefaultVariables();
          const input: WorkflowTemplateInput = {
            name: rsp.name || t("opc.demand.configDefaultWorkflowName"),
            description: rsp.description || t("opc.demand.configDefaultWorkflowDesc"),
            icon: rsp.icon || "🔍",
            tags: rsp.tags || ["opc", "demand"],
            triggerConfig: rsp.triggerConfig,
            nodes: rsp.nodes || [],
            edges: rsp.edges || [],
            inputSchema: rsp.inputSchema,
            outputSchema: rsp.outputSchema,
            variables: defaults,
            errorConfig: rsp.errorConfig,
          };
          invoke<boolean>("update_workflow_template", { id: TEMPLATE_ID, input }).catch(() => {});
          rsp.variables = defaults;
        }
        if (rsp) {
          setTemplate(rsp);
          const map: Record<string, unknown> = {};
          for (const v of rsp.variables) { map[v.name] = v.value; }
          setValues(map);
        } else {
          const defaults = getDefaultVariables();
          const map: Record<string, unknown> = {};
          for (const v of defaults) { map[v.name] = v.value; }
          setValues(map);
        }
      })
      .catch(() => {
        if (!cancelled) { message.error(t("demandDiscovery.settings.loadFailed")); }
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, [t, message]);

  const toolGroups = (() => {
    const allVars = template?.variables ?? getDefaultVariables();
    const varMap: Record<string, Variable> = {};
    for (const v of allVars) { varMap[v.name] = v; }
    const resolve = (names: string[]) => names.map((n) => varMap[n]).filter(Boolean);

    return [
      {
        tool: "domain_keywords",
        label: t("demandDiscovery.settings.group.domainKeywords"),
        vars: resolve([
          "domain_tech",
          "domain_design",
          "domain_marketing",
          "domain_business",
          "domain_content",
          "domain_edu",
          "domain_medical",
          "domain_finance",
        ]),
      },
      {
        tool: "budget",
        label: t("demandDiscovery.settings.group.budget"),
        vars: resolve([
          "budget_low_max",
          "budget_mid_max",
          "budget_high_max",
          "budget_weight_low",
          "budget_weight_mid",
          "budget_weight_high",
          "budget_weight_enterprise",
        ]),
      },
      {
        tool: "capability",
        label: t("demandDiscovery.settings.group.capability"),
        vars: resolve([
          "capability_source_skill_weight",
          "capability_source_mcp_weight",
          "capability_source_workflow_weight",
          "capability_source_tool_weight",
          "capability_gap_auto_create",
          "capability_sufficient_threshold",
        ]),
      },
      {
        tool: "scanner",
        label: t("demandDiscovery.settings.group.scanner"),
        vars: resolve([
          "scan_timeout_secs",
          "scan_retry_max",
          "scan_concurrency",
          "scan_rate_limit",
          "scan_auto_sync_interval_min",
          "scan_max_leads_per_sync",
          "scan_deduplicate_window_hours",
        ]),
      },
      {
        tool: "lead",
        label: t("demandDiscovery.settings.group.lead"),
        vars: resolve([
          "lead_auto_match",
          "lead_auto_confirm_threshold",
          "lead_min_budget_to_confirm",
          "lead_expiry_days",
          "lead_priority_by_budget",
        ]),
      },
      {
        tool: "delivery",
        label: t("demandDiscovery.settings.group.delivery"),
        vars: resolve([
          "delivery_timeout_secs",
          "delivery_retry_max",
          "delivery_progress_update_interval_secs",
          "delivery_auto_start",
          "delivery_parallel_workflows",
        ]),
      },
      {
        tool: "agent",
        label: t("demandDiscovery.settings.group.agent"),
        vars: resolve([
          "agent_temperature",
          "agent_max_tokens",
          "agent_timeout_secs",
          "agent_retry_max",
        ]),
      },
      {
        tool: "notification",
        label: t("demandDiscovery.settings.group.notification"),
        vars: resolve([
          "notify_new_lead",
          "notify_high_value_lead",
          "notify_capability_gap",
          "notify_delivery_status_change",
          "notify_delivery_failed",
          "notify_min_priority",
        ]),
      },
      {
        tool: "system",
        label: t("demandDiscovery.settings.group.system"),
        vars: resolve([
          "max_leads_cache",
          "max_capabilities_cache",
          "db_cleanup_days",
          "log_level",
          "enable_analytics",
        ]),
      },
    ].filter((g) => g.vars.length > 0);
  })();

  const handleChange = (name: string, val: unknown) => {
    setValues((prev) => ({ ...prev, [name]: val }));
  };

  const handleSave = async () => {
    if (!template) { return; }
    setSaving(true);
    const updatedVars = template.variables.map((v) => ({ ...v, value: values[v.name] ?? v.value }));
    const input: WorkflowTemplateInput = {
      name: template.name,
      description: template.description,
      icon: template.icon,
      tags: template.tags,
      triggerConfig: template.triggerConfig,
      nodes: template.nodes,
      edges: template.edges,
      inputSchema: template.inputSchema,
      outputSchema: template.outputSchema,
      variables: updatedVars,
      errorConfig: template.errorConfig,
      toolDefs: template.toolDefs,
    };
    try {
      await invoke<boolean>("update_workflow_template", { id: TEMPLATE_ID, input });
      message.success(t("demandDiscovery.settings.saveSuccess"));
    } catch (e) {
      console.error("[DemandDiscoveryConfigPanel] save failed:", e);
      message.error(t("demandDiscovery.settings.saveFailed", { error: String(e) }));
    } finally {
      setSaving(false);
    }
  };

  const handleReset = async () => {
    if (!template) { return; }
    const defaults = getDefaultVariables();
    const map: Record<string, unknown> = {};
    for (const v of defaults) { map[v.name] = v.value; }
    setValues(map);
  };

  if (loading) {
    return (
      <div style={{ textAlign: "center", padding: 24, color: token.colorTextQuaternary }}>
        {t("common.loading")}
      </div>
    );
  }

  const rowStyle = { padding: "4px 0" };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex justify-end gap-2">
        <Button size="small" onClick={handleReset}>
          {t("demandDiscovery.settings.resetToDefaults")}
        </Button>
      </div>

      {toolGroups.map((g) => (
        <SettingsGroup
          key={g.tool}
          title={
            <Space size={4}>
              <span>{g.label}</span>
              <Tag className="text-xs m-0" color="default">⚙️ {g.tool}</Tag>
            </Space>
          }
        >
          <div className="sacp-vars">
            {g.vars.map((v) => (
              <div key={v.name} style={rowStyle} className="flex items-center justify-between sacp-row">
                <span className="sacp-var-label" style={{ fontSize: 13, color: token.colorText }}>
                  {v.description ? t(v.description) : v.name}
                </span>
                <span style={{ display: "inline-flex", alignItems: "center", gap: 8, flexShrink: 0, marginLeft: 16 }}>
                  <VariableControl v={v} value={values[v.name]} onChange={handleChange} />
                </span>
              </div>
            ))}
          </div>
        </SettingsGroup>
      ))}

      <div style={{ display: "flex", justifyContent: "flex-end", paddingTop: 8 }}>
        <Button type="primary" loading={saving} onClick={handleSave}>
          {t("demandDiscovery.settings.saveConfig")}
        </Button>
      </div>
    </div>
  );
}
