// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 行业 Tab 内容组件 — 渲染指定 tab 的 actions 和 workflows
 */

import { Alert, Button, Card, Empty, Input, InputNumber, message, Tag, Typography } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type { IndustryConfig, IndustryTab } from "./types";
import { useIndustryData } from "./useIndustryData";

const { Text } = Typography;

/** Tab 内容属性 */
interface IndustryTabContentProps {
  industryId: string;
  config: IndustryConfig;
  tabKey: string;
}

/**
 * 从配置中查找指定 tab
 */
function findTab(config: IndustryConfig, tabKey: string): IndustryTab | undefined {
  return config.tabs?.find((t) => t.key === tabKey);
}

/**
 * 行业 Tab 内容组件
 */
export function IndustryTabContent({ industryId, config, tabKey }: IndustryTabContentProps) {
  const { t } = useTranslation();
  const data = useIndustryData(industryId);

  const tab = useMemo(() => findTab(config, tabKey), [config, tabKey]);

  // 工作流用户输入值：key = "{wfId}.{fieldKey}"
  const [inputValues, setInputValues] = useState<Record<string, unknown>>({});

  const setInputValue = (wfId: string, fieldKey: string, value: unknown) => {
    setInputValues((prev) => ({ ...prev, [`${wfId}.${fieldKey}`]: value }));
  };

  const getInputValues = (wfId: string): Record<string, unknown> => {
    const result: Record<string, unknown> = {};
    const prefix = `${wfId}.`;
    for (const [k, v] of Object.entries(inputValues)) {
      if (k.startsWith(prefix)) {
        result[k.slice(prefix.length)] = v;
      }
    }
    return result;
  };

  const handleAction = (actionKey: string) => {
    message.info(t("opc.industry.tab.actionTriggered", { key: actionKey }));
  };

  const handleExecute = async (workflowId: string, userInput?: Record<string, unknown>) => {
    try {
      const result = await data.executeWorkflow(workflowId, userInput);
      // completed=全部节点完成；success=成功但有节点未执行（如装饰/跳过节点），均视为成功
      if (result.status === "completed" || result.status === "success") {
        message.success(t("opc.industry.tab.executeSuccess", { id: workflowId }));
      } else {
        message.error(t("opc.industry.tab.executeFailed", { id: workflowId }));
      }
    } catch {
      message.error(t("opc.industry.tab.executeFailed", { id: workflowId }));
    }
  };

  if (!tab) {
    return (
      <div style={{ padding: 24 }}>
        <Empty description={t("opc.industry.notFound")} />
      </div>
    );
  }

  return (
    <div style={{ padding: "16px 24px", height: "100%", overflow: "auto" }}>
      {/* Tab 描述 */}
      {tab.description && (
        <Alert
          style={{ marginBottom: 16 }}
          type="info"
          showIcon
          message={tab.description}
        />
      )}

      {/* 操作项 */}
      {tab.actions && tab.actions.length > 0 && (
        <Card
          title={
            <span>
              <strong>{t("opc.industry.tab.actions")}</strong>
              <Text type="secondary" style={{ marginLeft: 8 }}>
                ({tab.actions.length})
              </Text>
            </span>
          }
          style={{ marginBottom: 16 }}
          size="small"
        >
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))",
              gap: 12,
            }}
          >
            {tab.actions.map((action) => (
              <div
                key={action.key}
                onClick={() => handleAction(action.key)}
                style={{
                  cursor: "pointer",
                  padding: 12,
                  border: "1px solid var(--color-border)",
                  borderRadius: 8,
                  transition: "all 0.2s",
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "center",
                  gap: 6,
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.borderColor = "var(--color-primary)";
                  e.currentTarget.style.boxShadow = "0 2px 8px rgba(0,0,0,0.1)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.borderColor = "var(--color-border)";
                  e.currentTarget.style.boxShadow = "none";
                }}
              >
                <div style={{ fontSize: 28 }}>{action.icon}</div>
                <Text strong style={{ fontSize: 13 }}>
                  {action.label || action.key}
                </Text>
                <Tag
                  color={action.type === "workflow" ? "blue" : "green"}
                  style={{ fontSize: 11 }}
                >
                  {action.type === "workflow"
                    ? t("opc.industry.tab.type.workflow")
                    : t("opc.industry.tab.type.conversation")}
                </Tag>
              </div>
            ))}
          </div>
        </Card>
      )}

      {/* 工作流列表 */}
      {tab.workflows && tab.workflows.length > 0 && (
        <Card
          title={
            <span>
              <strong>{t("opc.industry.tab.workflows")}</strong>
              <Text type="secondary" style={{ marginLeft: 8 }}>
                ({tab.workflows.length})
              </Text>
            </span>
          }
          style={{ marginBottom: 16 }}
          size="small"
        >
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {tab.workflows.map((wf) => {
              const hasInputs = wf.inputFields && wf.inputFields.length > 0;
              return (
                <div
                  key={wf.id}
                  style={{
                    padding: 12,
                    border: "1px solid var(--color-border)",
                    borderRadius: 6,
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      marginBottom: hasInputs ? 8 : 0,
                    }}
                  >
                    <div>
                      <Text strong>{wf.name || wf.id}</Text>
                      {wf.description && (
                        <div>
                          <Text type="secondary" style={{ fontSize: 12 }}>
                            {wf.description}
                          </Text>
                        </div>
                      )}
                    </div>
                    <Button
                      size="small"
                      type="primary"
                      loading={data.workflowExecuting}
                      onClick={() =>
                        handleExecute(
                          wf.id,
                          hasInputs ? getInputValues(wf.id) : undefined,
                        )}
                    >
                      {t("opc.industry.tab.execute")}
                    </Button>
                  </div>
                  {hasInputs && (
                    <div
                      style={{
                        display: "grid",
                        gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
                        gap: 8,
                        marginTop: 4,
                      }}
                    >
                      {wf.inputFields!.map((field) => (
                        <div key={field.key}>
                          <Text
                            type="secondary"
                            style={{ fontSize: 12, display: "block", marginBottom: 2 }}
                          >
                            {t(field.label)}
                            {field.required ? " *" : ""}
                          </Text>
                          {field.type === "textarea"
                            ? (
                              <Input.TextArea
                                rows={2}
                                placeholder={field.placeholder ? t(field.placeholder) : undefined}
                                value={(inputValues[`${wf.id}.${field.key}`] as string)
                                  ?? field.default
                                  ?? ""}
                                onChange={(e) => setInputValue(wf.id, field.key, e.target.value)}
                              />
                            )
                            : field.type === "number"
                            ? (
                              <InputNumber
                                style={{ width: "100%" }}
                                placeholder={field.placeholder ? t(field.placeholder) : undefined}
                                value={(inputValues[`${wf.id}.${field.key}`] as number)
                                  ?? (field.default ? Number(field.default) : undefined)}
                                onChange={(v) => setInputValue(wf.id, field.key, v)}
                              />
                            )
                            : (
                              <Input
                                placeholder={field.placeholder ? t(field.placeholder) : undefined}
                                value={(inputValues[`${wf.id}.${field.key}`] as string)
                                  ?? field.default
                                  ?? ""}
                                onChange={(e) => setInputValue(wf.id, field.key, e.target.value)}
                              />
                            )}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </Card>
      )}

      {/* 空状态 */}
      {(!tab.actions || tab.actions.length === 0)
        && (!tab.workflows || tab.workflows.length === 0) && <Empty description={t("opc.industry.tab.noContent")} />}
    </div>
  );
}
