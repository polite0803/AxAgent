// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Collapse } from "antd";

interface CollapseItem {
  key: string;
  label: string;
  children: React.ReactNode;
}

export const AccordionContainer: React.FC<DynamicUIProps> = ({
  schema,
}) => {
  const { accordion = true, bordered = true, ghost = false, items = [] } = schema.props as {
    accordion?: boolean;
    bordered?: boolean;
    ghost?: boolean;
    items?: CollapseItem[];
  };

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
