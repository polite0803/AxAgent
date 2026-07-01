// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Card } from "antd";

export const CardContainer: React.FC<DynamicUIProps> = ({
  schema,
  children,
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
      {children}
    </Card>
  );
};
