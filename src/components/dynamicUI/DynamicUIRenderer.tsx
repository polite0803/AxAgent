// SPDX-License-Identifier: AGPL-3.0-only
/* eslint-disable react-refresh/only-export-components */

import { componentRegistry } from "@/lib/dynamicUI/ComponentRegistry";
import { evaluateConditions } from "@/lib/dynamicUI/ConditionalRenderer";
import { type DataSourceSubscriber, subscribeDataSource } from "@/lib/dynamicUI/DataBindingEngine";
import { executeActions, getLifecycleHandlers, handleEvents } from "@/lib/dynamicUI/EventHandlerEngine";
import { validateSchema } from "@/lib/dynamicUI/SchemaValidator";
import type { DynamicAction, DynamicUIProps, UISchema } from "@/types";
import { Alert, Skeleton } from "antd";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { SchemaRenderContext } from "./SchemaRenderContext";

interface SchemaUpdateEventDetail {
  schemaId: string;
  operation: "replace" | "append" | "remove";
  path?: string;
  newSchema?: UISchema;
  /** 目标 scope；"__global__" 广播到所有 renderer，否则仅匹配 rendererId */
  scope?: string;
}

const NEEDS_CHILD_PREPROCESSING = new Set(["Tabs", "Accordion", "Form"]);

function genRendererId(): string {
  return `dui-${Math.random().toString(36).slice(2, 10)}`;
}

function deepCloneSchema(schema: UISchema): UISchema {
  return JSON.parse(JSON.stringify(schema));
}

function updateSchemaAtPath(
  root: UISchema,
  schemaId: string,
  operation: SchemaUpdateEventDetail["operation"],
  _path: string | undefined,
  newSchema: UISchema | undefined,
): UISchema | null {
  const cloned = deepCloneSchema(root);

  function findAndUpdate(node: UISchema): UISchema | null {
    if (node.id === schemaId) {
      switch (operation) {
        case "replace":
          return newSchema ? deepCloneSchema(newSchema) : node;
        case "append":
          if (newSchema) {
            return {
              ...node,
              children: [...(node.children || []), deepCloneSchema(newSchema)],
            };
          }
          return node;
        case "remove":
          return null;
      }
    }
    if (node.children) {
      const newChildren: UISchema[] = [];
      let changed = false;
      for (const child of node.children) {
        const updated = findAndUpdate(child);
        if (updated === null) {
          changed = true;
          continue;
        }
        if (updated !== child) {
          changed = true;
        }
        newChildren.push(updated);
      }
      if (changed) {
        return { ...node, children: newChildren };
      }
    }
    return node;
  }

  return findAndUpdate(cloned);
}

interface SchemaNodeRendererProps {
  schema: UISchema;
  externalContext?: Record<string, unknown>;
  onAction?: (action: DynamicAction) => void;
  scope: string;
}

