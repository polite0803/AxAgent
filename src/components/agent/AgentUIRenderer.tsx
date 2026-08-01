// SPDX-License-Identifier: AGPL-3.0-only

import { DynamicUIRenderer } from "@/components/dynamicUI/DynamicUIRenderer";
import { listen } from "@/lib/invoke";
import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import type { UISchema } from "@/types";
import { DeleteOutlined } from "@ant-design/icons";
import { Alert, Empty, Tag, Tooltip } from "antd";
import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";

/**
 * AgentUIRenderer — 渲染 Agent 生成的动态 UI Schema 列表
 *
 * - 通过 Tauri 事件系统监听后端 `agent-render-ui` / `agent-update-ui` / `agent-remove-ui`
 * - 将收到的 UISchema 通过 DynamicUIRenderer 渲染
 * - 支持手动移除已渲染的 UI 组件
 */
export function AgentUIRenderer() {
  const { t } = useTranslation();
  const agentUISchemas = useAgentPanelStore((s) => s.agentUISchemas);
  const renderAgentUI = useAgentPanelStore((s) => s.renderAgentUI);
  const updateAgentUI = useAgentPanelStore((s) => s.updateAgentUI);
  const removeAgentUI = useAgentPanelStore((s) => s.removeAgentUI);
  const clearAgentUI = useAgentPanelStore((s) => s.clearAgentUI);

  // 监听后端 Agent UI 事件（通过 Tauri IPC）
  useEffect(() => {
    let unlistenRender: (() => void) | null = null;
    let unlistenUpdate: (() => void) | null = null;
    let unlistenRemove: (() => void) | null = null;
    let mounted = true;

    const initListeners = async () => {
      try {
        unlistenRender = await listen<{
          schema: Record<string, unknown>;
          targetId?: string;
          replace?: boolean;
        }>("agent-render-ui", (event) => {
          const payload = event.payload;
          if (payload?.schema) {
            renderAgentUI(payload.schema, payload.targetId, payload.replace ?? true);
          }
        });

        if (!mounted) { return; }

        unlistenUpdate = await listen<{
          operation: string;
          schemaId: string;
          newSchema?: Record<string, unknown>;
          path?: string;
        }>("agent-update-ui", (event) => {
          const payload = event.payload;
          if (payload) {
            updateAgentUI(
              payload.schemaId,
              payload.operation as "replace" | "append" | "remove",
              payload.newSchema,
              payload.path,
            );
          }
        });

        if (!mounted) { return; }

        unlistenRemove = await listen<{ schemaId: string }>(
          "agent-remove-ui",
          (event) => {
            const payload = event.payload;
            if (payload?.schemaId) {
              removeAgentUI(payload.schemaId);
            }
          },
        );
      } catch (err) {
        console.error("[AgentUIRenderer] Failed to initialize event listeners:", err);
      }
    };

    initListeners();

    return () => {
      mounted = false;
      unlistenRender?.();
      unlistenUpdate?.();
      unlistenRemove?.();
    };
  }, [renderAgentUI, updateAgentUI, removeAgentUI]);

  const schemas = useMemo(() => agentUISchemas, [agentUISchemas]);

  if (schemas.length === 0) {
    return (
      <div className="flex items-center justify-center py-8">
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("agent.uiRenderer.empty")}
        />
      </div>
    );
  }

  return (
    <div className="agent-ui-renderer space-y-3 p-2">
      <div className="flex items-center justify-between mb-2 px-1">
        <span className="text-xs text-gray-500">
          {t("agent.uiRenderer.title")}
          <Tag color="blue" style={{ marginLeft: 4 }}>
            {schemas.length}
          </Tag>
        </span>
        <Tooltip
          title={t("agent.uiRenderer.clearAll")}
        >
          <DeleteOutlined
            className="text-gray-400 hover:text-red-500 cursor-pointer text-xs"
            onClick={() => clearAgentUI()}
          />
        </Tooltip>
      </div>
      {schemas.map((entry) => {
        const uiSchema = entry.schema as unknown as UISchema;
        if (!uiSchema?.type) {
          return (
            <Alert
              key={entry.id}
              type="error"
              message={t("agent.uiRenderer.invalidSchema")}
              description={t("agent.uiRenderer.invalidSchemaDesc")}
              showIcon
            />
          );
        }

        return (
          <div
            key={entry.id}
            className="agent-ui-entry rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden bg-white dark:bg-gray-900"
          >
            <div className="agent-ui-entry-header flex items-center justify-between px-3 py-1.5 border-b border-gray-100 dark:border-gray-800 bg-gray-50 dark:bg-gray-800/50">
              <span className="text-xs font-medium text-gray-600 dark:text-gray-400 truncate">
                {uiSchema.id || entry.id}
                <Tag color="default" style={{ marginLeft: 4 }}>
                  {uiSchema.type}
                </Tag>
              </span>
              <Tooltip
                title={t("agent.uiRenderer.remove")}
              >
                <DeleteOutlined
                  className="text-gray-400 hover:text-red-500 cursor-pointer text-xs"
                  onClick={() => removeAgentUI(entry.id)}
                />
              </Tooltip>
            </div>
            <div className="agent-ui-entry-body p-3 max-h-96 overflow-auto">
              <DynamicUIRenderer schema={uiSchema} />
            </div>
          </div>
        );
      })}
    </div>
  );
}
