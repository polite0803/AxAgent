import { Button, Card, Input, Modal, Space, Switch, Typography } from "antd";
import { Copy, Link, Shield, Terminal, Users } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

interface SessionShareDialogProps {
  open: boolean;
  sessionId: string;
  inviteCode: string;
  permissions: {
    allow_terminal_access: boolean;
    allow_file_access: boolean;
    allow_model_access: boolean;
    require_approval_for_actions: boolean;
    max_participants: number;
  };
  onClose: () => void;
  onPermissionsChange?: (permissions: Record<string, boolean>) => void;
  onJoinSession?: (inviteCode: string) => void;
}

function SessionShareDialog({
  open,
  sessionId: _sessionId,
  inviteCode,
  permissions,
  onClose,
  onPermissionsChange,
  onJoinSession,
}: SessionShareDialogProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const [joinCode, setJoinCode] = useState("");
  const [mode, setMode] = useState<"share" | "join">("share");

  const copyInviteLink = () => {
    navigator.clipboard.writeText(inviteCode).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  return (
    <Modal
      title={null}
      open={open}
      onCancel={onClose}
      footer={null}
      width={480}
    >
      <Card size="small" className="session-share-dialog">
        <div className="flex items-center gap-2 mb-4">
          <Users size={18} className="text-blue-500" />
          <Title level={5} className="mb-0">
            {t("chat.collaboration.sessionShare.title")}
          </Title>
        </div>

        {/* Mode toggle */}
        <div className="flex gap-2 mb-4">
          <Button
            type={mode === "share" ? "primary" : "default"}
            size="small"
            onClick={() => setMode("share")}
          >
            {t("chat.collaboration.sessionShare.shareMode")}
          </Button>
          <Button
            type={mode === "join" ? "primary" : "default"}
            size="small"
            onClick={() => setMode("join")}
          >
            {t("chat.collaboration.sessionShare.joinMode")}
          </Button>
        </div>

        {mode === "share"
          ? (
            <div className="space-y-4">
              {/* Invite Code */}
              <div>
                <Text type="secondary" className="block mb-1 text-xs">
                  {t("chat.collaboration.sessionShare.inviteCode")}
                </Text>
                <div className="flex gap-2">
                  <Input
                    value={inviteCode}
                    readOnly
                    size="small"
                    suffix={<Link size={14} className="text-gray-400" />}
                  />
                  <Button
                    size="small"
                    icon={<Copy size={14} />}
                    onClick={copyInviteLink}
                  >
                    {copied ? t("chat.collaboration.sessionShare.copied") : t("chat.collaboration.sessionShare.copy")}
                  </Button>
                </div>
              </div>

              {/* Permissions */}
              <div>
                <div className="flex items-center gap-2 mb-2">
                  <Shield size={14} className="text-gray-500" />
                  <Text strong className="text-sm">
                    {t("chat.collaboration.sessionShare.permissions")}
                  </Text>
                </div>
                <Space direction="vertical" className="w-full">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Terminal size={14} className="text-gray-500" />
                      <Text className="text-sm">
                        {t("chat.collaboration.sessionShare.terminalAccess")}
                      </Text>
                    </div>
                    <Switch
                      size="small"
                      checked={permissions.allow_terminal_access}
                      onChange={(v) => onPermissionsChange?.({ allow_terminal_access: v })}
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <Text className="text-sm">
                      {t("chat.collaboration.sessionShare.fileAccess")}
                    </Text>
                    <Switch
                      size="small"
                      checked={permissions.allow_file_access}
                      onChange={(v) => onPermissionsChange?.({ allow_file_access: v })}
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <Text className="text-sm">
                      {t("chat.collaboration.sessionShare.modelAccess")}
                    </Text>
                    <Switch
                      size="small"
                      checked={permissions.allow_model_access}
                      onChange={(v) => onPermissionsChange?.({ allow_model_access: v })}
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <Text className="text-sm">
                      {t("chat.collaboration.sessionShare.requireApproval")}
                    </Text>
                    <Switch
                      size="small"
                      checked={permissions.require_approval_for_actions}
                      onChange={(v) => onPermissionsChange?.({ require_approval_for_actions: v })}
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
                  placeholder="XXXX-XXXX"
                  size="middle"
                />
              </div>
              <Button
                type="primary"
                block
                disabled={!joinCode}
                onClick={() => onJoinSession?.(joinCode)}
              >
                {t("chat.collaboration.sessionShare.joinSession")}
              </Button>
            </div>
          )}
      </Card>
    </Modal>
  );
}

export default SessionShareDialog;
