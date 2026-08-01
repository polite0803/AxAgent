// SPDX-License-Identifier: AGPL-3.0-only
// antd v6 中 List 已弃用，此文件提供基于 Flex + Pagination 的兼容实现
// 保持 API 与原 antd List 一致，避免修改 10+ 消费文件

import { Empty, Flex, Pagination, Spin } from "antd";
import type { CSSProperties, ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";

export interface ListPagination {
  current?: number;
  pageSize?: number;
  total?: number;
  showSizeChanger?: boolean;
  hideOnSinglePage?: boolean;
  showTotal?: (total: number, range: [number, number]) => ReactNode;
  onChange?: (current: number, pageSize: number) => void;
}

export interface ListProps<T> {
  dataSource?: T[];
  renderItem?: (item: T, index: number) => ReactNode;
  rowKey?: string | ((item: T) => string | number);
  size?: "small" | "default" | "large";
  bordered?: boolean;
  split?: boolean;
  loading?: boolean;
  header?: ReactNode;
  footer?: ReactNode;
  pagination?: false | ListPagination;
  locale?: { emptyText?: ReactNode };
  style?: CSSProperties;
  className?: string;
  children?: ReactNode;
}

export interface ListItemProps {
  style?: CSSProperties;
  className?: string;
  onClick?: () => void;
  children?: ReactNode;
  actions?: ReactNode[];
  extra?: ReactNode;
  main?: ReactNode;
}

export interface ListItemMetaProps {
  avatar?: ReactNode;
  title?: ReactNode;
  description?: ReactNode;
  className?: string;
  style?: CSSProperties;
}

export function ListItemMeta({
  avatar,
  title,
  description,
  className,
  style,
}: ListItemMetaProps) {
  return (
    <div className={className} style={{ display: "flex", gap: 12, ...style }}>
      {avatar && <div style={{ flexShrink: 0 }}>{avatar}</div>}
      <div style={{ flex: 1, minWidth: 0 }}>
        {title && <div style={{ fontWeight: 500, marginBottom: description ? 4 : 0 }}>{title}</div>}
        {description && (
          <div style={{ color: "var(--color-text-secondary, #999)", fontSize: 12 }}>
            {description}
          </div>
        )}
      </div>
    </div>
  );
}

export function AntdListItem({
  style,
  className,
  onClick,
  children,
  actions,
  extra,
  main,
}: ListItemProps) {
  const itemStyle: CSSProperties = {
    display: "flex",
    alignItems: "center",
    padding: "8px 0",
    cursor: onClick ? "pointer" : "default",
    width: "100%",
    ...style,
  };

  const validActions = (actions ?? []).filter(Boolean);

  return (
    <div
      className={className}
      style={itemStyle}
      onClick={onClick}
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
    >
      <div style={{ flex: 1, minWidth: 0 }}>
        {main ?? children}
      </div>
      {extra && <div style={{ marginLeft: "auto", flexShrink: 0 }}>{extra}</div>}
      {validActions.length > 0 && (
        <div style={{ marginLeft: 8, display: "flex", gap: 8, flexShrink: 0 }}>
          {validActions.map((action, idx) => <span key={idx}>{action}</span>)}
        </div>
      )}
    </div>
  );
}

export function AntdList<T>({
  dataSource,
  renderItem,
  rowKey,
  size = "default",
  bordered,
  split = true,
  loading,
  header,
  footer,
  pagination,
  locale,
  style,
  className,
  children,
}: ListProps<T>) {
  const padding = size === "small" ? "4px 8px" : size === "large" ? "16px" : "8px 12px";
  const borderStyle: CSSProperties = bordered
    ? { border: "1px solid var(--color-border-secondary, #f0f0f0)", borderRadius: 8 }
    : {};

  const hasPagination = pagination !== false && pagination !== undefined;
  const paginationConfig = hasPagination && typeof pagination === "object" ? pagination as ListPagination : {};

  const [currentPage, setCurrentPage] = useState(paginationConfig.current ?? 1);
  const [currentPageSize, setCurrentPageSize] = useState(paginationConfig.pageSize ?? 10);

  // dataSource 变化时重置分页到第一页
  useEffect(() => {
    setCurrentPage(1);
  }, [dataSource]);

  // 计算分页数据
  const { pagedData, totalItems } = useMemo(() => {
    const data = dataSource ?? [];
    if (!hasPagination || data.length === 0) {
      return { pagedData: data, totalItems: data.length };
    }
    const total = paginationConfig.total ?? data.length;
    const pageSize = currentPageSize;
    const startIdx = (currentPage - 1) * pageSize;
    return {
      pagedData: data.slice(startIdx, startIdx + pageSize),
      totalItems: total,
    };
  }, [dataSource, hasPagination, paginationConfig.total, currentPage, currentPageSize]);

  const showPagination = hasPagination
    && (paginationConfig.hideOnSinglePage ? totalItems > currentPageSize : true);

  const items: ReactNode[] = [];

  if (header) {
    items.push(
      <div
        key="header"
        style={{
          padding,
          borderBottom: split ? "1px solid var(--color-border-secondary, #f0f0f0)" : "none",
          fontWeight: 500,
        }}
      >
        {header}
      </div>,
    );
  }

  if (loading) {
    items.push(
      <div key="loading" style={{ padding: 24, textAlign: "center" }}>
        <Spin />
      </div>,
    );
  } else if (pagedData.length === 0 && !children) {
    items.push(
      <div key="empty" style={{ padding: 24, textAlign: "center" }}>
        <Empty description={locale?.emptyText} image={Empty.PRESENTED_IMAGE_SIMPLE} />
      </div>,
    );
  } else if (renderItem) {
    const startIndex = hasPagination ? (currentPage - 1) * currentPageSize : 0;
    pagedData.forEach((item, idx) => {
      const rendered = renderItem(item, startIndex + idx);
      const itemKey = rowKey
        ? typeof rowKey === "function"
          ? rowKey(item)
          : (item as Record<string, unknown>)[rowKey]
        : startIndex + idx;
      items.push(
        <div
          key={String(itemKey)}
          style={{
            padding,
            borderBottom: split && idx < pagedData.length - 1
              ? "1px solid var(--color-border-secondary, #f0f0f0)"
              : "none",
          }}
        >
          {rendered}
        </div>,
      );
    });
  }

  if (children) {
    items.push(<div key="children">{children}</div>);
  }

  if (footer) {
    items.push(
      <div
        key="footer"
        style={{ padding, borderTop: split ? "1px solid var(--color-border-secondary, #f0f0f0)" : "none" }}
      >
        {footer}
      </div>,
    );
  }

  return (
    <Flex
      vertical
      className={className}
      style={{ width: "100%", ...borderStyle, ...style }}
    >
      {items}
      {showPagination && totalItems > 0 && (
        <div
          key="pagination"
          style={{
            padding: "8px 12px",
            borderTop: split ? "1px solid var(--color-border-secondary, #f0f0f0)" : "none",
            display: "flex",
            justifyContent: "flex-end",
          }}
        >
          <Pagination
            current={currentPage}
            pageSize={currentPageSize}
            total={totalItems}
            showSizeChanger={paginationConfig.showSizeChanger ?? false}
            onChange={(page, pageSize) => {
              setCurrentPage(page);
              setCurrentPageSize(pageSize);
              paginationConfig.onChange?.(page, pageSize);
            }}
            {...(paginationConfig.showTotal ? { showTotal: paginationConfig.showTotal } : {})}
          />
        </div>
      )}
    </Flex>
  );
}

// 兼容导出：保持原来的命名导出名 "List"，但使用新的 AntdList 实现
type ListComponent =
  & (<T>(props: ListProps<T>) => ReactNode)
  & { Item: AntdListItemComponent };

type AntdListItemComponent =
  & ((props: ListItemProps) => ReactNode)
  & { Meta: typeof ListItemMeta };

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const _listItem = Object.assign(AntdListItem, { Meta: ListItemMeta }) as AntdListItemComponent;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const _list = Object.assign(AntdList, { Item: _listItem }) as any;
export const List: ListComponent = _list;
export const ListItem = _listItem;
