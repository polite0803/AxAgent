// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Timeline } from "antd";

interface TimelineItem {
  label: string;
  content: string;
  color?: string;
}

/**
 * 时间线组件，基于 Ant Design Timeline。
 */
export const TimelineView: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
}) => {
  const items: TimelineItem[] =
    (schema.props.items as TimelineItem[])
    || (dataContext &&
        Array.isArray((dataContext as Record<string, unknown>)[schema.id]))
      ? (
        (dataContext as Record<string, unknown>)[schema.id] as TimelineItem[]
      )
      : [];

  return (
    <Timeline
      items={items.map((item) => ({
        children: (
          <div>
            <div className="font-medium">{item.label}</div>
            <div className="text-gray-500 text-sm">{item.content}</div>
          </div>
        ),
        color: item.color,
      }))}
      style={schema.style as React.CSSProperties}
    />
  );
};

export default TimelineView;
