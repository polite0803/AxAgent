// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 行业工作流向导 — 可扩展的配置化步骤引擎
 *
 * 核心设计：
 * - 步骤数组由 IndustryWorkflow.wizardSteps 配置驱动
 * - 内置 5 种步骤类型：form / confirm / execute / result / custom
 * - 支持 canSkip 动态跳过、validate 自定义校验、render 完全自定义渲染
 * - 未配置 wizardSteps 时自动生成默认 4 步流程（兼容现有工作流）
 */

import { CheckCircleFilled, LoadingOutlined } from "@ant-design/icons";
import { Alert, Button, Input, InputNumber, Modal, Steps, Tag, Typography } from "antd";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type { IndustryWorkflow, WizardContext, WizardStep, WorkflowInputField } from "./types";
import type { useIndustryData } from "./useIndustryData";

const { Text, Title } = Typography;

type IndustryDataHook = ReturnType<typeof useIndustryData>;

interface WorkflowWizardProps {
  open: boolean;
  workflow: IndustryWorkflow | null;
  data: IndustryDataHook;
  onClose: () => void;
}

/** 生成默认步骤配置（未指定 wizardSteps 时使用） */
function createDefaultSteps(workflow: IndustryWorkflow): WizardStep[] {
  const hasInputs = (workflow.inputFields?.length ?? 0) > 0;
  const steps: WizardStep[] = [];

  if (hasInputs) {
    steps.push({
      id: "config",
      title: "opc.industry.wizard.steps.config",
      description: "opc.industry.wizard.steps.configDesc",
      type: "form",
      fields: workflow.inputFields,
      validate: (ctx) => {
        const required = (workflow.inputFields ?? []).filter((f) => f.required);
        return required.every((f) => {
          const v = ctx.values[f.key];
          return v !== undefined && v !== null && v !== "";
        });
      },
      nextLabel: "opc.industry.wizard.next",
      showBack: false,
    });
  }

  steps.push({
    id: "confirm",
    title: "opc.industry.wizard.steps.confirm",
    description: "opc.industry.wizard.steps.confirmDesc",
    type: "confirm",
    canSkip: () => !hasInputs,
    nextLabel: "opc.industry.wizard.confirmExecute",
    prevLabel: "opc.industry.wizard.back",
    showBack: hasInputs,
  });

  steps.push({
    id: "execute",
    title: "opc.industry.wizard.steps.execute",
    description: "opc.industry.wizard.steps.executeDesc",
    type: "execute",
    showBack: false,
  });

  steps.push({
    id: "result",
    title: "opc.industry.wizard.steps.result",
    type: "result",
    showBack: false,
  });

  return steps;
}

/** 渲染 form 类型步骤 */
function renderFormStep(
  fields: WorkflowInputField[],
  values: Record<string, unknown>,
  onChange: (key: string, value: unknown) => void,
  t: (key: string, params?: Record<string, unknown>) => string,
) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16, padding: "8px 0" }}>
      {fields.map((field) => (
        <div key={field.key}>
          <Text type="secondary" style={{ fontSize: 13, display: "block", marginBottom: 6 }}>
            {t(field.label)}
            {field.required ? " *" : ""}
          </Text>
          {field.type === "textarea"
            ? (
              <Input.TextArea
                rows={3}
                placeholder={field.placeholder ? t(field.placeholder) : undefined}
                value={(values[field.key] as string) ?? ""}
                onChange={(e) => onChange(field.key, e.target.value)}
              />
            )
            : field.type === "number"
            ? (
              <InputNumber
                style={{ width: "100%" }}
                placeholder={field.placeholder ? t(field.placeholder) : undefined}
                value={(values[field.key] as number) ?? undefined}
                onChange={(v) => onChange(field.key, v)}
              />
            )
            : (
              <Input
                placeholder={field.placeholder ? t(field.placeholder) : undefined}
                value={(values[field.key] as string) ?? ""}
                onChange={(e) => onChange(field.key, e.target.value)}
              />
            )}
        </div>
      ))}
    </div>
  );
}

