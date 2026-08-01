// SPDX-License-Identifier: AGPL-3.0-only

// ! [AxInvest 本地专属] Agent 动态 UI 宿主 — 股票工作区内嵌容器
// !
// ! 与上游侧边栏 `AgentUIRenderer` 的区别：
// ! - 侧边栏显示所有 agent-render-ui 渲染（targetId 为空或任意）
// ! - 本组件按 `targetId === "stock-workspace"` **过滤**，只显示发往股票工作区的渲染
// ! - 挂载于 StockWorkspaceShell 中栏视图区底部，跨 6 个视图 Tab 常驻
// !
// ! 事件协议与上游一致：监听 `agent-render-ui` / `agent-update-ui` / `agent-remove-ui`，
// ! 复用 agentPanelStore 的 renderAgentUI/updateAgentUI/removeAgentUI 写入同一份
// ! `agentUISchemas`，因此侧边栏与股票工作区看到的是同一数据源（双向同步）。

import { DynamicUIRenderer } from "@/components/dynamicUI/DynamicUIRenderer";
import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import type { UISchema } from "@/types";
import { DeleteOutlined, DownOutlined, UpOutlined } from "@ant-design/icons";
import { Alert, Button, Empty, Space, Tag, Tooltip, Typography } from "antd";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/** 本组件响应的渲染目标容器 ID（与 stock_render_ui 工具指引一致） */
export const STOCK_AGENT_UI_TARGET = "stock-workspace";

/**
 * AgentStockUIHost — 股票工作区内的 Agent 动态 UI 容器
 *
 * 只渲染 targetId === "stock-workspace" 的 schema；可折叠/展开、可清空。
 */
export function AgentStockUIHost() {
  const { t } = useTranslation();
  const agentUISchemas = useAgentPanelStore((s) => s.agentUISchemas);
  const removeAgentUI = useAgentPanelStore((s) => s.removeAgentUI);
  const [collapsed, setCollapsed] = useState(false);

  // 按 targetId 过滤：只显示发往股票工作区的渲染
  const schemas = useMemo(
    () => agentUISchemas.filter((e) => e.targetId === STOCK_AGENT_UI_TARGET),
    [agentUISchemas],
  );

  // 空态零占位：无渲染时不渲染任何内容（不挤压业务视图）
  if (schemas.length === 0) {
    return null;
  }

  return (
    <div className="agent-stock-ui-host rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 m-2 overflow-hidden">
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-gray-100 dark:border-gray-800 bg-gray-50 dark:bg-gray-800/50">
        <Space size={6}>
          <Text strong style={{ fontSize: 12 }} className="text-gray-600 dark:text-gray-300">
            {t("agent.stockUiHost.title")}
          </Text>
          <Tag color="amber" style={{ marginInlineEnd: 0 }}>
            {schemas.length}
          </Tag>
        </Space>
        <Space size={2}>
          <Tooltip
            title={collapsed
              ? t("agent.stockUiHost.expand")
              : t("agent.stockUiHost.collapse")}
          >
            <Button
              type="text"
              size="small"
              icon={collapsed ? <UpOutlined /> : <DownOutlined />}
              onClick={() => setCollapsed((v) => !v)}
            />
          </Tooltip>
          <Tooltip title={t("agent.stockUiHost.clear")}>
            <Button
              type="text"
              size="small"
              danger
              icon={<DeleteOutlined />}
              onClick={() => {
                schemas.forEach((e) => removeAgentUI(e.id));
              }}
            />
          </Tooltip>
        </Space>
      </div>

      {!collapsed && (
        <div className="space-y-2 p-2 max-h-72 overflow-y-auto">
          {schemas.map((entry) => {
            const uiSchema = entry.schema as unknown as UISchema;
            if (!uiSchema?.type) {
              return (
                <Alert
                  key={entry.id}
                  type="error"
                  message={t("agent.uiRenderer.invalidSchema")}
                  showIcon
                />
              );
            }
            return (
              <div
                key={entry.id}
                className="agent-stock-ui-entry rounded-md border border-gray-100 dark:border-gray-800 overflow-hidden"
              >
                <div className="flex items-center justify-between px-2 py-1 border-b border-gray-100 dark:border-gray-800 bg-gray-50 dark:bg-gray-800/40">
                  <Text type="secondary" style={{ fontSize: 11 }} className="truncate">
                    {uiSchema.id || entry.id}
                    <Tag color="default" style={{ marginLeft: 4, marginInlineEnd: 0 }}>
                      {uiSchema.type}
                    </Tag>
                  </Text>
                  <Tooltip title={t("agent.stockUiHost.remove")}>
                    <Button
                      type="text"
                      size="small"
                      danger
                      icon={<DeleteOutlined />}
                      onClick={() => removeAgentUI(entry.id)}
                    />
                  </Tooltip>
                </div>
                <div className="p-2">
                  <DynamicUIRenderer schema={uiSchema} />
                </div>
              </div>
            );
          })}
          {schemas.length === 0 && (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("agent.uiRenderer.empty")}
            />
          )}
        </div>
      )}
    </div>
  );
}
