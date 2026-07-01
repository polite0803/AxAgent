// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Tabs } from "antd";

interface TabItem {
  key: string;
  label: string;
  children: React.ReactNode;
}

export const TabsContainer: React.FC<DynamicUIProps> = ({
  schema,
}) => {
  const {
    tabPosition = "top",
    centered = false,
    type,
    items = [],
  } = schema.props as {
    tabPosition?: "top" | "bottom" | "left" | "right";
    centered?: boolean;
    type?: "line" | "card" | "editable-card";
    items?: TabItem[];
  };

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
