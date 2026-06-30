// SPDX-License-Identifier: AGPL-3.0-only

import { componentRegistry } from "@/lib/dynamicUI/ComponentRegistry";
import { evaluateConditions } from "@/lib/dynamicUI/ConditionalRenderer";
import { resolveDataSource } from "@/lib/dynamicUI/DataBindingEngine";
import { executeActions, getLifecycleHandlers, handleEvents } from "@/lib/dynamicUI/EventHandlerEngine";
import { validateSchema } from "@/lib/dynamicUI/SchemaValidator";
import type { DynamicUIProps } from "@/types";
import { Alert } from "antd";
import React, { useEffect, useMemo, useState } from "react";

/**
 * 核心递归渲染器。
 * 接收 DynamicUIProps（schema + dataContext + onAction），
 * 递归渲染 UISchema 为 React 组件树。
 */
export const DynamicUIRenderer: React.FC<DynamicUIProps> = React.memo(
  ({ schema, dataContext: externalContext, onAction }) => {
    // ── 1. 校验 Schema 合法性 ──
    const validation = useMemo(() => validateSchema(schema), [schema]);
    if (!validation.valid) {
      return (
        <Alert
          type="error"
          message="Schema 校验失败"
          description={
            <ul className="list-disc pl-4 mt-1">
              {validation.errors.slice(0, 5).map((err, i) => (
                <li key={i}>
                  {err.path}: {err.message}
                </li>
              ))}
              {validation.errors.length > 5 ? <li>... 及其他 {validation.errors.length - 5} 个错误</li> : null}
            </ul>
          }
          showIcon
        />
      );
    }

    return (
      <SchemaErrorBoundary schemaId={schema.id}>
        <DynamicUIRendererInner
          schema={schema}
          externalContext={externalContext}
          onAction={onAction}
        />
      </SchemaErrorBoundary>
    );
  },
);

DynamicUIRenderer.displayName = "DynamicUIRenderer";

// ── Inner Component ──

const DynamicUIRendererInner: React.FC<
  DynamicUIProps & { externalContext?: Record<string, unknown> }
> = ({ schema, externalContext, onAction }) => {
  const [resolvedData, setResolvedData] = useState<unknown>(null);

  // ── 2. 解析 dataSource ──
  useEffect(() => {
    if (schema.dataSource) {
      let cancelled = false;
      resolveDataSource(schema.dataSource)
        .then((data) => {
          if (!cancelled) {
            setResolvedData(data);
          }
        })
        .catch(() => {
          // dataSource 解析失败，错误被静默处理
        });
      return () => {
        cancelled = true;
      };
    }
  }, [schema.dataSource]);

  // ── 合并数据上下文 ──
  const mergedContext = useMemo(() => {
    const base = { ...(externalContext || {}) };
    if (resolvedData && typeof resolvedData === "object") {
      Object.assign(base, resolvedData as Record<string, unknown>);
    }
    return base;
  }, [externalContext, resolvedData]);

  // ── 3. 解析 conditionalDisplay ──
  const shouldRender = useMemo(
    () => evaluateConditions(schema.conditionalDisplay || [], mergedContext),
    [schema.conditionalDisplay, mergedContext],
  );

  // ── 4. 生命周期事件 ──
  useEffect(() => {
    if (schema.events) {
      const { onMount } = getLifecycleHandlers(schema.events);
      if (onMount.length > 0) {
        void executeActions(onMount, mergedContext);
      }
    }
    return () => {
      if (schema.events) {
        const { onUnmount } = getLifecycleHandlers(schema.events);
        if (onUnmount.length > 0) {
          void executeActions(onUnmount, mergedContext);
        }
      }
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // ── 5. 获取注册组件 ──
  const entry = componentRegistry.get(schema.type);

  // ── 6. 构建子组件渲染 ──
  const childNodes = useMemo(() => {
    if (!schema.children || schema.children.length === 0) {
      return null;
    }
    return schema.children.map((child) => (
      <DynamicUIRenderer
        key={child.id}
        schema={child}
        dataContext={mergedContext}
        onAction={onAction}
      />
    ));
  }, [schema.children, mergedContext, onAction]);

  // ── 7. 事件绑定 ──
  const eventBindings = useMemo(
    () => handleEvents(schema.events || [], mergedContext),
    [schema.events, mergedContext],
  );

  // ── 8. 合并 props ──
  const mergedProps = useMemo(() => {
    const base = {
      ...(entry?.defaultProps || {}),
      ...(schema.props || {}),
    };
    // dataSource 已解析，合并到 props 的 dataSource 字段
    if (resolvedData) {
      base.dataSource = resolvedData;
    }
    return base;
  }, [entry?.defaultProps, schema.props, resolvedData]);

  // ── 条件渲染（在所有 hooks 之后检查） ──
  if (!shouldRender) {
    return null;
  }
  if (!entry) {
    return <UnregisteredPlaceholder type={schema.type} />;
  }

  const Component = entry.component;

  // 包裹 children 和事件绑定（通过 SchemaErrorBoundary 捕获错误，无需 try/catch）
  return (
    <Component
      schema={{ ...schema, props: mergedProps, children: undefined }}
      dataContext={mergedContext}
      onAction={onAction}
      {...eventBindings}
    >
      {childNodes}
    </Component>
  );
};

// ── 辅助组件 ──

function UnregisteredPlaceholder({ type }: { type: string }): React.ReactElement {
  return (
    <div
      className="border border-yellow-400 bg-yellow-50 dark:bg-yellow-900/20 rounded p-3 my-1"
      role="alert"
    >
      <div className="text-yellow-700 dark:text-yellow-400 font-medium text-sm">
        未注册组件: {type}
      </div>
      <div className="text-yellow-600 dark:text-yellow-500 text-xs mt-1">
        请通过 componentRegistry.register() 注册此组件
      </div>
    </div>
  );
}

function ErrorPlaceholder({
  type,
  error,
}: {
  type: string;
  error: unknown;
}): React.ReactElement {
  return (
    <Alert
      type="error"
      message={`组件 "${type}" 渲染失败`}
      description={
        <pre className="text-xs whitespace-pre-wrap">
          {error instanceof Error ? error.message : String(error)}
        </pre>
      }
      showIcon
    />
  );
}

// ── Error Boundary ──

class SchemaErrorBoundary extends React.Component<
  { schemaId: string; children: React.ReactNode },
  { hasError: boolean; error: Error | null }
> {
  constructor(props: { schemaId: string; children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  render() {
    if (this.state.hasError) {
      return (
        <ErrorPlaceholder
          type={this.props.schemaId}
          error={this.state.error}
        />
      );
    }
    return this.props.children;
  }
}
