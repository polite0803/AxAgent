import { Badge, Button, Card, Space, Tag, Tooltip, Typography } from "antd";
import { Clock, Copy, Link, Plus, UserPlus, Users } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { SessionShareDialog } from "./SessionShareDialog";

const { Text, Title } = Typography;

interface Participant {
  user_id: string;
  display_name: string;
  role: "Owner" | "Editor" | "Viewer";
  joined_at: number;
  last_active: number;
}

interface SharedSession {
  session_id: string;
  owner_id: string;
  invite_code: string;
  participants: Participant[];
  shared_resources: Array<{
    resource_type: string;
    resource_id: string;
    access_level: string;
  }>;
  permissions: {
    allow_terminal_access: boolean;
    allow_file_access: boolean;
    allow_model_access: boolean;
    require_approval_for_actions: boolean;
    max_participants: number;
  };
  created_at: number;
  expires_at: number | null;
  is_active: boolean;
}

interface CollaborationPanelProps {
  conversationId: string;
}

export function CollaborationPanel({ conversationId }: CollaborationPanelProps) {
  const { t } = useTranslation();
  const [sessions, setSessions] = useState<SharedSession[]>([]);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const copyTimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    return () => clearTimeout(copyTimerRef.current);
  }, []);

  useEffect(() => {
    const fetchSessions = async () => {
      try {
        const { invoke } = await import("@/lib/invoke");
        const data = await invoke<SharedSession[]>(
          "collaboration_list_sessions",
        ).catch((e) => {
          if (import.meta.env.DEV) { console.warn("Failed to fetch sessions:", e); }
          return [];
        });
        setSessions(data);
      } catch {
        // ignore
      }
    };
    fetchSessions();
    const interval = setInterval(fetchSessions, 10000);
    return () => clearInterval(interval);
  }, []);

  const copyInviteCode = (code: string) => {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      clearTimeout(copyTimerRef.current);
      copyTimerRef.current = setTimeout(() => setCopied(false), 2000);
    });
  };

  const [dialogPermissions, setDialogPermissions] = useState({
    allow_terminal_access: true,
    allow_file_access: true,
    allow_model_access: false,
    require_approval_for_actions: true,
    max_participants: 5,
  });

  const handleJoinSession = async (code: string) => {
    try {
      const { invoke } = await import("@/lib/invoke");
      await invoke("collaboration_join_session", {
        conversation_id: conversationId,
        invite_code: code,
      });
      setDialogOpen(false);
      const updated = await invoke<SharedSession[]>(
        "collaboration_list_sessions",
      ).catch(() => []);
      setSessions(updated);
    } catch (e) {
      if (import.meta.env.DEV) { console.warn("Failed to join session:", e); }
      throw e;
    }
  };

  const shareDialog = (
    <SessionShareDialog
      open={dialogOpen}
      sessionId={conversationId}
      permissions={dialogPermissions}
      onClose={() => setDialogOpen(false)}
      onPermissionsChange={(perms) => setDialogPermissions(perms)}
      onJoinSession={handleJoinSession}
    />
  );

  if (sessions.length === 0) {
    return (
      <Card size="small">
        {shareDialog}
        <div className="flex items-center gap-2 mb-2">
          <Users size={16} className="text-blue-500" />
          <Title level={5} className="mb-0">
            {t("chat.collaboration.title")}
          </Title>
        </div>
        <Text type="secondary" className="text-xs block mb-3">
          {t("chat.collaboration.noSessions")}
        </Text>
        <Button
          type="primary"
          size="small"
          icon={<Plus size={14} />}
          onClick={() => setDialogOpen(true)}
          block
        >
          {t("chat.collaboration.sessionShare.title")}
        </Button>
      </Card>
    );
  }

  return (
    <Card size="small" className="collaboration-panel">
      {shareDialog}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Users size={16} className="text-blue-500" />
          <Title level={5} className="mb-0">
            {t("chat.collaboration.title")}
          </Title>
          <Badge count={sessions.length} size="small" />
        </div>
        <Button
          type="primary"
          size="small"
          icon={<Plus size={14} />}
          onClick={() => setDialogOpen(true)}
        >
          {t("chat.collaboration.sessionShare.shareMode")}
        </Button>
      </div>

      <div className="space-y-2">
        {sessions.map((session) => {
          const isExpanded = expanded === session.session_id;
          const activeCount = session.participants.length;

          return (
            <Card
              key={session.session_id}
              size="small"
              className={session.is_active ? "border-blue-200" : "border-zinc-200"}
            >
              <div
                className="flex items-center justify-between cursor-pointer"
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    setExpanded(isExpanded ? null : session.session_id);
                  }
                }}
                onClick={() => setExpanded(isExpanded ? null : session.session_id)}
              >
                <Space>
                  <div
                    className={`w-2 h-2 rounded-full ${session.is_active ? "bg-green-500" : "bg-zinc-400"}`}
                  />
                  <Text strong className="text-sm">
                    {t("chat.collaboration.session")} {session.session_id.slice(0, 8)}
                  </Text>
                  <Badge
                    count={activeCount}
                    size="small"
                    style={{ backgroundColor: "#52c41a" }}
                  />
                </Space>
                <Space>
                  {session.invite_code && (
                    <Tooltip title={copied ? t("chat.collaboration.copied") : t("chat.collaboration.copyCode")}>
                      <Button
                        type="link"
                        size="small"
                        icon={<Copy size={12} />}
                        onClick={(e) => {
                          e.stopPropagation();
                          copyInviteCode(session.invite_code);
                        }}
                      >
                        {session.invite_code}
                      </Button>
                    </Tooltip>
                  )}
                </Space>
              </div>

              {isExpanded && (
                <div className="mt-2 pt-2 border-t border-zinc-100 dark:border-zinc-800 space-y-2">
                  <div>
                    <Text type="secondary" className="text-xs block mb-1">
                      {t("chat.collaboration.participants")}
                    </Text>
                    {session.participants.map((p) => (
                      <div
                        key={p.user_id}
                        className="flex items-center gap-2 py-0.5"
                      >
                        <UserPlus size={10} className="text-zinc-400" />
                        <Text className="text-xs">{p.display_name}</Text>
                        <Tag
                          color={p.role === "Owner"
                            ? "gold"
                            : p.role === "Editor"
                            ? "blue"
                            : undefined}
                          className="text-xs"
                        >
                          {p.role}
                        </Tag>
                        <Clock size={10} className="text-zinc-400 ml-auto" />
                      </div>
                    ))}
                  </div>

                  {session.shared_resources.length > 0 && (
                    <div>
                      <Text type="secondary" className="text-xs block mb-1">
                        {t("chat.collaboration.sharedResources")}
                      </Text>
                      {session.shared_resources.map((r, i) => (
                        <div key={i} className="flex items-center gap-2 py-0.5">
                          <Link size={10} className="text-zinc-400" />
                          <Text className="text-xs">{r.resource_type}</Text>
                          <Tag color="geekblue" className="text-xs">
                            {r.access_level}
                          </Tag>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </Card>
          );
        })}
      </div>
    </Card>
  );
}
