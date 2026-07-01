// SPDX-License-Identifier: AGPL-3.0-only
/* eslint-disable react-refresh/only-export-components */

import { useSchemaRenderer } from "@/components/dynamicUI/SchemaRenderContext";
import { evaluateConditions } from "@/lib/dynamicUI/ConditionalRenderer";
import { executeActions } from "@/lib/dynamicUI/EventHandlerEngine";
import type { DynamicAction, DynamicUIProps } from "@/types";
import { Button, Form } from "antd";
import { createContext, useContext, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const InFormContext = createContext<boolean>(false);

export function useInFormContext(): boolean {
  return useContext(InFormContext);
}

export const FormRenderer: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
  onAction,
}) => {
  const { t } = useTranslation();
  const [form] = Form.useForm();
  const [submitting, setSubmitting] = useState(false);
  const [formValues, setFormValues] = useState<Record<string, unknown>>({});
  const { renderSchema } = useSchemaRenderer();

  const {
    layout = "vertical",
    submitText = t("dynamicUI.submit"),
    resetText,
  } = schema.props as {
    layout?: "horizontal" | "vertical" | "inline";
    submitText?: string;
    resetText?: string;
  };

  const mergedDataContext = useMemo(() => ({
    ...(dataContext || {}),
    ...formValues,
  }), [dataContext, formValues]);

  const initialValues = useMemo(() => {
    const init: Record<string, unknown> = {};
    if (schema.children) {
      for (const child of schema.children) {
        const props = child.props as Record<string, unknown> | undefined;
        const name = props?.name as string | undefined;
        if (name && props?.defaultValue !== undefined) {
          init[name] = props.defaultValue;
        }
      }
    }
    return init;
  }, [schema.children]);

  const appliedDataRef = useRef<Record<string, unknown>>({});
  useEffect(() => {
    if (!dataContext || !schema.children) {
      return;
    }
    const toSet: Record<string, unknown> = {};
    let changed = false;
    for (const child of schema.children) {
      const props = child.props as Record<string, unknown> | undefined;
      const name = props?.name as string | undefined;
      if (!name) {
        continue;
      }
      if (name in dataContext && appliedDataRef.current[name] !== dataContext[name]) {
        toSet[name] = dataContext[name];
        appliedDataRef.current[name] = dataContext[name];
        changed = true;
      }
    }
    if (changed) {
      form.setFieldsValue(toSet);
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setFormValues((prev) => ({ ...prev, ...toSet }));
    }
  }, [dataContext, schema.children, form]);

  const visibleChildren = useMemo(() => {
    if (!schema.children || schema.children.length === 0) {
      return [];
    }
    return schema.children.filter((child) => evaluateConditions(child.conditionalDisplay, mergedDataContext));
  }, [schema.children, mergedDataContext]);

  const handleSubmit = async (values: Record<string, unknown>) => {
    setSubmitting(true);
    try {
      const submitHandler = schema.events?.find(
        (e) => e.trigger === "onSubmit",
      );
      if (submitHandler) {
        const submitContext = {
          ...(dataContext || {}),
          ...values,
          formValues: values,
        };
        const enrichedActions: DynamicAction[] = submitHandler.actions.map((action) => ({
          ...action,
          config: {
            ...(action.config as Record<string, unknown>),
            formValues: values,
            values,
          },
        }));
        await executeActions(enrichedActions, { context: submitContext, onAction });
      } else if (onAction) {
        onAction({
          type: "store",
          config: { formValues: values, values },
        });
      }
    } finally {
      setSubmitting(false);
    }
  };

  const handleReset = () => {
    form.resetFields();
    setFormValues({});
  };

  return (
    <InFormContext.Provider value={true}>
      <Form
        form={form}
        layout={layout}
        initialValues={initialValues}
        onFinish={handleSubmit}
        onValuesChange={(_changed, allValues) => setFormValues(allValues)}
        style={schema.style as React.CSSProperties}
      >
        {visibleChildren.map((child) => <div key={child.id}>{renderSchema(child, mergedDataContext)}</div>)}

        <Form.Item>
          <Button type="primary" htmlType="submit" loading={submitting}>
            {submitText}
          </Button>
          {resetText
            ? (
              <Button style={{ marginLeft: 8 }} onClick={handleReset}>
                {resetText}
              </Button>
            )
            : null}
        </Form.Item>
      </Form>
    </InFormContext.Provider>
  );
};
