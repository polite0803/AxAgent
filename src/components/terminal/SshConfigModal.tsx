import { Input, InputNumber, Modal, Typography } from "antd";
import { useState } from "react";

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
      title="SSH Configuration"
      open={open}
      onCancel={onClose}
      onOk={handleConnect}
      okText="Connect"
      okButtonProps={{ disabled: !host.trim() }}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <div>
          <Text type="secondary">Host</Text>
          <Input
            value={host}
            onChange={(e) => setHost(e.target.value)}
            placeholder="192.168.1.100 or server.example.com"
          />
        </div>
        <div>
          <Text type="secondary">Port</Text>
          <InputNumber
            value={port}
            onChange={(v) => setPort(v ?? 22)}
            min={1}
            max={65535}
            style={{ width: "100%" }}
          />
        </div>
        <div>
          <Text type="secondary">Username</Text>
          <Input
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder="root"
          />
        </div>
        <div>
          <Text type="secondary">SSH Key Path (optional)</Text>
          <Input
            value={keyPath}
            onChange={(e) => setKeyPath(e.target.value)}
            placeholder="~/.ssh/id_rsa"
          />
        </div>
      </div>
    </Modal>
  );
}
