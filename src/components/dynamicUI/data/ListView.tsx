// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Empty, List } from "antd";
import { lazy, Suspense } from "react";

/**
 * 列表组件，基于 Ant Design List。
 * 如果 react-virtuoso 可用则使用虚拟滚动，否则降级到普通 List。
 */
export const ListView: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
}) => {
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

  const data = (schema.props.dataSource as Record<string, unknown>[])
      || (dataContext
        && Array.isArray(
          (dataContext as Record<string, unknown>)[schema.id],
        ))
    ? (
      (dataContext as Record<string, unknown>)[schema.id] as Record<
        string,
        unknown
      >[]
    )
    : [];

  if (data.length === 0) {
    return <Empty description="暂无数据" />;
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
        <Suspense
          fallback={
            <List
              itemLayout={itemLayout}
              size={size}
              bordered={bordered}
              split={split}
              dataSource={data}
              renderItem={renderItem}
            />
          }
        >
          <VirtuosoListView data={data} renderItem={renderItem} />
        </Suspense>
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

/** 延迟加载 react-virtuoso */
const VirtuosoListView = lazy(
  () =>
    import("react-virtuoso").then((m) => {
      const { Virtuoso } = m;
      return {
        default: ({
          data,
          renderItem,
        }: {
          data: Record<string, unknown>[];
          renderItem: (item: Record<string, unknown>) => React.ReactNode;
        }) => (
          <Virtuoso
            style={{ height: "400px" }}
            totalCount={data.length}
            itemContent={(index: number) => renderItem(data[index])}
          />
        ),
      };
    }),
);

export default ListView;
