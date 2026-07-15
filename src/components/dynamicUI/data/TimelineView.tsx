// SPDX-License-Identifier: AGPL-3.0-only

import { resolveDynamicArray } from "@/lib/dynamicUI/utils";
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
  const items: TimelineItem[] = resolveDynamicArray<TimelineItem>(
    schema.props.items as TimelineItem[] | undefined,
    dataContext,
    schema.id,
  );

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
