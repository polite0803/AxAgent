import { DockerConfigModal } from "@/components/terminal/DockerConfigModal";
import { IntegratedTerminal } from "@/components/terminal/IntegratedTerminal";
import { SshConfigModal } from "@/components/terminal/SshConfigModal";
import { StatusBarWidget } from "@/components/terminal/StatusBarWidget";
import { TerminalBackendSelector } from "@/components/terminal/TerminalBackendSelector";
import { useTerminalStore } from "@/stores/feature/terminalStore";
import { message } from "antd";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

export function TerminalPage() {
  const { t } = useTranslation();
  const { sessions, activeSessionId } = useTerminalStore();
  const activeSession = sessions.find((s) => s.id === activeSessionId);

  const [dockerModalOpen, setDockerModalOpen] = useState(false);
  const [sshModalOpen, setSshModalOpen] = useState(false);
  const [selectedBackend, setSelectedBackend] = useState("local");

  const backends = [
    {
      type: "local",
      connected: true,
      sessions: sessions.filter((s) => s.status === "running").length,
    },
    { type: "docker", connected: false, sessions: 0 },
    { type: "ssh", connected: false, sessions: 0 },
  ];

  const handleBackendSelect = useCallback((backendType: string) => {
    setSelectedBackend(backendType);
  }, []);

  const handleConfigure = useCallback((backendType: string) => {
    if (backendType === "docker") {
      setDockerModalOpen(true);
    } else if (backendType === "ssh") {
      setSshModalOpen(true);
    }
  }, []);

  const handleDockerConnect = useCallback(
    (_config: { socketPath: string }) => {
      message.info(t("terminal.dockerConnectPending"));
      setDockerModalOpen(false);
    },
    [t],
  );

  const handleSshConnect = useCallback(
    (_config: {
      host: string;
      port: number;
      username: string;
      keyPath: string;
    }) => {
      message.info(t("terminal.sshConnectPending"));
      setSshModalOpen(false);
    },
    [t],
  );

  return (
    <div className="term-layout">
      <div className="term-topbar">
        <TerminalBackendSelector
          current={selectedBackend}
          backends={backends}
          onSelect={handleBackendSelect}
          onConfigure={handleConfigure}
        />
      </div>

      <div className="term-main">
        <IntegratedTerminal height={typeof window !== "undefined" ? window.innerHeight - 160 : 600} />
      </div>

      <DockerConfigModal
        open={dockerModalOpen}
        onClose={() => setDockerModalOpen(false)}
        onConnect={handleDockerConnect}
      />

      <SshConfigModal
        open={sshModalOpen}
        onClose={() => setSshModalOpen(false)}
        onConnect={handleSshConnect}
      />

      <StatusBarWidget sessionId={activeSession?.id} />
    </div>
  );
}
