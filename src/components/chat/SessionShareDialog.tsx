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

function generateInviteCode(sessionId: string): string {
  let hash = 0;
  const base = sessionId.replace(/-/g, "");
  for (let i = 0; i < base.length; i++) {
    const char = base.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash |= 0;
  }
  const ts = Date.now().toString(16).slice(-4).toUpperCase();
  const code = Math.abs(hash).toString(16).slice(0, 4).toUpperCase();
  return `${code}${ts}`;
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
  const copiedTimerRef = useRef<number | undefined>(undefined);

  const inviteCode = useMemo(
    () => (sessionId ? generateInviteCode(sessionId) : ""),
    [sessionId],
  );

  useEffect(() => {
    return () => clearTimeout(copiedTimerRef.current);
  }, []);

  useEffect(() => {
    if (open) {
      setMode("share");
      setJoinCode("");
      setCopied(false);
      setJoining(false);
    }
  }, [open]);

  const handleSwitchMode = useCallback((newMode: "share" | "join") => {
    setMode(newMode);
    setJoinCode("");
    setCopied(false);
  }, []);

  const copyInviteCode = useCallback(() => {
    navigator.clipboard.writeText(inviteCode).then(
      () => {
        setCopied(true);
        clearTimeout(copiedTimerRef.current);
        copiedTimerRef.current = window.setTimeout(() => setCopied(false), 2000);
      },
      () => {
        message.error(t("chat.collaboration.sessionShare.copyFailed") || "Copy failed");
      },
    );
  }, [inviteCode, t]);

  const handleJoin = useCallback(async () => {
    const trimmed = joinCode.trim();
    if (!trimmed || !onJoinSession) { return; }
    setJoining(true);
    try {
      await onJoinSession(trimmed);
      setJoinCode("");
    } catch {
      message.error(t("chat.collaboration.sessionShare.joinFailed") || "Join failed");
    } finally {
      setJoining(false);
    }
  }, [joinCode, onJoinSession, t]);

  const handlePermissionChange = useCallback(
    (key: keyof SessionPermissions, value: boolean | number) => {
      onPermissionsChange?.({ ...permissions, [key]: value });
    },
    [permissions, onPermissionsChange],
  );

  return (
    <Modal
      title={null}
      open={open}
      onCancel={onClose}
      footer={null}
      width={480}
      destroyOnClose
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
                  placeholder={t("chat.collaboration.sessionShare.codePlaceholder")}
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
