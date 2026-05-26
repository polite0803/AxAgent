import { invoke } from "@/lib/invoke";
import type { SharePermissions, ShareSessionInfo } from "@/types";
import { Button, Card, Input, InputNumber, message, Modal, Space, Switch, Typography } from "antd";
import { Copy, Link, Shield, Terminal, Users } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

interface SessionShareDialogProps {
  open: boolean;
  sessionId: string;
  permissions: SessionPermissions;
  onClose: () => void;
  onPermissionsChange?: (permissions: SessionPermissions) => void;
  onJoinSession?: (inviteCode: string) => Promise<void>;
}

interface SessionPermissions {
  allow_terminal_access: boolean;
  allow_file_access: boolean;
  allow_model_access: boolean;
  require_approval_for_actions: boolean;
  max_participants: number;
}

export function SessionShareDialog({
  open,
  sessionId,
  permissions,
  onClose,
  onPermissionsChange,
  onJoinSession,
}: SessionShareDialogProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const [joinCode, setJoinCode] = useState("");
  const [mode, setMode] = useState<"share" | "join">("share");
  const [joining, setJoining] = useState(false);
  const [creating, setCreating] = useState(false);
  const [shareInfo, setShareInfo] = useState<ShareSessionInfo | null>(null);
  const copiedTimerRef = useRef<number | undefined>(undefined);
  const previousOpenRef = useRef(false);

  // 转换为后端 SharePermissions 格式
  const toBackendPermissions = useCallback(
    (p: SessionPermissions): SharePermissions => ({
      allow_terminal_access: p.allow_terminal_access,
      allow_file_access: p.allow_file_access,
      allow_model_access: p.allow_model_access,
      require_approval_for_actions: p.require_approval_for_actions,
      max_participants: p.max_participants,
    }),
    [],
  );

  // 打开弹窗或切换到共享模式时，创建/更新共享会话以获取邀请码
  useEffect(() => {
    if (open && mode === "share" && sessionId) {
      let cancelled = false;
      const createSession = async () => {
        setCreating(true);
        try {
          const info = await invoke<ShareSessionInfo>(
            "create_share_session",
            {
              conversationId: sessionId,
              permissions: toBackendPermissions(permissions),
            },
          );
          if (!cancelled) {
            setShareInfo(info);
          }
        } catch {
          if (!cancelled) {
            message.error(
              t("chat.collaboration.sessionShare.createFailed")
                || "Failed to create share session",
            );
          }
        } finally {
          if (!cancelled) {
            setCreating(false);
          }
        }
      };
      createSession();
      return () => {
        cancelled = true;
      };
    }
  }, [open, mode, sessionId, permissions, toBackendPermissions, t]);

  // 弹窗打开/关闭时重置状态
  useEffect(() => {
    if (open && !previousOpenRef.current) {
      setMode("share");
      setJoinCode("");
      setCopied(false);
      setJoining(false);
      setShareInfo(null);
    }
    previousOpenRef.current = open;
  }, [open]);

  // 后端返回的邀请码（优先于前端生成）
  const inviteCode = useMemo(
    () => shareInfo?.invite_code ?? "",
    [shareInfo],
  );

  const handleSwitchMode = useCallback(
    (newMode: "share" | "join") => {
      setMode(newMode);
      setJoinCode("");
      setCopied(false);
      if (newMode !== "share") {
        setShareInfo(null);
      }
    },
    [],
  );

  const copyInviteCode = useCallback(() => {
    if (!inviteCode) { return; }
    navigator.clipboard.writeText(inviteCode).then(
      () => {
        setCopied(true);
        clearTimeout(copiedTimerRef.current);
        copiedTimerRef.current = window.setTimeout(
          () => setCopied(false),
          2000,
        );
      },
      () => {
        message.error(
          t("chat.collaboration.sessionShare.copyFailed") || "Copy failed",
        );
      },
    );
  }, [inviteCode, t]);

  const handleJoin = useCallback(async () => {
    const trimmed = joinCode.trim();
    if (!trimmed) {
      return;
    }
    setJoining(true);
    try {
      // 调用真实的 Tauri join_share_session 命令
      await invoke<ShareSessionInfo>("join_share_session", {
        inviteCode: trimmed,
      });
      setJoinCode("");
      message.success(
        t("chat.collaboration.sessionShare.joinSuccess") || "Joined session successfully",
      );
      // 同时通知父组件（向后兼容）
      onJoinSession?.(trimmed);
    } catch {
      message.error(
        t("chat.collaboration.sessionShare.joinFailed") || "Join failed",
      );
    } finally {
      setJoining(false);
    }
  }, [joinCode, onJoinSession, t]);

  const handlePermissionChange = useCallback(
    (key: keyof SessionPermissions, value: boolean | number) => {
      const newPermissions = { ...permissions, [key]: value };
      // 调用后端 create_share_session 更新权限
      if (sessionId) {
        invoke<ShareSessionInfo>("create_share_session", {
          conversationId: sessionId,
          permissions: toBackendPermissions(newPermissions),
        })
          .then((info) => setShareInfo(info))
          .catch(() => {
            message.error(
              t("chat.collaboration.sessionShare.updateFailed")
                || "Failed to update permissions",
            );
          });
      }
      onPermissionsChange?.(newPermissions);
    },
    [permissions, sessionId, onPermissionsChange, t, toBackendPermissions],
  );

  return (
    <Modal
      title={null}
      open={open}
      onCancel={onClose}
      footer={null}
      width={480}
      destroyOnHidden
    >
      <Card size="small" className="session-share-dialog">
        <div className="flex items-center gap-2 mb-4">
          <Users size={18} className="text-blue-500" />
          <Title level={5} className="mb-0">
            {t("chat.collaboration.sessionShare.title")}
          </Title>
        </div>

        <div className="flex gap-2 mb-4">
          <Button
            type={mode === "share" ? "primary" : "default"}
            size="small"
            onClick={() => handleSwitchMode("share")}
          >
            {t("chat.collaboration.sessionShare.shareMode")}
          </Button>
          <Button
            type={mode === "join" ? "primary" : "default"}
            size="small"
            onClick={() => handleSwitchMode("join")}
          >
            {t("chat.collaboration.sessionShare.joinMode")}
          </Button>
        </div>

        {mode === "share"
          ? (
            <div className="space-y-4">
              <div>
                <Text type="secondary" className="block mb-1 text-xs">
                  {t("chat.collaboration.sessionShare.inviteCode")}
                </Text>
                <div className="flex gap-2">
                  <Input
                    value={inviteCode}
                    readOnly
                    size="small"
                    placeholder={creating ? "Generating..." : ""}
                    suffix={<Link size={14} className="text-zinc-400" />}
                  />
                  <Button
                    size="small"
                    icon={<Copy size={14} />}
                    onClick={copyInviteCode}
                    disabled={!inviteCode}
                  >
                    {copied
                      ? t("chat.collaboration.sessionShare.copied")
                      : t("chat.collaboration.sessionShare.copy")}
                  </Button>
                </div>
              </div>

              <div>
                <div className="flex items-center gap-2 mb-2">
                  <Shield size={14} className="text-zinc-500" />
                  <Text strong className="text-sm">
                    {t("chat.collaboration.sessionShare.permissions")}
                  </Text>
                </div>
                <Space direction="vertical" className="w-full">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Terminal size={14} className="text-zinc-500" />
                      <Text className="text-sm">
                        {t("chat.collaboration.sessionShare.terminalAccess")}
                      </Text>
                    </div>
                    <Switch
                      size="small"
                      checked={permissions.allow_terminal_access}
                      onChange={(v) => handlePermissionChange("allow_terminal_access", v)}
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <Text className="text-sm">
                      {t("chat.collaboration.sessionShare.fileAccess")}
                    </Text>
                    <Switch
                      size="small"
                      checked={permissions.allow_file_access}
                      onChange={(v) => handlePermissionChange("allow_file_access", v)}
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <Text className="text-sm">
                      {t("chat.collaboration.sessionShare.modelAccess")}
                    </Text>
                    <Switch
                      size="small"
                      checked={permissions.allow_model_access}
                      onChange={(v) => handlePermissionChange("allow_model_access", v)}
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <Text className="text-sm">
                      {t("chat.collaboration.sessionShare.requireApproval")}
                    </Text>
                    <Switch
                      size="small"
                      checked={permissions.require_approval_for_actions}
                      onChange={(v) => handlePermissionChange("require_approval_for_actions", v)}
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <Text className="text-sm">
                      {t("chat.collaboration.sessionShare.maxParticipants")}
                    </Text>
                    <InputNumber
                      size="small"
                      min={1}
                      max={50}
                      value={permissions.max_participants}
                      onChange={(v) => handlePermissionChange("max_participants", v ?? 5)}
                      style={{ width: 72 }}
                    />
                  </div>
                </Space>
              </div>
            </div>
          )
          : (
            <div className="space-y-4">
              <div>
                <Text type="secondary" className="block mb-1 text-xs">
                  {t("chat.collaboration.sessionShare.enterInviteCode")}
                </Text>
                <Input
                  value={joinCode}
                  onChange={(e) => setJoinCode(e.target.value)}
                  placeholder={t(
                    "chat.collaboration.sessionShare.codePlaceholder",
                  )}
                  size="middle"
                  onPressEnter={handleJoin}
                />
              </div>
              <Button
                type="primary"
                block
                disabled={!joinCode.trim() || joining}
                loading={joining}
                onClick={handleJoin}
              >
                {t("chat.collaboration.sessionShare.joinSession")}
              </Button>
            </div>
          )}
      </Card>
    </Modal>
  );
}
