// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 工作流级验证规则面板
 *
 * 在 WorkflowEditor 中提供全局验证规则配置：
 * - JSON Schema 验证：校验最终输出结构
 * - 路径断言：校验关键节点的中间输出
 * - 阈值规则：校验响应时间、token 用量等指标
 * - 规则启用/禁用切换
 */
import { useWorkflowEditorStore } from "@/stores";
import { MinusCircleOutlined, PlusOutlined, SaveOutlined, SettingOutlined } from "@ant-design/icons";
import { App, Button, Collapse, Empty, Input, Select, Space, Switch, Tag, theme, Tooltip, Typography } from "antd";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useShallow } from "zustand/react/shallow";

const { Text } = Typography;
const { TextArea } = Input;

export type RuleType = "json_schema" | "path_assertion" | "threshold";
export type ThresholdMetric = "duration_ms" | "token_count" | "node_count" | "retry_count";
export type AssertionOp = "equals" | "not_equals" | "contains" | "exists" | "matches" | "gt" | "lt";

export interface ValidationRule {
  id: string;
  name: string;
  enabled: boolean;
  rule_type: RuleType;
  description: string;
  // JSON Schema 规则
  schema?: string;
  // 路径断言规则
  source_node_id?: string;
  expected_path?: string;
  assertion_op?: AssertionOp;
  expected_value?: string;
  // 阈值规则
  metric?: ThresholdMetric;
  max_value?: number;
  min_value?: number;
}

export interface WorkflowValidationConfig {
  enabled: boolean;
  rules: ValidationRule[];
  on_fail: "warn" | "block" | "continue";
}

interface ValidationRulesPanelProps {
  onClose?: () => void;
}