/** 渲染 confirm 类型步骤 */
function renderConfirmStep(
  workflow: IndustryWorkflow,
  values: Record<string, unknown>,
  t: (key: string, params?: Record<string, unknown>) => string,
) {
  const fields = workflow.inputFields ?? [];
  return (
    <div style={{ padding: "8px 0" }}>
      <Title level={5} style={{ marginBottom: 12 }}>
        {workflow.name || workflow.id}
      </Title>
      {workflow.description && (
        <Text type="secondary" style={{ display: "block", marginBottom: 16 }}>
          {workflow.description}
        </Text>
      )}
      {workflow.version && (
        <Tag color="blue" style={{ marginBottom: 16 }}>
          {t("opc.industry.wizard.version", { version: workflow.version })}
        </Tag>
      )}
      {fields.length > 0 && (
        <div
          style={{
            background: "var(--color-bg-soft, #f5f5f5)",
            borderRadius: 8,
            padding: 16,
          }}
        >
          <Text strong style={{ display: "block", marginBottom: 12 }}>
            {t("opc.industry.wizard.paramsTitle")}
          </Text>
          {fields.map((field) => {
            const rawValue = values[field.key];
            const displayValue = rawValue !== undefined && rawValue !== ""
              ? String(rawValue)
              : t("opc.industry.wizard.notFilled");
            return (
              <div
                key={field.key}
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  padding: "4px 0",
                  borderBottom: "1px dashed var(--color-border-soft, #e8e8e8)",
                }}
              >
                <Text type="secondary">{t(field.label)}</Text>
                <Text>{displayValue}</Text>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

/** 渲染 execute 类型步骤 */
function renderExecuteStep(t: (key: string) => string) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        padding: "48px 0",
        gap: 16,
      }}
    >
      <LoadingOutlined style={{ fontSize: 48, color: "var(--color-primary)" }} />
      <Text style={{ fontSize: 16 }}>{t("opc.industry.wizard.executing")}</Text>
    </div>
  );
}

/** 渲染 result 类型步骤 */
function renderResultStep(
  status: "success" | "failed" | null,
  message: string,
  t: (key: string) => string,
) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        padding: "32px 0",
        gap: 16,
      }}
    >
      {status === "success"
        ? (
          <>
            <CheckCircleFilled style={{ fontSize: 64, color: "#52c41a" }} />
            <Title level={4} style={{ margin: 0, color: "#52c41a" }}>
              {t("opc.industry.wizard.resultSuccess")}
            </Title>
            <Alert type="success" showIcon message={message} style={{ width: "100%" }} />
          </>
        )
        : status === "failed"
        ? (
          <>
            <CheckCircleFilled
              style={{ fontSize: 64, color: "#ff4d4f", transform: "rotate(180deg)" }}
            />
            <Title level={4} style={{ margin: 0, color: "#ff4d4f" }}>
              {t("opc.industry.wizard.resultFailed")}
            </Title>
            <Alert type="error" showIcon message={message} style={{ width: "100%" }} />
          </>
        )
        : null}
    </div>
  );
}

