// SPDX-License-Identifier: AGPL-3.0-only

import { DockerConfigModal } from "@/components/terminal/DockerConfigModal";
import { IntegratedTerminal } from "@/components/terminal/IntegratedTerminal";
import { SshConfigModal } from "@/components/terminal/SshConfigModal";
import { StatusBarWidget } from "@/components/terminal/StatusBarWidget";
import { TerminalBackendSelector } from "@/components/terminal/TerminalBackendSelector";
import { message } from "@/lib/toast";
import { useTerminalStore } from "@/stores/feature/terminalStore";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

/**
 * 终端页面：直接渲染终端界面，不再嵌套二级 Tab。
 * 文件功能已提升为 WorkspaceSwitcher 一级 Tab。
 */
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
    <div className="term-layout" style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
      <div className="term-topbar" style={{ flexShrink: 0 }}>
        <TerminalBackendSelector
          current={selectedBackend}
          backends={backends}
          onSelect={handleBackendSelect}
          onConfigure={handleConfigure}
        />
      </div>

      <div className="term-main" style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
        <IntegratedTerminal />
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