function generateId(): string {
  return `rule_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

function defaultRule(): ValidationRule {
  return {
    id: generateId(),
    name: "",
    enabled: true,
    rule_type: "json_schema",
    description: "",
    schema: JSON.stringify({ type: "object", properties: {} }, null, 2),
  };
}

const RULE_TYPE_OPTIONS: { value: RuleType; label: string }[] = [
  { value: "json_schema", label: "JSON Schema" },
  { value: "path_assertion", label: "Path Assertion" },
  { value: "threshold", label: "Threshold" },
];

const ASSERTION_OPTS: { value: AssertionOp; label: string }[] = [
  { value: "equals", label: "equals" },
  { value: "not_equals", label: "not equals" },
  { value: "contains", label: "contains" },
  { value: "exists", label: "exists" },
  { value: "matches", label: "matches (regex)" },
  { value: "gt", label: "greater than" },
  { value: "lt", label: "less than" },
];

const METRIC_OPTS: { value: ThresholdMetric; label: string }[] = [
  { value: "duration_ms", label: "Execution Duration (ms)" },
  { value: "token_count", label: "Token Count" },
  { value: "node_count", label: "Node Count" },
  { value: "retry_count", label: "Retry Count" },
];

export function ValidationRulesPanel({ onClose }: ValidationRulesPanelProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message } = App.useApp();

  const [validationConfig, setValidationConfig] = useState<WorkflowValidationConfig>({
    enabled: true,
    rules: [],
    on_fail: "warn",
  });

  const nodes = useWorkflowEditorStore(useShallow((s) => s.nodes));

  const nodeOptions = useMemo(() => {
    return (nodes || []).map((n: { id: string; title?: string }) => ({
      value: n.id,
      label: n.title || n.id,
    }));
  }, [nodes]);

  const addRule = useCallback(() => {
    setValidationConfig((prev) => ({
      ...prev,
      rules: [...prev.rules, defaultRule()],
    }));
  }, []);

  const removeRule = useCallback((ruleId: string) => {
    setValidationConfig((prev) => ({
      ...prev,
      rules: prev.rules.filter((r) => r.id !== ruleId),
    }));
  }, []);

  const updateRule = useCallback((ruleId: string, updates: Partial<ValidationRule>) => {
    setValidationConfig((prev) => ({
      ...prev,
      rules: prev.rules.map((r) => (r.id === ruleId ? { ...r, ...updates } : r)),
    }));
  }, []);

  const saveConfig = useCallback(async () => {
    // TODO: persist to backend via invoke
    message.success(t("workflow.validation.saved"));
  }, [message, t]);

  const hasRules = validationConfig.rules.length > 0;

  const failOptions = [
    { value: "warn", label: t("workflow.validation.warn") },
    { value: "block", label: t("workflow.validation.block") },
    { value: "continue", label: t("workflow.validation.continue") },
  ];

  return (
    <div className="p-3" style={{ maxHeight: "100%", overflow: "auto" }}>
      {/* 头部：启用开关 + 保存按钮 */}
      <div className="flex items-center justify-between mb-3">
        <Space>
          <SettingOutlined />
          <Text strong>{t("workflow.validation.title")}</Text>
        </Space>
        <Space>
          <Switch
            checked={validationConfig.enabled}
            onChange={(v) => setValidationConfig((p) => ({ ...p, enabled: v }))}
            size="small"
          />
          <Button type="primary" size="small" icon={<SaveOutlined />} onClick={saveConfig}>
            {t("common.save")}
          </Button>
        </Space>
      </div>

      {/* 失败策略 */}
      <div className="mb-3">
        <Text type="secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>
          {t("workflow.validation.onFailStrategy")}
        </Text>
        <Select
          value={validationConfig.on_fail}
          onChange={(v) => setValidationConfig((p) => ({ ...p, on_fail: v }))}
          size="small"
          style={{ width: "100%" }}
          options={failOptions}
        />
      </div>

      {/* 规则列表 */}
      {!hasRules && (
        <div className="mb-3">
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("workflow.validation.noRules")}
              </Text>
            }
          />
        </div>
      )}

      <Collapse
        ghost
        size="small"
        items={validationConfig.rules.map((rule, idx) => ({
          key: rule.id,
          label: (
            <Space>
              <div onPointerDown={(e) => e.stopPropagation()}>
                <Switch
                  checked={rule.enabled}
                  onChange={(v) => updateRule(rule.id, { enabled: v })}
                  size="small"
                />
              </div>
              <Text style={{ fontSize: 12 }} ellipsis>
                {rule.name || `${t("workflow.validation.rule")} ${idx + 1}`}
              </Text>
              <Tag style={{ fontSize: 9, lineHeight: "14px" }}>{rule.rule_type}</Tag>
            </Space>
          ),
          extra: (
            <Tooltip title={t("common.delete")}>
              <MinusCircleOutlined
                style={{ color: token.colorError, fontSize: 12 }}
                onClick={(e) => {
                  e.stopPropagation();
                  removeRule(rule.id);
                }}
              />
            </Tooltip>
          ),
          children: (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {/* 规则名 */}
              <div>
                <Text type="secondary" style={{ fontSize: 10, display: "block", marginBottom: 2 }}>
                  {t("workflow.validation.ruleName")}
                </Text>
                <Input
                  size="small"
                  value={rule.name}
                  onChange={(e) => updateRule(rule.id, { name: e.target.value })}
                  placeholder={t("workflow.validation.ruleNamePlaceholder")}
                />
              </div>

              {/* 规则类型 */}
              <div>
                <Text type="secondary" style={{ fontSize: 10, display: "block", marginBottom: 2 }}>
                  {t("workflow.validation.ruleType")}
                </Text>
                <Select
                  size="small"
                  style={{ width: "100%" }}
                  value={rule.rule_type}
                  onChange={(v) => updateRule(rule.id, { rule_type: v as RuleType })}
                  options={RULE_TYPE_OPTIONS}
                />
              </div>

              {/* 描述 */}
              <div>
                <Text type="secondary" style={{ fontSize: 10, display: "block", marginBottom: 2 }}>
                  {t("workflow.validation.description")}
                </Text>
                <Input
                  size="small"
                  value={rule.description}
                  onChange={(e) => updateRule(rule.id, { description: e.target.value })}
                  placeholder={t("workflow.validation.descriptionPlaceholder")}
                />
              </div>

              {/* JSON Schema 编辑器 */}
              {rule.rule_type === "json_schema" && (
                <div>
                  <Text type="secondary" style={{ fontSize: 10, display: "block", marginBottom: 2 }}>
                    {t("workflow.validation.schema")}
                  </Text>
                  <TextArea
                    rows={6}
                    size="small"
                    value={rule.schema}
                    onChange={(e) => updateRule(rule.id, { schema: e.target.value })}
                    style={{ fontFamily: "monospace", fontSize: 11 }}
                  />
                </div>
              )}

              {/* 路径断言 */}
              {rule.rule_type === "path_assertion" && (
                <>
                  <div>
                    <Text type="secondary" style={{ fontSize: 10, display: "block", marginBottom: 2 }}>
                      {t("workflow.validation.sourceNode")}
                    </Text>
                    <Select
                      size="small"
                      style={{ width: "100%" }}
                      value={rule.source_node_id}
                      onChange={(v) => updateRule(rule.id, { source_node_id: v })}
                      options={nodeOptions}
                      placeholder={t("workflow.validation.selectNode")}
                    />
                  </div>
                  <div>
                    <Text type="secondary" style={{ fontSize: 10, display: "block", marginBottom: 2 }}>
                      {t("workflow.validation.expectedPath")}
                    </Text>
                    <Input
                      size="small"
                      value={rule.expected_path}
                      onChange={(e) => updateRule(rule.id, { expected_path: e.target.value })}
                      placeholder="e.g. output.status"
                    />
                  </div>
                  <div>
                    <Text type="secondary" style={{ fontSize: 10, display: "block", marginBottom: 2 }}>
                      {t("workflow.validation.assertionOp")}
                    </Text>
                    <Select
                      size="small"
                      style={{ width: "100%" }}
                      value={rule.assertion_op}
                      onChange={(v) => updateRule(rule.id, { assertion_op: v as AssertionOp })}
                      options={ASSERTION_OPTS}
                    />
                  </div>
                  <div>
                    <Text type="secondary" style={{ fontSize: 10, display: "block", marginBottom: 2 }}>
                      {t("workflow.validation.expectedValue")}
                    </Text>
                    <Input
                      size="small"
                      value={rule.expected_value}
                      onChange={(e) => updateRule(rule.id, { expected_value: e.target.value })}
                      placeholder={t("workflow.validation.expectedValuePlaceholder")}
                    />
                  </div>
                </>
              )}

              {/* 阈值规则 */}
              {rule.rule_type === "threshold" && (
                <>
                  <div>
                    <Text type="secondary" style={{ fontSize: 10, display: "block", marginBottom: 2 }}>
                      {t("workflow.validation.metric")}
                    </Text>
                    <Select
                      size="small"
                      style={{ width: "100%" }}
                      value={rule.metric}
                      onChange={(v) => updateRule(rule.id, { metric: v as ThresholdMetric })}
                      options={METRIC_OPTS}
                    />
                  </div>
                  <Space>
                    <div style={{ flex: 1 }}>
                      <Text type="secondary" style={{ fontSize: 10, display: "block", marginBottom: 2 }}>
                        {t("workflow.validation.maxValue")}
                      </Text>
                      <Input
                        size="small"
                        type="number"
                        value={rule.max_value ?? ""}
                        onChange={(e) => updateRule(rule.id, { max_value: parseInt(e.target.value) || undefined })}
                      />
                    </div>
                    <div style={{ flex: 1 }}>
                      <Text type="secondary" style={{ fontSize: 10, display: "block", marginBottom: 2 }}>
                        {t("workflow.validation.minValue")}
                      </Text>
                      <Input
                        size="small"
                        type="number"
                        value={rule.min_value ?? ""}
                        onChange={(e) => updateRule(rule.id, { min_value: parseInt(e.target.value) || undefined })}
                      />
                    </div>
                  </Space>
                </>
              )}
            </div>
          ),
        }))}
      />

      {/* 添加规则按钮 */}
      <Button
        type="dashed"
        size="small"
        icon={<PlusOutlined />}
        onClick={addRule}
        style={{ width: "100%", marginTop: 8 }}
      >
        {t("workflow.validation.addRule")}
      </Button>

      {onClose && (
        <Button
          size="small"
          onClick={onClose}
          style={{ width: "100%", marginTop: 8 }}
        >
          {t("common.close")}
        </Button>
      )}
    </div>
  );
}
