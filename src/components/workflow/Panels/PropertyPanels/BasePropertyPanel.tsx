import { Divider, Input, InputNumber, Select, Switch, theme } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { WorkflowNode } from "../../types";

interface CircuitBreakerConfig {
  failure_threshold: number;
  reset_timeout_ms: number;
}

/** 运行时动态注入的节点属性（不在 WorkflowNode 联合类型中定义） */
type WorkflowNodeWithExtras = WorkflowNode & {
  circuit_breaker?: CircuitBreakerConfig;
  _breakpoint?: boolean;
};

interface BasePropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const BasePropertyPanel: React.FC<BasePropertyPanelProps> = ({
  node,
  onUpdate,
}) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.title")}
        </label>
        <Input
          id="base-property-panel-input-80"
          value={node.title}
          onChange={(e) => onUpdate({ title: e.target.value })}
          size="small"
        />
      </div>

      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.description")}
        </label>
        <Input.TextArea
          id="base-property-panel-input-textarea-81"
          value={node.description || ""}
          onChange={(e) => onUpdate({ description: e.target.value })}
          rows={2}
          size="small"
        />
      </div>

      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          {t("workflow.props.enabled")}
        </label>
        <Switch
          size="small"
          checked={node.enabled}
          onChange={(checked) => onUpdate({ enabled: checked })}
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />

      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.retryPolicy")}
        </label>
        <Switch
          size="small"
          checked={node.retry.enabled}
          onChange={(enabled) => onUpdate({ retry: { ...node.retry, enabled } })}
        />
        {node.retry.enabled && (
          <div
            style={{
              marginTop: 8,
              display: "flex",
              flexDirection: "column",
              gap: 8,
            }}
          >
            <div>
              <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
                {t("workflow.props.maxRetries")}
              </label>
              <InputNumber
                id="base-property-panel-inputnumber-82"
                value={node.retry.max_retries}
                onChange={(value) =>
                  onUpdate({
                    retry: { ...node.retry, max_retries: value || 3 },
                  })}
                min={1}
                max={10}
                size="small"
                style={{ width: "100%" }}
              />
            </div>
            <div>
              <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
                {t("workflow.props.backoffStrategy")}
              </label>
              <Select
                value={node.retry.backoff_type}
                onChange={(backoff_type) => onUpdate({ retry: { ...node.retry, backoff_type } })}
                size="small"
                style={{ width: "100%" }}
                options={[
                  { value: "Linear", label: t("workflow.props.linear") },
                  {
                    value: "Exponential",
                    label: t("workflow.props.exponential"),
                  },
                  { value: "Fixed", label: t("workflow.props.fixed") },
                ]}
              />
            </div>
            <div>
              <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
                {t("workflow.props.baseDelayMs")}
              </label>
              <InputNumber
                id="base-property-panel-inputnumber-83"
                value={node.retry.base_delay_ms}
                onChange={(value) =>
                  onUpdate({
                    retry: { ...node.retry, base_delay_ms: value || 1000 },
                  })}
                min={100}
                max={60000}
                size="small"
                style={{ width: "100%" }}
              />
            </div>
            <div>
              <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
                {t("workflow.props.maxDelayMs")}
              </label>
              <InputNumber
                value={node.retry.max_delay_ms}
                onChange={(value) =>
                  onUpdate({
                    retry: { ...node.retry, max_delay_ms: value ?? 30000 },
                  })}
                min={1000}
                max={300000}
                size="small"
                style={{ width: "100%" }}
              />
            </div>
          </div>
        )}
      </div>

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />

      {/* 熔断器 */}
      <div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
          <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
            ⚡ {t("workflow.props.circuitBreaker")}
          </label>
          <Switch
            size="small"
            checked={(node as WorkflowNodeWithExtras).circuit_breaker != null}
            onChange={(enabled) =>
              onUpdate({
                circuit_breaker: enabled
                  ? { failure_threshold: 3, reset_timeout_ms: 60000 }
                  : undefined,
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
              } as any)}
          />
        </div>
        {(node as WorkflowNodeWithExtras).circuit_breaker && (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <div>
              <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
                {t("workflow.props.failureThreshold")}
              </label>
              <InputNumber
                value={(node as WorkflowNodeWithExtras).circuit_breaker.failure_threshold ?? 3}
                onChange={(v) =>
                  onUpdate({
                    circuit_breaker: {
                      ...(node as WorkflowNodeWithExtras).circuit_breaker,
                      failure_threshold: v ?? 3,
                    },
                    // eslint-disable-next-line @typescript-eslint/no-explicit-any
                  } as any)}
                min={1}
                max={20}
                size="small"
                style={{ width: "100%" }}
              />
            </div>
            <div>
              <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
                {t("workflow.props.resetTimeoutMs")}
              </label>
              <InputNumber
                value={(node as WorkflowNodeWithExtras).circuit_breaker.reset_timeout_ms ?? 60000}
                onChange={(v) =>
                  onUpdate({
                    circuit_breaker: {
                      ...(node as WorkflowNodeWithExtras).circuit_breaker,
                      reset_timeout_ms: v ?? 60000,
                    },
                    // eslint-disable-next-line @typescript-eslint/no-explicit-any
                  } as any)}
                min={5000}
                max={600000}
                step={5000}
                size="small"
                style={{ width: "100%" }}
              />
              <div style={{ fontSize: 11, color: token.colorTextQuaternary, marginTop: 2 }}>
                {t("workflow.props.resetTimeoutHint")}
              </div>
            </div>
          </div>
        )}
      </div>

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />

      <div>
        <label
          style={{
            display: "block",
            color: token.colorTextTertiary,
            fontSize: 12,
            marginBottom: 4,
          }}
        >
          {t("workflow.props.timeoutSeconds")}
        </label>
        <InputNumber
          id="base-property-panel-inputnumber-84"
          value={node.timeout}
          onChange={(value) => onUpdate({ timeout: value ?? undefined })}
          min={1}
          placeholder={t("workflow.props.notSet")}
          size="small"
          style={{ width: "100%" }}
        />
      </div>

      <Divider style={{ margin: "8px 0", borderColor: token.colorBorderSecondary }} />

      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <label style={{ color: token.colorTextTertiary, fontSize: 12 }}>
          🔴 {t("workflow.props.breakpoint")}
        </label>
        <Switch
          size="small"
          checked={(node as WorkflowNodeWithExtras)._breakpoint ?? false}
          onChange={(checked) =>
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            onUpdate({ _breakpoint: checked || undefined } as any)}
        />
      </div>
    </div>
  );
};