export function WorkflowWizard({ open, workflow, data, onClose }: WorkflowWizardProps) {
  const { t } = useTranslation();

  const [step, setStep] = useState(0);
  const [formValues, setFormValues] = useState<Record<string, unknown>>({});
  const [executing, setExecuting] = useState(false);
  const [resultStatus, setResultStatus] = useState<"success" | "failed" | null>(null);
  const [resultMessage, setResultMessage] = useState("");

  // 步骤数组（支持动态跳过）
  const allSteps = useMemo(() => {
    if (!workflow) { return [] as WizardStep[]; }
    return workflow.wizardSteps ?? createDefaultSteps(workflow);
  }, [workflow]);

  // 计算有效步骤（过滤掉 canSkip 返回 true 的步骤）
  const effectiveSteps = useMemo(() => {
    if (!workflow) { return [] as WizardStep[]; }
    const ctx: WizardContext = {
      values: formValues,
      setValue: (k, v) => setFormValues((p) => ({ ...p, [k]: v })),
      setValues: (v) => setFormValues(v),
      stepIndex: step,
      workflow,
      execute: async () => {},
      executing,
      resultStatus,
      resultMessage,
      close: onClose,
    };
    return allSteps.filter((s) => !s.canSkip?.(ctx));
  }, [allSteps, workflow, formValues, step, executing, resultStatus, resultMessage, onClose]);

  // 重置状态
  useEffect(() => {
    if (open && workflow) {
      setStep(0);
      setFormValues({});
      setExecuting(false);
      setResultStatus(null);
      setResultMessage("");
    }
  }, [open, workflow]);

  // 关闭时完全重置
  useEffect(() => {
    if (!open) {
      setStep(0);
      setFormValues({});
      setExecuting(false);
      setResultStatus(null);
      setResultMessage("");
    }
  }, [open]);

  // 确保当前步骤索引在有效范围内
  useEffect(() => {
    if (step >= effectiveSteps.length && effectiveSteps.length > 0) {
      setStep(effectiveSteps.length - 1);
    }
  }, [step, effectiveSteps.length]);

  if (!workflow) { return null; }

  const currentStep = effectiveSteps[step];

  const setValue = (key: string, value: unknown) => {
    setFormValues((prev) => ({ ...prev, [key]: value }));
  };

  const ctx: WizardContext = {
    values: formValues,
    setValue,
    setValues: setFormValues,
    stepIndex: step,
    workflow,
    execute: async () => {
      setExecuting(true);
      try {
        const result = await data.executeWorkflow(workflow.id, formValues);
        setExecuting(false);
        if (result.status === "completed" || result.status === "success") {
          setResultStatus("success");
          setResultMessage(t("opc.industry.wizard.executeSuccess", { id: workflow.id }));
        } else {
          setResultStatus("failed");
          setResultMessage(t("opc.industry.wizard.executeFailed", { id: workflow.id }));
        }
      } catch {
        setExecuting(false);
        setResultStatus("failed");
        setResultMessage(t("opc.industry.wizard.executeFailed", { id: workflow.id }));
      }
    },
    executing,
    resultStatus,
    resultMessage,
    close: onClose,
  };

  const canProceed = currentStep?.validate ? currentStep.validate(ctx) : true;
  const isExecuteStep = currentStep?.type === "execute";
  const isResultStep = currentStep?.type === "result";
  const showBack = currentStep?.showBack ?? (!isExecuteStep && !isResultStep);
  const isLastStep = step === effectiveSteps.length - 1;

  const goNext = async () => {
    if (isExecuteStep) {
      await ctx.execute();
      setStep((s) => s + 1);
    } else if (isLastStep && isResultStep) {
      onClose();
    } else {
      setStep((s) => s + 1);
    }
  };

  const goBack = () => {
    if (step > 0) {
      setStep((s) => s - 1);
    }
  };

  const renderStepContent = () => {
    if (!currentStep) { return null; }

    switch (currentStep.type) {
      case "form":
        return renderFormStep(
          currentStep.fields ?? [],
          formValues,
          setValue,
          t,
        );
      case "confirm":
        return renderConfirmStep(workflow, formValues, t);
      case "execute":
        return renderExecuteStep(t);
      case "result":
        return renderResultStep(resultStatus, resultMessage, t);
      case "custom":
        return currentStep.render ? currentStep.render(ctx) : null;
      default:
        return null;
    }
  };

  const nextLabel = currentStep?.nextLabel
    ? t(currentStep.nextLabel)
    : isLastStep
    ? t("opc.industry.wizard.finish")
    : t("opc.industry.wizard.next");

  return (
    <Modal
      open={open}
      onCancel={onClose}
      title={t("opc.industry.wizard.title", {
        name: workflow.name || workflow.id,
      })}
      width={600}
      footer={
        <div style={{ display: "flex", justifyContent: "space-between" }}>
          <div>
            {showBack && step > 0 && (
              <Button onClick={goBack} style={{ marginRight: 8 }}>
                {currentStep?.prevLabel
                  ? t(currentStep.prevLabel)
                  : t("opc.industry.wizard.back")}
              </Button>
            )}
          </div>
          <div>
            {!isResultStep && (
              <Button
                type="primary"
                onClick={() => {
                  void goNext();
                }}
                disabled={!canProceed}
                loading={isExecuteStep || executing}
              >
                {nextLabel}
              </Button>
            )}
            {isResultStep && (
              <Button type="primary" onClick={onClose}>
                {t("opc.industry.wizard.finish")}
              </Button>
            )}
          </div>
        </div>
      }
      destroyOnHidden
    >
      <Steps
        current={step}
        direction="vertical"
        size="small"
        style={{ marginBottom: 24 }}
        items={effectiveSteps.map((s, idx) => ({
          title: t(s.title),
          description: s.description ? t(s.description) : undefined,
          status: idx === step
            ? isExecuteStep
              ? "process"
              : "wait"
            : idx < step
            ? "finish"
            : "wait",
        }))}
      />
      {renderStepContent()}
    </Modal>
  );
}
