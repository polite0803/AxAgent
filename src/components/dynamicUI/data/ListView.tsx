// SPDX-License-Identifier: AGPL-3.0-only

import { resolveDynamicArray } from "@/lib/dynamicUI/utils";
import type { DynamicUIProps } from "@/types";
import { List } from "@/components/common/AntdList";
import { Empty } from "antd";
import React from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useTranslation } from "react-i18next";

/**
 * 列表组件，基于 Ant Design List。
 * 如果 react-virtuoso 可用则使用虚拟滚动，否则降级到普通 List。
 */
export const ListView: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
}) => {
  const { t } = useTranslation();
  const {
    itemLayout = "vertical",
    size = "default",
    bordered = false,
    split = true,
  } = schema.props as {
    itemLayout?: "vertical" | "horizontal";
    size?: "small" | "default" | "large";
    bordered?: boolean;
    split?: boolean;
  };

  const data = resolveDynamicArray(
    schema.props.dataSource as Record<string, unknown>[] | undefined,
    dataContext,
    schema.id,
  );

  if (data.length === 0) {
    return <Empty description={t("dynamicUI.noData")} />;
  }

  const renderItem = (item: Record<string, unknown>) => {
    const title = item.title || item.label || item.name;
    const description = item.description || item.content || item.summary;

    return (
      <List.Item>
        <List.Item.Meta
          title={title as string}
          description={description as string}
        />
      </List.Item>
    );
  };

  // 尝试使用虚拟滚动
  if (data.length > 50) {
    return (
      <div style={schema.style as React.CSSProperties}>
        <VirtualListView data={data} renderItem={renderItem} />
      </div>
    );
  }

  return (
    <List
      itemLayout={itemLayout}
      size={size}
      bordered={bordered}
      split={split}
      dataSource={data}
      renderItem={renderItem}
      style={schema.style as React.CSSProperties}
    />
  );
};

/** 虚拟滚动列表，基于 @tanstack/react-virtual */
function VirtualListView({
  data,
  renderItem,
}: {
  data: Record<string, unknown>[];
  renderItem: (item: Record<string, unknown>) => React.ReactNode;
}) {
  const parentRef = React.useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: data.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 48,
  });

  return (
    <div ref={parentRef} style={{ height: 400, overflow: "auto" }}>
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((item) => (
          <div
            key={item.key}
            style={{
              position: "absolute",
              transform: `translateY(${item.start}px)`,
              left: 0,
              right: 0,
            }}
          >
            {renderItem(data[item.index])}
          </div>
        ))}
      </div>
    </div>
  );
}
