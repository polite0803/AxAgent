import { Input, Modal, Typography } from "antd";
import { useState } from "react";

const { Text } = Typography;

interface DockerConfigModalProps {
  open: boolean;
  onClose: () => void;
  onConnect: (config: { socketPath: string }) => void;
}

export function DockerConfigModal({ open, onClose, onConnect }: DockerConfigModalProps) {
  const [socketPath, setSocketPath] = useState("");

  const handleConnect = () => {
    onConnect({ socketPath: socketPath || "unix:///var/run/docker.sock" });
    onClose();
  };

  return (
    <Modal
      title="Docker Configuration"
      open={open}
      onCancel={onClose}
      onOk={handleConnect}
      okText="Connect"
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <div>
          <Text type="secondary">Docker Socket Path</Text>
          <Input
            value={socketPath}
            onChange={(e) => setSocketPath(e.target.value)}
            placeholder="unix:///var/run/docker.sock"
          />
          <Text type="secondary" style={{ fontSize: 12 }}>
            Leave empty for default Docker socket
          </Text>
        </div>
      </div>
    </Modal>
  );
}
