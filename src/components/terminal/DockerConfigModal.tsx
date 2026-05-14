import { Input, Modal, Typography } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface DockerConfigModalProps {
  open: boolean;
  onClose: () => void;
  onConnect: (config: { socketPath: string }) => void;
}

export function DockerConfigModal({ open, onClose, onConnect }: DockerConfigModalProps) {
  const { t } = useTranslation();
  const [socketPath, setSocketPath] = useState("");

  const handleConnect = () => {
    onConnect({ socketPath: socketPath || "unix:///var/run/docker.sock" });
    onClose();
  };

  return (
    <Modal
      title={t("dockerConfig.title")}
      open={open}
      onCancel={onClose}
      onOk={handleConnect}
      okText={t("dockerConfig.connect")}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <div>
          <Text type="secondary">{t("dockerConfig.socketPath")}</Text>
          <Input
            id="docker-config-modal-input-64"
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
