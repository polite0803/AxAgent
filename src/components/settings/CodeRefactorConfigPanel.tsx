import type { Variable, WorkflowTemplateResponse } from "@/components/workflow/types";
import { invoke } from "@/lib/invoke";
import { App, Button, Input, InputNumber, Select, Switch, Tag, theme, Tooltip } from "antd";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

/** 重构工作流 ID 映射 */
const REFACTOR_WORKFLOW_MAP: Record<string, { templateId: string; fields: string[] }> = {
  "wf-eng-refactor": {
    templateId: "wf-eng-refactor",
    fields: [
      "refactor_goal",
      "target_modules",
      "codebase_path",
      "risk_level",
      "test_coverage_min",
      "auto_fix",
      "max_file_changes",
    ],
  },
  "wf-eng-refactor-lite": {
    templateId: "wf-eng-refactor-lite",
    fields: [
      "refactor_goal",
      "target_modules",
      "codebase_path",
      "risk_level",
      "max_file_changes",
      "auto_fix",
    ],
  },
  "wf-eng-tech-debt": {
    templateId: "wf-eng-tech-debt",
    fields: [
      "codebase_path",
      "tech_debt_categories",
      "severity_threshold",
      "estimated_hours",
      "priority_fix",
      "generate_report",
    ],
  },
};

/** 变量类型 */
type VarType = "string" | "number" | "boolean" | "enum";

/** 变量定义 */
interface RefactorVariable {
  name: string;
  label: string;
  type: VarType;
  default: unknown;
  description: string;
  options?: string[];
  min?: number;
  max?: number;
}

/** 重构工作流变量定义（与后端种子同步） */
const REFACTOR_VARIABLES: RefactorVariable[] = [
  // 通用字段
  {
    name: "refactor_goal",
    label: "opc.refactor.fields.refactor_goal.label",
    type: "string",
    default: "",
    description: "opc.refactor.fields.refactor_goal.desc",
  },
  {
    name: "target_modules",
    label: "opc.refactor.fields.target_modules.label",
    type: "string",
    default: "",
    description: "opc.refactor.fields.target_modules.desc",
  },
  {
    name: "codebase_path",
    label: "opc.refactor.fields.codebase_path.label",
    type: "string",
    default: "",
    description: "opc.refactor.fields.codebase_path.desc",
  },
  // 重构配置
  {
    name: "risk_level",
    label: "opc.refactor.fields.risk_level.label",
    type: "enum",
    default: "low",
    description: "opc.refactor.fields.risk_level.desc: low / medium / high",
    options: ["low", "medium", "high"],
  },
  {
    name: "test_coverage_min",
    label: "opc.refactor.fields.test_coverage_min.label",
    type: "number",
    default: 70,
    description: "opc.refactor.fields.test_coverage_min.desc",
    min: 0,
    max: 100,
  },
  {
    name: "auto_fix",
    label: "opc.refactor.fields.auto_fix.label",
    type: "boolean",
    default: false,
    description: "opc.refactor.fields.auto_fix.desc",
  },
  {
    name: "max_file_changes",
    label: "opc.refactor.fields.max_file_changes.label",
    type: "number",
    default: 50,
    description: "opc.refactor.fields.max_file_changes.desc",
    min: 1,
    max: 500,
  },
  // 技术债专属
  {
    name: "tech_debt_categories",
    label: "opc.refactor.fields.tech_debt_categories.label",
    type: "enum",
    default: "all",
    description:
      "opc.refactor.fields.tech_debt_categories.desc: all / security / performance / maintainability / duplicate",
    options: ["all", "security", "performance", "maintainability", "duplicate"],
  },
  {
    name: "severity_threshold",
    label: "opc.refactor.fields.severity_threshold.label",
    type: "enum",
    default: "medium",
    description: "opc.refactor.fields.severity_threshold.desc: low / medium / high / critical",
    options: ["low", "medium", "high", "critical"],
  },
  {
    name: "estimated_hours",
    label: "opc.refactor.fields.estimated_hours.label",
    type: "number",
    default: 40,
    description: "opc.refactor.fields.estimated_hours.desc",
    min: 1,
    max: 10000,
  },
  {
    name: "priority_fix",
    label: "opc.refactor.fields.priority_fix.label",
    type: "boolean",
    default: true,
    description: "opc.refactor.fields.priority_fix.desc",
  },
  {
    name: "generate_report",
    label: "opc.refactor.fields.generate_report.label",
    type: "boolean",
    default: true,
    description: "opc.refactor.fields.generate_report.desc",
  },
];