const SchemaNodeRenderer = React.memo(function SchemaNodeRenderer({
  schema,
  externalContext,
  onAction,
  scope,
}: SchemaNodeRendererProps) {
  const { t } = useTranslation();
  const [resolvedData, setResolvedData] = useState<unknown>(null);
  const [dataError, setDataError] = useState<Error | null>(null);
  const [dataLoading, setDataLoading] = useState<boolean>(!!schema.dataSource);
  const subscriberRef = useRef<DataSourceSubscriber | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    if (subscriberRef.current) {
      subscriberRef.current.unsubscribe();
      subscriberRef.current = null;
    }

    if (!schema.dataSource) {
      return;
    }

    // eslint-disable-next-line react-hooks/set-state-in-effect
    setDataLoading(true);
    let cancelled = false;

    subscribeDataSource(
      schema.dataSource,
      (data) => {
        if (!cancelled && mountedRef.current) {
          setResolvedData(data);
          setDataError(null);
          setDataLoading(false);
        }
      },
      (error) => {
        if (!cancelled && mountedRef.current) {
          console.error(`[DynamicUI] DataSource "${schema.id}" resolve failed:`, error);
          setDataError(error);
          setDataLoading(false);
        }
      },
    ).then((subscriber) => {
      if (!cancelled && mountedRef.current) {
        subscriberRef.current = subscriber;
      } else {
        subscriber.unsubscribe();
      }
    });

    return () => {
      cancelled = true;
      if (subscriberRef.current) {
        subscriberRef.current.unsubscribe();
        subscriberRef.current = null;
      }
    };
  }, [schema.dataSource, schema.id]);

  const mergedContext = useMemo(() => {
    const base = { ...(externalContext || {}) };
    if (resolvedData && typeof resolvedData === "object") {
      Object.assign(base, resolvedData as Record<string, unknown>);
    }
    return base;
  }, [externalContext, resolvedData]);

  const shouldRender = useMemo(
    () => evaluateConditions(schema.conditionalDisplay || [], mergedContext),
    [schema.conditionalDisplay, mergedContext],
  );

  const mergedContextRef = useRef(mergedContext);
  mergedContextRef.current = mergedContext;

  useEffect(() => {
    if (schema.events) {
      const { onMount } = getLifecycleHandlers(schema.events);
      if (onMount.length > 0) {
        void executeActions(onMount, { context: mergedContextRef.current, onAction, scope });
      }
    }
    return () => {
      if (schema.events) {
        const { onUnmount } = getLifecycleHandlers(schema.events);
        if (onUnmount.length > 0) {
          void executeActions(onUnmount, { context: mergedContextRef.current, onAction, scope });
        }
      }
    };
  }, [schema.events, schema.id, onAction, scope]);

  const entry = componentRegistry.get(schema.type);

  const renderSchema = useCallback(
    (childSchema: UISchema, childContext?: Record<string, unknown>) => (
      <SchemaNodeRenderer
        key={childSchema.id}
        schema={childSchema}
        externalContext={childContext ?? mergedContext}
        onAction={onAction}
        scope={scope}
      />
    ),
    [mergedContext, onAction, scope],
  );

  const contextValue = useMemo(() => ({ renderSchema }), [renderSchema]);

  const processedProps = useMemo(() => {
    const base = {
      ...(entry?.defaultProps || {}),
      ...(schema.props || {}),
    };
    if (resolvedData) {
      base.dataSource = resolvedData;
    }
    if (dataError) {
      base.dataError = dataError;
    }
    if (dataLoading) {
      base.dataLoading = true;
    }

    if (schema.children && schema.children.length > 0) {
      if (schema.type === "Tabs" || schema.type === "Accordion") {
        const itemKey = schema.type === "Tabs" ? t("dynamicUI.tab") : t("dynamicUI.section");
        base.items = schema.children.map((child, index) => {
          const childProps = (child.props as Record<string, unknown>) || {};
          return {
            key: child.id || `${itemKey.toLowerCase()}-${index}`,
            label: (childProps.label as string) || `${itemKey} ${index + 1}`,
            children: renderSchema(child),
          };
        });
      }
    }

    return base;
  }, [
    entry?.defaultProps,
    schema.props,
    schema.type,
    schema.children,
    resolvedData,
    dataError,
    dataLoading,
    renderSchema,
    t,
  ]);

  const childNodes = useMemo(() => {
    if (!schema.children || schema.children.length === 0) {
      return null;
    }
    if (NEEDS_CHILD_PREPROCESSING.has(schema.type)) {
      return null;
    }
    return schema.children.map((child) => renderSchema(child));
  }, [schema.children, schema.type, renderSchema]);

  const eventBindings = useMemo(
    () => handleEvents(schema.events || [], mergedContext, onAction, scope),
    [schema.events, mergedContext, onAction, scope],
  );

  if (!shouldRender) {
    return null;
  }

  if (dataLoading && !resolvedData) {
    return <Skeleton active paragraph={{ rows: 2 }} />;
  }

  if (!entry) {
    return <UnregisteredPlaceholder type={schema.type} />;
  }

  const Component = entry.component;

  return (
    <SchemaRenderContext.Provider value={contextValue}>
      <Component
        schema={{ ...schema, props: processedProps, children: undefined }}
        dataContext={mergedContext}
        onAction={onAction}
        {...eventBindings}
      >
        {childNodes}
      </Component>
    </SchemaRenderContext.Provider>
  );
});

SchemaNodeRenderer.displayName = "SchemaNodeRenderer";

