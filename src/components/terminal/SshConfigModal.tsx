import { Input, InputNumber, Modal, Typography } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface SshConfigModalProps {
  open: boolean;
  onClose: () => void;
  onConnect: (config: {
    host: string;
    port: number;
    username: string;
    keyPath: string;
  }) => void;
}

export function SshConfigModal({ open, onClose, onConnect }: SshConfigModalProps) {
  const { t } = useTranslation();
  const [host, setHost] = useState("");
  const [port, setPort] = useState(22);
  const [username, setUsername] = useState("");
  const [keyPath, setKeyPath] = useState("");

  const handleConnect = () => {
    if (!host.trim()) { return; }
    onConnect({ host: host.trim(), port, username: username.trim(), keyPath: keyPath.trim() });
    onClose();
  };

  return (
    <Modal
      title={t("sshConfig.title")}
      open={open}
      onCancel={onClose}
      onOk={handleConnect}
      okText={t("sshConfig.connect")}
      okButtonProps={{ disabled: !host.trim() }}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <div>
          <Text type="secondary">{t("sshConfig.host")}</Text>
          <Input
            id="ssh-config-modal-input-65"
            value={host}
            onChange={(e) => setHost(e.target.value)}
            placeholder="192.168.1.100 or server.example.com"
          />
        </div>
        <div>
          <Text type="secondary">{t("sshConfig.port")}</Text>
          <InputNumber
            id="ssh-config-modal-inputnumber-66"
            value={port}
            onChange={(v) => setPort(v ?? 22)}
            min={1}
            max={65535}
            style={{ width: "100%" }}
          />
        </div>
        <div>
          <Text type="secondary">{t("sshConfig.username")}</Text>
          <Input
            id="ssh-config-modal-input-67"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder="root"
          />
        </div>
        <div>
          <Text type="secondary">SSH Key Path (optional)</Text>
          <Input
            id="ssh-config-modal-input-68"
            value={keyPath}
            onChange={(e) => setKeyPath(e.target.value)}
            placeholder="~/.ssh/id_rsa"
          />
        </div>
      </div>
    </Modal>
  );
}
