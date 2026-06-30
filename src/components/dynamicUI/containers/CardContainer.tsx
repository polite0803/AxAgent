// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Card } from "antd";

/**
 * 卡片容器，基于 Ant Design Card。
 * 支持 title、extra、bordered、hoverable 等 props。
 */
export const CardContainer: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
  onAction,
}) => {
  const {
    title,
    extra,
    bordered = true,
    hoverable = false,
    size,
  } = schema.props as {
    title?: string;
    extra?: string;
    bordered?: boolean;
    hoverable?: boolean;
    size?: "default" | "small";
  };

  return (
    <Card
      title={title}
      extra={extra}
      bordered={bordered}
      hoverable={hoverable}
      size={size}
      style={schema.style as React.CSSProperties}
    >
      {renderChildren(schema, dataContext, onAction)}
    </Card>
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

export default CardContainer;
