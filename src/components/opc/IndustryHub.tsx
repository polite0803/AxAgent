// SPDX-License-Identifier: AGPL-3.0-only

import { Tabs, type TabsProps, Typography } from "antd";
import { useMemo } from "react";
import { useSearchParams } from "react-router-dom";

import { IndustryTabContent } from "@/pages/opc/industries/IndustryTabContent";
import type { IndustryConfig } from "@/pages/opc/industries/types";

const { Title } = Typography;

/**
 * IndustryHub — 行业业务统一入口（与 InvestHub 结构一致）。
 *
 * 将单个行业的业务流程集成到一个页面内的 Tab 中，按业务逻辑排序。
 *
 * URL 参数：
 *   - ?tab=xxx — 当前激活的业务流程 Tab
 */
export function IndustryHub({
  industryId,
  config,
  industryTitle,
  industryIcon,
}: {
  industryId: string;
  config: IndustryConfig;
  industryTitle: string;
  industryIcon?: React.ReactNode;
}) {
  const [searchParams, setSearchParams] = useSearchParams();

  // 从配置中获取 tabs
  const tabs = config.tabs || [];

  // 构建合法 tab key 集合
  const validTabs = useMemo(() => new Set(tabs.map((tab) => tab.key)), [tabs]);

  // 默认激活的 tab
  const defaultTab = tabs[0]?.key || "";

  // 从 URL 读取当前 tab（非法值回退到默认）
  const currentTab = useMemo(() => {
    const raw = searchParams.get("tab");
    if (raw && validTabs.has(raw)) {
      return raw;
    }
    return defaultTab;
  }, [searchParams, validTabs, defaultTab]);

  // tab 切换 → 更新 URL
  const handleTabChange = (key: string) => {
    const next = new URLSearchParams(searchParams);
    next.set("tab", key);
    setSearchParams(next, { replace: true });
  };

  // 构建 Tab items
  const items: TabsProps["items"] = useMemo(
    () =>
      tabs.map((tab) => ({
        key: tab.key,
        label: (
          <span>
            {tab.icon && <span style={{ marginRight: 8 }}>{tab.icon}</span>}
            {tab.label}
          </span>
        ),
        children: (
          <IndustryTabContent
            industryId={industryId}
            config={config}
            tabKey={tab.key}
          />
        ),
      })),
    [tabs, industryId, config],
  );

  return (
    <div className="flex flex-col h-full w-full min-h-0">
      <div style={{ padding: "12px 16px 0", background: "var(--color-bg-container)" }}>
        <Title level={3} style={{ margin: 0 }}>
          {industryIcon && <span style={{ marginRight: 8, verticalAlign: "middle" }}>{industryIcon}</span>}
          {industryTitle}
        </Title>
      </div>
      {tabs.length > 0 && (
        <Tabs
          activeKey={currentTab}
          onChange={handleTabChange}
          items={items}
          className="industry-hub-tabs ax-fill-tabs"
          tabBarStyle={{
            margin: 0,
            padding: "0 16px",
            background: "var(--color-bg-container)",
            borderBottom: "1px solid var(--color-border-secondary)",
          }}
          tabBarGutter={16}
          size="small"
          destroyOnHidden={false}
        />
      )}
    </div>
  );
}
