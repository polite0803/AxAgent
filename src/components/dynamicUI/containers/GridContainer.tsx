// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";

/**
 * 网格布局容器。
 * 渲染 CSS Grid 布局，支持 columns（列数）、gap 等 props。
 */
export const GridContainer: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
  onAction,
}) => {
  const { columns = 2, gap = 16, className } = schema.props as {
    columns?: number;
    gap?: number;
    className?: string;
  };

  return (
    <div
      className={`grid ${className || ""}`}
      style={{
        gridTemplateColumns: `repeat(${columns}, 1fr)`,
        gap: `${gap}px`,
        ...(schema.style as React.CSSProperties),
      }}
    >
      {renderChildren(schema, dataContext, onAction)}
    </div>
  );
};

function renderChildren(
  schema: DynamicUIProps["schema"],
  dataContext: Record<string, unknown> | undefined,
  onAction: DynamicUIProps["onAction"],
): React.ReactNode {
  if (!schema.children || schema.children.length === 0) {
    return null;
  }
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const DynamicUIRenderer =
    require("../DynamicUIRenderer").DynamicUIRenderer as React.ComponentType<DynamicUIProps>;
  return schema.children.map((child) => (
    <DynamicUIRenderer
      key={child.id}
      schema={child}
      dataContext={dataContext}
      onAction={onAction}
    />
  ));
}

export default GridContainer;