/** 生成 Variable[] */
function buildVariables(workflowId: string): Variable[] {
  const mapping = REFACTOR_WORKFLOW_MAP[workflowId];
  const fieldNames = mapping?.fields ?? REFACTOR_VARIABLES.map((v) => v.name);
  const vars: Variable[] = [];

  for (const name of fieldNames) {
    const def = REFACTOR_VARIABLES.find((v) => v.name === name);
    if (!def) { continue; }
    vars.push({
      name: def.name,
      varType: def.type === "enum" ? "enum" : def.type,
      value: def.default,
      description: def.description,
      isSecret: false,
    });
  }
  return vars;
}

/** 获取默认变量 */
function getDefaultVariables(workflowId: string): Variable[] {
  return buildVariables(workflowId);
}

interface Props {
  workflowId: string;
  onVariablesChange?: (vars: Record<string, unknown>) => void;
}

/** 变量控件 */
function VariableControl({ v, value, onChange, disabled }: {
  v: RefactorVariable;
  value: unknown;
  onChange: (name: string, val: unknown) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation();

  const translateOption = (option: string): string => {
    const keyPrefix = v.name === "risk_level"
      ? "opc.refactor.risk_levels"
      : v.name === "tech_debt_categories"
      ? "opc.refactor.debt_categories"
      : "opc.refactor.severity_levels";
    return t(`${keyPrefix}.${option}`);
  };

  switch (v.type) {
    case "boolean":
      return <Switch checked={!!value} disabled={disabled} onChange={(c) => onChange(v.name, c)} />;
    case "enum":
      return (
        <Select
          size="small"
          style={{ width: 140 }}
          value={String(value ?? "")}
          disabled={disabled}
          onChange={(val) => onChange(v.name, val)}
          options={(v.options ?? []).map((o) => ({
            value: o,
            label: translateOption(o),
          }))}
        />
      );
    case "number":
      return (
        <InputNumber
          size="small"
          style={{ width: 120 }}
          min={v.min}
          max={v.max}
          value={Number(value ?? 0)}
          disabled={disabled}
          onChange={(n) => n != null && onChange(v.name, n)}
        />
      );
    default:
      return (
        <Input
          size="small"
          style={{ maxWidth: 200 }}
          value={String(value ?? "")}
          disabled={disabled}
          placeholder={v.description ? undefined : v.label}
          onChange={(e) => onChange(v.name, e.target.value)}
        />
      );
  }
}

/** 代码重构配置面板 */
export function CodeRefactorConfigPanel({ workflowId, onVariablesChange }: Props) {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [template, setTemplate] = useState<WorkflowTemplateResponse | null>(null);
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  const mapping = REFACTOR_WORKFLOW_MAP[workflowId];
  const templateId = mapping?.templateId ?? workflowId;

  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    invoke<WorkflowTemplateResponse | null>("get_workflow_template", { id: templateId })
      .then((rsp) => {
        if (cancelled) { return; }
        if (rsp && (!rsp.variables || rsp.variables.length === 0)) {
          const defaults = getDefaultVariables(workflowId);
          const input = {
            name: rsp.name,
            description: rsp.description,
            icon: rsp.icon,
            tags: rsp.tags,
            triggerConfig: rsp.triggerConfig,
            nodes: rsp.nodes,
            edges: rsp.edges,
            inputSchema: rsp.inputSchema,
            outputSchema: rsp.outputSchema,
            variables: defaults,
            errorConfig: rsp.errorConfig,
          };
          invoke<boolean>("update_workflow_template", { id: templateId, input }).catch(() => {});
          rsp.variables = defaults;
        }
        if (rsp) {
          setTemplate(rsp);
          const map: Record<string, unknown> = {};
          for (const v of rsp.variables) { map[v.name] = v.value; }
          setValues(map);
          onVariablesChange?.(map);
        } else {
          const defaults = getDefaultVariables(workflowId);
          const map: Record<string, unknown> = {};
          for (const v of defaults) { map[v.name] = v.value; }
          setValues(map);
          onVariablesChange?.(map);
        }
      })
      .catch(() => {
        if (!cancelled) { message.error(t("opc.refactor.settings.loadFailed")); }
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });

    return () => {
      cancelled = true;
    };
  }, [templateId, workflowId, t, message, onVariablesChange]);

  const handleChange = (name: string, val: unknown) => {
    setValues((prev) => {
      const next = { ...prev, [name]: val };
      onVariablesChange?.(next);
      return next;
    });
  };

  const handleSave = async () => {
    if (!template) { return; }
    setSaving(true);
    const updatedVars = template.variables.map((v) => ({ ...v, value: values[v.name] ?? v.value }));
    const input = {
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
      toolDefs: (template as WorkflowTemplateResponse & { toolDefs?: unknown[] }).toolDefs,
    };
    try {
      await invoke<boolean>("update_workflow_template", { id: templateId, input });
      message.success(t("opc.refactor.settings.saveSuccess"));
    } catch (e) {
      console.error("[CodeRefactorConfigPanel] save failed:", e);
      message.error(t("opc.refactor.settings.saveFailed", { error: String(e) }));
    } finally {
      setSaving(false);
    }
  };

  // 变量分组
  const variableGroups = useMemo(() => {
    const fieldNames = mapping?.fields ?? [];
    const defs = REFACTOR_VARIABLES.filter((v) => fieldNames.includes(v.name));

    // 通用配置
    const common: RefactorVariable[] = [];
    const refactor: RefactorVariable[] = [];
    const techDebt: RefactorVariable[] = [];

    for (const def of defs) {
      if (["refactor_goal", "target_modules", "codebase_path"].includes(def.name)) {
        common.push(def);
      } else if (
        ["tech_debt_categories", "severity_threshold", "estimated_hours", "priority_fix", "generate_report"].includes(
          def.name,
        )
      ) {
        techDebt.push(def);
      } else {
        refactor.push(def);
      }
    }

    const groups: { key: string; label: string; vars: RefactorVariable[] }[] = [];
    if (common.length > 0) {
      groups.push({
        key: "common",
        label: t("opc.refactor.settings.group.common"),
        vars: common,
      });
    }
    if (refactor.length > 0) {
      groups.push({
        key: "refactor",
        label: t("opc.refactor.settings.group.refactor"),
        vars: refactor,
      });
    }
    if (techDebt.length > 0) {
      groups.push({
        key: "tech_debt",
        label: t("opc.refactor.settings.group.techDebt"),
        vars: techDebt,
      });
    }
    return groups;
  }, [mapping, t]);

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
      {template?.description && (
        <div
          style={{
            background: token.colorBgLayout,
            borderRadius: 8,
            padding: 12,
            marginBottom: 4,
          }}
        >
          <Tag color="blue">{template.version}</Tag>
          <span style={{ marginLeft: 8, color: token.colorTextDescription }}>
            {template.description}
          </span>
        </div>
      )}

      {variableGroups.map((g) => (
        <SettingsGroup key={g.key} title={<span>{g.label}</span>}>
          <div>
            {g.vars.map((v) => (
              <div key={v.name} style={rowStyle} className="flex items-center justify-between">
                <Tooltip title={t(v.description)}>
                  <span style={{ fontSize: 13, color: token.colorText }}>
                    {t(v.label)}
                  </span>
                </Tooltip>
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
          {t("opc.refactor.settings.saveConfig")}
        </Button>
      </div>
    </div>
  );
}

/** 风险等级标签 */
export function RiskLevelTag({ level }: { level: string }) {
  const { t } = useTranslation();
  const colorMap: Record<string, string> = {
    low: "green",
    medium: "orange",
    high: "red",
    critical: "magenta",
  };
  return (
    <Tag color={colorMap[level] ?? "default"}>
      {t(`opc.refactor.risk_levels.${level}`)}
    </Tag>
  );
}
