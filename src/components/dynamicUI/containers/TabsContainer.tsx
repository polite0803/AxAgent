// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Tabs } from "antd";

/**
 * 标签页容器，基于 Ant Design Tabs。
 * children 中每个子组件映射为一个 TabPane，通过子组件的 props.label 设置标签名。
 */
export const TabsContainer: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
  onAction,
}) => {
  const {
    tabPosition = "top",
    centered = false,
    type,
  } = schema.props as {
    tabPosition?: "top" | "bottom" | "left" | "right";
    centered?: boolean;
    type?: "line" | "card" | "editable-card";
  };

  const items = (schema.children || []).map((child, index) => ({
    key: child.id || `tab-${index}`,
    label: ((child.props as Record<string, unknown>)?.label as string)
      || `Tab ${index + 1}`,
    children: renderChild(child, dataContext, onAction),
  }));

  return (
    <Tabs
      tabPosition={tabPosition}
      centered={centered}
      type={type}
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
  const DynamicUIRenderer =
    require("../DynamicUIRenderer").DynamicUIRenderer as React.ComponentType<DynamicUIProps>;
  return (
    <DynamicUIRenderer
      schema={child}
      dataContext={dataContext}
      onAction={onAction}
    />
  );
}

export default TabsContainer;