export const DynamicUIRenderer: React.FC<DynamicUIProps> = React.memo(
  ({ schema: initialSchema, dataContext: externalContext, onAction }) => {
    const { t } = useTranslation();
    const [schema, setSchema] = useState<UISchema>(initialSchema);
    const schemaRef = useRef(schema);
    const rendererId = useMemo(() => genRendererId(), []);

    useEffect(() => {
      schemaRef.current = schema;
    }, [schema]);

    useEffect(() => {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setSchema(initialSchema);
    }, [initialSchema]);

    useEffect(() => {
      const handleSchemaUpdate = (event: Event) => {
        const detail = (event as CustomEvent<SchemaUpdateEventDetail>).detail;
        if (detail.scope && detail.scope !== "__global__" && detail.scope !== rendererId) {
          return;
        }
        const currentSchema = schemaRef.current;
        const updated = updateSchemaAtPath(
          currentSchema,
          detail.schemaId,
          detail.operation,
          detail.path,
          detail.newSchema,
        );
        if (updated) {
          setSchema(updated);
        }
      };

      window.addEventListener("dynamic-ui:schema-update", handleSchemaUpdate);
      return () => {
        window.removeEventListener("dynamic-ui:schema-update", handleSchemaUpdate);
      };
    }, [rendererId]);

    const validation = useMemo(() => validateSchema(schema), [schema]);
    if (!validation.valid) {
      return (
        <Alert
          type="error"
          message={t("dynamicUI.schemaValidationFailed")}
          description={
            <ul className="list-disc pl-4 mt-1">
              {validation.errors.slice(0, 5).map((err, i) => (
                <li key={`${err.path}-${i}`}>
                  {err.path}: {err.message}
                </li>
              ))}
              {validation.errors.length > 5
                ? <li>{t("dynamicUI.moreErrors", { count: validation.errors.length - 5 })}</li>
                : null}
            </ul>
          }
          showIcon
        />
      );
    }

    return (
      <SchemaErrorBoundary schemaId={schema.id} t={t}>
        <SchemaNodeRenderer
          schema={schema}
          externalContext={externalContext}
          onAction={onAction}
          scope={rendererId}
        />
      </SchemaErrorBoundary>
    );
  },
);

DynamicUIRenderer.displayName = "DynamicUIRenderer";

function UnregisteredPlaceholder({ type }: { type: string }): React.ReactElement {
  const { t } = useTranslation();
  return (
    <div
      className="border border-yellow-400 bg-yellow-50 dark:bg-yellow-900/20 rounded p-3 my-1"
      role="alert"
    >
      <div className="text-yellow-700 dark:text-yellow-400 font-medium text-sm">
        {t("dynamicUI.unregisteredComponent", { type })}
      </div>
      <div className="text-yellow-600 dark:text-yellow-500 text-xs mt-1">
        {t("dynamicUI.registerHint")}
      </div>
    </div>
  );
}

function ErrorPlaceholder({
  type,
  error,
  t,
}: {
  type: string;
  error: unknown;
  t: (key: string, options?: Record<string, unknown>) => string;
}): React.ReactElement {
  return (
    <Alert
      type="error"
      message={t("dynamicUI.renderFailed", { type })}
      description={
        <pre className="text-xs whitespace-pre-wrap">
          {error instanceof Error ? error.message : String(error)}
        </pre>
      }
      showIcon
    />
  );
}

interface SchemaErrorBoundaryProps {
  schemaId: string;
  children: React.ReactNode;
  t: (key: string, options?: Record<string, unknown>) => string;
}

interface SchemaErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

class SchemaErrorBoundary extends React.Component<
  SchemaErrorBoundaryProps,
  SchemaErrorBoundaryState
> {
  constructor(props: SchemaErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): SchemaErrorBoundaryState {
    return { hasError: true, error };
  }

  render() {
    if (this.state.hasError) {
      return (
        <ErrorPlaceholder
          type={this.props.schemaId}
          error={this.state.error}
          t={this.props.t}
        />
      );
    }
    return this.props.children;
  }
}

// 导出给需要主动触发 schema-update 的外部模块使用
export const DYNAMIC_UI_SCHEMA_UPDATE_EVENT = "dynamic-ui:schema-update";
export const GLOBAL_SCOPE = "__global__";

export function dispatchSchemaUpdate(
  detail: Omit<SchemaUpdateEventDetail, "scope"> & { scope?: string },
): void {
  window.dispatchEvent(
    new CustomEvent(DYNAMIC_UI_SCHEMA_UPDATE_EVENT, {
      detail: { scope: detail.scope ?? GLOBAL_SCOPE, ...detail },
    }),
  );
}
