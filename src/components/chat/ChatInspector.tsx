import { useArtifactStore, useConversationStore } from "@/stores";
import { Descriptions, Empty, List, Tabs, Tag, theme, Typography } from "antd";
import { FileText, Info, Paperclip, Search, Wrench } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

interface ChatInspectorProps {
  visible: boolean;
  activeTab: string;
  onTabChange: (tab: string) => void;
  conversationId: string | null;
}

export function ChatInspector({
  visible,
  activeTab,
  onTabChange,
  conversationId,
}: ChatInspectorProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  // 使用各自 primitive selector，避免每次返回新对象导致无限重渲染
  const convId = useConversationStore(
    (s) => s.conversations.find((c) => c.id === s.activeConversationId)?.id,
  );
  const convProviderId = useConversationStore(
    (s) => s.conversations.find((c) => c.id === s.activeConversationId)?.provider_id ?? "",
  );
  const convModelId = useConversationStore(
    (s) => s.conversations.find((c) => c.id === s.activeConversationId)?.model_id ?? "",
  );
  const convCreatedAt = useConversationStore(
    (s) => s.conversations.find((c) => c.id === s.activeConversationId)?.created_at,
  );
  const convMessageCount = useConversationStore(
    (s) => s.conversations.find((c) => c.id === s.activeConversationId)?.message_count ?? 0,
  );

  const [conversationCreatedFormatted, setConversationCreatedFormatted] = useState("");
  useEffect(() => {
    if (convCreatedAt) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setConversationCreatedFormatted(
        new Date(convCreatedAt).toLocaleString(),
      );
    }
  }, [convCreatedAt]);

  const workspaceSnapshot = useConversationStore((s) => s.workspaceSnapshot);
  const messages = useConversationStore((s) => s.messages);
  const { artifacts } = useArtifactStore();

  const contextSources = useMemo(() => {
    if (!workspaceSnapshot) {
      return [];
    }
    const sources: { type: string; title: string }[] = [];
    if (workspaceSnapshot.knowledgeBinding?.knowledgeBaseIds?.length) {
      workspaceSnapshot.knowledgeBinding.knowledgeBaseIds.forEach((id) =>
        sources.push({ type: "knowledge", title: id })
      );
    }
    if (workspaceSnapshot.searchPolicy?.enabled) {
      sources.push({
        type: "search",
        title: workspaceSnapshot.searchPolicy.searchProviderId ?? "search",
      });
    }
    if (workspaceSnapshot.memoryPolicy?.enabled) {
      sources.push({
        type: "memory",
        title: workspaceSnapshot.memoryPolicy.namespaceId ?? "memory",
      });
    }
    if (workspaceSnapshot.toolBinding?.serverIds?.length) {
      workspaceSnapshot.toolBinding.serverIds.forEach((id) => sources.push({ type: "tool", title: id }));
    }
    return sources;
  }, [workspaceSnapshot]);

  const toolCalls = useMemo(() => {
    return messages.flatMap((m) => {
      if (m.role !== "assistant" || !m.content) {
        return [];
      }
      const calls: { name: string; messageId: string }[] = [];
      const regex = /tool_call|function_call|<tool>(.*?)<\/tool>/g;
      if (regex.test(m.content)) {
        calls.push({ name: "tool_call", messageId: m.id });
      }
      return calls;
    });
  }, [messages]);

  const conversationArtifacts = useMemo(() => {
    if (!conversationId) {
      return [];
    }
    return artifacts.filter((a) => a.conversationId === conversationId);
  }, [artifacts, conversationId]);

  const tabItems = useMemo(
    () => [
      {
        key: "sources",
        label: t("chat.inspector.sources"),
        icon: <Search size={14} />,
        children: contextSources.length > 0
          ? (
            <List
              size="small"
              dataSource={contextSources}
              renderItem={(item) => (
                <List.Item>
                  <List.Item.Meta
                    title={<Typography.Text>{item.title}</Typography.Text>}
                    description={<Tag>{item.type}</Tag>}
                  />
                </List.Item>
              )}
            />
          )
          : (
            <Empty
              description={conversationId ? t("common.noData") : t("common.noData")}
              style={{ marginTop: 48 }}
            />
          ),
      },
      {
        key: "tools",
        label: t("chat.inspector.tools"),
        icon: <Wrench size={14} />,
        children: toolCalls.length > 0
          ? (
            <List
              size="small"
              dataSource={toolCalls}
              renderItem={(item) => (
                <List.Item>
                  <Typography.Text code>{item.name}</Typography.Text>
                </List.Item>
              )}
            />
          )
          : (
            <Empty
              description={t("chat.inspector.tools")}
              style={{ marginTop: 48 }}
            />
          ),
      },
      {
        key: "attachments",
        label: t("chat.inspector.attachments"),
        icon: <Paperclip size={14} />,
        children: (() => {
          const attachments = messages.flatMap((m) => m.attachments ?? []);
          return attachments.length > 0
            ? (
              <List
                size="small"
                dataSource={attachments}
                renderItem={(item) => (
                  <List.Item>
                    <Typography.Text ellipsis>{item.file_name}</Typography.Text>
                  </List.Item>
                )}
              />
            )
            : (
              <Empty
                description={t("chat.inspector.attachments")}
                style={{ marginTop: 48 }}
              />
            );
        })(),
      },
      {
        key: "session",
        label: t("chat.inspector.session"),
        icon: <Info size={14} />,
        children: convId
          ? (
            <Descriptions column={1} size="small" style={{ padding: "8px 0" }}>
              <Descriptions.Item label={t("chat.inspector.session")}>
                <Typography.Text copyable={{ text: convId }}>
                  {convId.slice(0, 8)}…
                </Typography.Text>
              </Descriptions.Item>
              <Descriptions.Item label={t("gateway.defaultProvider")}>
                {convProviderId || "-"}
              </Descriptions.Item>
              <Descriptions.Item label={t("gateway.defaultModel")}>
                {convModelId || "-"}
              </Descriptions.Item>
              <Descriptions.Item label={t("gateway.created")}>
                {conversationCreatedFormatted}
              </Descriptions.Item>
              <Descriptions.Item label={t("chat.inspector.tools")}>
                {convMessageCount}
              </Descriptions.Item>
            </Descriptions>
          )
          : <Empty description={t("common.noData")} style={{ marginTop: 48 }} />,
      },
      {
        key: "artifacts",
        label: t("chat.inspector.artifacts"),
        icon: <FileText size={14} />,
        children: conversationArtifacts.length > 0
          ? (
            <List
              size="small"
              dataSource={conversationArtifacts}
              renderItem={(item) => (
                <List.Item>
                  <List.Item.Meta
                    title={<Typography.Text>{item.title}</Typography.Text>}
                    description={<Tag>{item.kind}</Tag>}
                  />
                </List.Item>
              )}
            />
          )
          : (
            <Empty
              description={conversationId ? t("common.noData") : t("common.noData")}
              style={{ marginTop: 48 }}
            />
          ),
      },
    ],
    [
      t,
      conversationId,
      contextSources,
      toolCalls,
      messages,
      convId,
      convProviderId,
      convModelId,
      convMessageCount,
      conversationCreatedFormatted,
      conversationArtifacts,
    ],
  );

  return (
    <div
      style={{
        width: visible ? 360 : 0,
        minWidth: visible ? 360 : 0,
        overflow: "hidden",
        transition: "width 0.2s ease, min-width 0.2s ease",
        borderLeft: visible ? "1px solid var(--border-color)" : "none",
        backgroundColor: token.colorBgContainer,
        display: "flex",
        flexDirection: "column",
      }}
    >
      {visible && (
        <Tabs
          activeKey={activeTab}
          onChange={onTabChange}
          items={tabItems}
          size="small"
          style={{ flex: 1, padding: "0 12px" }}
        />
      )}
    </div>
  );
}
