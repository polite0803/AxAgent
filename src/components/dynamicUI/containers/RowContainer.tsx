// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";

/**
 * 水平布局容器。
 * 渲染 div.flex.flex-row，支持 gap、align、justify 等 props。
 */
export const RowContainer: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
  onAction,
}) => {
  const { gap = 8, align = "center", justify = "start", wrap, className } = schema.props as {
    gap?: number;
    align?: string;
    justify?: string;
    wrap?: boolean;
    className?: string;
  };

  return (
    <div
      className={`flex flex-row ${wrap ? "flex-wrap" : ""} ${className || ""}`}
      style={{
        gap: `${gap}px`,
        alignItems: align,
        justifyContent: justify,
        ...(schema.style as React.CSSProperties),
      }}
    >
      {renderChildren(schema, dataContext, onAction)}
    </div>
  );
};

// ── 内部工具函数：递归渲染子组件（延迟导入避免循环依赖） ──

function renderChildren(
  schema: DynamicUIProps["schema"],
  dataContext: Record<string, unknown> | undefined,
  onAction: DynamicUIProps["onAction"],
): React.ReactNode {
  if (!schema.children || schema.children.length === 0) {
    return null;
  }

  // 延迟导入 DynamicUIRenderer 避免循环依赖
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const DynamicUIRenderer = require("../DynamicUIRenderer").DynamicUIRenderer as React.ComponentType<DynamicUIProps>;

  return schema.children.map((child) => (
    <DynamicUIRenderer
      key={child.id}
      schema={child}
      dataContext={dataContext}
      onAction={onAction}
    />
  ));
}

export default RowContainer;
