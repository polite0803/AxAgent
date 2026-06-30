// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";

/**
 * 垂直布局容器。
 * 渲染 div.flex.flex-col，支持 gap、align 等 props。
 */
export const ColumnContainer: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
  onAction,
}) => {
  const { gap = 8, align = "stretch", className } = schema.props as {
    gap?: number;
    align?: string;
    className?: string;
  };

  return (
    <div
      className={`flex flex-col ${className || ""}`}
      style={{
        gap: `${gap}px`,
        alignItems: align,
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

export default ColumnContainer;
