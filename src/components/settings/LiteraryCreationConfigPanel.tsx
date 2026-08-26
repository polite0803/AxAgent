import type { Variable, WorkflowTemplateInput, WorkflowTemplateResponse } from "@/components/workflow/types";
import { invoke } from "@/lib/invoke";
import { App, Button, Input, InputNumber, Select, Switch, theme } from "antd";
import i18next from "i18next";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const TEMPLATE_ID = "workflow-cm-literary-creation";

/** 获取默认变量列表（与后端种子保持同步） */
function getDefaultVariables(): Variable[] {
  const vars: Variable[] = [];
  const b = (name: string, val: unknown, desc: string, type: string) =>
    vars.push({ name, var_type: type, value: val, description: desc, is_secret: false });

  // 输出配置
  b("output_dir", "./output/literary_creation", "literaryCreation.configDescriptions.outputDir", "string");
  b(
    "document_title",
    i18next.t("literaryCreation.defaultDocumentTitle"),
    "literaryCreation.configDescriptions.documentTitle",
    "string",
  );
  b("file_format", "docx", "literaryCreation.configDescriptions.fileFormat", "enum");

  // 内容配置
  b("chapter_separator", "\n\n---\n\n", "literaryCreation.configDescriptions.chapterSeparator", "string");
  b("include_chapter_numbers", true, "literaryCreation.configDescriptions.includeChapterNumbers", "boolean");
  b("word_count_min", 1000, "literaryCreation.configDescriptions.wordCountMin", "number");
  b("word_count_max", 50000, "literaryCreation.configDescriptions.wordCountMax", "number");

  // 评审配置
  b("review_strictness", "balanced", "literaryCreation.configDescriptions.reviewStrictness", "enum");
  b("tolerance_threshold", 1, "literaryCreation.configDescriptions.toleranceThreshold", "number");

  return vars;
}

function parseEnumOptions(desc?: string): string[] {
  if (!desc) { return []; }
  const match = desc.match(/: (.+)/);
  if (match) { return match[1].split(/\s*\/\s*/).map((s) => s.trim()); }
  return [];
}

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
interface Props {}

/** 数值控件 */
function NumberControl({ v, value, onChange }: {
  v: Variable;
  value: unknown;
  onChange: (name: string, val: unknown) => void;
}) {
  const val = Number(value ?? 0);
  return (
    <InputNumber
      size="small"
      style={{ width: 120 }}
      min={0}
      value={val}
      onChange={(v2) => v2 != null && onChange(v.name, v2)}
    />
  );
}

/** 变量控件 */
function VariableControl({ v, value, onChange }: {
  v: Variable;
  value: unknown;
  onChange: (name: string, val: unknown) => void;
}) {
  const { t } = useTranslation();
  const desc = t(v.description ?? "");
  switch (v.var_type) {
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
          style={{ maxWidth: 200 }}
          value={String(value ?? "")}
          onChange={(e) => onChange(v.name, e.target.value)}
        />
      );
  }
}

export function LiteraryCreationConfigPanel(_props: Props) {
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
            name: rsp.name,
            description: rsp.description,
            icon: rsp.icon,
            tags: rsp.tags,
            trigger_config: rsp.triggerConfig,
            nodes: rsp.nodes,
            edges: rsp.edges,
            input_schema: rsp.inputSchema,
            output_schema: rsp.outputSchema,
            variables: defaults,
            error_config: rsp.errorConfig,
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
        if (!cancelled) { message.error(t("literaryCreation.settings.loadFailed")); }
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, [t, message]);

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
      trigger_config: template.triggerConfig,
      nodes: template.nodes,
      edges: template.edges,
      input_schema: template.inputSchema,
      output_schema: template.outputSchema,
      variables: updatedVars,
      error_config: template.errorConfig,
      tool_defs: template.toolDefs,
    };
    try {
      await invoke<boolean>("update_workflow_template", { id: TEMPLATE_ID, input });
      message.success(t("literaryCreation.settings.saveSuccess"));
    } catch (e) {
      console.error("[LiteraryCreationConfigPanel] save failed:", e, { input });
      message.error(t("literaryCreation.settings.saveFailed", { error: String(e) }));
    } finally {
      setSaving(false);
    }
  };

  // 变量分组
  const variableGroups = useMemo(() => {
    const allVars = template?.variables ?? getDefaultVariables();
    const varMap: Record<string, Variable> = {};
    for (const v of allVars) { varMap[v.name] = v; }

    const resolve = (names: string[]) => names.map((n) => varMap[n]).filter(Boolean);

    return [
      {
        key: "output",
        label: t("literaryCreation.settings.group.output"),
        vars: resolve(["output_dir", "document_title", "file_format"]),
      },
      {
        key: "content",
        label: t("literaryCreation.settings.group.content"),
        vars: resolve(["chapter_separator", "include_chapter_numbers", "word_count_min", "word_count_max"]),
      },
      {
        key: "review",
        label: t("literaryCreation.settings.group.review"),
        vars: resolve(["review_strictness", "tolerance_threshold"]),
      },
    ].filter((g) => g.vars.length > 0);
  }, [template, t]);

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
      {variableGroups.map((g) => (
        <SettingsGroup key={g.key} title={<span>{g.label}</span>}>
          <div>
            {g.vars.map((v) => (
              <div key={v.name} style={rowStyle} className="flex items-center justify-between">
                <span style={{ fontSize: 13, color: token.colorText }}>
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
          {t("literaryCreation.settings.saveConfig")}
        </Button>
      </div>
    </div>
  );
}
