// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps, DynamicAction } from "@/types";
import { Form, Button } from "antd";
import { useState } from "react";
import { evaluateConditions } from "@/lib/dynamicUI/ConditionalRenderer";

/**
 * 表单渲染器，基于 Ant Design Form。
 * 动态渲染表单字段（children 中的 Input、Select、DatePicker 等），
 * 支持 onSubmit 事件（触发 onAction 回调）。
 */
export const FormRenderer: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
  onAction,
}) => {
  const [form] = Form.useForm();
  const [submitting, setSubmitting] = useState(false);

  const {
    layout = "vertical",
    submitText = "提交",
    resetText,
  } = schema.props as {
    layout?: "horizontal" | "vertical" | "inline";
    submitText?: string;
    resetText?: string;
  };

  const handleSubmit = async (values: Record<string, unknown>) => {
    setSubmitting(true);
    try {
      // 找到 onSubmit 事件并触发
      const submitHandler = schema.events?.find(
        (e) => e.trigger === "onSubmit",
      );
      if (submitHandler && onAction) {
        for (const action of submitHandler.actions) {
          // 将表单值合并到 action config 中
          const enrichedAction: DynamicAction = {
            ...action,
            config: {
              ...(action.config as Record<string, unknown>),
              formValues: values,
            },
          };
          onAction(enrichedAction);
        }
      }
    } finally {
      setSubmitting(false);
    }
  };

  const handleReset = () => {
    form.resetFields();
  };

  // 过滤条件不满足的子组件
  const visibleChildren = (schema.children || []).filter((child) => {
    if (!child.conditionalDisplay || child.conditionalDisplay.length === 0) {
      return true;
    }
    return evaluateConditions(child.conditionalDisplay, {
      ...(dataContext || {}),
      ...form.getFieldsValue(),
    });
  });

  return (
    <Form
      form={form}
      layout={layout}
      onFinish={handleSubmit}
      style={schema.style as React.CSSProperties}
    >
      {visibleChildren.map((child) =>
        renderFormField(child, dataContext, onAction),
      )}

      <Form.Item>
        <Button type="primary" htmlType="submit" loading={submitting}>
          {submitText}
        </Button>
        {resetText ? (
          <Button style={{ marginLeft: 8 }} onClick={handleReset}>
            {resetText}
          </Button>
        ) : null}
      </Form.Item>
    </Form>
  );
};

function renderFormField(
  child: DynamicUIProps["schema"],
  dataContext: Record<string, unknown> | undefined,
  onAction: DynamicUIProps["onAction"],
): React.ReactNode {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const DynamicUIRenderer =
    require("../DynamicUIRenderer").DynamicUIRenderer as React.ComponentType<DynamicUIProps>;
  return (
    <DynamicUIRenderer
      key={child.id}
      schema={child}
      dataContext={dataContext}
      onAction={onAction}
    />
  );
}

export default FormRenderer;
