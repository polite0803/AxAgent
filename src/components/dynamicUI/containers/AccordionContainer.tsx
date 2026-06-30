// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Collapse } from "antd";

/**
 * 手风琴折叠容器，基于 Ant Design Collapse。
 * children 中每个子组件映射为一个折叠面板。
 */
export const AccordionContainer: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
  onAction,
}) => {
  const { accordion = true, bordered = true, ghost = false } = schema.props as {
    accordion?: boolean;
    bordered?: boolean;
    ghost?: boolean;
  };

  const items = (schema.children || []).map((child, index) => ({
    key: child.id || `panel-${index}`,
    label: ((child.props as Record<string, unknown>)?.label as string)
      || `Section ${index + 1}`,
    children: renderChild(child, dataContext, onAction),
  }));

  return (
    <Collapse
      accordion={accordion}
      bordered={bordered}
      ghost={ghost}
      items={items}
      style={schema.style as React.CSSProperties}
    />
  );
};

function renderChild(
  child: DynamicUIProps["schema"],
  dataContext: Record<string, unknown> | undefined,
  onAction: DynamicUIProps["onAction"],
): React.ReactNode {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const DynamicUIRenderer = require("../DynamicUIRenderer").DynamicUIRenderer as React.ComponentType<DynamicUIProps>;
  return (
    <DynamicUIRenderer
      schema={child}
      dataContext={dataContext}
      onAction={onAction}
    />
  );
}

export default AccordionContainer;
